//! Read-only summaries projected from the current document definition registry.

use crate::chrome::shortcuts::ShortcutPlatform;
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_model::{
    BehaviorTargetResolution, ElementDefinition, ParagraphBehaviorKey, Project,
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
    pub(crate) key: String,
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
        active_flags.push("bold");
    }
    if font.italic {
        active_flags.push("italic");
    }
    if font.underline {
        active_flags.push("underline");
    }
    if font.all_caps {
        active_flags.push("all caps");
    }
    if font.hidden_text {
        active_flags.push("hidden");
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
        key: definition.key.as_str().to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
    use scriptcraft_model::{
        ElementBehavior, ElementFontSpec, ElementOutline, ElementParagraphSpec, ElementTypeKey,
    };

    fn definition(key: &str) -> ElementDefinition {
        ElementDefinition {
            key: ElementTypeKey::new(key).expect("test key is valid"),
            font: ElementFontSpec {
                family: "Declared Family".to_owned(),
                size: 12,
                bold: false,
                italic: false,
                underline: false,
                all_caps: false,
                hidden_text: false,
                color: None,
                adornment_style: 0,
                revision_id: 0,
            },
            paragraph: ElementParagraphSpec {
                alignment: ParagraphAlignment::Left,
                first_indent: FixedPoint::from_points(0),
                left_indent: FixedPoint::from_points(0),
                right_indent: FixedPoint::from_points(0),
                leading: "Regular".to_owned(),
                space_before: FixedPoint::from_points(0),
                spacing: 1,
                starts_new_page: false,
            },
            behavior: ElementBehavior {
                paginate_as: None,
                return_key: None,
                shortcut: Some(String::new()),
                tab_key: None,
                win_shortcut: Some(String::new()),
            },
            outline: ElementOutline {
                can_hide: false,
                level: 0,
            },
        }
    }

    fn project(definitions: Vec<ElementDefinition>) -> Project {
        let mut project = Project::empty_projection().expect("empty projection is valid");
        project
            .replace_element_definitions(definitions)
            .expect("test definitions are valid");
        project
    }

    #[test]
    fn summaries_cover_complete_values_and_preserve_registry_case_and_order() {
        let mut lyrics = definition("Lyrics");
        lyrics.font.family = "Times New Roman".to_owned();
        lyrics.font.size = 14;
        lyrics.font.bold = true;
        lyrics.font.italic = true;
        lyrics.font.underline = true;
        lyrics.font.all_caps = true;
        lyrics.font.hidden_text = true;
        lyrics.font.color = Some("#87878F8FE0E0".to_owned());
        lyrics.paragraph.alignment = ParagraphAlignment::Justified;
        lyrics.paragraph.first_indent = FixedPoint::parse_inches("-2").expect("inches parse");
        lyrics.paragraph.left_indent = FixedPoint::parse_inches("1").expect("inches parse");
        lyrics.paragraph.right_indent = FixedPoint::parse_inches("3").expect("inches parse");
        lyrics.paragraph.leading = "Custom leading".to_owned();
        lyrics.paragraph.spacing = 2;
        lyrics.paragraph.space_before = FixedPoint::parse_points("4.5").expect("points parse");
        lyrics.paragraph.starts_new_page = true;
        lyrics.behavior.paginate_as = Some("Action".to_owned());
        lyrics.behavior.return_key = Some(ElementTypeKey::new("later").expect("key"));
        lyrics.behavior.tab_key = Some(ElementTypeKey::new("Lyrics").expect("key"));
        lyrics.behavior.shortcut = Some("Ctrl+L".to_owned());
        lyrics.behavior.win_shortcut = Some("Alt+L".to_owned());
        lyrics.outline = ElementOutline {
            can_hide: true,
            level: 2,
        };
        let lower = definition("lyrics");

        let project = project(vec![lyrics, lower]);
        let summaries = element_definition_summaries(&project, ShortcutPlatform::MacOS);

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].key, "Lyrics");
        assert_eq!(summaries[1].key, "lyrics");
        assert_eq!(summaries[0].font.family, "Times New Roman");
        assert_eq!(summaries[0].font.size, 14);
        assert_eq!(
            summaries[0].font.active_flags,
            vec!["bold", "italic", "underline", "all caps", "hidden"]
        );
        assert_eq!(summaries[0].font.color.as_deref(), Some("#87878F8FE0E0"));
        assert_eq!(summaries[0].paragraph.alignment, "Justified");
        assert_eq!(summaries[0].paragraph.first_indent, "-2");
        assert_eq!(summaries[0].paragraph.left_indent, "1");
        assert_eq!(summaries[0].paragraph.right_indent, "3");
        assert_eq!(summaries[0].paragraph.leading, "Custom leading");
        assert_eq!(summaries[0].paragraph.spacing, 2);
        assert_eq!(summaries[0].paragraph.space_before, "4.5");
        assert!(summaries[0].paragraph.starts_new_page);
        assert_eq!(summaries[0].behavior.paginate_as.as_deref(), Some("Action"));
        assert_eq!(summaries[0].behavior.return_target, "later (unresolved)");
        assert_eq!(summaries[0].behavior.tab_target, "Lyrics");
        assert_eq!(summaries[0].behavior.shortcut.as_deref(), Some("Ctrl+L"));
        assert_eq!(
            summaries[0].outline.as_deref(),
            Some("level 2, hide supported")
        );
        assert_eq!(
            element_definition_summaries(&project, ShortcutPlatform::Windows)[0]
                .behavior
                .shortcut
                .as_deref(),
            Some("Alt+L")
        );
    }

    #[test]
    fn inch_formatting_is_trimmed_and_preserves_negative_fractional_values() {
        assert_eq!(
            inches_string(FixedPoint::parse_inches("-0.125").expect("inches parse")),
            "-0.125"
        );
        assert_eq!(
            inches_string(FixedPoint::parse_inches("0.333333").expect("inches parse")),
            "0.333333"
        );
        assert_eq!(
            inches_string(FixedPoint::parse_inches("0.123456").expect("inches parse")),
            "0.123472"
        );
    }

    #[test]
    fn absent_behavior_blank_platform_shortcuts_and_registry_refresh_are_exact() {
        let mut original = project(vec![definition("One"), definition("Two")]);
        let initial = element_definition_summaries(&original, ShortcutPlatform::Windows);
        assert_eq!(
            initial
                .iter()
                .map(|row| row.key.as_str())
                .collect::<Vec<_>>(),
            ["One", "Two"]
        );
        assert_eq!(initial[0].behavior.return_target, "Absent");
        assert_eq!(initial[0].behavior.tab_target, "Absent");
        assert_eq!(initial[0].behavior.shortcut, None);
        assert_eq!(initial[0].font.color, None);
        assert_eq!(initial[0].outline, None);

        let mut replacement = definition("After");
        replacement.behavior.shortcut = Some("MacOnly".to_owned());
        replacement.behavior.win_shortcut = Some("  ".to_owned());
        original
            .replace_element_definitions(vec![replacement])
            .expect("replacement registry is valid");
        let refreshed = element_definition_summaries(&original, ShortcutPlatform::MacOS);
        assert_eq!(refreshed.len(), 1);
        assert_eq!(refreshed[0].key, "After");
        assert_eq!(refreshed[0].behavior.shortcut.as_deref(), Some("MacOnly"));
        assert_eq!(
            element_definition_summaries(&original, ShortcutPlatform::Windows)[0]
                .behavior
                .shortcut,
            None
        );
    }
}
