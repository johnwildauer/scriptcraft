//! Backend-neutral, transient resolved typography inputs. Line-box bottoms are
//! canonical baselines; font-table metrics are retained as natural extents.
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
/// resolved face. Metrics are integral font units and scale with one defined
/// integer rounding step; descent remains signed.
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

    /// Scale the natural font-table ascent into signed hundredths of a point.
    pub fn scale_ascent(&self, size_points: u16) -> Result<FixedPoint, ModelError> {
        self.validate()?;
        validate_font_size(size_points)?;
        scale_signed_font_units(self.ascender, self.units_per_em, size_points)
    }

    /// Scale the signed natural font-table descent into hundredths of a point.
    pub fn scale_descent(&self, size_points: u16) -> Result<FixedPoint, ModelError> {
        self.validate()?;
        validate_font_size(size_points)?;
        scale_signed_font_units(self.descender, self.units_per_em, size_points)
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

fn scale_signed_font_units(
    units: i32,
    units_per_em: u32,
    size_points: u16,
) -> Result<FixedPoint, ModelError> {
    let numerator = i128::from(units)
        .checked_mul(i128::from(size_points))
        .and_then(|value| value.checked_mul(i128::from(FixedPoint::SCALE)))
        .ok_or(ModelError::InvalidFontMetrics)?;
    let denominator = i128::from(units_per_em);
    let magnitude = numerator
        .checked_abs()
        .and_then(|value| value.checked_add(denominator / 2))
        .ok_or(ModelError::InvalidFontMetrics)?
        / denominator;
    let signed = if numerator < 0 {
        magnitude.checked_neg().ok_or(ModelError::InvalidFontMetrics)?
    } else {
        magnitude
    };
    let raw = i64::try_from(signed).map_err(|_| ModelError::InvalidFontMetrics)?;
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
    pub leading_resolver: LeadingResolverKind,
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
        leading_resolver: LeadingResolverKind,
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
            leading_resolver,
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
        leading_resolver: LeadingResolverKind,
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
            leading_resolver,
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

/// Explicit per-face baseline-leading category. Request/fallback metadata does
/// not select the layout resolver; this category on the selected face does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeadingResolverKind {
    Courier,
    Proportional,
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
        hash.byte(face.leading_resolver as u8);
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

