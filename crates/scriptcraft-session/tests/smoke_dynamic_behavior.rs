#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, Position, Project, RunAttribute, ScreenplayElement, Selection,
    StableId, TextRun,
};
use scriptcraft_session::editing::{EditingSession, ParagraphKeyTarget};

const ACTIVE_DUAL_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/active_dual_dialogue.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("dynamic Behavior source uses valid bounded input"))
}

fn key(label: &str) -> ElementTypeKey {
    valid(ElementTypeKey::new(label))
}

fn ensure_definition(definitions: &mut Vec<scriptcraft_model::ElementDefinition>, label: &str) {
    if definitions
        .iter()
        .any(|definition| definition.key.as_str() == label)
    {
        return;
    }
    let mut definition = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("the Project contains its required Action definition"));
    definition.key = key(label);
    definitions.push(definition);
}

fn behavior_project(
    source_label: &str,
    text: &str,
    return_label: Option<&str>,
    tab_label: Option<&str>,
) -> Project {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ensure_definition(&mut definitions, source_label);
    for target in [return_label, tab_label].into_iter().flatten() {
        if target != "__unresolved-target__" {
            ensure_definition(&mut definitions, target);
        }
    }
    let source = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == source_label)
        .unwrap_or_else(|| unreachable!("the source definition was added"));
    source.behavior.return_key = return_label.map(key);
    source.behavior.tab_key = tab_label.map(key);
    valid(project.replace_element_definitions(definitions));
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "behavior-source",
        key(source_label),
        text,
    )))];
    project
}

fn caret(element: usize, offset: usize) -> Selection {
    Selection::caret(Position { element, offset })
}

fn assert_noop_state(
    session: &EditingSession,
    before: &EditingSession,
    revision: u64,
    mutation_token: u64,
) {
    assert_eq!(session, before);
    assert_eq!(session.revision(), revision);
    assert_eq!(session.mutation_token(), mutation_token);
}

#[test]
fn custom_behavior_graph_drives_exact_open_key_guidance_enter_and_tab() {
    let project = behavior_project("Verse", "north south", Some("action"), Some("Aside"));
    let mut enter_session = valid(EditingSession::new(project.clone()));
    valid(enter_session.set_selection(caret(0, "north ".len())));
    let targets = enter_session.paragraph_key_targets();
    assert_eq!(targets.enter, ParagraphKeyTarget::Target(key("action")));
    assert_eq!(targets.tab, ParagraphKeyTarget::Target(key("Aside")));
    valid(enter_session.enter());
    assert_eq!(
        enter_session.project().elements[0].element_type(),
        key("Verse")
    );
    assert_eq!(enter_session.project().elements[0].text, "north ");
    assert_eq!(
        enter_session.project().elements[1].element_type(),
        key("action")
    );
    assert_eq!(enter_session.project().elements[1].text, "south");

    let mut tab_session = valid(EditingSession::new(project));
    valid(tab_session.set_selection(caret(0, 0)));
    assert_eq!(
        tab_session.paragraph_key_targets().tab,
        ParagraphKeyTarget::Target(key("Aside"))
    );
    valid(tab_session.tab());
    assert_eq!(
        tab_session.project().elements[0].element_type(),
        key("Aside")
    );
    assert_eq!(tab_session.project().elements[0].text, "north south");
}

#[test]
fn absent_and_unresolved_behavior_targets_are_consumed_complete_noops() {
    let absent = behavior_project("Freeform", "unchanged", None, None);
    let mut enter_session = valid(EditingSession::new(absent));
    valid(enter_session.set_selection(caret(0, 0)));
    assert_eq!(
        enter_session.paragraph_key_targets().enter,
        ParagraphKeyTarget::NoOp
    );
    let before_enter = enter_session.clone();
    let revision = enter_session.revision();
    let mutation_token = enter_session.mutation_token();
    valid(enter_session.enter());
    assert_noop_state(&enter_session, &before_enter, revision, mutation_token);
    assert!(!enter_session.can_undo());

    let unresolved = behavior_project(
        "Freeform",
        "unchanged",
        Some("__unresolved-target__"),
        Some("__unresolved-target__"),
    );
    let mut tab_session = valid(EditingSession::new(unresolved));
    valid(tab_session.set_selection(caret(0, 0)));
    let targets = tab_session.paragraph_key_targets();
    assert_eq!(targets.enter, ParagraphKeyTarget::NoOp);
    assert_eq!(targets.tab, ParagraphKeyTarget::NoOp);
    let before_tab = tab_session.clone();
    let revision = tab_session.revision();
    let mutation_token = tab_session.mutation_token();
    valid(tab_session.tab());
    assert_noop_state(&tab_session, &before_tab, revision, mutation_token);
    assert!(!tab_session.can_undo());
}

#[test]
fn resolved_same_key_tab_is_guided_but_does_not_publish_a_mutation() {
    let project = behavior_project("Notation", "still here", Some("Notation"), Some("Notation"));
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(caret(0, 0)));
    assert_eq!(
        session.paragraph_key_targets().tab,
        ParagraphKeyTarget::Target(key("Notation"))
    );
    let before = session.clone();
    let revision = session.revision();
    let mutation_token = session.mutation_token();
    valid(session.tab());
    assert_noop_state(&session, &before, revision, mutation_token);
    assert!(!session.can_undo());
}

#[test]
fn exact_capabilities_gate_strict_boundaries_and_blank_character_tab() {
    for structural_label in ["Scene Heading", "Character", "Transition"] {
        let mut strict_boundary = valid(EditingSession::new(behavior_project(
            structural_label,
            "INT. ROOM",
            Some("BoundaryResult"),
            Some("BoundaryResult"),
        )));
        valid(strict_boundary.set_selection(caret(0, 3)));
        assert_eq!(
            strict_boundary.paragraph_key_targets().enter,
            ParagraphKeyTarget::NoOp
        );
        let before = strict_boundary.clone();
        let revision = strict_boundary.revision();
        let mutation_token = strict_boundary.mutation_token();
        valid(strict_boundary.enter());
        assert_noop_state(&strict_boundary, &before, revision, mutation_token);
    }

    let mut case_project = behavior_project("scene heading", "INT. ROOM", Some("OpenResult"), None);
    let mut definitions = case_project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "scene heading")
        .unwrap_or_else(|| unreachable!("case-distinct definition exists"))
        .behavior
        .paginate_as = Some("Scene Heading".to_owned());
    valid(case_project.replace_element_definitions(definitions));
    let mut case_variant = valid(EditingSession::new(case_project));
    valid(case_variant.set_selection(caret(0, 3)));
    assert_eq!(
        case_variant.paragraph_key_targets().enter,
        ParagraphKeyTarget::Target(key("OpenResult"))
    );
    valid(case_variant.enter());
    assert_eq!(
        case_variant.project().elements[1].element_type(),
        key("OpenResult")
    );

    let mut character = valid(EditingSession::new(behavior_project(
        "Character",
        "ALICE",
        None,
        Some("CueNext"),
    )));
    valid(character.set_selection(caret(0, 5)));
    assert_eq!(
        character.paragraph_key_targets().tab,
        ParagraphKeyTarget::NoOp
    );
    let before = character.clone();
    let revision = character.revision();
    let mutation_token = character.mutation_token();
    valid(character.tab());
    assert_noop_state(&character, &before, revision, mutation_token);

    let mut blank_character = valid(EditingSession::new(behavior_project(
        "Character",
        "",
        None,
        Some("CueNext"),
    )));
    assert_eq!(
        blank_character.paragraph_key_targets().tab,
        ParagraphKeyTarget::Target(key("CueNext"))
    );
    valid(blank_character.tab());
    assert_eq!(
        blank_character.project().elements[0].element_type(),
        key("CueNext")
    );
}

#[test]
fn dialogue_interior_enter_retains_exact_key_while_end_and_empty_use_return_behavior() {
    let mut interior = valid(EditingSession::new(behavior_project(
        "Dialogue",
        "A line",
        Some("DialogueExit"),
        None,
    )));
    valid(interior.set_selection(caret(0, 2)));
    assert_eq!(
        interior.paragraph_key_targets().enter,
        ParagraphKeyTarget::Target(key("Dialogue"))
    );
    valid(interior.enter());
    assert_eq!(
        interior.project().elements[1].element_type(),
        key("Dialogue")
    );
    assert_eq!(interior.project().elements[0].text, "A ");
    assert_eq!(interior.project().elements[1].text, "line");

    for text in ["A line", ""] {
        let mut session = valid(EditingSession::new(behavior_project(
            "Dialogue",
            text,
            Some("DialogueExit"),
            None,
        )));
        valid(session.set_selection(caret(0, text.len())));
        assert_eq!(
            session.paragraph_key_targets().enter,
            ParagraphKeyTarget::Target(key("DialogueExit"))
        );
        valid(session.enter());
        assert_eq!(
            session.project().elements[1].element_type(),
            key("DialogueExit")
        );
    }
}

#[test]
fn parenthetical_repairs_and_delimiter_transitions_resolve_before_mutating() {
    let mut enter_repair = valid(EditingSession::new(behavior_project(
        "Parenthetical",
        "(thought",
        Some("ReturnShape"),
        Some("TabShape"),
    )));
    valid(enter_repair.set_selection(caret(0, "(thought".len())));
    assert_eq!(
        enter_repair.paragraph_key_targets().enter,
        ParagraphKeyTarget::Target(key("ReturnShape"))
    );
    valid(enter_repair.enter());
    assert_eq!(enter_repair.project().elements[0].text, "(thought)");
    assert_eq!(
        enter_repair.project().elements[1].element_type(),
        key("ReturnShape")
    );

    let mut tab_repair = valid(EditingSession::new(behavior_project(
        "Parenthetical",
        "(thought",
        Some("ReturnShape"),
        Some("TabShape"),
    )));
    valid(tab_repair.set_selection(caret(0, "(thought".len())));
    valid(tab_repair.tab());
    assert_eq!(tab_repair.project().elements[0].text, "(thought)");
    assert_eq!(
        tab_repair.project().elements[1].element_type(),
        key("TabShape")
    );

    let mut delimiter = valid(EditingSession::new(behavior_project(
        "Parenthetical",
        "()",
        Some("ReturnShape"),
        Some("TabShape"),
    )));
    assert_eq!(
        delimiter.paragraph_key_targets().tab,
        ParagraphKeyTarget::Target(key("TabShape"))
    );
    valid(delimiter.tab());
    assert_eq!(
        delimiter.project().elements[0].element_type(),
        key("TabShape")
    );
    assert_eq!(delimiter.project().elements[0].text, "");

    let same_key_delimiter = behavior_project("Parenthetical", "()", None, Some("Parenthetical"));
    let mut same_key_session = valid(EditingSession::new(same_key_delimiter));
    assert_eq!(
        same_key_session.paragraph_key_targets().tab,
        ParagraphKeyTarget::Target(key("Parenthetical"))
    );
    let before_same_key = same_key_session.clone();
    let revision = same_key_session.revision();
    let mutation_token = same_key_session.mutation_token();
    valid(same_key_session.tab());
    assert_noop_state(
        &same_key_session,
        &before_same_key,
        revision,
        mutation_token,
    );

    let mut unresolved_repair = valid(EditingSession::new(behavior_project(
        "Parenthetical",
        "(untouched",
        Some("__unresolved-target__"),
        None,
    )));
    valid(unresolved_repair.set_selection(caret(0, "(untouched".len())));
    assert_eq!(
        unresolved_repair.paragraph_key_targets().enter,
        ParagraphKeyTarget::NoOp
    );
    let before = unresolved_repair.clone();
    let revision = unresolved_repair.revision();
    let mutation_token = unresolved_repair.mutation_token();
    valid(unresolved_repair.enter());
    assert_noop_state(&unresolved_repair, &before, revision, mutation_token);
}

#[test]
fn generated_parenthetical_cleanup_splits_to_current_return_key() {
    let project = behavior_project("Freeform", "", None, Some("Parenthetical"));
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ensure_definition(&mut definitions, "GeneratedExit");
    let parenthetical = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Parenthetical")
        .unwrap_or_else(|| unreachable!("required Parenthetical definition exists"));
    parenthetical.behavior.return_key = Some(key("GeneratedExit"));
    let mut project = project;
    valid(project.replace_element_definitions(definitions));

    let mut session = valid(EditingSession::new(project));
    valid(session.tab());
    assert_eq!(
        session.project().elements[0].element_type(),
        key("Parenthetical")
    );
    assert_eq!(session.project().elements[0].text, "()");
    valid(session.set_selection(caret(0, 1)));
    assert_eq!(
        session.paragraph_key_targets().enter,
        ParagraphKeyTarget::Target(key("GeneratedExit"))
    );
    valid(session.enter());
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(
        session.project().elements[1].element_type(),
        key("GeneratedExit")
    );
}

#[test]
fn final_active_dual_right_member_uses_its_return_key_and_exits_source_membership() {
    let mut project = valid(read_v6(ACTIVE_DUAL_FIXTURE)).project;
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ensure_definition(&mut definitions, "RightExit");
    let dialogue = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Dialogue")
        .unwrap_or_else(|| unreachable!("required Dialogue definition exists"));
    dialogue.behavior.return_key = Some(key("RightExit"));
    valid(project.replace_element_definitions(definitions));
    let right_index = project
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("active-right-line")
        })
        .unwrap_or_else(|| unreachable!("fixture has a managed right Dialogue member"));
    let group_members = project.dual_dialogue_groups[0]
        .member_ids()
        .cloned()
        .collect::<Vec<_>>();
    let end = project.elements[right_index].text.len();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(caret(right_index, end)));
    assert_eq!(
        session.paragraph_key_targets().enter,
        ParagraphKeyTarget::Target(key("RightExit"))
    );
    valid(session.enter());

    let inserted = &session.project().elements[right_index + 1];
    assert_eq!(inserted.element_type(), key("RightExit"));
    assert!(!group_members.contains(&inserted.id));
    assert_eq!(
        session.project().dual_dialogue_groups[0]
            .member_ids()
            .cloned()
            .collect::<Vec<_>>(),
        group_members
    );
}

#[test]
fn exact_open_key_splits_preserve_runs_ids_and_direct_formatting() {
    let mut project = behavior_project("Verse", "", Some("Verse"), None);
    let source_key = key("Verse");
    let runs = vec![
        TextRun {
            text: "Café ".to_owned(),
            attributes: vec![RunAttribute {
                name: "Style".to_owned(),
                value: "Bold".to_owned(),
            }],
        },
        TextRun {
            text: "🎬 end".to_owned(),
            attributes: vec![RunAttribute {
                name: "Style".to_owned(),
                value: "Italic".to_owned(),
            }],
        },
    ];
    let expected_left_runs = vec![runs[0].clone()];
    let expected_right_runs = vec![runs[1].clone()];
    let element = valid(ScreenplayElement::with_runs(
        "run-owner",
        source_key.clone(),
        runs,
        None,
    ));
    project.elements = vec![BodyOwner::shared(element)];
    project.direct_paragraph_overrides.insert(
        valid(StableId::new("run-owner")),
        scriptcraft_model::formatting::SourceParagraphOverrides {
            paragraph: scriptcraft_model::formatting::SourceParagraphSpec {
                alignment: Some(scriptcraft_model::formatting::ParagraphAlignment::Center),
                ..scriptcraft_model::formatting::SourceParagraphSpec::default()
            },
            starts_new_page: Some(false),
        },
    );
    let override_snapshot = project.direct_paragraph_overrides.clone();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(caret(0, "Café ".len())));
    let original_id = session.project().elements[0].id.clone();
    valid(session.enter());
    assert_eq!(session.project().elements[0].element_type(), source_key);
    assert_eq!(session.project().elements[1].element_type(), key("Verse"));
    assert_eq!(session.project().elements[0].id, original_id);
    assert_ne!(session.project().elements[1].id, original_id);
    assert_eq!(session.project().elements[0].text, "Café ");
    assert_eq!(session.project().elements[1].text, "🎬 end");
    assert_eq!(
        session.project().elements[0].as_element().runs,
        expected_left_runs
    );
    assert_eq!(
        session.project().elements[1].as_element().runs,
        expected_right_runs
    );
    assert_eq!(
        session.project().direct_paragraph_overrides,
        override_snapshot
    );
}
