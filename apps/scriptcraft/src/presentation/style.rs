//! Pure desktop presentation resolution for declared screenplay font families
//! and bounded FDX source colors. This module does not load fonts or paint.

use scriptcraft_model::formatting::EffectiveTextStyle;
use scriptcraft_model::typography::{parse_effective_text_color, EffectiveTextColor};

/// Startup compatibility metadata retained by the old font bootstrap. Script
/// text paint and interaction do not branch on this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AdvanceMode {
    FixedCell,
    Proportional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RendererFamilyChoice {
    Arial,
    CourierPrime,
}

/// Resolve only the source-owned color spelling here; host faces and their
/// identities are obtained from the prepared document-font environment.
pub(crate) fn resolve_effective_text_color(style: &EffectiveTextStyle) -> EffectiveTextColor {
    parse_effective_text_color(style.source_color.as_deref())
}
