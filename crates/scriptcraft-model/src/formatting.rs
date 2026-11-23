//! Shared direct-formatting and fixed-point values.

use crate::{identity::StableId, ModelError};
use std::collections::BTreeMap;

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

    /// One shared deterministic conversion for integer-point geometry and
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

/// Bounded direct paragraph leading choices. Definition-owned leading remains
/// a string and is interpreted only for these exact supported values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParagraphLeading {
    Regular,
    Tight,
    Loose,
}

impl ParagraphLeading {
    pub const fn fdx_value(self) -> &'static str {
        match self {
            Self::Regular => "Regular",
            Self::Tight => "Tight",
            Self::Loose => "Loose",
        }
    }

    pub fn from_fdx_value(value: &str) -> Option<Self> {
        match value {
            "Regular" => Some(Self::Regular),
            "Tight" => Some(Self::Tight),
            "Loose" => Some(Self::Loose),
            _ => None,
        }
    }

    /// Definition data also uses `Single` for the regular baseline.
    pub fn from_definition_value(value: &str) -> Option<Self> {
        match value {
            "Regular" | "Single" => Some(Self::Regular),
            "Tight" => Some(Self::Tight),
            "Loose" => Some(Self::Loose),
            _ => None,
        }
    }

    pub const fn adjustment_hundredths(self) -> i64 {
        match self {
            Self::Regular => 0,
            Self::Tight => -FixedPoint::SCALE,
            Self::Loose => FixedPoint::SCALE,
        }
    }
}

/// Bounded direct paragraph spacing choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParagraphSpacing {
    Single,
    OneAndHalf,
    Double,
}

impl ParagraphSpacing {
    pub const fn fdx_value(self) -> &'static str {
        match self {
            Self::Single => "1",
            Self::OneAndHalf => "1.5",
            Self::Double => "2",
        }
    }

    pub fn from_fdx_value(value: &str) -> Option<Self> {
        match value {
            "1" => Some(Self::Single),
            "1.5" => Some(Self::OneAndHalf),
            "2" => Some(Self::Double),
            _ => None,
        }
    }

    pub const fn from_definition_value(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Single),
            2 => Some(Self::Double),
            _ => None,
        }
    }

    pub const fn ratio(self) -> (i64, i64) {
        match self {
            Self::Single => (1, 1),
            Self::OneAndHalf => (3, 2),
            Self::Double => (2, 1),
        }
    }
}

/// Only supported imported fields are populated. `None` deliberately means
/// that field is absent from a direct paragraph override.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceParagraphSpec {
    pub alignment: Option<ParagraphAlignment>,
    pub first_indent: Option<FixedPoint>,
    pub left_indent: Option<FixedPoint>,
    pub right_indent: Option<FixedPoint>,
    pub leading: Option<ParagraphLeading>,
    pub space_before: Option<FixedPoint>,
    pub spacing: Option<ParagraphSpacing>,
}

/// Typed, sparse request for one paragraph-formatting operation. `None` means
/// unchanged; `space_before_lines` is converted using the current definition's
/// base font size by the shared model operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParagraphFormattingPatch {
    pub alignment: Option<ParagraphAlignment>,
    pub first_indent: Option<FixedPoint>,
    pub leading: Option<ParagraphLeading>,
    pub left_indent: Option<FixedPoint>,
    pub right_indent: Option<FixedPoint>,
    pub space_before_lines: Option<u8>,
    pub spacing: Option<ParagraphSpacing>,
    pub starts_new_page: Option<bool>,
}

/// Valid direct paragraph attributes associated with one imported managed body
/// element. `None` means that field continues through its document definition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceParagraphOverrides {
    pub paragraph: SourceParagraphSpec,
    pub starts_new_page: Option<bool>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceLayoutSettings {
    /// Direct paragraph projection keyed by managed semantic identity, never
    /// by ordinal, foreign ID, or visible line position.
    pub direct_paragraph_overrides: BTreeMap<StableId, SourceParagraphOverrides>,
}

impl SourceLayoutSettings {
    pub fn direct_paragraph_spec(&self, id: &StableId) -> SourceParagraphOverrides {
        self.direct_paragraph_overrides
            .get(id)
            .copied()
            .unwrap_or_default()
    }
}
