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

    #[cfg(test)]
    pub(crate) fn with_test_faces(faces: Vec<NativeFontFace>) -> Self {
        Self::with_test_faces_and_query_log(faces).0
    }

    #[cfg(test)]
    pub(crate) fn with_test_faces_and_query_log(
        faces: Vec<NativeFontFace>,
    ) -> (Self, Arc<std::sync::Mutex<Vec<(String, FaceStyle)>>>) {
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        (
            Self::with_backend(Arc::new(TestFontSource {
                faces,
                calls: calls.clone(),
            })),
            calls,
        )
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

#[cfg(test)]
struct TestFontSource {
    faces: Vec<NativeFontFace>,
    calls: Arc<std::sync::Mutex<Vec<(String, FaceStyle)>>>,
}

#[cfg(test)]
impl FontSourceBackend for TestFontSource {
    fn query(&self, family: &str, style: FaceStyle, _limits: QueryLimits) -> BackendQuery {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push((family.to_owned(), style));
        }
        let matching = self
            .faces
            .iter()
            .filter(|face| family_matches(family, &face.family_label) && face.style == style)
            .cloned()
            .collect::<Vec<_>>();
        BackendQuery {
            candidate_count: matching.len(),
            read_count: matching.len(),
            faces: matching,
            overflowed: false,
        }
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::*;

    #[derive(Clone)]
    struct FakeSpec {
        family: String,
        style: FaceStyle,
        bytes: Option<Vec<u8>>,
        index: u32,
        candidate_count: usize,
        read_count: usize,
        overflowed: bool,
        duplicate_faces: usize,
    }

    impl FakeSpec {
        fn face(family: &str, style: FaceStyle) -> Self {
            Self {
                family: family.to_owned(),
                style,
                bytes: Some(sfnt_bytes()),
                index: 0,
                candidate_count: 1,
                read_count: 1,
                overflowed: false,
                duplicate_faces: 1,
            }
        }

        fn response(&self, requested: &str) -> BackendQuery {
            BackendQuery {
                candidate_count: self.candidate_count,
                read_count: self.read_count,
                faces: self
                    .bytes
                    .as_ref()
                    .map(|bytes| {
                        let face = NativeFontFace {
                            requested_family: requested.to_owned(),
                            family_label: self.family.clone(),
                            full_name: Some(format!("{} {:?}", self.family, self.style)),
                            postscript_name: Some("SyntheticFace".to_owned()),
                            style: self.style,
                            source: FontSourceLocation::Memory,
                            bytes: Arc::from(bytes.clone()),
                            collection_index: self.index,
                        };
                        vec![face; self.duplicate_faces]
                    })
                    .unwrap_or_default(),
                overflowed: self.overflowed,
            }
        }
    }

    struct FakeBackend {
        specs: Mutex<Vec<FakeSpec>>,
        queries: AtomicUsize,
        observed_limits: Mutex<Vec<QueryLimits>>,
    }

    impl FakeBackend {
        fn new(specs: Vec<FakeSpec>) -> Self {
            Self {
                specs: Mutex::new(specs),
                queries: AtomicUsize::new(0),
                observed_limits: Mutex::new(Vec::new()),
            }
        }

        fn query_count(&self) -> usize {
            self.queries.load(Ordering::SeqCst)
        }
    }

    impl FontSourceBackend for FakeBackend {
        fn query(&self, family: &str, style: FaceStyle, limits: QueryLimits) -> BackendQuery {
            self.queries.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut observed) = self.observed_limits.lock() {
                observed.push(limits);
            }
            let Ok(specs) = self.specs.lock() else {
                return BackendQuery::empty();
            };
            specs
                .iter()
                .find(|spec| family_matches(family, &spec.family) && spec.style == style)
                .or_else(|| {
                    specs
                        .iter()
                        .find(|spec| family_matches(family, &spec.family))
                })
                .or_else(|| specs.iter().find(|spec| spec.style == style))
                .map(|spec| spec.response(family))
                .unwrap_or_else(BackendQuery::empty)
        }
    }

    fn catalog(specs: Vec<FakeSpec>) -> (NativeFontCatalog, Arc<FakeBackend>) {
        let backend = Arc::new(FakeBackend::new(specs));
        (NativeFontCatalog::with_backend(backend.clone()), backend)
    }

    fn sfnt_bytes() -> Vec<u8> {
        let mut bytes = vec![0; 12];
        bytes[0..4].copy_from_slice(&[0, 1, 0, 0]);
        bytes[4..6].copy_from_slice(&1u16.to_be_bytes());
        bytes.resize(29, 0);
        bytes[12..16].copy_from_slice(b"test");
        bytes[20..24].copy_from_slice(&28u32.to_be_bytes());
        bytes[24..28].copy_from_slice(&1u32.to_be_bytes());
        bytes
    }

    fn collection_bytes(count: u32) -> Vec<u8> {
        let mut bytes = vec![0; 53];
        bytes[0..4].copy_from_slice(b"ttcf");
        bytes[8..12].copy_from_slice(&count.to_be_bytes());
        for index in 0..(count as usize).min(3) {
            let offset = 12 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&24u32.to_be_bytes());
        }
        bytes[24..53].copy_from_slice(&sfnt_bytes());
        bytes[44..48].copy_from_slice(&52u32.to_be_bytes());
        bytes
    }

    fn name_table(records: &[(u16, &str)]) -> Vec<u8> {
        let mut strings = Vec::new();
        let mut encoded_records = Vec::new();
        for (name_id, value) in records {
            let encoded = value
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect::<Vec<_>>();
            encoded_records.push((*name_id, encoded.len() as u16, strings.len() as u16));
            strings.extend(encoded);
        }
        let string_offset = 6 + records.len() * 12;
        let mut table = Vec::with_capacity(string_offset + strings.len());
        table.extend_from_slice(&0u16.to_be_bytes());
        table.extend_from_slice(&(records.len() as u16).to_be_bytes());
        table.extend_from_slice(&(string_offset as u16).to_be_bytes());
        for (name_id, length, offset) in encoded_records {
            table.extend_from_slice(&3u16.to_be_bytes());
            table.extend_from_slice(&1u16.to_be_bytes());
            table.extend_from_slice(&0x0409u16.to_be_bytes());
            table.extend_from_slice(&name_id.to_be_bytes());
            table.extend_from_slice(&length.to_be_bytes());
            table.extend_from_slice(&offset.to_be_bytes());
        }
        table.extend_from_slice(&strings);
        table
    }

    #[test]
    fn authoritative_subfamily_labels_distinguish_the_four_standard_styles() {
        let cases = [
            ("Medium", FaceStyle::Regular),
            ("Regular", FaceStyle::Regular),
            ("Medium Italic", FaceStyle::Italic),
            ("Bold", FaceStyle::Bold),
            ("Bold Italic", FaceStyle::BoldItalic),
        ];
        for (label, expected) in cases {
            assert_eq!(
                classify_subfamily_names(&[label.to_owned()]),
                Some(expected),
                "SFNT subfamily {label:?} must determine style independently of uniform host Properties"
            );
        }
    }

    #[test]
    fn exact_full_and_postscript_labels_override_uniform_or_bogus_properties() {
        let cases = [
            ("Futura Medium", "Futura-Medium", FaceStyle::Regular),
            (
                "Futura Medium Italic",
                "Futura-MediumItalic",
                FaceStyle::Italic,
            ),
            ("Futura Bold", "Futura-Bold", FaceStyle::Bold),
            (
                "Futura Bold Italic",
                "Futura-BoldItalic",
                FaceStyle::BoldItalic,
            ),
        ];
        for (full_name, postscript_name, expected) in cases {
            assert_eq!(
                classify_named_face_style("Futura", full_name, Some(postscript_name)),
                NamedStyleEvidence::Classified(expected)
            );
        }
        assert_eq!(
            classify_named_face_style(
                "Futura",
                "Futura Condensed Medium",
                Some("Futura-CondensedMedium")
            ),
            NamedStyleEvidence::Invalid
        );
        assert_eq!(
            classify_named_face_style("Futura", "Futura Medium", Some("Futura-Bold")),
            NamedStyleEvidence::Invalid
        );
        assert_eq!(
            classify_named_face_style("Other Family", "Futura Medium", Some("Futura-Medium")),
            NamedStyleEvidence::Missing
        );

        let mut faces = vec![
            (0, "Futura Medium", "Futura-Medium"),
            (1, "Futura Medium Italic", "Futura-MediumItalic"),
            (2, "Futura Bold", "Futura-Bold"),
            (3, "Futura Condensed Medium", "Futura-CondensedMedium"),
            (4, "Futura Condensed ExtraBold", "Futura-CondensedExtraBold"),
        ];
        let requested = [
            (FaceStyle::Regular, vec![0]),
            (FaceStyle::Bold, vec![2]),
            (FaceStyle::Italic, vec![1]),
            (FaceStyle::BoldItalic, Vec::new()),
        ];
        for (style, expected_indices) in requested {
            let matches = faces
                .iter()
                .filter_map(|(index, full, postscript)| {
                    (classify_named_face_style("Futura", full, Some(postscript))
                        == NamedStyleEvidence::Classified(style))
                    .then_some(*index)
                })
                .collect::<Vec<_>>();
            assert_eq!(matches, expected_indices, "bounded selection for {style:?}");
        }

        // A second authoritative exact face is still an ambiguity, not an
        // invitation to choose based on source order.
        faces.push((5, "Futura Medium", "Futura-Medium"));
        let regular_matches = faces
            .iter()
            .filter(|(_, full, postscript)| {
                classify_named_face_style("Futura", full, Some(postscript))
                    == NamedStyleEvidence::Classified(FaceStyle::Regular)
            })
            .count();
        assert_eq!(regular_matches, 2);
    }

    #[test]
    fn standard_arial_mt_postscript_names_classify_exactly() {
        let cases = [
            ("Arial", "ArialMT", FaceStyle::Regular),
            ("Arial Bold", "Arial-BoldMT", FaceStyle::Bold),
            ("Arial Italic", "Arial-ItalicMT", FaceStyle::Italic),
            (
                "Arial Bold Italic",
                "Arial-BoldItalicMT",
                FaceStyle::BoldItalic,
            ),
        ];
        for (full_name, postscript_name, expected) in cases {
            assert_eq!(
                classify_named_face_style("Arial", full_name, Some(postscript_name)),
                NamedStyleEvidence::Classified(expected),
                "standard Arial label pair {full_name:?} / {postscript_name:?}"
            );
        }

        assert_eq!(
            classify_named_face_style("Arial", "Arial Bold", Some("Arial-BoldItalicMT")),
            NamedStyleEvidence::Invalid,
            "meaningful full-name and PostScript evidence must agree"
        );
        assert_eq!(
            classify_named_face_style("Arial", "Arial BoldMT", None),
            NamedStyleEvidence::Invalid,
            "vendor suffix spellings are accepted only as PostScript evidence"
        );
        for postscript_name in ["Arial-NarrowMT", "Arial-CondensedMT", "Arial-ExtraBoldMT"] {
            assert_eq!(
                classify_named_face_style("Arial", "Arial", Some(postscript_name)),
                NamedStyleEvidence::Invalid,
                "unsupported Arial variant {postscript_name:?} must remain rejected"
            );
        }
    }

    #[test]
    fn condensed_unknown_and_conflicting_subfamilies_are_rejected() {
        for label in ["Condensed Medium", "ExtraBold", "Expanded", "Black"] {
            assert_eq!(
                classify_subfamily_names(&[label.to_owned()]),
                None,
                "unsupported variant {label:?} must not fall through to a standard style"
            );
        }
        assert_eq!(
            classify_subfamily_names(&["Medium".to_owned(), "Condensed Medium".to_owned()]),
            None,
            "conflicting family subfamily records must be rejected"
        );
    }

    #[test]
    fn sfnt_name_table_uses_subfamily_name_ids_and_rejects_ambiguity() {
        let table = name_table(&[(2, "Medium Italic"), (17, "Medium Italic")]);
        let names = sfnt_subfamily_names(&table).expect("valid SFNT name table");
        assert_eq!(classify_subfamily_names(&names), Some(FaceStyle::Italic));

        let conflicting = name_table(&[(2, "Medium"), (17, "Bold")]);
        let names = sfnt_subfamily_names(&conflicting).expect("valid SFNT name table");
        assert_eq!(classify_subfamily_names(&names), None);
    }

    #[test]
    fn original_memory_collection_bytes_and_face_index_are_preserved() {
        let bytes = Arc::new(collection_bytes(3));
        let handle = Handle::Memory {
            bytes: bytes.clone(),
            font_index: 2,
        };
        let (source, index) = source_location(&handle).expect("memory handle location");
        let acquired =
            original_handle_bytes(&handle, MAX_FACE_BYTES).expect("bounded source bytes");
        assert_eq!(source, FontSourceLocation::Memory);
        assert_eq!(index, 2);
        assert!(Arc::ptr_eq(&acquired, &bytes));
        assert_eq!(acquired.as_slice(), bytes.as_slice());
        assert!(valid_font_container(&acquired, index));
    }

    #[test]
    fn exact_family_and_case_insensitive_policy_are_validated() {
        let mut spec = FakeSpec::face("Example Sans", FaceStyle::Regular);
        spec.family = "eXaMpLe SaNs".to_owned();
        let (mut catalog, backend) = catalog(vec![spec]);
        let face = catalog
            .query_exact("EXAMPLE SANS", FaceStyle::Regular)
            .expect("case-insensitive exact family should resolve");
        assert_eq!(face.family_label, "eXaMpLe SaNs");
        assert_eq!(backend.query_count(), 1);
        assert!(catalog
            .query_exact("example sans", FaceStyle::Regular)
            .is_some());
        assert_eq!(
            backend.query_count(),
            1,
            "normalized exact queries are cached"
        );
    }

    #[test]
    fn four_styles_are_queried_independently() {
        let styles = [
            FaceStyle::Regular,
            FaceStyle::Bold,
            FaceStyle::Italic,
            FaceStyle::BoldItalic,
        ];
        let specs = styles
            .into_iter()
            .map(|style| FakeSpec::face("Four Styles", style))
            .collect();
        let (mut catalog, backend) = catalog(specs);
        for style in styles {
            assert_eq!(
                catalog
                    .query_exact("Four Styles", style)
                    .map(|face| face.style),
                Some(style)
            );
        }
        assert_eq!(backend.query_count(), 4);
    }

    #[test]
    fn nearest_family_and_wrong_style_are_rejected() {
        let (mut near_catalog, _) =
            catalog(vec![FakeSpec::face("Nearby Sans", FaceStyle::Regular)]);
        assert!(near_catalog
            .query_exact("Requested Sans", FaceStyle::Regular)
            .is_none());

        let (mut style_catalog, _) = catalog(vec![FakeSpec::face("Exact Sans", FaceStyle::Bold)]);
        assert!(style_catalog
            .query_exact("Exact Sans", FaceStyle::Regular)
            .is_none());
    }

    #[test]
    fn ambiguous_exact_faces_are_rejected_without_arbitrary_selection() {
        let mut ambiguous = FakeSpec::face("Ambiguous Family", FaceStyle::Regular);
        ambiguous.duplicate_faces = 2;
        let (mut catalog, _) = catalog(vec![ambiguous]);
        assert!(catalog
            .query_exact("Ambiguous Family", FaceStyle::Regular)
            .is_none());
    }

    #[test]
    fn missing_and_malformed_bytes_are_bounded_misses() {
        let mut missing = FakeSpec::face("Missing Bytes", FaceStyle::Regular);
        missing.bytes = None;
        let (mut missing_catalog, _) = catalog(vec![missing]);
        assert!(missing_catalog
            .query_exact("Missing Bytes", FaceStyle::Regular)
            .is_none());

        let mut malformed = FakeSpec::face("Malformed Bytes", FaceStyle::Regular);
        malformed.bytes = Some(vec![0; 12]);
        let (mut malformed_catalog, _) = catalog(vec![malformed]);
        assert!(malformed_catalog
            .query_exact("Malformed Bytes", FaceStyle::Regular)
            .is_none());
    }

    #[test]
    fn candidate_read_label_and_byte_limits_are_enforced() {
        let mut too_many = FakeSpec::face("Too Many", FaceStyle::Regular);
        too_many.candidate_count = MAX_CANDIDATES_PER_QUERY + 1;
        let (mut candidate_catalog, _) = catalog(vec![too_many]);
        assert!(candidate_catalog
            .query_exact("Too Many", FaceStyle::Regular)
            .is_none());

        let mut too_many_reads = FakeSpec::face("Too Many Reads", FaceStyle::Regular);
        too_many_reads.read_count = MAX_READS_PER_QUERY + 1;
        let (mut read_catalog, _) = catalog(vec![too_many_reads]);
        assert!(read_catalog
            .query_exact("Too Many Reads", FaceStyle::Regular)
            .is_none());

        let mut too_large = FakeSpec::face("Too Large", FaceStyle::Regular);
        too_large.bytes = Some(vec![0; MAX_FACE_BYTES + 1]);
        let (mut bytes_catalog, _) = catalog(vec![too_large]);
        assert!(bytes_catalog
            .query_exact("Too Large", FaceStyle::Regular)
            .is_none());

        let mut too_long_label = FakeSpec::face("Long Label", FaceStyle::Regular);
        too_long_label.family = "f".repeat(MAX_FAMILY_LABEL_BYTES + 1);
        let (mut label_catalog, _) = catalog(vec![too_long_label]);
        assert!(label_catalog
            .query_exact(&"f".repeat(MAX_FAMILY_LABEL_BYTES + 1), FaceStyle::Regular)
            .is_none());
    }

    #[test]
    fn collection_face_index_is_preserved_and_checked() {
        let mut valid = FakeSpec::face("Collection Face", FaceStyle::Regular);
        valid.bytes = Some(collection_bytes(3));
        valid.index = 2;
        let (mut valid_catalog, _) = catalog(vec![valid]);
        assert_eq!(
            valid_catalog
                .query_exact("Collection Face", FaceStyle::Regular)
                .map(|face| face.collection_index),
            Some(2)
        );

        let mut invalid = FakeSpec::face("Bad Collection Face", FaceStyle::Regular);
        invalid.bytes = Some(collection_bytes(2));
        invalid.index = 2;
        let (mut invalid_catalog, _) = catalog(vec![invalid]);
        assert!(invalid_catalog
            .query_exact("Bad Collection Face", FaceStyle::Regular)
            .is_none());
    }

    #[test]
    fn explicit_invalidation_performs_fresh_queries_and_misses_are_cached() {
        let (mut catalog, backend) = catalog(Vec::new());
        assert!(catalog
            .query_exact("Activated Later", FaceStyle::Regular)
            .is_none());
        assert!(catalog
            .query_exact("Activated Later", FaceStyle::Regular)
            .is_none());
        assert_eq!(backend.query_count(), 1);
        let old_generation = catalog.generation();
        if let Ok(mut specs) = backend.specs.lock() {
            specs.push(FakeSpec::face("Activated Later", FaceStyle::Regular));
        }
        catalog.invalidate();
        assert!(catalog.generation() > old_generation);
        assert!(catalog
            .query_exact("Activated Later", FaceStyle::Regular)
            .is_some());
        assert_eq!(backend.query_count(), 2);
    }

    #[test]
    fn backend_is_given_bounded_candidate_read_and_byte_budgets() {
        let (mut catalog, backend) = catalog(vec![FakeSpec::face("Budgeted", FaceStyle::Regular)]);
        let _ = catalog.query_exact("Budgeted", FaceStyle::Regular);
        let observed = backend.observed_limits.lock().ok();
        let Some(limits) = observed.and_then(|limits| limits.first().copied()) else {
            assert!(false, "the query backend should observe its budgets");
            return;
        };
        assert_eq!(limits.candidates, MAX_CANDIDATES_PER_QUERY);
        assert_eq!(limits.reads, MAX_READS_PER_QUERY);
        assert_eq!(limits.face_bytes, MAX_FACE_BYTES);
        assert_eq!(limits.total_bytes, MAX_TOTAL_BYTES_PER_QUERY);
    }
}
