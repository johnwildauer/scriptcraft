#![forbid(unsafe_code)]

use scriptcraft_model::dual_dialogue::{
    active_dual_dialogue_projection, classify_structural_result, recognize_candidate,
    DualDialogueGroup, SelectionSurface, StructuralGroupOutcome,
};
use scriptcraft_model::formatted_text::{parse_style_attribute, RunStyleFlags};
use scriptcraft_model::{
    BodyOwner, ElementBehavior, ElementDefinition, ElementFontSpec, ElementOutline,
    ElementParagraphSpec, ElementTypeKey, ModelError, Position, Project, RunAttribute,
    ScreenplayElement, Selection, SourceFormat, SourceProvenance, SourceSection, TextRun,
    TitlePageAlignment, TitlePageParagraph, TitlePageParagraphFormatting,
};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("model smoke uses valid bounded input"))
}

fn test_project() -> Project {
    let definitions = [
        "Scene Heading",
        "Action",
        "Character",
        "Dialogue",
        "Parenthetical",
        "Transition",
        "General",
        "Shot",
        "Cast List",
    ]
    .into_iter()
    .map(|label| {
        valid(ElementDefinition::new(
            valid(ElementTypeKey::new(label)),
            ElementFontSpec {
                family: "Courier Prime".to_owned(),
                size: 12,
                bold: false,
                italic: false,
                underline: false,
                all_caps: false,
                hidden_text: false,
                color: None,
                adornment_style: 0,
                revision_id: 0,
            },
            ElementParagraphSpec {
                alignment: scriptcraft_model::formatting::ParagraphAlignment::Left,
                first_indent: Default::default(),
                left_indent: Default::default(),
                right_indent: scriptcraft_model::formatting::FixedPoint::from_points(500),
                leading: "Regular".to_owned(),
                space_before: Default::default(),
                spacing: 1,
                starts_new_page: false,
            },
            ElementBehavior {
                paginate_as: None,
                return_key: None,
                shortcut: None,
                tab_key: None,
                win_shortcut: None,
            },
            ElementOutline {
                can_hide: false,
                level: 0,
            },
        ))
    })
    .collect();
    valid(Project::with_element_definitions(definitions))
}

fn add_custom_type(project: &mut Project, label: &str, paginate_as: Option<&str>) {
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut definition = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("test fixture supplies the base Action definition"));
    definition.key = valid(ElementTypeKey::new(label));
    definition.behavior.paginate_as = paginate_as.map(str::to_owned);
    definitions.push(definition);
    valid(project.replace_element_definitions(definitions));
}

#[test]
fn model_public_api_smoke() {
    const BODY_TEXT: &str = "Café — opening";

    let mut project = test_project();
    let element = valid(ScreenplayElement::with_runs(
        "smoke-body",
        valid(ElementTypeKey::new("Action")),
        vec![
            TextRun {
                text: "Café".into(),
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold+VendorToken".into(),
                }],
            },
            TextRun {
                text: " — opening".into(),
                attributes: Vec::new(),
            },
        ],
        None,
    ));
    project.elements = vec![BodyOwner::shared(element)];
    project.title_page = vec![valid(TitlePageParagraph::new(
        "smoke-title",
        "Smoke Story",
        Default::default(),
    ))];

    assert_eq!(project.elements.len(), 1);
    assert_eq!(project.title_page.len(), 1);
    let body = project.elements[0].as_element();
    let title = &project.title_page[0];
    assert_eq!(body.id.as_str(), "smoke-body");
    assert_eq!(body.element_type().as_str(), "Action");
    assert_eq!(body.text, BODY_TEXT);
    assert_eq!(title.id.as_str(), "smoke-title");
    assert_eq!(title.element_type(), None);
    assert_eq!(title.text, "Smoke Story");

    let run_text = body
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    assert_eq!(body.text, run_text);
    assert_eq!(body.runs[0].attributes.len(), 1);
    assert_eq!(body.runs[0].attributes[0].name, "Style");
    assert_eq!(body.runs[0].attributes[0].value, "Bold+VendorToken");
    let style = valid(parse_style_attribute(&body.runs[0].attributes));
    assert_eq!(
        style.flags,
        RunStyleFlags {
            bold: true,
            ..RunStyleFlags::default()
        }
    );
    assert_eq!(style.unknown_tokens, vec!["VendorToken".to_owned()]);

    let e_acute = match BODY_TEXT.find('é') {
        Some(offset) => offset,
        None => unreachable!("body literal contains é"),
    };
    for offset in BODY_TEXT
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([BODY_TEXT.len()])
    {
        assert_eq!(body.validate_boundary(offset), Ok(()));
    }
    assert_eq!(
        body.validate_boundary(e_acute + 1),
        Err(ModelError::InvalidUtf8Boundary)
    );

    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn typed_paragraphs_resolve_definitions_and_typeless_titles_are_valid() {
    let mut body_project = test_project();
    body_project.title_page.clear();
    body_project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "missing-body-type",
        valid(ElementTypeKey::new("Action")),
        "body",
    )))];
    valid(body_project.replace_element_definitions(Vec::new()));
    assert_eq!(
        body_project.validate(),
        Err(ModelError::MissingElementDefinition(valid(
            ElementTypeKey::new("Action")
        )))
    );

    let mut title_project = test_project();
    title_project.elements.clear();
    title_project.title_page = vec![valid(TitlePageParagraph::with_runs(
        "missing-title-type",
        Some(valid(ElementTypeKey::new("Action"))),
        Vec::new(),
        None,
        Default::default(),
    ))];
    valid(title_project.replace_element_definitions(Vec::new()));
    assert_eq!(
        title_project.validate(),
        Err(ModelError::MissingElementDefinition(valid(
            ElementTypeKey::new("Action")
        )))
    );

    let mut typeless_title_project = test_project();
    typeless_title_project.elements.clear();
    valid(typeless_title_project.replace_element_definitions(Vec::new()));
    assert_eq!(typeless_title_project.validate(), Ok(()));
}

#[test]
fn canonical_open_keys_preserve_paragraph_identity_and_direct_formatting() {
    let mut project = test_project();
    let action_key = valid(ElementTypeKey::new("Action"));
    let lyrics_key = valid(ElementTypeKey::new("Lyrics"));
    let lowercase_lyrics_key = valid(ElementTypeKey::new("lyrics"));

    let mut lyrics_definition = project
        .element_definitions()
        .get(&action_key)
        .cloned()
        .unwrap_or_else(|| unreachable!("static bootstrap contains Action"));
    lyrics_definition.key = lyrics_key.clone();
    let mut lowercase_lyrics_definition = lyrics_definition.clone();
    lowercase_lyrics_definition.key = lowercase_lyrics_key.clone();
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions.extend([lyrics_definition, lowercase_lyrics_definition]);
    valid(project.replace_element_definitions(definitions));

    let body_source = SourceProvenance {
        format: SourceFormat::Fdx,
        section: SourceSection::Content,
        ordinal: 4,
        foreign_id: Some("foreign-body".to_owned()),
        foreign_role: Some("Action".to_owned()),
    };
    let body = valid(ScreenplayElement::with_runs(
        "open-key-body",
        action_key,
        vec![TextRun {
            text: "A sung line".to_owned(),
            attributes: vec![RunAttribute {
                name: "Style".to_owned(),
                value: "Italic".to_owned(),
            }],
        }],
        Some(body_source),
    ));
    let body_id = body.id.clone();
    let body_runs = body.runs.clone();
    let body_provenance = body.source_provenance.clone();
    project.elements = vec![BodyOwner::shared(body)];
    project
        .direct_paragraph_overrides
        .insert(body_id.clone(), Default::default());

    let title_formatting = TitlePageParagraphFormatting {
        alignment: Some(TitlePageAlignment::Center),
        first_indent: None,
        left_indent: None,
        right_indent: None,
        leading_regular: Some(false),
        space_before: None,
        spacing_single: Some(true),
        starts_new_page: Some(false),
    };
    let title_source = SourceProvenance {
        format: SourceFormat::Fdx,
        section: SourceSection::TitlePage,
        ordinal: 0,
        foreign_id: Some("foreign-title".to_owned()),
        foreign_role: Some("Action".to_owned()),
    };
    let title = valid(TitlePageParagraph::with_runs(
        "open-key-title",
        Some(valid(ElementTypeKey::new("Action"))),
        vec![TextRun {
            text: "Song title".to_owned(),
            attributes: Vec::new(),
        }],
        Some(title_source),
        title_formatting,
    ));
    let title_id = title.id.clone();
    let title_runs = title.runs.clone();
    let title_provenance = title.source_provenance.clone();
    let title_formatting_before = title.formatting;
    project.title_page = vec![title];

    assert_ne!(lyrics_key, lowercase_lyrics_key);
    assert!(project.element_definitions().get(&lyrics_key).is_some());
    assert!(project
        .element_definitions()
        .get(&lowercase_lyrics_key)
        .is_some());

    BodyOwner::update(&mut project.elements[0], |element| {
        element.set_element_type(lyrics_key.clone());
    });
    project.title_page[0].set_element_type(Some(lyrics_key.clone()));

    let body = project.elements[0].as_element();
    let title = &project.title_page[0];
    assert_eq!(body.element_type(), lyrics_key);
    assert_eq!(body.id, body_id);
    assert_eq!(body.runs, body_runs);
    assert_eq!(body.source_provenance, body_provenance);
    assert!(project.direct_paragraph_overrides.contains_key(&body_id));
    assert_eq!(title.element_type(), Some(lyrics_key));
    assert_eq!(title.id, title_id);
    assert_eq!(title.runs, title_runs);
    assert_eq!(title.source_provenance, title_provenance);
    assert_eq!(title.formatting, title_formatting_before);
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn arbitrary_missing_definition_fails_and_definition_values_affect_equality() {
    let lyrics_key = valid(ElementTypeKey::new("Lyrics"));
    let mut missing = test_project();
    missing.title_page.clear();
    missing.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "missing-lyrics",
        lyrics_key.clone(),
        "song",
    )))];
    valid(missing.replace_element_definitions(Vec::new()));
    assert_eq!(
        missing.validate(),
        Err(ModelError::MissingElementDefinition(lyrics_key))
    );

    let project = test_project();
    let mut equivalent_with_fresh_owner = project.clone();
    equivalent_with_fresh_owner.elements = project
        .elements
        .iter()
        .map(|owner| BodyOwner::shared(owner.as_element().clone()))
        .collect();
    assert_eq!(project, equivalent_with_fresh_owner);

    let mut changed_definition = project.clone();
    let mut definitions = changed_definition
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions[0].font.bold = !definitions[0].font.bold;
    valid(changed_definition.replace_element_definitions(definitions));
    assert_ne!(project, changed_definition);
}

#[test]
fn dual_group_membership_survives_inactive_projection_and_reactivation() {
    let mut project = test_project();
    let members = [
        valid(ScreenplayElement::new(
            "dual-left-cue",
            valid(ElementTypeKey::new("Character")),
            "LEFT",
        )),
        valid(ScreenplayElement::new(
            "dual-left-line",
            valid(ElementTypeKey::new("Dialogue")),
            "left line",
        )),
        valid(ScreenplayElement::new(
            "dual-right-cue",
            valid(ElementTypeKey::new("Character")),
            "RIGHT",
        )),
        valid(ScreenplayElement::new(
            "dual-right-line",
            valid(ElementTypeKey::new("Parenthetical")),
            "(right)",
        )),
    ];
    let source_members = members
        .iter()
        .map(|member| member.id.clone())
        .collect::<Vec<_>>();
    project.elements = members.into_iter().map(BodyOwner::shared).collect();
    let group = DualDialogueGroup {
        id: valid(scriptcraft_model::StableId::new("dual-source-group")),
        members: source_members.clone(),
        wrapper: Default::default(),
    };
    project.dual_dialogue_groups.push(group);

    let active = active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0])
        .unwrap_or_else(|| unreachable!("known two-cue pattern is active"));
    assert_eq!(active.left.character.as_str(), "dual-left-cue");
    assert_eq!(active.left.paragraphs[0].as_str(), "dual-left-line");
    assert_eq!(active.right.character.as_str(), "dual-right-cue");
    assert_eq!(active.right.paragraphs[0].as_str(), "dual-right-line");

    let active_baseline = project.clone();
    BodyOwner::update(&mut project.elements[1], |element| {
        element.set_element_type(valid(ElementTypeKey::new("Action")));
    });
    assert_eq!(project.validate(), Ok(()));
    assert_eq!(project.dual_dialogue_groups[0].members, source_members);
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    assert_eq!(
        classify_structural_result(
            &active_baseline,
            Selection::caret(Position {
                element: 1,
                offset: 0,
            }),
            &project,
        )[0]
        .outcome,
        StructuralGroupOutcome::UpdatedValid
    );

    BodyOwner::update(&mut project.elements[1], |element| {
        element.set_element_type(valid(ElementTypeKey::new("Dialogue")));
    });
    assert_eq!(project.validate(), Ok(()));
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_some());
}

#[test]
fn dual_recognition_uses_exact_capabilities_not_paginate_as_or_type_spelling() {
    let mut project = test_project();
    add_custom_type(&mut project, "character", Some("Character"));
    add_custom_type(&mut project, "Dialogue-like", Some("Dialogue"));
    add_custom_type(&mut project, "Parenthetical-like", Some("Parenthetical"));

    project.elements = [
        ("dual-cue-a", "Character", "A"),
        ("dual-line-a", "Dialogue-like", "line A"),
        ("dual-cue-b", "character", "B"),
        ("dual-line-b", "Parenthetical-like", "(line B)"),
    ]
    .into_iter()
    .map(|(id, element_type, text)| {
        BodyOwner::shared(valid(ScreenplayElement::new(
            id,
            valid(ElementTypeKey::new(element_type)),
            text,
        )))
    })
    .collect();
    let members = project
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: valid(scriptcraft_model::StableId::new("dual-capability-group")),
        members: members.clone(),
        wrapper: Default::default(),
    });

    assert_eq!(project.validate(), Ok(()));
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    assert_eq!(project.dual_dialogue_groups[0].members, members);

    // The same arbitrary pattern is not promoted to an active group when it
    // has no source membership, nor is it a toggle candidate by label or
    // PaginateAs resemblance alone.
    project.dual_dialogue_groups.clear();
    assert!(recognize_candidate(&project, 0).unwrap_or(None).is_none());
    assert!(scriptcraft_model::dual_dialogue::resolve_toggle_target(
        &project,
        SelectionSurface::Body,
        Selection::caret(Position {
            element: 0,
            offset: 0,
        }),
    )
    .unwrap_or(None)
    .is_none());
}

#[test]
fn dual_projection_allows_empty_sides_and_rejects_a_third_cue() {
    let mut project = test_project();
    project.elements = [
        ("empty-left-cue", "Character", "LEFT"),
        ("empty-right-cue", "Character", "RIGHT"),
    ]
    .into_iter()
    .map(|(id, element_type, text)| {
        BodyOwner::shared(valid(ScreenplayElement::new(
            id,
            valid(ElementTypeKey::new(element_type)),
            text,
        )))
    })
    .collect();
    let group = DualDialogueGroup {
        id: valid(scriptcraft_model::StableId::new("empty-sides-group")),
        members: project
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect(),
        wrapper: Default::default(),
    };
    project.dual_dialogue_groups.push(group);
    let projection = active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0])
        .unwrap_or_else(|| unreachable!("two adjacent Character capabilities permit empty sides"));
    assert!(projection.left.paragraphs.is_empty());
    assert!(projection.right.paragraphs.is_empty());

    let third_cue = BodyOwner::shared(valid(ScreenplayElement::new(
        "third-dual-cue",
        valid(ElementTypeKey::new("Character")),
        "THIRD",
    )));
    project.elements.push(third_cue.clone());
    project.dual_dialogue_groups[0]
        .members
        .push(third_cue.id.clone());
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    assert!(recognize_candidate(&project, 0).unwrap_or(None).is_none());
}
