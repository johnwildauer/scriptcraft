//! Export-local host/synthetic font embedding eligibility and original-byte
//! budgets. This module never constructs PDF font objects or changes layout.

use crate::{ExportError, PdfResolvedFaceBytes};
use scriptcraft_model::typography::{ResolvedFaceMetrics, ResolvedFontSourceKind};
use std::collections::BTreeSet;
use ttf_parser::{Face, GlyphId, Permissions, Tag};

pub(crate) const MAX_PDF_EMBEDDED_FACE_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_PDF_EMBEDDED_BYTES: usize = 24 * 1024 * 1024;

/// Fixed, privacy-safe reasons that an otherwise valid host/synthetic face
/// remains on the bounded Base-14 path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfFontEmbeddingFallbackReason {
    RestrictedLicense,
    PermissionUnavailable,
    OutlineEmbeddingRestricted,
    UnsupportedContainer,
    UnsupportedOutline,
    UnsupportedVariableFont,
    UnsupportedSource,
    InvalidFont,
    FaceByteLimit,
    AggregateByteBudget,
}

pub(crate) enum HostFaceDecision<'a> {
    Embed {
        face: Face<'a>,
        postscript_name: String,
    },
    Fallback(PdfFontEmbeddingFallbackReason),
}

/// Counts original, complete font programs, reserving packaged faces before
/// any host/synthetic candidate can consume aggregate capacity.
#[derive(Debug, Default)]
pub(crate) struct PdfFontByteBudget {
    reserved_bytes: usize,
}

impl PdfFontByteBudget {
    pub(crate) fn reserve_packaged(&mut self, byte_count: usize) -> Result<(), ExportError> {
        if byte_count > MAX_PDF_EMBEDDED_FACE_BYTES {
            return Err(ExportError::RequiredPackagedFontEmbedding);
        }
        let next = self
            .reserved_bytes
            .checked_add(byte_count)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        if next > MAX_PDF_EMBEDDED_BYTES {
            return Err(ExportError::RequiredPackagedFontEmbedding);
        }
        self.reserved_bytes = next;
        Ok(())
    }

    pub(crate) fn reserve_host(
        &mut self,
        byte_count: usize,
    ) -> Result<Option<PdfFontEmbeddingFallbackReason>, ExportError> {
        if byte_count > MAX_PDF_EMBEDDED_FACE_BYTES {
            return Ok(Some(PdfFontEmbeddingFallbackReason::FaceByteLimit));
        }
        let next = self
            .reserved_bytes
            .checked_add(byte_count)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        if next > MAX_PDF_EMBEDDED_BYTES {
            return Ok(Some(
                PdfFontEmbeddingFallbackReason::AggregateByteBudget,
            ));
        }
        self.reserved_bytes = next;
        Ok(None)
    }
}

/// Checks the exact host/synthetic program without changing its bytes. Parser
/// metric disagreement is always an invalid payload, never a fallback reason.
pub(crate) fn assess_host_face<'a>(
    entry: &'a PdfResolvedFaceBytes,
    visible_scalars: &BTreeSet<char>,
) -> Result<HostFaceDecision<'a>, ExportError> {
    let face = match Face::parse(&entry.bytes, entry.face_index) {
        Ok(face) => face,
        Err(_) => {
            let reason = if entry.face_index != 0 || entry.bytes.starts_with(b"ttcf") {
                PdfFontEmbeddingFallbackReason::UnsupportedContainer
            } else if entry.bytes.get(..4) == Some(b"OTTO") {
                PdfFontEmbeddingFallbackReason::UnsupportedOutline
            } else if entry.bytes.get(..4) == Some(b"wOFF")
                || entry.bytes.get(..4) == Some(b"wOF2")
            {
                PdfFontEmbeddingFallbackReason::UnsupportedContainer
            } else {
                PdfFontEmbeddingFallbackReason::InvalidFont
            };
            return Ok(HostFaceDecision::Fallback(reason));
        }
    };

    validate_vertical_metrics(&face, &entry.face.metrics)?;

    if !matches!(
        entry.face.source_kind,
        ResolvedFontSourceKind::Host | ResolvedFontSourceKind::Synthetic
    ) {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::UnsupportedSource,
        ));
    }

    let signature = entry.bytes.get(..4);
    if entry.face_index != 0 || entry.bytes.starts_with(b"ttcf") {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::UnsupportedContainer,
        ));
    }
    if signature == Some(b"OTTO") {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::UnsupportedOutline,
        ));
    }
    if signature != Some(&[0, 1, 0, 0]) {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::UnsupportedContainer,
        ));
    }
    if face.is_variable() {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::UnsupportedVariableFont,
        ));
    }

    let tables = face.tables();
    if tables.cmap.is_none() || tables.hmtx.is_none() {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::InvalidFont,
        ));
    }
    validate_scalar_metrics(&face, &entry.face.metrics, visible_scalars)?;

    let has_glyf = tables.glyf.is_some();
    let has_loca = face.table_data(Tag::from_bytes(b"loca")).is_some();
    let has_cff = face.table_data(Tag::from_bytes(b"CFF ")).is_some()
        || face.table_data(Tag::from_bytes(b"CFF2")).is_some();
    if !has_glyf || !has_loca || has_cff {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::UnsupportedOutline,
        ));
    }
    if tables.post.is_none()
        || tables.os2.is_none()
        || face.table_data(Tag::from_bytes(b"name")).is_none()
    {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::InvalidFont,
        ));
    }

    let permissions = match face.permissions() {
        Some(permissions) => permissions,
        None => {
            return Ok(HostFaceDecision::Fallback(
                PdfFontEmbeddingFallbackReason::PermissionUnavailable,
            ));
        }
    };
    if permissions == Permissions::Restricted {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::RestrictedLicense,
        ));
    }
    if !face.is_outline_embedding_allowed() {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::OutlineEmbeddingRestricted,
        ));
    }

    let names = face
        .names()
        .into_iter()
        .filter(|name| name.name_id == 6)
        .filter_map(|name| name.to_string())
        .collect::<BTreeSet<_>>();
    if names.len() != 1 {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::InvalidFont,
        ));
    }
    let postscript_name = names
        .into_iter()
        .next()
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    if !valid_postscript_name(&postscript_name) {
        return Ok(HostFaceDecision::Fallback(
            PdfFontEmbeddingFallbackReason::InvalidFont,
        ));
    }

    Ok(HostFaceDecision::Embed {
        face,
        postscript_name,
    })
}

fn validate_vertical_metrics(
    face: &Face<'_>,
    metrics: &ResolvedFaceMetrics,
) -> Result<(), ExportError> {
    if u32::from(face.units_per_em()) != metrics.units_per_em
        || i32::from(face.ascender()) != metrics.ascender
        || i32::from(face.descender()) != metrics.descender
        || i32::from(face.line_gap()) != metrics.line_gap
    {
        return Err(ExportError::InvalidPdfFontPayload);
    }
    Ok(())
}

fn validate_scalar_metrics(
    face: &Face<'_>,
    metrics: &ResolvedFaceMetrics,
    visible_scalars: &BTreeSet<char>,
) -> Result<(), ExportError> {
    let glyph_zero_advance = face
        .glyph_hor_advance(GlyphId(0))
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    for scalar in visible_scalars {
        let expected = metrics
            .advance_for(*scalar)
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        let glyph = face.glyph_index(*scalar);
        let is_missing = glyph.map_or(true, |glyph_id| glyph_id.0 == 0);
        let glyph_id = glyph.unwrap_or(GlyphId(0));
        let advance = face
            .glyph_hor_advance(glyph_id)
            .or_else(|| is_missing.then_some(glyph_zero_advance))
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        if expected.missing_glyph != is_missing || u32::from(advance) != expected.advance_units {
            return Err(ExportError::InvalidPdfFontPayload);
        }
    }
    Ok(())
}

fn valid_postscript_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 127
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+'))
}
