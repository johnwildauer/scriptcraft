#![forbid(unsafe_code)]

use scriptcraft_export::{
    export_pdf, export_pdf_with_options, export_text, ExportError, PdfExportOptions,
    PdfFallbackKind,
};
use scriptcraft_layout::LayoutEnvironment;
use scriptcraft_model::{
    formatting::{FixedPoint, ParagraphAlignment},
    typography::{
        DocumentFontEnvironment, FaceStyle, FontResolutionKind, RequestedFontResolution,
        ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
        ScalarAdvance,
    },
    BodyOwner, ElementBehavior, ElementDefinition, ElementFontSpec, ElementOutline,
    ElementParagraphSpec, ElementTypeKey, Project, ScreenplayElement, TitlePageParagraph,
};
use std::collections::BTreeSet;

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

fn synthetic_layout_environment(project: &Project) -> LayoutEnvironment {
    let ids = FaceStyle::ALL.map(|style| {
        let label = match style {
            FaceStyle::Regular => "regular",
            FaceStyle::Bold => "bold",
            FaceStyle::Italic => "italic",
            FaceStyle::BoldItalic => "bold-italic",
        };
        valid(ResolvedFontId::new(format!("synthetic-{label}")))
    });
    let mut scalars = (32_u8..=126).map(char::from).collect::<BTreeSet<_>>();
    for owner in &project.elements {
        let element = owner.as_element();
        for character in element
            .text
            .chars()
            .chain(element.runs.iter().flat_map(|run| run.text.chars()))
        {
            scalars.insert(character);
            scalars.extend(character.to_uppercase());
        }
    }
    for paragraph in &project.title_page {
        for character in paragraph
            .text
            .chars()
            .chain(paragraph.runs.iter().flat_map(|run| run.text.chars()))
        {
            scalars.insert(character);
            scalars.extend(character.to_uppercase());
        }
    }
    scalars.insert('\u{fffd}');
    let advances = scalars
        .into_iter()
        .map(|character| {
            (
                character,
                ScalarAdvance {
                    advance_units: 600,
                    missing_glyph: character == '\u{fffd}',
                },
            )
        })
        .collect();
    let metrics = valid(ResolvedFaceMetrics::new(1_000, 800, -200, 0, advances));
    let faces = FaceStyle::ALL
        .into_iter()
        .zip(ids.iter())
        .map(|(style, id)| {
            let face = valid(ResolvedFontFace::new(
                id.clone(),
                "Synthetic",
                "Synthetic face",
                style,
                ResolvedFontSourceKind::Synthetic,
                1,
                1,
                metrics.clone(),
            ));
            (id.clone(), face)
        })
        .collect();
    let families = [
        ("Courier Prime", FontResolutionKind::CourierAlias),
        ("Courier Final Draft", FontResolutionKind::CourierAlias),
        ("Courier", FontResolutionKind::CourierAlias),
        ("Courier New", FontResolutionKind::CourierAlias),
        ("Arial", FontResolutionKind::NearestSans),
        ("Helvetica", FontResolutionKind::NearestSans),
        ("Times New Roman", FontResolutionKind::NearestSerif),
        ("Times", FontResolutionKind::NearestSerif),
        ("Unknown family", FontResolutionKind::UnknownFallback),
    ];
    let requests = families
        .into_iter()
        .map(|(family, kind)| {
            (
                family.to_owned(),
                RequestedFontResolution {
                    kind,
                    regular: ids[0].clone(),
                    bold: ids[1].clone(),
                    italic: ids[2].clone(),
                    bold_italic: ids[3].clone(),
                },
            )
        })
        .collect();
    let fonts = valid(DocumentFontEnvironment::new(requests, faces));
    LayoutEnvironment::new(project.session_layout_inputs().page_only(), fonts)
}

#[test]
fn export_public_api_smoke() {
    let mut project = valid(Project::empty_projection());
    valid(
        project.replace_element_definitions(vec![valid(ElementDefinition::new(
            valid(ElementTypeKey::new("Action")),
            ElementFontSpec {
                family: "Courier Prime".to_owned(),
                size: 12,
                bold: false,
                italic: false,
                underline: false,
                all_caps: false,
                hidden_text: false,
                color: None,
                adornment_style: 0,
                revision_id: 0,
            },
            ElementParagraphSpec {
                alignment: ParagraphAlignment::Left,
                first_indent: FixedPoint::default(),
                left_indent: FixedPoint::default(),
                right_indent: FixedPoint::default(),
                leading: "Regular".to_owned(),
                space_before: FixedPoint::default(),
                spacing: 1,
                starts_new_page: false,
            },
            ElementBehavior {
                paginate_as: None,
                return_key: None,
                shortcut: None,
                tab_key: None,
                win_shortcut: None,
            },
            ElementOutline {
                can_hide: false,
                level: 0,
            },
        ))]),
    );
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "export-action",
        scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
        "Export body smoke café",
    )))];
    project.title_page = vec![valid(TitlePageParagraph::new(
        "export-title",
        "Export Title Smoke",
        Default::default(),
    ))];
    project.preservation = None;
    let original = project.clone();
    let environment = synthetic_layout_environment(&project);

    let plain = valid(export_text(&project));
    assert!(plain.text.contains("Export body smoke"));
    assert!(plain.text.contains("Export Title Smoke"));
    for wrapper in ["<FinalDraft", "<Content", "<Paragraph", "LayoutComposition"] {
        assert!(!plain.text.contains(wrapper));
    }
    assert!(plain.loss.page_layout_lost);

    let default_pdf = valid(export_pdf(&project, &environment));
    let body_only_pdf = valid(export_pdf_with_options(
        &project,
        &environment,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    for pdf in [&default_pdf.bytes, &body_only_pdf.bytes] {
        assert!(!pdf.is_empty());
        assert!(pdf.starts_with(b"%PDF-1.4\n"));
        let text = valid(std::str::from_utf8(pdf));
        assert!(text
            .trim_end_matches(|character: char| character.is_ascii_whitespace())
            .ends_with("%%EOF"));
    }

    let default_observed = observed_pdf_text(&default_pdf.bytes);
    let body_only_observed = observed_pdf_text(&body_only_pdf.bytes);
    assert!(default_observed.contains("Export Title Smoke"));
    assert!(default_observed.contains("Export body smoke caf?"));
    assert!(body_only_observed.contains("Export body smoke caf?"));
    assert!(!body_only_observed.contains("Export Title Smoke"));
    assert_ne!(default_pdf.bytes, body_only_pdf.bytes);
    assert!(!default_pdf
        .typography
        .summary()
        .contains("canonical metric geometry unverified"));
    assert!(default_pdf
        .typography
        .entries()
        .iter()
        .any(|entry| matches!(entry.kind, PdfFallbackKind::UnsupportedScalarSubstitution)));
    assert!(default_pdf.typography.entries().len() <= 8);
    assert!(!default_pdf
        .typography
        .summary()
        .contains("Export body smoke"));
    let mut mismatched_environment = environment.clone();
    mismatched_environment.page.margin_top =
        mismatched_environment.page.margin_top.saturating_add(1);
    assert!(matches!(
        export_pdf(&project, &mismatched_environment),
        Err(ExportError::InvalidTypographyEnvironment)
    ));
    assert_eq!(project, original);
}
