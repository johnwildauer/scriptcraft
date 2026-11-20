#![forbid(unsafe_code)]

//! Typography and canonical metric integration regressions.

use scriptcraft_fdx::read_v6;
use scriptcraft_layout::{
    resolve_effective_dual_wrapper_paragraph, resolve_effective_paragraph,
    resolve_effective_run_text_style, resolve_effective_text_style, resolve_effective_title, Caret,
    LayoutCache, LayoutComposition, LayoutEnvironment, PaginationPolicy, ScreenplayLayout,
};
use scriptcraft_model::dual_dialogue::active_dual_dialogue_projection;
use scriptcraft_model::{
    formatting::{FixedPoint, ParagraphAlignment, SourceLayoutSettings, SourceParagraphOverrides},
    title_page::{TitlePageAlignment, TitlePageParagraphFormatting},
    typography::{
        DocumentFontEnvironment, FaceStyle, FontResolutionKind, RequestedFontResolution,
        ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
        ScalarAdvance,
    },
    BodyOwner, ElementTypeKey, Project, RunAttribute, TextRun, TitlePageParagraph,
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
    let wrapper_inset = FixedPoint::parse_inches("0.30").unwrap_or_default();
    assert_eq!(
        wrapper.right,
        FixedPoint::from_raw(
            FixedPoint::from_points(i64::from(environment.page.paper_width))
                .raw()
                .saturating_sub(wrapper_inset.raw()),
        )
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
fn active_dual_members_use_current_definitions_direct_overrides_and_metrics() {
    let mut project = imported_project();
    let left_id = foreign_body(&project, "typography-active-left-line")
        .id
        .clone();
    let right_id = foreign_body(&project, "typography-active-right-line")
        .id
        .clone();
    let left_index = project
        .elements
        .iter()
        .position(|owner| owner.as_element().id == left_id)
        .unwrap_or_else(|| unreachable!("left active member exists"));
    let long_text = "Wide WWW and narrow iii. ".repeat(18);
    BodyOwner::update(&mut project.elements[left_index], |element| {
        element.text = long_text.clone();
        element.runs = vec![TextRun {
            text: long_text.clone(),
            attributes: vec![
                RunAttribute {
                    name: "Font".to_owned(),
                    value: "Arial".to_owned(),
                },
                RunAttribute {
                    name: "Size".to_owned(),
                    value: "22".to_owned(),
                },
                RunAttribute {
                    name: "Style".to_owned(),
                    value: "Bold".to_owned(),
                },
            ],
        }];
    });

    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let dialogue = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Dialogue")
        .unwrap_or_else(|| unreachable!("current Dialogue definition exists"));
    dialogue.font.family = "Times New Roman".to_owned();
    dialogue.font.size = 18;
    valid(project.replace_element_definitions(definitions));

    let mut direct = SourceParagraphOverrides::default();
    direct.paragraph.alignment = Some(ParagraphAlignment::Right);
    project
        .direct_paragraph_overrides
        .insert(left_id.clone(), direct);

    let inputs = project.session_layout_inputs();
    let source = SourceLayoutSettings::default();
    let narrow = synthetic_environment(&project, 1, 0);
    let wide = synthetic_environment(&project, 2, 0);
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
    let lines_for = |layout: &ScreenplayLayout, index| {
        layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .filter(|line| line.element == index)
            .cloned()
            .collect::<Vec<_>>()
    };
    let narrow_lines = lines_for(&narrow_layout, left_index);
    let wide_lines = lines_for(&wide_layout, left_index);
    let right_index = project
        .elements
        .iter()
        .position(|owner| owner.as_element().id == right_id)
        .unwrap_or_else(|| unreachable!("right active member exists"));
    assert!(!narrow_lines.is_empty());
    assert!(wide_lines.len() > narrow_lines.len());
    assert!(narrow_lines.iter().all(|line| {
        line.resolved_text_style
            .as_ref()
            .is_some_and(|style| style.declared_family == "Times New Roman" && style.size == 18)
            && line.alignment == ParagraphAlignment::Right
            && line.spans.iter().all(|span| {
                span.resolved_text_style.as_ref().is_some_and(|style| {
                    style.declared_family == "Arial" && style.size == 22 && style.bold
                }) && span.display_boundary_offsets.len() == span.display_text.chars().count() + 1
            })
    }));
    let right_lines = lines_for(&narrow_layout, right_index);
    assert!(!right_lines.is_empty());
    assert!(right_lines.iter().all(|line| {
        line.resolved_text_style
            .as_ref()
            .is_some_and(|style| style.declared_family == "Times New Roman" && style.size == 18)
    }));
}

#[test]
fn title_composition_uses_resolved_styles_metrics_and_editable_formatting() {
    let mut project = imported_project();
    let neutral_index = project
        .title_page
        .iter()
        .position(|title| {
            title
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("typography-title-neutral")
        })
        .unwrap_or_else(|| unreachable!("fixture contains a type-less title"));

    let long_text = "Wide WWW and narrow iii café ".repeat(70);
    let mut editable = valid(TitlePageParagraph::new(
        "typography-title-editable-resolved",
        long_text.clone(),
        TitlePageParagraphFormatting {
            alignment: Some(TitlePageAlignment::Right),
            first_indent: Some(FixedPoint::from_points(12)),
            left_indent: Some(FixedPoint::from_points(80)),
            right_indent: Some(FixedPoint::from_points(500)),
            leading_regular: Some(false),
            space_before: Some(FixedPoint::from_points(7)),
            spacing_single: Some(true),
            starts_new_page: Some(true),
        },
    ));
    editable.runs = vec![TextRun {
        text: long_text,
        attributes: vec![
            RunAttribute {
                name: "Font".to_owned(),
                value: "Arial".to_owned(),
            },
            RunAttribute {
                name: "Size".to_owned(),
                value: "20".to_owned(),
            },
            RunAttribute {
                name: "Style".to_owned(),
                value: "Bold+Italic+Underline".to_owned(),
            },
            RunAttribute {
                name: "Color".to_owned(),
                value: "#abcdef".to_owned(),
            },
        ],
    }];
    project.title_page.push(editable);

    let mut hidden = valid(TitlePageParagraph::new(
        "typography-title-hidden-print",
        "hidden title text",
        TitlePageParagraphFormatting::default(),
    ));
    hidden.set_element_type(Some(valid(ElementTypeKey::new("Mystery Layout"))));
    project.title_page.push(hidden);

    let inputs = project.session_layout_inputs();
    let source = SourceLayoutSettings::default();
    let narrow = synthetic_environment(&project, 1, 0);
    let wide = synthetic_environment(&project, 2, 0);
    let editable_element = project.title_page.len() - 2;
    let (editable_paragraph, editable_style) = valid(resolve_effective_title(
        &project,
        &project.title_page[editable_element],
        &narrow,
    ));
    assert_eq!(editable_style.declared_family, "Courier Prime");
    assert_eq!(
        editable_paragraph.alignment,
        scriptcraft_model::formatting::ParagraphAlignment::Right
    );
    assert_eq!(editable_paragraph.left, FixedPoint::from_points(80));
    assert_eq!(editable_paragraph.right, FixedPoint::from_points(500));
    assert_eq!(editable_paragraph.first_indent, FixedPoint::from_points(12));
    assert_eq!(editable_paragraph.space_before, FixedPoint::from_points(7));
    assert!(editable_paragraph.starts_new_page);
    let build = |environment: &LayoutEnvironment, composition| {
        valid(ScreenplayLayout::build_with_composition_and_environment(
            &project,
            &inputs,
            &source,
            composition,
            environment,
        ))
    };
    let narrow_layout = build(&narrow, LayoutComposition::TitlePageEditor);
    let wide_layout = build(&wide, LayoutComposition::TitlePageEditor);
    let lines_for = |layout: &ScreenplayLayout, element| {
        layout
            .pages
            .iter()
            .enumerate()
            .flat_map(|(page, layout_page)| {
                layout_page
                    .lines
                    .iter()
                    .filter(move |line| line.element == element)
                    .cloned()
                    .map(move |line| (page, line))
            })
            .collect::<Vec<_>>()
    };
    let narrow_lines = lines_for(&narrow_layout, editable_element);
    let wide_lines = lines_for(&wide_layout, editable_element);
    assert!(wide_lines.len() > narrow_lines.len());
    assert!(narrow_lines
        .iter()
        .all(|(_, line)| line.resolved_text_style.is_some()));
    assert!(narrow_lines
        .iter()
        .all(|(_, line)| line.spans.iter().all(|span| {
            span.resolved_text_style.as_ref().is_some_and(|style| {
                style.declared_family == "Arial"
                    && style.size == 20
                    && style.bold
                    && style.italic
                    && style.underline
                    && style.source_color.as_deref() == Some("#abcdef")
            }) && span.display_boundary_offsets.len() == span.display_text.chars().count() + 1
                && span.display_source_map.iter().all(|mapping| {
                    mapping.source_start < mapping.source_end
                        && mapping.source_end <= project.title_page[editable_element].text.len()
                        && mapping.display_end.saturating_sub(mapping.display_start)
                            == project.title_page[editable_element].text
                                [mapping.source_start..mapping.source_end]
                                .chars()
                                .count()
                })
        })));
    assert!(narrow_lines.iter().all(|(page, _)| *page > 0));
    assert!(narrow_layout
        .title_page_for_position(editable_element, 0)
        .is_some());

    let typed_index = project
        .title_page
        .iter()
        .position(|title| {
            title
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("typography-title-serif")
        })
        .unwrap_or_else(|| unreachable!("fixture contains a typed title"));
    let typed_lines = lines_for(&narrow_layout, typed_index);
    assert!(typed_lines
        .iter()
        .flat_map(|(_, line)| line.spans.iter())
        .any(|span| {
            span.resolved_text_style
                .as_ref()
                .is_some_and(|style| style.declared_family == "Times New Roman" && style.size == 18)
        }));
    assert!(typed_lines
        .iter()
        .flat_map(|(_, line)| line.spans.iter())
        .any(|span| {
            span.resolved_text_style.as_ref().is_some_and(|style| {
                style.declared_family == "Arial"
                    && style.size == 20
                    && style.bold
                    && style.italic
                    && style.underline
                    && style.source_color.as_deref() == Some("#123456789ABC")
            })
        }));

    let neutral = lines_for(&narrow_layout, neutral_index);
    assert!(neutral
        .iter()
        .flat_map(|(_, line)| line.spans.iter())
        .any(|span| {
            span.resolved_text_style
                .as_ref()
                .is_some_and(|style| style.declared_family == "Courier Prime")
        }));

    let mut title_only_project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    let mut definitions = title_only_project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let general = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "General")
        .unwrap_or_else(|| unreachable!("new projects include the General definition"));
    general.font.hidden_text = true;
    valid(title_only_project.replace_element_definitions(definitions));
    let mut hidden_print_title = valid(TitlePageParagraph::new(
        "hidden-print-title",
        "hidden title text",
        TitlePageParagraphFormatting::default(),
    ));
    hidden_print_title.set_element_type(Some(valid(ElementTypeKey::new("General"))));
    title_only_project.title_page.push(hidden_print_title);
    let print_environment = synthetic_environment(&title_only_project, 1, 0);
    let print_layout = valid(ScreenplayLayout::build_with_composition_and_environment(
        &title_only_project,
        &title_only_project.session_layout_inputs(),
        &source,
        LayoutComposition::Print {
            include_title_page: true,
        },
        &print_environment,
    ));
    assert!(!print_layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .any(|line| { line.display_text.contains("hidden title text") }));
}

#[test]
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
    assert!(editor_text.contains("UNSUPPORTED EXPLICIT COLOR"));
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
    assert!(!print_text.contains("UNSUPPORTED EXPLICIT COLOR"));
}
