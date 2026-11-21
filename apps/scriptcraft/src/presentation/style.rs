//! Pure desktop presentation resolution for declared screenplay font families
//! and bounded FDX source colors. This module does not load fonts or paint.

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
