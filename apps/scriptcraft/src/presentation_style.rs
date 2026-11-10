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
        parse_fdx_source_color, resolve_effective_text_style, RendererFamilyChoice,
    };
    use scriptcraft_core::formatting::{EffectiveTextStyle, SourceLayoutSettings};
    use scriptcraft_core::Role;

    #[test]
    fn all_five_hidden_role_defaults_resolve_to_arial_and_default_rgb8() {
        for role in [
            Role::Summary,
            Role::Outline1,
            Role::Outline2,
            Role::Outline3,
            Role::Note,
        ] {
            let style = SourceLayoutSettings::default().text_style(role);
            let resolved = resolve_effective_text_style(&style);
            assert_eq!(resolved.family, RendererFamilyChoice::Arial);
            assert_eq!(resolved.color_rgb8, Some([0x87, 0x8f, 0xe0]));
        }
    }

    #[test]
    fn arial_family_matching_is_ascii_case_insensitive() {
        let mut style = test_style("aRiAl", None);
        style.hidden_text = false;
        assert_eq!(
            resolve_effective_text_style(&style).family,
            RendererFamilyChoice::Arial
        );
    }

    #[test]
    fn courier_and_unsupported_hidden_families_keep_courier_fallback() {
        for family in ["Courier Final Draft", "Courier Prime", "Helvetica"] {
            let mut style = test_style(family, None);
            style.hidden_text = family == "Helvetica";
            assert_eq!(
                resolve_effective_text_style(&style).family,
                RendererFamilyChoice::CourierPrime
            );
        }
    }

    #[test]
    fn valid_explicit_imported_color_is_resolved_instead_of_the_role_default() {
        let style = test_style("Arial", Some("#1234aBcDffff"));
        assert_eq!(
            resolve_effective_text_style(&style).color_rgb8,
            Some([0x12, 0xab, 0xff])
        );
    }

    #[test]
    fn malformed_and_unsupported_color_spellings_resolve_to_none() {
        for value in [
            "",
            "87878F8FE0E0",
            "#878FE0",
            "#87878F8FE0E",
            "#87878F8FE0E00",
            "#87878F8FE0EG",
            "#FFFFFFFFFFFFFFFF",
            " #87878F8FE0E0",
            "#ＦＦＦＦＦＦＦＦＦＦＦＦＦＦＦＦ",
        ] {
            assert_eq!(parse_fdx_source_color(value), None, "input: {value:?}");
            let style = test_style("Arial", Some(value));
            assert_eq!(
                resolve_effective_text_style(&style).color_rgb8,
                None,
                "resolved input: {value:?}"
            );
        }
        assert_eq!(parse_fdx_source_color("#FFFFFFFFFFFF"), Some([255; 3]));
    }

    #[test]
    fn resolution_does_not_mutate_the_effective_style_or_source_settings() {
        let mut settings = SourceLayoutSettings::default();
        let mut found_summary = false;
        for (role_index, role) in Role::ALL.iter().enumerate() {
            if *role == Role::Summary {
                settings.role_font_specs[role_index].family = Some("aRiAl".to_owned());
                settings.role_font_specs[role_index].color = Some("#1234567890ab".to_owned());
                found_summary = true;
                break;
            }
        }
        assert!(found_summary);
        let settings_before = settings.clone();
        let style = settings.text_style(Role::Summary);
        let style_before = style.clone();

        let resolved = resolve_effective_text_style(&style);

        assert_eq!(resolved.family, RendererFamilyChoice::Arial);
        assert_eq!(resolved.color_rgb8, Some([0x12, 0x56, 0x90]));
        assert_eq!(style, style_before);
        assert_eq!(settings, settings_before);
    }

    fn test_style(family: &str, source_color: Option<&str>) -> EffectiveTextStyle {
        EffectiveTextStyle {
            declared_family: family.to_owned(),
            family: scriptcraft_core::formatting::FontFallback::CourierPrime,
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            source_color: source_color.map(str::to_owned),
        }
    }
}
