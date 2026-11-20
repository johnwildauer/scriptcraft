#![forbid(unsafe_code)]

use scriptcraft_fdx::{
    builtin_template_metadata, builtin_templates, instantiate_builtin_template, read_v6, write_v6,
    BuiltinTemplateId, DEFAULT_BUILTIN_TEMPLATE_ID,
};
use scriptcraft_model::SourceSection;
use std::sync::Arc;

#[track_caller]
fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("built-in template regression input failed: {error:?}"))
}

fn text_sequence(project: &scriptcraft_model::Project) -> Vec<(String, String)> {
    project
        .elements
        .iter()
        .map(|paragraph| {
            (
                paragraph.element_type().as_str().to_owned(),
                paragraph.text.clone(),
            )
        })
        .collect()
}

fn definition_sequence(project: &scriptcraft_model::Project) -> Vec<String> {
    project
        .element_definitions()
        .iter()
        .map(|definition| definition.key.as_str().to_owned())
        .collect()
}

fn source_content_paragraph_count(source: &str, within_title_page: bool) -> usize {
    let content = if within_title_page {
        source
            .split_once("<TitlePage>")
            .and_then(|(_, title)| title.split_once("</TitlePage>"))
            .and_then(|(title, _)| title.split_once("<Content>"))
            .and_then(|(_, content)| content.split_once("</Content>"))
            .map(|(content, _)| content)
            .unwrap_or_else(|| unreachable!("built-in title page has a Content section"))
    } else {
        source
            .split_once("<Content>")
            .and_then(|(_, content)| content.split_once("</Content>"))
            .map(|(content, _)| content)
            .unwrap_or_else(|| unreachable!("built-in document has a body Content section"))
    };
    if within_title_page {
        content.matches("<Paragraph ").count()
    } else {
        // Scene-property metadata may contain nested layout Paragraph nodes;
        // only typed direct body paragraphs are managed document elements.
        content.matches("<Paragraph Type=").count()
    }
}

fn title_has_style(paragraph: &scriptcraft_model::TitlePageParagraph, style: &str) -> bool {
    paragraph.runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value.split('+').any(|token| token == style)
        })
    })
}

fn assert_all_direct_content_is_managed(project: &scriptcraft_model::Project, source: &str) {
    assert_eq!(
        project.elements.len(),
        source_content_paragraph_count(source, false),
        "every direct body Content paragraph has one managed paragraph"
    );
    assert_eq!(
        project.title_page.len(),
        source_content_paragraph_count(source, true),
        "every direct TitlePage Content paragraph has one managed title"
    );
    assert!(project.elements.iter().all(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .is_some_and(|source| source.section == SourceSection::Content)
    }));
    assert!(project.title_page.iter().all(|paragraph| {
        paragraph
            .source_provenance
            .as_ref()
            .is_some_and(|source| source.section == SourceSection::TitlePage)
    }));
}

#[test]
fn public_catalog_is_exact_and_each_call_is_a_fresh_full_source_backed_document() {
    let catalog = builtin_templates();
    assert_eq!(
        catalog
            .iter()
            .map(|entry| (entry.id, entry.display_name, entry.description, entry.order))
            .collect::<Vec<_>>(),
        vec![
            (
                BuiltinTemplateId::Screenplay,
                "Screenplay",
                "The industry standard.",
                0
            ),
            (
                BuiltinTemplateId::Stageplay,
                "Stageplay",
                "An alternative for modern plays with looser dialogue margins in Times New Roman.",
                1,
            ),
            (
                BuiltinTemplateId::DramatistsGuildMusical,
                "Dramatists Guild Musical",
                "The Guild's modern format for musicals in Times New Roman.",
                2,
            ),
        ]
    );
    assert_eq!(DEFAULT_BUILTIN_TEMPLATE_ID, BuiltinTemplateId::Screenplay);

    for metadata in catalog {
        assert_eq!(builtin_template_metadata(metadata.id), metadata);
        let first = valid(instantiate_builtin_template(metadata.id));
        let second = valid(instantiate_builtin_template(metadata.id));
        let source = std::str::from_utf8(
            &first
                .project
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!("common reader retains original source"))
                .source
                .original_bytes,
        )
        .unwrap_or_else(|_| unreachable!("embedded FDXT source is UTF-8"));

        assert!(!Arc::ptr_eq(
            first.project.layout_identity(),
            second.project.layout_identity()
        ));
        assert_all_direct_content_is_managed(&first.project, source);
        assert!(first.project.preservation.is_some());
        assert!(source.contains("Template=\"Yes\""));
        assert!(source.contains("<Preview"));
        assert!(source.contains("<ScriptNotes"));
        assert!(source.contains("<HeaderAndFooter"));
        assert_eq!(
            first.project.element_definitions(),
            second.project.element_definitions()
        );
        assert_eq!(
            text_sequence(&first.project),
            text_sequence(&second.project)
        );
        assert_eq!(first.project.title_page, second.project.title_page);
    }
}

#[test]
fn complete_body_sequences_and_template_specific_typography_are_retained() {
    let common_definitions = [
        "General",
        "Scene Heading",
        "Action",
        "Character",
        "Parenthetical",
        "Dialogue",
        "Transition",
        "Shot",
        "Cast List",
        "New Act",
        "Sequence",
        "End of Act",
        "Summary",
        "Outline 1",
        "Outline 2",
        "Outline 3",
        "Note",
    ];
    let screenplay = valid(instantiate_builtin_template(BuiltinTemplateId::Screenplay));
    assert_eq!(
        definition_sequence(&screenplay.project),
        common_definitions.map(str::to_owned)
    );
    assert_eq!(
        text_sequence(&screenplay.project),
        vec![("Scene Heading".to_owned(), String::new())]
    );
    assert_eq!(screenplay.project.title_page.len(), 54);
    assert!(screenplay
        .project
        .title_page
        .iter()
        .any(|paragraph| paragraph.text == "Script Title"));
    assert!(screenplay
        .project
        .title_page
        .iter()
        .any(|paragraph| paragraph.text == "Name of First Writer"));
    assert!(screenplay
        .project
        .title_page
        .iter()
        .any(|paragraph| paragraph.text.is_empty()));
    let screenplay_title = screenplay
        .project
        .title_page
        .iter()
        .find(|paragraph| paragraph.text == "Script Title")
        .unwrap_or_else(|| unreachable!("Screenplay title placeholder remains editable"));
    assert_eq!(
        screenplay_title.formatting.alignment,
        Some(scriptcraft_model::TitlePageAlignment::Center)
    );
    assert!(title_has_style(screenplay_title, "Underline"));
    assert!(title_has_style(screenplay_title, "AllCaps"));

    let stageplay = valid(instantiate_builtin_template(BuiltinTemplateId::Stageplay));
    let mut expected_stageplay_definitions = common_definitions.map(str::to_owned).to_vec();
    expected_stageplay_definitions.push("Notations".to_owned());
    assert_eq!(
        definition_sequence(&stageplay.project),
        expected_stageplay_definitions
    );
    let expected_stageplay = [
        ("Scene Heading", "SCENE 1"),
        ("Notations", "DR. JUDITH BROWNE’s consulting room in a New York Health Center. There is a main door to the reception area of the practice and a back door. It is a post-war building, with Victorian trimmings around the main door and the frames. This play goes further in describing the setting."),
        ("Notations", "This format uses Times New Roman font."),
        ("Notations", "Notice that the opening description uses the Notations element in this form."),
        ("Notations", "Stage action not reliant on dialogue, however, uses an Action element and is formatted as below."),
        ("Notations", "This is one type of format for dramatic stage play. You can modify the elements any way you want by going to Format > Elements."),
        ("Action", "A SILENCE."), ("Character", "JUDITH"), ("Dialogue", "I have something to tell you."),
        ("Character", "ROBERT"), ("Dialogue", "But..."), ("Character", "JUDITH"), ("Dialogue", "I love you."),
        ("Action", "ROBERT takes a breath to speak but JUDITH stops him."), ("Character", "ROBERT"),
        ("Dialogue", "Judith."), ("Character", "JUDITH"), ("Dialogue", "Oh, what you must think?"),
        ("Character", "ROBERT"), ("Dialogue", "Come on, now."), ("Character", "JUDITH"),
        ("Dialogue", "It’s true. I can’t help it. I know you should never become involved with a patient."),
        ("Character", "ROBERT"), ("Dialogue", "Judith, I love you too."),
        ("Action", "She looks at him her mouth open a little. For a moment she is lost."),
        ("Action", "A SILENCE."),
        ("Action", "Then she allows ROBERT to help her back onto the chair. She smoothes her clothes down. ROBERT returns to his chair."),
        ("Character", "JUDITH"), ("Dialogue", "I-I can’t believe it."), ("Character", "ROBERT"),
        ("Dialogue", "It’s true."), ("Action", "The lost look is on JUDITH’s face again. ROBERT speaks very gently."),
        ("Dialogue", "You heard what you wanted to hear."), ("Action", "A PAUSE."),
        ("Action", "And JUDITH is near tears."), ("Transition", "FADE TO BLACK."),
    ];
    assert_eq!(
        text_sequence(&stageplay.project),
        expected_stageplay
            .iter()
            .map(|(kind, text)| ((*kind).to_owned(), (*text).to_owned()))
            .collect::<Vec<_>>()
    );
    assert_eq!(stageplay.project.element_definitions().len(), 18);
    assert!(stageplay
        .project
        .element_definitions()
        .get_by_label("Notations")
        .is_some());
    assert_eq!(
        stageplay
            .project
            .element_definitions()
            .get_by_label("Action")
            .map(|definition| definition.font.family.as_str()),
        Some("Times New Roman")
    );
    assert_eq!(
        stageplay
            .project
            .element_definitions()
            .get_by_label("Scene Heading")
            .map(|definition| definition.paragraph.alignment),
        Some(scriptcraft_model::formatting::ParagraphAlignment::Center)
    );
    assert!(stageplay.project.title_page.iter().any(|paragraph| {
        paragraph.text == "(Name of Project)"
            && paragraph.formatting.alignment == Some(scriptcraft_model::TitlePageAlignment::Center)
    }));

    let musical = valid(instantiate_builtin_template(
        BuiltinTemplateId::DramatistsGuildMusical,
    ));
    let mut expected_musical_definitions = common_definitions.map(str::to_owned).to_vec();
    expected_musical_definitions.extend([
        "Lyrics".to_owned(),
        "Scene Citation".to_owned(),
        "Notations".to_owned(),
    ]);
    assert_eq!(
        definition_sequence(&musical.project),
        expected_musical_definitions
    );
    let musical_body = text_sequence(&musical.project);
    let expected_musical_kinds = [
        "Scene Citation",
        "Notations",
        "Notations",
        "Notations",
        "Notations",
        "Notations",
        "Character",
        "Parenthetical",
        "Dialogue",
        "Action",
        "Character",
        "Parenthetical",
        "Dialogue",
        "Action",
        "Lyrics",
        "Lyrics",
        "Character",
        "Dialogue",
        "Character",
        "Dialogue",
        "General",
        "General",
        "General",
    ];
    assert_eq!(musical_body.len(), expected_musical_kinds.len());
    assert_eq!(
        musical_body
            .iter()
            .map(|(kind, _)| kind.as_str())
            .collect::<Vec<_>>(),
        expected_musical_kinds
    );
    assert!(musical_body[14]
        .1
        .contains("Lyrics are in all caps\nUse Shift + Enter/return to\nseparate lines"));
    assert!(musical_body[15].1.contains("Stanzas are separated by a blank line\nuse shift + enter/return twice to insert a new stanza"));
    assert!(musical_body[20]
        .1
        .contains("The Dramatists Guild of America"));
    assert!(musical_body[20].1.contains("www.dramatistsguild.com"));
    let guild_paragraph = musical.project.elements[20].as_ref();
    assert!(guild_paragraph.runs.iter().any(|run| {
        run.text.contains("The Dramatists Guild of America")
            && run
                .attributes
                .iter()
                .any(|attribute| attribute.name == "Style" && attribute.value.contains("Bold"))
    }));
    assert!(guild_paragraph.runs.iter().any(|run| {
        run.text.contains("was established over 80 years ago")
            && run
                .attributes
                .iter()
                .any(|attribute| attribute.name == "Style" && attribute.value.contains("Italic"))
    }));
    assert_eq!(musical.project.element_definitions().len(), 20);
    for key in ["Lyrics", "Scene Citation", "Notations"] {
        assert!(musical
            .project
            .element_definitions()
            .get_by_label(key)
            .is_some());
    }
    assert_eq!(
        musical
            .project
            .element_definitions()
            .get_by_label("Action")
            .map(|definition| definition.font.family.as_str()),
        Some("Times New Roman")
    );
    assert_eq!(
        musical
            .project
            .element_definitions()
            .get_by_label("Scene Citation")
            .and_then(|definition| definition.behavior.return_key.as_ref())
            .map(|key| key.as_str()),
        Some("Notations")
    );
    assert!(musical.project.title_page.iter().any(|paragraph| {
        paragraph.text == "Name of Musical"
            && paragraph.formatting.alignment == Some(scriptcraft_model::TitlePageAlignment::Center)
    }));
}

#[test]
fn ordinary_writer_removes_only_output_template_marker_and_preserves_opaque_title_structures() {
    for id in [
        BuiltinTemplateId::Screenplay,
        BuiltinTemplateId::Stageplay,
        BuiltinTemplateId::DramatistsGuildMusical,
    ] {
        let document = valid(instantiate_builtin_template(id));
        let original_bytes = document
            .project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("template import retains source"))
            .source
            .original_bytes
            .clone();
        let output = valid(write_v6(&document.project));
        let output_source =
            std::str::from_utf8(&output).unwrap_or_else(|_| unreachable!("writer output is UTF-8"));
        assert!(!output_source.contains("Template=\"Yes\""));
        assert!(!output_source.contains("Template=\"No\""));
        assert!(output_source.contains("<HeaderAndFooter"));
        assert!(output_source.contains("<ScriptNotes"));
        assert_eq!(
            document
                .project
                .preservation
                .as_ref()
                .map(|state| state.source.original_bytes.as_slice()),
            Some(original_bytes.as_slice()),
            "writing cannot mutate the original embedded source snapshot"
        );
        let reopened = valid(read_v6(&output));
        assert_eq!(reopened.project.elements, document.project.elements);
        assert_eq!(reopened.project.title_page, document.project.title_page);
        assert_eq!(
            reopened.project.element_definitions(),
            document.project.element_definitions()
        );
    }
}
