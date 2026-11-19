//! Backend-neutral, transient resolved typography inputs.
//!
//! This module deliberately owns no font discovery, byte loading, parsing, or
//! platform rendering. A desktop resolver supplies bounded face identities
//! and metrics; layout and output consumers can then make deterministic
//! geometry decisions without consulting a host API.

use crate::formatting::FixedPoint;
use crate::ModelError;
use std::collections::BTreeMap;

pub const MAX_RESOLVED_FONT_ID_BYTES: usize = 128;
pub const MAX_FONT_LABEL_BYTES: usize = 256;
pub const MAX_FONT_REQUESTS: usize = 64;
pub const MAX_RESOLVED_FACES: usize = 256;
pub const MAX_SCALAR_ADVANCES: usize = 4_096;
pub const MAX_FONT_BYTE_LENGTH: u64 = 64 * 1024 * 1024;
pub const MAX_UNITS_PER_EM: u32 = 65_536;
pub const MAX_METRIC_UNITS: i32 = 1_000_000;
pub const MAX_ADVANCE_UNITS: u32 = 1_000_000;
pub const MAX_SUPPORTED_FONT_SIZE: u16 = 1_000;
pub const MAX_COLOR_SPELLING_BYTES: usize = 64;
pub const NEUTRAL_COURIER_PRIME_FAMILY: &str = "Courier Prime";

/// The four independently resolved styles exposed by a requested family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaceStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl FaceStyle {
    pub const ALL: [Self; 4] = [Self::Regular, Self::Bold, Self::Italic, Self::BoldItalic];

    pub const fn from_flags(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (false, false) => Self::Regular,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (true, true) => Self::BoldItalic,
        }
    }
}

/// Opaque bounded identity for one exact resolved face payload.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolvedFontId(String);

impl ResolvedFontId {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_RESOLVED_FONT_ID_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(ModelError::InvalidResolvedFontId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How the resolved payload was supplied. This is descriptive metadata, not a
/// host lookup request and never contains a path or document text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolvedFontSourceKind {
    Bundled,
    Host,
    Synthetic,
}

/// One explicitly measured Unicode scalar. Missing glyphs still carry the
/// advance selected by the resolver so layout never has to guess at query time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScalarAdvance {
    pub advance_units: u32,
    pub missing_glyph: bool,
}

/// Vertical font-unit metrics and the bounded scalar repertoire claimed by a
/// resolved face. Metrics are integral font units, so scaling has one defined
/// rounding step and cannot produce NaN or negative geometry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedFaceMetrics {
    pub units_per_em: u32,
    pub ascender: i32,
    pub descender: i32,
    pub line_gap: i32,
    pub scalar_advances: BTreeMap<char, ScalarAdvance>,
}

impl ResolvedFaceMetrics {
    pub fn new(
        units_per_em: u32,
        ascender: i32,
        descender: i32,
        line_gap: i32,
        scalar_advances: BTreeMap<char, ScalarAdvance>,
    ) -> Result<Self, ModelError> {
        let metrics = Self {
            units_per_em,
            ascender,
            descender,
            line_gap,
            scalar_advances,
        };
        metrics.validate()?;
        Ok(metrics)
    }

    fn validate(&self) -> Result<(), ModelError> {
        if self.units_per_em == 0
            || self.units_per_em > MAX_UNITS_PER_EM
            || self.ascender.unsigned_abs() > MAX_METRIC_UNITS as u32
            || self.descender.unsigned_abs() > MAX_METRIC_UNITS as u32
            || self.line_gap.unsigned_abs() > MAX_METRIC_UNITS as u32
            || self.ascender <= self.descender
            || self.line_gap < 0
            || self.scalar_advances.len() > MAX_SCALAR_ADVANCES
            || self
                .scalar_advances
                .values()
                .any(|metric| metric.advance_units > MAX_ADVANCE_UNITS)
        {
            return Err(ModelError::InvalidFontMetrics);
        }
        Ok(())
    }

    pub fn advance_for(&self, scalar: char) -> Option<ScalarAdvance> {
        self.scalar_advances.get(&scalar).copied()
    }

    pub fn scale_advance(&self, scalar: char, size_points: u16) -> Result<FixedPoint, ModelError> {
        self.validate()?;
        validate_font_size(size_points)?;
        let advance = self
            .advance_for(scalar)
            .ok_or(ModelError::MissingScalarMetrics)?;
        scale_font_units(advance.advance_units, self.units_per_em, size_points)
    }

    /// Scale ascent, descent, and line gap together into hundredths of a point.
    pub fn scale_regular_line_advance(&self, size_points: u16) -> Result<FixedPoint, ModelError> {
        self.validate()?;
        validate_font_size(size_points)?;
        let units = self
            .ascender
            .checked_sub(self.descender)
            .and_then(|height| height.checked_add(self.line_gap))
            .filter(|height| *height > 0)
            .ok_or(ModelError::InvalidFontMetrics)?;
        scale_font_units(units as u32, self.units_per_em, size_points)
    }
}

fn validate_font_size(size_points: u16) -> Result<(), ModelError> {
    if size_points == 0 || size_points > MAX_SUPPORTED_FONT_SIZE {
        Err(ModelError::InvalidFontSize)
    } else {
        Ok(())
    }
}

fn scale_font_units(
    units: u32,
    units_per_em: u32,
    size_points: u16,
) -> Result<FixedPoint, ModelError> {
    let numerator = u128::from(units)
        .checked_mul(u128::from(size_points))
        .and_then(|value| value.checked_mul(FixedPoint::SCALE as u128))
        .ok_or(ModelError::InvalidFontMetrics)?;
    let denominator = u128::from(units_per_em);
    let rounded = numerator
        .checked_add(denominator / 2)
        .ok_or(ModelError::InvalidFontMetrics)?
        / denominator;
    let raw = i64::try_from(rounded).map_err(|_| ModelError::InvalidFontMetrics)?;
    Ok(FixedPoint::from_raw(raw))
}

/// Metadata and metrics for the exact face selected by the transient resolver.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedFontFace {
    pub id: ResolvedFontId,
    pub family_label: String,
    pub face_label: String,
    pub style: FaceStyle,
    pub source_kind: ResolvedFontSourceKind,
    pub byte_length: u64,
    pub content_fingerprint: u64,
    pub metrics: ResolvedFaceMetrics,
}

impl ResolvedFontFace {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ResolvedFontId,
        family_label: impl Into<String>,
        face_label: impl Into<String>,
        style: FaceStyle,
        source_kind: ResolvedFontSourceKind,
        byte_length: u64,
        content_fingerprint: u64,
        metrics: ResolvedFaceMetrics,
    ) -> Result<Self, ModelError> {
        let family_label = family_label.into();
        let face_label = face_label.into();
        if !valid_label(&family_label)
            || !valid_label(&face_label)
            || byte_length == 0
            || byte_length > MAX_FONT_BYTE_LENGTH
        {
            return Err(ModelError::TypographyValueTooLong);
        }
        metrics.validate()?;
        Ok(Self {
            id,
            family_label,
            face_label,
            style,
            source_kind,
            byte_length,
            content_fingerprint,
            metrics,
        })
    }

    /// Build identity metadata from the exact selected bytes. The byte payload
    /// is not retained by the model; the desktop resolver owns registration.
    #[allow(clippy::too_many_arguments)]
    pub fn from_bytes(
        id: ResolvedFontId,
        family_label: impl Into<String>,
        face_label: impl Into<String>,
        style: FaceStyle,
        source_kind: ResolvedFontSourceKind,
        bytes: &[u8],
        metrics: ResolvedFaceMetrics,
    ) -> Result<Self, ModelError> {
        let byte_length =
            u64::try_from(bytes.len()).map_err(|_| ModelError::TypographyValueTooLong)?;
        Self::new(
            id,
            family_label,
            face_label,
            style,
            source_kind,
            byte_length,
            content_fingerprint(bytes),
            metrics,
        )
    }
}

fn valid_label(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_FONT_LABEL_BYTES && !value.chars().any(char::is_control)
}

/// Stable FNV-1a identity for exact font bytes, independent of host paths and
/// standard-library hash randomization.
pub fn content_fingerprint(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Requested-family classification produced by a resolver. It contains no host
/// API behavior; every selected style is represented by an explicit face ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontResolutionKind {
    CourierAlias,
    /// A non-generic declared family resolved to its exact host family.
    ExactHost,
    NearestSans,
    NearestSerif,
    UnknownFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestedFontResolution {
    pub kind: FontResolutionKind,
    pub regular: ResolvedFontId,
    pub bold: ResolvedFontId,
    pub italic: ResolvedFontId,
    pub bold_italic: ResolvedFontId,
}

impl RequestedFontResolution {
    pub fn face_for_style(&self, style: FaceStyle) -> &ResolvedFontId {
        match style {
            FaceStyle::Regular => &self.regular,
            FaceStyle::Bold => &self.bold,
            FaceStyle::Italic => &self.italic,
            FaceStyle::BoldItalic => &self.bold_italic,
        }
    }

    pub fn face_for_flags(&self, bold: bool, italic: bool) -> &ResolvedFontId {
        self.face_for_style(FaceStyle::from_flags(bold, italic))
    }

    fn face_ids(&self) -> [&ResolvedFontId; 4] {
        [&self.regular, &self.bold, &self.italic, &self.bold_italic]
    }
}

/// Complete transient font environment for one document/layout request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentFontEnvironment {
    requests: BTreeMap<String, RequestedFontResolution>,
    faces: BTreeMap<ResolvedFontId, ResolvedFontFace>,
    neutral_request: RequestedFontResolution,
    content_fingerprint: u64,
}

impl DocumentFontEnvironment {
    pub fn new(
        requests: BTreeMap<String, RequestedFontResolution>,
        faces: BTreeMap<ResolvedFontId, ResolvedFontFace>,
    ) -> Result<Self, ModelError> {
        if requests.is_empty()
            || requests.len() > MAX_FONT_REQUESTS
            || faces.is_empty()
            || faces.len() > MAX_RESOLVED_FACES
        {
            return Err(ModelError::TooManyResolvedFonts);
        }
        let neutral = requests
            .get(NEUTRAL_COURIER_PRIME_FAMILY)
            .ok_or(ModelError::MissingResolvedFontFace)?;
        if neutral.kind != FontResolutionKind::CourierAlias {
            return Err(ModelError::MissingResolvedFontFace);
        }
        for (family, request) in &requests {
            if !valid_label(family) {
                return Err(ModelError::TypographyValueTooLong);
            }
            for id in request.face_ids() {
                let face = faces.get(id).ok_or(ModelError::MissingResolvedFontFace)?;
                // A requested style may deliberately point at another actual
                // face when the family lacks that style. The ID must still
                // resolve to the exact face record; style fallback is explicit
                // in the resolved face metadata rather than synthesized.
                if face.id != *id {
                    return Err(ModelError::MissingResolvedFontFace);
                }
            }
        }
        for (id, face) in &faces {
            if id != &face.id
                || !valid_label(&face.family_label)
                || !valid_label(&face.face_label)
                || face.byte_length == 0
                || face.byte_length > MAX_FONT_BYTE_LENGTH
            {
                return Err(ModelError::MissingResolvedFontFace);
            }
            face.metrics.validate()?;
        }
        let content_fingerprint = environment_fingerprint(&requests, &faces);
        let neutral_request = requests
            .get(NEUTRAL_COURIER_PRIME_FAMILY)
            .cloned()
            .ok_or(ModelError::MissingResolvedFontFace)?;
        Ok(Self {
            requests,
            faces,
            neutral_request,
            content_fingerprint,
        })
    }

    pub fn resolution_for(&self, declared_family: &str) -> Option<&RequestedFontResolution> {
        self.requests.get(declared_family)
    }

    pub fn require_resolution_for(
        &self,
        declared_family: &str,
    ) -> Result<&RequestedFontResolution, ModelError> {
        self.resolution_for(declared_family)
            .ok_or(ModelError::MissingResolvedFontRequest)
    }

    pub fn face(&self, id: &ResolvedFontId) -> Option<&ResolvedFontFace> {
        self.faces.get(id)
    }

    /// Resolve a requested style, preferring the element's unoverridden style
    /// when the requested style has no exact face. The request's style slot is
    /// already mapped to the family-wide fallback priority by the desktop
    /// resolver, so it is the final fallback if `preferred_style` is missing.
    pub fn resolve_face_id(
        &self,
        declared_family: &str,
        requested_style: FaceStyle,
        preferred_style: Option<FaceStyle>,
    ) -> Result<&ResolvedFontId, ModelError> {
        let request = self
            .resolution_for(declared_family)
            .ok_or(ModelError::MissingResolvedFontRequest)?;
        let requested_id = request.face_for_style(requested_style);
        let requested_face = self
            .face(requested_id)
            .ok_or(ModelError::MissingResolvedFontFace)?;
        if requested_face.style == requested_style {
            return Ok(requested_id);
        }
        if let Some(preferred_style) = preferred_style.filter(|style| *style != requested_style) {
            let preferred_id = request.face_for_style(preferred_style);
            let preferred_face = self
                .face(preferred_id)
                .ok_or(ModelError::MissingResolvedFontFace)?;
            if preferred_face.style == preferred_style {
                return Ok(preferred_id);
            }
        }
        Ok(requested_id)
    }

    pub fn require_face(&self, id: &ResolvedFontId) -> Result<&ResolvedFontFace, ModelError> {
        self.face(id).ok_or(ModelError::MissingResolvedFontFace)
    }

    pub fn requests(&self) -> &BTreeMap<String, RequestedFontResolution> {
        &self.requests
    }

    pub fn faces(&self) -> &BTreeMap<ResolvedFontId, ResolvedFontFace> {
        &self.faces
    }

    pub const fn content_fingerprint(&self) -> u64 {
        self.content_fingerprint
    }

    pub fn neutral_request(&self) -> &RequestedFontResolution {
        &self.neutral_request
    }
}

fn environment_fingerprint(
    requests: &BTreeMap<String, RequestedFontResolution>,
    faces: &BTreeMap<ResolvedFontId, ResolvedFontFace>,
) -> u64 {
    let mut hash = StableFingerprint::new();
    hash.usize(requests.len());
    for (family, request) in requests {
        hash.string(family);
        hash.byte(request.kind as u8);
        for id in request.face_ids() {
            hash.string(id.as_str());
        }
    }
    hash.usize(faces.len());
    for (id, face) in faces {
        hash.string(id.as_str());
        hash.string(&face.family_label);
        hash.string(&face.face_label);
        hash.byte(face.style as u8);
        hash.byte(face.source_kind as u8);
        hash.u64(face.byte_length);
        hash.u64(face.content_fingerprint);
        hash.u32(face.metrics.units_per_em);
        hash.i32(face.metrics.ascender);
        hash.i32(face.metrics.descender);
        hash.i32(face.metrics.line_gap);
        hash.usize(face.metrics.scalar_advances.len());
        for (scalar, advance) in &face.metrics.scalar_advances {
            hash.u32(*scalar as u32);
            hash.u32(advance.advance_units);
            hash.byte(u8::from(advance.missing_glyph));
        }
    }
    hash.finish()
}

struct StableFingerprint(u64);

impl StableFingerprint {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn byte(&mut self, byte: u8) {
        self.0 ^= u64::from(byte);
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.byte(*byte);
        }
    }

    fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.usize(value.len());
        self.bytes(value.as_bytes());
    }

    fn finish(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectiveTextColor {
    ThemeDefault,
    ExplicitRgb8 { red: u8, green: u8, blue: u8 },
    UnsupportedExplicit,
}

/// Parse only the normalized Final Draft `#RRRRGGGGBBBB` spelling. Each
/// four-digit channel is reduced to its most-significant byte as specified by
/// the typography contract.
pub fn parse_effective_text_color(source: Option<&str>) -> EffectiveTextColor {
    let Some(source) = source else {
        return EffectiveTextColor::ThemeDefault;
    };
    if source.len() != 13
        || !source.starts_with('#')
        || !source.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return EffectiveTextColor::UnsupportedExplicit;
    }
    let bytes = source.as_bytes();
    let parse_channel = |start: usize| -> Option<u8> {
        let pair = std::str::from_utf8(bytes.get(start..start + 2)?).ok()?;
        u8::from_str_radix(pair, 16).ok()
    };
    match (parse_channel(1), parse_channel(5), parse_channel(9)) {
        (Some(red), Some(green), Some(blue)) => {
            EffectiveTextColor::ExplicitRgb8 { red, green, blue }
        }
        _ => EffectiveTextColor::UnsupportedExplicit,
    }
}

/// Transient effective style supplied to layout/paint/output. Declared source
/// spellings remain alongside, rather than being rewritten to resolved values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectiveTextStyle {
    pub declared_family: String,
    pub source_color: Option<String>,
    pub resolved_font_id: ResolvedFontId,
    pub size: u16,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub all_caps: bool,
    pub hidden_text: bool,
    pub color: EffectiveTextColor,
}

impl EffectiveTextStyle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        declared_family: impl Into<String>,
        source_color: Option<String>,
        resolved_font_id: ResolvedFontId,
        size: u16,
        bold: bool,
        italic: bool,
        underline: bool,
        all_caps: bool,
        hidden_text: bool,
    ) -> Result<Self, ModelError> {
        let declared_family = declared_family.into();
        if !valid_label(&declared_family)
            || source_color
                .as_ref()
                .is_some_and(|color| color.len() > MAX_COLOR_SPELLING_BYTES)
        {
            return Err(ModelError::TypographyValueTooLong);
        }
        validate_font_size(size)?;
        let color = parse_effective_text_color(source_color.as_deref());
        Ok(Self {
            declared_family,
            source_color,
            resolved_font_id,
            size,
            bold,
            italic,
            underline,
            all_caps,
            hidden_text,
            color,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn metrics(a_advance: u32) -> Result<ResolvedFaceMetrics, ModelError> {
        ResolvedFaceMetrics::new(
            1_000,
            800,
            -200,
            0,
            BTreeMap::from([
                (
                    'i',
                    ScalarAdvance {
                        advance_units: a_advance,
                        missing_glyph: false,
                    },
                ),
                (
                    'W',
                    ScalarAdvance {
                        advance_units: 900,
                        missing_glyph: false,
                    },
                ),
                (
                    '�',
                    ScalarAdvance {
                        advance_units: 700,
                        missing_glyph: true,
                    },
                ),
            ]),
        )
    }

    fn environment(advance: u32) -> Result<DocumentFontEnvironment, ModelError> {
        let mut faces = BTreeMap::new();
        for (index, style) in FaceStyle::ALL.into_iter().enumerate() {
            let id = ResolvedFontId::new(match style {
                FaceStyle::Regular => "synthetic-Regular",
                FaceStyle::Bold => "synthetic-Bold",
                FaceStyle::Italic => "synthetic-Italic",
                FaceStyle::BoldItalic => "synthetic-BoldItalic",
            })?;
            let face = ResolvedFontFace::new(
                id.clone(),
                "Synthetic",
                format!("Synthetic {style:?}"),
                style,
                ResolvedFontSourceKind::Synthetic,
                128 + index as u64,
                10 + index as u64,
                metrics(advance)?,
            )?;
            faces.insert(id.clone(), face);
        }
        let ids = [
            ResolvedFontId::new("synthetic-Regular")?,
            ResolvedFontId::new("synthetic-Bold")?,
            ResolvedFontId::new("synthetic-Italic")?,
            ResolvedFontId::new("synthetic-BoldItalic")?,
        ];
        let resolution = RequestedFontResolution {
            kind: FontResolutionKind::CourierAlias,
            regular: ids[0].clone(),
            bold: ids[1].clone(),
            italic: ids[2].clone(),
            bold_italic: ids[3].clone(),
        };
        DocumentFontEnvironment::new(
            BTreeMap::from([
                ("Courier Prime".to_owned(), resolution.clone()),
                ("Courier".to_owned(), resolution),
            ]),
            faces,
        )
    }

    #[test]
    fn selects_each_face_style_independently() -> Result<(), ModelError> {
        let env = environment(500)?;
        let resolution = env
            .resolution_for("Courier")
            .ok_or(ModelError::MissingResolvedFontFace)?;
        for (style, bold, italic) in [
            (FaceStyle::Regular, false, false),
            (FaceStyle::Bold, true, false),
            (FaceStyle::Italic, false, true),
            (FaceStyle::BoldItalic, true, true),
        ] {
            assert_eq!(FaceStyle::from_flags(bold, italic), style);
            assert_eq!(
                resolution.face_for_flags(bold, italic),
                resolution.face_for_style(style)
            );
        }
        assert_ne!(resolution.regular, resolution.bold);
        assert_ne!(resolution.italic, resolution.bold_italic);
        Ok(())
    }

    #[test]
    fn scales_proportional_metrics_once_to_hundredths_of_a_point() -> Result<(), ModelError> {
        let metrics = metrics(333)?;
        assert_eq!(metrics.scale_advance('i', 12).map(FixedPoint::raw), Ok(400));
        assert_eq!(
            metrics.scale_advance('W', 12).map(FixedPoint::raw),
            Ok(1_080)
        );
        assert_eq!(
            metrics.scale_regular_line_advance(12).map(FixedPoint::raw),
            Ok(1_200)
        );
        assert_eq!(metrics.scale_advance('i', 1).map(FixedPoint::raw), Ok(33));
        let halfway = ResolvedFaceMetrics::new(
            8,
            7,
            -1,
            0,
            BTreeMap::from([(
                'x',
                ScalarAdvance {
                    advance_units: 1,
                    missing_glyph: false,
                },
            )]),
        )?;
        assert_eq!(halfway.scale_advance('x', 1).map(FixedPoint::raw), Ok(13));
        Ok(())
    }

    #[test]
    fn explicit_missing_glyph_advance_and_missing_repertoire_are_distinct() -> Result<(), ModelError>
    {
        let metrics = metrics(400)?;
        assert_eq!(
            metrics.advance_for('�').map(|metric| metric.missing_glyph),
            Some(true)
        );
        assert_eq!(metrics.scale_advance('�', 12).map(FixedPoint::raw), Ok(840));
        assert_eq!(
            metrics.scale_advance('x', 12),
            Err(ModelError::MissingScalarMetrics)
        );
        Ok(())
    }

    #[test]
    fn color_distinguishes_absent_explicit_and_unsupported_spelling() {
        assert_eq!(
            parse_effective_text_color(None),
            EffectiveTextColor::ThemeDefault
        );
        assert_eq!(
            parse_effective_text_color(Some("#87878F8FE0E0")),
            EffectiveTextColor::ExplicitRgb8 {
                red: 0x87,
                green: 0x8f,
                blue: 0xe0,
            }
        );
        assert_eq!(
            parse_effective_text_color(Some("black")),
            EffectiveTextColor::UnsupportedExplicit
        );
        assert_eq!(
            parse_effective_text_color(Some("#112233")),
            EffectiveTextColor::UnsupportedExplicit
        );
        assert_eq!(
            parse_effective_text_color(Some("#87ZZ8F8FE0E0")),
            EffectiveTextColor::UnsupportedExplicit
        );
    }

    #[test]
    fn environment_identity_changes_with_face_metrics_and_byte_identity() -> Result<(), ModelError>
    {
        let original = environment(333)?;
        let changed_metrics = environment(334)?;
        assert_ne!(original, changed_metrics);
        assert_ne!(
            original.content_fingerprint(),
            changed_metrics.content_fingerprint()
        );
        assert_eq!(original, environment(333)?);

        let mut changed_mapping = original.requests().clone();
        if let Some(request) = changed_mapping.get_mut("Courier") {
            request.kind = FontResolutionKind::UnknownFallback;
        }
        let mapping_variant =
            DocumentFontEnvironment::new(changed_mapping, original.faces().clone())?;
        assert_ne!(original, mapping_variant);
        assert_ne!(
            original.content_fingerprint(),
            mapping_variant.content_fingerprint()
        );

        let mut changed_face_identity = original.faces().clone();
        for face in changed_face_identity.values_mut() {
            face.content_fingerprint = face.content_fingerprint.wrapping_add(1);
        }
        let byte_variant =
            DocumentFontEnvironment::new(original.requests().clone(), changed_face_identity)?;
        assert_ne!(original, byte_variant);
        assert_ne!(
            original.content_fingerprint(),
            byte_variant.content_fingerprint()
        );
        assert!(original.resolution_for("Not Registered").is_none());
        assert_eq!(
            original.require_resolution_for("Not Registered"),
            Err(ModelError::MissingResolvedFontRequest)
        );

        assert_eq!(content_fingerprint(b"face"), content_fingerprint(b"face"));
        assert_ne!(content_fingerprint(b"face"), content_fingerprint(b"other"));
        use std::hash::{Hash, Hasher};
        let mut first_hash = std::collections::hash_map::DefaultHasher::new();
        let mut second_hash = std::collections::hash_map::DefaultHasher::new();
        original.hash(&mut first_hash);
        environment(333)?.hash(&mut second_hash);
        assert_eq!(first_hash.finish(), second_hash.finish());
        Ok(())
    }

    #[test]
    fn rejects_unbounded_or_unusable_metrics_and_font_sizes() -> Result<(), ModelError> {
        assert_eq!(
            ResolvedFaceMetrics::new(0, 800, -200, 0, BTreeMap::new()),
            Err(ModelError::InvalidFontMetrics)
        );
        let metrics = metrics(400)?;
        assert_eq!(
            metrics.scale_advance('i', 0),
            Err(ModelError::InvalidFontSize)
        );
        assert_eq!(
            metrics.scale_advance('i', MAX_SUPPORTED_FONT_SIZE + 1),
            Err(ModelError::InvalidFontSize)
        );
        assert_eq!(
            ResolvedFontId::new("x".repeat(MAX_RESOLVED_FONT_ID_BYTES + 1)),
            Err(ModelError::InvalidResolvedFontId)
        );
        let too_many_scalars = (0..=MAX_SCALAR_ADVANCES)
            .filter_map(|value| char::from_u32(value as u32))
            .map(|scalar| {
                (
                    scalar,
                    ScalarAdvance {
                        advance_units: 1,
                        missing_glyph: false,
                    },
                )
            })
            .collect();
        assert_eq!(
            ResolvedFaceMetrics::new(1_000, 800, -200, 0, too_many_scalars),
            Err(ModelError::InvalidFontMetrics)
        );
        Ok(())
    }

    #[test]
    fn exact_byte_construction_records_a_bounded_content_identity() -> Result<(), ModelError> {
        let face = ResolvedFontFace::from_bytes(
            ResolvedFontId::new("synthetic-bytes")?,
            "Synthetic",
            "Synthetic Regular",
            FaceStyle::Regular,
            ResolvedFontSourceKind::Synthetic,
            b"exact-font-bytes",
            metrics(400)?,
        )?;
        assert_eq!(face.byte_length, 16);
        assert_eq!(
            face.content_fingerprint,
            content_fingerprint(b"exact-font-bytes")
        );
        Ok(())
    }

    #[test]
    fn effective_style_keeps_source_color_and_resolved_color_separate() -> Result<(), ModelError> {
        let id = ResolvedFontId::new("face-1")?;
        let style = EffectiveTextStyle::new(
            "Courier",
            Some("#123456789ABC".to_owned()),
            id,
            14,
            true,
            false,
            true,
            false,
            false,
        )?;
        assert_eq!(style.source_color.as_deref(), Some("#123456789ABC"));
        assert_eq!(
            style.color,
            EffectiveTextColor::ExplicitRgb8 {
                red: 0x12,
                green: 0x56,
                blue: 0x9a,
            }
        );
        Ok(())
    }
}
