#![forbid(unsafe_code)]

//! PDF typography policy and canonical-offset output regressions.

use scriptcraft_export::{export_pdf, PdfBase14Family, PdfFallbackKind};
use scriptcraft_layout::{LayoutComposition, LayoutEnvironment, ScreenplayLayout};
use scriptcraft_model::{
    formatting::{FixedPoint, SourceLayoutSettings},
    typography::{
        DocumentFontEnvironment, FaceStyle, FontResolutionKind, RequestedFontResolution,
        ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
        ScalarAdvance,
    },
    BodyOwner, Project, ScreenplayElement,
};
use std::collections::{BTreeMap, BTreeSet};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("synthetic PDF policy input is valid"))
}

fn synthetic_environment(project: &Project) -> LayoutEnvironment {
    let mut families = project
        .element_definitions()
        .iter()
        .map(|definition| definition.font.family.clone())
        .collect::<BTreeSet<_>>();
    families.extend([
        "Courier Prime".to_owned(),
        "Courier Final Draft".to_owned(),
        "Courier".to_owned(),
        "Courier New".to_owned(),
        "Arial".to_owned(),
        "Helvetica".to_owned(),
        "Times New Roman".to_owned(),
        "Times".to_owned(),
        "Futura Exact".to_owned(),
        "Unknown family".to_owned(),
    ]);
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
                    advance_units: match character {
                        ' ' => 300,
                        'i' | 'l' => 250,
                        'W' | 'M' => 900,
                        _ => 560,
                    },
                    missing_glyph: character == '\u{fffd}',
                },
            )
        })
        .collect();
    let metrics = valid(ResolvedFaceMetrics::new(1_000, 800, -200, 0, advances));
    let mut faces = BTreeMap::new();
    let mut requests = BTreeMap::new();
    for (family_index, family) in families.into_iter().enumerate() {
        let ids = FaceStyle::ALL.map(|style| {
            let style_index = match style {
                FaceStyle::Regular => 0,
                FaceStyle::Bold => 1,
                FaceStyle::Italic => 2,
                FaceStyle::BoldItalic => 3,
            };
            valid(ResolvedFontId::new(format!(
                "pdf-{family_index}-{style_index}"
            )))
        });
        for (style, id) in FaceStyle::ALL.into_iter().zip(ids.iter()) {
            faces.insert(
                id.clone(),
                valid(ResolvedFontFace::new(
                    id.clone(),
                    family.clone(),
                    format!("{family} {style:?}"),
                    style,
                    ResolvedFontSourceKind::Synthetic,
                    128,
                    family_index as u64 + 1,
                    metrics.clone(),
                )),
            );
        }
        let kind = if family.to_ascii_lowercase().contains("courier") {
            FontResolutionKind::CourierAlias
        } else if matches!(family.as_str(), "Times" | "Times New Roman") {
            FontResolutionKind::NearestSerif
        } else if family == "Futura Exact" {
            FontResolutionKind::ExactHost
        } else if matches!(family.as_str(), "Arial" | "Helvetica") {
            FontResolutionKind::NearestSans
        } else {
            FontResolutionKind::UnknownFallback
        };
        requests.insert(
            family,
            RequestedFontResolution {
                kind,
                regular: ids[0].clone(),
                bold: ids[1].clone(),
                italic: ids[2].clone(),
                bold_italic: ids[3].clone(),
            },
        );
    }
    let fonts = valid(DocumentFontEnvironment::new(requests, faces));
    let inputs = project.session_layout_inputs();
    LayoutEnvironment::new(inputs.page_only(), fonts)
}

#[test]
fn pdf_fallback_report_is_bounded_privacy_safe_and_non_mutating() {
    let mut project = valid(scriptcraft_fdx::new_project());
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let action = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Action")
        .unwrap_or_else(|| unreachable!("required Action definition exists"));
    action.font.family = "Arial".to_owned();
    action.font.color = Some("private-invalid-color-value".to_owned());
    action.font.bold = true;
    let dialogue = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Dialogue")
        .unwrap_or_else(|| unreachable!("required Dialogue definition exists"));
    dialogue.font.family = "Times New Roman".to_owned();
    dialogue.font.italic = true;
    dialogue.font.color = Some("#123456789ABC".to_owned());
    let scene_heading = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Scene Heading")
        .unwrap_or_else(|| unreachable!("required Scene Heading definition exists"));
    scene_heading.font.family = "Uncatalogued Output Family".to_owned();
    scene_heading.font.bold = true;
    scene_heading.font.italic = true;
    let transition = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Transition")
        .unwrap_or_else(|| unreachable!("required Transition definition exists"));
    transition.font.family = "Futura Exact".to_owned();
    let mut summary = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("required Action definition exists"));
    summary.key = scriptcraft_model::ElementTypeKey::new("Summary").expect("valid fixture label");
    summary.font.hidden_text = true;
    definitions.push(summary);
    valid(project.replace_element_definitions(definitions));
    project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-action",
            scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
            "Output report café — proportional Wiii WWW",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-dialogue",
            scriptcraft_model::ElementTypeKey::new("Dialogue").expect("valid fixture label"),
            "Serif resource and explicit RGB output.",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-scene",
            scriptcraft_model::ElementTypeKey::new("Scene Heading").expect("valid fixture label"),
            "Unknown-family output substitution.",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-exact-host",
            scriptcraft_model::ElementTypeKey::new("Transition").expect("valid fixture label"),
            "Exact host family uses a generic PDF resource.",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-hidden",
            scriptcraft_model::ElementTypeKey::new("Summary").expect("valid fixture label"),
            "PRIVATE HIDDEN FIXTURE CONTENT",
        ))),
    ];
    let original = project.clone();
    let environment = synthetic_environment(&project);
    let pdf = export_pdf(&project, &environment)
        .unwrap_or_else(|error| panic!("synthetic PDF policy input failed: {error:?}"));
    let inputs = project.session_layout_inputs();
    let layout = valid(ScreenplayLayout::build_with_composition_and_environment(
        &project,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print {
            include_title_page: true,
        },
        &environment,
    ));

    assert!(pdf.bytes.starts_with(b"%PDF-1.4\n"));
    let pdf_source = valid(std::str::from_utf8(&pdf.bytes));
    assert!(!pdf_source.contains("PRIVATE HIDDEN FIXTURE CONTENT"));
    let action_line = layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .find(|line| line.element == 0 && line.display_text.contains('W'))
        .unwrap_or_else(|| unreachable!("action fixture has a visible W"));
    let display_index = action_line
        .display_text
        .chars()
        .position(|character| character == 'W')
        .unwrap_or_else(|| unreachable!("selected line contains W"));
    let mut span_start = 0usize;
    let (span, span_index) = action_line
        .spans
        .iter()
        .find_map(|span| {
            let count = span.display_text.chars().count();
            let found = (display_index >= span_start
                && display_index < span_start.saturating_add(count))
            .then_some((span, display_index - span_start));
            span_start = span_start.saturating_add(count);
            found
        })
        .unwrap_or_else(|| unreachable!("W is represented by a canonical span"));
    let glyph_x = FixedPoint::from_raw(
        span.geometry
            .x
            .raw()
            .saturating_add(span.display_boundary_offsets[span_index].raw()),
    );
    let glyph_y = FixedPoint::from_raw(
        FixedPoint::from_points(i64::from(layout.inputs.paper_height))
            .raw()
            .saturating_sub(span.geometry.y.raw())
            .saturating_sub(span.geometry.height.raw()),
    );
    let expected_glyph_command = format!(
        "{} {} Tm (W) Tj ET",
        glyph_x.decimal_string(),
        glyph_y.decimal_string(),
    );
    assert!(
        pdf_source.contains(&expected_glyph_command),
        "expected canonical PDF glyph command {expected_glyph_command}; got {}",
        pdf_source
            .lines()
            .filter(|line| line.contains("(W) Tj"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(pdf.typography.entries().len() <= 8);
    assert!(pdf.typography.entries().iter().any(|entry| matches!(
        entry.kind,
        PdfFallbackKind::Base14FontSubstitution {
            requested: FontResolutionKind::NearestSans,
            resource: PdfBase14Family::Helvetica,
        }
    )));
    assert!(pdf.typography.entries().iter().any(|entry| matches!(
        entry.kind,
        PdfFallbackKind::Base14FontSubstitution {
            requested: FontResolutionKind::NearestSerif,
            resource: PdfBase14Family::Times,
        }
    )));
    assert!(pdf.typography.entries().iter().any(|entry| matches!(
        entry.kind,
        PdfFallbackKind::Base14FontSubstitution {
            requested: FontResolutionKind::UnknownFallback,
            resource: PdfBase14Family::Courier,
        }
    )));
    assert!(pdf.typography.entries().iter().any(|entry| matches!(
        entry.kind,
        PdfFallbackKind::Base14FontSubstitution {
            requested: FontResolutionKind::ExactHost,
            resource: PdfBase14Family::Helvetica,
        }
    )));
    assert!(pdf
        .typography
        .entries()
        .iter()
        .any(|entry| matches!(entry.kind, PdfFallbackKind::UnsupportedScalarSubstitution)));
    assert!(pdf
        .typography
        .entries()
        .iter()
        .any(|entry| matches!(entry.kind, PdfFallbackKind::UnsupportedExplicitColor)));
    let summary = pdf.typography.summary();
    assert!(!summary.contains("canonical metric geometry unverified"));
    assert!(summary.contains("generic resource (not embedded)"));
    assert!(!summary.contains("Output report"));
    assert!(!summary.contains("private-invalid-color-value"));
    assert_eq!(project, original);
}
