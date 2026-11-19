#![forbid(unsafe_code)]

//! Prepared WO-005.10 PDF policy source; bytes/report assertions are unexecuted.

use scriptcraft_export::{export_pdf, PdfBase14Family, PdfFallbackKind};
use scriptcraft_layout::LayoutEnvironment;
use scriptcraft_model::{
    typography::{
        DocumentFontEnvironment, FaceStyle, FontResolutionKind, RequestedFontResolution,
        ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
        ScalarAdvance,
    },
    BodyOwner, Project, Role, ScreenplayElement,
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
        "Unknown family".to_owned(),
    ]);
    let advances = (32_u8..=126)
        .map(|byte| {
            (
                char::from(byte),
                ScalarAdvance {
                    advance_units: match char::from(byte) {
                        ' ' => 300,
                        'i' | 'l' => 250,
                        'W' | 'M' => 900,
                        _ => 560,
                    },
                    missing_glyph: false,
                },
            )
        })
        .chain([(
            '\u{fffd}',
            ScalarAdvance {
                advance_units: 560,
                missing_glyph: true,
            },
        )])
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
#[ignore = "source-only acceptance pending WO-005.4/.5 active-builder definition/style routing"]
fn pdf_fallback_report_is_bounded_privacy_safe_and_non_mutating() {
    let mut project = valid(Project::new());
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
    valid(project.replace_element_definitions(definitions));
    project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-action",
            Role::Action,
            "Output report café — proportional Wiii WWW",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-dialogue",
            Role::Dialogue,
            "Serif resource and explicit RGB output.",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-scene",
            Role::SceneHeading,
            "Unknown-family output substitution.",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "typography-output-hidden",
            Role::Summary,
            "PRIVATE HIDDEN FIXTURE CONTENT",
        ))),
    ];
    let original = project.clone();
    let environment = synthetic_environment(&project);
    let pdf = valid(export_pdf(&project, &environment));

    assert!(pdf.bytes.starts_with(b"%PDF-1.4\n"));
    let pdf_source = valid(std::str::from_utf8(&pdf.bytes));
    assert!(!pdf_source.contains("PRIVATE HIDDEN FIXTURE CONTENT"));
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
    assert!(pdf
        .typography
        .entries()
        .iter()
        .any(|entry| matches!(entry.kind, PdfFallbackKind::MetricGeometryUnverified)));
    let summary = pdf.typography.summary();
    assert!(summary.contains("canonical metric geometry unverified"));
    assert!(!summary.contains("Output report"));
    assert!(!summary.contains("private-invalid-color-value"));
    assert_eq!(project, original);
}
