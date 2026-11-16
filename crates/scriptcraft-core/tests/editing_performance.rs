#![forbid(unsafe_code)]

use scriptcraft_core::editing::{
    edit_validation_diagnostics, reset_edit_validation_diagnostics, EditingSession, Position,
    Selection,
};
use scriptcraft_core::fdx::read_v6;
use scriptcraft_core::layout::{
    BodyEditorPresentation, LayoutBuildDiagnostics, LayoutCache, LayoutPurpose,
};
use scriptcraft_core::preservation::{
    copy_on_write_diagnostics, managed_paragraph_index_diagnostics,
    preservation_reconciliation_diagnostics, reset_copy_on_write_diagnostics,
    reset_managed_paragraph_index_diagnostics, reset_preservation_reconciliation_diagnostics,
    reset_source_identity_use_index_diagnostics, source_identity_use_index_diagnostics,
};
use scriptcraft_core::{
    reset_project_validation_diagnostics, project_validation_diagnostics, Project,
};

const ITEMS_PER_PAGE_SCALE: usize = 24;
const PAGE_SCALES: [usize; 3] = [1, 15, 50];

#[derive(Debug, Clone, Copy)]
enum Target {
    Beginning,
    Middle,
    End,
}

impl Target {
    const ALL: [Self; 3] = [Self::Beginning, Self::Middle, Self::End];

    fn index(self, item_count: usize) -> usize {
        match self {
            Self::Beginning => 0,
            Self::Middle => item_count / 2,
            Self::End => item_count - 1,
        }
    }

    fn empty_position(self, item_count: usize) -> usize {
        match self {
            Self::Beginning => 0,
            Self::Middle => item_count / 2,
            Self::End => item_count,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EditScenario {
    CharacterInsertion,
    BackspaceWithinText,
    EmptyElementDeletion,
    SameRoleJoin,
    Split,
    RangedReplacement,
    UndoRedo,
}

impl EditScenario {
    const ALL: [Self; 7] = [
        Self::CharacterInsertion,
        Self::BackspaceWithinText,
        Self::EmptyElementDeletion,
        Self::SameRoleJoin,
        Self::Split,
        Self::RangedReplacement,
        Self::UndoRedo,
    ];

    fn is_structural(self) -> bool {
        matches!(
            self,
            Self::EmptyElementDeletion
                | Self::SameRoleJoin
                | Self::Split
                | Self::RangedReplacement
        )
    }

    fn expects_scoped_validation(self) -> bool {
        matches!(
            self,
            Self::CharacterInsertion
                | Self::BackspaceWithinText
                | Self::RangedReplacement
                | Self::UndoRedo
        )
    }

    fn expects_scoped_fallback(self) -> bool {
        matches!(self, Self::EmptyElementDeletion | Self::SameRoleJoin)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct CounterDelta {
    project_validations: u64,
    edit_scoped_attempts: u64,
    edit_scoped_accepts: u64,
    edit_full_validations: u64,
    edit_full_fallbacks: u64,
    managed_index_rebuilds: u64,
    managed_index_source_nodes: u64,
    managed_index_lookups: u64,
    managed_index_entries_examined: u64,
    managed_index_fallback_nodes_examined: u64,
    source_use_index_rebuilds: u64,
    source_use_index_source_nodes: u64,
    source_use_identity_queries: u64,
    source_use_reserved_id_queries: u64,
    body_survivors_visited: u64,
    body_unchanged_survivors_skipped: u64,
    body_owners_projected: u64,
    body_owners_removed: u64,
    body_owners_inserted: u64,
    title_survivors_visited: u64,
    title_unchanged_survivors_skipped: u64,
    title_owners_projected: u64,
    title_owners_removed: u64,
    title_owners_inserted: u64,
    source_text_byte_delta_updates: u64,
    cow_shared_vector_clones: u64,
    cow_detached_vectors: u64,
    cow_shared_text_payload_clones: u64,
    cow_detached_text_payloads: u64,
    cow_shared_source_byte_clones: u64,
    cow_detached_source_byte_payloads: u64,
    cow_shared_image_payload_clones: u64,
    cow_detached_image_payloads: u64,
}

fn reset_counters() {
    reset_project_validation_diagnostics();
    reset_edit_validation_diagnostics();
    reset_managed_paragraph_index_diagnostics();
    reset_source_identity_use_index_diagnostics();
    reset_preservation_reconciliation_diagnostics();
    reset_copy_on_write_diagnostics();
}

/// Because all process-global counters are reset immediately before a scenario,
/// this read is that scenario's value-free counter delta.
fn counter_delta() -> CounterDelta {
    let project = project_validation_diagnostics();
    let editing = edit_validation_diagnostics();
    let managed = managed_paragraph_index_diagnostics();
    let source_use = source_identity_use_index_diagnostics();
    let reconciliation = preservation_reconciliation_diagnostics();
    let cow = copy_on_write_diagnostics();
    CounterDelta {
        project_validations: project.complete_project_validations,
        edit_scoped_attempts: editing.scoped_candidate_attempts,
        edit_scoped_accepts: editing.scoped_candidate_accepts,
        edit_full_validations: editing.full_candidate_validations,
        edit_full_fallbacks: editing.full_validation_fallbacks,
        managed_index_rebuilds: managed.rebuilds,
        managed_index_source_nodes: managed.source_nodes_indexed,
        managed_index_lookups: managed.lookups,
        managed_index_entries_examined: managed.indexed_entries_examined,
        managed_index_fallback_nodes_examined: managed.fallback_tree_nodes_examined,
        source_use_index_rebuilds: source_use.rebuilds,
        source_use_index_source_nodes: source_use.source_nodes_indexed,
        source_use_identity_queries: source_use.identity_queries,
        source_use_reserved_id_queries: source_use.reserved_id_queries,
        body_survivors_visited: reconciliation.body_survivors_visited,
        body_unchanged_survivors_skipped: reconciliation.body_unchanged_survivors_skipped,
        body_owners_projected: reconciliation.body_owners_projected,
        body_owners_removed: reconciliation.body_owners_removed,
        body_owners_inserted: reconciliation.body_owners_inserted,
        title_survivors_visited: reconciliation.title_survivors_visited,
        title_unchanged_survivors_skipped: reconciliation.title_unchanged_survivors_skipped,
        title_owners_projected: reconciliation.title_owners_projected,
        title_owners_removed: reconciliation.title_owners_removed,
        title_owners_inserted: reconciliation.title_owners_inserted,
        source_text_byte_delta_updates: reconciliation.source_text_byte_delta_updates,
        cow_shared_vector_clones: cow.shared_vector_clones,
        cow_detached_vectors: cow.detached_vectors,
        cow_shared_text_payload_clones: cow.shared_text_payload_clones,
        cow_detached_text_payloads: cow.detached_text_payloads,
        cow_shared_source_byte_clones: cow.shared_source_byte_clones,
        cow_detached_source_byte_payloads: cow.detached_source_byte_payloads,
        cow_shared_image_payload_clones: cow.shared_image_payload_clones,
        cow_detached_image_payloads: cow.detached_image_payloads,
    }
}

fn fixture_pair(page_scale: usize, empty_target: Target) -> (Project, Project) {
    let item_count = page_scale * ITEMS_PER_PAGE_SCALE;
    let empty_position = empty_target.empty_position(item_count);
    let mut source = String::from(
        r#"<FinalDraft DocumentType="Script" Version="6"><Content>"#,
    );
    for index in 0..item_count {
        if empty_position == index {
            source.push_str(
                "<Paragraph Type=\"Action\" id=\"empty-target\"><Text/></Paragraph>",
            );
        }
        source.push_str(&format!(
            "<Paragraph Type=\"Action\" id=\"synthetic-{index}\"><Text>synthetic action line {index} with deterministic baseline content</Text></Paragraph>"
        ));
    }
    if empty_position == item_count {
        source.push_str(
            "<Paragraph Type=\"Action\" id=\"empty-target\"><Text/></Paragraph>",
        );
    }
    source.push_str("</Content></FinalDraft>");
    let imported = read_v6(source.as_bytes())
        .unwrap_or_else(|_| unreachable!("deterministic synthetic FDX fixture imports"));
    let backed = imported.project;
    let mut source_less = backed.clone();
    source_less.preservation = None;
    for element in &mut source_less.elements {
        element.source_provenance = None;
    }
    for paragraph in &mut source_less.title_page {
        paragraph.source_provenance = None;
    }
    (backed, source_less)
}

fn title_fixture(owner_count: usize, empty_index: usize) -> Project {
    let mut source = String::from(
        r#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="title-scale-body"><Text>Body</Text></Paragraph></Content><TitlePage><Content>"#,
    );
    for index in 0..owner_count {
        let text = if index == empty_index {
            String::new()
        } else {
            format!("synthetic title paragraph {index}")
        };
        source.push_str(&format!(
            "<Paragraph Type=\"Title\" id=\"synthetic-title-{index}\"><Text>{text}</Text></Paragraph>"
        ));
    }
    source.push_str("</Content></TitlePage></FinalDraft>");
    read_v6(source.as_bytes())
        .unwrap_or_else(|_| unreachable!("deterministic synthetic title fixture imports"))
        .project
}

fn assert_canonical_parity(backed: &Project, source_less: &Project) {
    assert_eq!(backed.elements.len(), source_less.elements.len());
    assert_eq!(backed.title_page.len(), source_less.title_page.len());
    assert_eq!(backed.direct_paragraph_overrides, source_less.direct_paragraph_overrides);
    for (backed_element, source_less_element) in backed.elements.iter().zip(&source_less.elements) {
        assert_eq!(backed_element.role, source_less_element.role);
        assert_eq!(backed_element.text, source_less_element.text);
        assert_eq!(backed_element.runs, source_less_element.runs);
    }
    for (backed_title, source_less_title) in backed.title_page.iter().zip(&source_less.title_page) {
        assert_eq!(backed_title.text, source_less_title.text);
        assert_eq!(backed_title.runs, source_less_title.runs);
    }
}

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("baseline scenario uses valid input"))
}

fn regular_index(target: Target, item_count: usize) -> usize {
    let index = target.index(item_count);
    index + usize::from(target.empty_position(item_count) <= index)
}

fn prepare_selection(
    session: &mut EditingSession,
    scenario: EditScenario,
    target: Target,
    item_count: usize,
) {
    let index = regular_index(target, item_count);
    let text_len = session.project().elements[index].text.len();
    match scenario {
        EditScenario::CharacterInsertion => {
            valid(session.set_selection(Selection::caret(Position {
                element: index,
                offset: text_len / 2,
            })));
        }
        EditScenario::BackspaceWithinText => {
            valid(session.set_selection(Selection::caret(Position {
                element: index,
                offset: text_len - 1,
            })));
        }
        EditScenario::EmptyElementDeletion => {
            valid(session.set_selection(Selection::caret(Position {
                element: target.empty_position(item_count),
                offset: 0,
            })));
        }
        EditScenario::SameRoleJoin => {
            let left = match target {
                Target::Beginning => 0,
                Target::Middle => item_count / 2,
                Target::End => item_count - 2,
            };
            let left = left + usize::from(target.empty_position(item_count) <= left);
            valid(session.set_selection(Selection::caret(Position {
                element: left + 1,
                offset: 0,
            })));
        }
        EditScenario::Split => {
            valid(session.set_selection(Selection::caret(Position {
                element: index,
                offset: text_len / 2,
            })));
        }
        EditScenario::RangedReplacement => {
            valid(session.set_selection(Selection {
                anchor: Position {
                    element: index,
                    offset: 2,
                },
                focus: Position {
                    element: index,
                    offset: 8,
                },
            }));
        }
        EditScenario::UndoRedo => {
            valid(session.set_selection(Selection::caret(Position {
                element: index,
                offset: text_len,
            })));
        }
    }
}

fn mutate(session: &mut EditingSession, scenario: EditScenario) {
    match scenario {
        EditScenario::CharacterInsertion => valid(session.insert("x")),
        EditScenario::BackspaceWithinText => valid(session.backspace()),
        EditScenario::EmptyElementDeletion => valid(session.delete()),
        EditScenario::SameRoleJoin => valid(session.backspace()),
        EditScenario::Split => valid(session.enter()),
        EditScenario::RangedReplacement => valid(session.insert("replacement")),
        EditScenario::UndoRedo => {
            valid(session.insert("x"));
            valid(session.undo());
            valid(session.redo());
        }
    }
}

fn run_edit_scenario(
    project: Project,
    scenario: EditScenario,
    target: Target,
    item_count: usize,
) -> (Project, CounterDelta) {
    let mut session = valid(EditingSession::new(project));
    // Selection preparation and fixture/session construction are outside the
    // measured operation interval.
    prepare_selection(&mut session, scenario, target, item_count);
    let unchanged = session.clone();
    reset_counters();
    assert_eq!(session, unchanged, "counter reset does not mutate the session");
    assert_eq!(counter_delta(), CounterDelta::default(), "reset establishes a zero baseline");

    mutate(&mut session, scenario);
    let after = session.clone();
    let delta = counter_delta();
    assert_eq!(session, after, "diagnostic reads do not mutate the session");
    assert_ne!(session, unchanged, "each baseline scenario performs a semantic edit");
    assert_eq!(
        delta.edit_scoped_attempts,
        u64::from(scenario.expects_scoped_validation()),
        "only a proven single-owner candidate enters scoped validation",
    );
    assert_eq!(
        delta.edit_scoped_accepts,
        delta.edit_scoped_attempts,
        "eligible single-owner edits are scoped only when the validator proves them",
    );
    assert_eq!(
        delta.edit_full_fallbacks,
        u64::from(scenario.expects_scoped_fallback()),
        "uncertain or structural single-paragraph requests fall back exactly once",
    );
    if scenario.expects_scoped_validation() && !matches!(scenario, EditScenario::UndoRedo) {
        assert_eq!(delta.edit_full_validations, 0);
    } else {
        assert!(delta.edit_full_validations > 0);
    }
    if scenario.is_structural() {
        assert!(
            delta.body_survivors_visited > 0 || delta.body_owners_removed > 0,
            "structural baseline records body reconciliation work"
        );
    }
    (session.project().clone(), delta)
}

fn run_unchanged_layout_lookup(project: &Project) -> (bool, LayoutBuildDiagnostics, CounterDelta) {
    let mut cache = LayoutCache::default();
    let inputs = Default::default();
    let mut warmup = LayoutBuildDiagnostics::default();
    let (warm_layout, _) = valid(cache.get_or_build_with_inputs_for_purpose_and_diagnostics(
        0,
        project,
        &inputs,
        LayoutPurpose::Editor,
        &mut warmup,
    ));

    reset_counters();
    let before = project.clone();
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let (layout, hit) = valid(cache.get_or_build_with_inputs_for_purpose_and_diagnostics(
        0,
        project,
        &inputs,
        LayoutPurpose::Editor,
        &mut diagnostics,
    ));
    let delta = counter_delta();
    assert_eq!(project, &before, "layout diagnostics leave the project unchanged");
    assert!(
        std::sync::Arc::ptr_eq(&warm_layout, &layout),
        "unchanged lookup retains layout identity"
    );
    (hit, diagnostics, delta)
}

fn assert_unchanged_session_layout_hits(project: &Project) {
    let session = valid(EditingSession::new(project.clone()));
    let presentation = BodyEditorPresentation::for_project(session.project(), None);

    let mut body_cache = LayoutCache::default();
    let (body_layout, body_hit) = valid(
        body_cache.get_or_build_body_editor_for_session_with_diagnostics(
            &session,
            &presentation,
            &mut LayoutBuildDiagnostics::default(),
        ),
    );
    assert!(!body_hit);
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let (cached_body, body_hit) = valid(
        body_cache.get_or_build_body_editor_for_session_with_diagnostics(
            &session,
            &presentation,
            &mut diagnostics,
        ),
    );
    assert!(body_hit);
    assert!(std::sync::Arc::ptr_eq(&body_layout, &cached_body));
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.layout_builds, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);

    let mut title_cache = LayoutCache::default();
    let (title_layout, title_hit) = valid(
        title_cache.get_or_build_title_page_editor_for_session_with_diagnostics(
            &session,
            &mut LayoutBuildDiagnostics::default(),
        ),
    );
    assert!(!title_hit);
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let (cached_title, title_hit) = valid(
        title_cache.get_or_build_title_page_editor_for_session_with_diagnostics(
            &session,
            &mut diagnostics,
        ),
    );
    assert!(title_hit);
    assert!(std::sync::Arc::ptr_eq(&title_layout, &cached_title));
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.layout_builds, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
}

#[test]
fn editing_baseline_matrix_records_value_free_counter_deltas() {
    for page_scale in PAGE_SCALES {
        let item_count = page_scale * ITEMS_PER_PAGE_SCALE;
        for target in Target::ALL {
            let (backed, source_less) = fixture_pair(page_scale, target);
            assert_eq!(backed.elements.len(), item_count + 1);
            assert_canonical_parity(&backed, &source_less);
            assert!(backed.preservation.is_some());
            assert!(source_less.preservation.is_none());

            for scenario in EditScenario::ALL {
                // Compare diagnostics only after canonical parity has been
                // established for the equivalent source-less/source-backed pair.
                let (backed_after, backed_delta) =
                    run_edit_scenario(backed.clone(), scenario, target, item_count);
                let (source_less_after, source_less_delta) =
                    run_edit_scenario(source_less.clone(), scenario, target, item_count);
                assert_canonical_parity(&backed_after, &source_less_after);
                if matches!(scenario, EditScenario::CharacterInsertion) {
                    assert_eq!(backed_delta.managed_index_rebuilds, 0);
                    assert_eq!(backed_delta.source_use_index_rebuilds, 0);
                    assert_eq!(backed_delta.source_text_byte_delta_updates, 1);
                }
                assert_eq!(source_less_delta.managed_index_rebuilds, 0);
                assert_eq!(source_less_delta.source_use_index_rebuilds, 0);
                assert_eq!(source_less_delta.body_survivors_visited, 0);
                assert_eq!(source_less_delta.title_survivors_visited, 0);
                if scenario.is_structural() {
                    assert!(
                        backed_delta.body_survivors_visited > 0
                            || backed_delta.body_owners_removed > 0,
                        "source-backed structural edit reports reconciliation counters"
                    );
                }
            }

            for project in [&backed, &source_less] {
                let (hit, diagnostics, delta) = run_unchanged_layout_lookup(project);
                assert!(hit, "the unchanged lookup is a cache hit");
                assert_eq!(diagnostics.layout_builds, 0);
                assert_eq!(diagnostics.semantic_signature_constructions, 1);
                assert_eq!(delta.project_validations, 1);
                assert_eq!(delta.edit_full_validations, 0);
                assert_eq!(delta.body_survivors_visited, 0);
                assert_unchanged_session_layout_hits(project);
            }
        }
    }
}

#[test]
fn scoped_edit_publication_preserves_history_noop_dirty_and_redo_semantics() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    let initial_revision = session.revision();

    reset_counters();
    valid(session.insert("first"));
    let insertion = counter_delta();
    assert!(session.last_edit_used_scoped_validation());
    assert_eq!(insertion.edit_scoped_attempts, 1);
    assert_eq!(insertion.edit_scoped_accepts, 1);
    assert_eq!(insertion.edit_full_validations, 0);
    assert_eq!(insertion.edit_full_fallbacks, 0);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert_eq!(session.revision(), initial_revision + 1);

    valid(session.undo());
    assert!(!session.dirty());
    assert!(session.can_redo());
    assert!(!session.last_edit_used_scoped_validation());
    valid(session.redo());
    assert!(session.dirty());
    assert!(!session.can_redo());
    assert!(!session.last_edit_used_scoped_validation());

    valid(session.undo());
    valid(session.insert("replacement"));
    assert!(session.dirty());
    assert!(!session.can_redo(), "a committed scoped edit clears redo history");

    let before_noop = session.clone();
    let revision_before_noop = session.revision();
    reset_counters();
    valid(session.insert(""));
    let no_op = counter_delta();
    assert_eq!(session, before_noop);
    assert_eq!(session.revision(), revision_before_noop);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert!(!session.can_redo());
    assert!(!session.last_edit_used_scoped_validation());
    assert_eq!(no_op.edit_scoped_attempts, 0);
    assert_eq!(no_op.edit_scoped_accepts, 0);
    assert_eq!(no_op.edit_full_fallbacks, 1);
    assert_eq!(no_op.edit_full_validations, 1);
}

#[test]
fn formatting_role_title_and_structural_families_retain_complete_validation() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("ordinary"));
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: "ordinary".len() },
    }));

    for operation in [
        0_u8, // run-style formatting
        1,     // paragraph alignment
        2,     // role change
    ] {
        reset_counters();
        match operation {
            0 => valid(session.toggle_body_run_style(scriptcraft_core::RunStyle::Bold)),
            1 => valid(session.set_body_alignment(
                scriptcraft_core::ParagraphAlignment::Right,
            )),
            _ => valid(session.change_role(scriptcraft_core::Role::General)),
        }
        let delta = counter_delta();
        assert!(!session.last_edit_used_scoped_validation());
        assert_eq!(delta.edit_scoped_accepts, 0);
        assert!(delta.edit_full_validations > 0);
        if operation == 2 {
            assert_eq!(delta.edit_full_fallbacks, 1);
        } else {
            assert_eq!(delta.edit_full_fallbacks, 0);
        }
    }

    reset_counters();
    valid(session.set_title_selection(Selection::caret(Position { element: 17, offset: 0 })));
    valid(session.insert_title("title"));
    let title = counter_delta();
    assert!(!session.last_edit_used_scoped_validation());
    assert_eq!(title.edit_scoped_attempts, 0);
    assert_eq!(title.edit_full_fallbacks, 0);
    assert!(title.edit_full_validations > 0);
}

#[test]
fn body_reconciliation_counter_coverage_scales_for_deletion_and_mixed_replacement() {
    let item_count = 50 * ITEMS_PER_PAGE_SCALE;
    for target in Target::ALL {
        let (backed, _) = fixture_pair(50, target);
        let (_, deletion) = run_edit_scenario(
            backed.clone(),
            EditScenario::EmptyElementDeletion,
            target,
            item_count,
        );
        assert_eq!(deletion.body_owners_removed, 1);
        assert_eq!(deletion.body_survivors_visited, item_count as u64);
        assert_eq!(deletion.body_unchanged_survivors_skipped, item_count as u64);
        assert_eq!(deletion.body_owners_projected, 0);
        assert_eq!(deletion.body_owners_inserted, 0);
        assert_eq!(deletion.managed_index_rebuilds, 1);
        assert_eq!(deletion.source_use_index_rebuilds, 0);

        let (_, join) = run_edit_scenario(
            backed.clone(),
            EditScenario::SameRoleJoin,
            target,
            item_count,
        );
        assert_eq!(join.body_owners_removed, 1);
        assert_eq!(join.body_survivors_visited, item_count as u64);
        assert_eq!(join.body_unchanged_survivors_skipped, (item_count - 1) as u64);
        assert_eq!(join.body_owners_projected, 1);
        assert_eq!(join.managed_index_rebuilds, 1);
        assert_eq!(join.source_use_index_rebuilds, 0);

        let (_, split) = run_edit_scenario(
            backed.clone(),
            EditScenario::Split,
            target,
            item_count,
        );
        assert_eq!(split.body_owners_removed, 0);
        assert_eq!(split.body_owners_inserted, 1);
        assert_eq!(split.body_survivors_visited, (item_count + 1) as u64);
        assert_eq!(split.body_unchanged_survivors_skipped, item_count as u64);
        assert_eq!(split.body_owners_projected, 2);

        let (_, replacement) = run_edit_scenario(
            backed,
            EditScenario::RangedReplacement,
            target,
            item_count,
        );
        assert_eq!(replacement.body_owners_removed, 0);
        assert_eq!(replacement.body_survivors_visited, (item_count + 1) as u64);
        assert_eq!(replacement.body_unchanged_survivors_skipped, item_count as u64);
        assert_eq!(replacement.body_owners_projected, 1);
        assert_eq!(replacement.managed_index_rebuilds, 0);
        assert_eq!(replacement.source_use_index_rebuilds, 0);
    }
}

#[test]
fn title_reconciliation_counter_coverage_scales_for_large_structural_deletion() {
    const OWNER_COUNT: usize = 96;
    let empty_index = OWNER_COUNT / 2;
    let mut session = valid(EditingSession::new(title_fixture(OWNER_COUNT, empty_index)));
    valid(session.set_title_selection(Selection::caret(Position {
        element: empty_index,
        offset: 0,
    })));

    reset_counters();
    valid(session.delete_title());
    let delta = counter_delta();

    assert_eq!(delta.title_owners_removed, 1);
    assert_eq!(delta.title_survivors_visited, (OWNER_COUNT - 1) as u64);
    assert_eq!(delta.title_unchanged_survivors_skipped, (OWNER_COUNT - 1) as u64);
    assert_eq!(delta.title_owners_projected, 0);
    assert_eq!(delta.title_owners_inserted, 0);
    assert_eq!(delta.managed_index_rebuilds, 1);
    assert_eq!(delta.source_use_index_rebuilds, 0);
    assert_eq!(delta.source_text_byte_delta_updates, 0);
    assert_eq!(session.project().title_page.len(), OWNER_COUNT - 1);
}
