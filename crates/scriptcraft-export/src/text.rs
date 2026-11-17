use crate::ExportError;
use scriptcraft_layout::{LayoutLine, LayoutPurpose, ScreenplayLayout};
use scriptcraft_model::{element::Role, layout_inputs::LayoutInputs, project::Project};

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
    // Print pages place the two columns in parallel. An oversized dual block
    // can therefore put right-side lines on an earlier page than the final
    // left-side line. Plain text is intentionally linear: retain title-page
    // order, then emit body lines by their real flat member/source order.
    let mut title_lines = Vec::<&LayoutLine>::new();
    let mut body_lines = Vec::<&LayoutLine>::new();
    for page in &layout.pages {
        let destination = if page.title_page {
            &mut title_lines
        } else {
            &mut body_lines
        };
        destination.extend(page.lines.iter());
    }
    body_lines.sort_by_key(|line| (line.element, line.source.start, line.source.end));
    for line in title_lines.into_iter().chain(body_lines) {
        text.push_str(&line.display_text);
        text.push('\n');
    }
    Ok(TextExport { text, loss })
}

#[allow(dead_code)]
fn role_name(role: Role) -> &'static str {
    role.as_str()
}
