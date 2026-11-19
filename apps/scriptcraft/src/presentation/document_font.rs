use std::sync::Arc;

use super::style::{AdvanceMode, RendererFamilyChoice};
use eframe::egui::{
    self,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    FontData, FontDefinitions, FontFamily,
};
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
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Bold",
        BOLD_KEY,
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Bold.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Italic",
        ITALIC_KEY,
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Italic.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Bold Italic",
        BOLD_ITALIC_KEY,
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Bold Italic.ttf"),
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

pub(crate) fn install(context: &egui::Context) -> DocumentFontStore {
    let (definitions, availability) = font_definitions();
    context.set_fonts(definitions);
    DocumentFontStore::new(availability)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidatePlatform {
    MacOs,
    Windows,
    Linux,
    Unsupported,
}

fn read_bounded(path: &Path) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut limited = file.take((MAX_ARIAL_FACE_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn load_host_arial_faces() -> [Option<Vec<u8>>; 4] {
    std::array::from_fn(|index| {
        let style = ResolvedFaceStyle::ALL[index];
        let candidates = host_font_candidates(FamilySelection::Sans, style);
        let sample = BTreeSet::from(['?']);
        candidates.into_iter().find_map(|candidate| {
            let path = candidate.path?;
            let bytes = read_bounded(&path).ok()?;
            if bytes.is_empty() || bytes.len() > MAX_ARIAL_FACE_BYTES {
                return None;
            }
            metrics_for_bytes(&bytes, &sample).ok()?;
            Some(bytes)
        })
    })
}

// WO-005.3 document resolver. The compatibility helpers above remain only for
// the existing WO-005.7 paint migration; canonical layout preparation uses the
// explicit model environment below and never exposes host paths.

use ab_glyph::{Font, FontRef};
use scriptcraft_model::typography::{
    content_fingerprint, DocumentFontEnvironment, FaceStyle as ResolvedFaceStyle,
    FontResolutionKind, RequestedFontResolution, ResolvedFaceMetrics, ResolvedFontFace,
    ResolvedFontId, ResolvedFontSourceKind, ScalarAdvance, MAX_FONT_REQUESTS, MAX_RESOLVED_FACES,
    MAX_SCALAR_ADVANCES,
};
use scriptcraft_model::{Project, TextRun};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Deref;

const MAX_DOCUMENT_FONT_SCALARS: usize = MAX_SCALAR_ADVANCES;
const MAX_DOCUMENT_FONT_REQUESTS: usize = MAX_FONT_REQUESTS;
const MAX_PREPARATION_BYTES: usize = 64 * 1024 * 1024;
const MAX_HOST_CANDIDATES: usize = 24;
const MAX_PREPARED_ENVIRONMENTS: usize = 2;
const MAX_REGISTERED_FONT_BYTES: usize = 256 * 1024 * 1024;
const HOST_CATALOG_GENERATION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FontPreparationError {
    pub(crate) reason: &'static str,
}

impl FontPreparationError {
    fn bounded(reason: &'static str) -> Self {
        Self { reason }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PreparationSignature {
    generation: u64,
    families: BTreeSet<String>,
    scalars: BTreeSet<char>,
}

/// Prepared model metrics and the same exact bytes/family names used by the
/// desktop renderer. Font bytes and this value are transient, never project data.
#[derive(Debug, Clone)]
pub(crate) struct PreparedDocumentFonts {
    pub(crate) environment: Arc<DocumentFontEnvironment>,
    pub(crate) egui_family_names: BTreeMap<ResolvedFontId, String>,
    face_bytes: BTreeMap<ResolvedFontId, Arc<[u8]>>,
    pub(crate) fallback_faces: BTreeSet<(String, ResolvedFaceStyle)>,
    signature: PreparationSignature,
}

impl PreparedDocumentFonts {
    pub(crate) fn face_bytes(&self, id: &ResolvedFontId) -> Option<&Arc<[u8]>> {
        self.face_bytes.get(id)
    }

    pub(crate) fn signature_matches(&self, other: &Self) -> bool {
        self.signature == other.signature && self.environment == other.environment
    }
}

/// Desktop-owned discovery/registration authority. The deref target is a
/// transitional read-only adapter for current paint call sites; new resolution
/// is performed by `prepare_document` and driven only by project values.
#[derive(Debug)]
pub(crate) struct DocumentFontStore {
    legacy_availability: FontAvailability,
    host_catalog: HostFontCatalog,
    prepared_cache: HashMap<PreparationSignature, PreparedDocumentFonts>,
    registered_bytes: BTreeMap<ResolvedFontId, Arc<[u8]>>,
    registered_byte_count: usize,
    catalog_generation: u64,
}

impl Default for DocumentFontStore {
    fn default() -> Self {
        Self::new(FontAvailability::default())
    }
}

impl Deref for DocumentFontStore {
    type Target = FontAvailability;

    fn deref(&self) -> &Self::Target {
        &self.legacy_availability
    }
}

impl DocumentFontStore {
    fn new(legacy_availability: FontAvailability) -> Self {
        Self {
            legacy_availability,
            host_catalog: HostFontCatalog::discover(),
            prepared_cache: HashMap::new(),
            registered_bytes: BTreeMap::new(),
            registered_byte_count: 0,
            catalog_generation: HOST_CATALOG_GENERATION,
        }
    }

    /// Prepares all declared families and the complete displayed scalar
    /// repertoire as one transaction. Any bound/parser/metric failure returns
    /// an error instead of publishing partial metrics.
    pub(crate) fn prepare_document(
        &mut self,
        project: &Project,
    ) -> Result<PreparedDocumentFonts, FontPreparationError> {
        let signature = project_font_signature(project, self.catalog_generation)?;
        if let Some(cached) = self.prepared_cache.get(&signature) {
            return Ok(cached.clone());
        }

        let mut faces = BTreeMap::new();
        let mut face_bytes = BTreeMap::new();
        let mut family_names = BTreeMap::new();
        let mut fallback_faces = BTreeSet::new();
        let bundled = bundled_courier_prime_faces();
        let mut bundled_ids = [None, None, None, None];
        for style in ResolvedFaceStyle::ALL {
            let index = resolved_style_index(style);
            let bytes = bundled[index];
            let metrics = metrics_for_bytes(bytes, &signature.scalars)?;
            let id = insert_resolved_face(
                &mut faces,
                &mut face_bytes,
                &mut family_names,
                bytes,
                "Courier Prime",
                courier_style_name(style),
                style,
                ResolvedFontSourceKind::Bundled,
                metrics,
            )?;
            bundled_ids[index] = Some(id);
        }

        let mut resolutions = BTreeMap::new();
        let mut bytes_read = 0usize;
        let mut selected_host_faces: HashMap<
            (FamilySelection, ResolvedFaceStyle),
            Option<ParsedHostFace>,
        > = HashMap::new();
        for family in &signature.families {
            let (kind, selection) = classify_requested_family(family);
            let mut selected = Vec::with_capacity(4);
            for style in ResolvedFaceStyle::ALL {
                let chosen = match selection {
                    FamilySelection::Courier => bundled_ids[resolved_style_index(style)]
                        .clone()
                        .ok_or_else(|| FontPreparationError::bounded("bundled-face-missing"))?,
                    FamilySelection::Sans | FamilySelection::Serif => {
                        let key = (selection, style);
                        if !selected_host_faces.contains_key(&key) {
                            let selected = resolve_host_style_with(
                                self.host_catalog.candidates(selection, style).to_vec(),
                                &signature.scalars,
                                &mut bytes_read,
                                read_bounded,
                            )?;
                            selected_host_faces.insert(key, selected);
                        }
                        match selected_host_faces.get(&key).cloned().flatten() {
                            Some(candidate) => insert_resolved_face(
                                &mut faces,
                                &mut face_bytes,
                                &mut family_names,
                                &candidate.bytes,
                                &candidate.family_label,
                                &candidate.face_label,
                                style,
                                ResolvedFontSourceKind::Host,
                                candidate.metrics,
                            )?,
                            None => {
                                fallback_faces.insert((family.clone(), style));
                                bundled_ids[resolved_style_index(style)]
                                    .clone()
                                    .ok_or_else(|| {
                                        FontPreparationError::bounded("bundled-face-missing")
                                    })?
                            }
                        }
                    }
                };
                selected.push(chosen);
            }
            if selected.len() != 4 {
                return Err(FontPreparationError::bounded("incomplete-face-selection"));
            }
            let resolution_kind = match kind {
                FamilyKind::CourierAlias => FontResolutionKind::CourierAlias,
                FamilyKind::Sans => FontResolutionKind::NearestSans,
                FamilyKind::Serif => FontResolutionKind::NearestSerif,
                FamilyKind::Unknown => FontResolutionKind::UnknownFallback,
            };
            resolutions.insert(
                family.clone(),
                RequestedFontResolution {
                    kind: resolution_kind,
                    regular: selected[0].clone(),
                    bold: selected[1].clone(),
                    italic: selected[2].clone(),
                    bold_italic: selected[3].clone(),
                },
            );
        }
        if faces.len() > MAX_RESOLVED_FACES {
            return Err(FontPreparationError::bounded("too-many-resolved-faces"));
        }
        let environment = DocumentFontEnvironment::new(resolutions, faces)
            .map_err(|_| FontPreparationError::bounded("invalid-resolved-environment"))?;
        let prepared = PreparedDocumentFonts {
            environment: Arc::new(environment),
            egui_family_names: family_names,
            face_bytes,
            fallback_faces,
            signature: signature.clone(),
        };
        if self.prepared_cache.len() >= MAX_PREPARED_ENVIRONMENTS {
            self.prepared_cache.clear();
        }
        self.prepared_cache.insert(signature, prepared.clone());
        Ok(prepared)
    }

    /// Queues the exact prepared payloads under unique resolved-ID names and
    /// reports whether every required data key and family binding is active in
    /// the current egui pass. `Context::add_font` applies on the next pass, so
    /// queued bytes alone are not proof that painting is safe.
    pub(crate) fn register_prepared_fonts(
        &mut self,
        context: &egui::Context,
        prepared: &PreparedDocumentFonts,
    ) -> Result<bool, FontPreparationError> {
        // Preflight the full registration set so a mismatch cannot install only
        // a prefix of the document's resolved faces into the host context.
        let mut additional_bytes = 0usize;
        for (id, bytes) in &prepared.face_bytes {
            if let Some(existing) = self.registered_bytes.get(id) {
                if existing.as_ref() != bytes.as_ref() {
                    return Err(FontPreparationError::bounded("resolved-id-byte-collision"));
                }
            } else {
                additional_bytes = additional_bytes.saturating_add(bytes.len());
            }
            if !prepared.egui_family_names.contains_key(id) {
                return Err(FontPreparationError::bounded("registered-family-missing"));
            }
        }
        if self.registered_byte_count.saturating_add(additional_bytes) > MAX_REGISTERED_FONT_BYTES {
            return Err(FontPreparationError::bounded("registered-font-byte-limit"));
        }
        for (id, bytes) in &prepared.face_bytes {
            if self.registered_bytes.contains_key(id) {
                continue;
            }
            let family_name = prepared
                .egui_family_names
                .get(id)
                .ok_or_else(|| FontPreparationError::bounded("registered-family-missing"))?;
            let data_key = format!("scriptcraft-resolved-{}", id.as_str());
            context.add_font(FontInsert::new(
                data_key.as_str(),
                FontData::from_owned(bytes.as_ref().to_vec()),
                vec![InsertFontFamily {
                    family: FontFamily::Name(family_name.clone().into()),
                    priority: FontPriority::Highest,
                }],
            ));
            self.registered_bytes.insert(id.clone(), bytes.clone());
            self.registered_byte_count = self.registered_byte_count.saturating_add(bytes.len());
        }

        let ready = context.fonts(|fonts| {
            let definitions = fonts.definitions();
            prepared.face_bytes.keys().all(|id| {
                let Some(family_name) = prepared.egui_family_names.get(id) else {
                    return false;
                };
                let data_key = format!("scriptcraft-resolved-{}", id.as_str());
                let family = FontFamily::Name(family_name.clone().into());
                definitions.font_data.contains_key(&data_key)
                    && definitions
                        .families
                        .get(&family)
                        .is_some_and(|keys| keys.iter().any(|key| key == &data_key))
            })
        });
        Ok(ready)
    }

    pub(crate) fn invalidate_catalog(&mut self) {
        self.catalog_generation = self.catalog_generation.saturating_add(1);
        self.host_catalog = HostFontCatalog::discover();
        self.prepared_cache.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FamilyKind {
    CourierAlias,
    Sans,
    Serif,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FamilySelection {
    Courier,
    Sans,
    Serif,
}

fn classify_requested_family(family: &str) -> (FamilyKind, FamilySelection) {
    let normalized = family.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "courier final draft" | "courier" | "courier new" | "courier prime" | "monospace"
    ) {
        (FamilyKind::CourierAlias, FamilySelection::Courier)
    } else if matches!(
        normalized.as_str(),
        "arial"
            | "helvetica"
            | "helvetica neue"
            | "helvetica-like"
            | "liberation sans"
            | "sans-serif"
    ) {
        (FamilyKind::Sans, FamilySelection::Sans)
    } else if matches!(
        normalized.as_str(),
        "times new roman" | "times" | "liberation serif" | "serif"
    ) {
        (FamilyKind::Serif, FamilySelection::Serif)
    } else {
        (FamilyKind::Unknown, FamilySelection::Courier)
    }
}

fn project_font_signature(
    project: &Project,
    generation: u64,
) -> Result<PreparationSignature, FontPreparationError> {
    let mut families = BTreeSet::from(["Courier Prime".to_owned()]);
    for definition in project.element_definitions().iter() {
        insert_bounded_family(&mut families, definition.font.family.clone())?;
    }
    let mut scalars = BTreeSet::new();
    for owner in &project.elements {
        let element = owner.as_element();
        let definition = project.element_definitions().get(&element.element_type);
        if let Some(definition) = definition {
            collect_text_scalars(&mut scalars, &element.text, definition.font.all_caps)?;
        } else {
            collect_text_scalars(&mut scalars, &element.text, false)?;
        }
        collect_run_inputs(&mut families, &mut scalars, &element.runs)?;
        for run in &element.runs {
            if run_has_all_caps(run) {
                collect_uppercase_scalars(&mut scalars, &run.text)?;
            }
        }
    }
    for paragraph in &project.title_page {
        let all_caps = paragraph
            .element_type
            .as_ref()
            .and_then(|key| project.element_definitions().get(key))
            .is_some_and(|definition| definition.font.all_caps);
        collect_text_scalars(&mut scalars, &paragraph.text, all_caps)?;
        collect_run_inputs(&mut families, &mut scalars, &paragraph.runs)?;
        for run in &paragraph.runs {
            if run_has_all_caps(run) {
                collect_uppercase_scalars(&mut scalars, &run.text)?;
            }
        }
    }
    scalars.insert('\u{fffd}');
    if scalars.len() > MAX_DOCUMENT_FONT_SCALARS {
        return Err(FontPreparationError::bounded("scalar-repertoire-limit"));
    }
    if families.len() > MAX_DOCUMENT_FONT_REQUESTS {
        return Err(FontPreparationError::bounded("font-request-limit"));
    }
    Ok(PreparationSignature {
        generation,
        families,
        scalars,
    })
}

fn insert_bounded_family(
    families: &mut BTreeSet<String>,
    family: String,
) -> Result<(), FontPreparationError> {
    if family.is_empty() || family.len() > 256 || family.chars().any(char::is_control) {
        return Err(FontPreparationError::bounded("invalid-family-request"));
    }
    families.insert(family);
    if families.len() > MAX_DOCUMENT_FONT_REQUESTS {
        return Err(FontPreparationError::bounded("font-request-limit"));
    }
    Ok(())
}

fn collect_run_inputs(
    families: &mut BTreeSet<String>,
    scalars: &mut BTreeSet<char>,
    runs: &[TextRun],
) -> Result<(), FontPreparationError> {
    for run in runs {
        for attribute in &run.attributes {
            if attribute.name == "Font" && !attribute.value.is_empty() {
                insert_bounded_family(families, attribute.value.clone())?;
            }
        }
        collect_text_scalars(scalars, &run.text, run_has_all_caps(run))?;
    }
    Ok(())
}

fn run_has_all_caps(run: &TextRun) -> bool {
    run.attributes.iter().any(|attribute| {
        attribute.name == "Style" && attribute.value.split('+').any(|token| token == "AllCaps")
    })
}

fn collect_text_scalars(
    scalars: &mut BTreeSet<char>,
    text: &str,
    all_caps: bool,
) -> Result<(), FontPreparationError> {
    scalars.extend(text.chars());
    if all_caps {
        collect_uppercase_scalars(scalars, text)?;
    }
    if scalars.len() > MAX_DOCUMENT_FONT_SCALARS {
        return Err(FontPreparationError::bounded("scalar-repertoire-limit"));
    }
    Ok(())
}

fn collect_uppercase_scalars(
    scalars: &mut BTreeSet<char>,
    text: &str,
) -> Result<(), FontPreparationError> {
    for scalar in text.chars().flat_map(char::to_uppercase) {
        scalars.insert(scalar);
        if scalars.len() > MAX_DOCUMENT_FONT_SCALARS {
            return Err(FontPreparationError::bounded("scalar-repertoire-limit"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedHostFace {
    bytes: Vec<u8>,
    metrics: ResolvedFaceMetrics,
    family_label: String,
    face_label: String,
}

fn resolve_host_style_with(
    candidates: Vec<FontCandidate>,
    scalars: &BTreeSet<char>,
    bytes_read: &mut usize,
    mut read: impl FnMut(&Path) -> io::Result<Vec<u8>>,
) -> Result<Option<ParsedHostFace>, FontPreparationError> {
    if candidates.len() > MAX_HOST_CANDIDATES {
        return Err(FontPreparationError::bounded("host-candidate-limit"));
    }
    for candidate in candidates {
        let Some(path) = candidate.path.as_deref() else {
            continue;
        };
        let bytes = match read(path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        *bytes_read = bytes_read.saturating_add(bytes.len());
        if *bytes_read > MAX_PREPARATION_BYTES
            || bytes.is_empty()
            || bytes.len() > MAX_ARIAL_FACE_BYTES
        {
            if *bytes_read > MAX_PREPARATION_BYTES {
                return Err(FontPreparationError::bounded("font-byte-budget"));
            }
            continue;
        }
        if let Ok(metrics) = metrics_for_bytes(&bytes, scalars) {
            return Ok(Some(ParsedHostFace {
                bytes,
                metrics,
                family_label: candidate.family_label,
                face_label: candidate.face_label,
            }));
        }
    }
    Ok(None)
}

fn metrics_for_bytes(
    bytes: &[u8],
    scalars: &BTreeSet<char>,
) -> Result<ResolvedFaceMetrics, FontPreparationError> {
    // Collections have a face index and are deliberately rejected rather than
    // silently measuring whichever member a parser happens to choose.
    if bytes.starts_with(b"ttcf") || bytes.starts_with(b"wOFF") || bytes.starts_with(b"wOF2") {
        return Err(FontPreparationError::bounded("unsupported-font-container"));
    }
    let font = FontRef::try_from_slice(bytes)
        .map_err(|_| FontPreparationError::bounded("malformed-font"))?;
    let upem = font
        .units_per_em()
        .ok_or_else(|| FontPreparationError::bounded("invalid-font-units"))?;
    if !upem.is_finite() || upem < 1.0 || upem > 65_536.0 {
        return Err(FontPreparationError::bounded("invalid-font-units"));
    }
    let ascender = finite_metric(font.ascent_unscaled())?;
    let descender = finite_metric(font.descent_unscaled())?;
    let line_gap = finite_metric(font.line_gap_unscaled())?;
    let mut advances = BTreeMap::new();
    for scalar in scalars {
        let glyph = font.glyph_id(*scalar);
        let advance = finite_advance(font.h_advance_unscaled(glyph))?;
        advances.insert(
            *scalar,
            ScalarAdvance {
                advance_units: advance,
                missing_glyph: glyph.0 == 0,
            },
        );
    }
    ResolvedFaceMetrics::new(
        upem.round() as u32,
        ascender,
        descender,
        line_gap.max(0),
        advances,
    )
    .map_err(|_| FontPreparationError::bounded("invalid-font-metrics"))
}

fn finite_metric(value: f32) -> Result<i32, FontPreparationError> {
    if !value.is_finite() || value.abs() > 1_000_000.0 {
        return Err(FontPreparationError::bounded("invalid-font-metrics"));
    }
    Ok(value.round() as i32)
}

fn finite_advance(value: f32) -> Result<u32, FontPreparationError> {
    if !value.is_finite() || value < 0.0 || value > 1_000_000.0 {
        return Err(FontPreparationError::bounded("invalid-glyph-advance"));
    }
    Ok(value.round() as u32)
}

fn insert_resolved_face(
    faces: &mut BTreeMap<ResolvedFontId, ResolvedFontFace>,
    bytes_by_id: &mut BTreeMap<ResolvedFontId, Arc<[u8]>>,
    family_names: &mut BTreeMap<ResolvedFontId, String>,
    bytes: &[u8],
    family_label: &str,
    face_label: &str,
    style: ResolvedFaceStyle,
    source_kind: ResolvedFontSourceKind,
    metrics: ResolvedFaceMetrics,
) -> Result<ResolvedFontId, FontPreparationError> {
    let fingerprint = content_fingerprint(bytes);
    let id = allocate_resolved_font_id(bytes, style, fingerprint, bytes_by_id)?;
    if !faces.contains_key(&id) {
        let face = ResolvedFontFace::from_bytes(
            id.clone(),
            family_label,
            face_label,
            style,
            source_kind,
            bytes,
            metrics,
        )
        .map_err(|_| FontPreparationError::bounded("resolved-face-invalid"))?;
        faces.insert(id.clone(), face);
        bytes_by_id.insert(id.clone(), Arc::from(bytes));
        let egui_name = format!("ScriptCraft Resolved {}", id.as_str());
        family_names.insert(id.clone(), egui_name);
    }
    Ok(id)
}

fn allocate_resolved_font_id(
    bytes: &[u8],
    style: ResolvedFaceStyle,
    fingerprint: u64,
    existing: &BTreeMap<ResolvedFontId, Arc<[u8]>>,
) -> Result<ResolvedFontId, FontPreparationError> {
    let style_suffix = match style {
        ResolvedFaceStyle::Regular => "r",
        ResolvedFaceStyle::Bold => "b",
        ResolvedFaceStyle::Italic => "i",
        ResolvedFaceStyle::BoldItalic => "bi",
    };
    for collision_index in 0..=16u32 {
        let spelling = if collision_index == 0 {
            format!("font-{fingerprint:016x}-{style_suffix}")
        } else {
            format!("font-{fingerprint:016x}-{style_suffix}-c{collision_index}")
        };
        let id = ResolvedFontId::new(spelling)
            .map_err(|_| FontPreparationError::bounded("resolved-id-bound"))?;
        match existing.get(&id) {
            Some(existing_bytes) if existing_bytes.as_ref() == bytes => return Ok(id),
            Some(_) => continue,
            None => return Ok(id),
        }
    }
    Err(FontPreparationError::bounded("resolved-id-collision-limit"))
}

fn resolved_style_index(style: ResolvedFaceStyle) -> usize {
    match style {
        ResolvedFaceStyle::Regular => 0,
        ResolvedFaceStyle::Bold => 1,
        ResolvedFaceStyle::Italic => 2,
        ResolvedFaceStyle::BoldItalic => 3,
    }
}

fn courier_style_name(style: ResolvedFaceStyle) -> &'static str {
    match style {
        ResolvedFaceStyle::Regular => "Courier Prime Regular",
        ResolvedFaceStyle::Bold => "Courier Prime Bold",
        ResolvedFaceStyle::Italic => "Courier Prime Italic",
        ResolvedFaceStyle::BoldItalic => "Courier Prime Bold Italic",
    }
}

fn bundled_courier_prime_faces() -> [&'static [u8]; 4] {
    [
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime.ttf"),
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Bold.ttf"),
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Italic.ttf"),
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Bold Italic.ttf"),
    ]
}

#[derive(Debug, Clone)]
struct FontCandidate {
    path: Option<PathBuf>,
    family_label: String,
    face_label: String,
}

#[derive(Debug)]
struct HostFontCatalog {
    sans: [Vec<FontCandidate>; 4],
    serif: [Vec<FontCandidate>; 4],
}

impl HostFontCatalog {
    fn discover() -> Self {
        Self {
            sans: std::array::from_fn(|index| {
                font_candidates_for_platform(
                    current_candidate_platform(),
                    current_windows_directory().as_deref(),
                    FamilySelection::Sans,
                    ResolvedFaceStyle::ALL[index],
                )
            }),
            serif: std::array::from_fn(|index| {
                font_candidates_for_platform(
                    current_candidate_platform(),
                    current_windows_directory().as_deref(),
                    FamilySelection::Serif,
                    ResolvedFaceStyle::ALL[index],
                )
            }),
        }
    }

    fn candidates(&self, selection: FamilySelection, style: ResolvedFaceStyle) -> &[FontCandidate] {
        let index = resolved_style_index(style);
        match selection {
            FamilySelection::Sans => &self.sans[index],
            FamilySelection::Serif => &self.serif[index],
            FamilySelection::Courier => &[],
        }
    }
}

fn current_candidate_platform() -> CandidatePlatform {
    #[cfg(target_os = "windows")]
    {
        CandidatePlatform::Windows
    }
    #[cfg(target_os = "macos")]
    {
        CandidatePlatform::MacOs
    }
    #[cfg(target_os = "linux")]
    {
        CandidatePlatform::Linux
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        CandidatePlatform::Unsupported
    }
}

fn current_windows_directory() -> Option<PathBuf> {
    std::env::var_os("WINDIR")
        .or_else(|| std::env::var_os("SystemRoot"))
        .map(PathBuf::from)
}

fn host_font_candidates(
    selection: FamilySelection,
    style: ResolvedFaceStyle,
) -> Vec<FontCandidate> {
    font_candidates_for_platform(
        current_candidate_platform(),
        current_windows_directory().as_deref(),
        selection,
        style,
    )
}

fn font_candidates_for_platform(
    platform: CandidatePlatform,
    windows_directory: Option<&Path>,
    selection: FamilySelection,
    style: ResolvedFaceStyle,
) -> Vec<FontCandidate> {
    let file_names: &[(&str, &str)] = match (selection, style) {
        (FamilySelection::Sans, ResolvedFaceStyle::Regular) => &[
            ("Arial", "arial.ttf"),
            ("Helvetica", "Helvetica.ttf"),
            ("Liberation Sans", "LiberationSans-Regular.ttf"),
            ("Segoe UI", "segoeui.ttf"),
        ],
        (FamilySelection::Sans, ResolvedFaceStyle::Bold) => &[
            ("Arial", "arialbd.ttf"),
            ("Helvetica", "Helvetica Bold.ttf"),
            ("Liberation Sans", "LiberationSans-Bold.ttf"),
            ("Segoe UI", "segoeuib.ttf"),
        ],
        (FamilySelection::Sans, ResolvedFaceStyle::Italic) => &[
            ("Arial", "ariali.ttf"),
            ("Helvetica", "Helvetica Oblique.ttf"),
            ("Liberation Sans", "LiberationSans-Italic.ttf"),
            ("Segoe UI", "segoeuii.ttf"),
        ],
        (FamilySelection::Sans, ResolvedFaceStyle::BoldItalic) => &[
            ("Arial", "arialbi.ttf"),
            ("Helvetica", "Helvetica Bold Oblique.ttf"),
            ("Liberation Sans", "LiberationSans-BoldItalic.ttf"),
            ("Segoe UI", "segoeuiz.ttf"),
        ],
        (FamilySelection::Serif, ResolvedFaceStyle::Regular) => &[
            ("Times New Roman", "times.ttf"),
            ("Times", "Times.ttf"),
            ("Liberation Serif", "LiberationSerif-Regular.ttf"),
            ("Georgia", "georgia.ttf"),
        ],
        (FamilySelection::Serif, ResolvedFaceStyle::Bold) => &[
            ("Times New Roman", "timesbd.ttf"),
            ("Times", "Times Bold.ttf"),
            ("Liberation Serif", "LiberationSerif-Bold.ttf"),
            ("Georgia", "georgiab.ttf"),
        ],
        (FamilySelection::Serif, ResolvedFaceStyle::Italic) => &[
            ("Times New Roman", "timesi.ttf"),
            ("Times", "Times Italic.ttf"),
            ("Liberation Serif", "LiberationSerif-Italic.ttf"),
            ("Georgia", "georgiai.ttf"),
        ],
        (FamilySelection::Serif, ResolvedFaceStyle::BoldItalic) => &[
            ("Times New Roman", "timesbi.ttf"),
            ("Times", "Times Bold Italic.ttf"),
            ("Liberation Serif", "LiberationSerif-BoldItalic.ttf"),
            ("Georgia", "georgiaz.ttf"),
        ],
        (FamilySelection::Courier, _) => &[],
    };
    let mut candidates = Vec::new();
    for (family_label, file_name) in file_names {
        let paths = candidate_paths(platform, windows_directory, family_label, file_name);
        if paths.is_empty() {
            candidates.push(FontCandidate {
                path: None,
                family_label: (*family_label).to_owned(),
                face_label: format!("{family_label} {style:?}"),
            });
        } else {
            candidates.extend(paths.into_iter().map(|path| FontCandidate {
                path: Some(path),
                family_label: (*family_label).to_owned(),
                face_label: format!("{family_label} {style:?}"),
            }));
        }
    }
    candidates.truncate(MAX_HOST_CANDIDATES);
    candidates
}

fn candidate_paths(
    platform: CandidatePlatform,
    windows_directory: Option<&Path>,
    family: &str,
    file_name: &str,
) -> Vec<PathBuf> {
    match platform {
        CandidatePlatform::Windows => windows_directory
            .map(|root| vec![root.join("Fonts").join(file_name)])
            .unwrap_or_default(),
        CandidatePlatform::MacOs => {
            let directory =
                if family == "Arial" || family == "Times New Roman" || family == "Georgia" {
                    "/System/Library/Fonts/Supplemental"
                } else {
                    "/Library/Fonts"
                };
            let mac_name = match file_name {
                "arial.ttf" => "Arial.ttf",
                "arialbd.ttf" => "Arial Bold.ttf",
                "ariali.ttf" => "Arial Italic.ttf",
                "arialbi.ttf" => "Arial Bold Italic.ttf",
                "times.ttf" => "Times New Roman.ttf",
                "timesbd.ttf" => "Times New Roman Bold.ttf",
                "timesi.ttf" => "Times New Roman Italic.ttf",
                "timesbi.ttf" => "Times New Roman Bold Italic.ttf",
                "georgia.ttf" => "Georgia.ttf",
                "georgiab.ttf" => "Georgia Bold.ttf",
                "georgiai.ttf" => "Georgia Italic.ttf",
                "georgiaz.ttf" => "Georgia Bold Italic.ttf",
                other => other,
            };
            vec![PathBuf::from(directory).join(mac_name)]
        }
        CandidatePlatform::Linux => {
            let mut paths = Vec::new();
            let linux_name = match (family, file_name) {
                ("Arial", "arial.ttf") => "Arial.ttf",
                ("Arial", "arialbd.ttf") => "Arial_Bold.ttf",
                ("Arial", "ariali.ttf") => "Arial_Italic.ttf",
                ("Arial", "arialbi.ttf") => "Arial_Bold_Italic.ttf",
                ("Times New Roman", "times.ttf") => "Times_New_Roman.ttf",
                ("Times New Roman", "timesbd.ttf") => "Times_New_Roman_Bold.ttf",
                ("Times New Roman", "timesi.ttf") => "Times_New_Roman_Italic.ttf",
                ("Times New Roman", "timesbi.ttf") => "Times_New_Roman_Bold_Italic.ttf",
                ("Georgia", "georgia.ttf") => "Georgia.ttf",
                ("Georgia", "georgiab.ttf") => "Georgia_Bold.ttf",
                ("Georgia", "georgiai.ttf") => "Georgia_Italic.ttf",
                ("Georgia", "georgiaz.ttf") => "Georgia_Bold_Italic.ttf",
                (_, other) => other,
            };
            let roots = [
                "/usr/share/fonts/truetype/msttcorefonts",
                "/usr/share/fonts/truetype/liberation",
                "/usr/share/fonts/truetype/liberation2",
                "/usr/local/share/fonts",
            ];
            for root in roots {
                paths.push(PathBuf::from(root).join(linux_name));
            }
            paths
        }
        CandidatePlatform::Unsupported => Vec::new(),
    }
}

#[cfg(test)]
mod resolved_font_tests {
    use super::*;
    use eframe::egui::{Color32, FontId};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_FONT_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

    struct TestFontDirectory(PathBuf);

    impl TestFontDirectory {
        fn new() -> Self {
            let id = TEMP_FONT_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "scriptcraft-font-readiness-{}-{id}",
                std::process::id()
            ));
            assert!(fs::create_dir_all(&path).is_ok());
            Self(path)
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            assert!(fs::write(&path, bytes).is_ok());
            path
        }
    }

    impl Drop for TestFontDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn project_with_family(family: &str) -> Option<Project> {
        let Ok(mut project) = Project::new() else {
            return None;
        };
        let mut definitions = project
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let Some(definition) = definitions.first_mut() else {
            return None;
        };
        definition.font.family = family.to_owned();
        definition.font.bold = true;
        definition.font.italic = true;
        if project.replace_element_definitions(definitions).is_err() {
            return None;
        }
        Some(project)
    }

    fn assert_prepared_faces_paintable(context: &egui::Context, prepared: &PreparedDocumentFonts) {
        let font_ids = context.fonts(|fonts| {
            let definitions = fonts.definitions();
            let mut font_ids = Vec::with_capacity(prepared.face_bytes.len());
            for id in prepared.face_bytes.keys() {
                let Some(name) = prepared.egui_family_names.get(id) else {
                    assert!(false, "every resolved face must have an egui family name");
                    continue;
                };
                let data_key = format!("scriptcraft-resolved-{}", id.as_str());
                let family = FontFamily::Name(name.clone().into());
                assert!(definitions.font_data.contains_key(&data_key));
                assert!(definitions
                    .families
                    .get(&family)
                    .is_some_and(|keys| keys.iter().any(|key| key == &data_key)));
                font_ids.push(FontId::new(12.0, family));
            }
            font_ids
        });
        context.fonts_mut(|fonts| {
            for font_id in font_ids {
                let _galley =
                    fonts.layout_no_wrap("activation pass".to_owned(), font_id, Color32::WHITE);
            }
        });
    }

    fn empty_host_catalog() -> HostFontCatalog {
        HostFontCatalog {
            sans: std::array::from_fn(|_| Vec::new()),
            serif: std::array::from_fn(|_| Vec::new()),
        }
    }

    #[test]
    fn family_aliases_and_unknown_fallbacks_are_classified_without_host_lookup() {
        for alias in [
            "Courier Final Draft",
            "Courier",
            "Courier New",
            "Courier Prime",
            "monospace",
        ] {
            assert_eq!(classify_requested_family(alias).0, FamilyKind::CourierAlias);
        }
        for sans in [
            "Arial",
            "Helvetica",
            "Helvetica Neue",
            "Liberation Sans",
            "sans-serif",
        ] {
            assert_eq!(classify_requested_family(sans).0, FamilyKind::Sans);
        }
        for serif in ["Times New Roman", "Times", "Liberation Serif", "serif"] {
            assert_eq!(classify_requested_family(serif).0, FamilyKind::Serif);
        }
        assert_eq!(
            classify_requested_family("Unknown Example").0,
            FamilyKind::Unknown
        );
        assert_eq!(
            classify_requested_family("Unknown Example").1,
            FamilySelection::Courier
        );
    }

    #[test]
    fn candidate_order_is_family_then_style_specific_and_unknown_is_courier() {
        let candidates = font_candidates_for_platform(
            CandidatePlatform::Linux,
            None,
            FamilySelection::Sans,
            ResolvedFaceStyle::BoldItalic,
        );
        assert!(candidates
            .first()
            .is_some_and(|candidate| candidate.family_label == "Arial"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.family_label == "Liberation Sans"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.face_label.contains("BoldItalic")));
        assert!(font_candidates_for_platform(
            CandidatePlatform::Linux,
            None,
            FamilySelection::Courier,
            ResolvedFaceStyle::Regular,
        )
        .is_empty());
        let serif = font_candidates_for_platform(
            CandidatePlatform::Linux,
            None,
            FamilySelection::Serif,
            ResolvedFaceStyle::Regular,
        );
        assert_eq!(
            serif
                .first()
                .map(|candidate| candidate.family_label.as_str()),
            Some("Times New Roman")
        );
        assert!(serif
            .iter()
            .any(|candidate| candidate.family_label == "Liberation Serif"));
        assert_eq!(
            serif
                .last()
                .map(|candidate| candidate.family_label.as_str()),
            Some("Georgia")
        );
    }

    #[test]
    fn bundled_font_parser_produces_bounded_four_style_metrics() {
        let scalars = BTreeSet::from(['A', 'W', 'i', '�']);
        let mut ids = BTreeSet::new();
        for (index, (style, bytes)) in ResolvedFaceStyle::ALL
            .into_iter()
            .zip(bundled_courier_prime_faces())
            .enumerate()
        {
            let metrics = metrics_for_bytes(bytes, &scalars);
            assert!(metrics.is_ok());
            if let Ok(metrics) = metrics {
                assert_eq!(metrics.scalar_advances.len(), 4);
                assert!(metrics.units_per_em > 0);
                assert!(metrics.scalar_advances.contains_key(&'�'));
            }
            let id = allocate_resolved_font_id(
                bytes,
                style,
                content_fingerprint(bytes),
                &BTreeMap::new(),
            );
            assert!(id.is_ok());
            if let Ok(id) = id {
                assert_eq!(resolved_style_index(style), index);
                ids.insert(id);
            }
        }
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn injected_candidates_skip_missing_malformed_and_oversized_faces() {
        let candidates = ["missing", "malformed", "oversized", "valid"]
            .into_iter()
            .map(|name| FontCandidate {
                path: Some(PathBuf::from(name)),
                family_label: name.to_owned(),
                face_label: format!("{name} Regular"),
            })
            .collect();
        let mut bytes_read = 0;
        let repertoire = BTreeSet::from(['?', 'A']);
        let bundled = bundled_courier_prime_faces()[0];
        let result =
            resolve_host_style_with(candidates, &repertoire, &mut bytes_read, |path| match path
                .to_string_lossy()
                .as_ref()
            {
                "missing" => Err(io::Error::new(io::ErrorKind::NotFound, "injected missing")),
                "malformed" => Ok(vec![0, 1, 2, 3]),
                "oversized" => Ok(vec![0; MAX_ARIAL_FACE_BYTES + 1]),
                "valid" => Ok(bundled.to_vec()),
                _ => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "unknown injected candidate",
                )),
            });
        assert!(result.is_ok());
        assert_eq!(
            result.ok().flatten().map(|face| face.family_label),
            Some("valid".to_owned())
        );
    }

    #[test]
    fn empty_default_project_waits_for_activation_pass_before_galley_paint() {
        let Ok(project) = Project::new() else {
            assert!(false, "default Project should be constructible");
            return;
        };
        let mut store = DocumentFontStore::default();
        store.host_catalog = empty_host_catalog();
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(false, "default Project font preparation should succeed");
            return;
        };
        let context = egui::Context::default();

        let _ = context.run(egui::RawInput::default(), |ctx| {
            assert_eq!(store.register_prepared_fonts(ctx, &prepared), Ok(false));
        });
        let _ = context.run(egui::RawInput::default(), |ctx| {
            assert_eq!(store.register_prepared_fonts(ctx, &prepared), Ok(true));
            assert_prepared_faces_paintable(ctx, &prepared);
        });

        let Some(courier) = prepared.environment.resolution_for("Courier Prime") else {
            assert!(
                false,
                "default project must contain the neutral Courier request"
            );
            return;
        };
        assert_ne!(courier.regular, courier.bold);
        assert_ne!(courier.regular, courier.bold_italic);
    }

    #[test]
    fn prepared_replacement_environments_wait_until_all_new_face_bindings_activate() {
        let Ok(default_project) = Project::new() else {
            assert!(false, "default Project should be constructible");
            return;
        };
        let Some(replacement_project) = project_with_family("Arial") else {
            assert!(false, "test replacement project should be constructible");
            return;
        };
        let directory = TestFontDirectory::new();
        let bundled = bundled_courier_prime_faces();
        let rotated = [bundled[1], bundled[2], bundled[3], bundled[0]];
        let paths: [PathBuf; 4] = std::array::from_fn(|index| {
            directory.write(&format!("host-face-{index}.ttf"), rotated[index])
        });
        let mut store = DocumentFontStore::default();
        store.host_catalog = HostFontCatalog {
            sans: std::array::from_fn(|index| {
                vec![FontCandidate {
                    path: Some(paths[index].clone()),
                    family_label: "Injected Sans".to_owned(),
                    face_label: format!("Injected Sans {:?}", ResolvedFaceStyle::ALL[index]),
                }]
            }),
            serif: std::array::from_fn(|_| Vec::new()),
        };
        let Ok(initial) = store.prepare_document(&default_project) else {
            assert!(false, "initial default font environment should prepare");
            return;
        };
        let Ok(replacement) = store.prepare_document(&replacement_project) else {
            assert!(false, "replacement font environment should prepare");
            return;
        };
        let Some(arial) = replacement.environment.resolution_for("Arial") else {
            assert!(
                false,
                "replacement should retain its declared Arial request"
            );
            return;
        };
        assert_eq!(arial.face_for_flags(true, true), &arial.bold_italic);
        assert_ne!(arial.regular, initial.environment.neutral_request().regular);
        assert_ne!(
            arial.bold_italic,
            initial.environment.neutral_request().bold_italic
        );
        assert!(replacement
            .egui_family_names
            .contains_key(arial.face_for_flags(true, true)));

        let context = egui::Context::default();
        let _ = context.run(egui::RawInput::default(), |ctx| {
            assert_eq!(store.register_prepared_fonts(ctx, &initial), Ok(false));
            // A second surface/environment prepared before activation cannot
            // infer readiness from the first environment's queued bytes.
            assert_eq!(store.register_prepared_fonts(ctx, &replacement), Ok(false));
            assert_eq!(store.register_prepared_fonts(ctx, &initial), Ok(false));
        });
        let _ = context.run(egui::RawInput::default(), |ctx| {
            assert_eq!(store.register_prepared_fonts(ctx, &initial), Ok(true));
            assert_eq!(store.register_prepared_fonts(ctx, &replacement), Ok(true));
            assert_prepared_faces_paintable(ctx, &initial);
            assert_prepared_faces_paintable(ctx, &replacement);
        });
    }

    #[test]
    fn missing_and_malformed_host_faces_resolve_to_bundled_courier_prime() {
        let Some(project) = project_with_family("Arial") else {
            assert!(false, "Arial test project should be constructible");
            return;
        };
        let directory = TestFontDirectory::new();
        let malformed_path = directory.write("malformed.ttf", &[0, 1, 2, 3]);
        let mut store = DocumentFontStore::default();
        store.host_catalog = HostFontCatalog {
            sans: std::array::from_fn(|index| {
                let path = if index % 2 == 0 {
                    Some(directory.0.join(format!("absent-{index}.ttf")))
                } else {
                    Some(malformed_path.clone())
                };
                vec![FontCandidate {
                    path,
                    family_label: "Injected Invalid Sans".to_owned(),
                    face_label: format!(
                        "Injected Invalid Sans {:?}",
                        ResolvedFaceStyle::ALL[index]
                    ),
                }]
            }),
            serif: std::array::from_fn(|_| Vec::new()),
        };
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(
                false,
                "invalid host faces should fall back instead of failing preparation"
            );
            return;
        };
        let Some(arial) = prepared.environment.resolution_for("Arial") else {
            assert!(
                false,
                "prepared environment should preserve the Arial request"
            );
            return;
        };
        let Some(courier) = prepared.environment.resolution_for("Courier Prime") else {
            assert!(
                false,
                "prepared environment should retain bundled Courier Prime"
            );
            return;
        };
        for style in ResolvedFaceStyle::ALL {
            assert!(prepared
                .fallback_faces
                .contains(&("Arial".to_owned(), style)));
            assert_eq!(arial.face_for_style(style), courier.face_for_style(style));
            let Some(face) = prepared.environment.face(arial.face_for_style(style)) else {
                assert!(false, "fallback id must resolve to an actual face");
                return;
            };
            assert_eq!(face.source_kind, ResolvedFontSourceKind::Bundled);
        }
        assert!(prepared
            .face_bytes
            .values()
            .all(|bytes| bundled_courier_prime_faces()
                .iter()
                .any(|face| *face == bytes.as_ref())));
    }

    #[test]
    fn uppercase_expansion_bounds_and_resolved_id_collision_suffix_are_deterministic() {
        let mut scalars = BTreeSet::new();
        assert!(collect_uppercase_scalars(&mut scalars, "ß").is_ok());
        assert!(scalars.contains(&'S'));

        let mut too_many = BTreeSet::new();
        for value in 0..=MAX_DOCUMENT_FONT_SCALARS {
            if let Some(scalar) = char::from_u32(value as u32) {
                too_many.insert(scalar);
            }
        }
        assert_eq!(
            collect_uppercase_scalars(&mut too_many, "x"),
            Err(FontPreparationError::bounded("scalar-repertoire-limit"))
        );

        let mut existing = BTreeMap::new();
        let base = allocate_resolved_font_id(b"first", ResolvedFaceStyle::Regular, 7, &existing);
        assert!(base.is_ok());
        if let Ok(base) = base {
            existing.insert(base.clone(), Arc::from(&b"first"[..]));
            let same =
                allocate_resolved_font_id(b"first", ResolvedFaceStyle::Regular, 7, &existing);
            let collision =
                allocate_resolved_font_id(b"second", ResolvedFaceStyle::Regular, 7, &existing);
            assert_eq!(same, Ok(base.clone()));
            assert!(collision.is_ok_and(|id| id != base && id.as_str().ends_with("-c1")));
        }
    }
}
