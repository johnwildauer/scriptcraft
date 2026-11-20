#![forbid(unsafe_code)]

use scriptcraft_layout::{
    Caret, LayoutCache, LayoutComposition, LayoutEnvironment, ScreenplayLayout,
};
use scriptcraft_model::dual_dialogue::{active_dual_dialogue_projection, DualDialogueGroup};
use scriptcraft_model::formatting::{
    FixedPoint, ParagraphAlignment, SourceLayoutSettings, SourceParagraphOverrides,
};
use scriptcraft_model::{
    typography::{
        DocumentFontEnvironment, FaceStyle, FontResolutionKind, RequestedFontResolution,
        ResolvedFaceMetrics, ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
        ScalarAdvance,
    },
    BodyOwner, ElementTypeKey, LayoutInputs, Project, RunAttribute, ScreenplayElement, StableId,
    TextRun,
};
use std::collections::{BTreeMap, BTreeSet};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("layout smoke scenario is valid"))
}

fn synthetic_body_project() -> Project {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    project.preservation = None;
    project
}

fn add_dialogue_paginate_alias(project: &mut Project) -> ElementTypeKey {
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut alias = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("Project::new supplies the base Action definition"));
    let key = valid(ElementTypeKey::new("Dialogue-like"));
    alias.key = key.clone();
    alias.behavior.paginate_as = Some("Dialogue".to_owned());
    definitions.push(alias);
    valid(project.replace_element_definitions(definitions));
    key
}

fn synthetic_layout_environment(project: &Project) -> LayoutEnvironment {
    let mut families = project
        .element_definitions()
        .iter()
        .map(|definition| definition.font.family.clone())
        .collect::<BTreeSet<_>>();
    families.insert("Courier Prime".to_owned());
    for owner in &project.elements {
        families.extend(owner.as_element().runs.iter().flat_map(|run| {
            run.attributes
                .iter()
                .filter_map(|attribute| (attribute.name == "Font").then(|| attribute.value.clone()))
        }));
    }
    let mut scalars = (32_u8..=126).map(char::from).collect::<BTreeSet<_>>();
    scalars.insert('ß');
    scalars.insert('ẞ');
    for owner in &project.elements {
        for scalar in owner.as_element().text.chars() {
            scalars.insert(scalar);
            scalars.extend(scalar.to_uppercase());
        }
    }
    let mut requests = BTreeMap::new();
    let mut faces = BTreeMap::new();
    for (family_index, family) in families.into_iter().enumerate() {
        let ids = FaceStyle::ALL.map(|style| {
            valid(ResolvedFontId::new(format!(
                "smoke-{family_index}-{style:?}"
            )))
        });
        for (style_index, (style, id)) in FaceStyle::ALL.into_iter().zip(ids.iter()).enumerate() {
            let advances = scalars
                .iter()
                .map(|scalar| {
                    let units = match scalar {
                        ' ' => 300,
                        'i' | 'l' | 'I' => 260,
                        'W' | 'M' => 920,
                        _ => 560,
                    };
                    (
                        *scalar,
                        ScalarAdvance {
                            advance_units: units,
                            missing_glyph: false,
                        },
                    )
                })
                .collect();
            let metrics = valid(ResolvedFaceMetrics::new(1_000, 800, -200, 0, advances));
            let face = valid(ResolvedFontFace::new(
                id.clone(),
                family.clone(),
                format!("{family} {style:?}"),
                style,
                ResolvedFontSourceKind::Synthetic,
                128,
                family_index as u64 + style_index as u64 + 1,
                metrics,
            ));
            faces.insert(id.clone(), face);
        }
        let kind = if family == "Courier Prime" {
            FontResolutionKind::CourierAlias
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
    LayoutEnvironment::new(project.session_layout_inputs().page_only(), fonts)
}

#[test]
fn layout_public_api_smoke() {
    let text = format!("{}éZ", "a".repeat(60));
    let mut project = synthetic_body_project();
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "layout-action",
        scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
        text.clone(),
    )))];
    let original = project.clone();
    let inputs = LayoutInputs::default();
    let source = SourceLayoutSettings::default();

    let first = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &source,
        LayoutComposition::BodyEditor,
    ));
    let second = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &source,
        LayoutComposition::BodyEditor,
    ));
    assert_eq!(first, second);
    assert!(!first.pages.is_empty());

    let lines = first
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element == 0)
        .collect::<Vec<_>>();
    // This legacy public-API smoke pins source/display behavior only; its fixed
    // default profile is not evidence of resolved-metric wrapping acceptance.
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].source.start, lines[0].source.end), (0, 62));
    assert_eq!(
        (lines[1].source.start, lines[1].source.end),
        (62, text.len())
    );
    assert_eq!(lines[0].source_for_display_char(60), 60);
    assert_eq!(lines[0].source_for_display_char(61), 62);
    for span in lines.iter().flat_map(|line| line.spans.iter()) {
        assert_eq!(
            span.display_boundary_offsets.first(),
            Some(&Default::default())
        );
        assert_eq!(
            span.display_boundary_offsets.len(),
            span.display_text.chars().count() + 1
        );
        assert_eq!(
            span.display_boundary_offsets
                .last()
                .map(|offset| offset.raw()),
            Some(span.geometry.width.raw())
        );
    }

    let caret = Caret {
        element: 0,
        offset: 60,
    };
    let caret_geometry = valid(first.caret_geometry(caret));
    assert_eq!(lines[0].page, 0);
    assert_eq!(first.pages[0].lines[0], *lines[0]);
    assert_eq!(caret_geometry.y, lines[0].geometry.y);
    assert_eq!(caret_geometry.height, lines[0].geometry.height);
    assert_eq!(project, original);
}

#[test]
fn sequential_body_uses_current_definition_inline_metrics_and_complete_cache_identity() {
    let mut project = synthetic_body_project();
    let action = valid(ElementTypeKey::new("Action"));
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let definition = definitions
        .iter_mut()
        .find(|definition| definition.key == action)
        .unwrap_or_else(|| unreachable!("Project::new supplies the Action definition"));
    definition.font.family = "Arial".to_owned();
    definition.font.size = 14;
    definition.paragraph.left_indent = FixedPoint::from_points(36);
    definition.paragraph.right_indent = FixedPoint::from_points(180);
    valid(project.replace_element_definitions(definitions));

    let direct_run = TextRun {
        text: "ßW ".to_owned(),
        attributes: vec![
            RunAttribute {
                name: "Font".to_owned(),
                value: "Times New Roman".to_owned(),
            },
            RunAttribute {
                name: "Size".to_owned(),
                value: "20".to_owned(),
            },
            RunAttribute {
                name: "Style".to_owned(),
                value: "Bold+Italic+Underline+AllCaps".to_owned(),
            },
            RunAttribute {
                name: "Color".to_owned(),
                value: "#FFFF00000000".to_owned(),
            },
        ],
    };
    let remaining = "iiii MMMM wide letters repeat for wrapping";
    let text = format!("{}{remaining}", direct_run.text);
    project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new(
            "prior-action",
            scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
            "preceding paragraph",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "metric-action",
            scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
            text,
        ))),
    ];
    BodyOwner::update(&mut project.elements[1], |element| {
        element.runs = vec![
            direct_run,
            TextRun {
                text: remaining.to_owned(),
                attributes: Vec::new(),
            },
        ];
    });
    let owner_id = project.elements[1].id.clone();
    let mut direct = SourceParagraphOverrides::default();
    direct.paragraph.alignment = Some(ParagraphAlignment::Center);
    direct.paragraph.first_indent = Some(FixedPoint::from_points(12));
    direct.paragraph.right_indent = Some(FixedPoint::from_points(170));
    direct.paragraph.space_before = Some(FixedPoint::from_points(4));
    direct.starts_new_page = Some(true);
    project
        .direct_paragraph_overrides
        .insert(owner_id.clone(), direct);

    let environment = synthetic_layout_environment(&project);
    let inputs = project.session_layout_inputs();
    let composition = LayoutComposition::BodyEditor;
    let source = SourceLayoutSettings::default();
    let mut cache = LayoutCache::default();
    let (layout, cache_hit) = valid(cache.get_or_build_with_composition_and_environment(
        1,
        &project,
        &inputs,
        composition,
        &environment,
    ));
    assert!(!cache_hit);
    let lines = layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .filter(|line| line.element == 1)
        .collect::<Vec<_>>();
    assert!(
        lines.len() > 1,
        "proportional scalar metrics must wrap this paragraph"
    );
    let first_line = lines
        .iter()
        .find(|line| line.source.start == 0)
        .copied()
        .unwrap_or_else(|| unreachable!("first canonical body line exists"));
    let previous_page = layout
        .pages
        .iter()
        .find(|page| page.lines.iter().any(|line| line.element == 0))
        .map(|page| page.number)
        .unwrap_or_else(|| unreachable!("preceding paragraph was laid out"));
    assert!(
        first_line.page > previous_page,
        "direct page start takes effect"
    );
    assert_eq!(
        first_line.paragraph_bounds.y,
        FixedPoint::from_points(i64::from(inputs.margin_top)),
        "space-before is suppressed at the fresh page boundary"
    );
    assert_eq!(first_line.alignment, ParagraphAlignment::Center);
    assert_eq!(first_line.paragraph_bounds.x, FixedPoint::from_points(36));
    assert_eq!(first_line.unaligned_line_left, FixedPoint::from_points(48));
    let definition_style = first_line
        .resolved_text_style
        .as_ref()
        .unwrap_or_else(|| unreachable!("resolved Body line retains its definition style"));
    assert_eq!(definition_style.declared_family, "Arial");
    assert_eq!(definition_style.size, 14);
    let inline = first_line
        .spans
        .iter()
        .find(|span| span.display_text.starts_with("SSW"))
        .unwrap_or_else(|| unreachable!("direct run remains in its metric span"));
    let inline_style = inline
        .resolved_text_style
        .as_ref()
        .unwrap_or_else(|| unreachable!("resolved Body span retains its inline style"));
    assert_eq!(inline.display_text.as_ref(), "SSW ");
    assert_eq!(inline_style.declared_family, "Times New Roman");
    assert_eq!(inline_style.size, 20);
    assert!(inline_style.bold && inline_style.italic && inline_style.underline);
    assert_eq!(
        inline_style.color,
        scriptcraft_model::typography::EffectiveTextColor::ExplicitRgb8 {
            red: 255,
            green: 0,
            blue: 0,
        }
    );
    assert_eq!(inline.display_source_map[0].display_start, 0);
    assert_eq!(inline.display_source_map[0].display_end, 1);
    assert_eq!(inline.display_source_map[0].source_start, 0);
    assert_eq!(inline.display_source_map[0].source_end, 2);
    assert_eq!(inline.display_source_map[1].display_start, 1);
    assert_eq!(inline.display_source_map[1].display_end, 2);
    assert_eq!(inline.display_source_map[1].source_start, 0);
    assert_eq!(inline.display_source_map[1].source_end, 2);
    assert_eq!(
        inline.display_boundary_offsets.len(),
        inline.display_text.chars().count() + 1
    );

    let (cached, same_cache_hit) = valid(cache.get_or_build_with_composition_and_environment(
        1,
        &project,
        &inputs,
        composition,
        &environment,
    ));
    assert!(same_cache_hit);
    assert_eq!(layout.pages, cached.pages);

    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions
        .iter_mut()
        .find(|definition| definition.key == action)
        .unwrap_or_else(|| unreachable!("current Action definition exists"))
        .font
        .size = 18;
    valid(project.replace_element_definitions(definitions));
    let (changed, changed_hit) = valid(cache.get_or_build_with_composition_and_environment(
        1,
        &project,
        &inputs,
        composition,
        &environment,
    ));
    assert!(
        !changed_hit,
        "same-revision definition changes cannot reuse complete layout"
    );
    let fresh = valid(ScreenplayLayout::build_with_composition_and_environment(
        &project,
        &inputs,
        &source,
        composition,
        &environment,
    ));
    assert_eq!(changed.pages, fresh.pages);
    assert_ne!(layout.pages, changed.pages);

    let direct = project
        .direct_paragraph_overrides
        .get_mut(&owner_id)
        .unwrap_or_else(|| unreachable!("matching stable-ID direct override exists"));
    direct.paragraph.alignment = Some(ParagraphAlignment::Right);
    direct.paragraph.first_indent = Some(FixedPoint::from_points(8));
    let (override_changed, override_hit) =
        valid(cache.get_or_build_with_composition_and_environment(
            1,
            &project,
            &inputs,
            composition,
            &environment,
        ));
    assert!(
        !override_hit,
        "direct paragraph edits must invalidate complete layout"
    );
    let fresh_override = valid(ScreenplayLayout::build_with_composition_and_environment(
        &project,
        &inputs,
        &source,
        composition,
        &environment,
    ));
    assert_eq!(override_changed.pages, fresh_override.pages);
    assert_ne!(changed.pages, override_changed.pages);
}

#[test]
fn dual_source_members_switch_between_sequential_and_active_columns() {
    let mut project = synthetic_body_project();
    let members = [
        valid(ScreenplayElement::new(
            "inactive-left-cue",
            scriptcraft_model::ElementTypeKey::new("Character").expect("valid fixture label"),
            "LEFT",
        )),
        valid(ScreenplayElement::new(
            "inactive-left-line",
            scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
            "left line",
        )),
        valid(ScreenplayElement::new(
            "inactive-right-cue",
            scriptcraft_model::ElementTypeKey::new("Character").expect("valid fixture label"),
            "RIGHT",
        )),
        valid(ScreenplayElement::new(
            "inactive-right-line",
            scriptcraft_model::ElementTypeKey::new("Dialogue").expect("valid fixture label"),
            "right line",
        )),
    ];
    let member_ids = members.iter().map(|member| member.id.clone()).collect();
    project.elements = members.into_iter().map(BodyOwner::shared).collect();
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: valid(StableId::new("inactive-dual-group")),
        members: member_ids,
        wrapper: Default::default(),
    });
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    assert_eq!(project.validate(), Ok(()));

    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &LayoutInputs::default(),
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));
    let lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element < 4)
        .collect::<Vec<_>>();
    assert_eq!(
        lines.iter().map(|line| line.element).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert!(lines
        .windows(2)
        .all(|pair| { pair[0].paragraph_bounds.y < pair[1].paragraph_bounds.y }));

    let dialogue_alias = add_dialogue_paginate_alias(&mut project);
    BodyOwner::update(&mut project.elements[1], |element| {
        element.set_element_type(dialogue_alias);
    });
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    let alias_layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &LayoutInputs::default(),
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));
    let alias_lines = alias_layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element < 4)
        .collect::<Vec<_>>();
    assert_eq!(
        alias_lines
            .iter()
            .map(|line| line.element)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );

    BodyOwner::update(&mut project.elements[1], |element| {
        element.set_element_type(ElementTypeKey::new("Dialogue").expect("valid fixture label"));
    });
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_some());
    let active_layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &LayoutInputs::default(),
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));
    let left_cue_y = active_layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .find(|line| line.element == 0)
        .map(|line| line.paragraph_bounds.y);
    let right_cue_y = active_layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .find(|line| line.element == 2)
        .map(|line| line.paragraph_bounds.y);
    assert_eq!(left_cue_y, right_cue_y);
}

#[test]
fn vertical_page_fit_ignores_legacy_occupied_line_capacity() {
    let mut project = synthetic_body_project();
    project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new(
            "fit-one",
            scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
            "one",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "fit-two",
            scriptcraft_model::ElementTypeKey::new("Action").expect("valid fixture label"),
            "two",
        ))),
    ];
    let mut inputs = LayoutInputs::default();
    inputs.lines_per_page = 1;
    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));

    let action_lines = layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter().map(move |line| (page.number, line)))
        .filter(|(_, line)| line.element < 2)
        .collect::<Vec<_>>();
    assert_eq!(action_lines.len(), 2);
    assert_eq!(action_lines[0].0, action_lines[1].0);
    assert!(action_lines[1].1.paragraph_bounds.y > action_lines[0].1.paragraph_bounds.y);
}
