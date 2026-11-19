#![forbid(unsafe_code)]

use scriptcraft_fdx::{
    import_fdx, read_v6, FdxImportDiagnostic, FdxImportOutcome, FdxMigrationConsent,
    FdxMigrationKind, MAX_FDX_IMPORT_DIAGNOSTICS,
};
use scriptcraft_model::element_definition::SemanticCapabilities;
use scriptcraft_model::formatted_text::{parse_style_attribute, RunStyleFlags};
use scriptcraft_model::formatting::ParagraphAlignment;
use scriptcraft_model::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_model::Project;

const OPEN_FIXTURE: &[u8] = include_bytes!("fixtures/open_element_settings.fdx");
const OLDER_FIXTURE: &[u8] = include_bytes!("fixtures/older_open_element_settings.fdx");
const SMOKE_FIXTURE: &[u8] = include_bytes!("fixtures/smoke_screenplay.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("open-import regression uses bounded valid fixtures"))
}

fn attribute<'a>(element: &'a PreservedElement, local_name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == local_name
        })
        .map(|attribute| attribute.value.as_str())
}

fn child_element<'a>(
    element: &'a PreservedElement,
    local_name: &str,
) -> Option<&'a PreservedElement> {
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child)
            if child.name.local_name == local_name
                && child.name.namespace_uri == element.name.namespace_uri =>
        {
            Some(child)
        }
        _ => None,
    })
}

fn child_element_with_attribute<'a>(
    element: &'a PreservedElement,
    local_name: &str,
    attribute_name: &str,
    expected_value: &str,
) -> Option<&'a PreservedElement> {
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child)
            if child.name.local_name == local_name
                && child.name.namespace_uri == element.name.namespace_uri
                && attribute(child, attribute_name) == Some(expected_value) =>
        {
            Some(child)
        }
        _ => None,
    })
}

fn direct_text(element: &PreservedElement) -> String {
    element
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn source_root(project: &Project) -> Option<&PreservedElement> {
    project
        .preservation
        .as_ref()?
        .sections
        .iter()
        .flat_map(|section| section.children.iter())
        .find_map(|node| match node {
            PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
            _ => None,
        })
}

fn body_by_foreign_id<'a>(
    project: &'a Project,
    foreign_id: &str,
) -> Option<&'a scriptcraft_model::ScreenplayElement> {
    project.elements.iter().find_map(|owner| {
        let element = owner.as_element();
        (element
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            == Some(foreign_id))
        .then_some(element)
    })
}

fn title_by_foreign_id<'a>(
    project: &'a Project,
    foreign_id: &str,
) -> Option<&'a scriptcraft_model::TitlePageParagraph> {
    project.title_page.iter().find(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            == Some(foreign_id)
    })
}

#[test]
fn open_import_completes_ordered_definitions_and_manages_arbitrary_content() {
    let imported = valid(read_v6(OPEN_FIXTURE));
    let project = imported.project;
    assert_eq!(project.validate(), Ok(()));
    assert_eq!(
        imported.import_diagnostics.entries,
        vec![FdxImportDiagnostic::DuplicateElementSettings {
            settings_ordinal: 5
        }]
    );
    assert_eq!(imported.import_diagnostics.suppressed_duplicate_count, 0);

    let definitions = project.element_definitions().iter().collect::<Vec<_>>();
    let labels = definitions
        .iter()
        .map(|definition| definition.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "Lyrics",
            "Notations",
            "action",
            "Action",
            "lyrics",
            "titlefirst",
            "TitleContent",
            "NoSettingsBody",
            "UsedArbitraryUnconfigured",
            "Dialogue",
            "General",
            "Scene Heading",
            "Character",
            "Parenthetical",
            "Transition",
            "Shot",
            "Cast List",
        ]
    );
    assert!(
        !labels.contains(&"TitleFirst"),
        "the opaque direct TitlePage Paragraph must not add a definition"
    );
    assert!(definitions
        .iter()
        .all(|definition| definition.validate().is_ok()));
    assert!(project
        .element_definitions()
        .get_by_label("TitleFirst")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("HeaderOnlyTitle")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("FooterOnlyTitle")
        .is_none());

    let required_bases = [
        "General",
        "Scene Heading",
        "Action",
        "Character",
        "Parenthetical",
        "Dialogue",
        "Transition",
        "Shot",
        "Cast List",
    ];
    assert!(required_bases
        .iter()
        .all(|label| project.element_definitions().get_by_label(label).is_some()));
    assert_eq!(
        project
            .element_definitions()
            .get_by_label("action")
            .map(|definition| definition.key.as_str()),
        Some("action")
    );
    assert!(project
        .element_definitions()
        .get_by_label("Action")
        .is_some());
    assert!(project
        .element_definitions()
        .get_by_label("ACTION")
        .is_none());

    let lyrics = project
        .element_definitions()
        .get_by_label("Lyrics")
        .unwrap_or_else(|| unreachable!("Lyrics has its winning source settings"));
    assert_eq!(lyrics.font.family, "Avenir Next");
    assert_eq!(lyrics.font.size, 14);
    assert!(lyrics.font.bold && lyrics.font.italic && lyrics.font.underline);
    assert!(lyrics.font.all_caps && lyrics.font.hidden_text);
    assert_eq!(lyrics.font.color.as_deref(), Some("#87878F8FE0E0"));
    assert_eq!(lyrics.font.adornment_style, 2);
    assert_eq!(lyrics.font.revision_id, 7);
    assert_eq!(lyrics.paragraph.alignment, ParagraphAlignment::Justified);
    assert_eq!(lyrics.paragraph.first_indent.raw(), -1_800);
    assert_eq!(lyrics.paragraph.left_indent.raw(), 9_000);
    assert_eq!(lyrics.paragraph.right_indent.raw(), 3_600);
    assert_eq!(lyrics.paragraph.leading, "1.3");
    assert_eq!(lyrics.paragraph.space_before.raw(), 450);
    assert_eq!(lyrics.paragraph.spacing, 2);
    assert!(lyrics.paragraph.starts_new_page);
    assert_eq!(lyrics.behavior.paginate_as.as_deref(), Some("Action"));
    assert_eq!(
        lyrics.behavior.return_key.as_ref().map(|key| key.as_str()),
        Some("action")
    );
    assert_eq!(
        lyrics.behavior.tab_key.as_ref().map(|key| key.as_str()),
        Some("ACTION")
    );
    assert_eq!(lyrics.behavior.shortcut.as_deref(), Some(""));
    assert_eq!(lyrics.behavior.win_shortcut.as_deref(), Some("Alt+L"));
    assert!(lyrics.outline.can_hide);
    assert_eq!(lyrics.outline.level, 4);
    assert_eq!(lyrics.capabilities(), SemanticCapabilities::default());

    let notations = project
        .element_definitions()
        .get_by_label("Notations")
        .unwrap_or_else(|| unreachable!("Notations retains its incomplete source definition"));
    assert_eq!(notations.font.family, "Courier Final Draft");
    assert_eq!(notations.font.size, 12);
    assert_eq!(notations.font.color.as_deref(), Some("#000000000001"));
    assert_eq!(notations.behavior.shortcut.as_deref(), Some(""));
    assert_eq!(notations.behavior.win_shortcut.as_deref(), Some("Alt+N"));
    assert_eq!(notations.outline.level, 1);

    let lower_lyrics = project
        .element_definitions()
        .get_by_label("lyrics")
        .unwrap_or_else(|| unreachable!("case-distinct lowercase settings are retained"));
    assert_eq!(lower_lyrics.font.family, "Book Antiqua");
    assert_eq!(lower_lyrics.font.size, 10);
    assert_eq!(lower_lyrics.font.italic, true);
    assert_eq!(lower_lyrics.capabilities(), SemanticCapabilities::default());

    let lower_action = project
        .element_definitions()
        .get_by_label("action")
        .unwrap_or_else(|| unreachable!("lowercase action has its own winning definition"));
    assert_eq!(lower_action.font.color, None);
    assert_eq!(
        lower_action
            .behavior
            .return_key
            .as_ref()
            .map(|key| key.as_str()),
        Some("Action")
    );
    assert_eq!(
        lower_action
            .behavior
            .tab_key
            .as_ref()
            .map(|key| key.as_str()),
        Some("dialogue")
    );
    assert_eq!(lower_action.behavior.shortcut.as_deref(), Some(""));
    assert_eq!(
        project
            .element_definitions()
            .get_by_label("Action")
            .and_then(|definition| definition.font.color.as_deref()),
        None
    );

    let dialogue = project
        .element_definitions()
        .get_by_label("Dialogue")
        .unwrap_or_else(|| unreachable!("used base type receives its packaged value"));
    assert_eq!(dialogue.font.family, "Courier Final Draft");
    assert_eq!(dialogue.paragraph.left_indent.raw(), 18_000);
    assert_eq!(dialogue.capabilities().dialogue, true);

    let general_derived = project
        .element_definitions()
        .get_by_label("NoSettingsBody")
        .unwrap_or_else(|| unreachable!("used arbitrary type receives a General-derived value"));
    let general = project
        .element_definitions()
        .get_by_label("General")
        .unwrap_or_else(|| unreachable!("missing required General base is appended"));
    assert_eq!(general_derived.font, general.font);
    assert_eq!(general_derived.paragraph, general.paragraph);
    assert_eq!(general_derived.behavior, general.behavior);
    assert_eq!(general_derived.outline, general.outline);
    assert_eq!(
        general_derived.capabilities(),
        SemanticCapabilities::default()
    );
    assert_eq!(
        project
            .element_definitions()
            .get_by_label("UsedArbitraryUnconfigured")
            .map(|definition| definition.font.family.as_str()),
        Some("Courier Final Draft")
    );

    assert_eq!(project.elements.len(), 7);
    let body = body_by_foreign_id(&project, "open-body-general-derived")
        .unwrap_or_else(|| unreachable!("arbitrary body paragraph is managed"));
    assert_eq!(body.element_type().as_str(), "NoSettingsBody");
    assert!(body.id.as_str().starts_with("fdx-element-"));
    assert_eq!(body.text, "General-derived body");
    assert_eq!(
        project
            .direct_paragraph_overrides
            .get(&body.id)
            .and_then(|overrides| overrides.paragraph.alignment),
        Some(ParagraphAlignment::Center)
    );
    assert_eq!(body.runs.len(), 1);
    assert_eq!(
        body.source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref()),
        Some("open-body-general-derived")
    );
    assert!(body_by_foreign_id(&project, "open-body-unconfigured").is_some());
    assert!(body_by_foreign_id(&project, "open-body-packaged-base")
        .is_some_and(|element| element.element_type().as_str() == "Dialogue"));
    assert!(body_by_foreign_id(&project, "open-body-lower-action")
        .is_some_and(|element| element.element_type().as_str() == "action"));
    assert!(body_by_foreign_id(&project, "open-body-action")
        .is_some_and(|element| element.element_type().as_str() == "Action"));
    assert!(body_by_foreign_id(&project, "open-body-missing-type").is_none());
    assert!(body_by_foreign_id(&project, "open-body-empty-type").is_none());

    let lyrics_body = body_by_foreign_id(&project, "open-body-lyrics")
        .unwrap_or_else(|| unreachable!("Lyrics body paragraph is managed"));
    assert_eq!(lyrics_body.element_type().as_str(), "Lyrics");
    assert_eq!(lyrics_body.runs.len(), 1);
    let run_style = valid(parse_style_attribute(&lyrics_body.runs[0].attributes));
    assert_eq!(
        run_style.flags,
        RunStyleFlags {
            bold: true,
            ..RunStyleFlags::default()
        }
    );

    assert_eq!(project.title_page.len(), 3);
    assert!(title_by_foreign_id(&project, "open-title-direct").is_none());
    assert!(
        title_by_foreign_id(&project, "open-title-case").is_some_and(|title| title
            .element_type
            .as_ref()
            .map(|key| key.as_str())
            == Some("titlefirst"))
    );
    assert!(
        title_by_foreign_id(&project, "open-title-content").is_some_and(|title| title
            .element_type
            .as_ref()
            .map(|key| key.as_str())
            == Some("TitleContent"))
    );
    assert!(title_by_foreign_id(&project, "open-title-typeless")
        .is_some_and(|title| title.element_type.is_none()));
    assert!(title_by_foreign_id(&project, "open-title-opaque").is_none());

    let root = source_root(&project)
        .unwrap_or_else(|| unreachable!("the fixture retains its FinalDraft source root"));
    let settings = root
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "ElementSettings" => {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(settings.len(), 6);
    assert_eq!(attribute(&settings[0], "Type"), Some("Lyrics"));
    assert_eq!(attribute(&settings[1], "Type"), Some("Notations"));
    assert_eq!(attribute(&settings[2], "Type"), Some("action"));
    assert_eq!(attribute(&settings[3], "Type"), Some("Action"));
    assert_eq!(attribute(&settings[4], "Type"), Some("lyrics"));
    let retained_lyrics = child_element(&settings[0], "FontSpec")
        .unwrap_or_else(|| unreachable!("winning Lyrics settings retain FontSpec"));
    assert_eq!(attribute(retained_lyrics, "Color"), Some("#87878F8FE0E0"));
    let retained_near_black = child_element(&settings[1], "FontSpec")
        .unwrap_or_else(|| unreachable!("near-black custom FontSpec remains retained"));
    assert_eq!(
        attribute(retained_near_black, "Color"),
        Some("#000000000001")
    );
    assert_eq!(
        attribute(
            child_element(&settings[2], "FontSpec")
                .unwrap_or_else(|| unreachable!("winning lowercase action has FontSpec")),
            "Color"
        ),
        None
    );
    assert_eq!(
        attribute(
            child_element(&settings[3], "FontSpec")
                .unwrap_or_else(|| unreachable!("winning Action has FontSpec")),
            "Color"
        ),
        None
    );
    let inert_duplicate_font = child_element(&settings[5], "FontSpec")
        .unwrap_or_else(|| unreachable!("inert duplicate retains its FontSpec"));
    assert_eq!(
        attribute(inert_duplicate_font, "Font"),
        Some("Duplicate Must Not Win")
    );
    assert_eq!(
        attribute(inert_duplicate_font, "Color"),
        Some("#123456789ABC")
    );
    assert_eq!(
        project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source.original_bytes.as_slice()),
        Some(OPEN_FIXTURE)
    );

    let direct_title_source = child_element_with_attribute(
        child_element(root, "TitlePage")
            .unwrap_or_else(|| unreachable!("TitlePage source remains retained")),
        "Paragraph",
        "id",
        "open-title-direct",
    )
    .unwrap_or_else(|| unreachable!("direct title source remains preserved"));
    assert!(matches!(
        &direct_title_source.ownership,
        SourceOwnership::Opaque
    ));
    assert!(direct_title_source.anchor.is_none());
    assert_eq!(attribute(direct_title_source, "Type"), Some("TitleFirst"));
    assert_eq!(attribute(direct_title_source, "Alignment"), Some("Center"));
    assert_eq!(
        child_element(direct_title_source, "Text").map(direct_text),
        Some("First ".to_owned())
    );
    assert_eq!(
        child_element(direct_title_source, "Text").and_then(|text| attribute(text, "Style")),
        None
    );
    let direct_bold_text = direct_title_source
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
            _ => None,
        })
        .nth(1)
        .unwrap_or_else(|| unreachable!("direct title's second styled Text remains retained"));
    assert_eq!(attribute(direct_bold_text, "Style"), Some("Bold"));
    assert_eq!(
        child_element(direct_title_source, "VendorTitlePayload").map(direct_text),
        Some("title payload".to_owned())
    );
    let direct_attachment = child_element(direct_title_source, "UserParagraphData")
        .unwrap_or_else(|| unreachable!("direct title attachment remains retained"));
    assert!(matches!(
        &direct_attachment.ownership,
        SourceOwnership::Opaque
    ));

    let header_footer = child_element(root, "HeaderAndFooter")
        .unwrap_or_else(|| unreachable!("HeaderAndFooter data remains retained"));
    assert!(matches!(&header_footer.ownership, SourceOwnership::Opaque));
    for (container, paragraph_id, payload_name, expected_payload) in [
        (
            "Header",
            "open-header-title",
            "VendorHeaderPayload",
            "header payload",
        ),
        (
            "Footer",
            "open-footer-title",
            "VendorFooterPayload",
            "footer payload",
        ),
    ] {
        let region = child_element(header_footer, container)
            .unwrap_or_else(|| unreachable!("header/footer region remains retained"));
        assert!(matches!(&region.ownership, SourceOwnership::Opaque));
        let paragraph = child_element_with_attribute(region, "Paragraph", "id", paragraph_id)
            .unwrap_or_else(|| unreachable!("header/footer paragraph remains retained"));
        assert!(matches!(&paragraph.ownership, SourceOwnership::Opaque));
        assert!(paragraph.anchor.is_none());
        assert_eq!(
            child_element(paragraph, payload_name).map(direct_text),
            Some(expected_payload.to_owned())
        );
    }

    let title_page_source = child_element(root, "TitlePage")
        .unwrap_or_else(|| unreachable!("TitlePage source remains retained"));
    let title_content_source = child_element(title_page_source, "Content")
        .unwrap_or_else(|| unreachable!("TitlePage Content source remains retained"));
    let content_title_source = child_element_with_attribute(
        title_content_source,
        "Paragraph",
        "id",
        "open-title-content",
    )
    .unwrap_or_else(|| unreachable!("typed TitlePage Content source remains anchored"));
    let content_title = title_by_foreign_id(&project, "open-title-content")
        .unwrap_or_else(|| unreachable!("typed TitlePage Content paragraph is managed"));
    assert!(matches!(
        &content_title_source.ownership,
        SourceOwnership::Managed(owner) if owner == &content_title.id
    ));
    assert!(content_title_source.anchor.is_some());

    let wrapper = title_content_source
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Wrapper" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("unsupported typed title lookalike remains retained"));
    assert!(matches!(&wrapper.ownership, SourceOwnership::Opaque));
    assert!(
        child_element_with_attribute(wrapper, "Paragraph", "id", "open-title-opaque").is_some()
    );

    let body_source = child_element_with_attribute(
        child_element(root, "Content").unwrap_or_else(|| unreachable!("Content remains retained")),
        "Paragraph",
        "id",
        "open-body-general-derived",
    )
    .unwrap_or_else(|| unreachable!("arbitrary body source remains anchored"));
    assert!(matches!(
        &body_source.ownership,
        SourceOwnership::Managed(owner) if owner == &body.id
    ));
    assert!(body_source.anchor.is_some());
    assert_eq!(attribute(body_source, "vendor-owner"), Some("preserve"));
    assert!(matches!(
        child_element(body_source, "SceneProperties").map(|child| &child.ownership),
        Some(SourceOwnership::Opaque)
    ));
    assert_eq!(
        child_element(body_source, "VendorBodyPayload").map(direct_text),
        Some("body payload".to_owned())
    );
    assert!(child_element(body_source, "UserParagraphData").is_some());
    assert!(matches!(
        child_element_with_attribute(
            child_element(root, "Content").unwrap_or_else(|| unreachable!("Content retained")),
            "Paragraph",
            "id",
            "open-body-missing-type"
        )
        .map(|element| &element.ownership),
        Some(SourceOwnership::Opaque)
    ));
    assert!(matches!(
        child_element_with_attribute(
            child_element(root, "Content").unwrap_or_else(|| unreachable!("Content retained")),
            "Paragraph",
            "id",
            "open-body-empty-type"
        )
        .map(|element| &element.ownership),
        Some(SourceOwnership::Opaque)
    ));
}

#[test]
fn duplicate_diagnostics_are_capped_and_count_suppressed_entries() {
    let fixture = std::str::from_utf8(SMOKE_FIXTURE)
        .unwrap_or_else(|_| unreachable!("sanitized FDX fixture is UTF-8"));
    let mut duplicates = String::from("  <ElementSettings Type=\"Probe\" />\n");
    for _ in 0..40 {
        duplicates.push_str("  <ElementSettings Type=\"Probe\" />\n");
    }
    let source = fixture.replace("  <Content>", &format!("{duplicates}  <Content>"));
    assert_ne!(source, fixture);

    let imported = valid(read_v6(source.as_bytes()));
    assert_eq!(
        imported.import_diagnostics.entries.len(),
        MAX_FDX_IMPORT_DIAGNOSTICS
    );
    assert_eq!(
        imported.import_diagnostics.entries,
        (1..=MAX_FDX_IMPORT_DIAGNOSTICS)
            .map(
                |settings_ordinal| FdxImportDiagnostic::DuplicateElementSettings {
                    settings_ordinal
                }
            )
            .collect::<Vec<_>>()
    );
    assert_eq!(imported.import_diagnostics.suppressed_duplicate_count, 8);
    assert!(!format!("{:?}", imported.import_diagnostics).contains("Probe"));
}

#[test]
fn older_import_preserves_arbitrary_types_and_reports_only_older_losses() {
    let cancelled = valid(import_fdx(OLDER_FIXTURE, FdxMigrationConsent::Cancel));
    assert_eq!(
        cancelled,
        FdxImportOutcome::Cancelled {
            declared_version: "5".to_owned()
        }
    );

    let outcome = valid(import_fdx(OLDER_FIXTURE, FdxMigrationConsent::Continue));
    let FdxImportOutcome::Candidate(candidate) = outcome else {
        unreachable!("consented older import returns a staged candidate")
    };
    assert_eq!(
        candidate.migration,
        Some(FdxMigrationKind::OlderLossy {
            declared: "5".to_owned()
        })
    );
    let project = &candidate.document.project;
    assert!(body_by_foreign_id(project, "older-body-lyrics")
        .is_some_and(|element| element.element_type().as_str() == "Lyrics"));
    assert!(body_by_foreign_id(project, "older-body-case")
        .is_some_and(|element| element.element_type().as_str() == "lyrics"));
    assert!(body_by_foreign_id(project, "older-body-missing-type").is_none());
    assert!(project
        .element_definitions()
        .get_by_label("Lyrics")
        .is_some());
    assert!(project
        .element_definitions()
        .get_by_label("lyrics")
        .is_some());
    assert!(!project.title_page.iter().any(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            == Some("older-title-direct")
    }));
    assert!(project
        .element_definitions()
        .get_by_label("Notations")
        .is_none());
    assert!(project.title_page.iter().any(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            == Some("older-title-content")
            && paragraph.element_type.as_ref().map(|key| key.as_str()) == Some("Scene Citation")
    }));
    assert!(project.title_page.iter().any(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_id.as_deref())
            == Some("older-title-typeless")
            && paragraph.element_type.is_none()
    }));
    let root = source_root(project)
        .unwrap_or_else(|| unreachable!("prepared older import retains a FinalDraft root"));
    let title_page = child_element(root, "TitlePage")
        .unwrap_or_else(|| unreachable!("prepared older import retains TitlePage"));
    let direct_title_source =
        child_element_with_attribute(title_page, "Paragraph", "id", "older-title-direct")
            .unwrap_or_else(|| unreachable!("direct title source remains preserved"));
    assert!(matches!(
        &direct_title_source.ownership,
        SourceOwnership::Opaque
    ));
    assert!(direct_title_source.anchor.is_none());
    assert_eq!(attribute(direct_title_source, "Type"), Some("Notations"));
    assert_eq!(
        child_element(direct_title_source, "Text").map(direct_text),
        Some("Older typed title".to_owned())
    );
    assert_eq!(
        child_element(direct_title_source, "VendorTitlePayload")
            .and_then(|payload| attribute(payload, "marker")),
        Some("retained")
    );
    let content = child_element(root, "Content")
        .unwrap_or_else(|| unreachable!("prepared older import retains Content"));
    let missing_type_source =
        child_element_with_attribute(content, "Paragraph", "id", "older-body-missing-type")
            .unwrap_or_else(|| unreachable!("missing-Type paragraph remains retained"));
    assert_eq!(attribute(missing_type_source, "Type"), None);
    assert!(matches!(
        &missing_type_source.ownership,
        SourceOwnership::Opaque
    ));
    assert_eq!(
        child_element_with_attribute(content, "Paragraph", "id", "older-body-lyrics")
            .and_then(|paragraph| attribute(paragraph, "Type")),
        Some("Lyrics")
    );
    assert_eq!(
        child_element_with_attribute(content, "Paragraph", "id", "older-body-case")
            .and_then(|paragraph| attribute(paragraph, "Type")),
        Some("lyrics")
    );
}
