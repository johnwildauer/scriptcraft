#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf_with_options, PdfExportOptions};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, FdxStorageError, FdxStorageErrorKind,
    FdxStorageFileSystem, StdFdxStorageFileSystem, StorageOperation, TransactionalSaveError,
};
use scriptcraft_core::formatted_text::styled_display_spans;
use scriptcraft_core::layout::{LayoutCache, LayoutComposition, ScreenplayLayout};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{LayoutInputs, ParagraphAlignment, Project, RunStyle};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("title-page integration scenario uses valid input"))
}

fn build(project: &Project, composition: LayoutComposition) -> ScreenplayLayout {
    let inputs = LayoutInputs::default();
    let source = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_default();
    valid(ScreenplayLayout::build_with_composition(
        project,
        &inputs,
        &source,
        composition,
    ))
}

fn pdf_page_count(pdf: &[u8]) -> usize {
    let text = String::from_utf8_lossy(pdf);
    let marker = " /Count ";
    let start = text
        .find(marker)
        .unwrap_or_else(|| unreachable!("PDF page tree has a count"))
        + marker.len();
    let digits = text[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    digits
        .parse()
        .unwrap_or_else(|_| unreachable!("PDF page count is numeric"))
}

fn find_managed_paragraph<'a>(
    nodes: &'a [PreservedNode],
    id: &scriptcraft_core::StableId,
) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id) {
            return Some(element);
        }
        if let Some(found) = find_managed_paragraph(&element.children, id) {
            return Some(found);
        }
    }
    None
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "injected temporary write failure"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Other, "injected temporary flush failure"))
    }
}

#[derive(Clone, Copy)]
struct FailingWriteFileSystem;

impl FdxStorageFileSystem for FailingWriteFileSystem {
    type TempFile = FailingWriter;
    type Permissions = std::fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        StdFdxStorageFileSystem.read_bounded(path, limit)
    }

    fn inspect_destination(
        &self,
        path: &Path,
    ) -> Result<Option<Self::Permissions>, FdxStorageError> {
        StdFdxStorageFileSystem.inspect_destination(path)
    }

    fn create_temp(
        &self,
        destination: &Path,
    ) -> Result<(PathBuf, Self::TempFile), FdxStorageError> {
        let (path, file) = StdFdxStorageFileSystem.create_temp(destination)?;
        drop(file);
        Ok((path, FailingWriter))
    }

    fn set_temp_permissions(
        &self,
        _temp: &mut Self::TempFile,
        _permissions: &Self::Permissions,
    ) -> Result<(), FdxStorageError> {
        Ok(())
    }

    fn sync_temp(&self, _temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
        Ok(())
    }

    fn replace(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.replace(temp, destination)
    }

    fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.install_new(temp, destination)
    }

    fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.remove_temp(temp)
    }
}

fn unique_directory(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()))
}

#[test]
fn default_document_keeps_editor_compositions_and_pdf_title_option_coherent() {
    let project = valid(Project::new());
    let before = project.clone();
    assert_eq!(project.title_page.len(), 54);
    assert_eq!(project.elements.len(), 1);
    assert!(project.elements[0].text.is_empty());

    let body = build(&project, LayoutComposition::BodyEditor);
    let title = build(&project, LayoutComposition::TitlePageEditor);
    let print_with_title = build(
        &project,
        LayoutComposition::Print {
            include_title_page: true,
        },
    );
    let print_without_title = build(
        &project,
        LayoutComposition::Print {
            include_title_page: false,
        },
    );
    assert!(body.pages.iter().all(|page| !page.title_page));
    assert!(title.pages.iter().all(|page| page.title_page));
    assert!(print_with_title.pages.first().is_some_and(|page| page.title_page));
    assert!(print_with_title.pages.iter().skip(1).all(|page| !page.title_page));
    assert!(print_without_title.pages.iter().all(|page| !page.title_page));

    let pdf_with_title = valid(export_pdf_with_options(
        &project,
        PdfExportOptions::default(),
    ));
    let pdf_without_title = valid(export_pdf_with_options(
        &project,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    assert_eq!(pdf_page_count(&pdf_with_title), print_with_title.pages.len());
    assert_eq!(pdf_page_count(&pdf_without_title), print_without_title.pages.len());
    assert!(pdf_page_count(&pdf_with_title) > pdf_page_count(&pdf_without_title));
    assert_eq!(project, before, "layout and PDF inclusion are transient projections");
}

#[test]
fn reference3_mixed_runs_and_direct_alignment_survive_session_edit_and_source_reread() {
    let imported = valid(read_v6(include_bytes!("../../../.reference/reference3.fdx")));
    let mut project = imported.project;
    assert_eq!(project.elements[1].text, "Action with BOLD, italics, and underline text styles. ");
    assert_eq!(
        project.direct_paragraph_overrides[&project.elements[1].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Center)
    );
    assert_eq!(project.elements[1].runs.len(), 7);
    let name_index = project
        .title_page
        .iter()
        .position(|paragraph| paragraph.text == "Name of First Writer")
        .unwrap_or_else(|| unreachable!("reference3 contains the mixed title placeholder"));
    assert_eq!(project.title_page[name_index].runs.len(), 6);
    assert!(project.title_page[name_index].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value == "AllCaps"
        })
    }));

    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection {
        anchor: Position { element: 1, offset: 12 },
        focus: Position { element: 1, offset: 16 },
    }));
    valid(session.toggle_body_run_style(RunStyle::Strikethrough));
    valid(session.set_body_alignment(ParagraphAlignment::Justified));
    valid(session.set_title_selection(Selection {
        anchor: Position {
            element: name_index,
            offset: "Name of ".len(),
        },
        focus: Position {
            element: name_index,
            offset: "Name of First".len(),
        },
    }));
    valid(session.toggle_title_run_style(RunStyle::Bold));
    valid(session.set_title_alignment(ParagraphAlignment::Right));

    let saved = valid(write_v6(session.project()));
    let reread = valid(read_v6(&saved));
    let saved_project = reread.project;
    assert_eq!(saved_project.elements[1].text, session.project().elements[1].text);
    assert_eq!(saved_project.elements[1].runs, session.project().elements[1].runs);
    assert_eq!(saved_project.title_page[name_index].text, "Name of First Writer");
    assert_eq!(
        saved_project.title_page[name_index].runs,
        session.project().title_page[name_index].runs
    );
    assert_eq!(
        saved_project.direct_paragraph_overrides[&saved_project.elements[1].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Justified)
    );
    assert_eq!(
        saved_project.title_page[name_index].formatting.alignment,
        Some(scriptcraft_core::TitlePageAlignment::Right)
    );
    let xml = String::from_utf8(saved).unwrap_or_else(|_| unreachable!("FDX writer emits UTF-8"));
    assert!(xml.contains("<Watermarking"));
    assert!(xml.contains("<DynamicLabel"));
    assert!(xml.contains("Style=\"Strikethrough\""));
    assert!(xml.contains("Alignment=\"Full\""));
    assert!(xml.contains("Alignment=\"Right\""));

    let title_spans = styled_display_spans(
        &saved_project.title_page[name_index].text,
        &saved_project.title_page[name_index].runs,
    )
        .unwrap_or_else(|_| unreachable!("re-read title run projection is valid"));
    assert!(title_spans.iter().any(|span| span.styles.bold));
}

#[test]
fn repeated_foreign_title_ids_remain_distinct_editable_occurrences_after_round_trip() {
    let imported = valid(read_v6(include_bytes!("../../../.reference/real_script_example.fdx")));
    let mut session = valid(EditingSession::new(imported.project));
    let repeated_id = "ae7f1040-c536-4085-aeec-464de6be3eac";
    let occurrences = session
        .project()
        .title_page
        .iter()
        .enumerate()
        .filter_map(|(index, paragraph)| {
            paragraph
                .source_provenance
                .as_ref()
                .filter(|provenance| provenance.foreign_id.as_deref() == Some(repeated_id))
                .map(|_| index)
        })
        .collect::<Vec<_>>();
    assert!(occurrences.len() > 1);
    let transient_ids = occurrences
        .iter()
        .map(|index| session.project().title_page[*index].id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(transient_ids.len(), occurrences.len());

    for (serial, index) in occurrences.iter().take(2).copied().enumerate() {
        valid(session.set_title_selection(Selection::caret(Position {
            element: index,
            offset: 0,
        })));
        valid(session.insert_title(&format!("occurrence-{serial} ")));
    }
    assert_ne!(
        session.project().title_page[occurrences[0]].text,
        session.project().title_page[occurrences[1]].text
    );
    let output = valid(write_v6(session.project()));
    let output_text = String::from_utf8(output.clone())
        .unwrap_or_else(|_| unreachable!("FDX writer emits UTF-8"));
    assert_eq!(output_text.matches(repeated_id).count(), occurrences.len());
    let reread = valid(read_v6(&output));
    assert_eq!(
        reread
            .project
            .title_page
            .iter()
            .filter(|paragraph| paragraph
                .source_provenance
                .as_ref()
                .is_some_and(|provenance| provenance.foreign_id.as_deref() == Some(repeated_id)))
            .count(),
        occurrences.len()
    );
}

#[test]
fn empty_title_typing_style_and_body_allcaps_strikethrough_justified_round_trip() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    valid(session.toggle_title_run_style(RunStyle::Bold));
    assert!(session.project().title_page[0].runs.is_empty());
    valid(session.insert_title("temporary title"));
    assert_eq!(session.project().title_page[0].text, "temporary title");
    assert!(session.project().title_page[0].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value.split('+').any(|token| token == "Bold")
        })
    }));
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position {
            element: 0,
            offset: "temporary title".len(),
        },
    }));
    valid(session.delete_title());
    assert!(session.project().title_page[0].text.is_empty());
    assert!(session.project().title_page[0].runs.is_empty());

    let body_text = "straße is a long screenplay line with enough words to wrap across the fixed page width";
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    valid(session.insert(body_text));
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position {
            element: 0,
            offset: body_text.len(),
        },
    }));
    valid(session.toggle_body_run_style(RunStyle::AllCaps));
    valid(session.toggle_body_run_style(RunStyle::Strikethrough));
    valid(session.set_body_alignment(ParagraphAlignment::Justified));
    assert_eq!(session.project().elements[0].text, body_text);
    let body_layout = build(session.project(), LayoutComposition::BodyEditor);
    let lines = body_layout.pages.iter().flat_map(|page| &page.lines).collect::<Vec<_>>();
    assert!(lines.len() > 1);
    assert!(lines.iter().any(|line| line.display_text.contains("STRASSE")));
    assert!(lines.iter().flat_map(|line| line.spans.iter()).any(|span| {
        span.text_style.all_caps && span.strikethrough
    }));
    let final_line = lines.last().unwrap_or_else(|| unreachable!("wrapped paragraph has final line"));
    assert_eq!(final_line.geometry.x, final_line.paragraph_bounds.x);

    let bytes = valid(write_v6(session.project()));
    let reread = valid(read_v6(&bytes));
    assert!(reread.project.title_page[0].text.is_empty());
    assert!(reread.project.title_page[0].runs.is_empty());
    assert_eq!(reread.project.elements[0].text, body_text);
    assert!(reread.project.elements[0].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style"
                && attribute.value.split('+').any(|token| token == "AllCaps")
                && attribute.value.split('+').any(|token| token == "Strikethrough")
        })
    }));
}

#[test]
fn default_and_edited_documents_support_save_save_as_reopen_and_save_again() {
    let directory = unique_directory("scriptcraft-native-save-round-trip");
    std::fs::create_dir(&directory)
        .unwrap_or_else(|_| unreachable!("unique native-save directory can be created"));
    let path = directory.join("round-trip.fdx");
    let mut session = valid(EditingSession::new(valid(Project::new())));

    let body = "INT. SAVE ROUND TRIP - DAY";
    valid(session.insert(body));
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: body.len() },
    }));
    valid(session.toggle_body_run_style(RunStyle::Italics));
    valid(session.set_body_alignment(ParagraphAlignment::Justified));
    let title = "Round Trip Title";
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 0,
    })));
    valid(session.set_title_selection(Selection {
        anchor: Position {
            element: 17,
            offset: 0,
        },
        focus: Position {
            element: 17,
            offset: "Script Title".len(),
        },
    }));
    valid(session.insert_title(title));
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 17, offset: 0 },
        focus: Position { element: 17, offset: title.len() },
    }));
    valid(session.toggle_title_run_style(RunStyle::Bold));
    valid(session.set_title_alignment(ParagraphAlignment::Right));

    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(path.clone()),
    ));
    assert!(!session.dirty());
    assert_eq!(target.path(), Some(path.as_path()));
    let body_id = session.project().elements[0].id.clone();
    let body_source = find_managed_paragraph(
        &session.project().preservation.as_ref().expect("Save As rebases to source").sections[0].children,
        &body_id,
    ).expect("saved body paragraph remains source-backed");
    let scene_properties = body_source.children.iter().position(|node| {
        matches!(node, PreservedNode::Element(element) if element.name.local_name == "SceneProperties")
    }).expect("the generated SceneProperties child remains preserved");
    let body_text = body_source.children.iter().position(|node| {
        matches!(node, PreservedNode::Element(element) if element.name.local_name == "Text")
    }).expect("the saved Scene Heading has its Text child");
    assert!(scene_properties < body_text);

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: body.len(),
    })));
    valid(session.insert(" revised"));
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: body.len() + " revised".len() },
    }));
    valid(session.toggle_body_run_style(RunStyle::Bold));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: title.len(),
    })));
    valid(session.insert_title(" Revised"));
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 17, offset: title.len() },
        focus: Position { element: 17, offset: title.len() + " Revised".len() },
    }));
    valid(session.toggle_title_run_style(RunStyle::Italics));
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));

    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    assert_eq!(opened.document.project.elements[0].text, "INT. SAVE ROUND TRIP - DAY revised");
    assert_eq!(opened.document.project.title_page[17].text, "Round Trip Title Revised");
    assert_eq!(
        opened.document.project.direct_paragraph_overrides
            [&opened.document.project.elements[0].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Justified)
    );
    assert_eq!(
        opened.document.project.title_page[17].formatting.alignment,
        Some(scriptcraft_core::TitlePageAlignment::Right)
    );
    let reopened_body = find_managed_paragraph(
        &opened.document.project.preservation.as_ref().expect("opened document retains source").sections[0].children,
        &body_id,
    ).expect("reopened Scene Heading remains source-backed");
    assert!(reopened_body.children.iter().any(|node| {
        matches!(node, PreservedNode::Element(element) if element.name.local_name == "SceneProperties")
    }));
    assert!(opened.document.project.elements[0].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| attribute.name == "Style" && attribute.value.contains("Bold"))
    }));
    assert!(opened.document.project.title_page[17].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| attribute.name == "Style" && attribute.value.contains("Italic"))
    }));
    let mut reopened_session = valid(EditingSession::new(opened.document.project));
    let mut reopened_target = FdxSaveTarget::from_baseline(opened.baseline);
    valid(reopened_session.set_selection(Selection::caret(Position {
        element: 0,
        offset: "INT. SAVE ROUND TRIP - DAY revised".len(),
    })));
    valid(reopened_session.insert(" after reopen"));
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut reopened_session,
        &mut reopened_target,
        FdxSaveMode::Save,
    ));
    let reopened_again = valid(open(&StdFdxStorageFileSystem, &path));
    assert_eq!(
        reopened_again.document.project.elements[0].text,
        "INT. SAVE ROUND TRIP - DAY revised after reopen"
    );
    assert!(reopened_again.document.project.title_page[17].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value.split('+').any(|token| token == "Bold")
        })
    }));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn save_as_rebase_allows_plain_body_edit_and_save_again() {
    let directory = unique_directory("scriptcraft-post-save-plain-body-edit");
    std::fs::create_dir(&directory)
        .unwrap_or_else(|_| unreachable!("unique post-save directory can be created"));
    let path = directory.join("plain-body.fdx");
    let mut project = valid(Project::new());
    project.elements[0].role = scriptcraft_core::Role::Action;
    assert!(project.preservation.is_none());
    let mut session = valid(EditingSession::new(project));
    valid(session.insert("plain body"));
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(path.clone()),
    ));
    let body_id = session.project().elements[0].id.clone();
    let paragraph = find_managed_paragraph(
        &session.project().preservation.as_ref().expect("Save As rebases source").sections[0].children,
        &body_id,
    ).expect("saved plain body paragraph remains source-backed");
    assert_eq!(paragraph.children.iter().position(|node| {
        matches!(node, PreservedNode::Element(element) if element.name.local_name == "Text")
    }), Some(0));

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: "plain body".len(),
    })));
    valid(session.insert(" edited"));
    assert!(session.dirty());
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    assert!(!session.dirty());
    let reopened = valid(open(&StdFdxStorageFileSystem, &path));
    assert_eq!(reopened.document.project.elements[0].text, "plain body edited");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn codec_candidate_failure_leaves_save_as_destination_and_live_session_unchanged() {
    let directory = unique_directory("scriptcraft-native-save-candidate-failure");
    std::fs::create_dir(&directory)
        .unwrap_or_else(|_| unreachable!("unique candidate-failure directory can be created"));
    let path = directory.join("must-not-be-created.fdx");
    let mut project = valid(Project::new());
    project.title_page[0].formatting.leading_regular = Some(false);
    let mut session = valid(EditingSession::new(project));
    let before = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();
    let before_target = target.clone();

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(path.clone()),
    )
    .err()
    .unwrap_or_else(|| unreachable!("unsupported candidate formatting must stop before publish"));
    assert!(matches!(
        error,
        TransactionalSaveError::Codec(scriptcraft_core::fdx::FdxError::UnsupportedConstruct)
    ));
    assert!(!path.exists());
    assert_eq!(session, before);
    assert_eq!(target, before_target);
    assert!(!session.dirty());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn saved_rebase_invalidates_layout_identity_and_keeps_nondefault_surface_selections_editable() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 0,
    })));
    valid(session.insert_title("Draft: "));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    valid(session.insert("INT. SHARED SESSION - DAY"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 5,
    })));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 4,
    })));
    let title_selection = session.title_selection();
    let body_selection = session.selection();
    let revision_before = session.revision();

    let mut cache = LayoutCache::default();
    let inputs = LayoutInputs::default();
    let (_, body_first_hit) = valid(cache.get_or_build_with_composition(
        revision_before,
        session.project(),
        &inputs,
        LayoutComposition::BodyEditor,
    ));
    assert!(!body_first_hit);
    let (_, title_first_hit) = valid(cache.get_or_build_with_composition(
        revision_before,
        session.project(),
        &inputs,
        LayoutComposition::TitlePageEditor,
    ));
    assert!(!title_first_hit);

    let directory = unique_directory("scriptcraft-live-rebase");
    std::fs::create_dir(&directory)
        .unwrap_or_else(|_| unreachable!("unique rebase directory can be created"));
    let path = directory.join("rebase.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(path.clone()),
    ));
    assert!(!session.dirty());
    assert!(session.revision() > revision_before);
    assert_eq!(target.path(), Some(path.as_path()));
    assert_eq!(session.title_selection(), title_selection);
    assert_eq!(session.selection(), body_selection);
    let (_, body_rebase_hit) = valid(cache.get_or_build_with_composition(
        session.revision(),
        session.project(),
        &inputs,
        LayoutComposition::BodyEditor,
    ));
    let (_, title_rebase_hit) = valid(cache.get_or_build_with_composition(
        session.revision(),
        session.project(),
        &inputs,
        LayoutComposition::TitlePageEditor,
    ));
    assert!(!body_rebase_hit);
    assert!(!title_rebase_hit);

    valid(session.insert_title("live"));
    assert!(session.dirty());
    assert!(session.project().title_page[17].text.starts_with("Draft: live"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 5,
    })));
    valid(session.insert(" BODY"));
    assert!(session.project().elements[0].text.contains("BODY"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn storage_write_failure_preserves_existing_destination_target_and_both_surface_state() {
    let directory = unique_directory("scriptcraft-title-storage-failure");
    std::fs::create_dir(&directory)
        .unwrap_or_else(|_| unreachable!("unique storage-failure directory can be created"));
    let path = directory.join("existing.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    std::fs::write(&path, &initial)
        .unwrap_or_else(|_| unreachable!("initial valid FDX can be written"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    valid(session.insert("unsaved body"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 4,
    })));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 0,
    })));
    valid(session.insert_title("unsaved title"));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 6,
    })));
    let before = session.clone();
    let before_target = target.clone();

    let error = save_session(
        &FailingWriteFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("injected temporary write must fail"));
    assert!(matches!(
        error,
        TransactionalSaveError::Storage(ref storage)
            if storage.kind == FdxStorageErrorKind::Io(StorageOperation::WriteTemporary)
    ));
    assert_eq!(std::fs::read(&path).ok().as_deref(), Some(initial.as_slice()));
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(target.path(), before_target.path());
    assert_eq!(target.baseline(), before_target.baseline());
    let _ = std::fs::remove_dir_all(directory);
}
