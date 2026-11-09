#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::exports::{
    export_pdf, export_pdf_with_layout_inputs_and_options, export_pdf_with_options,
    ExportError, PdfExportOptions,
};
use scriptcraft_core::formatted_text::set_paragraph_alignments;
use scriptcraft_core::layout::{LayoutComposition, ScreenplayLayout};
use scriptcraft_core::{LayoutInputs, ParagraphAlignment, ParagraphTarget, Project, Role, RunAttribute, ScreenplayElement, TextRun};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("PDF fixtures use valid bounded input"))
}

fn fixture() -> Project {
    let mut project = valid(Project::new());
    project.elements.clear();
    project.elements.push(valid(ScreenplayElement::new(
        "pdf-body",
        Role::Action,
        "BODY_ONLY_CHECK",
    )));
    project
}

fn pdf_text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or_else(|_| unreachable!("PDF content is ASCII after substitution"))
}

fn extracted_text(bytes: &[u8]) -> String {
    let mut output = String::new();
    let mut remaining = pdf_text(bytes);
    while let Some(end) = remaining.find(") Tj") {
        let (before_operator, after_operator) = remaining.split_at(end);
        if let Some(start) = before_operator.rfind('(') {
            output.push_str(&before_operator[start + 1..]);
        }
        remaining = &after_operator[4..];
    }
    output
}

fn page_object_count(bytes: &[u8]) -> usize {
    pdf_text(bytes).matches("/Type /Page /Parent").count()
}

#[test]
fn default_wrappers_include_title_and_explicit_options_control_print_page_order() {
    let project = fixture();
    let default_pdf = valid(export_pdf(&project));
    let explicit_default = valid(export_pdf_with_options(&project, PdfExportOptions::default()));
    assert_eq!(default_pdf, explicit_default);

    let body_only = valid(export_pdf_with_options(
        &project,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    assert!(page_object_count(&default_pdf) > page_object_count(&body_only));
    let included_text = extracted_text(&default_pdf);
    let excluded_text = extracted_text(&body_only);
    let title_position = included_text
        .find("SCRIPT TITLE")
        .unwrap_or_else(|| unreachable!("default title template includes the title placeholder"));
    let body_position = included_text
        .find("(BODY_ONLY_CHECK)")
        .unwrap_or_else(|| unreachable!("body fixture is included in Print output"));
    assert!(title_position < body_position);
    assert!(!excluded_text.contains("SCRIPT TITLE"));
    assert!(excluded_text.contains("BODY_ONLY_CHECK"));
}

#[test]
fn pdf_spans_use_run_font_and_owning_underline_and_strikethrough_geometry() {
    let mut project = fixture();
    project.elements[0] = valid(ScreenplayElement::with_runs(
        "pdf-body",
        Role::Action,
        vec![
            TextRun {
                text: "plain ".to_owned(),
                attributes: Vec::new(),
            },
            TextRun {
                text: "styled".to_owned(),
                attributes: vec![RunAttribute {
                    name: "Style".to_owned(),
                    value: "Bold+Underline+Strikethrough".to_owned(),
                }],
            },
        ],
        None,
    ));

    let pdf = valid(export_pdf_with_options(
        &project,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    let text = pdf_text(&pdf);
    assert!(text.contains("BT /F2 "));
    assert!(extracted_text(&pdf).contains("styled"));
    // Both decorations are emitted as independent span-width strokes.
    assert!(text.matches("q 0.6 w").count() >= 2);
}

#[test]
fn justified_pdf_text_commands_use_the_shared_positioned_span_coordinates() {
    let mut project = fixture();
    project.elements[0].text = (0..100)
        .map(|index| format!("word{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    set_paragraph_alignments(
        &mut project,
        &[ParagraphTarget::Body(0)],
        ParagraphAlignment::Justified,
    )
    .unwrap_or_else(|_| unreachable!("valid body paragraph accepts Justified alignment"));

    let inputs = LayoutInputs::default();
    let source = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_default();
    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &source,
        LayoutComposition::Print {
            include_title_page: false,
        },
    ));
    let pdf = valid(export_pdf_with_layout_inputs_and_options(
        &project,
        &inputs,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    let text = pdf_text(&pdf);

    let wrapped_lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.spans.len() > 1);
    let mut observed = false;
    let final_line = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .last()
        .unwrap_or_else(|| unreachable!("nonempty body fixture has a final line"));
    assert_eq!(final_line.geometry.x, final_line.paragraph_bounds.x);
    for line in wrapped_lines {
        for span in line.spans.iter() {
            let y = scriptcraft_core::FixedPoint::from_points(i64::from(inputs.paper_height)).raw()
                - span.geometry.y.raw()
                - span.geometry.height.raw();
            let command = format!(
                "1 0 0 1 {} {} Tm",
                span.geometry.x.decimal_string(),
                scriptcraft_core::FixedPoint::from_raw(y).decimal_string(),
            );
            assert!(text.contains(&command));
            observed = true;
        }
    }
    assert!(observed);
}

#[test]
fn pdf_export_borrows_project_without_changing_semantic_state() {
    let project = fixture();
    let before = project.clone();
    let _ = valid(export_pdf_with_options(
        &project,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    assert_eq!(project, before);
}

#[test]
fn non_ascii_pdf_substitution_remains_the_documented_question_mark_behavior() {
    let mut project = fixture();
    project.elements[0].text = "café".to_owned();
    let pdf = valid(export_pdf_with_options(
        &project,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    assert!(extracted_text(&pdf).contains("caf?"));
}

#[test]
fn pdf_layout_export_api_accepts_only_the_borrowed_project_inputs_and_title_option() {
    let _: fn(&Project, &LayoutInputs, PdfExportOptions) -> Result<Vec<u8>, ExportError> =
        export_pdf_with_layout_inputs_and_options;
    let options = PdfExportOptions {
        include_title_page: false,
    };
    assert!(!options.include_title_page);
}
