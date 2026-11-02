#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6, FdxError};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{Project, Role, ScreenplayElement, StableId};

const REFERENCE: &[u8] = include_bytes!("../../../.reference/reference2.fdx");
const REFERENCE_BLANK_ACTION_ID: &str = "7403381a-7dc5-498a-8ce0-da5ca4909a79";

fn managed_paragraph<'a>(project: &'a Project, id: &StableId) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &StableId) -> Option<&'a PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some(element);
            }
            if let Some(found) = find(&element.children, id) {
                return Some(found);
            }
        }
        None
    }

    let state = project.preservation.as_ref()?;
    for section in &state.sections {
        if let Some(paragraph) = find(&section.children, id) {
            return Some(paragraph);
        }
    }
    None
}

fn direct_text_count(paragraph: &PreservedElement) -> usize {
    paragraph
        .children
        .iter()
        .filter(|node| {
            matches!(node,
                PreservedNode::Element(element)
                    if element.name.local_name == "Text"
                        && element.name.namespace_uri == paragraph.name.namespace_uri)
        })
        .count()
}

fn reference_blank_index(project: &Project) -> usize {
    project
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some(REFERENCE_BLANK_ACTION_ID)
        })
        .unwrap_or_else(|| unreachable!("reference blank Action has its established source ID"))
}

fn caret(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element, offset }))
        .unwrap_or_else(|_| unreachable!("test caret is within canonical text"));
}

#[test]
fn reference_blank_action_survives_serialization_reopen_and_second_serialization() {
    let imported = read_v6(REFERENCE)
        .unwrap_or_else(|_| unreachable!("checked-in Final Draft reference imports"))
        .project;
    let index = reference_blank_index(&imported);
    assert!(imported.elements[index].text.is_empty());
    assert!(imported.elements[index].runs.is_empty());

    let first = write_v6(&imported)
        .unwrap_or_else(|_| unreachable!("reference blank Action serializes"));
    let reopened = read_v6(&first)
        .unwrap_or_else(|_| unreachable!("serialized reference reopens"))
        .project;
    let reopened_index = reference_blank_index(&reopened);
    let paragraph = managed_paragraph(&reopened, &reopened.elements[reopened_index].id)
        .unwrap_or_else(|| unreachable!("reference blank remains managed"));
    assert_eq!(direct_text_count(paragraph), 0);

    let second = write_v6(&reopened)
        .unwrap_or_else(|_| unreachable!("reopened reference serializes again"));
    let reopened_again = read_v6(&second)
        .unwrap_or_else(|_| unreachable!("second serialization reopens"))
        .project;
    let final_index = reference_blank_index(&reopened_again);
    let paragraph = managed_paragraph(&reopened_again, &reopened_again.elements[final_index].id)
        .unwrap_or_else(|| unreachable!("reopened blank remains managed"));
    assert_eq!(direct_text_count(paragraph), 0);
}

#[test]
fn imported_empty_text_runs_project_empty_and_are_removed_without_losing_attachments() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="empty" Marker="keep"><Text Style="Bold"></Text><VendorAttachment Value="keep"><Text Control="opaque"/><![CDATA[]]><!----><?keep value?><FutureChild/></VendorAttachment></Paragraph></Content></FinalDraft>"#;
    let imported = read_v6(source)
        .unwrap_or_else(|_| unreachable!("empty Text element imports"))
        .project;
    assert!(imported.elements[0].text.is_empty());
    assert!(imported.elements[0].runs.is_empty());

    let bytes = write_v6(&imported)
        .unwrap_or_else(|_| unreachable!("empty imported Text element reconciles"));
    let output = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(output.contains("Marker=\"keep\""));
    assert!(output.contains("VendorAttachment Value=\"keep\""));
    assert!(output.contains("<Text Control=\"opaque\""));
    assert!(output.contains("<![CDATA[]]>"));
    assert!(output.contains("<!---->"));
    assert!(output.contains("<?keep value?>"));
    assert!(output.contains("<FutureChild"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("empty managed paragraph reopens"))
        .project;
    let paragraph = managed_paragraph(&reopened, &reopened.elements[0].id)
        .unwrap_or_else(|| unreachable!("imported paragraph remains managed"));
    assert_eq!(direct_text_count(paragraph), 0);
    assert!(paragraph
        .attributes
        .iter()
        .any(|attribute| attribute.name.local_name == "Marker" && attribute.value == "keep"));
    assert!(paragraph.children.iter().any(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "VendorAttachment")));
}

#[test]
fn generated_blank_body_and_direct_title_role_paragraphs_have_no_text_children() {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!("empty project is valid"));
    project.elements.push(
        ScreenplayElement::new("blank-body", Role::Action, "")
            .unwrap_or_else(|_| unreachable!("blank body paragraph is valid")),
    );
    project.title_page.push(
        ScreenplayElement::new("blank-title", Role::Action, "")
            .unwrap_or_else(|_| unreachable!("blank title-role paragraph is valid")),
    );

    let bytes = write_v6(&project)
        .unwrap_or_else(|_| unreachable!("generated blank paragraphs serialize"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("generated blank paragraphs reopen"))
        .project;
    assert_eq!(reopened.elements.len(), 1);
    assert_eq!(reopened.title_page.len(), 1);
    assert!(reopened.elements[0].text.is_empty());
    assert!(reopened.elements[0].runs.is_empty());
    assert!(reopened.title_page[0].text.is_empty());
    assert!(reopened.title_page[0].runs.is_empty());
    let output = std::str::from_utf8(&bytes).unwrap_or("");
    assert_eq!(output.matches("<Text").count(), 0);
}

#[test]
fn no_text_paragraph_supports_type_clear_undo_redo_and_repeated_save_projection() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="blank" Marker="keep"/></Content></FinalDraft>"#;
    let project = read_v6(source)
        .unwrap_or_else(|_| unreachable!("native empty Paragraph imports"))
        .project;
    let mut session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("empty imported project forms a session"));
    let original = session.project().clone();

    caret(&mut session, 0, 0);
    session
        .insert("typed")
        .unwrap_or_else(|_| unreachable!("typing into childless Paragraph succeeds"));
    assert_eq!(session.project().elements[0].text, "typed");
    let typed_bytes = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("typed childless Paragraph saves"));
    let typed_reopen = read_v6(&typed_bytes)
        .unwrap_or_else(|_| unreachable!("typed Paragraph reopens"))
        .project;
    assert_eq!(typed_reopen.elements[0].text, "typed");
    assert_eq!(direct_text_count(
        managed_paragraph(&typed_reopen, &typed_reopen.elements[0].id)
            .unwrap_or_else(|| unreachable!("typed Paragraph remains managed"))
    ), 1);

    let typed_snapshot = session.project().clone();
    session.undo().unwrap_or_else(|_| unreachable!("insert undo succeeds"));
    assert_eq!(session.project(), &original);
    assert!(session.project().elements[0].runs.is_empty());
    assert_eq!(direct_text_count(
        managed_paragraph(&session.project(), &session.project().elements[0].id)
            .unwrap_or_else(|| unreachable!("undo restores source paragraph"))
    ), 0);
    session.redo().unwrap_or_else(|_| unreachable!("insert redo succeeds"));
    assert_eq!(session.project(), &typed_snapshot);

    session
        .set_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: 5 },
        })
        .unwrap_or_else(|_| unreachable!("typed range is valid"));
    session
        .delete()
        .unwrap_or_else(|_| unreachable!("clearing managed text succeeds"));
    assert!(session.project().elements[0].text.is_empty());
    assert!(session.project().elements[0].runs.is_empty());
    let empty_snapshot = session.project().clone();
    let paragraph = managed_paragraph(&session.project(), &session.project().elements[0].id)
        .unwrap_or_else(|| unreachable!("cleared Paragraph remains managed"));
    assert_eq!(direct_text_count(paragraph), 0);
    assert!(paragraph
        .attributes
        .iter()
        .any(|attribute| attribute.name.local_name == "Marker" && attribute.value == "keep"));

    let empty_bytes = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("cleared paragraph saves"));
    let reopened = read_v6(&empty_bytes)
        .unwrap_or_else(|_| unreachable!("cleared paragraph reopens"))
        .project;
    assert!(reopened.elements[0].text.is_empty());
    assert!(reopened.elements[0].runs.is_empty());
    let second = write_v6(&reopened)
        .unwrap_or_else(|_| unreachable!("reopened cleared paragraph saves again"));
    let reopened_again = read_v6(&second)
        .unwrap_or_else(|_| unreachable!("second cleared save reopens"))
        .project;
    assert_eq!(reopened_again.elements[0].text, "");
    assert_eq!(direct_text_count(
        managed_paragraph(&reopened_again, &reopened_again.elements[0].id)
            .unwrap_or_else(|| unreachable!("second-save Paragraph remains managed"))
    ), 0);

    session.undo().unwrap_or_else(|_| unreachable!("clear undo succeeds"));
    assert_eq!(session.project(), &typed_snapshot);
    session.redo().unwrap_or_else(|_| unreachable!("clear redo succeeds"));
    assert_eq!(session.project(), &empty_snapshot);
}

#[test]
fn clearing_multi_run_text_keeps_semantic_and_managed_source_empty() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="multi"><Text Style="Bold">one</Text><Text Style="Italic">two</Text></Paragraph></Content></FinalDraft>"#;
    let project = read_v6(source)
        .unwrap_or_else(|_| unreachable!("multi-run paragraph imports"))
        .project;
    let mut session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("multi-run project forms a session"));
    session
        .set_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: 6 },
        })
        .unwrap_or_else(|_| unreachable!("complete multi-run range is valid"));
    session
        .delete()
        .unwrap_or_else(|_| unreachable!("multi-run clear is supported"));
    assert!(session.project().elements[0].text.is_empty());
    assert!(session.project().elements[0].runs.is_empty());
    let bytes = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("cleared multi-run paragraph serializes"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("cleared multi-run paragraph reopens"))
        .project;
    assert!(reopened.elements[0].text.is_empty());
    assert!(reopened.elements[0].runs.is_empty());
    let paragraph = managed_paragraph(&reopened, &reopened.elements[0].id)
        .unwrap_or_else(|| unreachable!("multi-run Paragraph remains managed"));
    assert_eq!(direct_text_count(paragraph), 0);
}

#[test]
fn empty_text_nodes_are_ignored_only_by_strict_tree_comparison() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="text-node"><Text>canonical</Text></Paragraph></Content></FinalDraft>"#;
    let mut project = read_v6(source)
        .unwrap_or_else(|_| unreachable!("comparison fixture imports"))
        .project;
    let id = project.elements[0].id.clone();
    let paragraph = managed_paragraph_mut(&mut project, &id)
        .unwrap_or_else(|| unreachable!("managed source Paragraph exists"));
    let text = paragraph
        .children
        .iter_mut()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("managed Text element exists"));
    text.children.push(PreservedNode::Text(String::new()));

    let bytes = write_v6(&project)
        .unwrap_or_else(|_| unreachable!("serializer-invisible empty text is normalized"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("normalized comparison output reopens"))
        .project;
    assert_eq!(reopened.elements[0].text, "canonical");
}

fn managed_paragraph_mut<'a>(project: &'a mut Project, id: &StableId) -> Option<&'a mut PreservedElement> {
    fn find<'a>(nodes: &'a mut [PreservedNode], id: &StableId) -> Option<&'a mut PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some(element);
            }
            if let Some(found) = find(&mut element.children, id) {
                return Some(found);
            }
        }
        None
    }

    let state = project.preservation.as_mut()?;
    for section in &mut state.sections {
        if let Some(paragraph) = find(&mut section.children, id) {
            return Some(paragraph);
        }
    }
    None
}

#[test]
fn edits_with_parser_text_and_cdata_retain_canonical_content_after_reopen() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="mixed"><Text>left<![CDATA[middle]]>right</Text></Paragraph></Content></FinalDraft>"#;
    let project = read_v6(source)
        .unwrap_or_else(|_| unreachable!("text plus CDATA imports"))
        .project;
    let mut session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("mixed text project forms a session"));
    assert_eq!(session.project().elements[0].text, "leftmiddleright");
    caret(&mut session, 0, 4);
    session
        .insert("X")
        .unwrap_or_else(|_| unreachable!("edit at parser text boundary succeeds"));
    let bytes = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("edited text and CDATA serialize"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("edited text and CDATA reopen"))
        .project;
    assert_eq!(reopened.elements[0].text, "leftXmiddleright");
}

#[test]
fn whitespace_only_managed_content_keeps_a_text_run() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="spaces"><Text>   </Text></Paragraph></Content></FinalDraft>"#;
    let project = read_v6(source)
        .unwrap_or_else(|_| unreachable!("whitespace-only paragraph imports"))
        .project;
    assert_eq!(project.elements[0].text, "   ");
    let bytes = write_v6(&project)
        .unwrap_or_else(|_| unreachable!("whitespace-only paragraph serializes"));
    let output = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(output.contains("<Text>   </Text>"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("whitespace-only paragraph reopens"))
        .project;
    assert_eq!(reopened.elements[0].text, "   ");
}

#[test]
fn managed_run_mismatches_and_opaque_inline_content_remain_fatal() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="strict"><Text Style="Bold">source</Text></Paragraph></Content></FinalDraft>"#;
    let mut mismatched = read_v6(source)
        .unwrap_or_else(|_| unreachable!("strict mapping fixture imports"))
        .project;
    mismatched.elements[0].runs[0].attributes[0].value = "Italic".into();
    assert_eq!(
        write_v6(&mismatched),
        Err(FdxError::SaveDiagnosticUnsupportedConstruct(17))
    );

    let mut opaque_inline = read_v6(source)
        .unwrap_or_else(|_| unreachable!("opaque mapping fixture imports"))
        .project;
    let id = opaque_inline.elements[0].id.clone();
    let paragraph = managed_paragraph_mut(&mut opaque_inline, &id)
        .unwrap_or_else(|| unreachable!("opaque test Paragraph remains managed"));
    let text = paragraph
        .children
        .iter_mut()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("opaque test Text element exists"));
    text.children
        .push(PreservedNode::Comment("retained opaque inline".into()));
    assert_eq!(
        write_v6(&opaque_inline),
        Err(FdxError::SaveDiagnosticUnsupportedConstruct(17))
    );
}
