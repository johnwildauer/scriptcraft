use scriptcraft_core::preservation::{PreservationError, PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{
    fdx, parse_style_attribute, project_lit_styles, project_paragraph_alignment,
    set_paragraph_alignments, styled_display_spans, toggle_paragraph_run_style,
    toggle_paragraph_run_styles, toggle_style_for_range, FormattedTextError, ParagraphAlignment,
    project_style_at_caret, ParagraphTarget, Project, RunAttribute, RunBoundaryAffinity, RunStyle,
    RunStyleFlags, TextRun,
};

fn attribute(name: &str, value: &str) -> RunAttribute {
    RunAttribute {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

fn style(run: &TextRun) -> Option<&str> {
    run.attributes
        .iter()
        .find(|attribute| attribute.name == "Style")
        .map(|attribute| attribute.value.as_str())
}

fn find_managed<'a>(nodes: &'a [PreservedNode], id: &scriptcraft_core::StableId) -> Option<&'a PreservedElement> {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id) {
                return Some(element);
            }
            if let Some(found) = find_managed(&element.children, id) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn style_projection_recognizes_supported_tokens_alias_and_unknowns() {
    let projection = parse_style_attribute(&[
        attribute("Style", "Bold+Italic+Underline+AllCaps+StrikeThrough+VendorCase"),
        attribute("Font", "Courier Prime"),
    ])
    .expect("style projection is bounded");
    assert_eq!(
        projection.flags,
        RunStyleFlags {
            bold: true,
            italics: true,
            underline: true,
            all_caps: true,
            strikethrough: true,
        }
    );
    assert_eq!(projection.unknown_tokens, ["VendorCase"]);
}

#[test]
fn each_supported_style_token_projects_independently_and_run_boundary_affinity_is_explicit() {
    let tokens = [
        ("Bold", RunStyle::Bold),
        ("Italic", RunStyle::Italics),
        ("Underline", RunStyle::Underline),
        ("AllCaps", RunStyle::AllCaps),
        ("Strikethrough", RunStyle::Strikethrough),
    ];
    for (token, style) in tokens {
        assert!(parse_style_attribute(&[attribute("Style", token)])
            .expect("supported token parses")
            .flags
            .contains(style));
    }
    let runs = vec![
        TextRun {
            text: "a".to_owned(),
            attributes: vec![attribute("Style", "Bold")],
        },
        TextRun {
            text: "b".to_owned(),
            attributes: vec![attribute("Style", "Italic")],
        },
    ];
    assert!(project_style_at_caret("ab", &runs, 1, RunBoundaryAffinity::Left, RunStyleFlags::default())
        .expect("valid run boundary")
        .bold);
    assert!(project_style_at_caret("ab", &runs, 1, RunBoundaryAffinity::Right, RunStyleFlags::default())
        .expect("valid run boundary")
        .italics);
}

#[test]
fn mixed_selection_uses_any_lit_and_adds_style_at_utf8_boundaries_without_attribute_loss() {
    let runs = vec![
        TextRun {
            text: "aß".to_owned(),
            attributes: vec![
                attribute("Font", "Courier Prime"),
                attribute("Style", "Vendor+StrikeThrough"),
                attribute("RevisionID", "7"),
            ],
        },
        TextRun {
            text: "éz".to_owned(),
            attributes: vec![attribute("Font", "Courier Prime"), attribute("Color", "blue")],
        },
    ];
    let mut active = RunStyleFlags::default();
    active.bold = true;
    assert_eq!(
        project_lit_styles("aßéz", &runs, 1..6, active).expect("range is on UTF-8 boundaries"),
        RunStyleFlags {
            strikethrough: true,
            ..RunStyleFlags::default()
        }
    );

    let edited = toggle_style_for_range("aßéz", &runs, 1..6, active, RunStyle::Underline)
        .expect("multibyte range splits without losing attributes");
    assert_eq!(edited.runs.iter().map(|run| run.text.as_str()).collect::<String>(), "aßéz");
    assert_eq!(style(&edited.runs[0]), Some("Vendor+StrikeThrough"));
    assert_eq!(style(&edited.runs[1]), Some("Vendor+StrikeThrough+Underline"));
    assert_eq!(
        edited.runs[1]
            .attributes
            .iter()
            .map(|attribute| attribute.name.as_str())
            .collect::<Vec<_>>(),
        ["Font", "Style", "RevisionID"]
    );
    assert!(edited.runs[2]
        .attributes
        .iter()
        .any(|attribute| attribute.name == "Color" && attribute.value == "blue"));
    assert_eq!(edited.active_style, active);
}

#[test]
fn complete_selection_removes_style_and_only_equal_ordered_attributes_coalesce() {
    let runs = vec![
        TextRun {
            text: "ab".to_owned(),
            attributes: vec![attribute("Style", "Bold+Unknown"), attribute("Font", "A")],
        },
        TextRun {
            text: "cd".to_owned(),
            attributes: vec![attribute("Style", "Bold+Unknown"), attribute("Font", "A")],
        },
        TextRun {
            text: "ef".to_owned(),
            attributes: vec![attribute("Font", "A"), attribute("Style", "Bold+Unknown")],
        },
    ];
    let edited = toggle_style_for_range("abcdef", &runs, 0..4, RunStyleFlags::default(), RunStyle::Bold)
        .expect("all selected characters carry Bold");
    assert_eq!(edited.runs.len(), 2);
    assert_eq!(edited.runs[0].text, "abcd");
    assert_eq!(style(&edited.runs[0]), Some("Unknown"));
    assert_eq!(edited.runs[1].text, "ef");
    assert_eq!(style(&edited.runs[1]), Some("Bold+Unknown"));
    assert_eq!(edited.runs[1].attributes[0].name, "Font");
}

#[test]
fn collapsed_empty_paragraph_keeps_typing_style_transient_and_creates_no_runs() {
    let mut active = RunStyleFlags::default();
    active.italics = true;
    let edited = toggle_style_for_range("", &[], 0..0, active, RunStyle::AllCaps)
        .expect("empty paragraph caret is valid");
    assert!(edited.runs.is_empty());
    assert!(!edited.changed);
    assert!(edited.active_style.all_caps && edited.active_style.italics);
}

#[test]
fn cross_paragraph_mixed_style_selection_adds_style_everywhere() {
    let mut project = Project::new().expect("default project is valid");
    project.elements[0].text = "body".to_owned();
    project.elements[0].runs = vec![TextRun {
        text: "body".to_owned(),
        attributes: vec![attribute("Style", "")],
    }];
    let edits = toggle_paragraph_run_styles(
        &mut project,
        &[
            (ParagraphTarget::Body(0), 0..4),
            (ParagraphTarget::TitlePage(17), 0..11),
        ],
        RunStyleFlags::default(),
        RunStyle::Bold,
    )
    .expect("one all-on policy applies across body and title ranges");
    assert!(edits.iter().all(|edit| edit.changed));
    assert_eq!(style(&project.elements[0].runs[0]), Some("Bold"));
    assert_eq!(style(&project.title_page[17].runs[0]), Some("Underline+AllCaps+Bold"));
}

#[test]
fn all_caps_display_expansion_maps_back_to_unchanged_canonical_utf8_ranges() {
    let text = "ßa";
    let runs = vec![TextRun {
        text: text.to_owned(),
        attributes: vec![attribute("Style", "AllCaps")],
    }];
    let spans = styled_display_spans(text, &runs).expect("canonical text and run agree");
    assert_eq!(text, "ßa");
    assert_eq!(spans[0].display_text, "SSA");
    assert_eq!(spans[0].source_range, 0..3);
    assert_eq!(spans[0].display_to_source[0].display_range, 0..1);
    assert_eq!(spans[0].display_to_source[0].source_range, 0..2);
    assert_eq!(spans[0].display_to_source[1].display_range, 1..2);
    assert_eq!(spans[0].display_to_source[1].source_range, 0..2);
    assert_eq!(spans[0].display_to_source[2].source_range, 2..3);
}

#[test]
fn body_and_title_alignment_setters_update_only_alignment_and_project_mixed_state() {
    let mut project = Project::new().expect("default project is valid");
    for alignment in [
        ParagraphAlignment::Left,
        ParagraphAlignment::Center,
        ParagraphAlignment::Right,
        ParagraphAlignment::Justified,
    ] {
        set_paragraph_alignments(
            &mut project,
            &[ParagraphTarget::Body(0), ParagraphTarget::TitlePage(0)],
            alignment,
        )
        .expect("body and title alignments share one setter");
        assert_eq!(
            project.direct_paragraph_overrides[&project.elements[0].id]
                .paragraph
                .alignment,
            Some(alignment)
        );
        assert_eq!(
            project.title_page[0].formatting.alignment,
            Some(alignment.into())
        );
    }
    assert_eq!(
        project_paragraph_alignment(&[
            Some(ParagraphAlignment::Justified),
            Some(ParagraphAlignment::Justified),
        ])
        .alignment,
        Some(ParagraphAlignment::Justified)
    );
    assert_eq!(
        project_paragraph_alignment(&[
            Some(ParagraphAlignment::Left),
            Some(ParagraphAlignment::Justified),
        ])
        .alignment,
        None
    );
}

#[test]
fn direct_alignment_mutation_preserves_unrelated_source_attribute_order() {
    let source = "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" Alignment=\"Center\" StartsNewPage=\"Yes\" OutlineLevel=\"2\" vendor=\"keep\" id=\"p1\"><Text>body</Text></Paragraph></Content><TitlePage><Content><Paragraph Alignment=\"Right\" StartsNewPage=\"No\" vendor-title=\"keep\" id=\"tp1\"><Text>title</Text></Paragraph></Content></TitlePage></FinalDraft>";
    let mut project = fdx::read_v6(source.as_bytes())
        .expect("synthetic Version 6 fixture imports")
        .project;
    let id = project.elements[0].id.clone();
    let title_id = project.title_page[0].id.clone();
    set_paragraph_alignments(
        &mut project,
        &[ParagraphTarget::Body(0), ParagraphTarget::TitlePage(0)],
        ParagraphAlignment::Justified,
    )
    .expect("source-backed direct attribute is safely updated");
    let preservation = project.preservation.as_ref().expect("import retains source");
    let paragraph = find_managed(&preservation.sections[0].children, &id)
        .expect("managed body paragraph remains indexed");
    assert_eq!(
        paragraph.attributes.iter().map(|item| item.name.local_name.as_str()).collect::<Vec<_>>(),
        [
            "Type",
            "Alignment",
            "StartsNewPage",
            "OutlineLevel",
            "vendor",
            "id"
        ]
    );
    assert_eq!(paragraph.attributes[1].value, "Full");
    assert_eq!(paragraph.attributes[4].value, "keep");
    let title = find_managed(&preservation.sections[0].children, &title_id)
        .expect("managed title paragraph remains indexed");
    assert_eq!(
        title.attributes.iter().map(|item| item.name.local_name.as_str()).collect::<Vec<_>>(),
        ["Alignment", "StartsNewPage", "vendor-title", "id"]
    );
    assert_eq!(title.attributes[0].value, "Full");
    assert_eq!(title.attributes[1].value, "No");
}

#[test]
fn style_mutation_safe_stops_when_opaque_source_child_splits_text_runs() {
    let source = "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" id=\"p1\"><Text>a</Text><!-- preserved barrier --><Text>b</Text></Paragraph></Content></FinalDraft>";
    let mut project = fdx::read_v6(source.as_bytes())
        .expect("synthetic Version 6 fixture imports")
        .project;
    let before = project.clone();
    let error = toggle_paragraph_run_style(
        &mut project,
        ParagraphTarget::Body(0),
        0..2,
        RunStyleFlags::default(),
        RunStyle::Bold,
    )
    .expect_err("style range cannot cross an opaque source child");
    assert_eq!(
        error,
        FormattedTextError::Preservation(PreservationError::UnsupportedRunTopology)
    );
    assert_eq!(project, before);
}

#[test]
fn style_mutation_synchronizes_valid_adjacent_source_text_elements() {
    let source = "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" id=\"p1\"><Text>first</Text><Text Style=\"Italic\"> second</Text></Paragraph></Content></FinalDraft>";
    let mut project = fdx::read_v6(source.as_bytes())
        .expect("synthetic Version 6 fixture imports")
        .project;
    let text = project.elements[0].text.clone();
    toggle_paragraph_run_style(
        &mut project,
        ParagraphTarget::Body(0),
        0..text.len(),
        RunStyleFlags::default(),
        RunStyle::Bold,
    )
    .expect("adjacent source Text elements synchronize without an interior barrier");

    assert_eq!(project.elements[0].text, "first second");
    assert_eq!(project.elements[0].runs.len(), 2);
    assert_eq!(style(&project.elements[0].runs[0]), Some("Bold"));
    assert!(style(&project.elements[0].runs[1]).is_some_and(|value| {
        value.split('+').any(|token| token == "Bold")
            && value.split('+').any(|token| token == "Italic")
    }));
    let round_trip = fdx::read_v6(&fdx::write_v6(&project).expect("valid multiple Text source serializes"))
        .expect("synchronized multiple Text source remains readable");
    assert_eq!(round_trip.project.elements[0].text, "first second");
    assert_eq!(round_trip.project.elements[0].runs, project.elements[0].runs);
}

#[test]
fn style_mutation_without_a_text_template_returns_mapping_error_transactionally() {
    let source = "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" id=\"p1\"/></Content></FinalDraft>";
    let mut project = fdx::read_v6(source.as_bytes())
        .expect("synthetic Version 6 fixture imports")
        .project;
    project.elements[0].text = "body".to_owned();
    let before = project.clone();
    let error = toggle_paragraph_run_style(
        &mut project,
        ParagraphTarget::Body(0),
        0..4,
        RunStyleFlags::default(),
        RunStyle::Bold,
    )
    .expect_err("nonempty text without a source Text template is unmappable");
    assert_eq!(
        error,
        FormattedTextError::Preservation(PreservationError::UnsupportedTextMapping)
    );
    assert_eq!(project, before);
}

#[test]
fn newly_introduced_unresolvable_prefixed_run_attribute_is_rejected_by_serializer() {
    let mut project = Project::new().expect("default project is valid");
    project.elements[0].text = "safe".to_owned();
    project.elements[0].runs = vec![TextRun {
        text: "safe".to_owned(),
        attributes: vec![attribute("vendor:Custom", "kept only if resolvable")],
    }];
    assert!(fdx::write_v6(&project).is_err());
}

#[test]
fn full_alignment_maps_to_fdx_full_and_round_trips_from_import() {
    assert_eq!(ParagraphAlignment::from_fdx("Full"), Some(ParagraphAlignment::Justified));
    assert_eq!(ParagraphAlignment::Justified.fdx_value(), "Full");
    let source = "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" Alignment=\"Full\" id=\"p1\"><Text>body</Text></Paragraph></Content></FinalDraft>";
    let project = fdx::read_v6(source.as_bytes())
        .expect("synthetic Version 6 fixture imports")
        .project;
    assert_eq!(
        project.direct_paragraph_overrides[&project.elements[0].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Justified)
    );
}

#[test]
fn direct_body_alignment_on_new_project_is_emitted_as_fdx_full() {
    let mut project = Project::new().expect("default project is valid");
    set_paragraph_alignments(
        &mut project,
        &[ParagraphTarget::Body(0)],
        ParagraphAlignment::Justified,
    )
    .expect("new body paragraph accepts a direct override");
    let written = fdx::write_v6(&project).expect("bounded new document serializer accepts it");
    let reopened = fdx::read_v6(&written).expect("generated FDX is structurally readable");
    assert_eq!(
        reopened.project.direct_paragraph_overrides[&reopened.project.elements[0].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Justified)
    );
}
