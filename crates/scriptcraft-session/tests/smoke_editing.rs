#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::dual_dialogue::DualDialogueGroup;
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, Position, Project, ScreenplayElement, Selection, StableId,
};
use scriptcraft_session::editing::{EditingSession, ElementDefinitionCandidate};

const OPEN_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/open_element_settings.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("editing smoke uses valid bounded input"))
}

fn source_paragraph<'a>(
    element: &'a PreservedElement,
    foreign_id: &str,
) -> Option<&'a PreservedElement> {
    if element.name.local_name == "Paragraph"
        && element.attributes.iter().any(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name == "id"
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
        .unwrap_or_else(|| unreachable!("fixture paragraph remains retained"))
}

fn unqualified_attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
        })
        .map(|attribute| attribute.value.as_str())
}

#[test]
fn editing_history_public_api_smoke() {
    const EDIT_TEXT: &str = "Café — répétition";

    let original = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    let original_selection = Selection::caret(Position {
        element: 0,
        offset: 0,
    });
    let mut session = valid(EditingSession::new(original.clone()));
    let initial_revision = session.revision();
    assert_eq!(session.selection(), original_selection);
    assert!(!session.dirty());
    assert!(!session.can_undo());
    assert!(!session.can_redo());

    let edited_selection = Selection::caret(Position {
        element: original_selection.focus.element,
        offset: original_selection.focus.offset + EDIT_TEXT.len(),
    });
    valid(session.insert(EDIT_TEXT));
    assert_eq!(session.project().elements[0].text, EDIT_TEXT);
    assert_eq!(session.selection(), edited_selection);
    let edited_revision = session.revision();
    assert_eq!(edited_revision, initial_revision + 1);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert!(!session.can_redo());

    valid(session.undo());
    assert_eq!(session.project(), &original);
    assert_eq!(session.selection(), original_selection);
    assert!(!session.dirty());
    assert!(!session.can_undo());
    assert!(session.can_redo());
    let undone_revision = session.revision();
    assert_ne!(undone_revision, edited_revision);

    valid(session.redo());
    assert_eq!(session.project().elements[0].text, EDIT_TEXT);
    assert_eq!(session.selection(), edited_selection);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert!(!session.can_redo());
    assert_eq!(session.revision(), undone_revision + 1);
}

#[test]
fn arbitrary_paginate_as_text_edit_keeps_scoped_derived_fast_path() {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    project.preservation = None;
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut lyrics = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("base Action definition is present"));
    lyrics.key = valid(ElementTypeKey::new("Lyrics"));
    lyrics.behavior.paginate_as = Some("Scene Heading".to_owned());
    definitions.push(lyrics);
    valid(project.replace_element_definitions(definitions));
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "lyrics-owner",
        valid(ElementTypeKey::new("Lyrics")),
        "ordinary text",
    )))];

    let mut session = valid(EditingSession::new(project));
    valid(session.refresh_cast_lists());
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    valid(session.insert("edited "));
    assert!(!session.pending_derived_work_summary().pending);
}

#[test]
fn current_behavior_guidance_and_enter_use_the_exact_document_key() {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    project.preservation = None;
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut verse = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("required Action definition exists"));
    verse.key = valid(ElementTypeKey::new("Verse"));
    verse.behavior.return_key = Some(valid(ElementTypeKey::new("lowercase action")));
    let mut target = verse.clone();
    target.key = valid(ElementTypeKey::new("lowercase action"));
    definitions.push(verse);
    definitions.push(target);
    valid(project.replace_element_definitions(definitions));
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "behavior-body",
        valid(ElementTypeKey::new("Verse")),
        "verse body",
    )))];

    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 5,
    })));
    let expected = valid(ElementTypeKey::new("lowercase action"));
    assert_eq!(
        session.paragraph_key_targets().enter,
        scriptcraft_session::editing::ParagraphKeyTarget::Target(expected.clone())
    );
    valid(session.enter());
    assert_eq!(session.project().elements[1].element_type(), expected);
}

#[test]
fn preservation_free_definition_replacement_is_one_undoable_registry_edit() {
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
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "preservation-free-definition-replacement-body",
        valid(ElementTypeKey::new("Action")),
        "",
    )))];
    let mut session = valid(EditingSession::new(project.clone()));
    let initial_revision = session.revision();
    let definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .rev()
        .collect::<Vec<_>>();
    let expected = definitions
        .iter()
        .map(|definition| definition.key.as_str().to_owned())
        .collect::<Vec<_>>();

    valid(
        session.replace_element_definitions_and_remap_types(ElementDefinitionCandidate::new(
            definitions,
        )),
    );

    assert_eq!(session.revision(), initial_revision + 1);
    assert!(session.dirty());
    assert_eq!(
        session
            .project()
            .element_definitions()
            .iter()
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>(),
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
    valid(session.undo());
    assert_eq!(session.project(), &project);
    assert!(!session.dirty());
    valid(session.redo());
    assert_eq!(
        session
            .project()
            .element_definitions()
            .iter()
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>(),
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert!(session.dirty());
}

#[test]
fn source_backed_generic_body_type_change_is_atomic_and_undoable() {
    let project = valid(read_v6(OPEN_FIXTURE)).project;
    let body_index = project
        .elements
        .iter()
        .position(|owner| {
            owner
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-body-general-derived")
        })
        .unwrap_or_else(|| unreachable!("the fixture body paragraph is managed"));
    let mut session = valid(EditingSession::new(project));
    let selection = Selection::caret(Position {
        element: body_index,
        offset: 4,
    });
    valid(session.set_selection(selection));

    let baseline_session = session.clone();
    let before = session.project().elements[body_index].as_element().clone();
    let before_children = retained_paragraph(session.project(), "open-body-general-derived")
        .children
        .clone();
    let original_bytes = session
        .project()
        .preservation
        .as_ref()
        .map(|preservation| preservation.source.original_bytes.clone())
        .unwrap_or_default();
    let initial_revision = session.revision();
    let target = valid(ElementTypeKey::new("lyrics"));

    valid(session.change_element_type(target.clone()));

    let changed = session.project().elements[body_index].as_element();
    assert_eq!(changed.element_type(), target);
    assert_eq!(changed.id, before.id);
    assert_eq!(changed.text, before.text);
    assert_eq!(changed.runs, before.runs);
    assert_eq!(
        changed
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_role.as_deref()),
        Some("lyrics")
    );
    assert_eq!(session.selection(), selection);
    assert_eq!(session.revision(), initial_revision + 1);
    assert!(session.dirty());
    assert!(session.can_undo());

    let retained = retained_paragraph(session.project(), "open-body-general-derived");
    assert_eq!(unqualified_attribute(retained, "Type"), Some("lyrics"));
    assert_eq!(retained.children, before_children);
    assert_eq!(
        session
            .project()
            .preservation
            .as_ref()
            .map(|preservation| preservation.source.original_bytes.clone()),
        Some(original_bytes)
    );
    assert_eq!(
        session.project().direct_paragraph_overrides,
        baseline_session.project().direct_paragraph_overrides
    );

    let changed_session = session.clone();
    let changed_revision = session.revision();
    valid(session.change_element_type(target.clone()));
    assert_eq!(
        session, changed_session,
        "an exact-key no-op publishes nothing"
    );
    let missing = valid(ElementTypeKey::new("not in this document"));
    let before_failure = session.clone();
    assert!(session.change_element_type(missing).is_err());
    assert_eq!(
        session, before_failure,
        "a missing target leaves session state unchanged"
    );
    assert_eq!(session.revision(), changed_revision);

    valid(session.undo());
    assert_eq!(session.project(), &baseline_session.project().clone());
    assert_eq!(session.selection(), selection);
    assert!(!session.dirty());
    valid(session.redo());
    assert_eq!(session.project(), changed_session.project());
    assert_eq!(session.selection(), changed_session.selection());
    assert!(session.dirty());
}

#[test]
fn source_backed_title_type_changes_support_optional_keys_and_history() {
    let project = valid(read_v6(OPEN_FIXTURE)).project;
    let mut session = valid(EditingSession::new(project));
    let content_title = session
        .project()
        .title_page
        .iter()
        .position(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-title-content")
        })
        .unwrap_or_else(|| unreachable!("the fixture TitlePage Content paragraph is managed"));
    let typeless_title = session
        .project()
        .title_page
        .iter()
        .position(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-title-typeless")
        })
        .unwrap_or_else(|| unreachable!("the fixture type-less title is managed"));

    let direct_selection = Selection::caret(Position {
        element: content_title,
        offset: 2,
    });
    valid(session.set_title_selection(direct_selection));
    let initial = session.project().title_page[content_title].clone();
    let old_revision = session.revision();
    let content_key = valid(ElementTypeKey::new("titlefirst"));
    session
        .change_title_element_type(Some(content_key.clone()))
        .unwrap_or_else(|error| panic!("typed-title change failed: {error:?}"));
    assert_eq!(
        session.project().title_page[content_title].element_type(),
        Some(content_key)
    );
    assert_eq!(
        session.project().title_page[content_title].text,
        initial.text
    );
    assert_eq!(
        session.project().title_page[content_title].runs,
        initial.runs
    );
    assert_eq!(session.project().title_page[content_title].id, initial.id);
    assert_eq!(session.title_selection(), direct_selection);
    assert_eq!(session.revision(), old_revision + 1);
    assert_eq!(
        unqualified_attribute(
            retained_paragraph(session.project(), "open-title-content"),
            "Type"
        ),
        Some("titlefirst")
    );

    valid(session.change_title_element_type(None));
    assert_eq!(
        session.project().title_page[content_title].element_type(),
        None
    );
    assert_eq!(
        unqualified_attribute(
            retained_paragraph(session.project(), "open-title-content"),
            "Type"
        ),
        None
    );

    let typeless_selection = Selection::caret(Position {
        element: typeless_title,
        offset: 3,
    });
    valid(session.set_title_selection(typeless_selection));
    let typeless_before = session.project().title_page[typeless_title].clone();
    let first_key = valid(ElementTypeKey::new("titlefirst"));
    valid(session.change_title_element_type(Some(first_key.clone())));
    assert_eq!(
        session.project().title_page[typeless_title].element_type(),
        Some(first_key)
    );
    assert_eq!(
        session.project().title_page[typeless_title].text,
        typeless_before.text
    );
    assert_eq!(
        session.project().title_page[typeless_title].runs,
        typeless_before.runs
    );
    assert_eq!(
        session.project().title_page[typeless_title].id,
        typeless_before.id
    );
    assert_eq!(session.title_selection(), typeless_selection);
    assert_eq!(
        unqualified_attribute(
            retained_paragraph(session.project(), "open-title-typeless"),
            "Type"
        ),
        Some("titlefirst")
    );

    let changed_session = session.clone();
    valid(session.undo());
    assert_eq!(
        session.project().title_page[typeless_title].element_type(),
        None
    );
    valid(session.redo());
    assert_eq!(session.project(), changed_session.project());
    assert_eq!(session.title_selection(), changed_session.title_selection());
    assert!(session.dirty());
}

#[test]
fn open_type_change_is_type_only_without_the_removed_role_adapter() {
    let project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    let mut session = valid(EditingSession::new(project.clone()));
    let initial_revision = session.revision();

    valid(session.change_element_type(
        scriptcraft_model::ElementTypeKey::new("Parenthetical").expect("valid fixture label"),
    ));

    assert_eq!(
        session.project().elements[0].element_type().as_str(),
        "Parenthetical"
    );
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(
        session.selection(),
        Selection::caret(Position {
            element: 0,
            offset: 0
        })
    );
    assert_eq!(session.revision(), initial_revision + 1);
    assert!(session.dirty());
    valid(session.undo());
    assert_eq!(session.project(), &project);
    assert!(!session.dirty());
}

#[test]
fn generic_body_type_change_never_unwraps_an_invalidated_dual_group() {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    project.preservation = None;
    let left_cue = valid(ScreenplayElement::new(
        "left-cue",
        scriptcraft_model::ElementTypeKey::new("Character").expect("valid fixture label"),
        "LEFT",
    ));
    let left_dialogue = valid(ScreenplayElement::new(
        "left-dialogue",
        scriptcraft_model::ElementTypeKey::new("Dialogue").expect("valid fixture label"),
        "left",
    ));
    let right_cue = valid(ScreenplayElement::new(
        "right-cue",
        scriptcraft_model::ElementTypeKey::new("Character").expect("valid fixture label"),
        "RIGHT",
    ));
    let right_dialogue = valid(ScreenplayElement::new(
        "right-dialogue",
        scriptcraft_model::ElementTypeKey::new("Dialogue").expect("valid fixture label"),
        "right",
    ));
    let group_id = valid(StableId::new("dual-group"));
    project.elements = vec![
        BodyOwner::shared(left_cue),
        BodyOwner::shared(left_dialogue),
        BodyOwner::shared(right_cue),
        BodyOwner::shared(right_dialogue),
    ];
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: group_id,
        members: ["left-cue", "left-dialogue", "right-cue", "right-dialogue"]
            .into_iter()
            .map(|id| valid(StableId::new(id)))
            .collect(),
        wrapper: Default::default(),
    });
    assert_eq!(project.validate(), Ok(()));

    let original = project.clone();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: 2,
    })));
    let selection = session.selection();
    let revision = session.revision();
    let action = valid(ElementTypeKey::new("Action"));

    valid(session.change_element_type(action));
    assert_ne!(session.project(), &original);
    assert_eq!(
        session.project().dual_dialogue_groups[0].members,
        original.dual_dialogue_groups[0].members
    );
    assert!(
        scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(
            session.project(),
            &session.project().dual_dialogue_groups[0]
        )
        .is_none()
    );
    assert_eq!(session.selection(), selection);
    assert_eq!(session.revision(), revision + 1);
    assert!(session.can_undo());
    valid(session.undo());
    assert_eq!(session.project(), &original);
    assert!(
        scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(
            session.project(),
            &session.project().dual_dialogue_groups[0]
        )
        .is_some()
    );
    valid(session.redo());
    assert!(
        scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(
            session.project(),
            &session.project().dual_dialogue_groups[0]
        )
        .is_none()
    );
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
}
