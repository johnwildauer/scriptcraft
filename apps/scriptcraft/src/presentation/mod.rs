//! App-private shared presentation and resource adapters.

pub(crate) mod document_font;
pub(crate) mod material_font;
// This primitive is intentionally prepared for WO-001a.3 and not yet wired
// into visible painting.
#[allow(dead_code)]
pub(crate) mod geometry;
pub(crate) mod style;
pub(crate) mod runtime_icon;
pub(crate) mod wheel_zoom;
