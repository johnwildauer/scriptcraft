#![forbid(unsafe_code)]

use scriptcraft_export::{export_pdf, export_pdf_with_options, export_text, PdfExportOptions};
use scriptcraft_model::{BodyOwner, Project, Role, ScreenplayElement, TitlePageParagraph};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("export smoke scenario is valid"))
}

/// Observes only this exporter's known ASCII `(...) Tj` output; this is not a general PDF parser.
fn observed_pdf_text(bytes: &[u8]) -> String {
    let text = valid(std::str::from_utf8(bytes));
    let mut output = String::new();
    let mut remaining = text;
    while let Some(end) = remaining.find(") Tj") {
        let (command, rest) = remaining.split_at(end);
        if let Some(start) = command.rfind('(') {
            output.push_str(&command[start + 1..]);
        }
        remaining = &rest[4..];
    }
    output
}

#[test]
fn export_public_api_smoke() {
    let mut project = valid(Project::new());
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "export-action",
        Role::Action,
        "Export body smoke",
    )))];
    project.title_page = vec![valid(TitlePageParagraph::new(
        "export-title",
        "Export Title Smoke",
        Default::default(),
    ))];
    let original = project.clone();

    let plain = valid(export_text(&project));
    assert!(plain.text.contains("Export body smoke"));
    assert!(plain.text.contains("Export Title Smoke"));
    for wrapper in ["<FinalDraft", "<Content", "<Paragraph", "LayoutComposition"] {
        assert!(!plain.text.contains(wrapper));
    }
    assert!(plain.loss.page_layout_lost);

    let default_pdf = valid(export_pdf(&project));
    let body_only_pdf = valid(export_pdf_with_options(
        &project,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    for pdf in [&default_pdf, &body_only_pdf] {
        assert!(!pdf.is_empty());
        assert!(pdf.starts_with(b"%PDF-1.4\n"));
        let text = valid(std::str::from_utf8(pdf));
        assert!(text
            .trim_end_matches(|character: char| character.is_ascii_whitespace())
            .ends_with("%%EOF"));
    }

    let default_observed = observed_pdf_text(&default_pdf);
    let body_only_observed = observed_pdf_text(&body_only_pdf);
    assert!(default_observed.contains("Export Title Smoke"));
    assert!(default_observed.contains("Export body smoke"));
    assert!(body_only_observed.contains("Export body smoke"));
    assert!(!body_only_observed.contains("Export Title Smoke"));
    assert_ne!(default_pdf, body_only_pdf);
    assert_eq!(project, original);
}
