#![forbid(unsafe_code)]

use scriptcraft_core::dual_dialogue::{DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance};
use scriptcraft_core::exports::{export_pdf, export_pdf_with_options, export_text, PdfExportOptions};
use scriptcraft_core::fdx::read_v6;
use scriptcraft_core::formatting::{
    ParagraphAlignment, SourceLayoutSettings, SourceParagraphOverrides, SourceParagraphSpec,
};
use scriptcraft_core::layout::{
    BodyEditorPresentation, Caret, LayoutBuildDiagnostics, LayoutCache, LayoutComposition,
    LayoutPurpose, ScreenplayLayout,
};
use scriptcraft_core::{
    editing::{EditingSession, Position, Selection}, FixedPoint, LayoutInputs, Project, Role,
    ScreenplayElement, StableId,
};
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

fn effective_source_settings(project: &Project) -> SourceLayoutSettings {
    let mut settings = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source_layout_settings().clone())
        .unwrap_or_default();
    settings
        .direct_paragraph_overrides
        .extend(project.direct_paragraph_overrides.clone());
    settings
}

#[test]
fn session_layout_cache_hits_and_key_misses_keep_fresh_geometry_and_wrap_reuse() {
    let project = project_with(&[
        (Role::Action, "an opening action paragraph with enough words to wrap around"),
        (Role::Dialogue, "another unchanged element remains available for wrap reuse"),
    ]);
    let mut session = valid(EditingSession::new(project));
    let presentation = BodyEditorPresentation::for_project(session.project(), None);
    let mut cache = LayoutCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();

    let (first, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(diagnostics.layout_builds, 1);

    let (same, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut diagnostics,
    ));
    assert!(hit);
    assert!(std::sync::Arc::ptr_eq(&first, &same));
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(diagnostics.layout_builds, 0);

    let mut title_cache = LayoutCache::default();
    let (initial_title, hit) = valid(
        title_cache.get_or_build_title_page_editor_for_session_with_diagnostics(
            &session,
            &mut diagnostics,
        ),
    );
    assert!(!hit);
    let (same_title, hit) = valid(
        title_cache.get_or_build_title_page_editor_for_session_with_diagnostics(
            &session,
            &mut diagnostics,
        ),
    );
    assert!(hit);
    assert!(std::sync::Arc::ptr_eq(&initial_title, &same_title));
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(diagnostics.layout_builds, 0);

    // The full profile remains an equality key even when an identifying field
    // does not change the resulting geometry.
    let mut alternate_profile = session.layout_inputs().clone();
    alternate_profile.name.push_str("-cache-key");
    valid(session.set_layout_inputs(alternate_profile));
    let (profile_layout, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(diagnostics.layout_builds, 1);
    assert_eq!(
        *profile_layout,
        valid(ScreenplayLayout::build_with_body_presentation(
            session.project(),
            session.layout_inputs(),
            &effective_source_settings(session.project()),
            &presentation,
        )),
    );
    assert!(diagnostics.element_wraps_reused > 0);
    let (profile_title, hit) = valid(
        title_cache.get_or_build_title_page_editor_for_session_with_diagnostics(
            &session,
            &mut diagnostics,
        ),
    );
    assert!(!hit, "Title also keys on the complete profile");
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(
        *profile_title,
        valid(ScreenplayLayout::build_with_composition(
            session.project(),
            session.layout_inputs(),
            &effective_source_settings(session.project()),
            LayoutComposition::TitlePageEditor,
        )),
    );

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    valid(session.insert("edited "));
    let (revision_layout, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(diagnostics.layout_builds, 1);
    assert!(diagnostics.element_wraps_reused > 0);
    assert_eq!(
        *revision_layout,
        valid(ScreenplayLayout::build_with_body_presentation(
            session.project(),
            session.layout_inputs(),
            &effective_source_settings(session.project()),
            &presentation,
        )),
    );
    let (revision_title, hit) = valid(
        title_cache.get_or_build_title_page_editor_for_session_with_diagnostics(
            &session,
            &mut diagnostics,
        ),
    );
    assert!(!hit, "Title revision changes invalidate its complete layout");
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(
        *revision_title,
        valid(ScreenplayLayout::build_with_composition(
            session.project(),
            session.layout_inputs(),
            &effective_source_settings(session.project()),
            LayoutComposition::TitlePageEditor,
        )),
    );
}

#[test]
fn session_cache_misses_for_document_source_settings_and_canonical_direct_settings() {
    let first_project = project_with(&[(Role::Action, "same canonical screenplay")]);
    let first_session = valid(EditingSession::new(first_project));
    let first_presentation = BodyEditorPresentation::for_project(first_session.project(), None);
    let mut cache = LayoutCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let (_, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &first_session,
        &first_presentation,
        &mut diagnostics,
    ));
    assert!(!hit);

    // Replacing the document with an equal-looking but separately owned project
    // cannot return the prior layout identity.
    let replacement = valid(EditingSession::new(project_with(&[(
        Role::Action,
        "same canonical screenplay",
    )])));
    let replacement_presentation =
        BodyEditorPresentation::for_project(replacement.project(), None);
    let (replacement_layout, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &replacement,
        &replacement_presentation,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(
        *replacement_layout,
        valid(ScreenplayLayout::build_with_body_presentation(
            replacement.project(),
            replacement.layout_inputs(),
            &effective_source_settings(replacement.project()),
            &replacement_presentation,
        )),
    );

    // A source-backed import with a different retained Paragraph setting is a
    // distinct effective source-settings key. The canonical body is unchanged.
    let source_left = valid(read_v6(include_bytes!("fixtures/dual_dialogue_basic.fdx"))).project;
    let source_left = valid(EditingSession::new(source_left));
    let left_presentation = BodyEditorPresentation::for_project(source_left.project(), None);
    let _ = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &source_left,
        &left_presentation,
        &mut diagnostics,
    ));
    let source_right_bytes = include_bytes!("fixtures/dual_dialogue_basic.fdx");
    let source_right_text = std::str::from_utf8(source_right_bytes).expect("fixture is UTF-8");
    let changed_source = source_right_text.replacen(
        "Type=\"Action\"",
        "Type=\"Action\" Alignment=\"Center\"",
        1,
    );
    let source_right = valid(read_v6(changed_source.as_bytes())).project;
    let source_right = valid(EditingSession::new(source_right));
    let right_presentation = BodyEditorPresentation::for_project(source_right.project(), None);
    let (source_layout, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &source_right,
        &right_presentation,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(
        *source_layout,
        valid(ScreenplayLayout::build_with_body_presentation(
            source_right.project(),
            source_right.layout_inputs(),
            &effective_source_settings(source_right.project()),
            &right_presentation,
        )),
    );

    // Canonical direct-paragraph settings participate in the same complete key.
    let mut direct_project = project_with(&[(Role::Action, "direct override geometry")]);
    let direct_id = direct_project.elements[0].id.clone();
    direct_project.direct_paragraph_overrides.insert(
        direct_id,
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                alignment: Some(ParagraphAlignment::Right),
                ..SourceParagraphSpec::default()
            },
            ..SourceParagraphOverrides::default()
        },
    );
    let direct_session = valid(EditingSession::new(direct_project));
    let direct_presentation = BodyEditorPresentation::for_project(direct_session.project(), None);
    let (direct_layout, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &direct_session,
        &direct_presentation,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(
        *direct_layout,
        valid(ScreenplayLayout::build_with_body_presentation(
            direct_session.project(),
            direct_session.layout_inputs(),
            &effective_source_settings(direct_session.project()),
            &direct_presentation,
        )),
    );
}

#[test]
fn body_active_group_and_title_print_compositions_miss_without_partial_pagination() {
    let project = valid(read_v6(include_bytes!("fixtures/dual_dialogue_basic.fdx"))).project;
    let session = valid(EditingSession::new(project));
    let inactive = BodyEditorPresentation::for_project(session.project(), None);
    let group_id = session.project().dual_dialogue_groups[0].id.clone();
    let active = BodyEditorPresentation::for_project(session.project(), Some(group_id));
    let mut cache = LayoutCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let (_, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &inactive,
        &mut diagnostics,
    ));
    assert!(!hit);
    let (active_layout, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &active,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert!(!active_layout.editor_group_boundaries.is_empty());
    assert_eq!(
        *active_layout,
        valid(ScreenplayLayout::build_with_body_presentation(
            session.project(),
            session.layout_inputs(),
            &effective_source_settings(session.project()),
            &active,
        )),
    );
    assert!(diagnostics.element_wraps_reused > 0);

    let (title_layout, hit) = valid(cache.get_or_build_title_page_editor_for_session_with_diagnostics(
        &session,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(title_layout.composition, LayoutComposition::TitlePageEditor);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    assert_eq!(
        *title_layout,
        valid(ScreenplayLayout::build_with_composition(
            session.project(),
            session.layout_inputs(),
            &effective_source_settings(session.project()),
            LayoutComposition::TitlePageEditor,
        )),
    );

    let print_composition = LayoutComposition::Print {
        include_title_page: false,
    };
    let (print_layout, hit) = valid(cache.get_or_build_with_composition(
        session.revision(),
        session.project(),
        session.layout_inputs(),
        print_composition,
    ));
    assert!(!hit);
    assert_eq!(print_layout.composition, print_composition);
    assert_eq!(
        *print_layout,
        valid(ScreenplayLayout::build_with_composition(
            session.project(),
            session.layout_inputs(),
            &effective_source_settings(session.project()),
            print_composition,
        )),
    );
    assert!(print_layout.pages.iter().all(|page| !page.title_page));
}

#[test]
fn generic_signature_defense_and_session_metadata_transitions_remain_distinct() {
    let mut project = project_with(&[(Role::Action, "generic cache defenses")]);
    let inputs = LayoutInputs::default();
    let mut cache = LayoutCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let (_, hit) = valid(cache.get_or_build_with_inputs_for_purpose_and_diagnostics(
        0,
        &project,
        &inputs,
        LayoutPurpose::Editor,
        &mut diagnostics,
    ));
    assert!(!hit);
    assert_eq!(diagnostics.semantic_signature_constructions, 1);

    let session = valid(EditingSession::new(project.clone()));
    let unbound = BodyEditorPresentation::default();
    let (session_layout, hit) = valid(
        cache.get_or_build_body_editor_for_session_with_diagnostics(
            &session,
            &unbound,
            &mut diagnostics,
        ),
    );
    assert!(hit);
    assert_eq!(diagnostics.complete_project_validations, 0);
    assert_eq!(diagnostics.semantic_signature_constructions, 0);
    let (_, hit) = valid(cache.get_or_build_with_inputs_for_purpose_and_diagnostics(
        0,
        &project,
        &inputs,
        LayoutPurpose::Editor,
        &mut diagnostics,
    ));
    assert!(hit, "the session hit preserves the generic defensive signature metadata");
    assert_eq!(diagnostics.semantic_signature_constructions, 1);

    project.direct_paragraph_overrides.insert(
        project.elements[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                alignment: Some(ParagraphAlignment::Center),
                ..SourceParagraphSpec::default()
            },
            ..SourceParagraphOverrides::default()
        },
    );
    let (mutated, hit) = valid(cache.get_or_build_with_inputs_for_purpose_and_diagnostics(
        0,
        &project,
        &inputs,
        LayoutPurpose::Editor,
        &mut diagnostics,
    ));
    assert!(!hit, "same-revision canonical override mutation is defended");
    assert_eq!(diagnostics.semantic_signature_constructions, 1);
    assert_ne!(*mutated, *session_layout);
}

#[test]
fn cached_layout_remains_the_deterministic_caret_hit_page_and_export_authority() {
    let long_text = format!("{} end", "A deterministic screenplay line ".repeat(120));
    let project = project_with(&[
        (Role::Action, long_text.as_str()),
        (Role::Dialogue, "a second body element"),
    ]);
    let session = valid(EditingSession::new(project));
    let presentation = BodyEditorPresentation::for_project(session.project(), None);
    let mut cache = LayoutCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let (first, _) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut diagnostics,
    ));
    let (cached, hit) = valid(cache.get_or_build_body_editor_for_session_with_diagnostics(
        &session,
        &presentation,
        &mut diagnostics,
    ));
    assert!(hit);
    assert!(std::sync::Arc::ptr_eq(&first, &cached));

    let fresh = valid(ScreenplayLayout::build_with_body_presentation(
        session.project(),
        session.layout_inputs(),
        &effective_source_settings(session.project()),
        &presentation,
    ));
    assert_eq!(*cached, fresh);
    assert!(cached.pages.len() > 1);
    assert_eq!(cached.pages.len(), fresh.pages.len());
    let caret = Caret {
        element: 0,
        offset: 8,
    };
    assert_eq!(cached.caret_geometry(caret.clone()), fresh.caret_geometry(caret.clone()));
    let page = cached.page_for_position_with_diagnostics(
        caret.element,
        caret.offset,
        &mut Default::default(),
    );
    assert_eq!(page, fresh.page_for_position_with_diagnostics(
        caret.element,
        caret.offset,
        &mut Default::default(),
    ));
    let line = cached.line_for_caret_with_diagnostics(&caret, &mut Default::default());
    let fresh_line = fresh.line_for_caret_with_diagnostics(&caret, &mut Default::default());
    assert_eq!(line.unwrap().line, fresh_line.unwrap().line);
    let body_line_index = cached.body_line_index_for_position_with_diagnostics(
        caret.element,
        caret.offset,
        &mut Default::default(),
    );
    assert_eq!(
        body_line_index,
        fresh.body_line_index_for_position_with_diagnostics(
            caret.element,
            caret.offset,
            &mut Default::default(),
        ),
    );
    assert_eq!(cached.body_line(body_line_index.unwrap()), fresh.body_line(body_line_index.unwrap()));
    let visible_line = cached.pages[0].lines.first().expect("first page has a line");
    let x = FixedPoint::from_raw(visible_line.geometry.x.raw() + 2);
    let y = FixedPoint::from_raw(visible_line.paragraph_bounds.y.raw() + 1);
    assert_eq!(
        cached.hit_test_fixed(0, x, y),
        fresh.hit_test_fixed(0, x, y),
    );

    // PDF export remains a deterministic projection of the same semantic
    // document inputs; layout cache reads never mutate those inputs.
    assert_eq!(valid(export_pdf(session.project())), valid(export_pdf(session.project())));
}
