//! Enter/Tab role decisions and generated-parenthetical policy.

use super::selection::SelectionOrdering;
use super::{EditError, EditPolicy, Position, Selection};
use scriptcraft_model::{formatting, Project, Role};
use std::collections::BTreeSet;

/// Read-only outcome for a paragraph-key projection. A `Target` is the role
/// resolved by editing policy, not a promise that later safety checks will
/// allow the candidate to publish. `NoOp` describes a consumed key with no
/// semantic target; `Unavailable` describes a key that cannot resolve for the
/// current selection (or whose editing resolver rejects the request).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphKeyTarget {
    Target(scriptcraft_model::Role),
    NoOp,
    Unavailable,
}

/// Actual Enter and Tab target decisions for the session's current selection
/// and editing policy. This value is a nonmutating hint, not a preview edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParagraphKeyTargets {
    pub enter: ParagraphKeyTarget,
    pub tab: ParagraphKeyTarget,
}

fn strict_boundary_rejects(
    project: &Project,
    policy: &EditPolicy,
    element: usize,
    role: Role,
    text_len: usize,
    offset: usize,
) -> bool {
    policy.strict_transition_boundaries
        && matches!(
            role,
            Role::SceneHeading | Role::Character | Role::Transition
        )
        && project.elements.get(element).is_some()
        && offset != text_len
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EnterDecision {
    pub(super) target: ParagraphKeyTarget,
    pub(super) action: EnterAction,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum EnterAction {
    NoOp,
    InsertParentheticalBefore { element: usize, automatic: bool },
    ClearGeneratedAndSplit { position: Position, role: Role },
    RepairAndInsertDialogue { element: usize },
    Split { position: Position, role: Role },
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TabDecision {
    pub(super) target: ParagraphKeyTarget,
    pub(super) action: TabAction,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TabAction {
    NoOp,
    DelimiterParentheticalToDialogue {
        element: usize,
    },
    RepairAndInsertDialogue {
        element: usize,
    },
    SetRole {
        position: Position,
        role: Role,
        clear_generated_parenthetical: bool,
    },
}

/// The shared Enter decision is intentionally value-only. Editing executes its
/// action after this point; hint projection reads only `target`.
pub(super) fn resolve_enter(
    project: &Project,
    selection: Selection,
    generated: &BTreeSet<String>,
    policy: &EditPolicy,
) -> Result<EnterDecision, EditError> {
    let (start, end) = selection.ordered();
    if start != end {
        return Err(EditError::InvalidSelection);
    }
    let item = project
        .elements
        .get(start.element)
        .ok_or(EditError::InvalidElement)?;
    if strict_boundary_rejects(
        project,
        policy,
        start.element,
        item.transitional_role(),
        item.text.len(),
        start.offset,
    ) {
        return Ok(EnterDecision {
            target: ParagraphKeyTarget::NoOp,
            action: EnterAction::NoOp,
        });
    }

    let mut position = start;
    if item.transitional_role() == Role::Parenthetical {
        if position.offset == 0 && item.text.starts_with('(') {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(Role::Parenthetical),
                action: EnterAction::InsertParentheticalBefore {
                    element: position.element,
                    automatic: policy.auto_parenthetical,
                },
            });
        }
        if empty_parenthetical_at(item.text.as_str(), position.offset)
            && generated.contains(item.id.as_str())
        {
            position.offset = 0;
            let current_role = item.transitional_role();
            let role = enter_transition(current_role)
                .ok_or(EditError::UnsupportedTransition { from: current_role })?;
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(role),
                action: EnterAction::ClearGeneratedAndSplit { position, role },
            });
        }
        if parenthetical_repair_boundary(item.text.as_str(), position.offset).is_some() {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(Role::Dialogue),
                action: EnterAction::RepairAndInsertDialogue {
                    element: position.element,
                },
            });
        }
        if !item.text.is_empty() {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::NoOp,
                action: EnterAction::NoOp,
            });
        }
    }

    let current_role = item.transitional_role();
    let role = if current_role == Role::Dialogue && position.offset < item.text.len() {
        Role::Dialogue
    } else {
        enter_transition(current_role)
            .ok_or(EditError::UnsupportedTransition { from: current_role })?
    };
    Ok(EnterDecision {
        target: ParagraphKeyTarget::Target(role),
        action: EnterAction::Split { position, role },
    })
}

/// The shared Tab decision preserves consumed no-ops and same-role displayable
/// targets separately. Its action is also the sole source for real Tab edits.
pub(super) fn resolve_tab(
    project: &Project,
    selection: Selection,
    generated: &BTreeSet<String>,
    policy: &EditPolicy,
) -> Result<TabDecision, EditError> {
    let (start, end) = selection.ordered();
    if start != end {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::Unavailable,
            action: TabAction::NoOp,
        });
    }
    let item = project
        .elements
        .get(start.element)
        .ok_or(EditError::InvalidElement)?;
    if strict_boundary_rejects(
        project,
        policy,
        start.element,
        item.transitional_role(),
        item.text.len(),
        start.offset,
    ) {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        });
    }

    let mut position = start;
    let mut clear_generated_parenthetical = false;
    if policy.auto_parenthetical && item.transitional_role() == Role::Parenthetical {
        if is_delimiter_only_parenthetical(item.text.as_str()) {
            return Ok(TabDecision {
                target: ParagraphKeyTarget::Target(Role::Dialogue),
                action: TabAction::DelimiterParentheticalToDialogue {
                    element: position.element,
                },
            });
        }
        if empty_parenthetical_at(item.text.as_str(), position.offset)
            && generated.contains(item.id.as_str())
        {
            position.offset = 0;
            clear_generated_parenthetical = true;
        } else if parenthetical_repair_boundary(item.text.as_str(), position.offset).is_some() {
            return Ok(TabDecision {
                target: ParagraphKeyTarget::Target(Role::Dialogue),
                action: TabAction::RepairAndInsertDialogue {
                    element: position.element,
                },
            });
        } else if !item.text.is_empty() {
            return Ok(TabDecision {
                target: ParagraphKeyTarget::NoOp,
                action: TabAction::NoOp,
            });
        }
    }

    let role = tab_transition(item.transitional_role(), item.text.is_empty());
    Ok(match role {
        Some(role) => TabDecision {
            target: ParagraphKeyTarget::Target(role),
            action: TabAction::SetRole {
                position,
                role,
                clear_generated_parenthetical,
            },
        },
        None => TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        },
    })
}

fn empty_parenthetical_at(text: &str, offset: usize) -> bool {
    text == "()" && offset <= 2
}

pub(super) fn is_delimiter_only_parenthetical(text: &str) -> bool {
    matches!(text, "(" | "()" | ")")
}

fn parenthetical_repair_boundary(text: &str, offset: usize) -> Option<usize> {
    let closed_pair_inner_end = text.starts_with('(')
        && text.ends_with(')')
        && text.len() > 2
        && offset == text.len().saturating_sub(1);
    let accepted = closed_pair_inner_end
        || (text.starts_with('(') && !text.ends_with(')') && text.len() > 1)
        || (!text.starts_with('(') && text.ends_with(')') && text.len() > 1)
        || (!text.starts_with('(') && !text.ends_with(')') && !text.is_empty());
    let at_supported_boundary = closed_pair_inner_end || offset == text.len();
    if !accepted || !at_supported_boundary {
        return None;
    }
    Some(match (text.starts_with('('), text.ends_with(')')) {
        (true, false) => text.len() + 1,
        (false, true) => text.len(),
        (false, false) => text.len() + 1,
        (true, true) => text.len(),
    })
}

pub(super) fn repair_parenthetical(text: &str) -> String {
    match (text.starts_with('('), text.ends_with(')')) {
        (true, false) => format!("{text})"),
        (false, true) => format!("({text}"),
        (false, false) => format!("({text})"),
        (true, true) => text.to_owned(),
    }
}

/// Compatibility alias for the shared native ReturnKey application default.
pub fn transition(role: Role) -> Option<Role> {
    enter_transition(role)
}

pub fn enter_transition(role: Role) -> Option<Role> {
    role_for_native_behavior_target(formatting::formatting_for_role(role).behavior.return_key)
}

/// Returns the shared native TabKey application default, with the established
/// empty-Character condition retained as an application-specific boundary.
pub fn tab_transition(role: Role, current_text_is_empty: bool) -> Option<Role> {
    if role == Role::Character && !current_text_is_empty {
        return None;
    }
    role_for_native_behavior_target(formatting::formatting_for_role(role).behavior.tab_key)
}

fn role_for_native_behavior_target(target: &str) -> Option<Role> {
    formatting::formatting_in_native_order()
        .iter()
        .find(|formatting| formatting.fdx_label == target)
        .map(|formatting| formatting.role)
}
