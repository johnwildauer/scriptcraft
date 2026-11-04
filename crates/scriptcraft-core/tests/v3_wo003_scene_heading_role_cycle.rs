use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, PreservationError, SourceOwnership};
use scriptcraft_core::{Project, Role};

const EMPTY_GENERATED_HEADING: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" ID="heading"><SceneProperties Length="1/8" Page="1" Title=""><SceneArcBeats/></SceneProperties></Paragraph>
</Content></FinalDraft>"#;

type RunProjection = (String, Vec<(String, String)>);

fn managed_paragraph<'a>(project: &'a Project, id: &scriptcraft_core::StableId) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &scriptcraft_core::StableId) -> Option<&'a PreservedElement> {
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

    project
        .preservation
        .as_ref()?
        .sections
        .iter()
        .find_map(|section| find(&section.children, id))
}

fn source_runs(paragraph: &PreservedElement) -> Vec<RunProjection> {
    paragraph
        .children
        .iter()
        .filter_map(|node| {
            let PreservedNode::Element(text) = node else {
                return None;
            };
            if text.name.local_name != "Text"
                || text.name.namespace_uri != paragraph.name.namespace_uri
            {
                return None;
            }
            let value = text
                .children
                .iter()
                .filter_map(|child| match child {
                    PreservedNode::Text(value) | PreservedNode::CData(value) => {
                        Some(value.as_str())
                    }
                    _ => None,
                })
                .collect::<String>();
            let attributes = text
                .attributes
                .iter()
                .map(|attribute| {
                    (
                        attribute
                            .name
                            .qualified_name
                            .clone()
                            .unwrap_or_else(|| attribute.name.local_name.clone()),
                        attribute.value.clone(),
                    )
                })
                .collect();
            Some((value, attributes))
        })
        .filter(|(text, _)| !text.is_empty())
        .collect()
}

fn semantic_runs(element: &scriptcraft_core::ScreenplayElement) -> Vec<RunProjection> {
    if element.runs.is_empty() {
        return if element.text.is_empty() {
            Vec::new()
        } else {
            vec![(element.text.clone(), Vec::new())]
        };
    }
    element
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .map(|run| {
            (
                run.text.clone(),
                run.attributes
                    .iter()
                    .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
                    .collect(),
            )
        })
        .collect()
}

fn assert_semantic_source_synchronized(project: &Project, step: &str) {
    for element in &project.elements {
        let semantic_projection = semantic_runs(element);
        let semantic_text_from_runs = semantic_projection
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();
        assert_eq!(
            element.text, semantic_text_from_runs,
            "semantic text/run projection diverged after {step}"
        );

        let paragraph = managed_paragraph(project, &element.id)
            .unwrap_or_else(|| unreachable!("each fixture and inserted paragraph remains managed"));
        let retained_runs = source_runs(paragraph);
        let retained_text = retained_runs
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();
        assert_eq!(
            element.text, retained_text,
            "semantic/source text diverged after {step}"
        );
        assert_eq!(
            semantic_projection, retained_runs,
            "semantic/source run order or formatting diverged after {step}"
        );
    }
}

fn assert_scene_containers_retained(project: &Project, id: &scriptcraft_core::StableId) {
    let paragraph = managed_paragraph(project, id)
        .unwrap_or_else(|| unreachable!("Scene Heading remains source-managed"));
    let properties = paragraph.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
            Some(element)
        }
        _ => None,
    });
    let properties = properties
        .unwrap_or_else(|| unreachable!("generated SceneProperties remains attached"));
    assert!(properties.children.iter().any(|node| matches!(
        node,
        PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats"
    )));
}

fn assert_scene_heading_has_element_only_formatting_whitespace(
    project: &Project,
    id: &scriptcraft_core::StableId,
) {
    let paragraph = managed_paragraph(project, id)
        .unwrap_or_else(|| unreachable!("Scene Heading remains source-managed"));
    assert!(paragraph.children.iter().any(|node| matches!(
        node,
        PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty()
    )), "generated SceneProperties formatting is retained as direct whitespace, not screenplay text");
}

#[test]
fn generated_empty_heading_enter_tab_cycle_refresh_and_insert_stay_synchronized() {
    let project = read_v6(EMPTY_GENERATED_HEADING)
        .unwrap_or_else(|_| unreachable!("generated empty-heading fixture imports"))
        .project;
    assert_eq!(project.elements.len(), 1);
    assert_eq!(project.elements[0].role, Role::SceneHeading);
    assert!(project.elements[0].text.is_empty());
    assert!(project.elements[0].runs.is_empty());

    let mut session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("generated empty-heading fixture forms a session"));
    session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("initial derived refresh accepts the generated empty heading"));
    assert_semantic_source_synchronized(session.project(), "initial derived refresh");

    session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!("empty Scene Heading start is a valid caret"));
    assert_semantic_source_synchronized(session.project(), "starting caret selection");

    // The minimal role cycle is Enter (Scene Heading -> new Action), then Tab:
    // Action -> Character -> Transition -> Scene Heading on that inserted paragraph.
    assert_eq!(session.enter(), Ok(()));
    assert_semantic_source_synchronized(session.project(), "Enter split from empty Scene Heading");
    assert_eq!(session.project().elements.len(), 2);
    assert_eq!(session.project().elements[1].role, Role::Action);

    assert_eq!(session.tab(), Ok(()));
    assert_semantic_source_synchronized(session.project(), "Tab Action -> Character");
    assert_eq!(session.project().elements[1].role, Role::Character);

    assert_eq!(session.tab(), Ok(()));
    assert_semantic_source_synchronized(session.project(), "Tab Character -> Transition");
    assert_eq!(session.project().elements[1].role, Role::Transition);

    assert_eq!(session.tab(), Ok(()));
    assert_semantic_source_synchronized(session.project(), "Tab Transition -> Scene Heading");
    assert_eq!(session.project().elements[1].role, Role::SceneHeading);

    session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("derived refresh accepts the cycled Scene Heading"));
    assert_semantic_source_synchronized(session.project(), "derived refresh after role cycle");
    let cycled_heading_id = session.project().elements[1].id.clone();
    let cycled_foreign_id = session.project().elements[1]
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_id.clone())
        .unwrap_or_else(|| unreachable!("cycled heading has a source paragraph identity"));
    assert_scene_containers_retained(session.project(), &cycled_heading_id);

    // Generated SceneProperties/SceneArcBeats remain a structural role-change
    // guard. Preserve that policy and prove the rejected candidate rolls back exactly.
    let before_rejection = session.clone();
    // Probe the fixture's original generated heading, whose app-generated
    // scene containers are present independently of derived output for the
    // newly cycled paragraph.
    session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!("original empty Scene Heading start remains a valid caret"));
    assert_semantic_source_synchronized(session.project(), "caret moved to original generated Scene Heading");
    let before_rejection = session.clone();
    assert_eq!(
        session.change_role(Role::Action),
        Err(EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(session, before_rejection, "rejected role change must roll back exactly");
    assert_semantic_source_synchronized(session.project(), "rejected Scene Heading -> Action role change");

    session
        .set_selection(Selection::caret(Position {
            element: 1,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!("cycled empty Scene Heading start is a valid caret"));
    assert_semantic_source_synchronized(session.project(), "caret restored to cycled Scene Heading");
    assert_scene_heading_has_element_only_formatting_whitespace(
        session.project(),
        &cycled_heading_id,
    );
    assert_eq!(session.insert("INT. ROOM - DAY"), Ok(()));
    assert_semantic_source_synchronized(session.project(), "later insertion into cycled Scene Heading");
    assert_scene_heading_has_element_only_formatting_whitespace(
        session.project(),
        &cycled_heading_id,
    );
    assert_eq!(session.project().elements[1].text, "INT. ROOM - DAY");

    let cycled_heading_end = session.project().elements[1].text.len();
    session
        .set_selection(Selection::caret(Position {
            element: 1,
            offset: cycled_heading_end,
        }))
        .unwrap_or_else(|_| unreachable!("cycled heading end remains a valid caret"));
    assert_eq!(session.insert("!"), Ok(()));
    assert_semantic_source_synchronized(session.project(), "subsequent typing");
    assert_scene_containers_retained(session.project(), &cycled_heading_id);

    session
        .undo()
        .unwrap_or_else(|_| unreachable!("subsequent typing has an undo snapshot"));
    assert_semantic_source_synchronized(session.project(), "undo subsequent typing");
    assert_eq!(session.project().elements[1].text, "INT. ROOM - DAY");
    session
        .redo()
        .unwrap_or_else(|_| unreachable!("subsequent typing has a redo snapshot"));
    assert_semantic_source_synchronized(session.project(), "redo subsequent typing");
    assert_eq!(session.project().elements[1].text, "INT. ROOM - DAY!");

    let selected_text_end = session.project().elements[1].text.len();
    session
        .set_selection(Selection {
            anchor: Position {
                element: 1,
                offset: 0,
            },
            focus: Position {
                element: 1,
                offset: selected_text_end,
            },
        })
        .unwrap_or_else(|_| unreachable!("complete cycled heading text selection is valid"));
    assert_eq!(session.insert(""), Ok(()));
    assert_semantic_source_synchronized(session.project(), "clear cycled heading");
    assert!(session.project().elements[1].text.is_empty());
    assert_scene_containers_retained(session.project(), &cycled_heading_id);

    assert_eq!(session.insert("INT. ROOM - NIGHT"), Ok(()));
    assert_semantic_source_synchronized(session.project(), "retype cycled heading");
    assert_scene_containers_retained(session.project(), &cycled_heading_id);
    session
        .undo()
        .unwrap_or_else(|_| unreachable!("retyping has an undo snapshot"));
    assert_semantic_source_synchronized(session.project(), "undo retyping");
    assert!(session.project().elements[1].text.is_empty());
    session
        .redo()
        .unwrap_or_else(|_| unreachable!("retyping has a redo snapshot"));
    assert_semantic_source_synchronized(session.project(), "redo retyping");
    assert_eq!(session.project().elements[1].text, "INT. ROOM - NIGHT");

    let serialized = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("retyped cycled Scene Heading serializes"));
    let reopened = read_v6(&serialized)
        .unwrap_or_else(|_| unreachable!("retyped cycled Scene Heading reopens"))
        .project;
    assert_semantic_source_synchronized(&reopened, "FDX serialization/reopen");
    assert_eq!(reopened.elements[1].role, Role::SceneHeading);
    assert_eq!(reopened.elements[1].text, "INT. ROOM - NIGHT");
    assert_eq!(
        reopened.elements[1]
            .source_provenance
            .as_ref()
            .and_then(|provenance| provenance.foreign_id.as_deref()),
        Some(cycled_foreign_id.as_str())
    );
    assert_scene_containers_retained(&reopened, &reopened.elements[1].id);
}
