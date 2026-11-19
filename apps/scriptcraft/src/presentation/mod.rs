//! App-private shared presentation and resource adapters.

pub(crate) mod document_font;
pub(crate) mod host_font_catalog;
pub(crate) mod material_font;
// Shared app presentation geometry; Script View modules consume these primitives
// for painting and interaction.
#[allow(dead_code)]
pub(crate) mod geometry;
pub(crate) mod runtime_icon;
pub(crate) mod style;
pub(crate) mod wheel_zoom;
