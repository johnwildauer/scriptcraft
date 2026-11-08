//! Explicit, non-canonical export adapters.
//!
//! Both adapters consume the shared fixed layout. They only borrow the
//! project, so exporting cannot change the editing session or its clean state.

use crate::layout::{LayoutComposition, LayoutError, LayoutPurpose, ScreenplayLayout};
use crate::{FixedPoint, LayoutInputs, Project, Role};

/// Transient PDF composition options. These values are consumed by export only
/// and are never written into the Project or native FDX document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfExportOptions {
    pub include_title_page: bool,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            include_title_page: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossReport {
    pub roles_lost: usize,
    pub ids_lost: usize,
    pub page_layout_lost: bool,
    pub message: &'static str,
}

impl LossReport {
    fn for_project(project: &Project) -> Self {
        let count = project.title_page.len() + project.elements.len();
        Self {
            roles_lost: count,
            ids_lost: count,
            page_layout_lost: true,
            message: "LOSSY EXPORT: roles, stable IDs, and page layout are not preserved; this text is not a native-project round trip.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextExport {
    pub text: String,
    pub loss: LossReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Layout(LayoutError),
}

impl From<LayoutError> for ExportError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

/// Produces an intentionally lossy, human-readable text copy and its loss contract.
pub fn export_text(project: &Project) -> Result<TextExport, ExportError> {
    let inputs = project.session_layout_inputs();
    export_text_with_layout_inputs(project, &inputs)
}

pub fn export_text_with_layout_inputs(
    project: &Project,
    inputs: &LayoutInputs,
) -> Result<TextExport, ExportError> {
    let layout =
        ScreenplayLayout::build_with_inputs_for_purpose(project, inputs, LayoutPurpose::Print)?;
    let loss = LossReport::for_project(project);
    let mut text = String::new();
    text.push_str("# ScriptCraft plain-text export\n# ");
    text.push_str(loss.message);
    text.push('\n');
    text.push_str("# Fixed-layout pages: ");
    text.push_str(&layout.pages.len().to_string());
    text.push_str("\n\n");
    for page in &layout.pages {
        for line in &page.lines {
            text.push_str(&line.display_text);
            text.push('\n');
        }
    }
    Ok(TextExport { text, loss })
}

/// Produces a small deterministic PDF 1.4 file from the shared print layout.
/// The PDF is an output consumer, never a replacement for the native project.
pub fn export_pdf(project: &Project) -> Result<Vec<u8>, ExportError> {
    let inputs = project.session_layout_inputs();
    export_pdf_with_layout_inputs_and_options(project, &inputs, PdfExportOptions::default())
}

pub fn export_pdf_with_layout_inputs(
    project: &Project,
    inputs: &LayoutInputs,
) -> Result<Vec<u8>, ExportError> {
    export_pdf_with_layout_inputs_and_options(project, inputs, PdfExportOptions::default())
}

/// Produces a PDF using the selected title-page composition and shared Print
/// layout. The borrowed Project and layout inputs are never mutated.
pub fn export_pdf_with_options(
    project: &Project,
    options: PdfExportOptions,
) -> Result<Vec<u8>, ExportError> {
    let inputs = project.session_layout_inputs();
    export_pdf_with_layout_inputs_and_options(project, &inputs, options)
}

pub fn export_pdf_with_layout_inputs_and_options(
    project: &Project,
    inputs: &LayoutInputs,
    options: PdfExportOptions,
) -> Result<Vec<u8>, ExportError> {
    let source_settings = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings.clone())
        .unwrap_or_default();
    let layout = ScreenplayLayout::build_with_composition(
        project,
        inputs,
        &source_settings,
        LayoutComposition::Print {
            include_title_page: options.include_title_page,
        },
    )?;
    let mut objects = Vec::<Vec<u8>>::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let page_count = layout.pages.len();
    let mut kids = String::new();
    for index in 0..page_count {
        if !kids.is_empty() {
            kids.push(' ');
        }
        kids.push_str(&(7 + index * 2).to_string());
        kids.push_str(" 0 R");
    }
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>").into_bytes());
    for font_name in [
        "Courier",
        "Courier-Bold",
        "Courier-Oblique",
        "Courier-BoldOblique",
    ] {
        objects
            .push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /{font_name} >>").into_bytes());
    }
    for page in &layout.pages {
        let content = page_content(page, layout.inputs.paper_height);
        let length = content.len();
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R >> >> /Contents {} 0 R >>",
                layout.inputs.paper_width,
                layout.inputs.paper_height,
                8 + (page.number * 2)
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
    Ok(pdf)
}

fn page_content(page: &crate::layout::LayoutPage, paper_height: u32) -> Vec<u8> {
    let mut output = String::new();
    for line in &page.lines {
        for span in line.spans.iter() {
            let x = span.geometry.x;
            let y = FixedPoint::from_points(i64::from(paper_height))
                .raw()
                .saturating_sub(span.geometry.y.raw())
                .saturating_sub(span.geometry.height.raw());
            let font_resource = match (span.text_style.bold, span.text_style.italic) {
                (true, true) => "F4",
                (true, false) => "F2",
                (false, true) => "F3",
                (false, false) => "F1",
            };
            let font_size = span.text_style.size;
            let horizontal_scale = pdf_fixed_cell_scale(span.char_width, font_size);
            output.push_str(&format!(
                "BT /{font_resource} {font_size} Tf {horizontal_scale} Tz\n1 0 0 1 {} {} Tm\n(",
                x.decimal_string(),
                FixedPoint::from_raw(y).decimal_string()
            ));
            for character in span.display_text.chars() {
                match character {
                    '(' | ')' | '\\' => {
                        output.push('\\');
                        output.push(character);
                    }
                    c if c.is_ascii_graphic() || c == ' ' => output.push(c),
                    // TODO: Preserve non-ASCII text in PDF output instead of substituting it.
                    _ => output.push('?'),
                }
            }
            output.push_str(") Tj\nET\n");

            if span.geometry.width.raw() <= 0 {
                continue;
            }
            let right = FixedPoint::from_raw(x.raw().saturating_add(span.geometry.width.raw()));
            if span.underline {
                let underline_y = FixedPoint::from_raw(
                    y.saturating_add((i64::from(font_size) * 12).div_euclid(100)),
                );
                output.push_str(&format!(
                    "q 0.6 w {} {} m {} {} l S Q\n",
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
                    "q 0.6 w {} {} m {} {} l S Q\n",
                    x.decimal_string(),
                    strike_y.decimal_string(),
                    right.decimal_string(),
                    strike_y.decimal_string()
                ));
            }
        }
    }
    output.into_bytes()
}

fn pdf_fixed_cell_scale(char_width: FixedPoint, font_size: u8) -> String {
    let denominator = i64::from(font_size.max(1)).saturating_mul(60);
    let scaled_thousandths = char_width
        .raw()
        .saturating_mul(100_000)
        .saturating_add(denominator / 2)
        .div_euclid(denominator);
    format!(
        "{}.{:03}",
        scaled_thousandths / 1_000,
        scaled_thousandths % 1_000
    )
}

#[allow(dead_code)]
fn role_name(role: Role) -> &'static str {
    role.as_str()
}
