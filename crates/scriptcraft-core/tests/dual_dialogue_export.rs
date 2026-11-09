#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::dual_dialogue::{
    DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance,
};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{
    export_pdf_with_layout_inputs_and_options, export_text_with_layout_inputs, ExportError,
    PdfExportOptions, TextExport,
};
use scriptcraft_core::fdx::{import_fdx, read_v6, FdxImportOutcome, FdxMigrationConsent};
use scriptcraft_core::formatting::SourceLayoutSettings;
use scriptcraft_core::layout::{
    BodyEditorPresentation, LayoutCache, LayoutComposition, ScreenplayLayout,
};
use scriptcraft_core::{
    FixedPoint, LayoutInputs, Project, Role, RunAttribute, ScreenplayElement, StableId, TextRun,
};
use std::ops::Range;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("dual-dialogue export fixture is valid"))
}

fn project_with_roles(roles: &[Role], texts: &[&str]) -> Project {
    let mut project = valid(Project::new());
    project.elements = roles
        .iter()
        .zip(texts)
        .enumerate()
        .map(|(index, (role, text))| {
            valid(ScreenplayElement::new(
                format!("dual-export-{index}"),
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

fn add_group(project: &mut Project, id: &str, left: Range<usize>, right: Range<usize>) {
    let group = DualDialogueGroup {
        id: valid(StableId::new(id)),
        left: side(project, left),
        right: side(project, right),
        wrapper: DualDialogueWrapperProvenance::default(),
    };
    project.dual_dialogue_groups.push(group);
}

fn pdf_text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes)
        .unwrap_or_else(|_| unreachable!("PDF output substitutes unsupported non-ASCII"))
}

fn pdf_text_command(
    span: &scriptcraft_core::layout::PositionedStyledSpan,
    paper_height: u32,
) -> String {
    let y = FixedPoint::from_points(i64::from(paper_height))
        .raw()
        .saturating_sub(span.geometry.y.raw())
        .saturating_sub(span.geometry.height.raw());
    format!(
        "1 0 0 1 {} {} Tm",
        span.geometry.x.decimal_string(),
        FixedPoint::from_raw(y).decimal_string(),
    )
}

fn positioned_export_fixture() -> (Project, LayoutInputs) {
    let mut project = project_with_roles(
        &[
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Action,
        ],
        &[
            "preceding action",
            "LEFT CUE",
            "left (escaped) café line",
            "RIGHT CUE",
            "A long response that wraps across several lines with (styled) words and more text.",
            "after group",
        ],
    );
    project.elements[4] = valid(ScreenplayElement::with_runs(
        "dual-export-4",
        Role::Dialogue,
        vec![
            TextRun {
                text: "A long response that wraps across several lines with ".to_owned(),
                attributes: Vec::new(),
            },
            TextRun {
                text: "(styled) words and more text.".to_owned(),
                attributes: vec![
                    RunAttribute {
                        name: "Style".to_owned(),
                        value: "Bold+Underline+Strikethrough".to_owned(),
                    },
                    RunAttribute {
                        name: "Size".to_owned(),
                        value: "16".to_owned(),
                    },
                ],
            },
        ],
        None,
    ));
    add_group(&mut project, "wrapper-export-positioned", 1..3, 3..5);
    let inputs = LayoutInputs {
        lines_per_page: 4,
        ..LayoutInputs::default()
    };
    (project, inputs)
}

#[test]
fn pdf_uses_print_columns_positioned_spans_and_atomic_next_page_group_output() {
    let (project, inputs) = positioned_export_fixture();
    let source = SourceLayoutSettings::default();
    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &source,
        LayoutComposition::Print {
            include_title_page: false,
        },
    ));
    let pdf = valid(export_pdf_with_layout_inputs_and_options(
        &project,
        &inputs,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    let output = pdf_text(&pdf);
    let included_pdf = valid(export_pdf_with_layout_inputs_and_options(
        &project,
        &inputs,
        PdfExportOptions {
            include_title_page: true,
        },
    ));
    let included_output = pdf_text(&included_pdf);

    assert_eq!(layout.pages.len(), 2);
    assert_eq!(layout.pages[0].lines.len(), 1);
    assert!(layout.pages[0].lines.iter().all(|line| line.element == 0));
    let group_lines = &layout.pages[1].lines;
    assert!(group_lines.iter().all(|line| (1..5).contains(&line.element)));
    let left = group_lines
        .iter()
        .filter(|line| line.element < 3)
        .collect::<Vec<_>>();
    let right = group_lines
        .iter()
        .filter(|line| line.element >= 3)
        .collect::<Vec<_>>();
    assert!(left.len() < right.len());
    assert!(group_lines.iter().take(left.len()).all(|line| line.element < 3));
    assert!(group_lines.iter().skip(left.len()).all(|line| line.element >= 3));
    assert_eq!(left[0].paragraph_bounds.x, FixedPoint::from_points(108));
    assert_eq!(left[0].paragraph_bounds.width, FixedPoint::from_points(198));
    assert_eq!(right[0].paragraph_bounds.x, FixedPoint::from_points(342));
    assert_eq!(right[0].paragraph_bounds.width, FixedPoint::from_points(198));
    assert_eq!(left[0].paragraph_bounds.y, right[0].paragraph_bounds.y);
    assert!(left[0].geometry.x > left[0].paragraph_bounds.x);

    let span_commands = group_lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| pdf_text_command(span, inputs.paper_height))
        .collect::<Vec<_>>();
    assert!(span_commands.iter().all(|command| output.contains(command)));
    assert!(output.contains("BT /F2 16 Tf"));
    assert!(output.contains("\\(styled\\)"));
    assert!(output.contains("\\(escaped\\)"));
    assert!(output.contains("caf?"));
    assert!(output.matches("q 0.6 w").count() >= 2);
    assert!(output.contains("/BaseFont /Courier >>"));
    assert!(output.contains("/BaseFont /Courier-Bold >>"));
    assert!(!output.contains("/FontFile"));
    assert!(!output.contains(" d\n"));
    assert!(!output.contains("SCRIPT TITLE"));
    assert!(included_output.contains("SCRIPT TITLE"));
    assert!(included_output.contains("LEFT CUE"));
}

#[test]
fn print_layout_after_active_body_cache_is_equivalent_to_a_fresh_columnar_print_layout() {
    let (project, inputs) = positioned_export_fixture();
    let active = BodyEditorPresentation::for_project(
        &project,
        Some(project.dual_dialogue_groups[0].id.clone()),
    );
    let mut cache = LayoutCache::default();
    let _ = valid(cache.get_or_build_body_editor(1, &project, &inputs, &active));
    let (cached_print, cache_hit) = valid(cache.get_or_build_with_composition(
        1,
        &project,
        &inputs,
        LayoutComposition::Print { include_title_page: false },
    ));
    let fresh_print = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print { include_title_page: false },
    ));

    assert!(!cache_hit);
    assert_eq!(*cached_print, fresh_print);
    assert_eq!(cached_print.active_group_id, None);
    assert!(cached_print.editor_group_boundaries.is_empty());
    let cues = cached_print.pages.iter().flat_map(|page| &page.lines)
        .filter(|line| matches!(line.element, 1 | 3))
        .collect::<Vec<_>>();
    assert_eq!(cues.len(), 2);
    assert_ne!(cues[0].paragraph_bounds.x, cues[1].paragraph_bounds.x);
}

#[test]
fn plain_text_keeps_each_groups_complete_left_then_right_order_and_visible_loss_counts() {
    let mut project = project_with_roles(
        &[
            Role::Character,
            Role::Dialogue,
            Role::Parenthetical,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Character,
            Role::Dialogue,
        ],
        &[
            "unused cue placeholder",
            "LEFT ONE",
            "(left aside)",
            "LEFT TWO",
            "RIGHT ONE",
            "RIGHT RESPONSE",
            "between groups",
            "LEFT TWO CUE",
            "LEFT SECOND GROUP",
            "RIGHT TWO CUE",
            "RIGHT SECOND GROUP",
        ],
    );
    project.elements[0] = valid(ScreenplayElement::with_runs(
        "dual-export-0",
        Role::Character,
        vec![TextRun {
            text: "straße cue".to_owned(),
            attributes: vec![RunAttribute {
                name: "Style".to_owned(),
                value: "AllCaps".to_owned(),
            }],
        }],
        None,
    ));
    add_group(&mut project, "wrapper-export-order-one", 0..4, 4..6);
    add_group(&mut project, "wrapper-export-order-two", 7..9, 9..11);
    let original = project.clone();
    let inputs = LayoutInputs::default();
    let export = valid(export_text_with_layout_inputs(&project, &inputs));
    let body = export.text.as_str();
    let ordered_terms = [
        "STRASSE CUE",
        "LEFT ONE",
        "(left aside)",
        "LEFT TWO",
        "RIGHT ONE",
        "RIGHT RESPONSE",
        "between groups",
        "LEFT TWO CUE",
        "LEFT SECOND GROUP",
        "RIGHT TWO CUE",
        "RIGHT SECOND GROUP",
    ];
    let offsets = ordered_terms
        .iter()
        .map(|term| body.find(term).unwrap_or_else(|| unreachable!("plain output contains every visible group line")))
        .collect::<Vec<_>>();
    assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(export.loss.roles_lost, project.title_page.len() + project.elements.len());
    assert_eq!(export.loss.ids_lost, project.title_page.len() + project.elements.len());
    assert_eq!(export.loss.roles_lost, export.loss.ids_lost);
    assert_eq!(project, original);
}

#[test]
fn plain_text_omits_hidden_dialogue_from_imported_dual_group_without_counting_wrapper() {
    let source = include_str!("fixtures/dual_dialogue_basic.fdx");
    let source = source.replace(
        "</FinalDraft>",
        "<ElementSettings Type=\"Dialogue\"><FontSpec Style=\"HiddenText\" /></ElementSettings></FinalDraft>",
    );
    let imported = valid(import_fdx(source.as_bytes(), FdxMigrationConsent::Cancel));
    let FdxImportOutcome::Candidate(candidate) = imported else {
        unreachable!("Version 6 fixture imports as a candidate")
    };
    let project = candidate.document.project;
    let export = valid(export_text_with_layout_inputs(
        &project,
        &LayoutInputs::default(),
    ));

    assert!(!export.text.contains("First half"));
    assert!(!export.text.contains("你好, world."));
    assert!(export.text.contains("MARA"));
    assert_eq!(export.loss.roles_lost, project.title_page.len() + project.elements.len());
    assert_eq!(export.loss.ids_lost, project.title_page.len() + project.elements.len());
}

#[test]
fn export_api_types_exclude_desktop_presentation_and_outputs_are_repeatable_and_borrow_only() {
    let (project, inputs) = positioned_export_fixture();
    let project_before = project.clone();
    let inputs_before = inputs.clone();
    let _: fn(&Project, &LayoutInputs) -> Result<TextExport, ExportError> =
        export_text_with_layout_inputs;
    let _: fn(&Project, &LayoutInputs, PdfExportOptions) -> Result<Vec<u8>, ExportError> =
        export_pdf_with_layout_inputs_and_options;

    let text_first = valid(export_text_with_layout_inputs(&project, &inputs));
    let text_second = valid(export_text_with_layout_inputs(&project, &inputs));
    let pdf_first = valid(export_pdf_with_layout_inputs_and_options(
        &project,
        &inputs,
        PdfExportOptions {
            include_title_page: false,
        },
    ));
    let pdf_second = valid(export_pdf_with_layout_inputs_and_options(
        &project,
        &inputs,
        PdfExportOptions {
            include_title_page: false,
        },
    ));

    assert_eq!(text_first, text_second);
    assert_eq!(pdf_first, pdf_second);
    assert_eq!(project, project_before);
    assert_eq!(inputs, inputs_before);
}

#[test]
fn integrated_layout_export_fixture_matrix_preserves_mapping_columns_atomicity_and_isolation() {
    struct Fixture {
        name: &'static str,
        project: Project,
        groups: Vec<(Range<usize>, Range<usize>)>,
        inputs: LayoutInputs,
        oversized: bool,
        expected_group_page: Option<usize>,
    }

    let standard = LayoutInputs::default();
    let mut fixtures = Vec::new();

    fixtures.push(Fixture {
        name: "minimal four paragraphs",
        project: project_with_roles(
            &[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue],
            &["LEFT CUE", "left line", "RIGHT CUE", "right line"],
        ),
        groups: vec![(0..2, 2..4)],
        inputs: standard.clone(),
        oversized: false,
        expected_group_page: None,
    });

    fixtures.push(Fixture {
        name: "multiple paragraphs per side",
        project: project_with_roles(
            &[
                Role::Character,
                Role::Dialogue,
                Role::Parenthetical,
                Role::Character,
                Role::Dialogue,
                Role::Parenthetical,
            ],
            &["LEFT MULTI", "left answer", "(left beat)", "RIGHT MULTI", "right answer", "(right beat)"],
        ),
        groups: vec![(0..3, 3..6)],
        inputs: standard.clone(),
        oversized: false,
        expected_group_page: None,
    });

    fixtures.push(Fixture {
        name: "unequal side heights",
        project: project_with_roles(
            &[
                Role::Character,
                Role::Dialogue,
                Role::Character,
                Role::Dialogue,
                Role::Parenthetical,
            ],
            &["LEFT UNEQUAL", "short", "RIGHT UNEQUAL", "long enough to wrap across lines in a screenplay column", "(extra beat)"],
        ),
        groups: vec![(0..2, 2..5)],
        inputs: standard.clone(),
        oversized: false,
        expected_group_page: None,
    });

    let mut unicode = project_with_roles(
        &[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue],
        &["MARA", "plain", "NOAH", "你好, café — déjà vu"],
    );
    unicode.elements[1] = valid(ScreenplayElement::with_runs(
        "dual-export-1",
        Role::Dialogue,
        vec![
            TextRun { text: "formatted ".to_owned(), attributes: vec![RunAttribute { name: "Style".to_owned(), value: "Bold".to_owned() }] },
            TextRun { text: "Unicode: café ✨".to_owned(), attributes: vec![RunAttribute { name: "Style".to_owned(), value: "Italic".to_owned() }] },
        ],
        None,
    ));
    fixtures.push(Fixture {
        name: "formatted Unicode",
        project: unicode,
        groups: vec![(0..2, 2..4)],
        inputs: standard.clone(),
        oversized: false,
        expected_group_page: None,
    });

    fixtures.push(Fixture {
        name: "two groups",
        project: project_with_roles(
            &[
                Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
                Role::Action, Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
            ],
            &["LEFT ONE", "left one", "RIGHT ONE", "right one", "between", "LEFT TWO", "left two", "RIGHT TWO", "right two"],
        ),
        groups: vec![(0..2, 2..4), (5..7, 7..9)],
        inputs: standard.clone(),
        oversized: false,
        expected_group_page: None,
    });

    fixtures.push(Fixture {
        name: "exact page boundary",
        project: project_with_roles(
            &[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue],
            &["LEFT EXACT", "left exact", "RIGHT EXACT", "right exact"],
        ),
        groups: vec![(0..2, 2..4)],
        inputs: LayoutInputs {
            paper_height: 40,
            margin_top: 8,
            margin_bottom: 8,
            lines_per_page: 2,
            ..standard.clone()
        },
        oversized: false,
        expected_group_page: Some(0),
    });

    fixtures.push(Fixture {
        name: "normal block moves to next page",
        project: project_with_roles(
            &[
                Role::Action, Role::Action,
                Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
            ],
            &["before one", "before two", "LEFT NEXT", "left next", "RIGHT NEXT", "right next"],
        ),
        groups: vec![(2..4, 4..6)],
        inputs: LayoutInputs {
            paper_height: 160,
            margin_top: 8,
            margin_bottom: 8,
            lines_per_page: 5,
            ..standard.clone()
        },
        oversized: false,
        expected_group_page: Some(1),
    });

    fixtures.push(Fixture {
        name: "oversized fallback",
        project: project_with_roles(
            &[
                Role::Character, Role::Dialogue, Role::Dialogue, Role::Dialogue,
                Role::Dialogue, Role::Dialogue, Role::Character, Role::Dialogue, Role::Action,
            ],
            &["LEFT LARGE", "l1", "l2", "l3", "l4", "l5", "RIGHT LARGE", "right", "after"],
        ),
        groups: vec![(0..6, 6..8)],
        inputs: LayoutInputs {
            paper_height: 40,
            margin_top: 8,
            margin_bottom: 8,
            lines_per_page: 2,
            ..standard
        },
        oversized: true,
        expected_group_page: None,
    });

    for fixture in fixtures {
        let _case_name = fixture.name;
        let mut project = fixture.project;
        for (index, (left, right)) in fixture.groups.iter().enumerate() {
            add_group(&mut project, &format!("matrix-{}-{index}", fixture.name.replace(' ', "-")), left.clone(), right.clone());
        }
        valid(project.validate());
        let before_export = (
            valid(export_pdf_with_layout_inputs_and_options(
                &project,
                &fixture.inputs,
                PdfExportOptions { include_title_page: false },
            )),
            valid(export_text_with_layout_inputs(&project, &fixture.inputs)),
        );
        let print = valid(ScreenplayLayout::build_with_composition(
            &project,
            &fixture.inputs,
            &SourceLayoutSettings::default(),
            LayoutComposition::Print { include_title_page: false },
        ));
        assert_eq!(print.active_group_id, None);
        assert!(print.editor_group_boundaries.is_empty());
        for page in &print.pages {
            for line in &page.lines {
                assert_eq!(line.source.element, line.element, "{} source map", fixture.name);
            }
        }

        for (group_index, group) in project.dual_dialogue_groups.iter().enumerate() {
            let left_range = &fixture.groups[group_index].0;
            let right_range = &fixture.groups[group_index].1;
            let left_id = &group.left.character;
            let right_id = &group.right.character;
            assert_eq!(&project.elements[left_range.start].id, left_id);
            assert_eq!(&project.elements[right_range.start].id, right_id);
            assert_eq!(group.member_ids().cloned().collect::<Vec<_>>(), project.elements[left_range.start..right_range.end].iter().map(|item| item.id.clone()).collect::<Vec<_>>());

            let lines = print.pages.iter().flat_map(|page| &page.lines).collect::<Vec<_>>();
            let left = lines.iter().find(|line| line.element == left_range.start).copied();
            let right = lines.iter().find(|line| line.element == right_range.start).copied();
            let (Some(left), Some(right)) = (left, right) else {
                unreachable!("{} has both printed columns", fixture.name)
            };
            assert_eq!(left.paragraph_bounds.x, FixedPoint::from_points(108), "{} left column", fixture.name);
            assert_eq!(right.paragraph_bounds.x, FixedPoint::from_points(342), "{} right column", fixture.name);
            assert_eq!(left.paragraph_bounds.y, right.paragraph_bounds.y, "{} shared column top", fixture.name);

            let group_lines = lines.iter().filter(|line| {
                group.member_ids().any(|id| project.elements.get(line.element).is_some_and(|element| &element.id == id))
            }).collect::<Vec<_>>();
            assert!(!group_lines.is_empty(), "{} group lines", fixture.name);
            if fixture.oversized {
                assert!(group_lines.iter().all(|line| line.oversized), "{} explicit fallback", fixture.name);
            } else {
                assert!(group_lines.iter().all(|line| line.page == group_lines[0].page), "{} atomic group", fixture.name);
            }
            if let Some(expected_page) = fixture.expected_group_page {
                assert!(group_lines.iter().all(|line| line.page == expected_page), "{} page placement", fixture.name);
            }
        }

        let active = BodyEditorPresentation::for_project(
            &project,
            project.dual_dialogue_groups.first().map(|group| group.id.clone()),
        );
        let mut editor_cache = LayoutCache::default();
        let _active_editor_layout = valid(editor_cache.get_or_build_body_editor(
            1,
            &project,
            &fixture.inputs,
            &active,
        ));
        let after_export = (
            valid(export_pdf_with_layout_inputs_and_options(
                &project,
                &fixture.inputs,
                PdfExportOptions { include_title_page: false },
            )),
            valid(export_text_with_layout_inputs(&project, &fixture.inputs)),
        );
        assert_eq!(after_export, before_export, "{} active-editor export isolation", fixture.name);
        let text = after_export.1.text.as_str();
        let ordered_indices = project.elements.iter().map(|element| {
            text.find(&element.text).unwrap_or_else(|| unreachable!("{} plain text contains each fixture line", fixture.name))
        }).collect::<Vec<_>>();
        assert!(ordered_indices.windows(2).all(|pair| pair[0] < pair[1]), "{} left-before-right text order", fixture.name);
        assert!(!pdf_text(&after_export.0).contains(" d\n"), "{} export has no editor boundary stroke", fixture.name);
    }
}

#[test]
fn empty_sides_and_inline_alts_never_leak_into_print_pdf_or_plain_text_including_after_deletion() {
    let imported = valid(read_v6(include_bytes!("fixtures/dialogue_alts.fdx")));
    let project = imported.project;
    let inputs = LayoutInputs::default();
    let print = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print { include_title_page: false },
    ));
    let active = BodyEditorPresentation::for_project(
        &project,
        project.dual_dialogue_groups.first().map(|group| group.id.clone()),
    );
    let mut cache = LayoutCache::default();
    let (active_body, _) = valid(cache.get_or_build_body_editor(1, &project, &inputs, &active));
    assert!(active_body.active_group_id.is_some());
    let (cached_print, cache_hit) = valid(cache.get_or_build_with_composition(
        1,
        &project,
        &inputs,
        LayoutComposition::Print { include_title_page: false },
    ));
    assert!(!cache_hit);
    assert_eq!(*cached_print, print, "Print remains fixed-column after active Body layout");
    assert_eq!(cached_print.active_group_id, None);
    assert!(cached_print.editor_group_boundaries.is_empty());

    let pdf = valid(export_pdf_with_layout_inputs_and_options(
        &project,
        &inputs,
        PdfExportOptions { include_title_page: false },
    ));
    let text = valid(export_text_with_layout_inputs(&project, &inputs)).text;
    for sentinel in [
        "INLINE_ALT_SENTINEL",
        "MEMBER_ALT_SENTINEL",
        "COLLECTION_MEMBER_SENTINEL",
        "COLLECTION_ORDINARY_SENTINEL",
        "COLLECTION_REPEAT_SENTINEL",
        "ORPHAN_SENTINEL",
    ] {
        assert!(!text.contains(sentinel), "plain text excludes {sentinel}");
        assert!(!pdf_text(&pdf).contains(sentinel), "PDF excludes {sentinel}");
    }
    assert!(text.find("Left").is_some_and(|left| text.find("Right").is_some_and(|right| left < right)));
    assert!(!pdf_text(&pdf).contains(" d\n"), "Print/PDF has no editor boundary");
    assert_eq!(
        print.pages.iter().flat_map(|page| &page.lines).filter(|line| matches!(line.element, 1 | 2 | 3)).map(|line| line.element).collect::<Vec<_>>(),
        [1, 2, 3],
        "C,C,D emits only its real cues and Dialogue member",
    );

    let mut session = valid(EditingSession::new(project));
    assert!(session.set_selection(Selection {
        anchor: Position { element: 2, offset: session.project().elements[2].text.len() },
        focus: Position { element: 3, offset: session.project().elements[3].text.len() },
    }).is_ok());
    assert!(session.delete().is_ok(), "the combined owner deletion leaves C,C");
    let deleted = session.project();
    assert_eq!(deleted.elements.len(), 3);
    assert!(deleted.dual_dialogue_groups.first().is_some_and(|group| group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()));
    let deleted_text = valid(export_text_with_layout_inputs(deleted, &inputs)).text;
    let deleted_pdf = valid(export_pdf_with_layout_inputs_and_options(
        deleted,
        &inputs,
        PdfExportOptions { include_title_page: false },
    ));
    for sentinel in ["INLINE_ALT_SENTINEL", "MEMBER_ALT_SENTINEL", "COLLECTION_MEMBER_SENTINEL", "COLLECTION_ORDINARY_SENTINEL", "COLLECTION_REPEAT_SENTINEL", "ORPHAN_SENTINEL"] {
        assert!(!deleted_text.contains(sentinel), "post-deletion plain text excludes {sentinel}");
        assert!(!pdf_text(&deleted_pdf).contains(sentinel), "post-deletion PDF excludes {sentinel}");
    }
    let cue_lines = valid(ScreenplayLayout::build_with_composition(
        deleted,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print { include_title_page: false },
    )).pages.into_iter().flat_map(|page| page.lines).filter(|line| matches!(line.element, 1 | 2)).collect::<Vec<_>>();
    assert_eq!(cue_lines.len(), 2, "deletion exports exactly the two actual cue lines");
}
