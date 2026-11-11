#![forbid(unsafe_code)]

use scriptcraft_core::dual_dialogue::{DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance};
use scriptcraft_core::exports::{export_pdf, export_pdf_with_options, export_text, PdfExportOptions};
use scriptcraft_core::fdx::read_v6;
use scriptcraft_core::formatting::{SourceLayoutSettings, SourceParagraphOverrides, SourceParagraphSpec};
use scriptcraft_core::layout::{LayoutComposition, ScreenplayLayout};
use scriptcraft_core::{FixedPoint, LayoutInputs, Project, Role, ScreenplayElement, StableId};
use std::ops::Range;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("layout/export scenario is valid"))
}

fn add_group(project: &mut Project, id: &str, left: Range<usize>, right: Range<usize>) {
    let side = |range: Range<usize>| DualDialogueSide {
        character: project.elements[range.start].id.clone(),
        paragraphs: project.elements[range.start + 1..range.end]
            .iter()
            .map(|item| item.id.clone())
            .collect(),
    };
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: valid(StableId::new(id)),
        left: side(left),
        right: side(right),
        wrapper: DualDialogueWrapperProvenance::default(),
    });
}

fn project_with(roles: &[(Role, &str)]) -> Project {
    let mut project = valid(Project::new());
    project.elements = roles
        .iter()
        .enumerate()
        .map(|(index, (role, text))| valid(ScreenplayElement::new(format!("layout-{index}"), *role, *text)))
        .collect();
    project
}

fn build(project: &Project, composition: LayoutComposition, inputs: &LayoutInputs) -> ScreenplayLayout {
    let source = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_default();
    valid(ScreenplayLayout::build_with_composition(project, inputs, &source, composition))
}

fn pdf_text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or_else(|_| unreachable!("exported PDF text is ASCII"))
}

fn extracted_pdf_text(bytes: &[u8]) -> String {
    let mut output = String::new();
    let mut remaining = pdf_text(bytes);
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
fn layout_is_repeatable_and_maps_wrapped_utf8_display_back_to_canonical_bytes() {
    let mut project = project_with(&[(Role::Action, &format!("{}éZ", "a".repeat(60)))]);
    let original = project.clone();
    let inputs = LayoutInputs::default();
    let composition = LayoutComposition::Print { include_title_page: false };
    let first = build(&project, composition, &inputs);
    let second = build(&project, composition, &inputs);
    assert_eq!(first, second);
    let lines = first.pages.iter().flat_map(|page| &page.lines).filter(|line| line.element == 0).collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].source.start, 0);
    assert_eq!(lines[0].source.end, 62);
    assert_eq!(lines[1].source.start, 62);
    assert_eq!(lines[1].source.end, 63);
    assert_eq!(lines[0].source_for_display_char(60), 60);
    assert_eq!(lines[0].source_for_display_char(61), 62);
    assert_eq!(project, original);
}

#[test]
fn dual_block_exact_fit_and_one_fixed_point_unit_overflow_paginate_atomically() {
    let mut fitting = project_with(&[
        (Role::Character, "LEFT"), (Role::Dialogue, "left"),
        (Role::Character, "RIGHT"), (Role::Dialogue, "right"),
    ]);
    add_group(&mut fitting, "fit", 0..2, 2..4);
    let compact = LayoutInputs { paper_height: 40, margin_top: 8, margin_bottom: 8, lines_per_page: 2, ..LayoutInputs::default() };
    let layout = build(&fitting, LayoutComposition::BodyEditor, &compact);
    assert_eq!(layout.pages.len(), 1);
    assert_eq!(layout.pages[0].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [0, 1, 2, 3]);

    let mut overflow = project_with(&[
        (Role::Action, "before"), (Role::Character, "LEFT"), (Role::Dialogue, "left"),
        (Role::Character, "RIGHT"), (Role::Dialogue, "right"), (Role::Action, "after"),
    ]);
    add_group(&mut overflow, "overflow", 1..3, 3..5);
    overflow.dual_dialogue_paragraph_overrides.insert(
        overflow.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec { space_before: Some(FixedPoint::from_raw(1)), ..SourceParagraphSpec::default() },
            starts_new_page: None,
        },
    );
    let tight = LayoutInputs { paper_height: 52, margin_top: 8, margin_bottom: 8, ..LayoutInputs::default() };
    let moved = build(&overflow, LayoutComposition::BodyEditor, &tight);
    assert_eq!(moved.pages[0].lines.iter().map(|line| line.element).collect::<Vec<_>>(), [0]);
    let members = moved.pages[1].lines.iter().filter(|line| (1..5).contains(&line.element)).collect::<Vec<_>>();
    assert_eq!(members.len(), 4);
    assert!(members.iter().all(|line| line.page == 1 && line.source.element == line.element));
    assert_eq!(members[0].paragraph_bounds.y, FixedPoint::from_points(8));
    assert_eq!(moved.pages[1].lines.last().map(|line| line.element), Some(5));
}

#[test]
fn title_page_and_dual_columns_keep_separate_geometry_and_print_order() {
    let project = valid(read_v6(include_bytes!("fixtures/dual_dialogue_basic.fdx"))).project;
    let inputs = LayoutInputs::default();
    let body = build(&project, LayoutComposition::BodyEditor, &inputs);
    let title = build(&project, LayoutComposition::TitlePageEditor, &inputs);
    let print = build(&project, LayoutComposition::Print { include_title_page: true }, &inputs);
    assert!(body.pages.iter().all(|page| !page.title_page));
    assert!(title.pages.iter().all(|page| page.title_page));
    assert!(print.pages.first().is_some_and(|page| page.title_page));
    assert!(print.pages.iter().skip(1).all(|page| !page.title_page));

    let left = project.elements.iter().position(|item| item.id == project.dual_dialogue_groups[0].left.character).expect("left cue exists");
    let right = project.elements.iter().position(|item| item.id == project.dual_dialogue_groups[0].right.character).expect("right cue exists");
    let side_lines = body.pages.iter().flat_map(|page| &page.lines).collect::<Vec<_>>();
    let left_line = side_lines.iter().find(|line| line.element == left).expect("left cue lays out");
    let right_line = side_lines.iter().find(|line| line.element == right).expect("right cue lays out");
    assert_eq!(left_line.paragraph_bounds.x, FixedPoint::from_points(108));
    assert_eq!(right_line.paragraph_bounds.x, FixedPoint::from_points(333));
    assert_eq!(left_line.paragraph_bounds.y, right_line.paragraph_bounds.y);
}

#[test]
fn pdf_options_hidden_roles_plain_text_loss_and_non_ascii_limit_are_explicit_and_borrow_only() {
    let mut project = project_with(&[
        (Role::Summary, "HIDDEN_SUMMARY_SENTINEL"),
        (Role::Action, "Visible café — script"),
    ]);
    let before = project.clone();
    let full_pdf = valid(export_pdf(&project));
    let body_pdf = valid(export_pdf_with_options(&project, PdfExportOptions { include_title_page: false }));
    let full_text = extracted_pdf_text(&full_pdf);
    let body_text = extracted_pdf_text(&body_pdf);
    assert!(full_text.contains("SCRIPT TITLE"));
    assert!(full_text.find("SCRIPT TITLE").unwrap_or(usize::MAX) < full_text.find("Visible").unwrap_or(usize::MAX));
    assert!(!body_text.contains("SCRIPT TITLE"));
    assert!(body_text.contains("Visible"));
    assert!(body_text.contains("caf"));
    assert!(body_text.contains('?'), "PDF's documented non-ASCII substitution is visible");

    let plain = valid(export_text(&project));
    assert!(plain.text.contains("Visible café"));
    assert!(!plain.text.contains("HIDDEN_SUMMARY_SENTINEL"));
    assert!(plain.loss.page_layout_lost);
    assert_eq!(project, before);
    assert_eq!(valid(export_pdf(&project)), full_pdf);
}

#[test]
fn plain_text_exports_each_dual_side_in_order_without_synthetic_wrapper_text() {
    let imported = valid(read_v6(include_bytes!("fixtures/dual_dialogue_basic.fdx")));
    let output = valid(export_text(&imported.project));
    let left = output.text.find("First half").expect("left dialogue appears");
    let right = output.text.find("你好").expect("right dialogue appears");
    assert!(left < right);
    assert!(!output.text.contains("DualDialogue"));
}
