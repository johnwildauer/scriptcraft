use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6, FdxError};
use scriptcraft_core::{Project, Role, ScreenplayElement};

#[test]
fn generated_tree_uses_reference_structure_without_rewriting_canonical_text() {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    let canonical_heading = "INT. ROOM\t\n\r";
    project.elements = [
        ("heading", Role::SceneHeading, canonical_heading),
        ("character", Role::Character, "Alice"),
        ("whitespace", Role::Action, "\t\n\r "),
    ]
    .into_iter()
    .map(|(id, role, text)| ScreenplayElement::new(id, role, text).unwrap_or_else(|_| unreachable!()))
    .collect();

    let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
    let output = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(output.starts_with(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n<FinalDraft"
    ));
    assert!(output.contains("<Content>\n    <Paragraph Type=\"Scene Heading\""));
    assert!(output.contains("</SceneProperties>\n      <Text>INT. ROOM\t\n&#xD;</Text>"));
    assert!(output.contains("</SceneProperties>\n      <Text>INT. ROOM\t\n&#xD;</Text>\n    </Paragraph>\n    <Paragraph Type=\"Character\""));
    assert!(output.contains("</Content>\n\n  <ElementSettings Type=\"General\""));
    assert!(output.contains("</ElementSettings>\n\n  <ElementSettings Type=\"Scene Heading\""));
    assert!(output.contains("<SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\">\n        <SceneArcBeats>"));
    assert!(output.contains("\n          <CharacterArcBeat"));
    assert!(output.contains("</FinalDraft>\n"));

    let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
    assert_eq!(reopened.project.elements[0].text, canonical_heading);
    assert_eq!(reopened.project.elements[2].text, "\t\n\r ");
}

#[test]
fn split_paragraph_gets_sibling_whitespace_and_survives_reopen_save() {
    let source = b"<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" ID=\"p1\"><Text>alpha\t\n&#xD;</Text></Paragraph></Content><Vendor><!--keep--><?vendor keep?><Payload><![CDATA[keep\t\n]]></Payload></Vendor></FinalDraft>";
    let project = read_v6(source).unwrap_or_else(|_| unreachable!()).project;
    let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
    session
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: 5,
            },
            focus: Position {
                element: 0,
                offset: 5,
            },
        })
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(session.split(), Ok(()));

    let first_save = write_v6(session.project()).unwrap_or_else(|_| unreachable!());
    let first_xml = std::str::from_utf8(&first_save).unwrap_or("");
    assert!(first_xml.contains("</Paragraph>\n    <Paragraph Type=\"Action\""));
    assert!(first_xml.contains("<!--keep--><?vendor keep?><Payload><![CDATA[keep\t\n]]></Payload>"));

    let reopened = read_v6(&first_save).unwrap_or_else(|_| unreachable!());
    assert_eq!(reopened.project.elements[0].text, "alpha");
    assert_eq!(reopened.project.elements[1].text, "\t\n\r");
    let second_save = write_v6(&reopened.project).unwrap_or_else(|_| unreachable!());
    let second_xml = std::str::from_utf8(&second_save).unwrap_or("");
    assert!(second_xml.contains("</Paragraph>\n    <Paragraph Type=\"Action\""));
    let reopened_again = read_v6(&second_save).unwrap_or_else(|_| unreachable!());
    assert_eq!(reopened_again.project.elements[0].text, "alpha");
    assert_eq!(reopened_again.project.elements[1].text, "\t\n\r");
}

#[test]
fn xml_space_preserve_and_mixed_content_are_not_prettified() {
    let source = b"<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content xml:space=\"preserve\"><Paragraph Type=\"Action\" ID=\"p1\"><Text>alpha beta</Text></Paragraph></Content><Vendor>opaque <Emphasis>mixed</Emphasis> content</Vendor></FinalDraft>";
    let mut session = EditingSession::new(
        read_v6(source)
            .unwrap_or_else(|_| unreachable!())
            .project,
    )
    .unwrap_or_else(|_| unreachable!());
    session
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: 5,
            },
            focus: Position {
                element: 0,
                offset: 5,
            },
        })
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(session.split(), Ok(()));

    assert_eq!(write_v6(session.project()), Err(FdxError::UnsupportedConstruct));
    assert_eq!(session.project().elements[0].text, "alpha");
    assert_eq!(session.project().elements[1].text, " beta");
    assert_eq!(
        session
            .project()
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .source
            .original_bytes,
        source
    );

    let mixed_source = b"<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content>opaque <Paragraph Type=\"Action\" ID=\"p1\"><Text>alpha beta</Text></Paragraph> tail</Content></FinalDraft>";
    let mut mixed_session = EditingSession::new(
        read_v6(mixed_source)
            .unwrap_or_else(|_| unreachable!())
            .project,
    )
    .unwrap_or_else(|_| unreachable!());
    mixed_session
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: 5,
            },
            focus: Position {
                element: 0,
                offset: 5,
            },
        })
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(mixed_session.split(), Ok(()));
    assert_eq!(
        write_v6(mixed_session.project()),
        Err(FdxError::UnsupportedConstruct)
    );
}
