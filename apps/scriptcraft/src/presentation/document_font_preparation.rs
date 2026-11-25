use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use ab_glyph::{Font, FontRef};
use scriptcraft_model::typography::{
    content_fingerprint, DocumentFontEnvironment, FaceStyle as ResolvedFaceStyle,
    FontResolutionKind, LeadingResolverKind, RequestedFontResolution, ResolvedFaceMetrics, ResolvedFontFace,
    ResolvedDecorationMetrics, ResolvedFontId, ResolvedFontSourceKind, ScalarAdvance,
    MAX_FONT_REQUESTS, MAX_METRIC_UNITS, MAX_RESOLVED_FACES, MAX_SCALAR_ADVANCES,
};
use scriptcraft_model::{Project, TextRun};

const MAX_DOCUMENT_FONT_SCALARS: usize = MAX_SCALAR_ADVANCES;
const MAX_DOCUMENT_FONT_REQUESTS: usize = MAX_FONT_REQUESTS;
const MAX_PREPARATION_BYTES: usize = 64 * 1024 * 1024;
const MAX_PREPARED_ENVIRONMENTS: usize = 2;
const HOST_CATALOG_GENERATION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FontPreparationError {
    pub(crate) reason: &'static str,
}

impl FontPreparationError {
    pub(crate) fn bounded(reason: &'static str) -> Self {
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
    pub(super) egui_data_keys: BTreeMap<ResolvedFontId, String>,
    pub(super) face_bytes: BTreeMap<ResolvedFontId, Arc<[u8]>>,
    pub(super) face_indices: BTreeMap<ResolvedFontId, u32>,
    pub(crate) fallback_faces: BTreeSet<(String, ResolvedFaceStyle)>,
    signature: PreparationSignature,
}

impl PreparedDocumentFonts {
    pub(crate) fn face_bytes(&self, id: &ResolvedFontId) -> Option<&Arc<[u8]>> {
        self.face_bytes.get(id)
    }

    pub(crate) fn resolved_ids(&self) -> impl Iterator<Item = &ResolvedFontId> {
        self.face_bytes.keys()
    }

    pub(crate) fn face_index(&self, id: &ResolvedFontId) -> Option<u32> {
        self.face_indices.get(id).copied()
    }

    pub(crate) fn is_exact_packaged_courier_prime(&self, id: &ResolvedFontId) -> bool {
        let Some(face) = self.environment.face(id) else {
            return false;
        };
        let Some(bytes) = self.face_bytes.get(id) else {
            return false;
        };
        let Some(face_index) = self.face_indices.get(id) else {
            return false;
        };
        let expected_style_index = resolved_style_index(face.style);
        face.source_kind == ResolvedFontSourceKind::Bundled
            && face.leading_resolver == LeadingResolverKind::Courier
            && face.family_label == "Courier Prime"
            && face.face_label == courier_style_name(face.style)
            && *face_index == 0
            && bytes.as_ref() == super::bundled_font_startup::courier_prime_faces()[expected_style_index]
    }

    pub(crate) fn signature_matches(&self, other: &Self) -> bool {
        self.signature == other.signature && self.environment == other.environment
    }
}


#[derive(Debug)]
pub(super) struct DocumentFontPreparation {
    native_font_catalog: super::host_font_catalog::NativeFontCatalog,
    prepared_cache: HashMap<PreparationSignature, PreparedDocumentFonts>,
    catalog_generation: u64,
}

impl Default for DocumentFontPreparation {
    fn default() -> Self {
        Self {
            native_font_catalog: super::host_font_catalog::NativeFontCatalog::default(),
            prepared_cache: HashMap::new(),
            catalog_generation: HOST_CATALOG_GENERATION,
        }
    }
}

impl DocumentFontPreparation {
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
        let mut data_keys = BTreeMap::new();
        let mut fallback_faces = BTreeSet::new();
        let bundled = super::bundled_font_startup::courier_prime_faces();
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
                &mut data_keys,
                bytes,
                "Courier Prime",
                courier_style_name(style),
                style,
                ResolvedFontSourceKind::Bundled,
                LeadingResolverKind::Courier,
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
                // Resolve each concrete family/style exactly before ordered
                // candidates. Literal generic requests skip their pseudo-family
                // query; unresolved styles end at matching-style Courier Prime.
                for style in ResolvedFaceStyle::ALL {
                    let index = resolved_style_index(style);
                    let exact = if is_literal_generic_family(family) {
                        None
                    } else {
                        resolve_native_style(
                            &mut self.native_font_catalog,
                            family,
                            style,
                            &signature.scalars,
                            &mut bytes_read,
                        )?
                    };
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
                            &mut data_keys,
                            &candidate.bytes,
                            &candidate.family_label,
                            &candidate.face_label,
                            style,
                            ResolvedFontSourceKind::Host,
                            LeadingResolverKind::Proportional,
                            candidate.face_index,
                            candidate.metrics,
                        )?);
                    }
                }

                for style in ResolvedFaceStyle::ALL {
                    let index = resolved_style_index(style);
                    if selected[index].is_some() {
                        continue;
                    }
                    fallback_faces.insert((family.clone(), style));
                    selected[index] = Some(
                        bundled_ids[index]
                            .clone()
                            .ok_or_else(|| FontPreparationError::bounded("bundled-face-missing"))?,
                    );
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
            egui_data_keys: data_keys,
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
            | "segoe ui"
            | "sans-serif"
    ) {
        (FamilyKind::Sans, FamilySelection::Sans)
    } else if matches!(
        normalized.as_str(),
        "times new roman" | "times" | "liberation serif" | "georgia" | "serif"
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
    let mut families = BTreeSet::from(["Courier Prime".to_owned(), "Arial".to_owned()]);
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
    // Beat geometry and viewport state do not participate in font identity.
    // Only the declared families and displayed scalar repertoire are relevant.
    collect_text_scalars(&mut scalars, "Beat TitleBody…", false)?;
    for card in &project.beat_board().cards {
        collect_text_scalars(&mut scalars, &card.title, false)?;
        for paragraph in &card.paragraphs {
            collect_text_scalars(&mut scalars, &paragraph.text, false)?;
            collect_run_inputs(&mut families, &mut scalars, &paragraph.runs)?;
        }
    }
    // ScriptNote titles and bodies are rendered only in the fixed native card.
    // Keep note ranges, timestamps, writer names, and Type labels out of font
    // identity: none of those values is measured by the card text layout.
    collect_text_scalars(&mut scalars, "TitleBody…", false)?;
    for note in project.script_notes().iter() {
        collect_text_scalars(&mut scalars, &note.title, false)?;
        for paragraph in &note.paragraphs {
            collect_text_scalars(&mut scalars, &paragraph.text, false)?;
            collect_run_inputs(&mut families, &mut scalars, &paragraph.runs)?;
            for run in &paragraph.runs {
                if run_has_all_caps(run) {
                    collect_uppercase_scalars(&mut scalars, &run.text)?;
                }
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
        FamilySelection::Sans => &["Arial", "Helvetica", "Liberation Sans", "Segoe UI"],
        FamilySelection::Serif => &["Times New Roman", "Times", "Liberation Serif", "Georgia"],
        FamilySelection::Courier => &[],
    }
}

fn is_literal_generic_family(family: &str) -> bool {
    family.trim().eq_ignore_ascii_case("serif")
        || family.trim().eq_ignore_ascii_case("sans-serif")
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
    let parsed_face = ttf_parser::Face::from_slice(bytes, face_index)
        .map_err(|_| FontPreparationError::bounded("malformed-font"))?;
    let resolved_units_per_em = upem.round() as u32;
    if u32::from(parsed_face.units_per_em()) != resolved_units_per_em {
        return Err(FontPreparationError::bounded("invalid-font-units"));
    }
    let underline = parsed_face
        .underline_metrics()
        .ok_or_else(|| FontPreparationError::bounded("invalid-decoration-metrics"))?;
    let underline = decoration_metrics(underline.position, underline.thickness)?;
    let strikethrough = parsed_face
        .strikeout_metrics()
        .ok_or_else(|| FontPreparationError::bounded("invalid-decoration-metrics"))?;
    let strikethrough = decoration_metrics(strikethrough.position, strikethrough.thickness)?;
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
        underline,
        strikethrough,
        advances,
    )
    .map_err(|_| FontPreparationError::bounded("invalid-font-metrics"))
}

fn decoration_metrics(
    position: i16,
    thickness: i16,
) -> Result<ResolvedDecorationMetrics, FontPreparationError> {
    let position = i32::from(position);
    let thickness = u32::try_from(thickness)
        .map_err(|_| FontPreparationError::bounded("invalid-decoration-metrics"))?;
    if position.unsigned_abs() > MAX_METRIC_UNITS as u32
        || thickness == 0
        || thickness > MAX_METRIC_UNITS as u32
        || position
            .checked_sub(i32::try_from(thickness).map_err(|_| {
                FontPreparationError::bounded("invalid-decoration-metrics")
            })?)
            .is_none()
    {
        return Err(FontPreparationError::bounded("invalid-decoration-metrics"));
    }
    Ok(ResolvedDecorationMetrics {
        position,
        thickness,
    })
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
    data_keys_by_id: &mut BTreeMap<ResolvedFontId, String>,
    bytes: &[u8],
    family_label: &str,
    face_label: &str,
    style: ResolvedFaceStyle,
    source_kind: ResolvedFontSourceKind,
    leading_resolver: LeadingResolverKind,
    face_index: u32,
    metrics: ResolvedFaceMetrics,
) -> Result<ResolvedFontId, FontPreparationError> {
    let fingerprint = resolved_identity_fingerprint(
        bytes,
        face_index,
        style,
        source_kind,
        leading_resolver,
        &metrics,
    );
    let id = allocate_resolved_font_id(
        bytes,
        family_label,
        face_label,
        face_index,
        style,
        source_kind,
        leading_resolver,
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
            leading_resolver,
            bytes,
            metrics,
        )
        .map_err(|_| FontPreparationError::bounded("resolved-face-invalid"))?;
        faces.insert(id.clone(), face);
        bytes_by_id.insert(id.clone(), Arc::from(bytes));
        indices_by_id.insert(id.clone(), face_index);
        let data_key = renderer_data_key(bytes, face_index);
        let egui_name = renderer_family_name(bytes, face_index);
        family_names.insert(id.clone(), egui_name);
        data_keys_by_id.insert(id.clone(), data_key);
    }
    Ok(id)
}

fn renderer_data_key(bytes: &[u8], face_index: u32) -> String {
    format!(
        "scriptcraft-font-{:016x}-f{face_index}",
        content_fingerprint(bytes)
    )
}

fn renderer_family_name(bytes: &[u8], face_index: u32) -> String {
    format!(
        "ScriptCraft Font {:016x} Face {face_index}",
        content_fingerprint(bytes)
    )
}

fn resolved_identity_fingerprint(
    bytes: &[u8],
    face_index: u32,
    style: ResolvedFaceStyle,
    source_kind: ResolvedFontSourceKind,
    leading_resolver: LeadingResolverKind,
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
    mix(leading_resolver as u8);
    for value in [
        metrics.units_per_em as i64,
        i64::from(metrics.ascender),
        i64::from(metrics.descender),
        i64::from(metrics.line_gap),
        i64::from(metrics.underline.position),
        i64::from(metrics.underline.thickness),
        i64::from(metrics.strikethrough.position),
        i64::from(metrics.strikethrough.thickness),
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
    family_label: &str,
    face_label: &str,
    face_index: u32,
    style: ResolvedFaceStyle,
    source_kind: ResolvedFontSourceKind,
    leading_resolver: LeadingResolverKind,
    metrics: &ResolvedFaceMetrics,
    fingerprint: u64,
    existing: &BTreeMap<ResolvedFontId, Arc<[u8]>>,
    existing_indices: &BTreeMap<ResolvedFontId, u32>,
    existing_faces: &BTreeMap<ResolvedFontId, ResolvedFontFace>,
) -> Result<ResolvedFontId, FontPreparationError> {
    let byte_length =
        u64::try_from(bytes.len()).map_err(|_| FontPreparationError::bounded("resolved-id-bound"))?;
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
                        face.family_label == family_label
                            && face.face_label == face_label
                            && face.byte_length == byte_length
                            && face.content_fingerprint == content_fingerprint(bytes)
                            && face.style == style
                            && face.source_kind == source_kind
                            && face.leading_resolver == leading_resolver
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
