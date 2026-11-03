use scriptcraft_core::editing::EditingSession;
use scriptcraft_core::fdx::{self, FdxDocument, WindowState};
use scriptcraft_core::fdx_storage::{
    save_session, save_session_with_window_state, FdxSaveMode, FdxSaveTarget,
    StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, PreservedSection};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let serial = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v6-wo0012-{}-{serial}",
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

fn read(bytes: &[u8]) -> FdxDocument {
    fdx::read_v6(bytes).unwrap_or_else(|_| unreachable!("WindowState fixture opens"))
}

fn root_children(sections: &[PreservedSection]) -> &[PreservedNode] {
    let root = sections
        .iter()
        .flat_map(|section| &section.children)
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("root exists"));
    &root.children
}

fn named_child<'a>(children: &'a [PreservedNode], name: &str) -> Vec<&'a PreservedElement> {
    children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == name => Some(element),
            _ => None,
        })
        .collect()
}

fn attr<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.namespace_uri.is_none() && attribute.name.local_name == name)
        .map(|attribute| attribute.value.as_str())
}

fn direct_xrefs(document_ref: &PreservedElement) -> Vec<&PreservedElement> {
    document_ref
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "XRef" => Some(element),
            _ => None,
        })
        .collect()
}

const SIMPLE: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content></FinalDraft>"#;

#[test]
fn checked_in_reference_root_window_states_project_without_nested_title_page_state() {
    let references: [(&[u8], WindowState); 4] = [
        (
            include_bytes!("../../../.reference/reference.fdx"),
            WindowState { left: 0, top: 75, width: 870, height: 874 },
        ),
        (
            include_bytes!("../../../.reference/reference2.fdx"),
            WindowState { left: 0, top: 95, width: 870, height: 854 },
        ),
        (
            include_bytes!("../../../.reference/arc copy.fdx"),
            WindowState { left: 29, top: 125, width: 870, height: 824 },
        ),
        (
            include_bytes!("../../../.reference/real_script_example.fdx"),
            WindowState { left: 2, top: 95, width: 1510, height: 854 },
        ),
    ];
    for (bytes, expected) in references {
        assert_eq!(read(bytes).window_state, Some(expected));
    }

    let nested_only = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content><TitlePage><WindowState Left="9" Top="8" Width="7" Height="6"/></TitlePage></FinalDraft>"#;
    assert_eq!(read(nested_only).window_state, None);

    let foreign_namespace_only = br#"<FinalDraft xmlns="urn:fdx" xmlns:other="urn:other" DocumentType="Script" Version="6"><other:WindowState Left="1" Top="2" Width="3" Height="4"/><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content></FinalDraft>"#;
    assert_eq!(read(foreign_namespace_only).window_state, None);
    let matching_namespace = br#"<FinalDraft xmlns="urn:fdx" DocumentType="Script" Version="6"><WindowState Left="-1" Top="2" Width="3" Height="4"/><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content></FinalDraft>"#;
    assert_eq!(
        read(matching_namespace).window_state,
        Some(WindowState { left: -1, top: 2, width: 3, height: 4 })
    );
}

#[test]
fn invalid_or_ambiguous_window_state_uses_safe_runtime_fallback_without_blocking_open() {
    let cases: [&[u8]; 8] = [
        SIMPLE,
        br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="1" Top="2" Width="3" Height="4"/><WindowState Left="5" Top="6" Width="7" Height="8"/><Content/></FinalDraft>"#,
        br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="x" Top="2" Width="3" Height="4"/><Content/></FinalDraft>"#,
        br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="2147483648" Top="2" Width="3" Height="4"/><Content/></FinalDraft>"#,
        br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="1" Top="2" Width="2147483648" Height="4"/><Content/></FinalDraft>"#,
        br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="1" Top="2" Width="3"/><Content/></FinalDraft>"#,
        br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="1" Top="2" Width="0" Height="4"/><Content/></FinalDraft>"#,
        br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="1" Top="2" Width="3" Height="-4"/><Content/></FinalDraft>"#,
    ];
    for bytes in cases {
        let document = read(bytes);
        assert_eq!(document.window_state, None);
    }

    let negative_coordinates = br#"<FinalDraft DocumentType="Script" Version="6"><WindowState Left="-1920" Top="-40" Width="870" Height="854" Mode="FutureMode"/><Content/></FinalDraft>"#;
    assert_eq!(
        read(negative_coordinates).window_state,
        Some(WindowState { left: -1920, top: -40, width: 870, height: 854 })
    );
}

#[test]
fn actual_save_replaces_only_direct_root_window_blocks_with_latest_geometry() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="doc"/><PageLayout Vendor="keep"/><WindowState Height="800" Left="1" Mode="Normal" Top="2" Width="900"/><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content><WindowState Height="700" Left="3" Mode="Normal" Top="4" Width="800"/><ElementSettings Type="Action"/><TitlePage><WindowState Left="77" Top="88" Width="99" Height="111"/><Opaque Keep="yes"/></TitlePage><VendorRoot Keep="yes"/></FinalDraft>"#;
    let opened = read(source);
    assert_eq!(opened.window_state, None);
    let preservation_before = opened
        .project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("import retains source"))
        .sections
        .clone();
    let direct = fdx::write_v6(&opened.project)
        .unwrap_or_else(|_| unreachable!("direct source-backed serialization succeeds"));
    let direct_document = read(&direct);
    let direct_root = root_children(
        &direct_document
            .project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("direct output retains source"))
            .sections,
    );
    assert_eq!(named_child(direct_root, "WindowState").len(), 2);

    let directory = TestDirectory::new();
    let destination = directory.path("window-save-as.fdx");
    let mut session = EditingSession::new(opened.project)
        .unwrap_or_else(|_| unreachable!("session initializes"));
    let revision = session.revision();
    let dirty = session.dirty();
    let mut target = FdxSaveTarget::new_unsaved();
    let geometry = WindowState { left: -1440, top: 20, width: 1200, height: 900 };
    save_session_with_window_state(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
        geometry,
    )
    .unwrap_or_else(|_| unreachable!("Save As with current geometry succeeds"));
    assert_eq!(session.revision(), revision);
    assert_eq!(session.dirty(), dirty);

    let saved = std::fs::read(destination)
        .unwrap_or_else(|_| unreachable!("saved FDX output can be read"));
    let saved_document = read(&saved);
    assert_eq!(saved_document.window_state, Some(geometry));
    let saved_state = saved_document
        .project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("saved output retains source"));
    let saved_root = root_children(&saved_state.sections);
    let windows = named_child(saved_root, "WindowState");
    assert_eq!(windows.len(), 1);
    assert_eq!(attr(windows[0], "Height"), Some("900"));
    assert_eq!(attr(windows[0], "Left"), Some("-1440"));
    assert_eq!(attr(windows[0], "Mode"), Some("Normal"));
    assert_eq!(attr(windows[0], "Top"), Some("20"));
    assert_eq!(attr(windows[0], "Width"), Some("1200"));
    let document_ref = named_child(saved_root, "DocumentRef")[0];
    let xrefs = direct_xrefs(document_ref);
    assert_eq!(xrefs.len(), 1);
    assert_eq!(attr(xrefs[0], "DateTime"), Some("20261001T120000"));
    assert_eq!(named_child(saved_root, "TitlePage")[0], named_child(root_children(&preservation_before), "TitlePage")[0]);
    assert_eq!(named_child(saved_root, "VendorRoot").len(), 1);
}

#[test]
fn imported_missing_window_state_uses_page_layout_then_element_settings_anchor() {
    let directory = TestDirectory::new();
    let geometry = WindowState { left: 8, top: 9, width: 1000, height: 700 };
    let cases: [(&str, &[u8], Option<&str>); 2] = [
        (
            "with-page-layout",
            br#"<FinalDraft DocumentType="Script" Version="6"><PageLayout/><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content><ElementSettings Type="Action"/></FinalDraft>"#,
            Some("PageLayout"),
        ),
        (
            "settings-fallback",
            br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content><ElementSettings Type="Action"/></FinalDraft>"#,
            None,
        ),
    ];
    for (name, source, page_layout) in cases {
        let mut session = EditingSession::new(read(source).project)
            .unwrap_or_else(|_| unreachable!("missing-window fixture initializes"));
        let destination = directory.path(&format!("{name}.fdx"));
        let mut target = FdxSaveTarget::new_unsaved();
        save_session_with_window_state(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
            geometry,
        )
        .unwrap_or_else(|_| unreachable!("missing WindowState is inserted on Save As"));
        let saved = std::fs::read(destination)
            .unwrap_or_else(|_| unreachable!("inserted-state output can be read"));
        let document = read(&saved);
        let children = root_children(
            &document
                .project
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!("saved output retains source"))
                .sections,
        );
        let window = children
            .iter()
            .position(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "WindowState"))
            .unwrap_or_else(|| unreachable!("WindowState inserted"));
        let settings = children
            .iter()
            .position(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "ElementSettings"))
            .unwrap_or_else(|| unreachable!("ElementSettings retained"));
        assert!(window < settings);
        if let Some(page_layout) = page_layout {
            let layout = children
                .iter()
                .position(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == page_layout))
                .unwrap_or_else(|| unreachable!("PageLayout retained"));
            assert!(layout < window);
        }
    }
}

#[test]
fn failed_save_as_does_not_publish_candidate_window_state_or_lineage() {
    let directory = TestDirectory::new();
    let destination = directory.path("already-exists.fdx");
    let prior_destination = b"destination remains intact";
    std::fs::write(&destination, prior_destination)
        .unwrap_or_else(|_| unreachable!("conflict destination can be prepared"));
    let opened = read(br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="doc"/><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content></FinalDraft>"#);
    let mut session = EditingSession::new(opened.project)
        .unwrap_or_else(|_| unreachable!("session initializes"));
    let original_project = session.project().clone();
    let original_revision = session.revision();
    let original_dirty = session.dirty();
    let mut target = FdxSaveTarget::new_unsaved();

    let result = save_session_with_window_state(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
        WindowState { left: 500, top: 600, width: 1000, height: 700 },
    );
    assert!(result.is_err());
    assert_eq!(session.project(), &original_project);
    assert_eq!(session.revision(), original_revision);
    assert_eq!(session.dirty(), original_dirty);
    assert!(target.path().is_none());
    assert_eq!(
        std::fs::read(destination).unwrap_or_else(|_| unreachable!("prior destination remains readable")),
        prior_destination
    );
}

#[test]
fn preservation_free_output_uses_default_window_state_after_content() {
    let project = scriptcraft_core::Project::new()
        .unwrap_or_else(|_| unreachable!("new project initializes"));
    let bytes = fdx::write_v6(&project)
        .unwrap_or_else(|_| unreachable!("new document serializes"));
    let document = read(&bytes);
    assert_eq!(document.window_state, Some(WindowState::DEFAULT));
    let children = root_children(
        &document
            .project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("new output is readable"))
            .sections,
    );
    let content = children
        .iter()
        .position(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "Content"))
        .unwrap_or_else(|| unreachable!("Content exists"));
    let window = children
        .iter()
        .position(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "WindowState"))
        .unwrap_or_else(|| unreachable!("WindowState exists"));
    let settings = children
        .iter()
        .position(|node| matches!(node, PreservedNode::Element(element) if element.name.local_name == "ElementSettings"))
        .unwrap_or_else(|| unreachable!("ElementSettings exists"));
    assert!(content < window && window < settings);
}

#[test]
fn actual_save_without_a_complete_native_snapshot_uses_the_documented_default() {
    let directory = TestDirectory::new();
    let destination = directory.path("default-window.fdx");
    let mut session = EditingSession::new(read(SIMPLE).project)
        .unwrap_or_else(|_| unreachable!("imported project initializes"));
    let mut target = FdxSaveTarget::new_unsaved();
    save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    )
    .unwrap_or_else(|_| unreachable!("Save As with default view state succeeds"));
    let saved = std::fs::read(destination)
        .unwrap_or_else(|_| unreachable!("default geometry output can be read"));
    assert_eq!(read(&saved).window_state, Some(WindowState::DEFAULT));
}
