use scriptcraft_core::editing::EditingSession;
use scriptcraft_core::fdx::{self, FdxDocument};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, PreservedSection};
use scriptcraft_core::Project;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

const OPENED: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="document-original"><XRef DateTime="20260930T120000" id="xref-original" Vendor="keep"/><OpaqueRefChild Retain="yes"/></DocumentRef><Content><Paragraph Type="Action" id="paragraph-1"><Text>lineage fixture</Text></Paragraph></Content></FinalDraft>"#;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let serial = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v6-wo0011-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|_| unreachable!("test directory can be created"));
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn imported(bytes: &[u8]) -> FdxDocument {
    fdx::read_v6(bytes).unwrap_or_else(|_| unreachable!("lineage fixture is valid"))
}

fn direct_children<'a>(sections: &'a [PreservedSection], root_name: &str) -> &'a [PreservedNode] {
    let root = sections
        .iter()
        .flat_map(|section| &section.children)
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == root_name => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("root element exists"));
    &root.children
}

fn document_ref(bytes: &[u8]) -> PreservedElement {
    let document = imported(bytes);
    let sections = &document
        .project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("imported FDX retains preservation"))
        .sections;
    direct_children(sections, "FinalDraft")
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "DocumentRef" => {
                Some(element.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("DocumentRef exists after save"))
}

fn attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.namespace_uri.is_none() && attribute.name.local_name == name)
        .map(|attribute| attribute.value.as_str())
}

fn xrefs(element: &PreservedElement) -> Vec<(String, String)> {
    element
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(xref) if xref.name.local_name == "XRef" => Some((
                attribute(xref, "DateTime")?.to_owned(),
                attribute(xref, "id")?.to_owned(),
            )),
            _ => None,
        })
        .collect()
}

fn save_as(
    fs: &StdFdxStorageFileSystem,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    destination: &Path,
) {
    save_session(
        fs,
        session,
        target,
        FdxSaveMode::SaveAs(destination.to_owned()),
    )
    .unwrap_or_else(|_| unreachable!("lineage Save As succeeds"));
}

#[test]
fn ordinary_saves_refresh_documentref_only_and_direct_write_is_lineage_neutral() {
    let directory = TestDirectory::new();
    let source_path = directory.path("ordinary.fdx");
    std::fs::write(&source_path, OPENED)
        .unwrap_or_else(|_| unreachable!("lineage fixture can be written"));
    let fs = StdFdxStorageFileSystem;
    let opened = open(&fs, &source_path)
        .unwrap_or_else(|_| unreachable!("lineage fixture opens"));
    let mut session = EditingSession::new(opened.document.project.clone())
        .unwrap_or_else(|_| unreachable!("lineage session initializes"));
    let before = opened.document.project.clone();
    let direct = fdx::write_v6(&opened.document.project)
        .unwrap_or_else(|_| unreachable!("direct source-backed serialization succeeds"));
    assert_eq!(opened.document.project, before);
    let direct_ref = document_ref(&direct);
    assert_eq!(attribute(&direct_ref, "DateTime"), Some("20261001T120000"));
    assert_eq!(xrefs(&direct_ref), vec![("20260930T120000".into(), "xref-original".into())]);

    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("ordinary Save succeeds"));
    let saved = std::fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("ordinary Save output is readable"));
    let saved_ref = document_ref(&saved);
    assert_eq!(attribute(&saved_ref, "id"), Some("document-original"));
    assert_ne!(attribute(&saved_ref, "DateTime"), Some("20261001T120000"));
    assert_eq!(xrefs(&saved_ref), vec![("20260930T120000".into(), "xref-original".into())]);
    let saved_xml = String::from_utf8(saved)
        .unwrap_or_else(|_| unreachable!("ordinary Save output is UTF-8"));
    assert!(saved_xml.contains("Vendor=\"keep\""));
    assert!(saved_xml.contains("<OpaqueRefChild Retain=\"yes\"/>"));
}

#[test]
fn repeated_save_as_reuses_the_session_open_timestamp_and_reopen_starts_another_session() {
    let directory = TestDirectory::new();
    let source_path = directory.path("opened.fdx");
    std::fs::write(&source_path, OPENED)
        .unwrap_or_else(|_| unreachable!("lineage fixture can be written"));
    let fs = StdFdxStorageFileSystem;
    let opened = open(&fs, &source_path)
        .unwrap_or_else(|_| unreachable!("lineage fixture opens"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("lineage session initializes"));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("first ordinary Save succeeds"));
    save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("second ordinary Save succeeds"));

    let first_copy = directory.path("first-copy.fdx");
    save_as(&fs, &mut session, &mut target, &first_copy);
    let second_copy = directory.path("second-copy.fdx");
    save_as(&fs, &mut session, &mut target, &second_copy);
    let second_bytes = std::fs::read(&second_copy)
        .unwrap_or_else(|_| unreachable!("second Save As output is readable"));
    let second_ref = document_ref(&second_bytes);
    assert_eq!(attribute(&second_ref, "id"), Some("document-original"));
    let second_xrefs = xrefs(&second_ref);
    assert_eq!(second_xrefs.len(), 3);
    assert_eq!(second_xrefs[0], ("20260930T120000".into(), "xref-original".into()));
    assert!(valid_timestamp(&second_xrefs[1].0));
    assert_eq!(second_xrefs[1].0, second_xrefs[2].0);
    assert_ne!(second_xrefs[1].1, second_xrefs[2].1);
    assert_ne!(attribute(&second_ref, "DateTime"), Some("20261001T120000"));

    let reopened = open(&fs, &second_copy)
        .unwrap_or_else(|_| unreachable!("saved copy reopens"));
    let mut reopened_session = EditingSession::new(reopened.document.project)
        .unwrap_or_else(|_| unreachable!("reopened session initializes"));
    let mut reopened_target = FdxSaveTarget::from_baseline(reopened.baseline);
    let third_copy = directory.path("third-copy.fdx");
    save_as(&fs, &mut reopened_session, &mut reopened_target, &third_copy);
    let third_bytes = std::fs::read(&third_copy)
        .unwrap_or_else(|_| unreachable!("third Save As output is readable"));
    let third_xrefs = xrefs(&document_ref(&third_bytes));
    assert_eq!(third_xrefs.len(), 4);
    assert!(valid_timestamp(&third_xrefs[3].0));
    assert_ne!(third_xrefs[2].1, third_xrefs[3].1);
}

#[test]
fn save_as_and_missing_or_malformed_documentref_keep_session_and_save_times_separate() {
    let directory = TestDirectory::new();
    let fs = StdFdxStorageFileSystem;

    let project = Project::new().unwrap_or_else(|_| unreachable!("new project initializes"));
    let mut new_session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("new session initializes"));
    let mut new_target = FdxSaveTarget::new_unsaved();
    let new_path = directory.path("new-project.fdx");
    save_as(&fs, &mut new_session, &mut new_target, &new_path);
    let new_bytes = std::fs::read(&new_path)
        .unwrap_or_else(|_| unreachable!("new Save As output is readable"));
    let new_ref = document_ref(&new_bytes);
    let new_xrefs = xrefs(&new_ref);
    assert_eq!(new_xrefs.len(), 1);
    assert!(valid_timestamp(&new_xrefs[0].0));
    assert!(valid_timestamp(attribute(&new_ref, "DateTime").unwrap_or("")));
    assert_ne!(new_xrefs[0].1, attribute(&new_ref, "id").unwrap_or(""));

    for (filename, source, expected_id) in [
        (
            "missing-ref.fdx",
            br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="p1"><Text>missing DocumentRef</Text></Paragraph></Content></FinalDraft>"#.as_slice(),
            None,
        ),
        (
            "malformed-date.fdx",
            br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20260230T120000" id="imported-non-uuid"><XRef DateTime="20260101T000000" id="old-xref"/></DocumentRef><Content><Paragraph Type="Action" id="p1"><Text>malformed DateTime</Text></Paragraph></Content></FinalDraft>"#.as_slice(),
            Some("imported-non-uuid"),
        ),
    ] {
        let source_path = directory.path(filename);
        std::fs::write(&source_path, source)
            .unwrap_or_else(|_| unreachable!("fallback fixture can be written"));
        let opened = open(&fs, &source_path)
            .unwrap_or_else(|_| unreachable!("fallback fixture opens"));
        let mut session = EditingSession::new(opened.document.project)
            .unwrap_or_else(|_| unreachable!("fallback session initializes"));
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let destination = directory.path(&format!("copy-{filename}"));
        save_as(&fs, &mut session, &mut target, &destination);
        let output = std::fs::read(&destination)
            .unwrap_or_else(|_| unreachable!("fallback Save As output is readable"));
        let reference = document_ref(&output);
        if let Some(expected_id) = expected_id {
            assert_eq!(attribute(&reference, "id"), Some(expected_id));
        }
        let output_xrefs = xrefs(&reference);
        let expected_xref_count = if filename == "malformed-date.fdx" { 2 } else { 1 };
        assert_eq!(output_xrefs.len(), expected_xref_count);
        assert!(valid_timestamp(&output_xrefs[expected_xref_count - 1].0));
        assert!(valid_timestamp(attribute(&reference, "DateTime").unwrap_or("")));
        let document = imported(&output);
        let root_children = direct_children(
            &document
                .project
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!("saved project retains source tree"))
                .sections,
            "FinalDraft",
        );
        let document_ref_index = root_children.iter().position(|node| matches!(node,
            PreservedNode::Element(element) if element.name.local_name == "DocumentRef"
        ));
        let content_index = root_children.iter().position(|node| matches!(node,
            PreservedNode::Element(element) if element.name.local_name == "Content"
        ));
        assert!(document_ref_index.zip(content_index).is_some_and(|(reference, content)| reference < content));
    }
}

#[test]
fn ordinary_save_initializes_only_missing_xref_history() {
    let directory = TestDirectory::new();
    let source_path = directory.path("no-xref-history.fdx");
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="document-original"/><Content><Paragraph Type="Action" id="paragraph-1"><Text>lineage fixture</Text></Paragraph></Content></FinalDraft>"#;
    std::fs::write(&source_path, source)
        .unwrap_or_else(|_| unreachable!("lineage fixture can be written"));
    let fs = StdFdxStorageFileSystem;
    let opened = open(&fs, &source_path)
        .unwrap_or_else(|_| unreachable!("lineage fixture opens"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("lineage session initializes"));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);

    save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("ordinary Save initializes lineage"));
    let saved = std::fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("ordinary Save output is readable"));
    let saved_ref = document_ref(&saved);
    assert_eq!(attribute(&saved_ref, "id"), Some("document-original"));
    assert_eq!(xrefs(&saved_ref).len(), 1);
    assert!(valid_timestamp(&xrefs(&saved_ref)[0].0));
    assert!(valid_timestamp(attribute(&saved_ref, "DateTime").unwrap_or("")));
}

fn valid_timestamp(value: &str) -> bool {
    value.len() == 15
        && value.as_bytes().get(8) == Some(&b'T')
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 8 || byte.is_ascii_digit())
}
