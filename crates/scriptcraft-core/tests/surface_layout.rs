#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::formatted_text::set_paragraph_alignments;
use scriptcraft_core::layout::{
    Caret, InteractionQueryDiagnostics, LayoutCache, LayoutComposition, ScreenplayLayout,
};
use scriptcraft_core::{
    FixedPoint, FontFallback, LayoutInputs, ParagraphAlignment, Project, Role, RunAttribute,
    ScreenplayElement, SourceLayoutSettings, SourceParagraphOverrides, TextRun,
};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("layout fixture uses valid bounded input"))
}

fn fixture() -> Project {
    let mut project = valid(Project::new());
    project.elements.clear();
    project.elements.push(valid(ScreenplayElement::new(
        "layout-body",
        Role::Action,
        "one two three four five six seven eight nine ten",
    )));
    project
}

fn build(project: &Project, composition: LayoutComposition) -> ScreenplayLayout {
    let inputs = LayoutInputs::default();
    let source = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_default();
    valid(ScreenplayLayout::build_with_composition(
        project,
        &inputs,
        &source,
        composition,
    ))
}

fn assert_default_action_wrap(layout: &ScreenplayLayout, canonical: &str, first_line_text: &str) {
    assert_eq!(layout.inputs.chars_per_line, 55);
    let lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element == 0)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);

    let first = lines[0];
    assert_eq!(first.text.as_ref(), first_line_text);
    assert_eq!(first.display_text.as_ref(), first_line_text);
    assert_eq!(first.source.start, 0);
    assert_eq!(first.source.end, first_line_text.len());
    assert_eq!(first.source_for_display_char(60), 60);
    assert_eq!(first.source_for_display_char(61), first_line_text.len());
    assert_eq!(first.display_source_map.last().map(|segment| segment.source_start), Some(60));
    assert_eq!(first.display_source_map.last().map(|segment| segment.source_end), Some(62));

    let second = lines[1];
    assert_eq!(second.text.as_ref(), "Z");
    assert_eq!(second.source.start, first_line_text.len());
    assert_eq!(second.source.end, canonical.len());

    for line in lines {
        assert_eq!(line.paragraph_bounds.x, FixedPoint::from_points(108));
        assert_eq!(line.paragraph_bounds.width, FixedPoint::from_points(432));
        assert_eq!(
            line.paragraph_bounds.x.raw() + line.paragraph_bounds.width.raw(),
            FixedPoint::from_points(540).raw()
        );
    }
}

#[test]
fn default_action_wraps_to_effective_bounds_and_preserves_utf8_source_mapping_in_editor_and_print() {
    let first_line_text = format!("{}é", "a".repeat(60));
    let canonical = format!("{first_line_text}Z");
    let mut project = fixture();
    project.elements[0].text = canonical.clone();

    let editor = build(&project, LayoutComposition::BodyEditor);
    let print = build(
        &project,
        LayoutComposition::Print {
            include_title_page: false,
        },
    );

    assert_default_action_wrap(&editor, &canonical, &first_line_text);
    assert_default_action_wrap(&print, &canonical, &first_line_text);
}

#[test]
fn compositions_keep_body_title_and_print_page_domains_separate_and_deterministic() {
    let mut project = fixture();
    project.title_page[0].formatting.alignment =
        Some(scriptcraft_core::title_page::TitlePageAlignment::Right);
    project.title_page[0].formatting.first_indent = Some(FixedPoint::from_points(12));
    let body = build(&project, LayoutComposition::BodyEditor);
    let title = build(&project, LayoutComposition::TitlePageEditor);
    let print = build(&project, LayoutComposition::Print {
        include_title_page: true,
    });
    let body_print = build(&project, LayoutComposition::Print {
        include_title_page: false,
    });

    assert!(body.pages.iter().all(|page| !page.title_page));
    assert!(title.pages.iter().all(|page| page.title_page));
    assert!(title.pages.iter().flat_map(|page| &page.lines).all(|line| line.element < project.title_page.len()));
    assert!(print.pages.first().is_some_and(|page| page.title_page));
    assert!(print.pages.iter().skip(1).all(|page| !page.title_page));
    assert!(body_print.pages.iter().all(|page| !page.title_page));
    assert_eq!(body.pages.len(), body_print.pages.len());
    let title_line = title.pages[0].lines.first().unwrap_or_else(|| unreachable!("title template has lines"));
    assert_eq!(title_line.alignment, ParagraphAlignment::Right);
    assert_eq!(
        title_line.unaligned_line_left,
        FixedPoint::from_raw(title_line.paragraph_bounds.x.raw() + FixedPoint::from_points(12).raw())
    );
    assert!(title.title_line_index_for_position(title_line.element, title_line.source.start).is_some());
    assert!(title.title_page_for_position(title_line.element, title_line.source.start).is_some());
    assert!(body.title_line_index_for_position(title_line.element, title_line.source.start).is_none());
    assert!(print.hit_test_fixed(0, title_line.geometry.x, title_line.geometry.y).is_none());
    let mut title_diagnostics = InteractionQueryDiagnostics::default();
    let located_title = title
        .nearest_line_location_fixed_with_diagnostics(
            0,
            title_line.geometry.x,
            title_line.geometry.y,
            &mut title_diagnostics,
        )
        .unwrap_or_else(|| unreachable!("title editor line is locatable"));
    assert_eq!(located_title.page_index, 0);
    assert_eq!(located_title.line, title_line);
    let mut print_diagnostics = InteractionQueryDiagnostics::default();
    assert!(print
        .nearest_line_location_fixed_with_diagnostics(
            0,
            title_line.geometry.x,
            title_line.geometry.y,
            &mut print_diagnostics,
        )
        .is_none());
    assert_eq!(print_diagnostics.index_queries, 1);
}

#[test]
fn mixed_runs_produce_positioned_spans_and_unicode_all_caps_maps_back_to_canonical_bytes() {
    let mut project = fixture();
    project.elements[0] = valid(ScreenplayElement::with_runs(
        "layout-body",
        Role::Action,
        vec![
            TextRun {
                text: "straße".to_owned(),
                attributes: vec![RunAttribute {
                    name: "Style".to_owned(),
                    value: "AllCaps".to_owned(),
                }],
            },
            TextRun {
                text: " Bold".to_owned(),
                attributes: vec![RunAttribute {
                    name: "Style".to_owned(),
                    value: "Bold+Underline+Strikethrough".to_owned(),
                }, RunAttribute { name: "Size".to_owned(), value: "24".to_owned() },
                    RunAttribute { name: "Font".to_owned(), value: "Unknown Typeface".to_owned() },
                    RunAttribute { name: "VendorStyle".to_owned(), value: "preserved".to_owned() }],
            },
        ],
        None,
    ));
    let layout = build(&project, LayoutComposition::BodyEditor);
    let line = layout.pages.iter().flat_map(|page| &page.lines).next();
    let line = line.unwrap_or_else(|| unreachable!("nonempty body has layout lines"));

    assert_eq!(line.display_text.as_ref(), "STRASSE Bold");
    assert!(line.spans.len() >= 2);
    assert!(line.spans[0].text_style.all_caps);
    assert!(line.spans[1].text_style.bold);
    assert!(line.spans[1].underline);
    assert!(line.spans[1].strikethrough);
    assert_eq!(line.spans[1].text_style.size, 24);
    assert_eq!(line.spans[1].text_style.family, FontFallback::CourierPrime);
    assert_eq!(line.source_for_display_char(6), "straß".len());
    assert_eq!(line.source_for_display_char(7), "straße".len());
    let mut location_diagnostics = InteractionQueryDiagnostics::default();
    let located = layout
        .nearest_line_location_fixed_with_diagnostics(
            line.page,
            line.geometry.x,
            line.geometry.y,
            &mut location_diagnostics,
        )
        .unwrap_or_else(|| unreachable!("mixed-style line is locatable"));
    assert_eq!(located.line, line);
    assert_eq!(located.fixed_caret_for_x(line.geometry.x).offset, line.source.start);
    let after_line = FixedPoint::from_raw(line.geometry.x.raw() + line.geometry.width.raw());
    assert_eq!(located.fixed_caret_for_x(after_line).offset, line.source.end);
    for offset in [0, "straße".len(), project.elements[0].text.len()] {
        let caret = Caret { element: 0, offset };
        let mut owner_diagnostics = InteractionQueryDiagnostics::default();
        let owner = layout
            .line_for_caret_with_diagnostics(&caret, &mut owner_diagnostics)
            .unwrap_or_else(|_| unreachable!("fixture offsets are canonical source boundaries"));
        let mut geometry_diagnostics = InteractionQueryDiagnostics::default();
        let fixed_geometry = layout.caret_geometry_with_diagnostics(
            caret.clone(),
            &mut geometry_diagnostics,
        );
        assert_eq!(owner.fixed_caret_geometry(&caret), fixed_geometry);
        assert_eq!(owner_diagnostics, geometry_diagnostics);
    }
}

#[test]
fn justified_wrapped_lines_fill_real_inter_word_geometry_but_leave_final_lines_left() {
    let mut project = fixture();
    project.elements[0].text = (0..80)
        .map(|index| format!("w{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    set_paragraph_alignments(
        &mut project,
        &[scriptcraft_core::ParagraphTarget::Body(0)],
        ParagraphAlignment::Justified,
    )
    .unwrap_or_else(|_| unreachable!("valid paragraph can be aligned"));
    let layout = build(&project, LayoutComposition::BodyEditor);
    let lines = layout.pages.iter().flat_map(|page| &page.lines).collect::<Vec<_>>();
    assert!(lines.len() > 1);
    let wrapped = &lines[0];
    assert!(wrapped.geometry.width.raw()
        > (wrapped.display_text.chars().count() as i64) * wrapped.char_width.raw());
    let final_line = lines.last().unwrap_or_else(|| unreachable!("wrapped output has a final line"));
    assert_eq!(final_line.geometry.x, final_line.paragraph_bounds.x);
    assert_eq!(final_line.alignment, ParagraphAlignment::Justified);
    assert_eq!(final_line.unaligned_line_left, final_line.paragraph_bounds.x);
}

#[test]
fn layout_cache_keys_composition_and_reuses_only_matching_composition() {
    let project = fixture();
    let inputs = LayoutInputs::default();
    let mut cache = LayoutCache::default();
    let (body, body_hit) = valid(cache.get_or_build_with_composition(
        1,
        &project,
        &inputs,
        LayoutComposition::BodyEditor,
    ));
    assert!(!body_hit);
    assert!(body.pages.iter().all(|page| !page.title_page));
    let (title, title_hit) = valid(cache.get_or_build_with_composition(
        1,
        &project,
        &inputs,
        LayoutComposition::TitlePageEditor,
    ));
    assert!(!title_hit);
    assert!(title.pages.iter().all(|page| page.title_page));
    let (_, repeated_title_hit) = valid(cache.get_or_build_with_composition(
        1,
        &project,
        &inputs,
        LayoutComposition::TitlePageEditor,
    ));
    assert!(repeated_title_hit);
}

#[test]
fn left_center_right_and_no_gap_justified_lines_keep_their_defined_geometry() {
    let alignments = [
        ParagraphAlignment::Left,
        ParagraphAlignment::Center,
        ParagraphAlignment::Right,
        ParagraphAlignment::Justified,
    ];
    let mut geometries = Vec::new();
    for (index, alignment) in alignments.into_iter().enumerate() {
        let mut project = fixture();
        project.elements[0].text = "single".to_owned();
        set_paragraph_alignments(
            &mut project,
            &[scriptcraft_core::ParagraphTarget::Body(0)],
            alignment,
        )
        .unwrap_or_else(|_| unreachable!("valid paragraph can be aligned"));
        let layout = build(&project, LayoutComposition::BodyEditor);
        let line = &layout.pages[0].lines[0];
        assert_eq!(line.alignment, alignment);
        assert_eq!(line.unaligned_line_left, line.paragraph_bounds.x);
        geometries.push(line.geometry.x);
        if index == 0 || index == 3 {
            assert_eq!(line.geometry.x, line.paragraph_bounds.x);
        }
    }
    assert!(geometries[1] > geometries[0]);
    assert!(geometries[2] > geometries[1]);

    let mut empty = fixture();
    empty.elements[0].text.clear();
    set_paragraph_alignments(
        &mut empty,
        &[scriptcraft_core::ParagraphTarget::Body(0)],
        ParagraphAlignment::Justified,
    )
    .unwrap_or_else(|_| unreachable!("valid empty paragraph can be aligned"));
    let empty_layout = build(&empty, LayoutComposition::BodyEditor);
    let empty_line = &empty_layout.pages[0].lines[0];
    assert_eq!(empty_line.alignment, ParagraphAlignment::Justified);
    assert_eq!(empty_line.unaligned_line_left, empty_line.paragraph_bounds.x);
    assert_eq!(empty_line.geometry.width.raw(), 0);
    assert_eq!(empty_line.geometry.x, empty_line.paragraph_bounds.x);
    let empty_caret = Caret { element: 0, offset: 0 };
    let mut empty_diagnostics = InteractionQueryDiagnostics::default();
    let empty_location = empty_layout
        .line_for_caret_with_diagnostics(&empty_caret, &mut empty_diagnostics)
        .unwrap_or_else(|_| unreachable!("empty paragraph has its zero-offset caret"));
    assert_eq!(empty_location.line, empty_line);
    assert_eq!(
        empty_location.fixed_caret_geometry(&empty_caret),
        empty_layout.caret_geometry(empty_caret)
    );
}

#[test]
fn presentation_metadata_records_effective_alignment_and_indent_anchor() {
    let mut project = fixture();
    project.elements[0].text = (0..80)
        .map(|index| format!("word{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut source = SourceLayoutSettings::default();
    let mut direct = SourceParagraphOverrides::default();
    direct.paragraph.alignment = Some(ParagraphAlignment::Justified);
    direct.paragraph.first_indent = Some(FixedPoint::from_points(20));
    source
        .direct_paragraph_overrides
        .insert(project.elements[0].id.clone(), direct);
    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &LayoutInputs::default(),
        &source,
        LayoutComposition::BodyEditor,
    ));
    let lines = layout.pages.iter().flat_map(|page| &page.lines).collect::<Vec<_>>();
    assert!(lines.len() > 1);
    assert!(lines
        .iter()
        .all(|line| line.alignment == ParagraphAlignment::Justified));
    assert_eq!(
        lines[0].unaligned_line_left,
        FixedPoint::from_raw(lines[0].paragraph_bounds.x.raw() + FixedPoint::from_points(20).raw())
    );
    assert_eq!(lines[1].unaligned_line_left, lines[1].paragraph_bounds.x);

    for alignment in [
        ParagraphAlignment::Left,
        ParagraphAlignment::Center,
        ParagraphAlignment::Right,
        ParagraphAlignment::Justified,
    ] {
        let mut aligned = fixture();
        set_paragraph_alignments(
            &mut aligned,
            &[scriptcraft_core::ParagraphTarget::Body(0)],
            alignment,
        )
        .unwrap_or_else(|_| unreachable!("valid paragraph can be aligned"));
        let aligned_layout = build(&aligned, LayoutComposition::BodyEditor);
        assert!(aligned_layout.pages.iter().flat_map(|page| &page.lines).all(|line| {
            line.alignment == alignment
                && line.unaligned_line_left == line.paragraph_bounds.x
        }));
    }
}

#[test]
fn located_line_fixed_helpers_preserve_nearest_and_caret_geometry_diagnostics() {
    let project = fixture();
    let layout = build(&project, LayoutComposition::BodyEditor);
    let expected_line = &layout.pages[0].lines[0];
    let pointer_x = expected_line.geometry.x;
    let pointer_y = expected_line.geometry.y;
    let mut location_diagnostics = InteractionQueryDiagnostics::default();
    let located = layout
        .nearest_line_location_fixed_with_diagnostics(
            expected_line.page,
            pointer_x,
            pointer_y,
            &mut location_diagnostics,
        )
        .unwrap_or_else(|| unreachable!("body line is locatable"));
    assert_eq!(located.page_index, expected_line.page);
    assert_eq!(located.line, expected_line);
    assert_eq!(located.fixed_caret_for_x(pointer_x), layout.nearest_line_hit_test_fixed(
        expected_line.page,
        pointer_x,
        pointer_y,
    ).unwrap_or_else(|| unreachable!("body line is hit-testable")));
    assert_eq!(
        located.fixed_caret_for_x(FixedPoint::from_points(0)).offset,
        expected_line.source.start
    );
    assert_eq!(
        located.fixed_caret_for_x(FixedPoint::from_points(10_000)).offset,
        expected_line.source.end
    );

    let mut hit_diagnostics = InteractionQueryDiagnostics::default();
    let _ = layout.nearest_line_hit_test_fixed_with_diagnostics(
        expected_line.page,
        pointer_x,
        pointer_y,
        &mut hit_diagnostics,
    );
    assert_eq!(location_diagnostics, hit_diagnostics);

    let caret = Caret {
        element: expected_line.element,
        offset: expected_line.source.start,
    };
    let mut owner_diagnostics = InteractionQueryDiagnostics::default();
    let owner = layout
        .line_for_caret_with_diagnostics(&caret, &mut owner_diagnostics)
        .unwrap_or_else(|_| unreachable!("line start is owned by a core line"));
    assert_eq!(owner.page_index, expected_line.page);
    assert_eq!(owner.line, expected_line);
    let mut geometry_diagnostics = InteractionQueryDiagnostics::default();
    let geometry = layout.caret_geometry_with_diagnostics(
        caret.clone(),
        &mut geometry_diagnostics,
    );
    assert_eq!(
        owner.fixed_caret_geometry(&caret),
        geometry
    );
    assert_eq!(owner_diagnostics, geometry_diagnostics);
    let mut invalid_diagnostics = InteractionQueryDiagnostics::default();
    let invalid = Caret {
        element: expected_line.element,
        offset: project.elements[0].text.len().saturating_add(1),
    };
    assert!(layout
        .line_for_caret_with_diagnostics(&invalid, &mut invalid_diagnostics)
        .is_err());
    assert_eq!(invalid_diagnostics.index_queries, 1);
    let mut invalid_page_diagnostics = InteractionQueryDiagnostics::default();
    assert!(layout
        .nearest_line_location_fixed_with_diagnostics(
            layout.pages.len(),
            pointer_x,
            pointer_y,
            &mut invalid_page_diagnostics,
        )
        .is_none());
    assert_eq!(invalid_page_diagnostics.index_queries, 1);
}

#[test]
fn nearest_line_location_uses_absolute_y_across_pages() {
    let mut project = fixture();
    project.elements[0].text = (0..200)
        .map(|index| format!("word{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut inputs = LayoutInputs::default();
    inputs.paper_height = 240;
    inputs.margin_top = 12;
    inputs.margin_bottom = 12;
    inputs.lines_per_page = 18;
    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));
    assert!(layout.pages.len() > 1);
    let target = &layout.pages[1].lines[0];
    let mut location_diagnostics = InteractionQueryDiagnostics::default();
    let located = layout
        .nearest_line_location_fixed_with_diagnostics(
            1,
            target.geometry.x,
            target.geometry.y,
            &mut location_diagnostics,
        )
        .unwrap_or_else(|| unreachable!("second-page line is nearest at its own coordinates"));
    assert_eq!(located.page_index, 1);
    assert_eq!(located.line, target);
    let mut hit_diagnostics = InteractionQueryDiagnostics::default();
    let hit = layout
        .nearest_line_hit_test_fixed_with_diagnostics(
            1,
            target.geometry.x,
            target.geometry.y,
            &mut hit_diagnostics,
        )
        .unwrap_or_else(|| unreachable!("second-page line remains fixed-hit-testable"));
    assert_eq!(hit, located.fixed_caret_for_x(target.geometry.x));
    assert_eq!(location_diagnostics, hit_diagnostics);
}
