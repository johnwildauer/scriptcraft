use std::sync::Arc;

use crate::presentation_style::{AdvanceMode, RendererFamilyChoice};
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub(crate) const COURIER_PRIME_FAMILY: &str = "Courier Prime";
const REGULAR_KEY: &str = "scriptcraft-courier-prime-regular";
const BOLD_KEY: &str = "scriptcraft-courier-prime-bold";
const ITALIC_KEY: &str = "scriptcraft-courier-prime-italic";
const BOLD_ITALIC_KEY: &str = "scriptcraft-courier-prime-bold-italic";

/// A host Arial face is accepted only up to this size. Reads are capped at one
/// byte over this limit so oversized files are detected without unbounded I/O.
pub(crate) const MAX_ARIAL_FACE_BYTES: usize = 16 * 1024 * 1024;

const ARIAL_REGULAR_FAMILY: &str = "ScriptCraft Arial Regular";
const ARIAL_BOLD_FAMILY: &str = "ScriptCraft Arial Bold";
const ARIAL_ITALIC_FAMILY: &str = "ScriptCraft Arial Italic";
const ARIAL_BOLD_ITALIC_FAMILY: &str = "ScriptCraft Arial Bold Italic";
const ARIAL_REGULAR_KEY: &str = "scriptcraft-arial-regular";
const ARIAL_BOLD_KEY: &str = "scriptcraft-arial-bold";
const ARIAL_ITALIC_KEY: &str = "scriptcraft-arial-italic";
const ARIAL_BOLD_ITALIC_KEY: &str = "scriptcraft-arial-bold-italic";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaceStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl FaceStyle {
    const ALL: [Self; 4] = [Self::Regular, Self::Bold, Self::Italic, Self::BoldItalic];

    const fn index(self) -> usize {
        match self {
            Self::Regular => 0,
            Self::Bold => 1,
            Self::Italic => 2,
            Self::BoldItalic => 3,
        }
    }

    const fn from_flags(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (false, false) => Self::Regular,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (true, true) => Self::BoldItalic,
        }
    }

    const fn bold(self) -> bool {
        matches!(self, Self::Bold | Self::BoldItalic)
    }

    const fn italic(self) -> bool {
        matches!(self, Self::Italic | Self::BoldItalic)
    }

    const fn arial_family(self) -> &'static str {
        match self {
            Self::Regular => ARIAL_REGULAR_FAMILY,
            Self::Bold => ARIAL_BOLD_FAMILY,
            Self::Italic => ARIAL_ITALIC_FAMILY,
            Self::BoldItalic => ARIAL_BOLD_ITALIC_FAMILY,
        }
    }

    const fn arial_key(self) -> &'static str {
        match self {
            Self::Regular => ARIAL_REGULAR_KEY,
            Self::Bold => ARIAL_BOLD_KEY,
            Self::Italic => ARIAL_ITALIC_KEY,
            Self::BoldItalic => ARIAL_BOLD_ITALIC_KEY,
        }
    }

    const fn courier_family(self) -> &'static str {
        match self {
            Self::Regular => COURIER_PRIME_FAMILY,
            Self::Bold => "Courier Prime Bold",
            Self::Italic => "Courier Prime Italic",
            Self::BoldItalic => "Courier Prime Bold Italic",
        }
    }
}

/// Which host Arial styles were actually loaded and registered at startup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FontAvailability {
    arial: [bool; 4],
}

/// The selected egui family and its advance policy, resolved together from the
/// requested logical family and the faces actually registered at startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedDocumentFace {
    pub family: FontFamily,
    pub advance_mode: AdvanceMode,
}

#[cfg(test)]
pub(super) fn all_arial_available_for_test() -> FontAvailability {
    FontAvailability { arial: [true; 4] }
}

pub(crate) fn font_definitions() -> (FontDefinitions, FontAvailability) {
    build_font_definitions(load_host_arial_faces())
}

fn build_font_definitions(
    arial_faces: [Option<Vec<u8>>; 4],
) -> (FontDefinitions, FontAvailability) {
    let mut definitions = FontDefinitions::default();
    add_face(
        &mut definitions,
        COURIER_PRIME_FAMILY,
        REGULAR_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Bold",
        BOLD_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Italic",
        ITALIC_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime Italic.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Bold Italic",
        BOLD_ITALIC_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold Italic.ttf"),
    );

    let mut availability = FontAvailability::default();
    for (style, face) in FaceStyle::ALL.into_iter().zip(arial_faces) {
        if let Some(bytes) = face {
            add_owned_face(
                &mut definitions,
                style.arial_family(),
                style.arial_key(),
                bytes,
            );
            availability.arial[style.index()] = true;
        }
    }
    (definitions, availability)
}

fn add_face(
    definitions: &mut FontDefinitions,
    family_name: &str,
    data_key: &str,
    bytes: &'static [u8],
) {
    definitions
        .font_data
        .insert(data_key.to_owned(), Arc::new(FontData::from_static(bytes)));
    definitions.families.insert(
        FontFamily::Name(family_name.into()),
        vec![data_key.to_owned()],
    );
}

fn add_owned_face(
    definitions: &mut FontDefinitions,
    family_name: &str,
    data_key: &str,
    bytes: Vec<u8>,
) {
    definitions
        .font_data
        .insert(data_key.to_owned(), Arc::new(FontData::from_owned(bytes)));
    definitions.families.insert(
        FontFamily::Name(family_name.into()),
        vec![data_key.to_owned()],
    );
}

/// Compatibility wrapper for family-only consumers. New paint consumers should
/// use [`resolve_document_face()`] so mode is derived from the same actual-face
/// decision. Missing Arial styles select matching Courier Prime, never a
/// synthesized Regular face.
pub(crate) fn family_for_style(
    choice: RendererFamilyChoice,
    bold: bool,
    italic: bool,
    availability: &FontAvailability,
) -> FontFamily {
    resolve_document_face(choice, bold, italic, availability).family
}

pub(crate) fn resolve_document_face(
    choice: RendererFamilyChoice,
    bold: bool,
    italic: bool,
    availability: &FontAvailability,
) -> ResolvedDocumentFace {
    let style = FaceStyle::from_flags(bold, italic);
    let (family, advance_mode) = match choice {
        RendererFamilyChoice::Arial if availability.arial[style.index()] => {
            (style.arial_family(), AdvanceMode::Proportional)
        }
        RendererFamilyChoice::Arial | RendererFamilyChoice::CourierPrime => {
            (style.courier_family(), AdvanceMode::FixedCell)
        }
    };
    ResolvedDocumentFace {
        family: FontFamily::Name(family.into()),
        advance_mode,
    }
}

pub(crate) fn install(context: &egui::Context) -> FontAvailability {
    let (definitions, availability) = font_definitions();
    context.set_fonts(definitions);
    availability
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidatePlatform {
    MacOs,
    Windows,
    Unsupported,
}

fn candidates_for_platform(
    platform: CandidatePlatform,
    windows_directory: Option<&Path>,
) -> [Option<PathBuf>; 4] {
    match platform {
        CandidatePlatform::MacOs => [
            Some(PathBuf::from("/System/Library/Fonts/Supplemental/Arial.ttf")),
            Some(PathBuf::from("/System/Library/Fonts/Supplemental/Arial Bold.ttf")),
            Some(PathBuf::from("/System/Library/Fonts/Supplemental/Arial Italic.ttf")),
            Some(PathBuf::from(
                "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf",
            )),
        ],
        CandidatePlatform::Windows => windows_directory.map_or([None, None, None, None], |root| {
            let fonts = root.join("Fonts");
            [
                Some(fonts.join("arial.ttf")),
                Some(fonts.join("arialbd.ttf")),
                Some(fonts.join("ariali.ttf")),
                Some(fonts.join("arialbi.ttf")),
            ]
        }),
        CandidatePlatform::Unsupported => [None, None, None, None],
    }
}

fn host_candidates() -> [Option<PathBuf>; 4] {
    #[cfg(target_os = "macos")]
    {
        candidates_for_platform(CandidatePlatform::MacOs, None)
    }
    #[cfg(target_os = "windows")]
    {
        let windows_directory = std::env::var_os("WINDIR")
            .or_else(|| std::env::var_os("SystemRoot"))
            .map(PathBuf::from);
        candidates_for_platform(CandidatePlatform::Windows, windows_directory.as_deref())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        candidates_for_platform(CandidatePlatform::Unsupported, None)
    }
}

fn read_bounded(path: &Path) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut limited = file.take((MAX_ARIAL_FACE_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn load_faces_with(
    candidates: [Option<PathBuf>; 4],
    mut read: impl FnMut(&Path) -> io::Result<Vec<u8>>,
) -> [Option<Vec<u8>>; 4] {
    std::array::from_fn(|index| {
        let path = candidates[index].as_deref()?;
        let bytes = read(path).ok()?;
        if bytes.is_empty() || bytes.len() > MAX_ARIAL_FACE_BYTES {
            return None;
        }
        Some(bytes)
    })
}

fn load_host_arial_faces() -> [Option<Vec<u8>>; 4] {
    load_faces_with(host_candidates(), read_bounded)
}

#[cfg(test)]
mod tests {
    use super::{
        build_font_definitions, candidates_for_platform, family_for_style, load_faces_with,
        resolve_document_face, CandidatePlatform, FaceStyle, FontAvailability,
        MAX_ARIAL_FACE_BYTES,
    };
    use crate::presentation_style::{AdvanceMode, RendererFamilyChoice};
    use eframe::egui::{FontDefinitions, FontFamily};
    use std::io;
    use std::path::{Path, PathBuf};

    fn test_candidates() -> [Option<PathBuf>; 4] {
        std::array::from_fn(|index| Some(PathBuf::from(format!("candidate-{index}"))))
    }

    #[test]
    fn registers_all_four_injected_arial_faces_with_stable_keys() {
        let mut next_byte = 0_u8;
        let faces = load_faces_with(test_candidates(), |_| {
            next_byte += 1;
            Ok(vec![next_byte])
        });
        let (definitions, availability) = build_font_definitions(faces);

        assert_eq!(availability.arial, [true; 4]);
        assert_bundled_courier_prime_definitions(&definitions);
        for style in FaceStyle::ALL {
            assert!(definitions.font_data.contains_key(style.arial_key()));
            assert!(definitions
                .families
                .contains_key(&FontFamily::Name(style.arial_family().into())));
            let resolved = resolve_document_face(
                RendererFamilyChoice::Arial,
                style.bold(),
                style.italic(),
                &availability,
            );
            assert_eq!(resolved.family, FontFamily::Name(style.arial_family().into()));
            assert_eq!(resolved.advance_mode, AdvanceMode::Proportional);
            assert_eq!(
                family_for_style(RendererFamilyChoice::Arial, style.bold(), style.italic(), &availability),
                FontFamily::Name(style.arial_family().into())
            );
        }
    }

    #[test]
    fn each_missing_arial_style_falls_back_to_its_corresponding_courier_face() {
        for missing in FaceStyle::ALL {
            let missing_path = PathBuf::from(format!("candidate-{}", missing.index()));
            let faces = load_faces_with(test_candidates(), |path| {
                if path == missing_path.as_path() {
                    Err(io::Error::new(io::ErrorKind::NotFound, "injected missing face"))
                } else {
                    Ok(vec![7])
                }
            });
            let (_, availability) = build_font_definitions(faces);

            assert!(!availability.arial[missing.index()]);
            assert_eq!(
                family_for_style(
                    RendererFamilyChoice::Arial,
                    missing.bold(),
                    missing.italic(),
                    &availability
                ),
                FontFamily::Name(missing.courier_family().into())
            );
            let resolved = resolve_document_face(
                RendererFamilyChoice::Arial,
                missing.bold(),
                missing.italic(),
                &availability,
            );
            assert_eq!(resolved.family, FontFamily::Name(missing.courier_family().into()));
            assert_eq!(resolved.advance_mode, AdvanceMode::FixedCell);
            assert_eq!(
                availability.arial.iter().filter(|available| **available).count(),
                3
            );
        }
    }

    #[test]
    fn unreadable_empty_and_oversized_faces_are_rejected_independently() {
        for (bad_index, result) in [
            (0, Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected read failure"))),
            (1, Ok(Vec::new())),
            (2, Ok(vec![0; MAX_ARIAL_FACE_BYTES + 1])),
        ] {
            let faces = load_faces_with(test_candidates(), |path| {
                if path == Path::new(&format!("candidate-{bad_index}")) {
                    result.as_ref().map(Clone::clone).map_err(|error| {
                        io::Error::new(error.kind(), error.to_string())
                    })
                } else {
                    Ok(vec![9])
                }
            });
            let (_, availability) = build_font_definitions(faces);
            assert!(!availability.arial[bad_index]);
            assert_eq!(availability.arial.iter().filter(|available| **available).count(), 3);
        }
    }

    #[test]
    fn unsupported_platform_has_no_candidates_or_font_reads() {
        let candidates = candidates_for_platform(CandidatePlatform::Unsupported, None);
        let mut reads = 0;
        let faces = load_faces_with(candidates, |_| {
            reads += 1;
            Ok(vec![1])
        });

        assert_eq!(reads, 0);
        assert!(faces.iter().all(Option::is_none));
        let (_, availability) = build_font_definitions(faces);
        for bold in [false, true] {
            for italic in [false, true] {
                assert_eq!(
                    family_for_style(
                        RendererFamilyChoice::Arial,
                        bold,
                        italic,
                        &availability
                    ),
                    family_for_style(
                        RendererFamilyChoice::CourierPrime,
                        bold,
                        italic,
                        &FontAvailability::default()
                    )
                );
            }
        }
    }

    #[test]
    fn explicit_courier_remains_fixed_for_every_style_even_when_arial_is_loaded() {
        let availability = FontAvailability { arial: [true; 4] };
        for style in FaceStyle::ALL {
            let resolved = resolve_document_face(
                RendererFamilyChoice::CourierPrime,
                style.bold(),
                style.italic(),
                &availability,
            );
            assert_eq!(resolved.family, FontFamily::Name(style.courier_family().into()));
            assert_eq!(resolved.advance_mode, AdvanceMode::FixedCell);
        }
    }

    #[test]
    fn target_candidate_locations_are_explicit_and_nonrecursive() {
        let mac = candidates_for_platform(CandidatePlatform::MacOs, None);
        assert_eq!(
            mac[0].as_deref(),
            Some(Path::new("/System/Library/Fonts/Supplemental/Arial.ttf"))
        );
        assert_eq!(
            mac[3].as_deref(),
            Some(Path::new(
                "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf"
            ))
        );
        let windows = candidates_for_platform(
            CandidatePlatform::Windows,
            Some(Path::new("C:/Windows")),
        );
        assert_eq!(windows[0].as_deref(), Some(Path::new("C:/Windows/Fonts/arial.ttf")));
        assert_eq!(windows[3].as_deref(), Some(Path::new("C:/Windows/Fonts/arialbi.ttf")));
    }

    #[test]
    fn bundled_courier_prime_definitions_remain_registered() {
        let (definitions, _) = build_font_definitions([None, None, None, None]);
        assert_bundled_courier_prime_definitions(&definitions);
    }

    fn assert_bundled_courier_prime_definitions(definitions: &FontDefinitions) {
        for (family, key) in [
            ("Courier Prime", "scriptcraft-courier-prime-regular"),
            ("Courier Prime Bold", "scriptcraft-courier-prime-bold"),
            ("Courier Prime Italic", "scriptcraft-courier-prime-italic"),
            (
                "Courier Prime Bold Italic",
                "scriptcraft-courier-prime-bold-italic",
            ),
        ] {
            assert!(definitions.font_data.contains_key(key));
            assert_eq!(
                definitions.families.get(&FontFamily::Name(family.into())),
                Some(&vec![key.to_owned()])
            );
        }
    }
}
