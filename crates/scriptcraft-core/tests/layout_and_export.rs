#![forbid(unsafe_code)]

use scriptcraft_core::dual_dialogue::{DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_pdf_with_options, export_text, PdfExportOptions};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::formatting::{
    ParagraphAlignment, SourceLayoutSettings, SourceParagraphOverrides, SourceParagraphSpec,
};
use scriptcraft_core::layout::{
    BodyEditorPresentation, Caret, InteractionQueryDiagnostics, LayoutBuildDiagnostics, LayoutCache,
    LayoutComposition, LayoutPurpose, ScreenplayLayout,
};
use scriptcraft_core::{
    FixedPoint, LayoutInputs, Project, Role, RunAttribute, ScreenplayElement, StableId, TextRun,
};
use std::ops::Range;
use std::sync::Arc;

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
        .map(|(index, (role, text))| scriptcraft_core::BodyOwner::shared(valid(ScreenplayElement::new(format!("layout-{index}"), *role, *text))))
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

fn assert_revision_reuse_matches_fresh(
    project: &Project,
    composition: LayoutComposition,
    inputs: &LayoutInputs,
    fresh: &ScreenplayLayout,
) {
    let mut cache = LayoutCache::default();
    let (cold, cold_hit) = valid(cache.get_or_build_with_composition(0, project, inputs, composition));
    let (reused, reused_hit) = valid(cache.get_or_build_with_composition(1, project, inputs, composition));
    assert!(!cold_hit);
    assert!(!reused_hit);
    assert_eq!(cold.as_ref(), fresh);
    assert_eq!(reused.as_ref(), fresh);
    assert!(!Arc::ptr_eq(&cold, &reused));
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
fn ordinary_lines_preserve_pages_mappings_indexes_and_cached_print_output() {
    let first = "a".repeat(70);
    let second = "b".repeat(70);
    let project = project_with(&[
        (Role::Action, &first),
        (Role::Action, &second),
        (Role::Action, "after"),
    ]);
    let inputs = LayoutInputs {
        paper_height: 52,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 3,
        ..LayoutInputs::default()
    };
    let layout = build(&project, LayoutComposition::BodyEditor, &inputs);
    assert_eq!(layout.pages.len(), 2);
    assert_eq!(layout.pages[0].occupied_lines, 3);
    assert_eq!(layout.pages[1].occupied_lines, 2);

    let lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .collect::<Vec<_>>();
    assert_eq!(
        lines.iter().map(|line| line.element).collect::<Vec<_>>(),
        [0, 0, 1, 1, 2]
    );
    assert_eq!(
        lines.iter().map(|line| line.page).collect::<Vec<_>>(),
        [0, 0, 0, 1, 1]
    );
    assert_eq!(
        lines
            .iter()
            .map(|line| (line.source.start, line.source.end))
            .collect::<Vec<_>>(),
        [(0, 61), (61, 70), (0, 61), (61, 70), (0, 5)]
    );

    for (body_index, line) in lines.iter().enumerate() {
        let caret = Caret {
            element: line.element,
            offset: line.source.start.saturating_add(1),
        };
        let mut diagnostics = InteractionQueryDiagnostics::default();
        let located = valid(layout.line_for_caret_with_diagnostics(&caret, &mut diagnostics));
        assert_eq!(located.page_index, line.page);
        assert_eq!(located.line, *line);
        assert_eq!(layout.body_line(body_index), Some(*line));
        assert_eq!(
            layout.page_for_position_with_diagnostics(
                caret.element,
                caret.offset,
                &mut diagnostics,
            ),
            Some(line.page)
        );
        assert_eq!(line.display_char_for_source(line.source.start, false), 0);
        assert_eq!(line.source_for_display_char(0), line.source.start);
        assert_eq!(
            line.source_for_display_char(line.display_text.chars().count()),
            line.source.end
        );
        assert_eq!(
            line.spans
                .iter()
                .map(|span| span.display_text.as_ref())
                .collect::<String>(),
            line.display_text.as_ref()
        );
        assert!(line.spans.iter().all(|span| {
            span.source.element == line.element && span.geometry.y == line.geometry.y
        }));
    }

    let source = SourceLayoutSettings::default();
    let mut cache = LayoutCache::default();
    let (fresh, fresh_hit) = valid(cache.get_or_build_with_composition(
        11,
        &project,
        &inputs,
        LayoutComposition::BodyEditor,
    ));
    assert!(!fresh_hit);
    assert_eq!(fresh.as_ref(), &layout);
    let (cached, cached_hit) = valid(cache.get_or_build_with_composition(
        11,
        &project,
        &inputs,
        LayoutComposition::BodyEditor,
    ));
    assert!(cached_hit);
    assert!(Arc::ptr_eq(&fresh, &cached));
    assert_eq!(cached, fresh);

    let print_composition = LayoutComposition::Print {
        include_title_page: false,
    };
    let print = build(&project, print_composition, &inputs);
    let mut print_cache = LayoutCache::default();
    let (fresh_print, print_fresh_hit) = valid(print_cache.get_or_build_with_composition(
        11,
        &project,
        &inputs,
        print_composition,
    ));
    assert!(!print_fresh_hit);
    assert_eq!(fresh_print.as_ref(), &print);
    let (cached_print, print_cached_hit) = valid(print_cache.get_or_build_with_composition(
        11,
        &project,
        &inputs,
        print_composition,
    ));
    assert!(print_cached_hit);
    assert!(Arc::ptr_eq(&fresh_print, &cached_print));
    assert_eq!(cached_print, fresh_print);

    let options = PdfExportOptions {
        include_title_page: false,
    };
    let pdf = valid(export_pdf_with_options(&project, options));
    assert_eq!(valid(export_pdf_with_options(&project, options)), pdf);
}

#[test]
fn oversized_ordinary_element_preserves_spill_geometry_and_source_order() {
    let text = "x".repeat(400);
    let project = project_with(&[(Role::Action, &text)]);
    let inputs = LayoutInputs {
        paper_height: 40,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 2,
        ..LayoutInputs::default()
    };
    let layout = build(&project, LayoutComposition::BodyEditor, &inputs);
    let lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .collect::<Vec<_>>();
    assert!(lines.len() > inputs.lines_per_page as usize);
    assert!(layout.pages.len() > 1);
    assert!(lines.iter().all(|line| line.oversized));

    let mut source_offset = 0;
    let mut display_text = String::new();
    for line in lines {
        assert_eq!(line.source.start, source_offset);
        source_offset = line.source.end;
        display_text.push_str(&line.display_text);
        assert_eq!(line.display_char_for_source(line.source.start, false), 0);
        assert_eq!(
            line.source_for_display_char(line.display_text.chars().count()),
            line.source.end
        );
    }
    assert_eq!(source_offset, text.len());
    assert_eq!(display_text, text);

    for (page_index, page) in layout.pages.iter().enumerate() {
        assert_eq!(page.number, page_index);
        assert_eq!(page.occupied_lines as usize, page.lines.len());
        assert!(page.lines.len() <= inputs.lines_per_page as usize);
        for (line_index, line) in page.lines.iter().enumerate() {
            assert_eq!(line.page, page_index);
            assert_eq!(
                line.geometry.y,
                FixedPoint::from_points(8 + 12 * line_index as i64)
            );
        }
    }
    assert_revision_reuse_matches_fresh(
        &project,
        LayoutComposition::BodyEditor,
        &inputs,
        &layout,
    );
}

#[test]
fn expansion_wrapping_preserves_alignment_spill_cache_interaction_and_exports() {
    let plain = "plain é🙂 words ".repeat(12);
    let expanded = "ßﬃ styled words ".repeat(12);
    let tail = "\r\nnext line\nlast line ".repeat(8);
    let text = format!("{plain}{expanded}{tail}");
    let mut project = project_with(&[(Role::Action, &text)]);
    scriptcraft_core::BodyOwner::update(&mut project.elements[0], |element| {
        element.runs = vec![
            TextRun {
                text: plain,
                attributes: Vec::new(),
            },
            TextRun {
                text: expanded,
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold+AllCaps".into(),
                }],
            },
            TextRun {
                text: tail,
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Italic".into(),
                }],
            },
        ];
    });
    let inputs = LayoutInputs {
        paper_height: 40,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 2,
        ..LayoutInputs::default()
    };

    for alignment in [
        ParagraphAlignment::Left,
        ParagraphAlignment::Right,
        ParagraphAlignment::Center,
        ParagraphAlignment::Justified,
    ] {
        let mut aligned = project.clone();
        aligned.direct_paragraph_overrides.insert(
            aligned.elements[0].id.clone(),
            SourceParagraphOverrides {
                paragraph: SourceParagraphSpec {
                    alignment: Some(alignment),
                    ..SourceParagraphSpec::default()
                },
                starts_new_page: None,
            },
        );
        let before = aligned.clone();
        let fresh = build(&aligned, LayoutComposition::BodyEditor, &inputs);
        let lines = fresh
            .pages
            .iter()
            .flat_map(|page| &page.lines)
            .filter(|line| line.element == 0)
            .collect::<Vec<_>>();
        assert!(fresh.pages.len() > 1);
        assert!(lines.iter().any(|line| line.oversized));
        assert!(lines.iter().all(|line| line.alignment == alignment));
        assert!(lines.iter().any(|line| line.display_text.contains("SS")));
        assert!(lines.iter().any(|line| line.display_text.contains("FFI")));
        for line in &lines {
            let mut display_offset = 0;
            let mut source_offset = line.source.start;
            for segment in line.display_source_map.iter() {
                assert_eq!(segment.display_start, display_offset);
                assert!(segment.display_end > segment.display_start);
                assert_eq!(segment.source_start, source_offset);
                assert!(segment.source_end > segment.source_start);
                display_offset = segment.display_end;
                source_offset = segment.source_end;
            }
            assert_eq!(display_offset, line.display_text.chars().count());
            assert_eq!(source_offset, line.source.end);
        }

        let mut cache = LayoutCache::default();
        let (cold, cold_hit) = valid(cache.get_or_build_with_composition(
            0,
            &aligned,
            &inputs,
            LayoutComposition::BodyEditor,
        ));
        assert!(!cold_hit);
        assert_eq!(cold.as_ref(), &fresh);
        let (complete_hit, complete_hit_flag) = valid(cache.get_or_build_with_composition(
            0,
            &aligned,
            &inputs,
            LayoutComposition::BodyEditor,
        ));
        assert!(complete_hit_flag);
        assert!(Arc::ptr_eq(&cold, &complete_hit));
        let (reused, reused_hit) = valid(cache.get_or_build_with_composition(
            1,
            &aligned,
            &inputs,
            LayoutComposition::BodyEditor,
        ));
        assert!(!reused_hit);
        assert_eq!(reused.as_ref(), &fresh);

        for line in lines.iter().filter(|line| !line.display_source_map.is_empty()) {
            let caret = Caret {
                element: line.element,
                offset: line.display_source_map[0].source_end,
            };
            assert_eq!(fresh.caret_geometry(caret.clone()), reused.caret_geometry(caret.clone()));
            let mut diagnostics = InteractionQueryDiagnostics::default();
            let located = valid(reused.line_for_caret_with_diagnostics(&caret, &mut diagnostics));
            assert_eq!(located.line, *line);
            assert_eq!(
                fresh.hit_test_fixed(line.page, line.geometry.x, line.geometry.y),
                reused.hit_test_fixed(line.page, line.geometry.x, line.geometry.y),
            );
        }

        let plain = valid(export_text(&aligned));
        assert!(plain.text.contains("plain é🙂"));
        assert!(plain.text.contains("ßﬃ"));
        let options = PdfExportOptions {
            include_title_page: false,
        };
        let pdf = valid(export_pdf_with_options(&aligned, options));
        assert_eq!(valid(export_pdf_with_options(&aligned, options)), pdf);
        assert_eq!(aligned, before);
    }
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
fn oversized_dual_spill_preserves_side_order_pages_and_fallback_output() {
    let left_dialogue = "l".repeat(300);
    let right_dialogue = "r".repeat(140);
    let mut project = project_with(&[
        (Role::Action, "before"),
        (Role::Character, "LEFT"),
        (Role::Dialogue, &left_dialogue),
        (Role::Character, "RIGHT"),
        (Role::Dialogue, &right_dialogue),
        (Role::Action, "after"),
    ]);
    add_group(&mut project, "spill", 1..3, 3..5);
    let inputs = LayoutInputs {
        paper_height: 40,
        margin_top: 8,
        margin_bottom: 8,
        lines_per_page: 2,
        ..LayoutInputs::default()
    };
    let layout = build(&project, LayoutComposition::BodyEditor, &inputs);
    assert_eq!(
        layout.pages[0]
            .lines
            .iter()
            .map(|line| line.element)
            .collect::<Vec<_>>(),
        [0]
    );

    let left_lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element == 1 || line.element == 2)
        .collect::<Vec<_>>();
    let right_lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element == 3 || line.element == 4)
        .collect::<Vec<_>>();
    assert!(left_lines.len() > inputs.lines_per_page as usize);
    assert!(right_lines.len() > inputs.lines_per_page as usize);
    assert!(left_lines.iter().chain(&right_lines).all(|line| {
        line.oversized && !line.keep_with_next && line.page > 0
    }));
    assert!(
        left_lines.iter().map(|line| line.page).max()
            > right_lines.iter().map(|line| line.page).max()
    );

    for (page_index, page) in layout.pages.iter().enumerate() {
        assert_eq!(page.number, page_index);
        let mut seen_right = false;
        for line in &page.lines {
            assert_eq!(line.page, page_index);
            if line.element == 3 || line.element == 4 {
                seen_right = true;
            } else if line.element == 1 || line.element == 2 {
                assert!(!seen_right, "left spill lines precede right spill lines");
            }
        }
    }
    assert_eq!(
        layout
            .pages
            .last()
            .and_then(|page| page.lines.last())
            .map(|line| line.element),
        Some(5)
    );
    assert_revision_reuse_matches_fresh(
        &project,
        LayoutComposition::BodyEditor,
        &inputs,
        &layout,
    );
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

#[test]
fn shared_owner_layout_and_exports_keep_title_body_and_owner_identity_exact() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body"><Text>Body text</Text></Paragraph></Content><TitlePage><Content><Paragraph id="title"><Text>Exact Title</Text></Paragraph></Content></TitlePage></FinalDraft>"#;
    let project = valid(read_v6(source)).project;
    let before = project.clone();
    let inputs = LayoutInputs::default();
    let body = build(&project, LayoutComposition::BodyEditor, &inputs);
    let title = build(&project, LayoutComposition::TitlePageEditor, &inputs);
    let print = build(&project, LayoutComposition::Print { include_title_page: true }, &inputs);
    assert_eq!(body, build(&project, LayoutComposition::BodyEditor, &inputs));
    assert_eq!(title, build(&project, LayoutComposition::TitlePageEditor, &inputs));
    assert_eq!(print, build(&project, LayoutComposition::Print { include_title_page: true }, &inputs));
    assert!(body.pages.iter().all(|page| !page.title_page));
    assert!(title.pages.iter().all(|page| page.title_page));
    assert!(print.pages.first().is_some_and(|page| page.title_page));

    let plain = valid(export_text(&project));
    assert!(plain.text.contains("Exact Title"));
    assert!(plain.text.contains("Body text"));
    let pdf = valid(export_pdf(&project));
    assert_eq!(valid(export_pdf(&project)), pdf);
    assert_eq!(project, before);
    for index in 0..before.elements.len() {
        assert!(Arc::ptr_eq(&before.elements[index], &project.elements[index]));
        assert_eq!(before.elements[index].revision(), project.elements[index].revision());
    }
}

#[test]
fn prepared_body_revision_reuse_isolates_title_print_and_export_composition() {
    let project = valid(read_v6(include_bytes!("fixtures/dual_dialogue_basic.fdx"))).project;
    let inputs = LayoutInputs::default();
    let fresh_body = build(&project, LayoutComposition::BodyEditor, &inputs);
    let fresh_title = build(&project, LayoutComposition::TitlePageEditor, &inputs);
    let print_composition = LayoutComposition::Print { include_title_page: true };
    let fresh_print = build(&project, print_composition, &inputs);
    let plain_before = valid(export_text(&project));
    let pdf_before = valid(export_pdf(&project));
    let mut cache = LayoutCache::default();

    let (body, body_hit) = valid(cache.get_or_build_with_composition(
        0,
        &project,
        &inputs,
        LayoutComposition::BodyEditor,
    ));
    assert!(!body_hit);
    assert_eq!(body.as_ref(), &fresh_body);
    let (title, title_hit) = valid(cache.get_or_build_with_composition(
        1,
        &project,
        &inputs,
        LayoutComposition::TitlePageEditor,
    ));
    assert!(!title_hit);
    assert_eq!(title.as_ref(), &fresh_title);
    assert!(title.pages.iter().all(|page| page.title_page));
    let (body_reused, body_reused_hit) = valid(cache.get_or_build_with_composition(
        2,
        &project,
        &inputs,
        LayoutComposition::BodyEditor,
    ));
    assert!(!body_reused_hit);
    assert_eq!(body_reused, body);
    let (print, print_hit) = valid(cache.get_or_build_with_composition(
        3,
        &project,
        &inputs,
        print_composition,
    ));
    assert!(!print_hit);
    assert_eq!(print.as_ref(), &fresh_print);
    assert!(print.pages.first().is_some_and(|page| page.title_page));
    assert_revision_reuse_matches_fresh(&project, print_composition, &inputs, &fresh_print);

    assert_eq!(valid(export_text(&project)), plain_before);
    assert_eq!(valid(export_pdf(&project)), pdf_before);
}

#[test]
fn incremental_preparation_matches_fresh_print_history_reopen_and_exports() {
    let source = format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" id=\"body\"><Text>{}</Text></Paragraph></Content></FinalDraft>",
        "source lorem é🙂 words ".repeat(1_024)
    );
    let project = valid(read_v6(source.as_bytes())).project;
    let mut session = valid(EditingSession::new(project));
    let presentation = BodyEditorPresentation::for_project(session.project(), None);
    let inputs = session.layout_inputs().clone();
    let mut body_cache = LayoutCache::default();
    let mut body_diagnostics = LayoutBuildDiagnostics::default();
    let (initial, _) = valid(body_cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut body_diagnostics,
    ));
    let mut print_cache = LayoutCache::default();
    let mut print_diagnostics = LayoutBuildDiagnostics::default();
    let (initial_print, _) = valid(print_cache.get_or_build_for_session_with_diagnostics(
        0,
        &session,
        LayoutPurpose::Print,
        &mut print_diagnostics,
    ));
    assert!(initial_print.pages.len() > 1);

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: session.project().elements[0].text.len(),
    })));
    valid(session.insert(" append é你🙂ßﬃ"));
    let (edited, _) = valid(body_cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut body_diagnostics,
    ));
    assert_eq!(body_diagnostics.prepared_incremental_hits, 1);
    assert_eq!(body_diagnostics.prepared_incremental_fallbacks, 0);
    let fresh = build(session.project(), LayoutComposition::BodyEditor, &inputs);
    assert_eq!(edited.as_ref(), &fresh);
    for line in fresh.pages.iter().flat_map(|page| &page.lines) {
        for offset in [line.source.start, line.source.end] {
            let caret = Caret {
                element: line.element,
                offset,
            };
            assert_eq!(fresh.caret_geometry(caret.clone()), edited.caret_geometry(caret.clone()));
            let mut queries = InteractionQueryDiagnostics::default();
            assert_eq!(
                valid(fresh.line_for_caret_with_diagnostics(&caret, &mut queries)).line,
                valid(edited.line_for_caret_with_diagnostics(&caret, &mut queries)).line
            );
        }
        assert_eq!(
            fresh.hit_test_fixed(line.page, line.geometry.x, line.geometry.y),
            edited.hit_test_fixed(line.page, line.geometry.x, line.geometry.y)
        );
    }

    valid(session.undo());
    let (undone, _) = valid(body_cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut body_diagnostics,
    ));
    assert_eq!(undone, initial);
    assert_eq!(body_diagnostics.prepared_incremental_hits, 0);
    valid(session.redo());
    let (redone, _) = valid(body_cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut body_diagnostics,
    ));
    assert_eq!(redone, edited);
    assert_eq!(body_diagnostics.prepared_incremental_hits, 0);

    let (printed, _) = valid(print_cache.get_or_build_for_session_with_diagnostics(
        session.revision(),
        &session,
        LayoutPurpose::Print,
        &mut print_diagnostics,
    ));
    assert_eq!(print_diagnostics.prepared_incremental_hits, 1);
    assert_eq!(print_diagnostics.prepared_incremental_fallbacks, 0);
    assert_eq!(
        printed.as_ref(),
        &build(session.project(), LayoutComposition::Print { include_title_page: true }, &inputs)
    );

    let plain = valid(export_text(session.project()));
    let pdf = valid(export_pdf(session.project()));
    let reopened = valid(read_v6(&valid(write_v6(session.project())))).project;
    assert_eq!(build(&reopened, LayoutComposition::BodyEditor, &inputs), fresh);
    assert_eq!(valid(export_text(&reopened)), plain);
    assert_eq!(valid(export_pdf(&reopened)), pdf);
}
