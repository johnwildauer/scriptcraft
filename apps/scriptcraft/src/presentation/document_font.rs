//! Desktop document-font ownership facade.
//!
//! Bundled startup registration, exact document preparation, and renderer
//! activation/readiness are implemented by separate private sibling modules.

use eframe::egui;
use scriptcraft_model::Project;

use super::document_font_preparation::DocumentFontPreparation;
use super::renderer_font_registration::RendererFontRegistration;

pub(crate) use super::document_font_preparation::{
    DocumentFontPreparationObservation, FontPreparationError, PreparedDocumentFonts,
};

pub(crate) fn install(context: &egui::Context) -> DocumentFontStore {
    super::bundled_font_startup::install(context);
    DocumentFontStore::default()
}

#[derive(Debug, Default)]
pub(crate) struct DocumentFontStore {
    preparation: DocumentFontPreparation,
    renderer_registration: RendererFontRegistration,
}

impl DocumentFontStore {
    pub(crate) fn prepare_document(
        &mut self,
        project: &Project,
    ) -> Result<PreparedDocumentFonts, FontPreparationError> {
        self.preparation.prepare_document(project)
    }

    pub(crate) fn prepare_document_observed(
        &mut self,
        project: &Project,
    ) -> Result<(PreparedDocumentFonts, DocumentFontPreparationObservation), FontPreparationError>
    {
        self.preparation.prepare_document_observed(project)
    }

    pub(crate) fn register_prepared_fonts(
        &mut self,
        context: &egui::Context,
        prepared: &PreparedDocumentFonts,
    ) -> Result<bool, FontPreparationError> {
        self.renderer_registration
            .register_prepared_fonts(context, prepared)
    }

    pub(crate) fn invalidate_catalog(&mut self) {
        self.preparation.invalidate_catalog();
    }
}
