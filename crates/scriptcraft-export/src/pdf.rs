use crate::ExportError;
use scriptcraft_layout::{LayoutComposition, LayoutEnvironment, LayoutPage, ScreenplayLayout};
use scriptcraft_model::{
    formatting::{FixedPoint, SourceLayoutSettings},
    layout_inputs::LayoutInputs,
    project::Project,
    typography::{parse_effective_text_color, EffectiveTextColor, FontResolutionKind},
};
use std::collections::BTreeMap;

const MAX_PDF_REPORT_ENTRIES: usize = 8;

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
    Base14FontSubstitution {
        requested: FontResolutionKind,
        resource: PdfBase14Family,
    },
    UnsupportedScalarSubstitution,
    UnsupportedExplicitColor,
    MetricGeometryUnverified,
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
                PdfFallbackKind::UnsupportedExplicitColor => "unsupported color fallback",
                PdfFallbackKind::MetricGeometryUnverified => "canonical metric geometry unverified",
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

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            include_title_page: true,
        }
    }
}

/// Produces a deterministic PDF from Print layout using an explicit resolved
/// typography environment. The PDF remains noncanonical and reports its
/// bounded Base-14 and metric-policy limitations.
pub fn export_pdf(
    project: &Project,
    environment: &LayoutEnvironment,
) -> Result<PdfExport, ExportError> {
    let inputs = project.session_layout_inputs();
    export_pdf_with_layout_inputs_and_options(
        project,
        &inputs,
        environment,
        PdfExportOptions::default(),
    )
}

pub fn export_pdf_with_layout_inputs(
    project: &Project,
    inputs: &LayoutInputs,
    environment: &LayoutEnvironment,
) -> Result<PdfExport, ExportError> {
    export_pdf_with_layout_inputs_and_options(
        project,
        inputs,
        environment,
        PdfExportOptions::default(),
    )
}

/// Produces a PDF using the selected title-page composition and shared Print
/// layout. The borrowed Project and layout inputs are never mutated.
pub fn export_pdf_with_options(
    project: &Project,
    environment: &LayoutEnvironment,
    options: PdfExportOptions,
) -> Result<PdfExport, ExportError> {
    let inputs = project.session_layout_inputs();
    export_pdf_with_layout_inputs_and_options(project, &inputs, environment, options)
}

pub fn export_pdf_with_layout_inputs_and_options(
    project: &Project,
    inputs: &LayoutInputs,
    environment: &LayoutEnvironment,
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
    // WO-005.4/5's current fixed-cell path is intentionally disclosed. The
    // presence of resolved host metrics does not establish that those metrics
    // produced the current canonical line/page geometry.
    report.record(PdfFallbackKind::MetricGeometryUnverified);

    let mut used_fonts = BTreeMap::<(PdfBase14Family, bool, bool), String>::new();
    for page in &layout.pages {
        for line in &page.lines {
            for span in line.spans.iter() {
                if span.text_style.hidden_text {
                    continue;
                }
                let requested = environment
                    .fonts
                    .resolution_for(&span.text_style.declared_family)
                    .ok_or(ExportError::InvalidTypographyEnvironment)?;
                let resource_family = match requested.kind {
                    FontResolutionKind::CourierAlias | FontResolutionKind::UnknownFallback => {
                        PdfBase14Family::Courier
                    }
                    FontResolutionKind::NearestSans => PdfBase14Family::Helvetica,
                    FontResolutionKind::NearestSerif => PdfBase14Family::Times,
                };
                report.record(PdfFallbackKind::Base14FontSubstitution {
                    requested: requested.kind,
                    resource: resource_family,
                });
                used_fonts.insert(
                    (
                        resource_family,
                        span.text_style.bold,
                        span.text_style.italic,
                    ),
                    String::new(),
                );
                if matches!(
                    parse_effective_text_color(span.text_style.source_color.as_deref()),
                    EffectiveTextColor::UnsupportedExplicit
                ) {
                    report.record(PdfFallbackKind::UnsupportedExplicitColor);
                }
            }
        }
    }

    let mut font_object_ids = BTreeMap::new();
    let mut next_object_id = 3usize;
    for (index, (font_key, resource_name)) in used_fonts.iter_mut().enumerate() {
        *resource_name = format!("F{}", index.saturating_add(1));
        font_object_ids.insert(*font_key, next_object_id);
        next_object_id = next_object_id.saturating_add(1);
    }
    let mut objects = Vec::<Vec<u8>>::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let page_count = layout.pages.len();
    let mut kids = String::new();
    for index in 0..page_count {
        if !kids.is_empty() {
            kids.push(' ');
        }
        kids.push_str(&(next_object_id + index * 2).to_string());
        kids.push_str(" 0 R");
    }
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>").into_bytes());
    for (font_key, _) in &used_fonts {
        let font_name = base14_font_name(font_key.0, font_key.1, font_key.2);
        objects
            .push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /{font_name} >>").into_bytes());
    }
    for (page_index, page) in layout.pages.iter().enumerate() {
        let content = page_content(
            page,
            layout.inputs.paper_height,
            &used_fonts,
            environment,
            &mut report,
        )?;
        let length = content.len();
        let page_object_id = next_object_id + page_index * 2;
        let mut font_dictionary = String::new();
        for (font_key, resource_name) in &used_fonts {
            if let Some(object_id) = font_object_ids.get(font_key) {
                font_dictionary.push_str(&format!(" /{resource_name} {object_id} 0 R"));
            }
        }
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font <<{font_dictionary} >> >> /Contents {} 0 R >>",
                layout.inputs.paper_width,
                layout.inputs.paper_height,
                page_object_id + 1
            )
            .into_bytes(),
        );
        objects.push(
            format!(
                "<< /Length {length} >>\nstream\n{}\nendstream",
                String::from_utf8_lossy(&content)
            )
            .into_bytes(),
        );
    }
    let mut pdf = b"%PDF-1.4\n%ScriptCraft\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    Ok(PdfExport {
        bytes: pdf,
        typography: report,
    })
}

fn page_content(
    page: &LayoutPage,
    paper_height: u32,
    fonts: &BTreeMap<(PdfBase14Family, bool, bool), String>,
    environment: &LayoutEnvironment,
    report: &mut PdfTypographyReport,
) -> Result<Vec<u8>, ExportError> {
    let mut output = String::new();
    for line in &page.lines {
        for span in line.spans.iter() {
            if span.text_style.hidden_text {
                continue;
            }
            let x = span.geometry.x;
            let y = FixedPoint::from_points(i64::from(paper_height))
                .raw()
                .saturating_sub(span.geometry.y.raw())
                .saturating_sub(span.geometry.height.raw());
            let resolution = environment
                .fonts
                .resolution_for(&span.text_style.declared_family)
                .ok_or(ExportError::InvalidTypographyEnvironment)?;
            let resource_family = match resolution.kind {
                FontResolutionKind::CourierAlias | FontResolutionKind::UnknownFallback => {
                    PdfBase14Family::Courier
                }
                FontResolutionKind::NearestSans => PdfBase14Family::Helvetica,
                FontResolutionKind::NearestSerif => PdfBase14Family::Times,
            };
            let requested_font = fonts
                .get(&(
                    resource_family,
                    span.text_style.bold,
                    span.text_style.italic,
                ))
                .ok_or(ExportError::InvalidTypographyEnvironment)?;
            let color = parse_effective_text_color(span.text_style.source_color.as_deref());
            let (red, green, blue) = match color {
                EffectiveTextColor::ExplicitRgb8 { red, green, blue } => (red, green, blue),
                EffectiveTextColor::ThemeDefault | EffectiveTextColor::UnsupportedExplicit => {
                    (0, 0, 0)
                }
            };
            let (red, green, blue) = (pdf_color(red), pdf_color(green), pdf_color(blue));
            let font_size = span.text_style.size;
            let scalar_count = span.display_text.chars().count();
            if span.display_boundary_offsets.len() != scalar_count.saturating_add(1) {
                return Err(ExportError::InvalidLayoutGeometry);
            }
            for (index, character) in span.display_text.chars().enumerate() {
                let boundary = span
                    .display_boundary_offsets
                    .get(index)
                    .ok_or(ExportError::InvalidLayoutGeometry)?;
                let glyph_x = FixedPoint::from_raw(x.raw().saturating_add(boundary.raw()));
                let pdf_character = if character.is_ascii() && !character.is_ascii_control() {
                    character
                } else {
                    report.record(PdfFallbackKind::UnsupportedScalarSubstitution);
                    '?'
                };
                output.push_str(&format!(
                    "BT {red} {green} {blue} rg /{requested_font} {font_size} Tf 1 0 0 1 {} {} Tm ({}) Tj ET\n",
                    glyph_x.decimal_string(),
                    FixedPoint::from_raw(y).decimal_string(),
                    pdf_escape_character(pdf_character),
                ));
            }

            if span.geometry.width.raw() <= 0 {
                continue;
            }
            let right = FixedPoint::from_raw(x.raw().saturating_add(span.geometry.width.raw()));
            if span.underline {
                let underline_y = FixedPoint::from_raw(
                    y.saturating_add((i64::from(font_size) * 12).div_euclid(100)),
                );
                output.push_str(&format!(
                    "q {red} {green} {blue} RG 0.6 w {} {} m {} {} l S Q\n",
                    x.decimal_string(),
                    underline_y.decimal_string(),
                    right.decimal_string(),
                    underline_y.decimal_string()
                ));
            }
            if span.strikethrough {
                let strike_y = FixedPoint::from_raw(
                    y.saturating_add((i64::from(font_size) * 48).div_euclid(100)),
                );
                output.push_str(&format!(
                    "q {red} {green} {blue} RG 0.6 w {} {} m {} {} l S Q\n",
                    x.decimal_string(),
                    strike_y.decimal_string(),
                    right.decimal_string(),
                    strike_y.decimal_string()
                ));
            }
        }
    }
    Ok(output.into_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base14_family_and_style_selection_is_explicit() {
        let expected = [
            (PdfBase14Family::Courier, false, false, "Courier"),
            (PdfBase14Family::Courier, true, false, "Courier-Bold"),
            (PdfBase14Family::Courier, false, true, "Courier-Oblique"),
            (PdfBase14Family::Courier, true, true, "Courier-BoldOblique"),
            (PdfBase14Family::Helvetica, false, false, "Helvetica"),
            (PdfBase14Family::Helvetica, true, false, "Helvetica-Bold"),
            (PdfBase14Family::Helvetica, false, true, "Helvetica-Oblique"),
            (
                PdfBase14Family::Helvetica,
                true,
                true,
                "Helvetica-BoldOblique",
            ),
            (PdfBase14Family::Times, false, false, "Times-Roman"),
            (PdfBase14Family::Times, true, false, "Times-Bold"),
            (PdfBase14Family::Times, false, true, "Times-Italic"),
            (PdfBase14Family::Times, true, true, "Times-BoldItalic"),
        ];
        for (family, bold, italic, name) in expected {
            assert_eq!(base14_font_name(family, bold, italic), name);
        }
    }

    #[test]
    fn fallback_report_is_bounded_and_contains_no_source_values() {
        let mut report = PdfTypographyReport::default();
        for requested in [
            FontResolutionKind::CourierAlias,
            FontResolutionKind::NearestSans,
            FontResolutionKind::NearestSerif,
            FontResolutionKind::UnknownFallback,
        ] {
            for resource in [
                PdfBase14Family::Courier,
                PdfBase14Family::Helvetica,
                PdfBase14Family::Times,
            ] {
                report.record(PdfFallbackKind::Base14FontSubstitution {
                    requested,
                    resource,
                });
            }
        }
        report.record(PdfFallbackKind::MetricGeometryUnverified);
        assert_eq!(report.entries.len(), MAX_PDF_REPORT_ENTRIES);
        assert!(report.suppressed_count > 0);
        let summary = report.summary();
        assert!(summary.contains("PDF bounded fallback report"));
        assert!(!summary.contains("synthetic"));
        assert!(!summary.contains("/Users/"));
    }

    #[test]
    fn rgb_conversion_and_pdf_string_escaping_are_bounded() {
        assert_eq!(pdf_color(0), "0.000");
        assert_eq!(pdf_color(255), "1.000");
        assert_eq!(pdf_escape_character('('), "\\(");
        assert_eq!(pdf_escape_character('\\'), "\\\\");
        assert_eq!(
            parse_effective_text_color(None),
            EffectiveTextColor::ThemeDefault
        );
        assert_eq!(
            parse_effective_text_color(Some("#FFFF00000000")),
            EffectiveTextColor::ExplicitRgb8 {
                red: 255,
                green: 0,
                blue: 0,
            }
        );
        assert_eq!(
            parse_effective_text_color(Some("not-a-color")),
            EffectiveTextColor::UnsupportedExplicit
        );
    }
}
