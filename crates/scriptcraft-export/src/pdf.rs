use crate::{
    pdf_document::{PdfDocumentWriter, PdfObjectId, MAX_PDF_OUTPUT_BYTES},
    pdf_font::{
        encode_complete_face, encode_packaged_courier_prime, write_font_objects,
        CompleteFontPlan, MAX_PDF_FONT_RESOURCES,
    },
    pdf_font_policy::{
        assess_host_face, HostFaceDecision, PdfFontByteBudget,
        PdfFontEmbeddingFallbackReason,
    },
    ExportError, PdfFontPayload,
};
use scriptcraft_layout::{
    LayoutComposition, LayoutEnvironment, LayoutLine, LayoutPage, PositionedStyledSpan,
    ScreenplayLayout,
};
use scriptcraft_model::{
    formatting::{FixedPoint, SourceLayoutSettings},
    layout_inputs::LayoutInputs,
    project::Project,
    typography::{
        EffectiveTextColor, FontResolutionKind, ResolvedFontId, ResolvedFontSourceKind,
    },
};
use std::collections::BTreeMap;

const MAX_PDF_REPORT_ENTRIES: usize = 16;

/// Transient PDF composition options. These values are consumed by export only
/// and are never written into the Project or native FDX document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfExportOptions {
    pub include_title_page: bool,
}

/// Font families available in the bounded non-embedded PDF policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PdfBase14Family {
    Courier,
    Helvetica,
    Times,
}

/// A privacy-safe category describing one deliberate output limitation.
/// Entries never contain document text, host paths, or arbitrary family labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfFallbackKind {
    HostFontEmbeddingFallback {
        reason: PdfFontEmbeddingFallbackReason,
    },
    Base14FontSubstitution {
        requested: FontResolutionKind,
        resource: PdfBase14Family,
    },
    UnsupportedScalarSubstitution,
    MissingGlyph,
    UnsupportedExplicitColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfFallbackEntry {
    pub kind: PdfFallbackKind,
    pub count: u32,
}

/// Bounded report for deliberate PDF typography/metric fallbacks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PdfTypographyReport {
    entries: Vec<PdfFallbackEntry>,
    /// Number of fallback occurrences not represented after the entry cap.
    suppressed_count: u32,
}

impl PdfTypographyReport {
    fn record(&mut self, kind: PdfFallbackKind) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.kind == kind) {
            entry.count = entry.count.saturating_add(1);
        } else if self.entries.len() < MAX_PDF_REPORT_ENTRIES {
            self.entries.push(PdfFallbackEntry { kind, count: 1 });
        } else {
            self.suppressed_count = self.suppressed_count.saturating_add(1);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.suppressed_count == 0
    }

    pub fn entries(&self) -> &[PdfFallbackEntry] {
        &self.entries
    }

    pub const fn suppressed_count(&self) -> u32 {
        self.suppressed_count
    }

    /// Produces a bounded summary containing categories and counts only.
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "PDF typography fallback: none".to_owned();
        }
        let mut parts = Vec::with_capacity(self.entries.len().saturating_add(1));
        for entry in &self.entries {
            let label = match entry.kind {
                PdfFallbackKind::HostFontEmbeddingFallback { reason } => match reason {
                    PdfFontEmbeddingFallbackReason::RestrictedLicense => {
                        "host font restricted by embedding license"
                    }
                    PdfFontEmbeddingFallbackReason::PermissionUnavailable => {
                        "host font embedding permission unavailable"
                    }
                    PdfFontEmbeddingFallbackReason::OutlineEmbeddingRestricted => {
                        "host font outline embedding restricted"
                    }
                    PdfFontEmbeddingFallbackReason::UnsupportedContainer => {
                        "host font container unsupported"
                    }
                    PdfFontEmbeddingFallbackReason::UnsupportedOutline => {
                        "host font outline format unsupported"
                    }
                    PdfFontEmbeddingFallbackReason::UnsupportedVariableFont => {
                        "host variable font unsupported"
                    }
                    PdfFontEmbeddingFallbackReason::UnsupportedSource => {
                        "font source not eligible for host embedding"
                    }
                    PdfFontEmbeddingFallbackReason::InvalidFont => {
                        "host font metadata or name invalid"
                    }
                    PdfFontEmbeddingFallbackReason::FaceByteLimit => {
                        "host font exceeds per-face byte limit"
                    }
                    PdfFontEmbeddingFallbackReason::AggregateByteBudget => {
                        "host fonts exceed aggregate byte budget"
                    }
                },
                PdfFallbackKind::Base14FontSubstitution {
                    requested,
                    resource,
                } => match (requested, resource) {
                    (FontResolutionKind::CourierAlias, PdfBase14Family::Courier) => {
                        "Courier request to Base-14 Courier"
                    }
                    (FontResolutionKind::NearestSans, PdfBase14Family::Helvetica) => {
                        "sans request to Base-14 Helvetica"
                    }
                    (FontResolutionKind::NearestSerif, PdfBase14Family::Times) => {
                        "serif request to Base-14 Times"
                    }
                    (FontResolutionKind::UnknownFallback, PdfBase14Family::Courier) => {
                        "unknown-family fallback to Base-14 Courier"
                    }
                    (FontResolutionKind::ExactHost, PdfBase14Family::Helvetica) => {
                        "exact host family to Base-14 Helvetica generic resource (not embedded)"
                    }
                    (FontResolutionKind::ExactHost, PdfBase14Family::Courier) => {
                        "exact host family to Base-14 Courier generic resource (not embedded)"
                    }
                    (FontResolutionKind::ExactHost, PdfBase14Family::Times) => {
                        "exact host family to Base-14 Times generic resource (not embedded)"
                    }
                    (FontResolutionKind::CourierAlias, PdfBase14Family::Helvetica) => {
                        "Courier request to Base-14 Helvetica"
                    }
                    (FontResolutionKind::CourierAlias, PdfBase14Family::Times) => {
                        "Courier request to Base-14 Times"
                    }
                    (FontResolutionKind::NearestSans, PdfBase14Family::Courier) => {
                        "sans request to Base-14 Courier"
                    }
                    (FontResolutionKind::NearestSans, PdfBase14Family::Times) => {
                        "sans request to Base-14 Times"
                    }
                    (FontResolutionKind::NearestSerif, PdfBase14Family::Courier) => {
                        "serif request to Base-14 Courier"
                    }
                    (FontResolutionKind::NearestSerif, PdfBase14Family::Helvetica) => {
                        "serif request to Base-14 Helvetica"
                    }
                    (FontResolutionKind::UnknownFallback, PdfBase14Family::Helvetica) => {
                        "unknown-family fallback to Base-14 Helvetica"
                    }
                    (FontResolutionKind::UnknownFallback, PdfBase14Family::Times) => {
                        "unknown-family fallback to Base-14 Times"
                    }
                },
                PdfFallbackKind::UnsupportedScalarSubstitution => "non-ASCII scalar substitution",
                PdfFallbackKind::MissingGlyph => "embedded missing glyph",
                PdfFallbackKind::UnsupportedExplicitColor => "unsupported color fallback",
            };
            parts.push(format!("{label}: {}", entry.count));
        }
        if self.suppressed_count > 0 {
            parts.push(format!(
                "additional fallback occurrences: {}",
                self.suppressed_count
            ));
        }
        format!("PDF bounded fallback report ({})", parts.join(", "))
    }
}

/// Owned PDF bytes and their bounded typography/metric fallback report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfExport {
    pub bytes: Vec<u8>,
    pub typography: PdfTypographyReport,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PdfFontKey {
    Embedded(ResolvedFontId),
    Base14(PdfBase14Family, bool, bool),
}

#[derive(Debug, Clone, Copy)]
struct PdfFontResource {
    name_index: usize,
    object_id: PdfObjectId,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            include_title_page: true,
        }
    }
}

/// Produces a deterministic PDF from Print layout and the matching exact-face
/// payload. Packaged Courier Prime is mandatory; host/synthetic faces use full
/// embedding only when the explicit permission, format, metric, and byte policy allows it.
pub fn export_pdf(
    project: &Project,
    environment: &LayoutEnvironment,
    font_payload: &PdfFontPayload,
) -> Result<PdfExport, ExportError> {
    let inputs = project.session_layout_inputs();
    export_pdf_with_layout_inputs_and_options(
        project,
        &inputs,
        environment,
        font_payload,
        PdfExportOptions::default(),
    )
}

pub fn export_pdf_with_layout_inputs(
    project: &Project,
    inputs: &LayoutInputs,
    environment: &LayoutEnvironment,
    font_payload: &PdfFontPayload,
) -> Result<PdfExport, ExportError> {
    export_pdf_with_layout_inputs_and_options(
        project,
        inputs,
        environment,
        font_payload,
        PdfExportOptions::default(),
    )
}

/// Produces a PDF using the selected title-page composition and shared Print
/// layout. The borrowed Project and layout inputs are never mutated.
pub fn export_pdf_with_options(
    project: &Project,
    environment: &LayoutEnvironment,
    font_payload: &PdfFontPayload,
    options: PdfExportOptions,
) -> Result<PdfExport, ExportError> {
    let inputs = project.session_layout_inputs();
    export_pdf_with_layout_inputs_and_options(
        project,
        &inputs,
        environment,
        font_payload,
        options,
    )
}

pub fn export_pdf_with_layout_inputs_and_options(
    project: &Project,
    inputs: &LayoutInputs,
    environment: &LayoutEnvironment,
    font_payload: &PdfFontPayload,
    options: PdfExportOptions,
) -> Result<PdfExport, ExportError> {
    if inputs.page_only() != environment.page {
        return Err(ExportError::InvalidTypographyEnvironment);
    }
    let source_settings = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_else(SourceLayoutSettings::default);
    let layout = ScreenplayLayout::build_with_composition_and_environment(
        project,
        inputs,
        &source_settings,
        LayoutComposition::Print {
            include_title_page: options.include_title_page,
        },
        environment,
    )?;
    let mut report = PdfTypographyReport::default();
    let mut packaged_scalars = BTreeMap::<ResolvedFontId, std::collections::BTreeSet<char>>::new();
    let mut host_scalars = BTreeMap::<ResolvedFontId, std::collections::BTreeSet<char>>::new();
    let mut host_fallback_candidates = BTreeMap::<
        ResolvedFontId,
        Vec<(FontResolutionKind, PdfBase14Family, bool, bool)>,
    >::new();
    for page in &layout.pages {
        for line in &page.lines {
            for span in line.spans.iter() {
                let style = &span.resolved_text_style;
                if style.hidden_text || span.display_text.is_empty() {
                    continue;
                }
                let face = environment
                    .fonts
                    .require_face(&style.resolved_font_id)
                    .map_err(|_| ExportError::InvalidTypographyEnvironment)?;
                font_payload.require_coherent_face(&style.resolved_font_id, face)?;
                if face.source_kind == ResolvedFontSourceKind::Bundled {
                    packaged_scalars
                        .entry(style.resolved_font_id.clone())
                        .or_default()
                        .extend(span.display_text.chars());
                } else {
                    let requested = environment
                        .fonts
                        .resolution_for(&style.declared_family)
                        .ok_or(ExportError::InvalidTypographyEnvironment)?;
                    let resource_family = match requested.kind {
                        FontResolutionKind::CourierAlias | FontResolutionKind::UnknownFallback => {
                            PdfBase14Family::Courier
                        }
                        FontResolutionKind::NearestSans | FontResolutionKind::ExactHost => {
                            PdfBase14Family::Helvetica
                        }
                        FontResolutionKind::NearestSerif => PdfBase14Family::Times,
                    };
                    host_scalars
                        .entry(style.resolved_font_id.clone())
                        .or_default()
                        .extend(span.display_text.chars());
                    host_fallback_candidates
                        .entry(style.resolved_font_id.clone())
                        .or_default()
                        .push((requested.kind, resource_family, style.bold, style.italic));
                }
                if matches!(style.color, EffectiveTextColor::UnsupportedExplicit) {
                    report.record(PdfFallbackKind::UnsupportedExplicitColor);
                }
            }
        }
    }

    let mut byte_budget = PdfFontByteBudget::default();
    for id in packaged_scalars.keys() {
        let entry = font_payload
            .face(id)
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        byte_budget.reserve_packaged(entry.bytes.len())?;
    }
    let mut plans = BTreeMap::<ResolvedFontId, CompleteFontPlan>::new();
    for (id, scalars) in &packaged_scalars {
        let entry = font_payload
            .face(id)
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        plans.insert(id.clone(), encode_packaged_courier_prime(entry, scalars)?);
    }
    let mut host_embedded_ids = std::collections::BTreeSet::new();
    let mut host_fallback_ids = std::collections::BTreeSet::new();
    for (id, scalars) in &host_scalars {
        let entry = font_payload
            .face(id)
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        let decision = assess_host_face(entry, scalars)?;
        let (face, postscript_name) = match decision {
            HostFaceDecision::Fallback(reason) => {
                report.record(PdfFallbackKind::HostFontEmbeddingFallback { reason });
                host_fallback_ids.insert(id.clone());
                continue;
            }
            HostFaceDecision::Embed {
                face,
                postscript_name,
            } => (face, postscript_name),
        };
        if let Some(reason) = byte_budget.reserve_host(entry.bytes.len())? {
            report.record(PdfFallbackKind::HostFontEmbeddingFallback { reason });
            host_fallback_ids.insert(id.clone());
            continue;
        }
        let plan = encode_complete_face(
            &face,
            &postscript_name,
            &entry.face.metrics,
            &entry.bytes,
            scalars,
        )?;
        plans.insert(id.clone(), plan);
        host_embedded_ids.insert(id.clone());
    }

    let mut fallback_fonts = BTreeMap::<(PdfBase14Family, bool, bool), ()>::new();
    for id in &host_fallback_ids {
        if let Some(candidates) = host_fallback_candidates.get(id) {
            for (_, family, bold, italic) in candidates {
                fallback_fonts.insert((*family, *bold, *italic), ());
            }
        }
    }
    let font_resource_count = plans
        .len()
        .checked_add(fallback_fonts.len())
        .ok_or(ExportError::OutputResourceLimitExceeded)?;
    if font_resource_count > MAX_PDF_FONT_RESOURCES {
        return Err(ExportError::OutputResourceLimitExceeded);
    }

    // Stable allocation order: catalog/pages; packaged faces by resolved ID;
    // eligible hosts by resolved ID; actual Base-14 family/style keys; pages.
    let mut writer = PdfDocumentWriter::default();
    let catalog_id = writer.reserve_object()?;
    let pages_id = writer.reserve_object()?;
    let mut resources = BTreeMap::<PdfFontKey, PdfFontResource>::new();
    let mut name_index = 1usize;
    for id in packaged_scalars.keys() {
        let plan = plans.get(id).ok_or(ExportError::InvalidPdfFontPayload)?;
        let object_id = write_font_objects(&mut writer, plan)?;
        resources.insert(
            PdfFontKey::Embedded(id.clone()),
            PdfFontResource { name_index, object_id },
        );
        name_index = name_index
            .checked_add(1)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
    }
    for id in &host_embedded_ids {
        let plan = plans.get(id).ok_or(ExportError::InvalidPdfFontPayload)?;
        let object_id = write_font_objects(&mut writer, plan)?;
        resources.insert(
            PdfFontKey::Embedded(id.clone()),
            PdfFontResource { name_index, object_id },
        );
        name_index = name_index
            .checked_add(1)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
    }
    for (family, bold, italic) in fallback_fonts.keys().copied() {
        let object_id = writer.reserve_object()?;
        let font_name = base14_font_name(family, bold, italic);
        writer.write_object(
            object_id,
            format!("<< /Type /Font /Subtype /Type1 /BaseFont /{font_name} /Encoding /WinAnsiEncoding >>")
                .into_bytes(),
            Vec::new(),
        )?;
        resources.insert(
            PdfFontKey::Base14(family, bold, italic),
            PdfFontResource { name_index, object_id },
        );
        name_index = name_index
            .checked_add(1)
            .ok_or(ExportError::OutputResourceLimitExceeded)?;
    }

    let mut page_ids = Vec::with_capacity(layout.pages.len());
    let mut content_ids = Vec::with_capacity(layout.pages.len());
    for _ in &layout.pages {
        page_ids.push(writer.reserve_object()?);
        content_ids.push(writer.reserve_object()?);
    }

    let catalog = format!("<< /Type /Catalog /Pages {} >>", ref_text(pages_id)).into_bytes();
    writer.write_object(catalog_id, catalog, vec![pages_id])?;
    let kids = page_ids.iter().map(|id| ref_text(*id)).collect::<Vec<_>>().join(" ");
    let pages = format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", page_ids.len())
        .into_bytes();
    writer.write_object(pages_id, pages, page_ids.clone())?;

    for (page_index, page) in layout.pages.iter().enumerate() {
        let content = page_content(
            page,
            layout.inputs.paper_height,
            &resources,
            &plans,
            environment,
            font_payload,
            &mut report,
        )?;
        let mut font_dictionary = String::new();
        let mut page_references = vec![pages_id, content_ids[page_index]];
        for resource in resources.values() {
            let name = resource_name(resource.name_index);
            font_dictionary.push_str(&format!(" /{name} {}", ref_text(resource.object_id)));
            page_references.push(resource.object_id);
        }
        let page_body = format!(
            "<< /Type /Page /Parent {} /MediaBox [0 0 {} {}] /Resources << /Font <<{font_dictionary} >> >> /Contents {} >>",
            ref_text(pages_id),
            layout.inputs.paper_width,
            layout.inputs.paper_height,
            ref_text(content_ids[page_index]),
        )
        .into_bytes();
        writer.write_object(page_ids[page_index], page_body, page_references)?;
        writer.write_stream(content_ids[page_index], Vec::new(), content, Vec::new())?;
    }
    let pdf = writer.finish(catalog_id)?;
    Ok(PdfExport {
        bytes: pdf,
        typography: report,
    })
}

fn page_content(
    page: &LayoutPage,
    paper_height: u32,
    fonts: &BTreeMap<PdfFontKey, PdfFontResource>,
    plans: &BTreeMap<ResolvedFontId, CompleteFontPlan>,
    environment: &LayoutEnvironment,
    font_payload: &PdfFontPayload,
    report: &mut PdfTypographyReport,
) -> Result<Vec<u8>, ExportError> {
    let mut output = String::new();
    for line in &page.lines {
        let baseline_pdf_y = pdf_baseline_y(paper_height, line)?;
        for span in line.spans.iter() {
            let style = &span.resolved_text_style;
            if style.hidden_text {
                continue;
            }
            validate_span_geometry(line, span)?;
            validate_scalar_boundaries(span)?;
            if span.display_text.is_empty() {
                continue;
            }
            let x = span.geometry.x;
            let y = baseline_pdf_y;
            let face = environment
                .fonts
                .require_face(&style.resolved_font_id)
                .map_err(|_| ExportError::InvalidTypographyEnvironment)?;
            font_payload.require_coherent_face(&style.resolved_font_id, face)?;
            let font_key = if plans.contains_key(&style.resolved_font_id) {
                PdfFontKey::Embedded(style.resolved_font_id.clone())
            } else {
                let resolution = environment
                    .fonts
                    .resolution_for(&style.declared_family)
                    .ok_or(ExportError::InvalidTypographyEnvironment)?;
                let resource_family = match resolution.kind {
                    FontResolutionKind::CourierAlias | FontResolutionKind::UnknownFallback => {
                        PdfBase14Family::Courier
                    }
                    FontResolutionKind::NearestSans | FontResolutionKind::ExactHost => {
                        PdfBase14Family::Helvetica
                    }
                    FontResolutionKind::NearestSerif => PdfBase14Family::Times,
                };
                report.record(PdfFallbackKind::Base14FontSubstitution {
                    requested: resolution.kind,
                    resource: resource_family,
                });
                PdfFontKey::Base14(resource_family, style.bold, style.italic)
            };
            let resource = fonts
                .get(&font_key)
                .ok_or(ExportError::InvalidTypographyEnvironment)?;
            let resource_name = resource_name(resource.name_index);
            let (red, green, blue) = match style.color {
                EffectiveTextColor::ExplicitRgb8 { red, green, blue } => (red, green, blue),
                EffectiveTextColor::ThemeDefault | EffectiveTextColor::UnsupportedExplicit => {
                    (0, 0, 0)
                }
            };
            let (red, green, blue) = (pdf_color(red), pdf_color(green), pdf_color(blue));
            let font_size = style.size;
            for (index, character) in span.display_text.chars().enumerate() {
                let boundary = span
                    .display_boundary_offsets
                    .get(index)
                    .ok_or(ExportError::InvalidLayoutGeometry)?;
                let glyph_x = FixedPoint::from_raw(
                    x.raw()
                        .checked_add(boundary.raw())
                        .ok_or(ExportError::InvalidLayoutGeometry)?,
                );
                if let PdfFontKey::Embedded(id) = &font_key {
                    let plan = plans.get(id).ok_or(ExportError::InvalidPdfFontPayload)?;
                    let cid = plan
                        .cids
                        .get(&character)
                        .ok_or(ExportError::InvalidPdfFontPayload)?;
                    if plan.missing_scalars.contains(&character) {
                        report.record(PdfFallbackKind::MissingGlyph);
                    }
                    append_content(&mut output, &format!(
                        "BT {red} {green} {blue} rg /{resource_name} {font_size} Tf 1 0 0 1 {} {} Tm <{cid:04X}> Tj ET\n",
                        glyph_x.decimal_string(),
                        y.decimal_string(),
                    ))?;
                } else {
                    let pdf_character = if character.is_ascii() && !character.is_ascii_control() {
                        character
                    } else {
                        report.record(PdfFallbackKind::UnsupportedScalarSubstitution);
                        '?'
                    };
                    append_content(&mut output, &format!(
                        "BT {red} {green} {blue} rg /{resource_name} {font_size} Tf 1 0 0 1 {} {} Tm ({}) Tj ET\n",
                        glyph_x.decimal_string(),
                        y.decimal_string(),
                        pdf_escape_character(pdf_character),
                    ))?;
                }
            }

            if span.geometry.width.raw() == 0 {
                continue;
            }
            let right = FixedPoint::from_raw(
                x.raw()
                    .checked_add(span.geometry.width.raw())
                    .ok_or(ExportError::InvalidLayoutGeometry)?,
            );
            let leading = line.vertical_metrics.leading;
            if span.underline {
                let underline_y = y
                    .raw()
                    .checked_add(ratio_raw(leading, 12)?)
                    .map(FixedPoint::from_raw)
                    .ok_or(ExportError::InvalidLayoutGeometry)?;
                append_content(&mut output, &format!(
                    "q {red} {green} {blue} RG 0.6 w {} {} m {} {} l S Q\n",
                    x.decimal_string(),
                    underline_y.decimal_string(),
                    right.decimal_string(),
                    underline_y.decimal_string()
                ))?;
            }
            if span.strikethrough {
                let strike_y = y
                    .raw()
                    .checked_add(ratio_raw(leading, 48)?)
                    .map(FixedPoint::from_raw)
                    .ok_or(ExportError::InvalidLayoutGeometry)?;
                append_content(&mut output, &format!(
                    "q {red} {green} {blue} RG 0.6 w {} {} m {} {} l S Q\n",
                    x.decimal_string(),
                    strike_y.decimal_string(),
                    right.decimal_string(),
                    strike_y.decimal_string()
                ))?;
            }
        }
    }
    Ok(output.into_bytes())
}

fn append_content(output: &mut String, text: &str) -> Result<(), ExportError> {
    let new_len = output
        .len()
        .checked_add(text.len())
        .ok_or(ExportError::OutputResourceLimitExceeded)?;
    if new_len > MAX_PDF_OUTPUT_BYTES {
        return Err(ExportError::OutputResourceLimitExceeded);
    }
    output.push_str(text);
    Ok(())
}

fn ref_text(id: PdfObjectId) -> String {
    format!("{} 0 R", id.0)
}

fn resource_name(index: usize) -> String {
    format!("F{index}")
}

/// Converts the canonical top-down line-box-bottom baseline to PDF's
/// bottom-up coordinate system without consulting font metrics or glyph ink.
fn pdf_baseline_y(paper_height: u32, line: &LayoutLine) -> Result<FixedPoint, ExportError> {
    let baseline = line.baseline_y();
    let checked_bottom = line
        .geometry
        .y
        .raw()
        .checked_add(line.geometry.height.raw())
        .ok_or(ExportError::InvalidLayoutGeometry)?;
    if line.vertical_metrics.leading.raw() <= 0
        || line.geometry.height != line.vertical_metrics.leading
        || line.paragraph_bounds.y != line.geometry.y
        || line.paragraph_bounds.height != line.geometry.height
        || checked_bottom != baseline.raw()
    {
        return Err(ExportError::InvalidLayoutGeometry);
    }
    let paper_height_raw = i64::from(paper_height)
        .checked_mul(FixedPoint::SCALE)
        .ok_or(ExportError::InvalidLayoutGeometry)?;
    paper_height_raw
        .checked_sub(baseline.raw())
        .map(FixedPoint::from_raw)
        .ok_or(ExportError::InvalidLayoutGeometry)
}

fn validate_span_geometry(
    line: &LayoutLine,
    span: &PositionedStyledSpan,
) -> Result<(), ExportError> {
    let bottom = span
        .geometry
        .y
        .raw()
        .checked_add(span.geometry.height.raw())
        .ok_or(ExportError::InvalidLayoutGeometry)?;
    if span.geometry.y != line.geometry.y
        || span.geometry.height != line.vertical_metrics.leading
        || span.geometry.height != line.geometry.height
        || bottom != line.baseline_y().raw()
        || span.geometry.width.raw() < 0
    {
        return Err(ExportError::InvalidLayoutGeometry);
    }
    Ok(())
}

/// Checks the canonical scalar-origin vector without consulting font widths.
fn validate_scalar_boundaries(span: &PositionedStyledSpan) -> Result<(), ExportError> {
    let scalar_count = span.display_text.chars().count();
    let expected_len = scalar_count
        .checked_add(1)
        .ok_or(ExportError::InvalidLayoutGeometry)?;
    if span.display_boundary_offsets.len() != expected_len
        || span
            .display_boundary_offsets
            .first()
            .map_or(true, |offset| offset.raw() != 0)
        || span
            .display_boundary_offsets
            .last()
            .map_or(true, |offset| offset.raw() != span.geometry.width.raw())
        || span
            .display_boundary_offsets
            .windows(2)
            .any(|pair| pair[0].raw() > pair[1].raw())
    {
        return Err(ExportError::InvalidLayoutGeometry);
    }
    Ok(())
}

/// Rounds a positive percentage of a leading value to the nearest raw
/// hundredth, with ties rounded upward, using checked integer arithmetic.
fn ratio_raw(leading: FixedPoint, numerator: i64) -> Result<i64, ExportError> {
    if leading.raw() <= 0 {
        return Err(ExportError::InvalidLayoutGeometry);
    }
    let scaled = i128::from(leading.raw())
        .checked_mul(i128::from(numerator))
        .and_then(|value| value.checked_add(50))
        .ok_or(ExportError::InvalidLayoutGeometry)?
        / 100;
    i64::try_from(scaled).map_err(|_| ExportError::InvalidLayoutGeometry)
}

fn base14_font_name(family: PdfBase14Family, bold: bool, italic: bool) -> &'static str {
    match (family, bold, italic) {
        (PdfBase14Family::Courier, false, false) => "Courier",
        (PdfBase14Family::Courier, true, false) => "Courier-Bold",
        (PdfBase14Family::Courier, false, true) => "Courier-Oblique",
        (PdfBase14Family::Courier, true, true) => "Courier-BoldOblique",
        (PdfBase14Family::Helvetica, false, false) => "Helvetica",
        (PdfBase14Family::Helvetica, true, false) => "Helvetica-Bold",
        (PdfBase14Family::Helvetica, false, true) => "Helvetica-Oblique",
        (PdfBase14Family::Helvetica, true, true) => "Helvetica-BoldOblique",
        (PdfBase14Family::Times, false, false) => "Times-Roman",
        (PdfBase14Family::Times, true, false) => "Times-Bold",
        (PdfBase14Family::Times, false, true) => "Times-Italic",
        (PdfBase14Family::Times, true, true) => "Times-BoldItalic",
    }
}

fn pdf_color(channel: u8) -> String {
    let thousandths = (u32::from(channel) * 1_000 + 127) / 255;
    format!("{}.{:03}", thousandths / 1_000, thousandths % 1_000)
}

fn pdf_escape_character(character: char) -> String {
    match character {
        '(' | ')' | '\\' => format!("\\{character}"),
        _ => character.to_string(),
    }
}
