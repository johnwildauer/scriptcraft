#![forbid(unsafe_code)]

use scriptcraft_model::dual_dialogue::{
    active_dual_dialogue_projection, DualDialogueGroup, SelectionSurface, ToggleTarget,
};
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, Position, Project, ScreenplayElement, Selection, StableId,
};
use scriptcraft_session::editing::EditingSession;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("dual-dialogue smoke scenario is valid"))
}

fn key(label: &str) -> ElementTypeKey {
    valid(ElementTypeKey::new(label))
}

fn caret(element: usize, offset: usize) -> Selection {
    Selection::caret(Position { element, offset })
}

fn body(project: &mut Project, elements: &[(&str, &str, &str)]) {
    project.elements = elements
        .iter()
        .map(|(id, element_type, text)| {
            BodyOwner::shared(valid(ScreenplayElement::new(
                *id,
                key(*element_type),
                *text,
            )))
        })
        .collect();
}

#[test]
fn toggle_candidate_requires_exact_capabilities_not_paginate_as() {
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
    let mut dialogue_like = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("Project::new supplies the base Action definition"));
    dialogue_like.key = key("Dialogue-like");
    dialogue_like.behavior.paginate_as = Some("Dialogue".to_owned());
    definitions.push(dialogue_like);
    valid(project.replace_element_definitions(definitions));
    body(
        &mut project,
        &[
            ("toggle-left-cue", "Character", "LEFT"),
            ("toggle-left-line", "Dialogue-like", "left line"),
            ("toggle-right-cue", "Character", "RIGHT"),
            ("toggle-right-line", "Dialogue", "right line"),
        ],
    );

    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(caret(0, 0)));
    let before = session.clone();
    let revision = session.revision();
    let mutation_token = session.mutation_token();
    assert!(valid(session.dual_dialogue_toggle_target(SelectionSurface::Body)).is_none());
    valid(session.toggle_dual_dialogue());
    assert_eq!(session, before);
    assert_eq!(session.revision(), revision);
    assert_eq!(session.mutation_token(), mutation_token);
    assert!(session.project().dual_dialogue_groups.is_empty());
}

#[test]
fn explicit_toggle_disables_an_inactive_group_without_changing_members() {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    project.preservation = None;
    body(
        &mut project,
        &[
            ("inactive-cue-left", "Character", "LEFT"),
            ("inactive-member", "Action", "sequential member"),
            ("inactive-cue-right", "Character", "RIGHT"),
            ("inactive-right-line", "Dialogue", "right line"),
        ],
    );
    let members = project
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: valid(StableId::new("inactive-explicit-disable")),
        members: members.clone(),
        wrapper: Default::default(),
    });
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    let original_elements = project.elements.clone();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(caret(1, 0)));

    valid(session.toggle_dual_dialogue());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert_eq!(session.project().elements, original_elements);
    valid(session.undo());
    assert_eq!(session.project().dual_dialogue_groups[0].members, members);
    assert!(active_dual_dialogue_projection(
        session.project(),
        &session.project().dual_dialogue_groups[0]
    )
    .is_none());
}

#[test]
fn explicit_toggle_records_only_the_exact_capability_candidate_members() {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    project.preservation = None;
    body(
        &mut project,
        &[
            ("toggle-exact-left", "Character", "LEFT"),
            ("toggle-exact-left-line", "Dialogue", "left line"),
            ("toggle-exact-right", "Character", "RIGHT"),
            ("toggle-exact-right-line", "Parenthetical", "(right)"),
            ("outside-action", "Action", "outside"),
        ],
    );
    let expected_members = project.elements[..4]
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(caret(0, 0)));

    assert!(matches!(
        valid(session.dual_dialogue_toggle_target(SelectionSurface::Body)),
        Some(ToggleTarget::Enable(_))
    ));
    valid(session.toggle_dual_dialogue());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(
        session.project().dual_dialogue_groups[0].members,
        expected_members
    );
    assert!(active_dual_dialogue_projection(
        session.project(),
        &session.project().dual_dialogue_groups[0]
    )
    .is_some());
    assert!(!session.project().dual_dialogue_groups[0]
        .member_ids()
        .any(|member| member.as_str() == "outside-action"));
}
