use std::collections::BTreeMap;
use std::sync::Arc;

use eframe::egui::{
    self,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    FontData, FontFamily,
};

use super::document_font_preparation::{FontPreparationError, PreparedDocumentFonts};

const MAX_REGISTERED_FONT_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Default)]
pub(super) struct RendererFontRegistration {
    registered_bytes: BTreeMap<String, Arc<[u8]>>,
    registered_face_indices: BTreeMap<String, u32>,
    registered_family_names: BTreeMap<String, String>,
    registered_byte_count: usize,
}

impl RendererFontRegistration {
    /// Queues each unique byte/index face under a stable renderer binding and
    /// reports whether every required data key and family binding is active in
    /// the current egui pass. `Context::add_font` applies on the next pass, so
    /// queued bytes alone are not proof that painting is safe.
    pub(crate) fn register_prepared_fonts(
        &mut self,
        context: &egui::Context,
        prepared: &PreparedDocumentFonts,
    ) -> Result<bool, FontPreparationError> {
        // Preflight the full registration set so a mismatch cannot install only
        // a prefix of the document's resolved faces into the host context.
        let mut pending = BTreeMap::<String, (Arc<[u8]>, u32, String)>::new();
        for (id, bytes) in &prepared.face_bytes {
            let face_index = prepared
                .face_indices
                .get(id)
                .ok_or_else(|| FontPreparationError::bounded("registered-face-index-missing"))?;
            let data_key = prepared
                .egui_data_keys
                .get(id)
                .ok_or_else(|| FontPreparationError::bounded("registered-data-key-missing"))?;
            let family_name = prepared
                .egui_family_names
                .get(id)
                .ok_or_else(|| FontPreparationError::bounded("registered-family-missing"))?;
            if let Some((pending_bytes, pending_index, pending_family)) = pending.get(data_key) {
                if pending_bytes.as_ref() != bytes.as_ref() {
                    return Err(FontPreparationError::bounded("renderer-key-byte-collision"));
                }
                if pending_index != face_index {
                    return Err(FontPreparationError::bounded(
                        "renderer-key-index-collision",
                    ));
                }
                if pending_family != family_name {
                    return Err(FontPreparationError::bounded(
                        "renderer-key-family-collision",
                    ));
                }
            } else {
                pending.insert(
                    data_key.clone(),
                    (bytes.clone(), *face_index, family_name.clone()),
                );
            }
            if let Some(existing) = self.registered_bytes.get(data_key) {
                if existing.as_ref() != bytes.as_ref() {
                    return Err(FontPreparationError::bounded("renderer-key-byte-collision"));
                }
                if self.registered_face_indices.get(data_key) != Some(face_index) {
                    return Err(FontPreparationError::bounded(
                        "renderer-key-index-collision",
                    ));
                }
                if self.registered_family_names.get(data_key) != Some(family_name) {
                    return Err(FontPreparationError::bounded(
                        "renderer-key-family-collision",
                    ));
                }
            }
        }
        let additional_bytes = pending
            .iter()
            .filter(|(data_key, _)| !self.registered_bytes.contains_key(*data_key))
            .map(|(_, (bytes, _, _))| bytes.len())
            .fold(0usize, usize::saturating_add);
        if self.registered_byte_count.saturating_add(additional_bytes) > MAX_REGISTERED_FONT_BYTES {
            return Err(FontPreparationError::bounded("registered-font-byte-limit"));
        }
        for (data_key, (bytes, face_index, family_name)) in &pending {
            if self.registered_bytes.contains_key(data_key) {
                continue;
            }
            let mut font_data = FontData::from_owned(bytes.as_ref().to_vec());
            font_data.index = *face_index;
            context.add_font(FontInsert::new(
                data_key.as_str(),
                font_data,
                vec![InsertFontFamily {
                    family: FontFamily::Name(family_name.clone().into()),
                    priority: FontPriority::Highest,
                }],
            ));
            self.registered_bytes
                .insert(data_key.clone(), bytes.clone());
            self.registered_face_indices
                .insert(data_key.clone(), *face_index);
            self.registered_family_names
                .insert(data_key.clone(), family_name.clone());
            self.registered_byte_count = self.registered_byte_count.saturating_add(bytes.len());
        }

        let ready = context.fonts(|fonts| {
            let definitions = fonts.definitions();
            prepared.face_bytes.keys().all(|id| {
                let Some(family_name) = prepared.egui_family_names.get(id) else {
                    return false;
                };
                let Some(face_index) = prepared.face_indices.get(id) else {
                    return false;
                };
                let Some(data_key) = prepared.egui_data_keys.get(id) else {
                    return false;
                };
                let family = FontFamily::Name(family_name.clone().into());
                definitions
                    .font_data
                    .get(data_key)
                    .is_some_and(|data| data.index == *face_index)
                    && definitions.families.get(&family).is_some_and(|keys| {
                        keys.iter().any(|key| key.as_str() == data_key.as_str())
                    })
            })
        });
        Ok(ready)
    }


}
