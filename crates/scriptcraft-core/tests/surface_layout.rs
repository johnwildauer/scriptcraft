#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::formatted_text::set_paragraph_alignments;
use scriptcraft_core::layout::{LayoutCache, LayoutComposition, ScreenplayLayout};
use scriptcraft_core::{
    FontFallback, LayoutInputs, ParagraphAlignment, Project, Role, RunAttribute,
    ScreenplayElement, TextRun,
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

#[test]
fn compositions_keep_body_title_and_print_page_domains_separate_and_deterministic() {
    let project = fixture();
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
    assert!(title.title_line_index_for_position(title_line.element, title_line.source.start).is_some());
    assert!(title.title_page_for_position(title_line.element, title_line.source.start).is_some());
    assert!(body.title_line_index_for_position(title_line.element, title_line.source.start).is_none());
    assert!(print.hit_test_fixed(0, title_line.geometry.x, title_line.geometry.y).is_none());
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
    assert_eq!(empty_line.geometry.width.raw(), 0);
    assert_eq!(empty_line.geometry.x, empty_line.paragraph_bounds.x);
}
