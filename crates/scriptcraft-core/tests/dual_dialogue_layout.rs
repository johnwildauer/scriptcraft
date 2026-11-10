#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::dual_dialogue::{
    DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance,
};
use scriptcraft_core::formatting::{
    PrintVisibility, SourceFontSpec, SourceLayoutSettings, SourceParagraphOverrides,
    SourceParagraphSpec,
};
use scriptcraft_core::layout::{
    BodyEditorPresentation, InteractionQueryDiagnostics, LayoutBuildDiagnostics, LayoutCache,
    LayoutComposition, LayoutErrorKind, ScreenplayLayout,
};
use scriptcraft_core::{
    FixedPoint, LayoutInputs, ParagraphAlignment, Project, Role, RunAttribute, ScreenplayElement,
    StableId, TextRun,
};
use std::ops::Range;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("dual-dialogue layout fixture is valid"))
}

fn role_index(role: Role) -> usize {
    Role::ALL
        .iter()
        .position(|candidate| *candidate == role)
        .unwrap_or_else(|| unreachable!("role belongs to the complete role catalog"))
}

fn project_with_roles(roles: &[Role], texts: &[&str]) -> Project {
    let mut project = valid(Project::new());
    project.elements = roles
        .iter()
        .zip(texts)
        .enumerate()
        .map(|(index, (role, text))| {
            valid(ScreenplayElement::new(
                format!("dual-layout-{index}"),
                *role,
                *text,
            ))
        })
        .collect();
    project
}

fn side(project: &Project, range: Range<usize>) -> DualDialogueSide {
    DualDialogueSide {
        character: project.elements[range.start].id.clone(),
        paragraphs: project.elements[range.start + 1..range.end]
            .iter()
            .map(|element| element.id.clone())
            .collect(),
    }
}

fn add_group(
    project: &mut Project,
    id: &str,
    left: Range<usize>,
    right: Range<usize>,
) {
    let group = DualDialogueGroup {
        id: valid(StableId::new(id)),
        left: side(project, left),
        right: side(project, right),
        wrapper: DualDialogueWrapperProvenance::default(),
    };
    project.dual_dialogue_groups.push(group);
}

fn build(
    project: &Project,
    settings: &SourceLayoutSettings,
    composition: LayoutComposition,
) -> ScreenplayLayout {
    build_with_inputs(project, settings, composition, &LayoutInputs::default())
}

fn build_with_inputs(
    project: &Project,
    settings: &SourceLayoutSettings,
    composition: LayoutComposition,
    inputs: &LayoutInputs,
) -> ScreenplayLayout {
    let mut settings = settings.clone();
    settings
        .direct_paragraph_overrides
        .extend(project.direct_paragraph_overrides.clone());
    valid(ScreenplayLayout::build_with_composition(
        project,
        inputs,
        &settings,
        composition,
    ))
}

fn build_presented(
    project: &Project,
    settings: &SourceLayoutSettings,
    presentation: &BodyEditorPresentation,
) -> ScreenplayLayout {
    let mut settings = settings.clone();
    settings
        .direct_paragraph_overrides
        .extend(project.direct_paragraph_overrides.clone());
    valid(ScreenplayLayout::build_with_body_presentation(
        project,
        &LayoutInputs::default(),
        &settings,
        presentation,
    ))
}

fn points(value: i64) -> FixedPoint {
    FixedPoint::from_points(value)
}

#[test]
fn default_columns_share_a_top_use_maximum_height_and_keep_left_before_right_order() {
    let mut project = project_with_roles(
        &[
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Action,
        ],
        &["before", "L", "short", "R", "123456789012345678901234567890", "after"],
    );
    add_group(&mut project, "wrapper-layout-default", 1..3, 3..5);
    let original = project.clone();
    let layout = build(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );
    let page = &layout.pages[0];
    let group_lines = page
        .lines
        .iter()
        .filter(|line| (1..5).contains(&line.element))
        .collect::<Vec<_>>();

    assert_eq!(project, original);
    assert_eq!(group_lines.iter().filter(|line| line.element < 3).count(), 2);
    assert!(group_lines.iter().take(2).all(|line| line.element < 3));
    assert!(group_lines.iter().skip(2).all(|line| line.element >= 3));
    assert_eq!(group_lines[0].source.element, 1);
    assert_eq!(group_lines[2].source.element, 3);
    assert_eq!(group_lines[0].paragraph_bounds.x, points(108));
    assert_eq!(group_lines[0].paragraph_bounds.width, points(198));
    assert_eq!(group_lines[2].paragraph_bounds.x, points(342));
    assert_eq!(group_lines[2].paragraph_bounds.width, points(198));
    assert_eq!(group_lines[0].paragraph_bounds.y, points(84));
    assert_eq!(group_lines[2].paragraph_bounds.y, points(84));
    assert_eq!(group_lines[0].geometry.x, FixedPoint::from_raw(20_350));
    assert_eq!(page.lines.last().map(|line| line.element), Some(5));
    assert_eq!(page.lines.last().map(|line| line.geometry.y), Some(points(120)));
    assert!(page.lines.iter().all(|line| line.role != Role::General));

    // Inner horizontal settings stay available when the same source paragraph
    // is composed sequentially rather than as an inactive dual group.
    project.direct_paragraph_overrides.insert(
        project.elements[2].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                left_indent: Some(points(200)),
                right_indent: Some(points(400)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    let mut sequential = project.clone();
    sequential.dual_dialogue_groups.clear();
    let ordinary = build(
        &sequential,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );
    let ordinary_dialogue = ordinary.pages[0]
        .lines
        .iter()
        .find(|line| line.element == 2)
        .unwrap_or_else(|| unreachable!("sequential dialogue line is present"));
    assert_eq!(ordinary_dialogue.paragraph_bounds.x, points(200));
    assert_eq!(ordinary_dialogue.paragraph_bounds.width, points(200));
}

#[test]
fn outer_extent_falls_back_independently_and_preserves_hundredth_point_boundaries() {
    let mut project = project_with_roles(
        &[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue],
        &["LEFT", "line", "RIGHT", "line"],
    );
    add_group(&mut project, "wrapper-layout-custom", 0..2, 2..4);
    let group_id = project.dual_dialogue_groups[0].id.clone();
    project.dual_dialogue_paragraph_overrides.insert(
        group_id,
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                right_indent: Some(FixedPoint::from_raw(60_075)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    project.direct_paragraph_overrides.insert(
        project.elements[5].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                alignment: Some(ParagraphAlignment::Right),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    let mut settings = SourceLayoutSettings::default();
    settings.role_paragraph_specs[role_index(Role::General)] = SourceParagraphSpec {
        left_indent: Some(FixedPoint::from_raw(12_025)),
        ..SourceParagraphSpec::default()
    };
    let layout = build(&project, &settings, LayoutComposition::BodyEditor);
    let first_left = layout.pages[0]
        .lines
        .iter()
        .find(|line| line.element == 0)
        .unwrap_or_else(|| unreachable!("left cue line is present"));
    let first_right = layout.pages[0]
        .lines
        .iter()
        .find(|line| line.element == 2)
        .unwrap_or_else(|| unreachable!("right cue line is present"));
    assert_eq!(first_left.paragraph_bounds.x, FixedPoint::from_raw(12_025));
    assert_eq!(first_left.paragraph_bounds.width, FixedPoint::from_raw(22_225));
    assert_eq!(first_right.paragraph_bounds.x, FixedPoint::from_raw(37_850));
    assert_eq!(first_right.paragraph_bounds.width, FixedPoint::from_raw(22_225));

    let mut default_fallback = project.clone();
    default_fallback.dual_dialogue_paragraph_overrides.clear();
    default_fallback.dual_dialogue_paragraph_overrides.insert(
        default_fallback.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                right_indent: Some(points(576)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    let fallback_layout = build(
        &default_fallback,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );
    let fallback_left = fallback_layout.pages[0]
        .lines
        .iter()
        .find(|line| line.element == 0)
        .unwrap_or_else(|| unreachable!("default-fallback left cue is present"));
    let fallback_right = fallback_layout.pages[0]
        .lines
        .iter()
        .find(|line| line.element == 2)
        .unwrap_or_else(|| unreachable!("default-fallback right cue is present"));
    assert_eq!(fallback_left.paragraph_bounds.x, points(108));
    assert_eq!(fallback_right.paragraph_bounds.x, points(342));
    assert_eq!(fallback_left.paragraph_bounds.width, points(216));

    let mut invalid = project;
    invalid.dual_dialogue_paragraph_overrides.insert(
        invalid.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                left_indent: Some(points(400)),
                right_indent: Some(points(420)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    assert!(matches!(
        ScreenplayLayout::build_with_composition(
            &invalid,
            &LayoutInputs::default(),
            &SourceLayoutSettings::default(),
            LayoutComposition::BodyEditor,
        ),
        Err(error) if error.kind == LayoutErrorKind::InvalidProfile
    ));
}

#[test]
fn outer_and_internal_space_before_apply_once_per_block_and_side_paragraph() {
    let mut project = project_with_roles(
        &[
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Parenthetical,
            Role::Character,
            Role::Dialogue,
            Role::Action,
        ],
        &["before", "L", "", "(beat)", "R", "reply", "after"],
    );
    add_group(&mut project, "wrapper-layout-spacing", 1..4, 4..6);
    let group_id = project.dual_dialogue_groups[0].id.clone();
    project.dual_dialogue_paragraph_overrides.insert(
        group_id,
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                space_before: Some(points(5)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    let mut settings = SourceLayoutSettings::default();
    settings.role_paragraph_specs[role_index(Role::Character)].space_before = Some(points(4));
    settings.role_paragraph_specs[role_index(Role::Dialogue)].space_before = Some(points(3));
    settings.role_paragraph_specs[role_index(Role::Parenthetical)].space_before = Some(points(2));
    let layout = build(&project, &settings, LayoutComposition::BodyEditor);
    let lines = &layout.pages[0].lines;
    let left = lines
        .iter()
        .filter(|line| (1..4).contains(&line.element))
        .collect::<Vec<_>>();
    let right = lines
        .iter()
        .filter(|line| (4..6).contains(&line.element))
        .collect::<Vec<_>>();
    assert_eq!(left.len(), 3);
    assert_eq!(right.len(), 2);
    assert_eq!(left[0].paragraph_bounds.y, points(89));
    assert_eq!(right[0].paragraph_bounds.y, points(89));
    assert_eq!(left[1].paragraph_bounds.y, points(104));
    assert_eq!(left[2].paragraph_bounds.y, points(118));
    assert_eq!(lines.last().map(|line| line.element), Some(6));
    assert_eq!(lines.last().map(|line| line.geometry.y), Some(points(130)));
    assert_eq!(left[1].display_text.as_ref(), "");
    assert_eq!(left[1].source.start, left[1].source.end);
    assert_eq!(left[2].paragraph_bounds.x, points(108));
    assert_eq!(left[2].paragraph_bounds.width, points(198));
    assert_eq!(right[1].paragraph_bounds.x, points(342));
    assert_eq!(right[1].paragraph_bounds.width, points(198));
    assert_eq!(right[1].geometry.x, points(505));

    let mut no_preceding_content = project.clone();
    no_preceding_content.elements.remove(0);
    let no_preceding_layout = build(
        &no_preceding_content,
        &settings,
        LayoutComposition::BodyEditor,
    );
    assert_eq!(no_preceding_layout.pages[0].lines[0].element, 0);
    assert_eq!(no_preceding_layout.pages[0].lines[0].geometry.y, points(72));
}

#[test]
fn unicode_all_caps_runs_and_centered_cues_keep_canonical_source_mapping() {
    let mut project = project_with_roles(
        &[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue],
        &["", "left", "RIGHT", "right"],
    );
    project.elements[0] = valid(ScreenplayElement::with_runs(
        "dual-layout-0",
        Role::Character,
        vec![
            TextRun {
                text: "straße".to_owned(),
                attributes: vec![RunAttribute {
                    name: "Style".to_owned(),
                    value: "AllCaps".to_owned(),
                }],
            },
            TextRun {
                text: " cue".to_owned(),
                attributes: vec![
                    RunAttribute {
                        name: "Style".to_owned(),
                        value: "Bold+Underline+Strikethrough".to_owned(),
                    },
                    RunAttribute {
                        name: "Size".to_owned(),
                        value: "24".to_owned(),
                    },
                ],
            },
        ],
        None,
    ));
    add_group(&mut project, "wrapper-layout-unicode", 0..2, 2..4);
    let mut settings = SourceLayoutSettings::default();
    settings.role_font_specs[role_index(Role::Character)] = SourceFontSpec {
        all_caps: Some(true),
        ..SourceFontSpec::default()
    };
    let layout = build(&project, &settings, LayoutComposition::BodyEditor);
    let cue = layout.pages[0]
        .lines
        .iter()
        .find(|line| line.element == 0)
        .unwrap_or_else(|| unreachable!("left cue line is present"));

    assert_eq!(cue.text.as_ref(), "straße cue");
    assert_eq!(cue.display_text.as_ref(), "STRASSE CUE");
    assert_eq!(cue.paragraph_bounds.x, points(108));
    assert_eq!(cue.paragraph_bounds.width, points(198));
    assert!(cue.geometry.x > cue.paragraph_bounds.x);
    assert!(cue.spans.iter().any(|span| {
        span.text_style.size == 24
            && span.text_style.bold
            && span.underline
            && span.strikethrough
    }));
    assert!(cue.spans.iter().any(|span| span.char_width == points(14)));
    assert!(cue.display_source_map.iter().any(|segment| {
        segment.source_start == 4
            && segment.source_end == 6
            && segment.display_start == 4
            && segment.display_end == 6
    }));
    assert_eq!(cue.source_for_display_char(7), "straße".len());
    assert_eq!(cue.source.end, "straße cue".len());
}

#[test]
fn hidden_text_is_retained_in_editor_and_omitted_from_print_across_multiple_groups() {
    let roles = [
        Role::Action,
        Role::Character,
        Role::Dialogue,
        Role::Character,
        Role::Dialogue,
        Role::Action,
        Role::Character,
        Role::Parenthetical,
        Role::Character,
        Role::Dialogue,
        Role::Action,
    ];
    let texts = [
        "before",
        "LEFT ONE",
        "left words",
        "RIGHT ONE",
        "right words",
        "between",
        "LEFT TWO",
        "(left)",
        "RIGHT TWO",
        "right reply",
        "after",
    ];
    let mut project = project_with_roles(&roles, &texts);
    add_group(&mut project, "wrapper-layout-hidden-a", 1..3, 3..5);
    add_group(&mut project, "wrapper-layout-hidden-b", 6..8, 8..10);
    let original = project.clone();
    let mut settings = SourceLayoutSettings::default();
    settings.role_print_visibility[role_index(Role::Character)] = Some(PrintVisibility::Hidden);

    let editor = build(&project, &settings, LayoutComposition::BodyEditor);
    let printed = build(
        &project,
        &settings,
        LayoutComposition::Print {
            include_title_page: false,
        },
    );
    assert!(editor.pages.iter().flat_map(|page| &page.lines).any(|line| {
        matches!(line.element, 1 | 3 | 6 | 8)
    }));
    assert!(printed.pages.iter().flat_map(|page| &page.lines).all(|line| {
        !matches!(line.element, 1 | 3 | 6 | 8)
    }));
    assert!(printed.pages.iter().flat_map(|page| &page.lines).any(|line| line.element == 5));
    assert!(printed.pages.iter().flat_map(|page| &page.lines).any(|line| line.element == 10));
    assert_eq!(project, original);
}

#[test]
fn print_and_editor_keep_ungrouped_lines_and_apply_wrapper_page_start_as_one_block() {
    let mut project = project_with_roles(
        &[
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Action,
        ],
        &["before", "LEFT", "left", "RIGHT", "right", "after"],
    );
    add_group(&mut project, "wrapper-layout-start-page", 1..3, 3..5);
    project.dual_dialogue_paragraph_overrides.insert(
        project.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            starts_new_page: Some(true),
            ..SourceParagraphOverrides::default()
        },
    );
    let layout = build(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );
    assert!(layout.pages.len() >= 2);
    assert_eq!(layout.pages[0].lines.last().map(|line| line.element), Some(0));
    let group_page = layout
        .pages
        .iter()
        .find(|page| page.lines.iter().any(|line| line.element == 1))
        .unwrap_or_else(|| unreachable!("group page is present"));
    assert_eq!(group_page.lines.first().map(|line| line.element), Some(1));
    assert_eq!(group_page.lines[0].paragraph_bounds.x, points(108));
    assert_eq!(group_page.lines[0].geometry.x, FixedPoint::from_raw(19_300));
    let printed = build(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print {
            include_title_page: false,
        },
    );
    assert!(printed.pages.iter().flat_map(|page| &page.lines).any(|line| line.element == 0));
    assert!(printed.pages.iter().flat_map(|page| &page.lines).any(|line| line.element == 5));
    let printed_left = printed.pages.iter().flat_map(|page| &page.lines)
        .find(|line| line.element == 1)
        .unwrap_or_else(|| unreachable!("printed left cue is present"));
    let printed_right = printed.pages.iter().flat_map(|page| &page.lines)
        .find(|line| line.element == 3)
        .unwrap_or_else(|| unreachable!("printed right cue is present"));
    assert_eq!(printed_left.paragraph_bounds.x, points(108));
    assert_eq!(printed_right.paragraph_bounds.x, points(342));
    assert_eq!(printed_left.paragraph_bounds.y, printed_right.paragraph_bounds.y);
}

#[test]
fn dual_block_exactly_fits_both_fixed_point_height_and_line_budget() {
    let mut project = project_with_roles(
        &[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue],
        &["LEFT", "left", "RIGHT", "right"],
    );
    add_group(&mut project, "wrapper-pagination-exact", 0..2, 2..4);
    let inputs = LayoutInputs {
        paper_height: 40,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 2,
        ..LayoutInputs::default()
    };

    let layout = build_with_inputs(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &inputs,
    );

    assert_eq!(layout.pages.len(), 1);
    assert_eq!(layout.pages[0].lines.len(), 4);
    assert!(layout.pages[0].lines.iter().all(|line| line.page == 0));
    assert!(layout.pages[0]
        .lines
        .iter()
        .all(|line| line.paragraph_bounds.y >= points(8)));
    assert_eq!(layout.pages[0].lines[0].element, 0);
    assert_eq!(layout.pages[0].lines[1].element, 1);
    assert_eq!(layout.pages[0].lines[2].element, 2);
    assert_eq!(layout.pages[0].lines[3].element, 3);
}

#[test]
fn one_fixed_point_unit_overflow_moves_the_whole_block_and_preserves_source_ownership() {
    let mut project = project_with_roles(
        &[
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Action,
        ],
        &["before", "LEFT", "left", "RIGHT", "right", "after"],
    );
    add_group(&mut project, "wrapper-pagination-one-unit", 1..3, 3..5);
    project.dual_dialogue_paragraph_overrides.insert(
        project.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                space_before: Some(FixedPoint::from_raw(1)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    let inputs = LayoutInputs {
        paper_height: 52,
        margin_top: 8,
        margin_bottom: 8,
        ..LayoutInputs::default()
    };

    let layout = build_with_inputs(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &inputs,
    );

    assert_eq!(layout.pages[0].lines.len(), 1);
    assert_eq!(layout.pages[0].lines[0].element, 0);
    let group_lines = layout.pages[1]
        .lines
        .iter()
        .filter(|line| (1..5).contains(&line.element))
        .collect::<Vec<_>>();
    assert_eq!(group_lines.len(), 4);
    assert!(group_lines.iter().all(|line| line.page == 1));
    assert!(group_lines
        .iter()
        .all(|line| line.source.element == line.element));
    assert!(group_lines.iter().all(|line| !line.keep_with_next));
    assert_eq!(group_lines[0].paragraph_bounds.y, points(8));
    assert_eq!(layout.pages[1].lines.last().map(|line| line.element), Some(5));
    assert_eq!(layout.pages[1].lines.last().map(|line| line.geometry.y), Some(points(32)));
    assert!(layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .all(|line| !line.text.contains("MORE") && !line.text.contains("CONT'D")));
}

#[test]
fn one_line_budget_overflow_moves_a_normal_block_intact() {
    let mut project = project_with_roles(
        &[
            Role::Action,
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
        ],
        &["before one", "before two", "LEFT", "left", "RIGHT", "right"],
    );
    add_group(&mut project, "wrapper-pagination-line-budget", 2..4, 4..6);
    let inputs = LayoutInputs {
        paper_height: 160,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 3,
        ..LayoutInputs::default()
    };

    let layout = build_with_inputs(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &inputs,
    );

    assert_eq!(layout.pages[0].lines.len(), 2);
    assert!(layout.pages[0].lines.iter().all(|line| line.element < 2));
    let group_lines = layout.pages[1]
        .lines
        .iter()
        .filter(|line| (2..6).contains(&line.element))
        .collect::<Vec<_>>();
    assert_eq!(group_lines.len(), 4);
    assert!(group_lines.iter().all(|line| line.page == 1));
    assert_eq!(group_lines[0].paragraph_bounds.y, points(8));
}

#[test]
fn starts_new_page_and_page_top_space_before_are_applied_once_without_empty_pages() {
    let mut project = project_with_roles(
        &[
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
        ],
        &["before", "LEFT", "left", "RIGHT", "right"],
    );
    add_group(&mut project, "wrapper-pagination-starts-new", 1..3, 3..5);
    project.dual_dialogue_paragraph_overrides.insert(
        project.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                space_before: Some(points(25)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: Some(true),
        },
    );

    let layout = build(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );
    assert_eq!(layout.pages[0].lines.len(), 1);
    assert_eq!(layout.pages[1].lines[0].element, 1);
    assert_eq!(layout.pages[1].lines[0].paragraph_bounds.y, points(72));

    let mut group_first = project;
    group_first.elements.remove(0);
    let first_group = build(
        &group_first,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );
    assert_eq!(first_group.pages.len(), 1);
    assert_eq!(first_group.pages[0].lines[0].element, 0);
    assert_eq!(first_group.pages[0].lines[0].paragraph_bounds.y, points(72));
}

#[test]
fn unequal_side_heights_keep_shared_tops_and_advance_following_content_by_the_taller_side() {
    let mut project = project_with_roles(
        &[
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Parenthetical,
            Role::Action,
        ],
        &["LEFT", "left", "RIGHT", "right", "(beat)", "after"],
    );
    add_group(&mut project, "wrapper-pagination-unequal", 0..2, 2..5);

    let layout = build(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );
    let left = layout.pages[0]
        .lines
        .iter()
        .filter(|line| line.element < 2)
        .collect::<Vec<_>>();
    let right = layout.pages[0]
        .lines
        .iter()
        .filter(|line| (2..5).contains(&line.element))
        .collect::<Vec<_>>();
    assert_eq!(left.len(), 2);
    assert_eq!(right.len(), 3);
    assert_eq!(left[0].paragraph_bounds.y, right[0].paragraph_bounds.y);
    assert_eq!(layout.pages[0].lines.last().map(|line| line.element), Some(5));
    assert_eq!(layout.pages[0].lines.last().map(|line| line.geometry.y), Some(points(108)));
}

#[test]
fn oversized_left_and_right_sides_spill_with_source_pages_and_bounded_forward_progress() {
    for oversized_side in [Role::Character, Role::Dialogue] {
        let roles = if oversized_side == Role::Character {
            [
                Role::Character,
                Role::Dialogue,
                Role::Dialogue,
                Role::Dialogue,
                Role::Dialogue,
                Role::Dialogue,
                Role::Character,
                Role::Dialogue,
                Role::Action,
            ]
        } else {
            [
                Role::Character,
                Role::Dialogue,
                Role::Character,
                Role::Dialogue,
                Role::Dialogue,
                Role::Dialogue,
                Role::Dialogue,
                Role::Dialogue,
                Role::Action,
            ]
        };
        let texts = ["LEFT", "l1", "l2", "l3", "l4", "l5", "RIGHT", "right", "after"];
        let mut project = project_with_roles(&roles, &texts);
        let split = if oversized_side == Role::Character { 6 } else { 2 };
        add_group(&mut project, "wrapper-pagination-oversized", 0..split, split..8);
        let inputs = LayoutInputs {
            paper_height: 40,
            margin_top: 8,
            margin_bottom: 8,
            lines_per_page: 2,
            ..LayoutInputs::default()
        };

        let layout = build_with_inputs(
            &project,
            &SourceLayoutSettings::default(),
            LayoutComposition::BodyEditor,
            &inputs,
        );
        let group_lines = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .filter(|line| line.element < 8)
            .collect::<Vec<_>>();
        assert_eq!(group_lines.len(), 8);
        assert!(group_lines.iter().all(|line| line.oversized));
        assert!(group_lines
            .iter()
            .all(|line| line.source.element == line.element));
        assert!(layout.pages.len() <= 4);
        assert!(layout.pages.iter().enumerate().all(|(index, page)| {
            page.number == index
                && page.lines.iter().all(|line| line.page == index)
        }));
        assert!(layout.pages.iter().flat_map(|page| page.lines.iter()).any(|line| {
            line.element == 8 && line.page + 1 == layout.pages.len()
        }));
    }
}

#[test]
fn both_oversized_sides_spill_without_blank_pages_or_unbounded_retries() {
    let mut project = project_with_roles(
        &[
            Role::Character,
            Role::Dialogue,
            Role::Dialogue,
            Role::Dialogue,
            Role::Dialogue,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Dialogue,
            Role::Dialogue,
            Role::Dialogue,
            Role::Dialogue,
            Role::Action,
        ],
        &[
            "LEFT", "l1", "l2", "l3", "l4", "l5", "RIGHT", "r1", "r2", "r3", "r4", "r5",
            "after",
        ],
    );
    add_group(&mut project, "wrapper-pagination-oversized-both", 0..6, 6..12);
    let inputs = LayoutInputs {
        paper_height: 40,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 2,
        ..LayoutInputs::default()
    };

    let layout = build_with_inputs(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &inputs,
    );
    let group_lines = layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .filter(|line| line.element < 12)
        .collect::<Vec<_>>();
    assert_eq!(group_lines.len(), 12);
    assert!(group_lines.iter().all(|line| line.oversized));
    assert!(group_lines
        .iter()
        .all(|line| line.source.element == line.element));
    assert!(layout.pages.len() <= 4);
    assert!(layout.pages.iter().enumerate().all(|(index, page)| {
        page.number == index && page.lines.iter().all(|line| line.page == index)
    }));
    assert!(layout.pages.iter().flat_map(|page| page.lines.iter()).any(|line| {
        line.element == 12 && line.page + 1 == layout.pages.len()
    }));
}

#[test]
fn ordinary_character_keep_with_next_pagination_remains_unchanged() {
    let project = project_with_roles(
        &[Role::Character, Role::Dialogue, Role::Action],
        &["CUE", "dialogue", "following action"],
    );
    let inputs = LayoutInputs {
        paper_height: 160,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 2,
        ..LayoutInputs::default()
    };

    let layout = build_with_inputs(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &inputs,
    );

    assert_eq!(layout.pages[0].lines.len(), 2);
    assert_eq!(layout.pages[0].lines[0].role, Role::Character);
    assert_eq!(layout.pages[0].lines[1].role, Role::Dialogue);
    assert_eq!(layout.pages[1].lines[0].role, Role::Action);
    assert!(layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .all(|line| !line.text.contains("MORE") && !line.text.contains("CONT'D")));
}

#[test]
fn active_body_group_is_sequential_while_other_groups_and_print_stay_columnar() {
    let mut project = project_with_roles(
        &[
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
        ],
        &["LEFT ONE", "left line", "RIGHT ONE", "right line", "between", "LEFT TWO", "left two", "RIGHT TWO", "right two"],
    );
    add_group(&mut project, "presentation-first", 0..2, 2..4);
    add_group(&mut project, "presentation-second", 5..7, 7..9);
    project.direct_paragraph_overrides.insert(
        project.elements[1].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                left_indent: Some(points(200)),
                right_indent: Some(points(400)),
                alignment: Some(ParagraphAlignment::Right),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );

    let first_id = project.dual_dialogue_groups[0].id.clone();
    let presentation = BodyEditorPresentation::for_project(&project, Some(first_id.clone()));
    let active = build_presented(&project, &SourceLayoutSettings::default(), &presentation);
    let mut sequential_reference = project.clone();
    sequential_reference.dual_dialogue_groups.clear();
    let ordinary = build(
        &sequential_reference,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    );

    assert_eq!(active.active_group_id.as_ref(), Some(&first_id));
    assert_eq!(active.editor_group_boundaries.len(), 1);
    let first_group_lines = active.pages[0]
        .lines
        .iter()
        .filter(|line| line.element < 4)
        .collect::<Vec<_>>();
    for actual in &first_group_lines {
        let expected = ordinary.pages[0]
            .lines
            .iter()
            .find(|line| line.element == actual.element)
            .unwrap_or_else(|| unreachable!("ordinary sequential reference line exists"));
        assert_eq!(actual.paragraph_bounds, expected.paragraph_bounds);
        assert_eq!(actual.geometry, expected.geometry);
    }
    assert_eq!(
        active.pages[0].lines.iter().find(|line| line.element == 1).map(|line| line.paragraph_bounds.x),
        Some(points(200)),
        "direct indents and alignment apply in the active sequential group",
    );
    let second_left = active.pages.iter().flat_map(|page| &page.lines)
        .find(|line| line.element == 5)
        .unwrap_or_else(|| unreachable!("inactive left cue is present"));
    let second_right = active.pages.iter().flat_map(|page| &page.lines)
        .find(|line| line.element == 7)
        .unwrap_or_else(|| unreachable!("inactive right cue is present"));
    assert_ne!(second_left.paragraph_bounds.x, second_right.paragraph_bounds.x);
    assert_eq!(second_left.paragraph_bounds.y, second_right.paragraph_bounds.y);

    let boundary = &active.editor_group_boundaries[0];
    assert_eq!(boundary.group_id, first_id);
    assert_eq!(boundary.upper_page, 0);
    assert_eq!(boundary.upper.y, first_group_lines[0].paragraph_bounds.y);
    let final_right = active.pages.iter().flat_map(|page| &page.lines)
        .filter(|line| line.element == 3)
        .last()
        .unwrap_or_else(|| unreachable!("final right member has a line"));
    assert_eq!(boundary.lower_page, final_right.page);
    assert_eq!(
        boundary.lower.y,
        FixedPoint::from_raw(final_right.paragraph_bounds.y.raw() + final_right.paragraph_bounds.height.raw()),
    );
    assert!(boundary.upper.height.raw() == 0 && boundary.lower.height.raw() == 0);

    let print = build(
        &project,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print { include_title_page: false },
    );
    assert_eq!(print.active_group_id, None);
    assert!(print.editor_group_boundaries.is_empty());
    let print_left = print.pages.iter().flat_map(|page| &page.lines)
        .find(|line| line.element == 0)
        .unwrap_or_else(|| unreachable!("Print left member remains present"));
    let print_right = print.pages.iter().flat_map(|page| &page.lines)
        .find(|line| line.element == 2)
        .unwrap_or_else(|| unreachable!("Print right member remains present"));
    assert_ne!(print_left.paragraph_bounds.x, print_right.paragraph_bounds.x);
    assert_eq!(print_left.paragraph_bounds.y, print_right.paragraph_bounds.y);
}

#[test]
fn presentation_cache_tracks_modes_edits_replacement_and_print_isolation() {
    let mut project = project_with_roles(
        &[
            Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
            Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
        ],
        &["LEFT", "left text", "RIGHT", "right text", "LEFT TWO", "left two", "RIGHT TWO", "right two"],
    );
    add_group(&mut project, "presentation-cache-group", 0..2, 2..4);
    add_group(&mut project, "presentation-cache-group-two", 4..6, 6..8);
    let group_id = project.dual_dialogue_groups[0].id.clone();
    let inputs = LayoutInputs::default();
    let inactive = BodyEditorPresentation::for_project(&project, None);
    let active = BodyEditorPresentation::for_project(&project, Some(group_id.clone()));
    let mut cache = LayoutCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();

    let (dual, dual_hit) = valid(cache.get_or_build_body_editor_with_diagnostics(
        4, &project, &inputs, &inactive, &mut diagnostics,
    ));
    assert!(!dual_hit);
    assert_eq!(diagnostics.layout_builds, 1);
    let (sequential, sequential_hit) = valid(cache.get_or_build_body_editor_with_diagnostics(
        4, &project, &inputs, &active, &mut diagnostics,
    ));
    assert!(!sequential_hit);
    assert_eq!(diagnostics.layout_builds, 1);
    assert_eq!(diagnostics.element_wrap_misses, 4);
    assert_eq!(diagnostics.element_wraps_reused, 4);
    assert_eq!(sequential.active_group_id.as_ref(), Some(&group_id));
    let (repeated, repeated_hit) = valid(cache.get_or_build_body_editor_with_diagnostics(
        4, &project, &inputs, &active, &mut diagnostics,
    ));
    assert!(repeated_hit);
    assert_eq!(diagnostics.layout_builds, 0);
    assert_eq!(sequential, repeated);
    let fresh = build_presented(&project, &SourceLayoutSettings::default(), &active);
    assert_eq!(*sequential, fresh);

    let (_, switched_back_hit) = valid(cache.get_or_build_body_editor(
        4, &project, &inputs, &inactive,
    ));
    assert!(!switched_back_hit);
    let (_, unchanged_hit) = valid(cache.get_or_build_body_editor(
        4, &project, &inputs, &inactive,
    ));
    assert!(unchanged_hit);

    let mut edited_group = project.clone();
    edited_group.dual_dialogue_groups[0].id = valid(StableId::new("presentation-cache-edited-group"));
    let (edited_layout, edited_hit) = valid(cache.get_or_build_body_editor(
        4, &edited_group, &inputs, &active,
    ));
    assert!(!edited_hit);
    assert_eq!(edited_layout.active_group_id, None);
    assert!(edited_layout.editor_group_boundaries.is_empty());

    let mut wrapper_edited = edited_group.clone();
    let edited_id = wrapper_edited.dual_dialogue_groups[0].id.clone();
    wrapper_edited.dual_dialogue_paragraph_overrides.insert(
        edited_id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                left_indent: Some(points(120)),
                ..SourceParagraphSpec::default()
            },
            starts_new_page: None,
        },
    );
    let wrapper_active = BodyEditorPresentation::for_project(&wrapper_edited, Some(edited_id));
    let (_, wrapper_edit_hit) = valid(cache.get_or_build_body_editor(
        4, &wrapper_edited, &inputs, &wrapper_active,
    ));
    assert!(!wrapper_edit_hit);

    let (print_after_body, print_hit) = valid(cache.get_or_build_with_composition(
        4,
        &project,
        &inputs,
        LayoutComposition::Print { include_title_page: false },
    ));
    assert!(!print_hit);
    assert_eq!(print_after_body.active_group_id, None);
    assert!(print_after_body.editor_group_boundaries.is_empty());
    let fresh_print = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print { include_title_page: false },
    ));
    assert_eq!(*print_after_body, fresh_print);
    assert_eq!(dual.active_group_id, None);
}

#[test]
fn unrelated_body_edit_rebuilds_identity_but_reuses_only_unchanged_wraps() {
    let mut project = project_with_roles(
        &[
            Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
            Role::Action,
        ],
        &["LEFT", "left", "RIGHT", "right", "outside before"],
    );
    add_group(&mut project, "unrelated-cache-group", 0..2, 2..4);
    let inputs = LayoutInputs::default();
    let group_id = project.dual_dialogue_groups[0].id.clone();
    let presentation = BodyEditorPresentation::for_project(&project, Some(group_id.clone()));
    let mut cache = LayoutCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let _ = valid(cache.get_or_build_body_editor_with_diagnostics(
        11, &project, &inputs, &presentation, &mut diagnostics,
    ));

    let mut unrelated_edit = project.clone();
    unrelated_edit.elements[4].text = "outside after".to_owned();
    let edited_presentation = BodyEditorPresentation::for_project(&unrelated_edit, Some(group_id));
    let (layout, cache_hit) = valid(cache.get_or_build_body_editor_with_diagnostics(
        12, &unrelated_edit, &inputs, &edited_presentation, &mut diagnostics,
    ));

    assert!(!cache_hit, "the complete semantic signature changed");
    assert_eq!(layout.editor_group_boundaries.len(), 1);
    assert!(diagnostics.element_wraps_reused > 0);
    assert!(diagnostics.element_wrap_misses > 0);
}

#[test]
fn replacement_and_stale_active_ids_do_not_resurrect_groups_and_indexes_round_trip_both_modes() {
    let mut project = project_with_roles(
        &[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue],
        &["LEFT", "left", "RIGHT", "right"],
    );
    add_group(&mut project, "presentation-replacement-group", 0..2, 2..4);
    let group_id = project.dual_dialogue_groups[0].id.clone();
    let stale_presentation = BodyEditorPresentation::for_project(&project, Some(group_id.clone()));
    let inactive = build_presented(&project, &SourceLayoutSettings::default(), &BodyEditorPresentation::default());
    let active = build_presented(&project, &SourceLayoutSettings::default(), &stale_presentation);
    let mut cache = LayoutCache::default();
    let inputs = LayoutInputs::default();
    let (_, _) = valid(cache.get_or_build_body_editor(1, &project, &inputs, &stale_presentation));

    let mut replacement = valid(Project::new());
    replacement.elements = project.elements.clone();
    replacement.dual_dialogue_groups = project.dual_dialogue_groups.clone();
    let (replaced, replaced_hit) = valid(cache.get_or_build_body_editor(
        1, &replacement, &inputs, &stale_presentation,
    ));
    assert!(!replaced_hit);
    assert_eq!(replaced.active_group_id, None);
    assert!(replaced.editor_group_boundaries.is_empty());
    let replacement_active = BodyEditorPresentation::for_project(&replacement, Some(group_id));
    let (replacement_layout, replacement_hit) = valid(cache.get_or_build_body_editor(
        1, &replacement, &inputs, &replacement_active,
    ));
    assert!(!replacement_hit);
    assert!(replacement_layout.active_group_id.is_some());

    let layouts = [(&inactive, false), (&active, true)];
    for (layout, is_sequential) in layouts {
        let left = layout.pages[0].lines.iter().find(|line| line.element == 0)
            .unwrap_or_else(|| unreachable!("left cue is indexed"));
        let right = layout.pages[0].lines.iter().find(|line| line.element == 2)
            .unwrap_or_else(|| unreachable!("right cue is indexed"));
        if is_sequential {
            assert_ne!(left.paragraph_bounds.y, right.paragraph_bounds.y);
        } else {
            assert_eq!(left.paragraph_bounds.y, right.paragraph_bounds.y);
            assert_eq!(layout.pages[0].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [0, 1, 2, 3]);
        }

        let left_hit = layout.nearest_line_hit_test_fixed(
            0, points(120), left.paragraph_bounds.y,
        ).unwrap_or_else(|| unreachable!("left column hit test returns a caret"));
        assert_eq!(left_hit.element, 0);
        if !is_sequential {
            let right_hit = layout.nearest_line_hit_test_fixed(
                0, points(350), right.paragraph_bounds.y,
            ).unwrap_or_else(|| unreachable!("right column hit test returns a caret"));
            assert_eq!(right_hit.element, 2);
            let mut location_tie_diagnostics = InteractionQueryDiagnostics::default();
            let tied_location = layout.nearest_line_location_fixed_with_diagnostics(
                0,
                points(324),
                left.paragraph_bounds.y,
                &mut location_tie_diagnostics,
            ).unwrap_or_else(|| unreachable!("equal-distance query locates a dual-dialogue line"));
            assert_eq!(tied_location.page_index, 0);
            assert_eq!(tied_location.line.element, 0);
            let mut tie_diagnostics = InteractionQueryDiagnostics::default();
            let tied = layout.nearest_line_hit_test_fixed_with_diagnostics(
                0, points(324), left.paragraph_bounds.y, &mut tie_diagnostics,
            ).unwrap_or_else(|| unreachable!("equal-distance hit test returns a caret"));
            assert_eq!(tied.element, 0, "equal horizontal distances choose document order");
            assert!(tie_diagnostics.equal_y_candidates_examined >= 2);
            assert_eq!(tie_diagnostics.index_queries, 1);
            assert_eq!(location_tie_diagnostics, tie_diagnostics);
        }

        for line in layout.pages.iter().flat_map(|page| &page.lines) {
            let start = line.source.start;
            let display_start = line.display_char_for_source(start, false);
            assert_eq!(line.source_for_display_char(display_start), start);
            let caret = scriptcraft_core::layout::Caret { element: line.element, offset: start };
            assert!(layout.caret_geometry(caret.clone()).is_ok());
            let mut index_diagnostics = InteractionQueryDiagnostics::default();
            assert!(layout.page_for_position_with_diagnostics(
                caret.element, caret.offset, &mut index_diagnostics,
            ).is_some());
            assert!(layout.body_line_index_for_position_with_diagnostics(
                caret.element, caret.offset, &mut index_diagnostics,
            ).is_some());
            assert!(index_diagnostics.index_queries >= 2);
        }
    }
}

#[test]
fn cue_only_shapes_layout_only_real_members_and_active_boundary_ends_at_right_cue() {
    let cases: &[(&str, &[Role], &[&str], Range<usize>, Range<usize>)] = &[
        ("cc", &[Role::Character, Role::Character], &["LEFT", "RIGHT"], 0..1, 1..2),
        ("ccd", &[Role::Character, Role::Character, Role::Dialogue], &["LEFT", "RIGHT", "right words"], 0..1, 1..3),
        ("cdc", &[Role::Character, Role::Dialogue, Role::Character], &["LEFT", "left words", "RIGHT"], 0..2, 2..3),
        ("ccp", &[Role::Character, Role::Character, Role::Parenthetical], &["LEFT", "RIGHT", "(right beat)"], 0..1, 1..3),
        ("cpc", &[Role::Character, Role::Parenthetical, Role::Character], &["LEFT", "(left beat)", "RIGHT"], 0..2, 2..3),
        ("prior-multi", &[Role::Character, Role::Dialogue, Role::Parenthetical, Role::Character, Role::Dialogue, Role::Dialogue], &["LEFT", "left", "(beat)", "RIGHT", "right one", "right two"], 0..3, 3..6),
    ];

    for (name, roles, texts, left_range, right_range) in cases {
        let mut project = project_with_roles(roles, texts);
        add_group(&mut project, &format!("empty-side-{name}"), left_range.clone(), right_range.clone());
        valid(project.validate());
        let group = &project.dual_dialogue_groups[0];
        let inactive = build(&project, &SourceLayoutSettings::default(), LayoutComposition::BodyEditor);
        let page = &inactive.pages[0];
        let actual_indices = group.member_ids().filter_map(|id| project.elements.iter().position(|element| &element.id == id)).collect::<Vec<_>>();
        let group_lines = page.lines.iter().filter(|line| actual_indices.contains(&line.element)).collect::<Vec<_>>();
        assert_eq!(group_lines.iter().map(|line| line.element).collect::<Vec<_>>(), actual_indices, "{name} only lays out actual flat members");
        assert!(group_lines.iter().all(|line| line.role != Role::General));
        let left_cue = group_lines.iter().find(|line| line.element == left_range.start).copied();
        let right_cue = group_lines.iter().find(|line| line.element == right_range.start).copied();
        let (Some(left_cue), Some(right_cue)) = (left_cue, right_cue) else {
            unreachable!("{name} retains both real cue lines")
        };
        assert_eq!(left_cue.paragraph_bounds.x, points(108), "{name} left column");
        assert_eq!(left_cue.paragraph_bounds.width, points(198), "{name} left column width");
        assert_eq!(right_cue.paragraph_bounds.x, points(342), "{name} right column");
        assert_eq!(right_cue.paragraph_bounds.width, points(198), "{name} right column width");
        assert_eq!(left_cue.paragraph_bounds.y, right_cue.paragraph_bounds.y, "{name} shared top");
        assert_eq!(left_cue.alignment, ParagraphAlignment::Center, "{name} keeps effective cue alignment");
        assert_eq!(right_cue.alignment, ParagraphAlignment::Center, "{name} keeps effective cue alignment");
        assert_eq!(left_cue.unaligned_line_left, left_cue.paragraph_bounds.x, "{name} left anchor is pre-alignment");
        assert_eq!(right_cue.unaligned_line_left, right_cue.paragraph_bounds.x, "{name} right anchor is pre-alignment");
        assert!(left_cue.geometry.x > left_cue.paragraph_bounds.x, "{name} centers the left cue");
        assert!(right_cue.geometry.x > right_cue.paragraph_bounds.x, "{name} centers the right cue");

        let left_hit = inactive.nearest_line_hit_test_fixed(0, points(150), left_cue.paragraph_bounds.y)
            .unwrap_or_else(|| unreachable!("{name} left cue is hit-testable"));
        let right_hit = inactive.nearest_line_hit_test_fixed(0, points(380), right_cue.paragraph_bounds.y)
            .unwrap_or_else(|| unreachable!("{name} right cue is hit-testable"));
        assert_eq!(left_hit.element, left_range.start, "{name} left hit targets a real cue");
        assert_eq!(right_hit.element, right_range.start, "{name} right hit targets a real cue");
        let blank_hit = inactive.nearest_line_hit_test_fixed(0, points(380), FixedPoint::from_raw(right_cue.paragraph_bounds.y.raw() + points(30).raw()))
            .unwrap_or_else(|| unreachable!("nearest-line fallback may select a real nearby line"));
        assert!(actual_indices.contains(&blank_hit.element), "{name} blank-space fallback never invents a member");

        let active_id = group.id.clone();
        let active = build_presented(&project, &SourceLayoutSettings::default(), &BodyEditorPresentation::for_project(&project, Some(active_id.clone())));
        let boundary = active.editor_group_boundaries.iter().find(|boundary| boundary.group_id == active_id)
            .unwrap_or_else(|| unreachable!("{name} active group has its editor boundary"));
        let actual_right_last = right_range.end - 1;
        let final_right = active.pages.iter().flat_map(|page| &page.lines).filter(|line| line.element == actual_right_last).last()
            .unwrap_or_else(|| unreachable!("{name} right final member has real sequential geometry"));
        assert_eq!(boundary.lower_page, final_right.page, "{name} lower boundary page");
        assert_eq!(boundary.lower.y, FixedPoint::from_raw(final_right.paragraph_bounds.y.raw() + final_right.paragraph_bounds.height.raw()), "{name} lower boundary follows actual right final member");

        let mut cache = LayoutCache::default();
        let (cached_inactive, inactive_hit) = valid(cache.get_or_build_body_editor(1, &project, &LayoutInputs::default(), &BodyEditorPresentation::for_project(&project, None)));
        let (cached_active, active_hit) = valid(cache.get_or_build_body_editor(1, &project, &LayoutInputs::default(), &BodyEditorPresentation::for_project(&project, Some(active_id.clone()))));
        assert!(!inactive_hit && !active_hit, "{name} cache distinguishes inactive columns from active sequential mode");
        assert_eq!(*cached_inactive, inactive);
        assert_eq!(*cached_active, active);

        let print = build(&project, &SourceLayoutSettings::default(), LayoutComposition::Print { include_title_page: false });
        assert_eq!(print.active_group_id, None, "{name} Print has no active state");
        assert!(print.editor_group_boundaries.is_empty(), "{name} Print has no dotted boundaries");
        assert!(print.pages.iter().flat_map(|page| &page.lines).filter(|line| actual_indices.contains(&line.element)).all(|line| actual_indices.contains(&line.source.element)), "{name} Print source mapping is a real member");
    }
}

#[test]
fn cue_only_dual_blocks_fit_move_and_spill_by_actual_lines() {
    let mut fitting = project_with_roles(&[Role::Action, Role::Character, Role::Character], &["before", "LEFT", "RIGHT"]);
    add_group(&mut fitting, "cue-only-fit", 1..2, 2..3);
    let fits = build_with_inputs(
        &fitting,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &LayoutInputs { lines_per_page: 3, ..LayoutInputs::default() },
    );
    assert_eq!(fits.pages.len(), 1, "one preceding line plus two actual cue lines fits");
    assert_eq!(fits.pages[0].occupied_lines, 3);
    assert_eq!(fits.pages[0].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [0, 1, 2]);

    let moves = build_with_inputs(
        &fitting,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &LayoutInputs { lines_per_page: 2, ..LayoutInputs::default() },
    );
    assert_eq!(moves.pages.len(), 2, "the C,C block moves atomically when only one occupied line remains");
    assert_eq!(moves.pages[0].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [0]);
    assert_eq!(moves.pages[1].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(moves.pages[1].occupied_lines, 2);

    let mut explicit_start = fitting.clone();
    explicit_start.dual_dialogue_paragraph_overrides.insert(
        explicit_start.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides { starts_new_page: Some(true), ..SourceParagraphOverrides::default() },
    );
    let page_started = build(&explicit_start, &SourceLayoutSettings::default(), LayoutComposition::BodyEditor);
    assert_eq!(page_started.pages.len(), 2, "explicit group page-start applies to the actual C,C block");
    assert_eq!(page_started.pages[0].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [0]);
    assert_eq!(page_started.pages[1].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [1, 2]);

    let mut oversized = project_with_roles(
        &[Role::Character, Role::Dialogue, Role::Dialogue, Role::Dialogue, Role::Character],
        &["LEFT", "one two three four five six seven", "more words", "last words", "RIGHT"],
    );
    add_group(&mut oversized, "cue-only-oversized", 0..4, 4..5);
    let spilled = build_with_inputs(
        &oversized,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
        &LayoutInputs { paper_height: 40, margin_top: 8, margin_bottom: 8, lines_per_page: 2, ..LayoutInputs::default() },
    );
    let spilled_group = spilled.pages.iter().flat_map(|page| &page.lines).filter(|line| line.element < 5).collect::<Vec<_>>();
    assert!(spilled.pages.len() > 1, "oversized actual members advance across pages");
    assert!(!spilled_group.is_empty());
    assert!(spilled_group.iter().all(|line| line.oversized), "only the oversized fallback marks member lines");
    assert!(spilled_group.iter().all(|line| (0..5).contains(&line.element)), "spilling creates no synthetic element index");
    assert!(spilled.pages.iter().all(|page| page.occupied_lines <= 2), "spilled actual line count respects page capacity");
}
