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

#[cfg(test)]
mod tests {
    use super::{
        resolve_effective_text_style, RendererFamilyChoice,
    };
    use scriptcraft_core::formatting::EffectiveTextStyle;

    #[test]
    fn arial_family_matching_is_ascii_case_insensitive() {
        let style = test_style("aRiAl");
        assert_eq!(
            resolve_effective_text_style(&style).family,
            RendererFamilyChoice::Arial
        );
    }

    #[test]
    fn courier_and_unsupported_hidden_families_keep_courier_fallback() {
        for family in ["Courier Final Draft", "Courier Prime", "Helvetica"] {
            let mut style = test_style(family);
            style.hidden_text = family == "Helvetica";
            assert_eq!(
                resolve_effective_text_style(&style).family,
                RendererFamilyChoice::CourierPrime
            );
        }
    }



    fn test_style(family: &str) -> EffectiveTextStyle {
        EffectiveTextStyle {
            declared_family: family.to_owned(),
            family: scriptcraft_core::formatting::FontFallback::CourierPrime,
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            source_color: None,
        }
    }

}
