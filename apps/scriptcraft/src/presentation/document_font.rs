use std::sync::Arc;

use super::style::{AdvanceMode, RendererFamilyChoice};
use eframe::egui::{
    self,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    FontData, FontDefinitions, FontFamily,
};

pub(crate) const COURIER_PRIME_FAMILY: &str = "Courier Prime";
const REGULAR_KEY: &str = "scriptcraft-courier-prime-regular";
const BOLD_KEY: &str = "scriptcraft-courier-prime-bold";
const ITALIC_KEY: &str = "scriptcraft-courier-prime-italic";
const BOLD_ITALIC_KEY: &str = "scriptcraft-courier-prime-bold-italic";

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
    build_font_definitions([None, None, None, None])
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
    face_indices: BTreeMap<ResolvedFontId, u32>,
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
    native_font_catalog: super::host_font_catalog::NativeFontCatalog,
    prepared_cache: HashMap<PreparationSignature, PreparedDocumentFonts>,
    registered_bytes: BTreeMap<ResolvedFontId, Arc<[u8]>>,
    registered_face_indices: BTreeMap<ResolvedFontId, u32>,
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
            native_font_catalog: super::host_font_catalog::NativeFontCatalog::default(),
            prepared_cache: HashMap::new(),
            registered_bytes: BTreeMap::new(),
            registered_face_indices: BTreeMap::new(),
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
        let generation = self
            .catalog_generation
            .saturating_add(self.native_font_catalog.generation());
        let signature = project_font_signature(project, generation)?;
        if let Some(cached) = self.prepared_cache.get(&signature) {
            return Ok(cached.clone());
        }

        let mut faces = BTreeMap::new();
        let mut face_bytes = BTreeMap::new();
        let mut face_indices = BTreeMap::new();
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
                &mut face_indices,
                &mut family_names,
                bytes,
                "Courier Prime",
                courier_style_name(style),
                style,
                ResolvedFontSourceKind::Bundled,
                0,
                metrics,
            )?;
            bundled_ids[index] = Some(id);
        }

        let mut resolutions = BTreeMap::new();
        let mut bytes_read = 0usize;
        for family in &signature.families {
            let (kind, selection) = classify_requested_family(family);
            let mut selected: [Option<ResolvedFontId>; 4] = std::array::from_fn(|_| None);
            if kind == FamilyKind::CourierAlias {
                for style in ResolvedFaceStyle::ALL {
                    selected[resolved_style_index(style)] =
                        bundled_ids[resolved_style_index(style)].clone();
                }
            } else {
                // Resolve each exact family/style independently before generic
                // matching-family faces. Missing styles are filled only after
                // all available faces are known, using the documented family
                // priority (Bold Italic, Bold, Italic, Regular).
                for style in ResolvedFaceStyle::ALL {
                    let index = resolved_style_index(style);
                    let exact = resolve_native_style(
                        &mut self.native_font_catalog,
                        family,
                        style,
                        &signature.scalars,
                        &mut bytes_read,
                    )?;
                    let (candidate, used_generic) = match exact {
                        Some(face) => (Some(face), false),
                        None => match selection {
                            FamilySelection::Sans | FamilySelection::Serif => {
                                let mut fallback = None;
                                for generic_family in generic_family_candidates(selection) {
                                    if generic_family.eq_ignore_ascii_case(family) {
                                        continue;
                                    }
                                    if let Some(face) = resolve_native_style(
                                        &mut self.native_font_catalog,
                                        generic_family,
                                        style,
                                        &signature.scalars,
                                        &mut bytes_read,
                                    )? {
                                        fallback = Some(face);
                                        break;
                                    }
                                }
                                (fallback, true)
                            }
                            FamilySelection::Courier => (None, false),
                        },
                    };
                    if let Some(candidate) = candidate {
                        if used_generic {
                            fallback_faces.insert((family.clone(), style));
                        }
                        selected[index] = Some(insert_resolved_face(
                            &mut faces,
                            &mut face_bytes,
                            &mut face_indices,
                            &mut family_names,
                            &candidate.bytes,
                            &candidate.family_label,
                            &candidate.face_label,
                            style,
                            ResolvedFontSourceKind::Host,
                            candidate.face_index,
                            candidate.metrics,
                        )?);
                    }
                }

                let family_preference = [
                    ResolvedFaceStyle::BoldItalic,
                    ResolvedFaceStyle::Bold,
                    ResolvedFaceStyle::Italic,
                    ResolvedFaceStyle::Regular,
                ];
                let preferred_face = family_preference
                    .into_iter()
                    .find_map(|style| selected[resolved_style_index(style)].clone());
                for style in ResolvedFaceStyle::ALL {
                    let index = resolved_style_index(style);
                    if selected[index].is_some() {
                        continue;
                    }
                    fallback_faces.insert((family.clone(), style));
                    selected[index] = Some(match &preferred_face {
                        Some(face) => face.clone(),
                        None => bundled_ids[index]
                            .clone()
                            .ok_or_else(|| FontPreparationError::bounded("bundled-face-missing"))?,
                    });
                }
            }
            let selected = selected
                .map(|face| {
                    face.ok_or_else(|| FontPreparationError::bounded("incomplete-face-selection"))
                })
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            let has_exact_family_face = selected.iter().any(|id| {
                faces
                    .get(id)
                    .is_some_and(|face| face.source_kind == ResolvedFontSourceKind::Host)
            });
            let resolution_kind = match kind {
                FamilyKind::CourierAlias => FontResolutionKind::CourierAlias,
                FamilyKind::Sans => FontResolutionKind::NearestSans,
                FamilyKind::Serif => FontResolutionKind::NearestSerif,
                FamilyKind::Unknown if has_exact_family_face => FontResolutionKind::ExactHost,
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
            face_indices,
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
            let face_index = prepared
                .face_indices
                .get(id)
                .ok_or_else(|| FontPreparationError::bounded("registered-face-index-missing"))?;
            if let Some(existing) = self.registered_bytes.get(id) {
                if existing.as_ref() != bytes.as_ref() {
                    return Err(FontPreparationError::bounded("resolved-id-byte-collision"));
                }
                if self.registered_face_indices.get(id) != Some(face_index) {
                    return Err(FontPreparationError::bounded("resolved-id-index-collision"));
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
            let face_index = *prepared
                .face_indices
                .get(id)
                .ok_or_else(|| FontPreparationError::bounded("registered-face-index-missing"))?;
            let data_key = format!("scriptcraft-resolved-{}", id.as_str());
            let mut font_data = FontData::from_owned(bytes.as_ref().to_vec());
            font_data.index = face_index;
            context.add_font(FontInsert::new(
                data_key.as_str(),
                font_data,
                vec![InsertFontFamily {
                    family: FontFamily::Name(family_name.clone().into()),
                    priority: FontPriority::Highest,
                }],
            ));
            self.registered_bytes.insert(id.clone(), bytes.clone());
            self.registered_face_indices.insert(id.clone(), face_index);
            self.registered_byte_count = self.registered_byte_count.saturating_add(bytes.len());
        }

        let ready = context.fonts(|fonts| {
            let definitions = fonts.definitions();
            prepared.face_bytes.keys().all(|id| {
                let Some(family_name) = prepared.egui_family_names.get(id) else {
                    return false;
                };
                let Some(face_index) = prepared.face_indices.get(id) else {
                    return false;
                };
                let data_key = format!("scriptcraft-resolved-{}", id.as_str());
                let family = FontFamily::Name(family_name.clone().into());
                definitions
                    .font_data
                    .get(&data_key)
                    .is_some_and(|data| data.index == *face_index)
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
        self.native_font_catalog.invalidate();
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
    face_index: u32,
    metrics: ResolvedFaceMetrics,
    family_label: String,
    face_label: String,
}

fn generic_family_candidates(selection: FamilySelection) -> &'static [&'static str] {
    match selection {
        FamilySelection::Sans => &["sans-serif", "Arial", "Helvetica", "Liberation Sans"],
        FamilySelection::Serif => &["serif", "Times New Roman", "Times", "Liberation Serif"],
        FamilySelection::Courier => &[],
    }
}

fn native_style(style: ResolvedFaceStyle) -> super::host_font_catalog::FaceStyle {
    use super::host_font_catalog::FaceStyle as NativeStyle;
    match style {
        ResolvedFaceStyle::Regular => NativeStyle::Regular,
        ResolvedFaceStyle::Bold => NativeStyle::Bold,
        ResolvedFaceStyle::Italic => NativeStyle::Italic,
        ResolvedFaceStyle::BoldItalic => NativeStyle::BoldItalic,
    }
}

fn resolve_native_style(
    catalog: &mut super::host_font_catalog::NativeFontCatalog,
    family: &str,
    style: ResolvedFaceStyle,
    scalars: &BTreeSet<char>,
    bytes_read: &mut usize,
) -> Result<Option<ParsedHostFace>, FontPreparationError> {
    let Some(native) = catalog.query_exact(family, native_style(style)) else {
        return Ok(None);
    };
    *bytes_read = bytes_read.saturating_add(native.bytes.len());
    if *bytes_read > MAX_PREPARATION_BYTES {
        return Err(FontPreparationError::bounded("font-byte-budget"));
    }
    let metrics = match metrics_for_bytes_and_index(&native.bytes, native.collection_index, scalars)
    {
        Ok(metrics) => metrics,
        Err(_) => return Ok(None),
    };
    let face_label = native
        .full_name
        .or(native.postscript_name)
        .unwrap_or_else(|| format!("{} {:?}", native.family_label, style));
    Ok(Some(ParsedHostFace {
        bytes: native.bytes.as_ref().to_vec(),
        face_index: native.collection_index,
        metrics,
        family_label: native.family_label,
        face_label,
    }))
}

fn metrics_for_bytes(
    bytes: &[u8],
    scalars: &BTreeSet<char>,
) -> Result<ResolvedFaceMetrics, FontPreparationError> {
    // This compatibility entry point has no selection context, so a collection
    // is ambiguous. Exact host descriptors use the indexed entry point below.
    if bytes.starts_with(b"ttcf") {
        return Err(FontPreparationError::bounded("unsupported-font-container"));
    }
    metrics_for_bytes_and_index(bytes, 0, scalars)
}

fn metrics_for_bytes_and_index(
    bytes: &[u8],
    face_index: u32,
    scalars: &BTreeSet<char>,
) -> Result<ResolvedFaceMetrics, FontPreparationError> {
    if bytes.starts_with(b"wOFF") || bytes.starts_with(b"wOF2") {
        return Err(FontPreparationError::bounded("unsupported-font-container"));
    }
    if bytes.starts_with(b"ttcf") && !has_collection_face(bytes, face_index) {
        return Err(FontPreparationError::bounded(
            "invalid-collection-face-index",
        ));
    }
    let font = FontRef::try_from_slice_and_index(bytes, face_index)
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

fn has_collection_face(bytes: &[u8], face_index: u32) -> bool {
    if bytes.len() < 12 || !bytes.starts_with(b"ttcf") {
        return false;
    }
    let face_count = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    face_index < face_count
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
    indices_by_id: &mut BTreeMap<ResolvedFontId, u32>,
    family_names: &mut BTreeMap<ResolvedFontId, String>,
    bytes: &[u8],
    family_label: &str,
    face_label: &str,
    style: ResolvedFaceStyle,
    source_kind: ResolvedFontSourceKind,
    face_index: u32,
    metrics: ResolvedFaceMetrics,
) -> Result<ResolvedFontId, FontPreparationError> {
    let fingerprint =
        resolved_identity_fingerprint(bytes, face_index, style, source_kind, &metrics);
    let id = allocate_resolved_font_id(
        bytes,
        face_index,
        style,
        source_kind,
        &metrics,
        fingerprint,
        bytes_by_id,
        indices_by_id,
        faces,
    )?;
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
        indices_by_id.insert(id.clone(), face_index);
        let egui_name = format!("ScriptCraft Resolved {}", id.as_str());
        family_names.insert(id.clone(), egui_name);
    }
    Ok(id)
}

fn resolved_identity_fingerprint(
    bytes: &[u8],
    face_index: u32,
    style: ResolvedFaceStyle,
    source_kind: ResolvedFontSourceKind,
    metrics: &ResolvedFaceMetrics,
) -> u64 {
    let mut hash = content_fingerprint(bytes);
    let mut mix = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in face_index.to_le_bytes() {
        mix(byte);
    }
    mix(style as u8);
    mix(source_kind as u8);
    for value in [
        metrics.units_per_em as i64,
        i64::from(metrics.ascender),
        i64::from(metrics.descender),
        i64::from(metrics.line_gap),
    ] {
        for byte in value.to_le_bytes() {
            mix(byte);
        }
    }
    for (scalar, advance) in &metrics.scalar_advances {
        for byte in (*scalar as u32).to_le_bytes() {
            mix(byte);
        }
        for byte in advance.advance_units.to_le_bytes() {
            mix(byte);
        }
        mix(u8::from(advance.missing_glyph));
    }
    hash
}

fn allocate_resolved_font_id(
    bytes: &[u8],
    face_index: u32,
    style: ResolvedFaceStyle,
    source_kind: ResolvedFontSourceKind,
    metrics: &ResolvedFaceMetrics,
    fingerprint: u64,
    existing: &BTreeMap<ResolvedFontId, Arc<[u8]>>,
    existing_indices: &BTreeMap<ResolvedFontId, u32>,
    existing_faces: &BTreeMap<ResolvedFontId, ResolvedFontFace>,
) -> Result<ResolvedFontId, FontPreparationError> {
    let style_suffix = match style {
        ResolvedFaceStyle::Regular => "r",
        ResolvedFaceStyle::Bold => "b",
        ResolvedFaceStyle::Italic => "i",
        ResolvedFaceStyle::BoldItalic => "bi",
    };
    for collision_index in 0..=16u32 {
        let spelling = if collision_index == 0 {
            format!("font-{fingerprint:016x}-f{face_index}-{style_suffix}")
        } else {
            format!("font-{fingerprint:016x}-f{face_index}-{style_suffix}-c{collision_index}")
        };
        let id = ResolvedFontId::new(spelling)
            .map_err(|_| FontPreparationError::bounded("resolved-id-bound"))?;
        match existing.get(&id) {
            Some(existing_bytes)
                if existing_bytes.as_ref() == bytes
                    && existing_indices.get(&id) == Some(&face_index)
                    && existing_faces.get(&id).is_some_and(|face| {
                        face.style == style
                            && face.source_kind == source_kind
                            && face.metrics == *metrics
                    }) =>
            {
                return Ok(id)
            }
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

#[cfg(test)]
mod resolved_font_tests {
    use super::*;
    use eframe::egui::{Color32, FontId};

    fn project_with_family(family: &str) -> Option<Project> {
        let Ok(mut project) = scriptcraft_fdx::new_project() else {
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
                assert_eq!(
                    definitions.font_data.get(&data_key).map(|data| data.index),
                    prepared.face_indices.get(id).copied()
                );
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

    fn is_prepared_environment_ready(
        context: &egui::Context,
        prepared: &PreparedDocumentFonts,
    ) -> bool {
        context.fonts(|fonts| {
            let definitions = fonts.definitions();
            prepared.face_bytes.keys().all(|id| {
                let (Some(name), Some(index)) = (
                    prepared.egui_family_names.get(id),
                    prepared.face_indices.get(id),
                ) else {
                    return false;
                };
                let data_key = format!("scriptcraft-resolved-{}", id.as_str());
                definitions
                    .font_data
                    .get(&data_key)
                    .is_some_and(|data| data.index == *index)
                    && definitions
                        .families
                        .get(&FontFamily::Name(name.clone().into()))
                        .is_some_and(|keys| keys.iter().any(|key| key == &data_key))
            })
        })
    }

    fn deterministic_two_face_collection() -> Vec<u8> {
        let source_faces = [
            bundled_courier_prime_faces()[0],
            bundled_courier_prime_faces()[1],
        ];
        let header_len = 12 + source_faces.len() * 4;
        let align_four = |value: usize| (value + 3) & !3;
        let mut offsets = Vec::with_capacity(source_faces.len());
        let mut total_len = align_four(header_len);
        for face in source_faces {
            offsets.push(total_len);
            total_len = align_four(total_len + face.len());
        }
        let mut collection = vec![0; total_len];
        collection[..4].copy_from_slice(b"ttcf");
        collection[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        collection[8..12].copy_from_slice(&(source_faces.len() as u32).to_be_bytes());

        for (face_index, (face, base_offset)) in source_faces.into_iter().zip(offsets).enumerate() {
            let offset_slot = 12 + face_index * 4;
            collection[offset_slot..offset_slot + 4]
                .copy_from_slice(&(base_offset as u32).to_be_bytes());
            collection[base_offset..base_offset + face.len()].copy_from_slice(face);

            let table_count = u16::from_be_bytes([face[4], face[5]]) as usize;
            for table_index in 0..table_count {
                let record_offset = base_offset + 12 + table_index * 16;
                let old_offset = u32::from_be_bytes([
                    collection[record_offset + 8],
                    collection[record_offset + 9],
                    collection[record_offset + 10],
                    collection[record_offset + 11],
                ]);
                collection[record_offset + 8..record_offset + 12]
                    .copy_from_slice(&(old_offset + base_offset as u32).to_be_bytes());
            }
        }
        collection
    }

    fn injected_native_face(
        family: &str,
        style: ResolvedFaceStyle,
        bytes: &[u8],
        collection_index: u32,
    ) -> super::super::host_font_catalog::NativeFontFace {
        use super::super::host_font_catalog::{FaceStyle as NativeStyle, FontSourceLocation};
        let style = match style {
            ResolvedFaceStyle::Regular => NativeStyle::Regular,
            ResolvedFaceStyle::Bold => NativeStyle::Bold,
            ResolvedFaceStyle::Italic => NativeStyle::Italic,
            ResolvedFaceStyle::BoldItalic => NativeStyle::BoldItalic,
        };
        super::super::host_font_catalog::NativeFontFace {
            requested_family: family.to_owned(),
            family_label: family.to_owned(),
            full_name: Some(format!("{family} {style:?}")),
            postscript_name: None,
            style,
            source: FontSourceLocation::Memory,
            bytes: Arc::from(bytes),
            collection_index,
        }
    }

    fn inject_native_faces(
        store: &mut DocumentFontStore,
        faces: Vec<super::super::host_font_catalog::NativeFontFace>,
    ) {
        store.native_font_catalog =
            super::super::host_font_catalog::NativeFontCatalog::with_test_faces(faces);
    }

    fn inject_native_faces_with_query_log(
        store: &mut DocumentFontStore,
        faces: Vec<super::super::host_font_catalog::NativeFontFace>,
    ) -> Arc<std::sync::Mutex<Vec<(String, super::super::host_font_catalog::FaceStyle)>>> {
        let (catalog, calls) =
            super::super::host_font_catalog::NativeFontCatalog::with_test_faces_and_query_log(
                faces,
            );
        store.native_font_catalog = catalog;
        calls
    }

    fn native_style_faces(family: &str) -> Vec<super::super::host_font_catalog::NativeFontFace> {
        ResolvedFaceStyle::ALL
            .into_iter()
            .enumerate()
            .map(|(index, style)| {
                let bytes = bundled_courier_prime_faces()[index];
                injected_native_face(family, style, &bytes, 0)
            })
            .collect()
    }

    #[test]
    fn document_store_owns_and_invalidates_native_catalog() {
        let mut store = DocumentFontStore::default();
        let initial_generation = store.native_font_catalog.generation();
        assert!(initial_generation > 0);
        store.invalidate_catalog();
        assert!(store.native_font_catalog.generation() > initial_generation);
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
    fn futura_like_declared_family_resolves_exact_host_faces_independently() {
        let Some(project) = project_with_family("Futura") else {
            assert!(false, "Futura project should be constructible");
            return;
        };
        let mut store = DocumentFontStore::default();
        inject_native_faces(&mut store, native_style_faces("Futura"));
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(false, "injected exact Futura faces should prepare");
            return;
        };
        let Some(request) = prepared.environment.resolution_for("Futura") else {
            assert!(false, "declared custom family must be retained");
            return;
        };
        assert_eq!(request.kind, FontResolutionKind::ExactHost);
        for style in ResolvedFaceStyle::ALL {
            let Some(face) = prepared.environment.face(request.face_for_style(style)) else {
                assert!(false, "resolved exact style must have a face record");
                return;
            };
            assert_eq!(face.source_kind, ResolvedFontSourceKind::Host);
            assert_eq!(face.family_label, "Futura");
            assert!(face.metrics.scalar_advances.contains_key(&'A'));
        }
        assert!(ResolvedFaceStyle::ALL.into_iter().all(|style| !prepared
            .fallback_faces
            .contains(&("Futura".to_owned(), style))));
    }

    #[test]
    fn courier_aliases_short_circuit_native_lookup_and_select_bundled_styles() {
        let Some(project) = project_with_family("Courier Final Draft") else {
            assert!(false, "Courier project should be constructible");
            return;
        };
        let mut store = DocumentFontStore::default();
        let calls = inject_native_faces_with_query_log(
            &mut store,
            native_style_faces("Courier Final Draft"),
        );
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(false, "Courier alias should prepare from bundled faces");
            return;
        };
        let Some(request) = prepared.environment.resolution_for("Courier Final Draft") else {
            assert!(
                false,
                "declared Courier alias must remain in the environment"
            );
            return;
        };
        assert_eq!(request.kind, FontResolutionKind::CourierAlias);
        assert!(calls.lock().is_ok_and(|calls| calls
            .iter()
            .all(|(family, _)| !family.eq_ignore_ascii_case("Courier Final Draft"))));
        for style in ResolvedFaceStyle::ALL {
            let Some(face) = prepared.environment.face(request.face_for_style(style)) else {
                assert!(false, "Courier style must resolve");
                return;
            };
            assert_eq!(face.source_kind, ResolvedFontSourceKind::Bundled);
        }
    }

    #[test]
    fn known_sans_and_serif_exact_host_faces_keep_their_base14_classes() {
        for (family, expected_kind) in [
            ("Arial", FontResolutionKind::NearestSans),
            ("Times New Roman", FontResolutionKind::NearestSerif),
        ] {
            let Some(project) = project_with_family(family) else {
                assert!(false, "known-family project should be constructible");
                return;
            };
            let mut store = DocumentFontStore::default();
            inject_native_faces(&mut store, native_style_faces(family));
            let Ok(prepared) = store.prepare_document(&project) else {
                assert!(false, "injected known-family faces should prepare");
                return;
            };
            let Some(request) = prepared.environment.resolution_for(family) else {
                assert!(false, "known family should remain the declared request");
                return;
            };
            assert_eq!(request.kind, expected_kind);
            let Some(face) = prepared.environment.face(&request.regular) else {
                assert!(false, "regular known-family face should exist");
                return;
            };
            assert_eq!(face.source_kind, ResolvedFontSourceKind::Host);
            assert_eq!(face.family_label, family);
        }
    }

    #[test]
    fn generic_face_is_used_only_after_exact_known_family_misses() {
        let Some(project) = project_with_family("Arial") else {
            assert!(false, "Arial project should be constructible");
            return;
        };
        let mut store = DocumentFontStore::default();
        let calls = inject_native_faces_with_query_log(&mut store, native_style_faces("Helvetica"));
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(false, "generic host face should prepare after exact miss");
            return;
        };
        let Some(request) = prepared.environment.resolution_for("Arial") else {
            assert!(false, "declared Arial family must be retained");
            return;
        };
        assert_eq!(request.kind, FontResolutionKind::NearestSans);
        assert!(prepared
            .fallback_faces
            .contains(&("Arial".to_owned(), ResolvedFaceStyle::Regular)));
        assert_eq!(
            prepared
                .environment
                .face(&request.regular)
                .map(|face| face.family_label.as_str()),
            Some("Helvetica")
        );
        let calls = calls.lock().ok();
        let Some(calls) = calls else {
            assert!(
                false,
                "injected native source should record its query order"
            );
            return;
        };
        let exact_attempt = calls.iter().position(|(family, style)| {
            family == "Arial" && *style == native_style(ResolvedFaceStyle::Regular)
        });
        let generic_attempt = calls.iter().position(|(family, style)| {
            family == "Helvetica" && *style == native_style(ResolvedFaceStyle::Regular)
        });
        assert!(exact_attempt.is_some() && generic_attempt.is_some());
        assert!(
            exact_attempt < generic_attempt,
            "exact family is queried before generic fallback"
        );
    }

    #[test]
    fn unknown_family_uses_matching_bundled_courier_fallback() {
        let Some(project) = project_with_family("Uninstalled Custom Family") else {
            assert!(false, "unknown-family project should be constructible");
            return;
        };
        let mut store = DocumentFontStore::default();
        inject_native_faces(&mut store, Vec::new());
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(false, "unknown-family fallback should prepare");
            return;
        };
        let Some(request) = prepared
            .environment
            .resolution_for("Uninstalled Custom Family")
        else {
            assert!(false, "unknown declared family must be retained");
            return;
        };
        assert_eq!(request.kind, FontResolutionKind::UnknownFallback);
        for style in ResolvedFaceStyle::ALL {
            assert!(prepared
                .fallback_faces
                .contains(&("Uninstalled Custom Family".to_owned(), style)));
            assert_eq!(
                prepared
                    .environment
                    .face(request.face_for_style(style))
                    .map(|face| face.source_kind),
                Some(ResolvedFontSourceKind::Bundled)
            );
        }
    }

    #[test]
    fn missing_custom_style_stays_in_family_using_global_style_priority() {
        let Some(project) = project_with_family("Futura") else {
            assert!(false, "Futura project should be constructible");
            return;
        };
        let faces = ResolvedFaceStyle::ALL
            .into_iter()
            .take(3)
            .enumerate()
            .map(|(index, style)| {
                injected_native_face(
                    "Futura",
                    style,
                    bundled_courier_prime_faces()[(index + 1) % 4],
                    0,
                )
            })
            .collect();
        let mut store = DocumentFontStore::default();
        inject_native_faces(&mut store, faces);
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(false, "partially available exact family should prepare");
            return;
        };
        let Some(request) = prepared.environment.resolution_for("Futura") else {
            assert!(false, "Futura request must be preserved");
            return;
        };
        assert_eq!(request.kind, FontResolutionKind::ExactHost);
        for style in ResolvedFaceStyle::ALL {
            if style == ResolvedFaceStyle::BoldItalic {
                assert!(prepared
                    .fallback_faces
                    .contains(&("Futura".to_owned(), style)));
                assert_eq!(request.bold_italic, request.bold);
            }
            assert_eq!(
                prepared
                    .environment
                    .face(request.face_for_style(style))
                    .map(|face| face.source_kind),
                Some(ResolvedFontSourceKind::Host)
            );
        }
        assert_ne!(request.regular, request.bold);
        assert_ne!(request.italic, request.bold);
    }

    #[test]
    fn direct_inline_font_run_family_is_prepared_exactly() {
        let Ok(mut project) = scriptcraft_fdx::new_project() else {
            assert!(false, "default project should be constructible");
            return;
        };
        let run = scriptcraft_model::TextRun {
            text: "inline".to_owned(),
            attributes: vec![scriptcraft_model::RunAttribute {
                name: "Font".to_owned(),
                value: "Run Futura".to_owned(),
            }],
        };
        let Ok(paragraph) = scriptcraft_model::TitlePageParagraph::with_runs(
            "font-test-title",
            None,
            vec![run],
            None,
            Default::default(),
        ) else {
            assert!(false, "inline title run should be constructible");
            return;
        };
        project.title_page.push(paragraph);
        let mut store = DocumentFontStore::default();
        inject_native_faces(&mut store, native_style_faces("Run Futura"));
        let Ok(prepared) = store.prepare_document(&project) else {
            assert!(false, "inline Font family should prepare");
            return;
        };
        assert_eq!(
            prepared
                .environment
                .resolution_for("Run Futura")
                .map(|request| request.kind),
            Some(FontResolutionKind::ExactHost)
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
            let Ok(metrics) = metrics_for_bytes(bytes, &scalars) else {
                assert!(false, "bundled style metrics should parse");
                return;
            };
            assert_eq!(metrics.scalar_advances.len(), 4);
            assert!(metrics.units_per_em > 0);
            assert!(metrics.scalar_advances.contains_key(&'�'));
            let fingerprint = resolved_identity_fingerprint(
                bytes,
                0,
                style,
                ResolvedFontSourceKind::Bundled,
                &metrics,
            );
            let id = allocate_resolved_font_id(
                bytes,
                0,
                style,
                ResolvedFontSourceKind::Bundled,
                &metrics,
                fingerprint,
                &BTreeMap::new(),
                &BTreeMap::new(),
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
    fn indexed_collection_metrics_and_resolved_identity_preserve_face_index() {
        let collection = deterministic_two_face_collection();
        let scalars = BTreeSet::from(['A', 'W', 'i']);
        assert_eq!(
            metrics_for_bytes(&collection, &scalars),
            Err(FontPreparationError::bounded("unsupported-font-container"))
        );
        let (Ok(face_zero), Ok(face_one)) = (
            metrics_for_bytes_and_index(&collection, 0, &scalars),
            metrics_for_bytes_and_index(&collection, 1, &scalars),
        ) else {
            assert!(false, "the deterministic collection faces must parse");
            return;
        };
        assert!(face_zero.units_per_em > 0 && face_one.units_per_em > 0);
        assert_eq!(face_zero.scalar_advances.len(), scalars.len());
        assert_eq!(face_one.scalar_advances.len(), scalars.len());
        assert!(metrics_for_bytes_and_index(&collection, 2, &scalars).is_err());
        assert!(metrics_for_bytes_and_index(b"wOFFbroken", 0, &scalars).is_err());
        assert!(metrics_for_bytes_and_index(b"wOF2broken", 0, &scalars).is_err());

        let mut faces = BTreeMap::new();
        let mut bytes_by_id = BTreeMap::new();
        let mut indices_by_id = BTreeMap::new();
        let mut family_names = BTreeMap::new();
        let style = ResolvedFaceStyle::Regular;
        let insert = |index,
                      metrics: ResolvedFaceMetrics,
                      faces: &mut BTreeMap<ResolvedFontId, ResolvedFontFace>,
                      bytes_by_id: &mut BTreeMap<ResolvedFontId, Arc<[u8]>>,
                      indices_by_id: &mut BTreeMap<ResolvedFontId, u32>,
                      family_names: &mut BTreeMap<ResolvedFontId, String>| {
            insert_resolved_face(
                faces,
                bytes_by_id,
                indices_by_id,
                family_names,
                &collection,
                "Injected Collection",
                "Regular",
                style,
                ResolvedFontSourceKind::Host,
                index,
                metrics,
            )
        };
        let Ok(zero_id) = insert(
            0,
            face_zero.clone(),
            &mut faces,
            &mut bytes_by_id,
            &mut indices_by_id,
            &mut family_names,
        ) else {
            assert!(false, "face index zero should insert");
            return;
        };
        let Ok(zero_reuse) = insert(
            0,
            face_zero,
            &mut faces,
            &mut bytes_by_id,
            &mut indices_by_id,
            &mut family_names,
        ) else {
            assert!(false, "identical indexed face should be reusable");
            return;
        };
        let Ok(one_id) = insert(
            1,
            face_one,
            &mut faces,
            &mut bytes_by_id,
            &mut indices_by_id,
            &mut family_names,
        ) else {
            assert!(false, "face index one should insert independently");
            return;
        };
        assert_eq!(zero_id, zero_reuse);
        assert_ne!(zero_id, one_id);
        assert_eq!(faces.len(), 2);
        assert_eq!(bytes_by_id.len(), 2);
        assert_eq!(indices_by_id.len(), 2);
    }

    #[test]
    fn same_bytes_and_index_with_different_metrics_get_distinct_resolved_ids() {
        let bytes = bundled_courier_prime_faces()[0];
        let scalars = BTreeSet::from(['A']);
        let Ok(metrics) = metrics_for_bytes(bytes, &scalars) else {
            assert!(false, "bundled metrics should parse");
            return;
        };
        let mut changed_advances = metrics.scalar_advances.clone();
        if let Some(advance) = changed_advances.get_mut(&'A') {
            advance.advance_units = advance.advance_units.saturating_add(1);
        }
        let Ok(changed_metrics) = ResolvedFaceMetrics::new(
            metrics.units_per_em,
            metrics.ascender,
            metrics.descender,
            metrics.line_gap,
            changed_advances,
        ) else {
            assert!(false, "modified test metrics should remain valid");
            return;
        };
        let mut faces = BTreeMap::new();
        let mut bytes_by_id = BTreeMap::new();
        let mut indices_by_id = BTreeMap::new();
        let mut family_names = BTreeMap::new();
        let first = insert_resolved_face(
            &mut faces,
            &mut bytes_by_id,
            &mut indices_by_id,
            &mut family_names,
            bytes,
            "Metric identity fixture",
            "Regular",
            ResolvedFaceStyle::Regular,
            ResolvedFontSourceKind::Host,
            0,
            metrics,
        );
        let second = insert_resolved_face(
            &mut faces,
            &mut bytes_by_id,
            &mut indices_by_id,
            &mut family_names,
            bytes,
            "Metric identity fixture",
            "Regular",
            ResolvedFaceStyle::Regular,
            ResolvedFontSourceKind::Host,
            0,
            changed_metrics,
        );
        assert!(first.is_ok() && second.is_ok());
        assert_ne!(first.ok(), second.ok());
    }

    #[test]
    fn indexed_host_cache_fingerprint_and_egui_registration_include_face_index() {
        let Some(project) = project_with_family("Arial") else {
            assert!(false, "Arial test project should be constructible");
            return;
        };
        let collection = deterministic_two_face_collection();
        let mut store = DocumentFontStore::default();
        inject_native_faces(
            &mut store,
            ResolvedFaceStyle::ALL
                .into_iter()
                .map(|style| injected_native_face("Arial", style, &collection, 0))
                .collect(),
        );
        let Ok(index_zero) = store.prepare_document(&project) else {
            assert!(false, "index-zero collection face should prepare");
            return;
        };
        let Some(zero_resolution) = index_zero.environment.resolution_for("Arial") else {
            assert!(false, "prepared environment must retain the Arial request");
            return;
        };
        let zero_regular = zero_resolution.regular.clone();
        let zero_fingerprint = index_zero.environment.content_fingerprint();
        let Ok(index_zero_cached) = store.prepare_document(&project) else {
            assert!(
                false,
                "a matching preparation request should reuse its cache entry"
            );
            return;
        };
        assert!(index_zero.signature_matches(&index_zero_cached));

        store.catalog_generation = store.catalog_generation.saturating_add(1);
        store.prepared_cache.clear();
        inject_native_faces(
            &mut store,
            ResolvedFaceStyle::ALL
                .into_iter()
                .map(|style| injected_native_face("Arial", style, &collection, 1))
                .collect(),
        );
        let Ok(index_one) = store.prepare_document(&project) else {
            assert!(false, "index-one collection face should prepare");
            return;
        };
        let Some(one_resolution) = index_one.environment.resolution_for("Arial") else {
            assert!(false, "prepared environment must retain the Arial request");
            return;
        };
        assert_ne!(zero_regular, one_resolution.regular);
        assert_ne!(
            zero_fingerprint,
            index_one.environment.content_fingerprint()
        );
        assert!(!index_zero.signature_matches(&index_one));

        let expected_registered_bytes = index_one
            .face_bytes
            .values()
            .map(|bytes| bytes.len())
            .sum::<usize>();
        let context = egui::Context::default();
        let _ = context.run(egui::RawInput::default(), |ctx| {
            assert_eq!(store.register_prepared_fonts(ctx, &index_one), Ok(false));
            assert!(!is_prepared_environment_ready(ctx, &index_one));
        });
        let _ = context.run(egui::RawInput::default(), |ctx| {
            assert_eq!(store.register_prepared_fonts(ctx, &index_one), Ok(true));
            assert!(is_prepared_environment_ready(ctx, &index_one));
            for (id, expected_index) in &index_one.face_indices {
                let data_key = format!("scriptcraft-resolved-{}", id.as_str());
                assert_eq!(
                    ctx.fonts(|fonts| fonts
                        .definitions()
                        .font_data
                        .get(&data_key)
                        .map(|data| data.index)),
                    Some(*expected_index)
                );
            }
            assert_prepared_faces_paintable(ctx, &index_one);
        });
        assert_eq!(store.registered_byte_count, expected_registered_bytes);
        let collection_face_registrations = index_one
            .face_bytes
            .iter()
            .filter(|(_, bytes)| bytes.as_ref() == collection.as_slice())
            .count();
        assert!(collection_face_registrations > 1);
        assert_eq!(
            index_one
                .face_bytes
                .iter()
                .filter(|(_, bytes)| bytes.as_ref() == collection.as_slice())
                .map(|(_, bytes)| bytes.len())
                .sum::<usize>(),
            collection.len() * collection_face_registrations
        );

        let mut bounded_store = DocumentFontStore::default();
        bounded_store.registered_byte_count = MAX_REGISTERED_FONT_BYTES
            .saturating_sub(expected_registered_bytes)
            .saturating_add(1);
        let bounded_context = egui::Context::default();
        let _ = bounded_context.run(egui::RawInput::default(), |ctx| {
            assert_eq!(
                bounded_store.register_prepared_fonts(ctx, &index_one),
                Err(FontPreparationError::bounded("registered-font-byte-limit"))
            );
        });
        assert!(bounded_store.registered_bytes.is_empty());
    }

    #[test]
    fn empty_default_project_waits_for_activation_pass_before_galley_paint() {
        let Ok(project) = scriptcraft_fdx::new_project() else {
            assert!(false, "default Project should be constructible");
            return;
        };
        let mut store = DocumentFontStore::default();
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
        let Ok(default_project) = scriptcraft_fdx::new_project() else {
            assert!(false, "default Project should be constructible");
            return;
        };
        let Some(replacement_project) = project_with_family("Arial") else {
            assert!(false, "test replacement project should be constructible");
            return;
        };
        let bundled = bundled_courier_prime_faces();
        let mut store = DocumentFontStore::default();
        inject_native_faces(&mut store, native_style_faces("Arial"));
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
        let mut store = DocumentFontStore::default();
        inject_native_faces(&mut store, Vec::new());
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

        let metrics = metrics_for_bytes(bundled_courier_prime_faces()[0], &BTreeSet::from(['A']));
        let Ok(metrics) = metrics else {
            assert!(false, "bundled test metrics should parse");
            return;
        };
        let mut existing = BTreeMap::new();
        let mut existing_indices = BTreeMap::new();
        let mut existing_faces = BTreeMap::new();
        let source_kind = ResolvedFontSourceKind::Synthetic;
        let fingerprint = 7;
        let base = allocate_resolved_font_id(
            b"first",
            0,
            ResolvedFaceStyle::Regular,
            source_kind,
            &metrics,
            fingerprint,
            &existing,
            &existing_indices,
            &existing_faces,
        );
        assert!(base.is_ok());
        if let Ok(base) = base {
            existing.insert(base.clone(), Arc::from(&b"first"[..]));
            existing_indices.insert(base.clone(), 0);
            if let Ok(face) = ResolvedFontFace::new(
                base.clone(),
                "test",
                "test regular",
                ResolvedFaceStyle::Regular,
                source_kind,
                5,
                content_fingerprint(b"first"),
                metrics.clone(),
            ) {
                existing_faces.insert(base.clone(), face);
            }
            let same = allocate_resolved_font_id(
                b"first",
                0,
                ResolvedFaceStyle::Regular,
                source_kind,
                &metrics,
                fingerprint,
                &existing,
                &existing_indices,
                &existing_faces,
            );
            let collision = allocate_resolved_font_id(
                b"second",
                0,
                ResolvedFaceStyle::Regular,
                source_kind,
                &metrics,
                fingerprint,
                &existing,
                &existing_indices,
                &existing_faces,
            );
            assert_eq!(same, Ok(base.clone()));
            assert!(collision.is_ok_and(|id| id != base && id.as_str().ends_with("-c1")));
        }
    }
}
