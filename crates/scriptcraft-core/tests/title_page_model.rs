use scriptcraft_core::{
    ModelError, Project, Role, RunAttribute, ScreenplayElement, TextRun, TitlePageAlignment,
    TitlePageParagraph,
};

#[test]
fn default_project_uses_the_54_paragraph_reference_order_and_keeps_body_default() {
    let project = Project::new().expect("project construction is fallible");
    assert_eq!(project.title_page.len(), 54);
    assert_eq!(project.elements.len(), 1);
    assert_eq!(project.elements[0].role, Role::SceneHeading);
    assert!(project.elements[0].text.is_empty());
    assert!(project.elements[0].runs.is_empty());

    let alignments = project
        .title_page
        .iter()
        .filter_map(|paragraph| paragraph.formatting.alignment)
        .collect::<Vec<_>>();
    assert_eq!(alignments.len(), 54);
    assert_eq!(alignments.iter().filter(|value| **value == TitlePageAlignment::Left).count(), 19);
    assert_eq!(alignments.iter().filter(|value| **value == TitlePageAlignment::Center).count(), 25);
    assert_eq!(alignments.iter().filter(|value| **value == TitlePageAlignment::Justified).count(), 10);
    assert!(project.title_page[0..17]
        .iter()
        .all(|paragraph| paragraph.formatting.alignment == Some(TitlePageAlignment::Left)));
    assert!(project.title_page[17..42]
        .iter()
        .all(|paragraph| paragraph.formatting.alignment == Some(TitlePageAlignment::Center)));
    assert!(project.title_page[42..44]
        .iter()
        .all(|paragraph| paragraph.formatting.alignment == Some(TitlePageAlignment::Left)));
    assert!(project.title_page[44..]
        .iter()
        .all(|paragraph| paragraph.formatting.alignment == Some(TitlePageAlignment::Justified)));

    let placeholders = [
        (17, "Script Title"),
        (21, "Written by"),
        (24, "Name of First Writer"),
        (29, "Based on, If Any"),
        (52, "Address"),
        (53, "Phone Number"),
    ];
    for (index, expected) in placeholders {
        let paragraph = &project.title_page[index];
        assert_eq!(paragraph.text, expected);
        assert_eq!(paragraph.runs.len(), 1);
    }
    assert_eq!(
        project.title_page[17].runs[0]
            .attributes
            .iter()
            .find(|attribute| attribute.name == "Style")
            .map(|attribute| attribute.value.as_str()),
        Some("Underline+AllCaps")
    );
    for index in [21, 24, 29, 52, 53] {
        assert_eq!(
            project.title_page[index].runs[0]
                .attributes
                .iter()
                .find(|attribute| attribute.name == "Style")
                .map(|attribute| attribute.value.as_str()),
            Some("")
        );
    }
    assert!(project
        .title_page
        .iter()
        .enumerate()
        .filter(|(index, _)| !placeholders.iter().any(|(placeholder, _)| placeholder == index))
        .all(|(_, paragraph)| paragraph.text.is_empty() && paragraph.runs.is_empty()));
}

#[test]
fn transient_title_ids_are_unique_project_wide_and_not_reference_ids() {
    let project = Project::new().expect("project construction is fallible");
    let mut ids = std::collections::BTreeSet::new();
    for paragraph in &project.title_page {
        assert!(paragraph.id.as_str().starts_with("title-"));
        assert!(ids.insert(paragraph.id.clone()));
    }
    for element in &project.elements {
        assert!(ids.insert(element.id.clone()));
    }
    assert!(!ids.contains(
        &scriptcraft_core::StableId::new("6b076af3-b05b-461b-9679-7959dbe3ed1d")
            .expect("reference foreign ID is valid as an opaque string")
    ));

    let mut duplicate = project.clone();
    duplicate.elements[0] = ScreenplayElement::new(
        duplicate.title_page[0].id.as_str(),
        Role::SceneHeading,
        "",
    )
    .expect("element construction is fallible");
    assert_eq!(duplicate.validate(), Err(ModelError::DuplicateId));
}

#[test]
fn title_validation_enforces_existing_text_run_attribute_and_count_limits() {
    let too_long = TitlePageParagraph::new(
        "oversized",
        "x".repeat(scriptcraft_core::MAX_TEXT_BYTES + 1),
        Default::default(),
    );
    assert_eq!(too_long.err(), Some(ModelError::TextTooLong));

    let too_many_runs = TitlePageParagraph::with_runs(
        "many-runs",
        None,
        vec![TextRun { text: String::new(), attributes: Vec::new() }; 1_025],
        None,
        Default::default(),
    );
    assert_eq!(too_many_runs.err(), Some(ModelError::TooManyRuns));

    let too_many_attributes = TitlePageParagraph::with_runs(
        "many-attributes",
        None,
        vec![TextRun {
            text: "x".into(),
            attributes: (0..129)
                .map(|index| RunAttribute { name: format!("a{index}"), value: String::new() })
                .collect(),
        }],
        None,
        Default::default(),
    );
    assert_eq!(too_many_attributes.err(), Some(ModelError::TooManyRunAttributes));

    let long_attribute = TitlePageParagraph::with_runs(
        "long-attribute",
        None,
        vec![TextRun {
            text: "x".into(),
            attributes: vec![RunAttribute {
                name: "a".repeat(4_097),
                value: String::new(),
            }],
        }],
        None,
        Default::default(),
    );
    assert_eq!(long_attribute.err(), Some(ModelError::StringTooLong));

    let mut bad_projection = TitlePageParagraph::with_runs(
        "bad-projection",
        None,
        vec![TextRun { text: "x".into(), attributes: Vec::new() }],
        None,
        Default::default(),
    )
    .expect("run constructor is fallible");
    bad_projection.runs[0].text = "different".into();
    assert_eq!(bad_projection.validate(), Err(ModelError::InvalidRunProjection));

    let mut too_many_paragraphs = Project::new().expect("project construction is fallible");
    too_many_paragraphs.title_page = (0..=scriptcraft_core::MAX_ELEMENTS)
        .map(|index| {
            TitlePageParagraph::new(format!("title-limit-{index}"), "", Default::default())
                .expect("title paragraph construction is fallible")
        })
        .collect();
    assert_eq!(too_many_paragraphs.validate(), Err(ModelError::TooManyElements));
}

#[test]
fn legacy_json_does_not_silently_drop_the_new_title_template() {
    let input = r#"{"schema_version":1,"project":{"id":"legacy-project","schema_version":1,"metadata":{"title":"","author":"","contact":"","created_at":"","updated_at":""},"title_page":[],"page_profile":{"name":"us_letter_screenplay","format_version":1,"paper_width":612,"paper_height":792,"margin_top":72,"margin_bottom":72,"margin_left":108,"margin_right":72,"chars_per_line":55,"lines_per_page":55,"line_advance":12,"boundary_gap":12,"uppercase_cues":true,"auto_parenthetical":true,"strict_transition_boundaries":true,"font_family":"Courier Prime","font_size":12,"char_width":7,"role_boxes":[{"x":108,"width":432},{"x":108,"width":432},{"x":259,"width":175},{"x":180,"width":216},{"x":216,"width":180},{"x":432,"width":108},{"x":108,"width":432}]},"elements":[{"id":"body-1","role":"scene_heading","text":""}]}}"#;
    let mut project = Project::from_json(input).expect("legacy JSON fixture is valid");
    project.title_page = Project::new()
        .expect("project construction is fallible")
        .title_page;
    assert_eq!(project.to_json(), Err(ModelError::UnsupportedJsonState));
}
