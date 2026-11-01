//! Explicit, non-canonical export adapters.
//!
//! Both adapters consume the shared fixed layout. They only borrow the
//! project, so exporting cannot change the editing session or its clean state.

use crate::layout::{LayoutError, LayoutPurpose, ScreenplayLayout};
use crate::{FixedPoint, LayoutInputs, Project, Role};

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
    let layout = ScreenplayLayout::build_with_inputs_for_purpose(
        project,
        inputs,
        LayoutPurpose::Print,
    )?;
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
            text.push_str(&line.text);
            text.push('\n');
        }
    }
    Ok(TextExport { text, loss })
}

/// Produces a small deterministic PDF 1.4 file from the shared print layout.
/// The PDF is an output consumer, never a replacement for the native project.
pub fn export_pdf(project: &Project) -> Result<Vec<u8>, ExportError> {
    let inputs = project.session_layout_inputs();
    export_pdf_with_layout_inputs(project, &inputs)
}

pub fn export_pdf_with_layout_inputs(
    project: &Project,
    inputs: &LayoutInputs,
) -> Result<Vec<u8>, ExportError> {
    let layout = ScreenplayLayout::build_with_inputs_for_purpose(
        project,
        inputs,
        LayoutPurpose::Print,
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
        objects.push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /{font_name} >>").into_bytes());
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
        let x = line.geometry.x;
        let y = FixedPoint::from_points(i64::from(paper_height))
            .raw()
            .saturating_sub(line.geometry.y.raw())
            .saturating_sub(line.geometry.height.raw());
        let font_resource = match (line.text_style.bold, line.text_style.italic) {
            (true, true) => "F4",
            (true, false) => "F2",
            (false, true) => "F3",
            (false, false) => "F1",
        };
        let font_size = line.text_style.size;
        let horizontal_scale = pdf_fixed_cell_scale(line.char_width, font_size);
        output.push_str(&format!(
            "BT /{font_resource} {font_size} Tf {horizontal_scale} Tz\n1 0 0 1 {} {} Tm\n(",
            x.decimal_string(),
            FixedPoint::from_raw(y).decimal_string()
        ));
        for character in line.display_text.chars() {
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
        if line.text_style.underline && line.geometry.width.raw() > 0 {
            let underline_y = FixedPoint::from_raw(
                y.saturating_add((i64::from(font_size) * 12).div_euclid(100)),
            );
            let right = FixedPoint::from_raw(x.raw().saturating_add(line.geometry.width.raw()));
            output.push_str(&format!(
                "q 0.6 w {} {} m {} {} l S Q\n",
                x.decimal_string(),
                underline_y.decimal_string(),
                right.decimal_string(),
                underline_y.decimal_string()
            ));
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
    format!("{}.{:03}", scaled_thousandths / 1_000, scaled_thousandths % 1_000)
}

#[allow(dead_code)]
fn role_name(role: Role) -> &'static str {
    role.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::EditingSession;
    use crate::layout::LINE_HEIGHT;

    fn project() -> Project {
        let mut project = Project::from_json(include_str!("../tests/fixtures/legacy_project.json"))
            .unwrap_or_else(|_| unreachable!());
        project.elements = vec![
            crate::ScreenplayElement::new("heading", Role::SceneHeading, "INT. ROOM - DAY")
                .unwrap_or_else(|_| unreachable!()),
            crate::ScreenplayElement::new("dialogue", Role::Dialogue, "Hello (世界)")
                .unwrap_or_else(|_| unreachable!()),
        ];
        project
    }

    #[test]
    fn text_export_reports_all_loss_categories_and_is_deterministic() {
        let project = project();
        let first = export_text(&project).unwrap_or_else(|_| unreachable!());
        let second = export_text(&project).unwrap_or_else(|_| unreachable!());
        assert_eq!(first, second);
        assert_eq!(first.loss.roles_lost, 2);
        assert_eq!(first.loss.ids_lost, 2);
        assert!(first.loss.page_layout_lost);
        assert!(first.text.contains("LOSSY EXPORT"));
        assert!(first.text.contains("roles"));
    }

    #[test]
    fn print_exports_omit_hidden_roles_but_plain_text_keeps_canonical_casing() {
        let imported = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Note'><FontSpec Style='HiddenText'/></ElementSettings><Content><Paragraph Type='Scene Heading' id='heading'><Text>MiXeD title</Text></Paragraph><Paragraph Type='Note' id='note'><Text>private note</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let text = export_text(&imported.project).unwrap_or_else(|_| unreachable!());
        assert!(text.text.contains("MiXeD title"));
        assert!(!text.text.contains("private note"));
        assert!(!text.text.contains("MIXED TITLE"));

        let pdf = export_pdf(&imported.project).unwrap_or_else(|_| unreachable!());
        let pdf = String::from_utf8_lossy(&pdf);
        assert!(pdf.contains("MIXED TITLE"));
        assert!(!pdf.contains("private note"));
    }

    #[test]
    fn pdf_applies_role_style_resources_size_and_separate_underline_geometry() {
        let imported = crate::fdx::read_v6(
            b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='New Act' id='act'><Text>New Act</Text></Paragraph></Content></FinalDraft>",
        )
        .unwrap_or_else(|_| unreachable!());
        let pdf = export_pdf(&imported.project).unwrap_or_else(|_| unreachable!());
        let pdf = String::from_utf8_lossy(&pdf);
        assert!(pdf.contains("BT /F2 12 Tf"));
        assert!(pdf.contains("NEW ACT"));
        assert!(pdf.contains("q 0.6 w"));
        assert!(pdf.contains("/F4 6 0 R"));
    }

    #[test]
    fn pdf_has_signature_deterministic_pages_and_basic_content() {
        let project = project();
        let first = export_pdf(&project).unwrap_or_else(|_| unreachable!());
        let second = export_pdf(&project).unwrap_or_else(|_| unreachable!());
        assert_eq!(first, second);
        assert!(first.starts_with(b"%PDF-1.4"));
        assert!(first.ends_with(b"%%EOF\n"));
        assert!(String::from_utf8_lossy(&first).contains("/Count 1"));
        assert!(String::from_utf8_lossy(&first).contains("INT. ROOM - DAY"));
        let output = String::from_utf8_lossy(&first);
        assert!(output.contains("/BaseFont /Courier"));
        assert!(output.contains("/BaseFont /Courier-Bold"));
        assert!(output.contains("/BaseFont /Courier-Oblique"));
        assert!(output.contains("/BaseFont /Courier-BoldOblique"));
    }

    #[test]
    fn pdf_declares_the_deterministic_standard_courier_style_subset() {
        let project = project();
        let inputs = LayoutInputs {
            font_family: "Courier Prime".into(),
            ..LayoutInputs::default()
        };
        let pdf =
            export_pdf_with_layout_inputs(&project, &inputs).unwrap_or_else(|_| unreachable!());
        let pdf = String::from_utf8_lossy(&pdf);
        assert!(pdf.contains("/BaseFont /Courier"));
        assert!(pdf.contains("/BaseFont /Courier-Bold"));
        assert!(pdf.contains("/BaseFont /Courier-Oblique"));
        assert!(pdf.contains("/BaseFont /Courier-BoldOblique"));
    }

    #[test]
    fn pdf_line_transforms_preserve_layout_positions_and_spacing() {
        let mut project = project();
        let inputs = LayoutInputs {
            font_size: 11,
            ..LayoutInputs::default()
        };
        project.elements.push(
            crate::ScreenplayElement::new("action", Role::Action, "Second line")
                .unwrap_or_else(|_| unreachable!()),
        );
        let layout = ScreenplayLayout::build_with_inputs(&project, &inputs)
            .unwrap_or_else(|_| unreachable!());
        let page = &layout.pages[0];
        let content_bytes = page_content(page, layout.inputs.paper_height);
        let content = String::from_utf8_lossy(&content_bytes);
        assert!(content.contains("BT /F1 12 Tf"));
        assert!(content.contains(" Tz\n"));
        let transforms: Vec<(String, String)> = content
            .lines()
            .filter_map(|line| {
                let values = line
                    .strip_suffix(" Tm")?
                    .split_whitespace()
                    .collect::<Vec<_>>();
                if values.len() == 6 && values[0] == "1" && values[1] == "0" {
                    Some((values[4].to_owned(), values[5].to_owned()))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(transforms.len(), page.lines.len());
        for (line, (x, y)) in page.lines.iter().zip(&transforms) {
            assert_eq!(x, &line.geometry.x.decimal_string());
            assert_eq!(
                y,
                &FixedPoint::from_raw(
                    FixedPoint::from_points(i64::from(layout.inputs.paper_height)).raw()
                        - line.geometry.y.raw()
                        - line.geometry.height.raw()
                )
                .decimal_string()
            );
        }
        let first_y = FixedPoint::parse_points(&transforms[0].1)
            .unwrap_or_default()
            .raw();
        let second_y = FixedPoint::parse_points(&transforms[1].1)
            .unwrap_or_default()
            .raw();
        assert_eq!(first_y - second_y, i64::from(LINE_HEIGHT) * FixedPoint::SCALE);
    }

    #[test]
    fn pdf_emits_fractional_role_coordinates_from_shared_fixed_geometry() {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = vec![
            crate::ScreenplayElement::new("transition", Role::Transition, "CUT")
                .unwrap_or_else(|_| unreachable!()),
        ];
        let layout = ScreenplayLayout::build(&project).unwrap_or_else(|_| unreachable!());
        let line = &layout.pages[0].lines[0];
        assert_eq!(line.geometry.x.decimal_string(), "490.2");
        let pdf = export_pdf(&project).unwrap_or_else(|_| unreachable!());
        assert!(String::from_utf8_lossy(&pdf).contains("1 0 0 1 490.2 708 Tm"));
    }

    #[test]
    fn failed_export_isolated_from_session_state() {
        let project = project();
        let session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        let before = session.clone();
        let inputs = LayoutInputs {
            paper_width: 1,
            ..LayoutInputs::default()
        };
        assert!(export_pdf_with_layout_inputs(session.project(), &inputs).is_err());
        assert!(export_text_with_layout_inputs(session.project(), &inputs).is_err());
        assert_eq!(session, before);
    }
}
