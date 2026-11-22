//! Deterministic, bounded PDF 1.4 byte writer.
//!
//! Object IDs are reserved in caller-defined order, every reservation must be
//! written exactly once, and stream payloads remain bytes through final output.
//! The writer emits classic xref offsets and rejects dangling references.

use crate::ExportError;

pub(crate) const MAX_PDF_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdfObjectId(pub(crate) usize);

#[derive(Debug)]
enum PdfObject {
    Bytes {
        body: Vec<u8>,
        references: Vec<PdfObjectId>,
    },
    Stream {
        dictionary_entries: Vec<u8>,
        data: Vec<u8>,
        references: Vec<PdfObjectId>,
    },
}

/// A per-export object table. Allocation is stable and serialization is
/// bounded at every append, including object wrappers, xref, and trailer.
#[derive(Debug, Default)]
pub(crate) struct PdfDocumentWriter {
    objects: Vec<Option<PdfObject>>,
    retained_bytes: usize,
    reserved_overhead: usize,
}

impl PdfDocumentWriter {
    pub(crate) fn reserve_object(&mut self) -> Result<PdfObjectId, ExportError> {
        let id = self
            .objects
            .len()
            .checked_add(1)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        if id > u32::MAX as usize {
            return Err(ExportError::OutputResourceLimitExceeded);
        }
        let object_overhead = id
            .to_string()
            .len()
            .checked_add(35)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        let reserved_overhead = self
            .reserved_overhead
            .checked_add(object_overhead)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        let estimated_total = self
            .retained_bytes
            .checked_add(reserved_overhead)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        if estimated_total > MAX_PDF_OUTPUT_BYTES {
            return Err(ExportError::OutputResourceLimitExceeded);
        }
        self.objects.push(None);
        self.reserved_overhead = reserved_overhead;
        Ok(PdfObjectId(id))
    }

    pub(crate) fn write_object(
        &mut self,
        id: PdfObjectId,
        body: Vec<u8>,
        references: Vec<PdfObjectId>,
    ) -> Result<(), ExportError> {
        self.store(
            id,
            PdfObject::Bytes { body, references },
        )
    }

    pub(crate) fn write_stream(
        &mut self,
        id: PdfObjectId,
        dictionary_entries: Vec<u8>,
        data: Vec<u8>,
        references: Vec<PdfObjectId>,
    ) -> Result<(), ExportError> {
        self.store(
            id,
            PdfObject::Stream {
                dictionary_entries,
                data,
                references,
            },
        )
    }

    fn store(&mut self, id: PdfObjectId, object: PdfObject) -> Result<(), ExportError> {
        let index = id
            .0
            .checked_sub(1)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        if self.objects.get(index).ok_or(ExportError::OutputResourceLimitExceeded)?.is_some() {
            return Err(ExportError::OutputResourceLimitExceeded);
        }
        let retained = match &object {
            PdfObject::Bytes { body, .. } => body.len(),
            PdfObject::Stream {
                dictionary_entries,
                data,
                ..
            } => dictionary_entries
                .len()
                .checked_add(data.len())
                .ok_or(ExportError::OutputResourceLimitExceeded)?,
        };
        self.retained_bytes = self
            .retained_bytes
            .checked_add(retained)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        let estimated_total = self
            .retained_bytes
            .checked_add(self.reserved_overhead)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        if estimated_total > MAX_PDF_OUTPUT_BYTES {
            return Err(ExportError::OutputResourceLimitExceeded);
        }
        self.objects[index] = Some(object);
        Ok(())
    }

    /// Finalizes only a complete graph. No partial output escapes on error.
    pub(crate) fn finish(self, root: PdfObjectId) -> Result<Vec<u8>, ExportError> {
        if root.0 == 0 || root.0 > self.objects.len() {
            return Err(ExportError::OutputResourceLimitExceeded);
        }
        for object in &self.objects {
            let object = object
                .as_ref()
                .ok_or(ExportError::OutputResourceLimitExceeded)?;
            let references = match object {
                PdfObject::Bytes { references, .. }
                | PdfObject::Stream { references, .. } => references,
            };
            if references
                .iter()
                .any(|reference| reference.0 == 0 || reference.0 > self.objects.len())
            {
                return Err(ExportError::OutputResourceLimitExceeded);
            }
        }

        let mut output = Vec::new();
        append(&mut output, b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n")?;
        let mut offsets = Vec::with_capacity(self.objects.len());
        for (index, object) in self.objects.iter().enumerate() {
            let id = index
                .checked_add(1)
                .ok_or(ExportError::OutputResourceLimitExceeded)?;
            offsets.push(output.len());
            append(&mut output, format!("{id} 0 obj\n").as_bytes())?;
            match object.as_ref().ok_or(ExportError::OutputResourceLimitExceeded)? {
                PdfObject::Bytes { body, .. } => append(&mut output, body)?,
                PdfObject::Stream {
                    dictionary_entries,
                    data,
                    ..
                } => {
                    let length = u64::try_from(data.len())
                        .map_err(|_| ExportError::OutputResourceLimitExceeded)?;
                    append(
                        &mut output,
                        format!("<< /Length {length} ").as_bytes(),
                    )?;
                    append(&mut output, dictionary_entries)?;
                    append(&mut output, b" >>\nstream\n")?;
                    append(&mut output, data)?;
                    append(&mut output, b"\nendstream")?;
                }
            }
            append(&mut output, b"\nendobj\n")?;
        }

        let xref_offset = output.len();
        let xref_entries = offsets
            .len()
            .checked_add(1)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        append(
            &mut output,
            format!("xref\n0 {xref_entries}\n0000000000 65535 f \n").as_bytes(),
        )?;
        for offset in offsets {
            if offset > 9_999_999_999usize {
                return Err(ExportError::OutputResourceLimitExceeded);
            }
            append(&mut output, format!("{offset:010} 00000 n \n").as_bytes())?;
        }
        let trailer_size = self
            .objects
            .len()
            .checked_add(1)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
        append(
            &mut output,
            format!(
                "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
                trailer_size,
                root.0,
            )
            .as_bytes(),
        )?;
        Ok(output)
    }
}

fn append(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ExportError> {
    let new_len = output
        .len()
        .checked_add(bytes.len())
        .ok_or(ExportError::OutputResourceLimitExceeded)?;
    if new_len > MAX_PDF_OUTPUT_BYTES {
        return Err(ExportError::OutputResourceLimitExceeded);
    }
    output.extend_from_slice(bytes);
    Ok(())
}
