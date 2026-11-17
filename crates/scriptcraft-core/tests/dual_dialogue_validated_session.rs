#![forbid(unsafe_code)]

use scriptcraft_core::dual_dialogue::{
    resolve_toggle_target, SelectionSurface, ToggleTarget,
};
use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::{
    project_validation_diagnostics, reset_project_validation_diagnostics, BodyOwner, ModelError,
    Project, Role, ScreenplayElement,
};

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("validated-session dual-dialogue case failed: {error:?}"))
}

fn two_sided_project() -> Project {
    let mut project = valid(Project::new());
    project.elements = [
        ("left-cue", Role::Character, "LEFT"),
        ("left-line", Role::Dialogue, "café"),
        ("right-cue", Role::Character, "RIGHT"),
        ("right-line", Role::Dialogue, "你好"),
    ]
    .into_iter()
    .map(|(id, role, text)| BodyOwner::shared(valid(ScreenplayElement::new(id, role, text))))
    .collect();
    project
}

fn caret(element: usize) -> Selection {
    Selection::caret(Position { element, offset: 0 })
}

fn set_selection(session: &mut EditingSession, selection: Selection) {
    valid(session.set_selection(selection));
}

fn assert_matches_public(session: &EditingSession, surface: SelectionSurface) {
    let selection = match surface {
        SelectionSurface::Body => session.selection(),
        SelectionSurface::TitlePage => session.title_selection(),
    };
    let expected = resolve_toggle_target(session.project(), surface, selection).map_err(EditError::Model);
    assert_eq!(session.dual_dialogue_toggle_target(surface), expected);
}

// This integration binary intentionally has one test entry point. Its complete-
// project validation counter assertions therefore cannot race another test in
// the same process.
#[test]
fn validated_session_resolution_matches_public_api_without_complete_revalidation() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    set_selection(&mut session, caret(0));

    assert_matches_public(&session, SelectionSurface::Body);
    let Some(ToggleTarget::Enable(pattern)) = valid(session.dual_dialogue_toggle_target(
        SelectionSurface::Body,
    )) else {
        panic!("the ungrouped two-sided sequence must be enable-eligible")
    };
    assert_eq!(pattern.flat_range, 0..4);

    set_selection(&mut session, caret(1));
    assert_matches_public(&session, SelectionSurface::Body);
    assert_eq!(
        session.dual_dialogue_toggle_target(SelectionSurface::Body),
        Ok(None)
    );

    // The right cue alone is not a new candidate: it overlaps the candidate
    // beginning at the left cue.
    set_selection(&mut session, caret(2));
    assert_matches_public(&session, SelectionSurface::Body);
    assert_eq!(
        session.dual_dialogue_toggle_target(SelectionSurface::Body),
        Ok(None)
    );

    for selection in [
        Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 2, offset: 0 },
        },
        Selection {
            anchor: Position { element: 2, offset: 0 },
            focus: Position { element: 0, offset: 0 },
        },
    ] {
        set_selection(&mut session, selection);
        assert_matches_public(&session, SelectionSurface::Body);
        assert_eq!(
            session.dual_dialogue_toggle_target(SelectionSurface::Body),
            Ok(None)
        );
    }
    assert_matches_public(&session, SelectionSurface::TitlePage);
    assert_eq!(
        session.dual_dialogue_toggle_target(SelectionSurface::TitlePage),
        Ok(None)
    );

    set_selection(&mut session, caret(0));
    valid(session.toggle_dual_dialogue());
    let group_id = session.project().dual_dialogue_groups[0].id.clone();
    for element in 0..4 {
        set_selection(&mut session, caret(element));
        assert_matches_public(&session, SelectionSurface::Body);
        assert_eq!(
            session.dual_dialogue_toggle_target(SelectionSurface::Body),
            Ok(Some(ToggleTarget::Disable(group_id.clone())))
        );
    }

    let mut counter_session = valid(EditingSession::new(two_sided_project()));
    set_selection(&mut counter_session, caret(0));
    let mut invalid_project = two_sided_project();
    invalid_project.elements[1] = BodyOwner::shared(valid(ScreenplayElement::new(
        "left-cue",
        Role::Dialogue,
        "duplicate identity",
    )));

    reset_project_validation_diagnostics();
    assert_eq!(
        project_validation_diagnostics().complete_project_validations,
        0
    );

    for _ in 0..4 {
        assert!(matches!(
            counter_session.dual_dialogue_toggle_target(SelectionSurface::Body),
            Ok(Some(ToggleTarget::Enable(_)))
        ));
    }
    for element in [1, 2, 0] {
        set_selection(&mut counter_session, caret(element));
        assert_eq!(
            counter_session.dual_dialogue_toggle_target(SelectionSurface::Body),
            counter_session.dual_dialogue_toggle_target(SelectionSurface::Body)
        );
    }
    assert_eq!(
        project_validation_diagnostics().complete_project_validations,
        0,
        "session target reads and selection movement add no complete validations"
    );

    assert!(matches!(
        resolve_toggle_target(
            counter_session.project(),
            SelectionSurface::Body,
            counter_session.selection(),
        ),
        Ok(Some(ToggleTarget::Enable(_)))
    ));
    assert_eq!(
        project_validation_diagnostics().complete_project_validations,
        1,
        "the arbitrary-project public wrapper still validates"
    );

    // Validation remains ahead of surface eligibility for arbitrary projects.
    assert_eq!(
        resolve_toggle_target(
            &invalid_project,
            SelectionSurface::TitlePage,
            caret(0),
        ),
        Err(ModelError::DuplicateId)
    );
    assert_eq!(
        project_validation_diagnostics().complete_project_validations,
        2
    );
}
