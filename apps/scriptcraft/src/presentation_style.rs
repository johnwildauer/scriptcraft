//! Pure desktop presentation resolution for declared screenplay font families
//! and bounded FDX source colors. This module does not load fonts or paint.

use scriptcraft_core::formatting::EffectiveTextStyle;

/// Horizontal layout policy selected from the actual renderer face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum AdvanceMode {
    FixedCell,
    Proportional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RendererFamilyChoice {
    Arial,
    CourierPrime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ResolvedTextPresentation {
    pub family: RendererFamilyChoice,
    pub color_rgb8: Option<[u8; 3]>,
}

/// Resolves only an explicitly declared Arial family to Arial. All other
/// families, including hidden styles, retain the established Courier fallback.
pub(super) fn resolve_effective_text_style(
    style: &EffectiveTextStyle,
) -> ResolvedTextPresentation {
    ResolvedTextPresentation {
        family: if style.declared_family.eq_ignore_ascii_case("Arial") {
            RendererFamilyChoice::Arial
        } else {
            RendererFamilyChoice::CourierPrime
        },
        color_rgb8: style
            .source_color
            .as_deref()
            .and_then(parse_fdx_source_color),
    }
}

/// Parses the existing `#RRRRGGGGBBBB` FDX color spelling: three 16-bit
/// channels represented by twelve hexadecimal digits. Each channel projects
/// to RGB8 by taking its most-significant byte (truncation). Six-digit RGB,
/// whitespace, and all other forms are intentionally rejected.
pub(super) fn parse_fdx_source_color(value: &str) -> Option<[u8; 3]> {
    let channels = value.strip_prefix('#')?;
    if channels.len() != 12 || !channels.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    Some([
        parse_channel(channels, 0)?,
        parse_channel(channels, 4)?,
        parse_channel(channels, 8)?,
    ])
}

fn parse_channel(channels: &str, start: usize) -> Option<u8> {
    let end = start.checked_add(4)?;
    let channel = channels.get(start..end)?;
    let value = u16::from_str_radix(channel, 16).ok()?;
    Some((value >> 8) as u8)
}

