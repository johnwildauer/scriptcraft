//! Read-only summaries projected from the current document definition registry.

use crate::chrome::shortcuts::ShortcutPlatform;
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::{
    BehaviorTargetResolution, ElementDefinition, ElementTypeKey, ParagraphBehaviorKey, Project,
};

/// Display-ready font values from one complete current definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FontSummary {
    pub(crate) family: String,
    pub(crate) size: u8,
    pub(crate) active_flags: Vec<&'static str>,
    pub(crate) color: Option<String>,
}

/// Display-ready paragraph values from one complete current definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParagraphSummary {
    pub(crate) alignment: &'static str,
    pub(crate) first_indent: String,
    pub(crate) left_indent: String,
    pub(crate) right_indent: String,
    pub(crate) leading: String,
    pub(crate) spacing: u8,
    pub(crate) space_before: String,
    pub(crate) starts_new_page: bool,
}

/// Display-ready Behavior values resolved only against the same current Project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BehaviorSummary {
    pub(crate) paginate_as: Option<String>,
    pub(crate) return_target: String,
    pub(crate) tab_target: String,
    pub(crate) shortcut: Option<String>,
}

/// One frame-local presentation value for an exact current registry entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElementDefinitionSummary {
    pub(crate) key: ElementTypeKey,
    pub(crate) font: FontSummary,
    pub(crate) paragraph: ParagraphSummary,
    pub(crate) behavior: BehaviorSummary,
    pub(crate) outline: Option<String>,
}

/// Projects all definitions afresh in exact current registry order.
///
/// This is intentionally an owned frame-local view, not a cached registry. It
/// does not inspect direct paragraph/run overrides or prepare host fonts.
pub(crate) fn element_definition_summaries(
    project: &Project,
    platform: ShortcutPlatform,
) -> Vec<ElementDefinitionSummary> {
    project
        .element_definitions()
        .iter()
        .map(|definition| summarize_definition(project, definition, platform))
        .collect()
}

fn summarize_definition(
    project: &Project,
    definition: &ElementDefinition,
    platform: ShortcutPlatform,
) -> ElementDefinitionSummary {
    let font = &definition.font;
    let mut active_flags = Vec::new();
    if font.bold {
        active_flags.push("Bold");
    }
    if font.italic {
        active_flags.push("Italic");
    }
    if font.underline {
        active_flags.push("Underline");
    }
    if font.all_caps {
        active_flags.push("All caps");
    }
    if font.hidden_text {
        active_flags.push("Hidden");
    }

    let paragraph = &definition.paragraph;
    let behavior = &definition.behavior;
    let shortcut = match platform {
        ShortcutPlatform::MacOS => behavior.shortcut.as_deref(),
        ShortcutPlatform::Windows => behavior.win_shortcut.as_deref(),
        ShortcutPlatform::Other => None,
    }
    .filter(|value| !value.trim().is_empty())
    .map(str::to_owned);

    let outline = (definition.outline.level > 0 || definition.outline.can_hide).then(|| {
        let mut values = Vec::new();
        if definition.outline.level > 0 {
            values.push(format!("level {}", definition.outline.level));
        }
        if definition.outline.can_hide {
            values.push("hide supported".to_owned());
        }
        values.join(", ")
    });

    ElementDefinitionSummary {
        key: definition.key.clone(),
        font: FontSummary {
            family: font.family.clone(),
            size: font.size,
            active_flags,
            color: font.color.clone(),
        },
        paragraph: ParagraphSummary {
            alignment: match paragraph.alignment {
                scriptcraft_model::formatting::ParagraphAlignment::Left => "Left",
                scriptcraft_model::formatting::ParagraphAlignment::Right => "Right",
                scriptcraft_model::formatting::ParagraphAlignment::Center => "Center",
                scriptcraft_model::formatting::ParagraphAlignment::Justified => "Justified",
            },
            first_indent: inches_string(paragraph.first_indent),
            left_indent: inches_string(paragraph.left_indent),
            right_indent: inches_string(paragraph.right_indent),
            leading: paragraph.leading.clone(),
            spacing: paragraph.spacing,
            space_before: paragraph.space_before.decimal_string(),
            starts_new_page: paragraph.starts_new_page,
        },
        behavior: BehaviorSummary {
            paginate_as: behavior.paginate_as.clone(),
            return_target: behavior_target_summary(
                project,
                &definition.key,
                ParagraphBehaviorKey::Return,
            ),
            tab_target: behavior_target_summary(
                project,
                &definition.key,
                ParagraphBehaviorKey::Tab,
            ),
            shortcut,
        },
        outline,
    }
}

fn inches_string(value: FixedPoint) -> String {
    // FDX inch values are stored in 1/7200-inch units. Keep the display
    // conversion integer-only and render at the parser's six-digit precision.
    const RAW_UNITS_PER_INCH: u64 = 7_200;
    const DECIMAL_SCALE: u64 = 1_000_000;

    let raw = value.raw();
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let mut whole = magnitude / RAW_UNITS_PER_INCH;
    let remainder = magnitude % RAW_UNITS_PER_INCH;
    let mut fraction = (remainder * DECIMAL_SCALE + RAW_UNITS_PER_INCH / 2) / RAW_UNITS_PER_INCH;

    if fraction == DECIMAL_SCALE {
        whole += 1;
        fraction = 0;
    }

    let sign = if negative { "-" } else { "" };
    if fraction == 0 {
        format!("{sign}{whole}")
    } else {
        let fraction = format!("{fraction:06}");
        format!("{sign}{whole}.{}", fraction.trim_end_matches('0'))
    }
}

fn behavior_target_summary(
    project: &Project,
    source: &scriptcraft_model::ElementTypeKey,
    behavior: ParagraphBehaviorKey,
) -> String {
    match project.resolve_paragraph_behavior_target(source, behavior) {
        Some(BehaviorTargetResolution::Resolved(target)) => target.as_str().to_owned(),
        Some(BehaviorTargetResolution::Absent) => "Absent".to_owned(),
        Some(BehaviorTargetResolution::Unresolved(target)) => {
            format!("{} (unresolved)", target.as_str())
        }
        None => "Unresolved source definition".to_owned(),
    }
}
