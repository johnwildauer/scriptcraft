use scriptcraft_core::editing::{DerivedWorkSummary, EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};

const IMPORTED_SCENE: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" ID="heading"><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Character" ID="alice"><Text>ALICE</Text></Paragraph>
<Paragraph Type="Character" ID="bob"><Text>BOB</Text></Paragraph>
<Paragraph Type="Cast List" ID="cast"><Text>(OLD</Text><Text>)</Text></Paragraph>
</Content></FinalDraft>"#;

fn imported_session() -> EditingSession {
    let project = read_v6(IMPORTED_SCENE)
        .unwrap_or_else(|_| unreachable!("imported repeated-refresh fixture is valid"))
        .project;
    EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("imported repeated-refresh fixture forms a session"))
}

fn assert_scoped_pending(session: &EditingSession) {
    assert_eq!(
        session.pending_derived_work_summary(),
        DerivedWorkSummary {
            pending: true,
            complete: false,
            affected_scene_count: 1,
            retryable: true,
            deferred_scene_count: 0,
            failed_complete: false,
            failed_scene_count: 0,
        }
    );
}

fn replace_character_text(session: &mut EditingSession, element: usize, replacement: &str) {
    let length = session.project().elements[element].text.len();
    session
        .set_selection(Selection {
            anchor: Position { element, offset: 0 },
            focus: Position {
                element,
                offset: length,
            },
        })
        .unwrap_or_else(|_| unreachable!("selected imported Character text is valid"));
    session
        .insert(replacement)
        .unwrap_or_else(|_| unreachable!("imported Character replacement is valid"));
}

fn assert_cast_and_arc_projection(session: &EditingSession, cast: &str, names: &[&str]) {
    let list = &session.project().elements[3];
    assert_eq!(list.text, cast);
    assert_eq!(list.runs.len(), 2, "both imported plain Text runs are retained");
    assert_eq!(list.runs[0].text, cast);
    assert_eq!(list.runs[1].text, "");

    let serialized = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("repeatedly corrected imported FDX serializes"));
    let reopened = read_v6(&serialized)
        .unwrap_or_else(|_| unreachable!("repeatedly corrected imported FDX reopens"))
        .project;
    assert_eq!(reopened.elements[3].text, cast);
    assert_eq!(reopened.elements[3].runs.len(), 2);
    assert_eq!(reopened.elements[3].runs[0].text, cast);
    assert_eq!(reopened.elements[3].runs[1].text, "");
    let serialized = String::from_utf8(serialized)
        .unwrap_or_else(|_| unreachable!("native FDX output is UTF-8"));
    for name in names {
        assert!(
            serialized.contains(&format!("<CharacterArcBeat Name=\"{name}\">")),
            "serialized generated scene arc should include {name}"
        );
    }
    for removed in ["ALICE", "ALISON", "CAROL"] {
        if !names.contains(&removed) {
            assert!(
                !serialized.contains(&format!("<CharacterArcBeat Name=\"{removed}\">")),
                "serialized generated scene arc should not retain {removed}"
            );
        }
    }
}

#[test]
fn imported_scene_repeated_affected_refreshes_follow_completed_initial_complete_pass() {
    let mut session = imported_session();
    let initial_pending = session.pending_derived_work_summary();
    assert!(initial_pending.pending && initial_pending.complete && initial_pending.retryable);

    let initial = session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("initial Complete refresh is valid"));
    assert_eq!(initial.requested_scenes, 1);
    assert_eq!(initial.attempted_scenes, 1);
    assert_eq!(initial.changed_scenes, 1);
    assert_eq!(initial.unchanged_scenes, 0);
    assert_eq!(initial.deferred_scenes, 0);
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_cast_and_arc_projection(&session, "(ALICE, BOB)", &["ALICE", "BOB"]);

    // A case-only Character rename creates real affected-scene work but leaves
    // the normalized Cast List and scene arcs unchanged.
    replace_character_text(&mut session, 2, "bob");
    assert_scoped_pending(&session);
    let unchanged = session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("case-only affected-scene refresh is valid"));
    assert!(!unchanged.changed);
    assert_eq!(unchanged.requested_scenes, 1);
    assert_eq!(unchanged.attempted_scenes, 1);
    assert_eq!(unchanged.changed_scenes, 0);
    assert_eq!(unchanged.unchanged_scenes, 1);
    assert_eq!(unchanged.deferred_scenes, 0);
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_cast_and_arc_projection(&session, "(ALICE, BOB)", &["ALICE", "BOB"]);

    replace_character_text(&mut session, 1, "ALISON");
    assert_scoped_pending(&session);
    let renamed = session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("first changed affected-scene refresh is valid"));
    assert_eq!(renamed.requested_scenes, 1);
    assert_eq!(renamed.attempted_scenes, 1);
    assert_eq!(renamed.changed_scenes, 1);
    assert_eq!(renamed.unchanged_scenes, 0);
    assert_eq!(renamed.deferred_scenes, 0);
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_cast_and_arc_projection(&session, "(ALISON, BOB)", &["ALISON", "BOB"]);

    let length = session.project().elements[1].text.len();
    session
        .set_selection(Selection {
            anchor: Position {
                element: 1,
                offset: 0,
            },
            focus: Position {
                element: 1,
                offset: length,
            },
        })
        .unwrap_or_else(|_| unreachable!("selected imported Character text is valid"));
    session
        .delete()
        .unwrap_or_else(|_| unreachable!("selected Character text can be removed"));
    assert_scoped_pending(&session);
    let removed = session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("removed-cue affected-scene refresh is valid"));
    assert_eq!(removed.requested_scenes, 1);
    assert_eq!(removed.attempted_scenes, 1);
    assert_eq!(removed.changed_scenes, 1);
    assert_eq!(removed.unchanged_scenes, 0);
    assert_eq!(removed.deferred_scenes, 0);
    assert_eq!(session.project().elements[1].text, "");
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_cast_and_arc_projection(&session, "(BOB)", &["BOB"]);

    session
        .set_selection(Selection::caret(Position {
            element: 1,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!("blank imported Character caret is valid"));
    session
        .insert("CAROL")
        .unwrap_or_else(|_| unreachable!("new Character cue text is valid"));
    assert_scoped_pending(&session);
    let added = session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("added-cue affected-scene refresh is valid"));
    assert_eq!(added.requested_scenes, 1);
    assert_eq!(added.attempted_scenes, 1);
    assert_eq!(added.changed_scenes, 1);
    assert_eq!(added.unchanged_scenes, 0);
    assert_eq!(added.deferred_scenes, 0);
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_cast_and_arc_projection(&session, "(BOB, CAROL)", &["BOB", "CAROL"]);
}
