//! Transient exact resolved-face bytes supplied to PDF export.
//!
//! The payload is deliberately separate from Project and canonical layout
//! inputs. Shared byte ownership keeps captured Print snapshots inexpensive.

use crate::ExportError;
use scriptcraft_model::typography::{
    content_fingerprint, ResolvedFontFace, ResolvedFontId, MAX_RESOLVED_FACES,
};
use std::{collections::BTreeMap, sync::Arc};

/// Exact bytes and collection face index corresponding to one resolved face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfResolvedFaceBytes {
    pub face: ResolvedFontFace,
    pub bytes: Arc<[u8]>,
    pub face_index: u32,
}

/// Ordered, transient face-byte set for one PDF export request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfFontPayload {
    faces: BTreeMap<ResolvedFontId, PdfResolvedFaceBytes>,
}

impl PdfFontPayload {
    /// Creates a payload while rejecting ambiguous or incoherent map entries.
    pub fn new(
        entries: impl IntoIterator<Item = (ResolvedFontId, PdfResolvedFaceBytes)>,
    ) -> Result<Self, ExportError> {
        let mut faces = BTreeMap::new();
        for (id, entry) in entries {
            if entry.bytes.is_empty()
                || id != entry.face.id
                || faces.contains_key(&id)
                || faces.len() >= MAX_RESOLVED_FACES
            {
                return Err(ExportError::InvalidPdfFontPayload);
            }
            faces.insert(id, entry);
        }
        Ok(Self { faces })
    }

    pub fn face(&self, id: &ResolvedFontId) -> Option<&PdfResolvedFaceBytes> {
        self.faces.get(id)
    }

    pub fn faces(&self) -> &BTreeMap<ResolvedFontId, PdfResolvedFaceBytes> {
        &self.faces
    }

    /// Requires the exact environment face and matching byte length/fingerprint;
    /// a fingerprint match never substitutes for complete metadata equality.
    pub(crate) fn require_coherent_face(
        &self,
        id: &ResolvedFontId,
        expected: &ResolvedFontFace,
    ) -> Result<&PdfResolvedFaceBytes, ExportError> {
        let entry = self
            .faces
            .get(id)
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        let byte_length = u64::try_from(entry.bytes.len())
            .map_err(|_| ExportError::InvalidPdfFontPayload)?;
        if entry.face.id != *id
            || entry.face != *expected
            || byte_length != entry.face.byte_length
            || content_fingerprint(&entry.bytes) != entry.face.content_fingerprint
        {
            return Err(ExportError::InvalidPdfFontPayload);
        }
        Ok(entry)
    }
}
