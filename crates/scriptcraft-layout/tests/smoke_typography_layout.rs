#![forbid(unsafe_code)]

//! Prepared WO-005.10 regression source. The ignored geometry acceptance below
//! remains blocked on the active-builder/metric-routing work in WO-005.4/.5.

use scriptcraft_fdx::read_v6;
use scriptcraft_layout::{
    resolve_effective_dual_wrapper_paragraph, resolve_effective_paragraph,
    resolve_effective_run_text_style, resolve_effective_text_style, resolve_effective_title, Caret,
    LayoutCache, LayoutComposition, LayoutEnvironment, PaginationPolicy, ScreenplayLayout,
};
use scriptcraft_model::dual_dialogue::active_dual_dialogue_projection;
use scriptcraft_model::{
    formatting::{FixedPoint, SourceLayoutSettings},
    typography::{
        DocumentFontEnvironment, FaceStyle, FontResolutionKind, RequestedFontResolution,
        ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
        ScalarAdvance,
    },
    BodyOwner, ElementTypeKey, Project,
};
use std::collections::{BTreeMap, BTreeSet};

const TYPOGRAPHY_FDX: &[u8] = include_bytes!("fixtures/typography_layout.fdx");

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result
        .unwrap_or_else(|error| panic!("sanitized typography fixture/input is invalid: {error:?}"))
}

fn imported_project() -> Project {
    valid(read_v6(TYPOGRAPHY_FDX)).project
}

fn foreign_body<'a>(
    project: &'a Project,
    foreign_id: &str,
) -> &'a scriptcraft_model::ScreenplayElement {
    project
        .elements
        .iter()
        .map(|owner| owner.as_element())
        .find(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some(foreign_id)
        })
        .unwrap_or_else(|| unreachable!("fixture body owner exists"))
}

fn synthetic_environment(project: &Project, width_scale: u32, line_gap: i32) -> LayoutEnvironment {
    let mut families = project
        .element_definitions()
        .iter()
        .map(|definition| definition.font.family.clone())
        .collect::<BTreeSet<_>>();
    families.extend([
        "Arial".to_owned(),
        "Times New Roman".to_owned(),
        "Uncatalogued Family".to_owned(),
        "Courier Prime".to_owned(),
    ]);
    for owner in &project.elements {
        for run in &owner.as_element().runs {
            families.extend(run.attributes.iter().filter_map(|attribute| {
                (attribute.name == "Font").then(|| attribute.value.clone())
            }));
        }
    }

    let mut scalars = (32_u8..=126).map(char::from).collect::<BTreeSet<_>>();
    scalars.extend(['é', 'É', 'ß', '—', 'À', 'İ']);
    for owner in &project.elements {
        for scalar in owner.as_element().text.chars() {
            scalars.insert(scalar);
            scalars.extend(scalar.to_uppercase());
        }
    }
    for title in &project.title_page {
        for scalar in title.text.chars() {
            scalars.insert(scalar);
            scalars.extend(scalar.to_uppercase());
        }
    }

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
                "fixture-{family_index}-{style_index}"
            )))
        });
        for (style_index, (style, id)) in FaceStyle::ALL.into_iter().zip(ids.iter()).enumerate() {
            let advances = scalars
                .iter()
                .map(|scalar| {
                    let nominal = match scalar {
                        ' ' => 300,
                        'i' | 'l' | 'I' => 260,
                        'W' | 'M' => 920,
                        _ => 560,
                    };
                    (
                        *scalar,
                        ScalarAdvance {
                            advance_units: nominal * width_scale,
                            missing_glyph: false,
                        },
                    )
                })
                .collect();
            let metrics = valid(ResolvedFaceMetrics::new(
                1_000,
                800,
                -200,
                line_gap.max(0),
                advances,
            ));
            let face = valid(ResolvedFontFace::new(
                id.clone(),
                family.clone(),
                format!("{family} {style:?}"),
                style,
                ResolvedFontSourceKind::Synthetic,
                256,
                0x5445_5354 + family_index as u64 + style_index as u64,
                metrics,
            ));
            faces.insert(id.clone(), face);
        }
        let kind = if family.to_ascii_lowercase().contains("courier") {
            FontResolutionKind::CourierAlias
        } else if family == "Times New Roman" {
            FontResolutionKind::NearestSerif
        } else if family == "Arial" || family == "Helvetica Neue" {
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
fn fixture_import_resolves_definition_direct_run_and_stable_owner_values() {
    let mut project = imported_project();
    assert_eq!(project.validate(), Ok(()));
    assert_eq!(
        project
            .element_definitions()
            .iter()
            .take(6)
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Mono Layout",
            "Sans Layout",
            "Serif Layout",
            "Mystery Layout",
            "Misleading Scene",
            "Page Break Custom",
        ]
    );

    let environment = synthetic_environment(&project, 1, 0);
    let mono = foreign_body(&project, "typography-mono-long");
    let mono_key = mono.element_type();
    let mono_style = valid(resolve_effective_text_style(
        &project,
        &mono_key,
        &environment,
    ));
    assert_eq!(mono_style.declared_family, "Courier Final Draft");
    assert_eq!(mono_style.size, 13);
    assert!(mono_style.bold && mono_style.italic);
    assert_eq!(
        mono_style.color,
        scriptcraft_model::typography::EffectiveTextColor::ExplicitRgb8 {
            red: 0x87,
            green: 0x8F,
            blue: 0xE0,
        }
    );
    let mono_face = valid(environment.fonts.require_face(&mono_style.resolved_font_id));
    assert!(
        valid(mono_face.metrics.scale_advance('W', mono_style.size))
            > valid(mono_face.metrics.scale_advance('i', mono_style.size))
    );

    let overridden = foreign_body(&project, "typography-direct-overrides");
    let overridden_key = overridden.element_type();
    let paragraph = valid(resolve_effective_paragraph(
        &project,
        &overridden_key,
        &overridden.id,
        &overridden.runs,
        &environment,
    ));
    assert_eq!(
        paragraph.alignment,
        scriptcraft_model::formatting::ParagraphAlignment::Center
    );
    assert_eq!(
        paragraph.first_indent,
        FixedPoint::parse_inches("0.35").unwrap_or_default()
    );
    assert!(paragraph.starts_new_page);
    assert_eq!(paragraph.space_before, FixedPoint::from_points(6));
    let direct_run = overridden
        .runs
        .iter()
        .find(|run| {
            run.attributes
                .iter()
                .any(|attribute| attribute.name == "Font")
        })
        .unwrap_or_else(|| unreachable!("fixture has a direct Font run"));
    let direct_style = valid(resolve_effective_run_text_style(
        &project,
        &overridden_key,
        direct_run,
        &environment,
    ));
    assert_eq!(direct_style.declared_family, "Times New Roman");
    assert_eq!(direct_style.size, 21);
    assert!(direct_style.bold && direct_style.italic && direct_style.underline);

    let mystery = foreign_body(&project, "typography-unsupported-color");
    let mystery_key = mystery.element_type();
    let mystery_style = valid(resolve_effective_text_style(
        &project,
        &mystery_key,
        &environment,
    ));
    assert_eq!(
        mystery_style.color,
        scriptcraft_model::typography::EffectiveTextColor::UnsupportedExplicit
    );
    assert_eq!(
        mystery_style.source_color.as_deref(),
        Some("not-a-supported-color")
    );

    let misleading = foreign_body(&project, "typography-misleading-pagination");
    let misleading_key = misleading.element_type();
    let misleading_definition = project
        .element_definitions()
        .get(&misleading_key)
        .unwrap_or_else(|| unreachable!("the exact current definition exists"));
    assert!(!misleading_definition.capabilities().scene_boundary);
    assert_eq!(
        valid(resolve_effective_paragraph(
            &project,
            &misleading_key,
            &misleading.id,
            &misleading.runs,
            &environment,
        ))
        .pagination,
        PaginationPolicy::KeepWithNext
    );
    let page_start = foreign_body(&project, "typography-starts-new-page");
    let page_start_key = page_start.element_type();
    assert!(
        valid(resolve_effective_paragraph(
            &project,
            &page_start_key,
            &page_start.id,
            &page_start.runs,
            &environment,
        ))
        .starts_new_page
    );

    let absent_color = foreign_body(&project, "typography-absent-color");
    assert_eq!(
        valid(resolve_effective_text_style(
            &project,
            &absent_color.element_type(),
            &environment,
        ))
        .color,
        scriptcraft_model::typography::EffectiveTextColor::ThemeDefault
    );
    let typed_title = project
        .title_page
        .iter()
        .find(|title| {
            title
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("typography-title-serif")
        })
        .unwrap_or_else(|| unreachable!("typed title fixture entry exists"));
    assert_eq!(
        valid(resolve_effective_title(&project, typed_title, &environment))
            .1
            .declared_family,
        "Times New Roman"
    );
    let neutral_title = project
        .title_page
        .iter()
        .find(|title| {
            title
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("typography-title-neutral")
        })
        .unwrap_or_else(|| unreachable!("type-less title fixture entry exists"));
    assert_eq!(
        valid(resolve_effective_title(
            &project,
            neutral_title,
            &environment
        ))
        .1
        .declared_family,
        "Courier Prime"
    );
    assert_eq!(project.dual_dialogue_groups.len(), 2);
    assert!(project
        .dual_dialogue_groups
        .iter()
        .any(|group| { active_dual_dialogue_projection(&project, group).is_some() }));
    assert!(project
        .dual_dialogue_groups
        .iter()
        .any(|group| { active_dual_dialogue_projection(&project, group).is_none() }));

    let stable_id = overridden.id.clone();
    assert!(project.direct_paragraph_overrides.contains_key(&stable_id));
    let remap_target = valid(ElementTypeKey::new("Serif Layout"));
    for owner in &mut project.elements {
        if owner.as_element().id == stable_id {
            BodyOwner::update(owner, |element| {
                element.set_element_type(remap_target.clone())
            });
        }
    }
    let remapped = valid(resolve_effective_paragraph(
        &project,
        &remap_target,
        &stable_id,
        &[],
        &environment,
    ));
    assert_eq!(remapped.alignment, paragraph.alignment);
    assert_eq!(remapped.first_indent, paragraph.first_indent);

    let tall_environment = synthetic_environment(&project, 1, 200);
    let tall_paragraph = valid(resolve_effective_paragraph(
        &project,
        &remap_target,
        &stable_id,
        &[],
        &tall_environment,
    ));
    assert!(tall_paragraph.line_advance > remapped.line_advance);

    let active_group = project
        .dual_dialogue_groups
        .iter()
        .find(|group| active_dual_dialogue_projection(&project, group).is_some())
        .unwrap_or_else(|| unreachable!("fixture contains one active dual group"));
    let wrapper = valid(resolve_effective_dual_wrapper_paragraph(
        &project,
        &active_group.id,
        &environment,
    ));
    assert_eq!(
        wrapper.left,
        FixedPoint::parse_inches("0.30").unwrap_or_default()
    );
}

#[test]
fn environment_and_definition_mutations_invalidate_complete_layout_cache() {
    let mut project = imported_project();
    let inputs = project.session_layout_inputs();
    let first_environment = synthetic_environment(&project, 1, 0);
    let changed_metrics = synthetic_environment(&project, 2, 0);
    let mut cache = LayoutCache::default();
    let composition = LayoutComposition::BodyEditor;

    let (first_layout, first_hit) = valid(cache.get_or_build_with_composition_and_environment(
        7,
        &project,
        &inputs,
        composition,
        &first_environment,
    ));
    assert!(!first_hit);
    let first_line = first_layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .find(|line| line.element == 0)
        .unwrap_or_else(|| unreachable!("fixture has a first body line"));
    assert!(first_line.spans.iter().all(|span| {
        span.display_boundary_offsets.len() == span.display_text.chars().count() + 1
    }));
    let caret = valid(first_layout.caret_geometry(Caret {
        element: first_line.element,
        offset: first_line.source.start,
    }));
    assert_eq!(caret.y, first_line.geometry.y);
    assert_eq!(caret.height, first_line.geometry.height);
    let (_, identical_hit) = valid(cache.get_or_build_with_composition_and_environment(
        7,
        &project,
        &inputs,
        composition,
        &first_environment,
    ));
    assert!(identical_hit);
    let (_, metrics_hit) = valid(cache.get_or_build_with_composition_and_environment(
        7,
        &project,
        &inputs,
        composition,
        &changed_metrics,
    ));
    assert!(!metrics_hit, "same face IDs with changed metrics must miss");

    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions[0].font.size = definitions[0].font.size.saturating_add(1);
    valid(project.replace_element_definitions(definitions));
    let (_, definition_hit) = valid(cache.get_or_build_with_composition_and_environment(
        7,
        &project,
        &inputs,
        composition,
        &changed_metrics,
    ));
    assert!(!definition_hit, "same-revision definition edits must miss");
}

#[test]
#[ignore = "source-only acceptance pending complete WO-005.4/.5 active-builder metric routing"]
fn proportional_metrics_drive_wrapping_leading_and_vertical_page_fit() {
    let project = imported_project();
    let narrow = synthetic_environment(&project, 1, 0);
    let wide = synthetic_environment(&project, 2, 200);
    let inputs = project.session_layout_inputs();
    let source = SourceLayoutSettings::default();
    let build = |environment: &LayoutEnvironment| {
        valid(ScreenplayLayout::build_with_composition_and_environment(
            &project,
            &inputs,
            &source,
            LayoutComposition::BodyEditor,
            environment,
        ))
    };
    let narrow_layout = build(&narrow);
    let wide_layout = build(&wide);
    let line_signature = |layout: &ScreenplayLayout| {
        layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .filter(|line| line.element == 0)
            .map(|line| {
                (
                    line.source.start,
                    line.source.end,
                    line.geometry.y.raw(),
                    line.geometry.height.raw(),
                    line.spans
                        .iter()
                        .flat_map(|span| {
                            span.display_boundary_offsets
                                .iter()
                                .map(|point| point.raw())
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_ne!(line_signature(&narrow_layout), line_signature(&wide_layout));

    let editor_text = narrow_layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .map(|line| line.display_text.as_ref())
        .collect::<String>();
    assert!(editor_text.contains("unsupported explicit color"));
    assert!(editor_text.contains("STRASSE"));
    let print_layout = valid(ScreenplayLayout::build_with_composition_and_environment(
        &project,
        &inputs,
        &source,
        LayoutComposition::Print {
            include_title_page: true,
        },
        &narrow,
    ));
    let print_text = print_layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .map(|line| line.display_text.as_ref())
        .collect::<String>();
    assert!(!print_text.contains("unsupported explicit color"));
}
