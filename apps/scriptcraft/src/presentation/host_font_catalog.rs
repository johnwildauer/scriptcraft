//! Bounded desktop-native font discovery and exact face acquisition.

use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use font_kit::font::Font as FontKitFont;
use font_kit::handle::Handle;
use font_kit::loader::Loader;
use font_kit::source::{Source, SystemSource};

const MAX_FAMILY_LABEL_BYTES: usize = 256;
const MAX_CANDIDATES_PER_QUERY: usize = 16;
const MAX_READS_PER_QUERY: usize = 8;
const MAX_FACE_BYTES: usize = 32 * 1024 * 1024;
const MAX_TOTAL_BYTES_PER_QUERY: usize = 48 * 1024 * 1024;
const MAX_NAME_TABLE_BYTES: usize = 64 * 1024;
const MAX_NAME_RECORDS: usize = 256;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum FaceStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FontSourceLocation {
    Path(PathBuf),
    Memory,
}

/// Exact face identity plus its owned source bytes. Native font objects are
/// intentionally short-lived inside the backend and never escape this adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeFontFace {
    pub requested_family: String,
    pub family_label: String,
    pub full_name: Option<String>,
    pub postscript_name: Option<String>,
    pub style: FaceStyle,
    pub source: FontSourceLocation,
    pub bytes: Arc<[u8]>,
    pub collection_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QueryLimits {
    candidates: usize,
    reads: usize,
    face_bytes: usize,
    total_bytes: usize,
}

const QUERY_LIMITS: QueryLimits = QueryLimits {
    candidates: MAX_CANDIDATES_PER_QUERY,
    reads: MAX_READS_PER_QUERY,
    face_bytes: MAX_FACE_BYTES,
    total_bytes: MAX_TOTAL_BYTES_PER_QUERY,
};

#[derive(Clone, Debug)]
struct BackendQuery {
    candidate_count: usize,
    read_count: usize,
    faces: Vec<NativeFontFace>,
    overflowed: bool,
}

trait FontSourceBackend: Send + Sync {
    fn query(&self, family: &str, style: FaceStyle, limits: QueryLimits) -> BackendQuery;
}

struct FontKitSystemBackend;

impl FontSourceBackend for FontKitSystemBackend {
    fn query(&self, requested_family: &str, style: FaceStyle, limits: QueryLimits) -> BackendQuery {
        let source = SystemSource::new();
        let Ok(family) = source.select_family_by_name(requested_family) else {
            return BackendQuery::empty();
        };
        if family.fonts().len() > limits.candidates {
            return BackendQuery {
                candidate_count: limits.candidates.saturating_add(1),
                read_count: 0,
                faces: Vec::new(),
                overflowed: true,
            };
        }

        let mut matching = Vec::new();
        let mut candidate_count = 0usize;
        for handle in family.fonts() {
            candidate_count = candidate_count.saturating_add(1);
            let Some((source_location, collection_index)) = source_location(handle) else {
                continue;
            };
            if !source_size_is_bounded(handle, limits.face_bytes) {
                continue;
            }
            let Ok(font) = handle.load() else {
                continue;
            };
            let family_label = font.family_name();
            if !family_matches(requested_family, &family_label)
                || classify_font_style(&font) != Some(style)
            {
                continue;
            }
            matching.push((
                handle.clone(),
                source_location,
                collection_index,
                family_label,
                Some(font.full_name()),
                font.postscript_name(),
            ));
        }

        // Several exact faces for the same family/style cannot be resolved
        // deterministically at this stage; refuse to choose one arbitrarily.
        if matching.len() != 1 || limits.reads == 0 {
            return BackendQuery {
                candidate_count,
                read_count: 0,
                faces: Vec::new(),
                overflowed: false,
            };
        }
        let Some((
            handle,
            source_location,
            collection_index,
            family_label,
            full_name,
            postscript_name,
        )) = matching.pop()
        else {
            return BackendQuery::empty();
        };
        let Some(bytes) = original_handle_bytes(&handle, limits.face_bytes) else {
            return BackendQuery {
                candidate_count,
                read_count: 1,
                faces: Vec::new(),
                overflowed: false,
            };
        };
        if bytes.len() > limits.face_bytes || bytes.len() > limits.total_bytes {
            return BackendQuery {
                candidate_count,
                read_count: 1,
                faces: Vec::new(),
                overflowed: false,
            };
        }
        BackendQuery {
            candidate_count,
            read_count: 1,
            faces: vec![NativeFontFace {
                requested_family: requested_family.to_owned(),
                family_label,
                full_name,
                postscript_name,
                style,
                source: source_location,
                bytes: Arc::from(bytes.as_slice()),
                collection_index,
            }],
            overflowed: false,
        }
    }
}

fn source_location(handle: &Handle) -> Option<(FontSourceLocation, u32)> {
    match handle {
        Handle::Path { path, font_index } => {
            Some((FontSourceLocation::Path(path.clone()), *font_index))
        }
        Handle::Memory { font_index, .. } => Some((FontSourceLocation::Memory, *font_index)),
    }
}

fn source_size_is_bounded(handle: &Handle, limit: usize) -> bool {
    match handle {
        Handle::Path { path, .. } => std::fs::metadata(path)
            .map(|metadata| metadata.len() <= limit as u64)
            .unwrap_or(false),
        Handle::Memory { bytes, .. } => bytes.len() <= limit,
    }
}

fn classify_font_style(font: &FontKitFont) -> Option<FaceStyle> {
    let family_label = font.family_name();
    match classify_named_face_style(
        &family_label,
        &font.full_name(),
        font.postscript_name().as_deref(),
    ) {
        NamedStyleEvidence::Classified(style) => return Some(style),
        NamedStyleEvidence::Invalid => return None,
        NamedStyleEvidence::Missing => {}
    }

    let table_tag = u32::from_be_bytes(*b"name");
    let name_table = font.load_font_table(table_tag)?;
    if name_table.len() > MAX_NAME_TABLE_BYTES {
        return None;
    }
    let subfamilies = sfnt_subfamily_names(&name_table)?;
    classify_subfamily_names(&subfamilies)
}

fn classify_subfamily_names(subfamilies: &[String]) -> Option<FaceStyle> {
    if subfamilies.is_empty() {
        return None;
    }
    let mut styles = subfamilies.iter().map(|name| classify_subfamily(name));
    let first = styles.next()??;
    if styles.any(|style| style != Some(first)) {
        return None;
    }
    Some(first)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NamedStyleEvidence {
    Missing,
    Classified(FaceStyle),
    Invalid,
}

fn classify_named_face_style(
    family: &str,
    full_name: &str,
    postscript_name: Option<&str>,
) -> NamedStyleEvidence {
    let full = named_label_style(full_name, family);
    let postscript = postscript_name
        .map(|name| named_postscript_label_style(name, family))
        .unwrap_or(Ok(None));
    match (full, postscript) {
        (Err(()), _) | (_, Err(())) => NamedStyleEvidence::Invalid,
        (Ok(Some(full)), Ok(Some(postscript))) if full != postscript => NamedStyleEvidence::Invalid,
        (Ok(Some(style)), _) | (_, Ok(Some(style))) => NamedStyleEvidence::Classified(style),
        (Ok(None), Ok(None)) => NamedStyleEvidence::Missing,
    }
}

fn named_label_style(label: &str, family: &str) -> Result<Option<FaceStyle>, ()> {
    named_label_style_with_suffix_classifier(label, family, classify_subfamily)
}

fn named_postscript_label_style(label: &str, family: &str) -> Result<Option<FaceStyle>, ()> {
    named_label_style_with_suffix_classifier(label, family, classify_postscript_suffix)
}

fn named_label_style_with_suffix_classifier(
    label: &str,
    family: &str,
    classify_suffix: fn(&str) -> Option<FaceStyle>,
) -> Result<Option<FaceStyle>, ()> {
    let Some(prefix) = label.get(..family.len()) else {
        return Ok(None);
    };
    if !prefix.eq_ignore_ascii_case(family) {
        return Ok(None);
    }
    let suffix = &label[family.len()..];
    let suffix = suffix.trim_start_matches([' ', '-']);
    if suffix.is_empty() {
        return Ok(None);
    }
    classify_suffix(suffix).map(Some).ok_or(())
}

fn classify_postscript_suffix(suffix: &str) -> Option<FaceStyle> {
    match suffix.to_ascii_lowercase().as_str() {
        // Arial's standard macOS PostScript names retain the legacy MT vendor
        // suffix after the style token (for example, Arial-BoldMT).
        "mt" => Some(FaceStyle::Regular),
        "boldmt" => Some(FaceStyle::Bold),
        "italicmt" => Some(FaceStyle::Italic),
        "bolditalicmt" => Some(FaceStyle::BoldItalic),
        _ => classify_subfamily(suffix),
    }
}

fn sfnt_subfamily_names(name_table: &[u8]) -> Option<Vec<String>> {
    let header = name_table.get(..6)?;
    let record_count = u16::from_be_bytes([header[2], header[3]]) as usize;
    let string_offset = u16::from_be_bytes([header[4], header[5]]) as usize;
    if record_count == 0 || record_count > MAX_NAME_RECORDS {
        return None;
    }
    let records_end = 6usize.checked_add(record_count.checked_mul(12)?)?;
    if records_end > name_table.len()
        || string_offset < records_end
        || string_offset > name_table.len()
    {
        return None;
    }
    let mut candidates = [Vec::<(u8, String)>::new(), Vec::<(u8, String)>::new()];
    for record in name_table[6..records_end].chunks_exact(12) {
        let platform_id = u16::from_be_bytes([record[0], record[1]]);
        let encoding_id = u16::from_be_bytes([record[2], record[3]]);
        let language_id = u16::from_be_bytes([record[4], record[5]]);
        let name_id = u16::from_be_bytes([record[6], record[7]]);
        if !matches!(name_id, 2 | 17) {
            continue;
        }
        let length = u16::from_be_bytes([record[8], record[9]]) as usize;
        let offset = u16::from_be_bytes([record[10], record[11]]) as usize;
        let start = string_offset.checked_add(offset)?;
        let end = start.checked_add(length)?;
        let bytes = name_table.get(start..end)?;
        let decoded = match platform_id {
            0 | 3 if length % 2 == 0 => String::from_utf16(
                &bytes
                    .chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                    .collect::<Vec<_>>(),
            )
            .ok(),
            // Mac Roman ASCII names are sufficient for these style labels;
            // reject non-ASCII data rather than misclassifying localized text.
            1 if encoding_id == 0 => std::str::from_utf8(bytes).ok().map(str::to_owned),
            _ => None,
        };
        if let Some(name) = decoded {
            if name.is_empty()
                || name.len() > MAX_FAMILY_LABEL_BYTES
                || name.chars().any(char::is_control)
            {
                return None;
            }
            let priority = match (platform_id, language_id) {
                (3, 0x0409) => 3,
                (0, 0) | (1, 0) => 2,
                _ => 1,
            };
            let slot = if name_id == 2 { 0 } else { 1 };
            candidates[slot].push((priority, name));
        }
    }
    let names = candidates
        .into_iter()
        .flat_map(|records| {
            let best_priority = records.iter().map(|(priority, _)| *priority).max();
            records
                .into_iter()
                .filter(move |(priority, _)| Some(*priority) == best_priority)
                .map(|(_, name)| name)
        })
        .collect::<Vec<_>>();
    (!names.is_empty()).then_some(names)
}

fn classify_subfamily(label: &str) -> Option<FaceStyle> {
    let normalized = label
        .trim()
        .chars()
        .map(|character| match character {
            '-' | '_' => ' ',
            other => other.to_ascii_lowercase(),
        })
        .collect::<String>();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    match normalized.as_str() {
        "regular" | "normal" | "roman" | "book" | "medium" => Some(FaceStyle::Regular),
        "italic" | "oblique" | "medium italic" | "medium oblique" => Some(FaceStyle::Italic),
        "bold" | "demi" | "demibold" | "semi bold" | "semibold" => Some(FaceStyle::Bold),
        "mediumitalic" | "mediumoblique" | "italicmedium" => Some(FaceStyle::Italic),
        "bolditalic" | "boldoblique" | "demiitalic" | "demibolditalic" | "semibolditalic" => {
            Some(FaceStyle::BoldItalic)
        }
        "bold italic" | "bold oblique" | "demi italic" | "demibold italic" | "semi bold italic"
        | "semibold italic" => Some(FaceStyle::BoldItalic),
        // Reject unsupported width/optical variants and unknown vendor labels.
        _ => None,
    }
}

fn original_handle_bytes(handle: &Handle, limit: usize) -> Option<Arc<Vec<u8>>> {
    match handle {
        Handle::Memory { bytes, .. } => (bytes.len() <= limit).then(|| bytes.clone()),
        Handle::Path { path, .. } => {
            let metadata = std::fs::metadata(path).ok()?;
            if metadata.len() > limit as u64 {
                return None;
            }
            let file = std::fs::File::open(path).ok()?;
            let mut bytes = Vec::new();
            file.take((limit as u64).saturating_add(1))
                .read_to_end(&mut bytes)
                .ok()?;
            (bytes.len() <= limit).then(|| Arc::new(bytes))
        }
    }
}

fn family_matches(requested: &str, actual: &str) -> bool {
    requested.trim().eq_ignore_ascii_case(actual.trim())
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct QueryKey {
    family: String,
    style: FaceStyle,
}

/// Cached, injectable host catalog. Query misses are cached until explicit
/// invalidation, which clears results so the next lookup performs a fresh source
/// query without promising automatic OS font-activation notifications.
pub(crate) struct NativeFontCatalog {
    backend: Arc<dyn FontSourceBackend>,
    cache: HashMap<QueryKey, Option<NativeFontFace>>,
    generation: u64,
}

impl fmt::Debug for NativeFontCatalog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeFontCatalog")
            .field("cached_queries", &self.cache.len())
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl Default for NativeFontCatalog {
    fn default() -> Self {
        Self::with_backend(Arc::new(FontKitSystemBackend))
    }
}

impl NativeFontCatalog {
    fn with_backend(backend: Arc<dyn FontSourceBackend>) -> Self {
        Self {
            backend,
            cache: HashMap::new(),
            generation: 1,
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn query_exact(&mut self, family: &str, style: FaceStyle) -> Option<NativeFontFace> {
        let requested = family.trim();
        if requested.is_empty()
            || requested.len() > MAX_FAMILY_LABEL_BYTES
            || requested.chars().any(char::is_control)
        {
            return None;
        }
        let key = QueryKey {
            family: requested.to_ascii_lowercase(),
            style,
        };
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone().map(|mut face| {
                face.requested_family = requested.to_owned();
                face
            });
        }

        let result = self
            .backend
            .query(requested, style, QUERY_LIMITS)
            .validate(requested, style);
        self.cache.insert(key, result.clone());
        result
    }

    pub(crate) fn invalidate(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.cache.clear();
    }
}

impl BackendQuery {
    fn empty() -> Self {
        Self {
            candidate_count: 0,
            read_count: 0,
            faces: Vec::new(),
            overflowed: false,
        }
    }

    fn validate(self, requested: &str, requested_style: FaceStyle) -> Option<NativeFontFace> {
        if self.overflowed
            || self.candidate_count > QUERY_LIMITS.candidates
            || self.read_count > QUERY_LIMITS.reads
            || self.faces.len() != 1
        {
            return None;
        }
        let mut faces = self.faces.into_iter();
        let face = faces.next()?;
        if !family_matches(requested, &face.family_label)
            || face.style != requested_style
            || !label_is_bounded(&face.requested_family)
            || !label_is_bounded(&face.family_label)
            || face
                .full_name
                .as_ref()
                .is_some_and(|name| !label_is_bounded(name))
            || face
                .postscript_name
                .as_ref()
                .is_some_and(|name| !label_is_bounded(name))
            || face.bytes.is_empty()
            || face.bytes.len() > QUERY_LIMITS.face_bytes
            || face.bytes.len() > QUERY_LIMITS.total_bytes
            || !valid_font_container(&face.bytes, face.collection_index)
        {
            return None;
        }
        Some(NativeFontFace {
            requested_family: requested.to_owned(),
            ..face
        })
    }
}

fn label_is_bounded(label: &str) -> bool {
    label.len() <= MAX_FAMILY_LABEL_BYTES && !label.chars().any(char::is_control)
}

fn valid_font_container(bytes: &[u8], collection_index: u32) -> bool {
    if bytes.len() < 12 {
        return false;
    }
    if bytes.get(0..4) == Some(b"ttcf") {
        let Some(count_bytes) = bytes.get(8..12) else {
            return false;
        };
        let count = u32::from_be_bytes([
            count_bytes[0],
            count_bytes[1],
            count_bytes[2],
            count_bytes[3],
        ]);
        if count == 0 || collection_index >= count {
            return false;
        }
        let Some(offset_table_end) = 12usize.checked_add((count as usize).saturating_mul(4)) else {
            return false;
        };
        if offset_table_end > bytes.len() {
            return false;
        }
        let face_offset_position =
            12usize.saturating_add((collection_index as usize).saturating_mul(4));
        let Some(face_offset_bytes) =
            bytes.get(face_offset_position..face_offset_position.saturating_add(4))
        else {
            return false;
        };
        let face_offset = u32::from_be_bytes([
            face_offset_bytes[0],
            face_offset_bytes[1],
            face_offset_bytes[2],
            face_offset_bytes[3],
        ]) as usize;
        return valid_sfnt_at(bytes, face_offset);
    }
    collection_index == 0 && valid_sfnt_at(bytes, 0)
}

fn valid_sfnt_at(bytes: &[u8], start: usize) -> bool {
    let Some(header) = bytes.get(start..start.saturating_add(12)) else {
        return false;
    };
    if !matches!(
        header.get(0..4),
        Some(b"\0\x01\0\0" | b"OTTO" | b"true" | b"typ1")
    ) {
        return false;
    }
    let Some(table_count_bytes) = header.get(4..6) else {
        return false;
    };
    let table_count = u16::from_be_bytes([table_count_bytes[0], table_count_bytes[1]]) as usize;
    if table_count == 0 {
        return false;
    }
    let Some(directory_end) = start
        .checked_add(12)
        .and_then(|header_end| header_end.checked_add(table_count.saturating_mul(16)))
    else {
        return false;
    };
    if directory_end > bytes.len() {
        return false;
    }
    (0..table_count).all(|table_index| {
        let record_start = start
            .saturating_add(12)
            .saturating_add(table_index.saturating_mul(16));
        let Some(record) = bytes.get(record_start..record_start.saturating_add(16)) else {
            return false;
        };
        let offset = u32::from_be_bytes([record[8], record[9], record[10], record[11]]) as usize;
        let length = u32::from_be_bytes([record[12], record[13], record[14], record[15]]) as usize;
        length > 0
            && offset
                .checked_add(length)
                .is_some_and(|end| end <= bytes.len())
    })
}

