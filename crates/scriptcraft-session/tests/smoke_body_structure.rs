#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::formatting::{
    ParagraphAlignment, SourceParagraphOverrides, SourceParagraphSpec,
};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, Position, Project, RunAttribute, ScreenplayElement, Selection,
    TextRun,
};
use scriptcraft_session::editing::EditingSession;

const OPEN_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/open_element_settings.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("body-structure source uses valid bounded input"))
}

fn source_paragraph<'a>(
    element: &'a PreservedElement,
    foreign_id: &str,
) -> Option<&'a PreservedElement> {
    if element.name.local_name == "Paragraph"
        && element.attributes.iter().any(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
                && attribute.value == foreign_id
        })
    {
        return Some(element);
    }
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child) => source_paragraph(child, foreign_id),
        _ => None,
    })
}

fn retained_paragraph<'a>(project: &'a Project, foreign_id: &str) -> &'a PreservedElement {
    project
        .preservation
        .as_ref()
        .and_then(|preservation| {
            preservation.sections.iter().find_map(|section| {
                section.children.iter().find_map(|node| match node {
                    PreservedNode::Element(element) => source_paragraph(element, foreign_id),
                    _ => None,
                })
            })
        })
        .unwrap_or_else(|| unreachable!("managed paragraph remains in retained source"))
}

#[test]
fn source_backed_arbitrary_split_join_keeps_owner_identity_and_attachments_left_biased() {
    let project = valid(read_v6(OPEN_FIXTURE)).project;
    let index = project
        .elements
        .iter()
        .position(|owner| {
            owner
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-body-general-derived")
        })
        .unwrap_or_else(|| unreachable!("fixture paragraph is a managed arbitrary type"));
    let original = project.elements[index].as_element().clone();
    let original_element_count = project.elements.len();
    let original_opaque_children = retained_paragraph(&project, "open-body-general-derived")
        .children
        .iter()
        .filter(|node| !matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text"))
        .cloned()
        .collect::<Vec<_>>();
    let original_overrides = project.direct_paragraph_overrides.clone();
    let original_key = original.element_type();
    let mut session = valid(EditingSession::new(project));

    let split_offset = "General-".len();
    valid(session.set_selection(Selection::caret(Position {
        element: index,
        offset: split_offset,
    })));
    valid(session.split());

    let left = session.project().elements[index].as_element();
    let right = session.project().elements[index + 1].as_element();
    assert_eq!(left.element_type(), original_key);
    assert_eq!(right.element_type(), original_key);
    assert_eq!(left.id, original.id);
    assert_ne!(right.id, original.id);
    assert_eq!(left.text, "General-");
    assert_eq!(right.text, "derived body");
    assert_eq!(
        session.project().direct_paragraph_overrides,
        original_overrides
    );
    let left_source = retained_paragraph(session.project(), "open-body-general-derived");
    let left_opaque_children = left_source
        .children
        .iter()
        .filter(|node| !matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text"))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(left_opaque_children, original_opaque_children);
    assert!(left_source
        .children
        .iter()
        .any(|node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "UserParagraphData")));
    assert!(session.project().elements[index + 1]
        .source_provenance
        .as_ref()
        .is_some_and(|source| source.foreign_id.is_some()));
    let split_right_id = session.project().elements[index + 1].id.clone();
    let right_foreign_id = session.project().elements[index + 1]
        .source_provenance
        .as_ref()
        .and_then(|source| source.foreign_id.as_deref())
        .unwrap_or_else(|| unreachable!("split allocates a source identity for the new owner"));
    let right_source = retained_paragraph(session.project(), right_foreign_id);
    assert!(right_source.children.iter().all(
        |node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text")
    ));

    valid(session.set_selection(Selection::caret(Position {
        element: index,
        offset: split_offset,
    })));
    valid(session.join_with_next());
    let joined = session.project().elements[index].as_element();
    assert_eq!(joined.element_type(), original_key);
    assert_eq!(joined.id, original.id);
    assert_eq!(joined.text, original.text);
    assert_eq!(
        session.project().direct_paragraph_overrides,
        original_overrides
    );
    assert_eq!(session.project().elements.len(), original_element_count);
    assert!(session
        .project()
        .elements
        .iter()
        .all(|element| element.id != split_right_id));
    assert!(retained_paragraph(session.project(), "open-body-general-derived")
        .children
        .iter()
        .any(|node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "UserParagraphData")));
}

fn custom_open_key_project() -> (Project, ElementTypeKey, Vec<TextRun>) {
    let template = valid(scriptcraft_fdx::instantiate_builtin_template(
        scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
    ));
    let mut project = valid(Project::empty_projection());
    valid(
        project.replace_element_definitions(
            template
                .project
                .element_definitions()
                .iter()
                .cloned()
                .collect(),
        ),
    );
    let custom_key = valid(ElementTypeKey::new("Custom prose"));
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut custom_definition = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("Project includes the required Action definition"));
    custom_definition.key = custom_key.clone();
    definitions.push(custom_definition);
    valid(project.replace_element_definitions(definitions));

    let runs = vec![
        TextRun {
            text: "Café 🎬 ".to_owned(),
            attributes: vec![RunAttribute {
                name: "Style".to_owned(),
                value: "Bold".to_owned(),
            }],
        },
        TextRun {
            text: "λ — end".to_owned(),
            attributes: vec![RunAttribute {
                name: "Style".to_owned(),
                value: "Italic".to_owned(),
            }],
        },
    ];
    let element = valid(ScreenplayElement::with_runs(
        "custom-open-key",
        custom_key.clone(),
        runs.clone(),
        None,
    ));
    project.elements = vec![BodyOwner::shared(element)];
    project.direct_paragraph_overrides.insert(
        valid(scriptcraft_model::StableId::new("custom-open-key")),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                alignment: Some(ParagraphAlignment::Center),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: Some(false),
        },
    );
    (project, custom_key, runs)
}

#[test]
fn preservation_free_open_key_split_join_type_change_and_mismatch_are_transactional() {
    let (project, custom_key, original_runs) = custom_open_key_project();
    let original = project.elements[0].as_element().clone();
    let original_overrides = project.direct_paragraph_overrides.clone();
    let split_offset = "Café 🎬 ".len();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: split_offset,
    })));

    valid(session.split());
    assert_eq!(session.project().elements[0].element_type(), custom_key);
    assert_eq!(session.project().elements[1].element_type(), custom_key);
    assert_eq!(session.project().elements[0].as_element().id, original.id);
    assert_ne!(session.project().elements[1].as_element().id, original.id);
    assert_eq!(
        session.project().elements[0].as_element().runs,
        vec![original_runs[0].clone()]
    );
    assert_eq!(
        session.project().elements[1].as_element().runs,
        vec![original_runs[1].clone()]
    );
    assert_eq!(
        session.project().direct_paragraph_overrides,
        original_overrides
    );

    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    let type_change_before = session.clone();
    let action_key = valid(ElementTypeKey::new("Action"));
    valid(session.change_element_type(action_key));
    assert_eq!(
        session.project().elements[1].as_element().text,
        original_runs[1].text
    );
    assert_eq!(
        session.project().elements[1].as_element().runs,
        original_runs[1..].to_vec()
    );
    assert_eq!(
        session.project().elements[1].as_element().id,
        type_change_before.project().elements[1].as_element().id
    );

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: split_offset,
    })));
    let before_mismatched_join = session.clone();
    assert!(session.join_with_next().is_err());
    assert_eq!(session, before_mismatched_join);

    valid(session.undo());
    assert_eq!(session.project().elements[1].element_type(), custom_key);
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: split_offset,
    })));
    valid(session.join_with_next());
    let joined = session.project().elements[0].as_element();
    assert_eq!(joined.element_type(), custom_key);
    assert_eq!(joined.id, original.id);
    assert_eq!(joined.text, original.text);
    assert_eq!(joined.runs, original_runs);
    assert_eq!(
        session.project().direct_paragraph_overrides,
        original_overrides
    );
    valid(session.undo());
    assert_eq!(session.project().elements.len(), 2);
    valid(session.redo());
    assert_eq!(session.project().elements.len(), 1);
    assert_eq!(
        session.project().elements[0].as_element().runs,
        original_runs
    );
}
