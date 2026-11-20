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
/// that field is absent from a direct paragraph override.
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
/// element. `None` means that field continues through its document definition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceParagraphOverrides {
    pub paragraph: SourceParagraphSpec,
    pub starts_new_page: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFallback {
    CourierPrime,
}

/// Bounded legacy-layout style shape for builders without a prepared font
/// environment. Definition values are projected by the layout crate.
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
