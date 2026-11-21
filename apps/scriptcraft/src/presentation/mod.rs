//! App-private shared presentation and resource adapters.

pub(crate) mod document_font;
mod bundled_font_startup;
mod document_font_preparation;
mod renderer_font_registration;
pub(crate) mod host_font_catalog;
pub(crate) mod material_font;
// Shared app presentation geometry; Script View modules consume these primitives
// for painting and interaction.
pub(crate) mod geometry;
pub(crate) mod runtime_icon;
pub(crate) mod style;
pub(crate) mod wheel_zoom;
