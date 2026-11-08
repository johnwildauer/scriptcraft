//! Shared, typed screenplay element formatting defaults.
//!
//! The ordered defaults in this module are evidenced by the blank and populated
//! Final Draft references. They are used for native emission and only for the
//! runtime fields currently interpreted by ScriptCraft; retaining these values
//! does not imply that the renderer applies every formatting category.

use crate::{Role, StableId};
use std::collections::BTreeMap;

pub const ROLE_FORMATTING_COUNT: usize = 17;

/// Signed point geometry stored as hundredths of a point. Decimal FDX inch
/// values are converted once at the source boundary and remain exact in every
/// layout consumer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedPoint(i64);

impl FixedPoint {
    pub const SCALE: i64 = 100;

    pub const fn from_raw(raw_hundredths: i64) -> Self {
        Self(raw_hundredths)
    }

    pub const fn from_points(points: i64) -> Self {
        Self(points.saturating_mul(Self::SCALE))
    }

    pub const fn raw(self) -> i64 {
        self.0
    }

    pub fn to_points_f32(self) -> f32 {
        self.0 as f32 / Self::SCALE as f32
    }

    pub fn from_points_f64(value: f64) -> Option<Self> {
        let scaled = value * Self::SCALE as f64;
        if !scaled.is_finite() || scaled <= i64::MIN as f64 || scaled >= i64::MAX as f64 {
            return None;
        }
        Some(Self(scaled.round() as i64))
    }

    /// Exact decimal spelling for PDF coordinates and other textual numeric
    /// sinks; unlike the desktop conversion this does not pass through float.
    pub fn decimal_string(self) -> String {
        let negative = self.0 < 0;
        let magnitude = self.0.unsigned_abs();
        let whole = magnitude / Self::SCALE as u64;
        let fraction = magnitude % Self::SCALE as u64;
        if fraction == 0 {
            format!("{}{whole}", if negative { "-" } else { "" })
        } else if fraction.is_multiple_of(10) {
            format!(
                "{}{whole}.{}",
                if negative { "-" } else { "" },
                fraction / 10
            )
        } else {
            format!("{}{whole}.{fraction:02}", if negative { "-" } else { "" })
        }
    }

    /// One shared deterministic conversion for legacy integer geometry and
    /// integer-only consumers. Half values round away from zero.
    pub fn rounded_points(self) -> i64 {
        if self.0 >= 0 {
            self.0.saturating_add(Self::SCALE / 2) / Self::SCALE
        } else {
            self.0.saturating_sub(Self::SCALE / 2) / Self::SCALE
        }
    }

    pub fn parse_inches(value: &str) -> Option<Self> {
        parse_decimal_scaled(value, 7_200).map(Self)
    }

    pub fn parse_points(value: &str) -> Option<Self> {
        parse_decimal_scaled(value, Self::SCALE).map(Self)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ParagraphAlignment {
    #[default]
    Left,
    Right,
    Center,
    Justified,
}

/// Only supported imported fields are populated. `None` deliberately means
/// that the corresponding shared role default remains effective.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceParagraphSpec {
    pub alignment: Option<ParagraphAlignment>,
    pub first_indent: Option<FixedPoint>,
    pub left_indent: Option<FixedPoint>,
    pub right_indent: Option<FixedPoint>,
    pub leading_regular: Option<bool>,
    pub space_before: Option<FixedPoint>,
    pub spacing_single: Option<bool>,
}

/// Valid direct paragraph attributes associated with one imported managed body
/// element. `None` means that field continues through role/default resolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceParagraphOverrides {
    pub paragraph: SourceParagraphSpec,
    pub starts_new_page: Option<bool>,
}

/// Recognized role-level FontSpec values. Unknown FDX attributes and style
/// tokens stay in the preservation tree; `None` means use the shared default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceFontSpec {
    pub family: Option<String>,
    pub size: Option<u8>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub all_caps: Option<bool>,
    pub hidden_text: Option<bool>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFallback {
    CourierPrime,
}

/// Effective application presentation for one semantic role. The declared FDX
/// family and color are retained for diagnostics/source projection only; the
/// renderer deliberately uses the documented fixed-cell Courier Prime fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectiveTextStyle {
    pub declared_family: String,
    pub family: FontFallback,
    pub size: u8,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub all_caps: bool,
    pub hidden_text: bool,
    pub source_color: Option<String>,
}

fn parse_decimal_scaled(value: &str, factor: i64) -> Option<i64> {
    if value.is_empty() || value.len() > 32 {
        return None;
    }
    let (negative, digits) = match value.as_bytes()[0] {
        b'-' => (true, &value[1..]),
        b'+' => (false, &value[1..]),
        _ => (false, value),
    };
    if digits.is_empty() {
        return None;
    }
    let mut parts = digits.split('.');
    let whole = parts.next()?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 6
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let whole = whole.parse::<i64>().ok()?;
    let denominator = 10_i64.checked_pow(fraction.len() as u32)?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i64>().ok()?
    };
    let numerator = whole.checked_mul(denominator)?.checked_add(fraction)?;
    let scaled = numerator.checked_mul(factor)?;
    let rounded = scaled
        .checked_add(denominator / 2)?
        .checked_div(denominator)?;
    if negative {
        rounded.checked_neg()
    } else {
        Some(rounded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontSpec {
    pub family: &'static str,
    pub size: u8,
    pub style_tokens: &'static [&'static str],
    pub color: Option<&'static str>,
    pub adornment_style: u8,
    pub revision_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParagraphSpec {
    pub alignment: &'static str,
    pub first_indent: &'static str,
    pub leading: &'static str,
    pub left_indent: &'static str,
    pub right_indent: &'static str,
    pub space_before: u8,
    pub spacing: u8,
    pub starts_new_page: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Behavior {
    pub paginate_as: &'static str,
    pub return_key: &'static str,
    pub shortcut: &'static str,
    pub tab_key: &'static str,
    pub win_shortcut: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outline {
    pub can_hide: bool,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleFormatting {
    pub role: Role,
    /// Native Final Draft element label.
    pub fdx_label: &'static str,
    pub font: FontSpec,
    pub paragraph: ParagraphSpec,
    pub behavior: Behavior,
    pub outline: Outline,
}

const COURIER: &str = "Courier Final Draft";
const ARIAL: &str = "Arial";
const SOURCE_COLOR: &str = "#87878F8FE0E0";
const NO_STYLE: &[&str] = &[];
const ALL_CAPS: &[&str] = &["AllCaps"];
const BOLD_ALL_CAPS: &[&str] = &["Bold", "Underline", "AllCaps"];
const HIDDEN: &[&str] = &["HiddenText"];
const BOLD_HIDDEN: &[&str] = &["Bold", "HiddenText"];
const ITALIC_HIDDEN: &[&str] = &["Italic", "HiddenText"];

const fn font(
    family: &'static str,
    size: u8,
    style_tokens: &'static [&'static str],
    color: Option<&'static str>,
) -> FontSpec {
    FontSpec {
        family,
        size,
        style_tokens,
        color,
        adornment_style: 0,
        revision_id: 0,
    }
}

const fn paragraph(
    alignment: &'static str,
    first_indent: &'static str,
    left_indent: &'static str,
    right_indent: &'static str,
    space_before: u8,
    starts_new_page: bool,
) -> ParagraphSpec {
    ParagraphSpec {
        alignment,
        first_indent,
        leading: "Regular",
        left_indent,
        right_indent,
        space_before,
        spacing: 1,
        starts_new_page,
    }
}

const fn behavior(
    paginate_as: &'static str,
    return_key: &'static str,
    shortcut: &'static str,
    tab_key: &'static str,
    win_shortcut: &'static str,
) -> Behavior {
    Behavior {
        paginate_as,
        return_key,
        shortcut,
        tab_key,
        win_shortcut,
    }
}

const fn role_formatting(
    role: Role,
    fdx_label: &'static str,
    font: FontSpec,
    paragraph: ParagraphSpec,
    behavior: Behavior,
    can_hide: bool,
    level: u8,
) -> RoleFormatting {
    RoleFormatting {
        role,
        fdx_label,
        font,
        paragraph,
        behavior,
        outline: Outline { can_hide, level },
    }
}

/// The single native-order default table. Its order is intentionally different
/// from `Role::ALL`, which remains the semantic-model order.
pub const ROLE_FORMATTING_DEFAULTS: [RoleFormatting; ROLE_FORMATTING_COUNT] = [
    role_formatting(
        Role::General,
        "General",
        font(COURIER, 12, NO_STYLE, None),
        paragraph("Left", "0.00", "1.50", "7.50", 0, false),
        behavior("General", "General", "0", "General", "0"),
        false,
        1,
    ),
    role_formatting(
        Role::SceneHeading,
        "Scene Heading",
        font(COURIER, 12, ALL_CAPS, None),
        paragraph("Left", "0.00", "1.50", "7.50", 24, false),
        behavior("Scene Heading", "Action", "1", "Action", "1"),
        false,
        2,
    ),
    role_formatting(
        Role::Action,
        "Action",
        font(COURIER, 12, NO_STYLE, None),
        paragraph("Left", "0.00", "1.50", "7.50", 12, false),
        behavior("Action", "Action", "2", "Character", "2"),
        false,
        1,
    ),
    role_formatting(
        Role::Character,
        "Character",
        font(COURIER, 12, ALL_CAPS, None),
        paragraph("Left", "0.00", "3.50", "7.25", 12, false),
        behavior("Character", "Dialogue", "3", "Transition", "3"),
        false,
        1,
    ),
    role_formatting(
        Role::Parenthetical,
        "Parenthetical",
        font(COURIER, 12, NO_STYLE, None),
        paragraph("Left", "-0.10", "3.00", "5.50", 0, false),
        behavior("Parenthetical", "Dialogue", "4", "Dialogue", "4"),
        false,
        1,
    ),
    role_formatting(
        Role::Dialogue,
        "Dialogue",
        font(COURIER, 12, NO_STYLE, None),
        paragraph("Left", "0.00", "2.50", "6.00", 0, false),
        behavior("Dialogue", "Action", "5", "Parenthetical", "5"),
        false,
        1,
    ),
    role_formatting(
        Role::Transition,
        "Transition",
        font(COURIER, 12, ALL_CAPS, None),
        paragraph("Right", "0.00", "5.50", "7.10", 12, false),
        behavior("Transition", "Scene Heading", "6", "Scene Heading", "6"),
        false,
        1,
    ),
    role_formatting(
        Role::Shot,
        "Shot",
        font(COURIER, 12, ALL_CAPS, None),
        paragraph("Left", "0.00", "1.50", "7.50", 12, false),
        behavior("Scene Heading", "Action", "7", "Action", "7"),
        false,
        1,
    ),
    role_formatting(
        Role::CastList,
        "Cast List",
        font(COURIER, 12, ALL_CAPS, None),
        paragraph("Left", "0.00", "1.50", "7.50", 0, false),
        behavior("Action", "Action", "8", "Action", "8"),
        false,
        1,
    ),
    role_formatting(
        Role::NewAct,
        "New Act",
        font(COURIER, 12, BOLD_ALL_CAPS, None),
        paragraph("Center", "0.00", "1.00", "7.50", 12, true),
        behavior("Scene Heading", "Scene Heading", "9", "Scene Heading", "9"),
        false,
        6,
    ),
    role_formatting(
        Role::Sequence,
        "Sequence",
        font(COURIER, 12, BOLD_ALL_CAPS, None),
        paragraph("Left", "0.00", "1.25", "7.50", 24, false),
        behavior(
            "Scene Heading",
            "Scene Heading",
            "Ctrl+8",
            "Scene Heading",
            "Shift+8",
        ),
        false,
        4,
    ),
    role_formatting(
        Role::EndOfAct,
        "End of Act",
        font(COURIER, 12, BOLD_ALL_CAPS, None),
        paragraph("Center", "0.00", "1.00", "7.50", 24, false),
        behavior("Character", "New Act", "Ctrl+9", "New Act", "Shift+9"),
        false,
        6,
    ),
    role_formatting(
        Role::Summary,
        "Summary",
        font(ARIAL, 12, HIDDEN, Some(SOURCE_COLOR)),
        paragraph("Left", "0.00", "0.88", "7.50", 12, false),
        behavior("Outline Body", "Summary", "Ctrl+0", "Outline 1", "Shift+0"),
        true,
        0,
    ),
    role_formatting(
        Role::Outline1,
        "Outline 1",
        font(ARIAL, 16, BOLD_HIDDEN, Some(SOURCE_COLOR)),
        paragraph("Left", "0.00", "0.88", "7.50", 16, false),
        behavior("Outline", "Summary", "Ctrl+1", "Outline 2", "Shift+1"),
        true,
        7,
    ),
    role_formatting(
        Role::Outline2,
        "Outline 2",
        font(ARIAL, 14, BOLD_HIDDEN, Some(SOURCE_COLOR)),
        paragraph("Left", "0.00", "1.12", "7.50", 14, false),
        behavior("Outline", "Summary", "Ctrl+2", "Outline 3", "Shift+2"),
        true,
        5,
    ),
    role_formatting(
        Role::Outline3,
        "Outline 3",
        font(ARIAL, 12, BOLD_HIDDEN, Some(SOURCE_COLOR)),
        paragraph("Left", "0.00", "1.38", "7.50", 12, false),
        behavior("Outline", "Summary", "Ctrl+3", "Note", "Shift+3"),
        true,
        3,
    ),
    role_formatting(
        Role::Note,
        "Note",
        font(ARIAL, 12, ITALIC_HIDDEN, Some(SOURCE_COLOR)),
        paragraph("Left", "0.00", "1.62", "7.50", 12, false),
        behavior("Outline", "Summary", "Ctrl+4", "Summary", "Shift+4"),
        true,
        1,
    ),
];

pub fn formatting_for_role(role: Role) -> &'static RoleFormatting {
    ROLE_FORMATTING_DEFAULTS
        .iter()
        .find(|formatting| formatting.role == role)
        .unwrap_or(&ROLE_FORMATTING_DEFAULTS[0])
}

pub fn formatting_in_native_order() -> &'static [RoleFormatting; ROLE_FORMATTING_COUNT] {
    &ROLE_FORMATTING_DEFAULTS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintVisibility {
    Visible,
    Hidden,
}

/// Ephemeral recognized settings. `None` means that this field is absent in
/// source and therefore falls back to the shared per-role native default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceLayoutSettings {
    pub role_print_visibility: [Option<PrintVisibility>; ROLE_FORMATTING_COUNT],
    pub role_starts_new_page: [Option<bool>; ROLE_FORMATTING_COUNT],
    pub role_paragraph_specs: [SourceParagraphSpec; ROLE_FORMATTING_COUNT],
    pub role_font_specs: [SourceFontSpec; ROLE_FORMATTING_COUNT],
    /// Ephemeral direct body Paragraph projection keyed by its managed semantic
    /// identity, never by ordinal, foreign ID, or visible line position.
    pub direct_paragraph_overrides: BTreeMap<StableId, SourceParagraphOverrides>,
}

impl SourceLayoutSettings {
    pub fn print_visibility(&self, role: Role) -> PrintVisibility {
        let index = role_index(role);
        self.role_print_visibility[index].unwrap_or_else(|| {
            let hidden = self.role_font_specs[index].hidden_text.unwrap_or_else(|| {
                formatting_for_role(role)
                    .font
                    .style_tokens
                    .contains(&"HiddenText")
            });
            if hidden {
                PrintVisibility::Hidden
            } else {
                PrintVisibility::Visible
            }
        })
    }

    pub fn starts_new_page(&self, role: Role) -> bool {
        self.role_starts_new_page[role_index(role)]
            .unwrap_or(formatting_for_role(role).paragraph.starts_new_page)
    }

    pub fn starts_new_page_for_element(&self, role: Role, id: &StableId) -> bool {
        self.direct_paragraph_overrides
            .get(id)
            .and_then(|overrides| overrides.starts_new_page)
            .unwrap_or_else(|| self.starts_new_page(role))
    }

    pub fn direct_paragraph_spec(&self, id: &StableId) -> SourceParagraphOverrides {
        self.direct_paragraph_overrides
            .get(id)
            .copied()
            .unwrap_or_default()
    }

    pub fn paragraph_spec(&self, role: Role) -> SourceParagraphSpec {
        self.role_paragraph_specs[role_index(role)]
    }

    pub fn font_spec(&self, role: Role) -> &SourceFontSpec {
        &self.role_font_specs[role_index(role)]
    }

    pub fn text_style(&self, role: Role) -> EffectiveTextStyle {
        let default = formatting_for_role(role).font;
        let source = self.font_spec(role);
        let default_has = |token: &str| default.style_tokens.contains(&token);
        EffectiveTextStyle {
            declared_family: source
                .family
                .clone()
                .unwrap_or_else(|| default.family.to_owned()),
            family: FontFallback::CourierPrime,
            size: source.size.unwrap_or(default.size),
            bold: source.bold.unwrap_or_else(|| default_has("Bold")),
            italic: source.italic.unwrap_or_else(|| default_has("Italic")),
            underline: source.underline.unwrap_or_else(|| default_has("Underline")),
            all_caps: source.all_caps.unwrap_or_else(|| default_has("AllCaps")),
            hidden_text: self.print_visibility(role) == PrintVisibility::Hidden,
            source_color: source
                .color
                .clone()
                .or_else(|| default.color.map(str::to_owned)),
        }
    }
}

pub(crate) fn role_index(role: Role) -> usize {
    Role::ALL
        .iter()
        .position(|candidate| *candidate == role)
        .unwrap_or(0)
}
