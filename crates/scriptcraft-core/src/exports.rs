//! Explicit, non-canonical export adapters.
//!
//! Both adapters consume the shared fixed layout. They only borrow the
//! project, so exporting cannot change the editing session or its clean state.

use crate::layout::{LayoutError, ScreenplayLayout};
use crate::{Project, Role};

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
    let layout = ScreenplayLayout::build(project)?;
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
    let layout = ScreenplayLayout::build(project)?;
    let mut objects = Vec::<Vec<u8>>::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let page_count = layout.pages.len();
    let mut kids = String::new();
    for index in 0..page_count {
        if !kids.is_empty() {
            kids.push(' ');
        }
        kids.push_str(&(4 + index * 2).to_string());
        kids.push_str(" 0 R");
    }
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>").into_bytes());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>".to_vec());
    for page in &layout.pages {
        let content = page_content(page, layout.profile.paper_height);
        let length = content.len();
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                layout.profile.paper_width,
                layout.profile.paper_height,
                5 + (page.number * 2)
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
    let mut output = String::from("BT\n/F1 10 Tf\n");
    for line in &page.lines {
        let x = line.rect.x;
        let y = paper_height.saturating_sub(line.rect.y + line.rect.height);
        output.push_str(&format!("1 0 0 1 {x} {y} Tm\n("));
        for character in line.text.chars() {
            match character {
                '(' | ')' | '\\' => {
                    output.push('\\');
                    output.push(character);
                }
                c if c.is_ascii_graphic() || c == ' ' => output.push(c),
                _ => output.push('?'),
            }
        }
        output.push_str(") Tj\n");
    }
    output.push_str("ET");
    output.into_bytes()
}

#[allow(dead_code)]
fn role_name(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "scene_heading",
        Role::Action => "action",
        Role::Character => "character",
        Role::Dialogue => "dialogue",
        Role::Parenthetical => "parenthetical",
        Role::Transition => "transition",
        Role::Note => "note",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::EditingSession;

    fn project() -> Project {
        let mut project = Project::new("p", "Title", "Author").unwrap_or_else(|_| unreachable!());
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
    fn pdf_has_signature_deterministic_pages_and_basic_content() {
        let project = project();
        let first = export_pdf(&project).unwrap_or_else(|_| unreachable!());
        let second = export_pdf(&project).unwrap_or_else(|_| unreachable!());
        assert_eq!(first, second);
        assert!(first.starts_with(b"%PDF-1.4"));
        assert!(first.ends_with(b"%%EOF\n"));
        assert!(String::from_utf8_lossy(&first).contains("/Count 1"));
        assert!(String::from_utf8_lossy(&first).contains("INT. ROOM - DAY"));
    }

    #[test]
    fn failed_export_isolated_from_session_state() {
        let mut project = project();
        project.page_profile.paper_width = 1;
        let session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        let before = session.clone();
        assert!(export_pdf(session.project()).is_err());
        assert!(export_text(session.project()).is_err());
        assert_eq!(session, before);
    }
}
