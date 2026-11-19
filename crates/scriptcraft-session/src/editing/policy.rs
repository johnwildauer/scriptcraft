//! Enter/Tab Behavior decisions and generated-parenthetical policy.

use super::selection::SelectionOrdering;
use super::{EditError, EditPolicy, Position, Selection};
use scriptcraft_model::{BehaviorTargetResolution, ElementTypeKey, ParagraphBehaviorKey, Project};
use std::collections::BTreeSet;

/// Read-only outcome for a paragraph-key projection. A `Target` is the exact
/// open key resolved by editing policy, not a promise that later safety checks will
/// allow the candidate to publish. `NoOp` describes a consumed key with no
/// semantic target; `Unavailable` describes a key that cannot resolve for the
/// current selection (or whose editing resolver rejects the request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphKeyTarget {
    Target(ElementTypeKey),
    NoOp,
    Unavailable,
}

/// Actual Enter and Tab target decisions for the session's current selection
/// and editing policy. This value is a nonmutating hint, not a preview edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphKeyTargets {
    pub enter: ParagraphKeyTarget,
    pub tab: ParagraphKeyTarget,
}

fn strict_boundary_rejects(
    policy: &EditPolicy,
    capabilities: scriptcraft_model::SemanticCapabilities,
    text_len: usize,
    offset: usize,
) -> bool {
    policy.strict_transition_boundaries
        && (capabilities.scene_boundary || capabilities.character_cue || capabilities.transition)
        && offset != text_len
}

fn resolved_target(
    project: &Project,
    source: &ElementTypeKey,
    behavior: ParagraphBehaviorKey,
) -> Option<ElementTypeKey> {
    match project.resolve_paragraph_behavior_target(source, behavior)? {
        BehaviorTargetResolution::Resolved(target) => Some(target),
        BehaviorTargetResolution::Absent | BehaviorTargetResolution::Unresolved(_) => None,
    }
}

#[derive(Debug, Clone)]
pub(super) struct EnterDecision {
    pub(super) target: ParagraphKeyTarget,
    pub(super) action: EnterAction,
}

#[derive(Debug, Clone)]
pub(super) enum EnterAction {
    NoOp,
    InsertParentheticalBefore {
        element: usize,
        automatic: bool,
    },
    ClearGeneratedAndSplit {
        position: Position,
        element_type: ElementTypeKey,
    },
    RepairAndInsert {
        element: usize,
        element_type: ElementTypeKey,
    },
    Split {
        position: Position,
        element_type: ElementTypeKey,
    },
}

#[derive(Debug, Clone)]
pub(super) struct TabDecision {
    pub(super) target: ParagraphKeyTarget,
    pub(super) action: TabAction,
}

#[derive(Debug, Clone)]
pub(super) enum TabAction {
    NoOp,
    DelimiterParentheticalToType {
        element: usize,
        element_type: ElementTypeKey,
    },
    RepairAndInsert {
        element: usize,
        element_type: ElementTypeKey,
    },
    SetElementType {
        position: Position,
        element_type: ElementTypeKey,
        clear_generated_parenthetical: bool,
    },
}

/// The shared Enter decision is value-only. Execution applies this action and
/// read-only guidance projects the exact same target.
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
    let source_key = item.element_type();
    let Some(semantics) = project.element_semantics(&source_key) else {
        return Ok(EnterDecision {
            target: ParagraphKeyTarget::NoOp,
            action: EnterAction::NoOp,
        });
    };
    let capabilities = semantics.capabilities();
    if strict_boundary_rejects(policy, capabilities, item.text.len(), start.offset) {
        return Ok(EnterDecision {
            target: ParagraphKeyTarget::NoOp,
            action: EnterAction::NoOp,
        });
    }

    let mut position = start;
    if capabilities.parenthetical {
        if position.offset == 0 && item.text.starts_with('(') {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(source_key),
                action: EnterAction::InsertParentheticalBefore {
                    element: position.element,
                    automatic: policy.auto_parenthetical,
                },
            });
        }
        if policy.auto_parenthetical
            && empty_parenthetical_at(item.text.as_str(), position.offset)
            && generated.contains(item.id.as_str())
        {
            position.offset = 0;
            let Some(element_type) =
                resolved_target(project, &source_key, ParagraphBehaviorKey::Return)
            else {
                return Ok(EnterDecision {
                    target: ParagraphKeyTarget::NoOp,
                    action: EnterAction::NoOp,
                });
            };
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(element_type.clone()),
                action: EnterAction::ClearGeneratedAndSplit {
                    position,
                    element_type,
                },
            });
        }
        if parenthetical_repair_boundary(item.text.as_str(), position.offset).is_some() {
            let Some(element_type) =
                resolved_target(project, &source_key, ParagraphBehaviorKey::Return)
            else {
                return Ok(EnterDecision {
                    target: ParagraphKeyTarget::NoOp,
                    action: EnterAction::NoOp,
                });
            };
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::Target(element_type.clone()),
                action: EnterAction::RepairAndInsert {
                    element: position.element,
                    element_type,
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

    let element_type = if capabilities.dialogue && position.offset < item.text.len() {
        source_key
    } else {
        let Some(target) = resolved_target(project, &source_key, ParagraphBehaviorKey::Return)
        else {
            return Ok(EnterDecision {
                target: ParagraphKeyTarget::NoOp,
                action: EnterAction::NoOp,
            });
        };
        target
    };
    Ok(EnterDecision {
        target: ParagraphKeyTarget::Target(element_type.clone()),
        action: EnterAction::Split {
            position,
            element_type,
        },
    })
}

/// The shared Tab decision preserves consumed no-ops and resolved open-key
/// targets separately. Its action is the sole source for real Tab edits.
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
    let source_key = item.element_type();
    let Some(semantics) = project.element_semantics(&source_key) else {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        });
    };
    let capabilities = semantics.capabilities();
    if strict_boundary_rejects(policy, capabilities, item.text.len(), start.offset) {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        });
    }

    let mut position = start;
    let mut clear_generated_parenthetical = false;
    if policy.auto_parenthetical && capabilities.parenthetical {
        if is_delimiter_only_parenthetical(item.text.as_str()) {
            let Some(element_type) =
                resolved_target(project, &source_key, ParagraphBehaviorKey::Tab)
            else {
                return Ok(TabDecision {
                    target: ParagraphKeyTarget::NoOp,
                    action: TabAction::NoOp,
                });
            };
            if element_type == source_key {
                return Ok(TabDecision {
                    target: ParagraphKeyTarget::Target(element_type),
                    action: TabAction::NoOp,
                });
            }
            return Ok(TabDecision {
                target: ParagraphKeyTarget::Target(element_type.clone()),
                action: TabAction::DelimiterParentheticalToType {
                    element: position.element,
                    element_type,
                },
            });
        }
        if empty_parenthetical_at(item.text.as_str(), position.offset)
            && generated.contains(item.id.as_str())
        {
            position.offset = 0;
            clear_generated_parenthetical = true;
        } else if parenthetical_repair_boundary(item.text.as_str(), position.offset).is_some() {
            let Some(element_type) =
                resolved_target(project, &source_key, ParagraphBehaviorKey::Tab)
            else {
                return Ok(TabDecision {
                    target: ParagraphKeyTarget::NoOp,
                    action: TabAction::NoOp,
                });
            };
            return Ok(TabDecision {
                target: ParagraphKeyTarget::Target(element_type.clone()),
                action: TabAction::RepairAndInsert {
                    element: position.element,
                    element_type,
                },
            });
        } else if !item.text.is_empty() {
            return Ok(TabDecision {
                target: ParagraphKeyTarget::NoOp,
                action: TabAction::NoOp,
            });
        }
    }

    if capabilities.character_cue && !item.text.is_empty() {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        });
    }
    let Some(element_type) = resolved_target(project, &source_key, ParagraphBehaviorKey::Tab)
    else {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::NoOp,
            action: TabAction::NoOp,
        });
    };
    if element_type == source_key {
        return Ok(TabDecision {
            target: ParagraphKeyTarget::Target(element_type),
            action: TabAction::NoOp,
        });
    }
    Ok(TabDecision {
        target: ParagraphKeyTarget::Target(element_type.clone()),
        action: TabAction::SetElementType {
            position,
            element_type,
            clear_generated_parenthetical,
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
