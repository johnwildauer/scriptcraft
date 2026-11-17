//! Transitional dual-dialogue re-exports for core editing, layout, and FDX consumers.

pub use scriptcraft_model::dual_dialogue::{
    caret_side, classify_structural_result, first_member, group_at_element_id,
    group_at_element_index, group_order, last_member, left_flat_range, map_group_identity,
    recognize_candidate, resolve_toggle_target, right_flat_range, side_at_element_index,
    CandidatePattern, DualDialogueError, DualDialogueGroup, DualDialogueSide,
    DualDialogueSideKind, DualDialogueWrapperProvenance, SelectionSurface,
    StructuralGroupClassification, StructuralGroupOutcome, ToggleTarget,
};

pub(crate) use scriptcraft_model::dual_dialogue::{
    insert_group_member_at_flat_index, prune_removed_group_paragraphs,
    remove_staged_group_member,
};

/// Transitional adapter for the session's construction/publication-validated project.
/// Unlike the arbitrary-project API, this preserves the session's no-extra-full-validation path.
pub(crate) fn resolve_toggle_target_for_validated_session(
    project: &scriptcraft_model::Project,
    surface: SelectionSurface,
    selection: scriptcraft_model::Selection,
) -> Result<Option<ToggleTarget>, scriptcraft_model::ModelError> {
    scriptcraft_model::dual_dialogue::resolve_toggle_target_for_validated_session(
        project, surface, selection,
    )
}
