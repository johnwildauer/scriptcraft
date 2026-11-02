use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6, FdxError};
use scriptcraft_core::preservation::PreservedNode;
use scriptcraft_core::{Project, Role, ScreenplayElement};

fn imported_session(source: &[u8]) -> EditingSession {
    EditingSession::new(read_v6(source).unwrap_or_else(|_| unreachable!()).project)
        .unwrap_or_else(|_| unreachable!())
}

fn select(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection {
            anchor: Position { element, offset },
            focus: Position { element, offset },
        })
        .unwrap_or_else(|_| unreachable!());
}

fn serialize_and_reopen(session: &EditingSession) -> (Vec<u8>, Project) {
    let bytes = write_v6(session.project()).unwrap_or_else(|_| unreachable!());
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!())
        .project;
    (bytes, reopened)
}

fn content_children(project: &Project) -> &[PreservedNode] {
    project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!())
        .sections
        .iter()
        .flat_map(|section| &section.children)
        .find_map(|node| {
            let PreservedNode::Element(root) = node else {
                return None;
            };
            root.children.iter().find_map(|child| {
                let PreservedNode::Element(content) = child else {
                    return None;
                };
                (content.name.local_name == "Content").then_some(content.children.as_slice())
            })
        })
        .unwrap_or_else(|| unreachable!())
}

fn non_paragraph_siblings(project: &Project) -> Vec<PreservedNode> {
    content_children(project)
        .iter()
        .filter(|node| {
            !matches!(node, PreservedNode::Element(element) if element.name.local_name == "Paragraph")
        })
        .cloned()
        .collect()
}

#[test]
fn xml_space_preserve_allows_managed_insertion_and_split_without_sibling_reformatting() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content xml:space="preserve">
  <Paragraph Type="Action" ID="p1"><Text>alpha beta</Text></Paragraph>
	<Paragraph Type="Action" ID="p2"><Text>tail</Text></Paragraph>
</Content></FinalDraft>"#;
    let original = read_v6(source).unwrap_or_else(|_| unreachable!()).project;
    let expected_siblings = non_paragraph_siblings(&original);

    let mut split = imported_session(source);
    select(&mut split, 0, 5);
    assert_eq!(split.split(), Ok(()));
    let (split_bytes, split_reopened) = serialize_and_reopen(&split);
    let split_xml = std::str::from_utf8(&split_bytes).unwrap_or("");
    assert!(split_xml.contains("xml:space=\"preserve\""));
    assert_eq!(split_reopened.elements[0].text, "alpha");
    assert_eq!(split_reopened.elements[1].text, " beta");
    assert_eq!(non_paragraph_siblings(&split_reopened), expected_siblings);

    let mut insertion = imported_session(source);
    select(&mut insertion, 0, 5);
    assert_eq!(insertion.enter(), Ok(()));
    assert_eq!(insertion.insert("fresh"), Ok(()));
    let (insert_bytes, insert_reopened) = serialize_and_reopen(&insertion);
    let insert_xml = std::str::from_utf8(&insert_bytes).unwrap_or("");
    assert!(insert_xml.contains("xml:space=\"preserve\""));
    assert_eq!(insert_reopened.elements[0].text, "alpha");
    assert!(insert_reopened
        .elements
        .iter()
        .any(|element| element.text.contains("fresh")));
    assert_eq!(non_paragraph_siblings(&insert_reopened), expected_siblings);
}

#[test]
fn mixed_and_opaque_content_siblings_are_retained_around_a_split() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="p1"><Text>alpha beta</Text></Paragraph><!-- keep comment --><?vendor keep-pi?><![CDATA[keep\t
cdata]]>mixed text<VendorChild mark="keep">opaque</VendorChild> tail<Paragraph Type="Action" ID="p2"><Text>end</Text></Paragraph></Content></FinalDraft>"#;
    let mut session = imported_session(source);
    let original = read_v6(source).unwrap_or_else(|_| unreachable!()).project;
    let expected_siblings = non_paragraph_siblings(&original);
    select(&mut session, 0, 5);
    assert_eq!(session.split(), Ok(()));

    let (bytes, reopened) = serialize_and_reopen(&session);
    let xml = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(xml.contains("<!-- keep comment -->"));
    assert!(xml.contains("<?vendor keep-pi?>"));
    assert!(xml.contains("<![CDATA[keep\\t\ncdata]]>"));
    assert!(xml.contains("mixed text<VendorChild mark=\"keep\">opaque</VendorChild> tail"));
    assert_eq!(reopened.elements[0].text, "alpha");
    assert_eq!(reopened.elements[1].text, " beta");
    assert_eq!(non_paragraph_siblings(&reopened), expected_siblings);
}

#[test]
fn ordinary_paragraph_content_keeps_safe_readable_normalization() {
    let source = b"<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" ID=\"p1\"><Text>alpha beta</Text></Paragraph></Content></FinalDraft>";
    let mut session = imported_session(source);
    select(&mut session, 0, 5);
    assert_eq!(session.split(), Ok(()));
    let (bytes, reopened) = serialize_and_reopen(&session);
    let xml = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(xml.contains("</Paragraph>\n    <Paragraph Type=\"Action\""));
    assert_eq!(reopened.elements[0].text, "alpha");
    assert_eq!(reopened.elements[1].text, " beta");
}

#[test]
fn formatting_best_effort_does_not_weaken_source_reconciliation_rejection() {
    let source = b"<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" ID=\"p1\"><Text>alpha</Text></Paragraph></Content></FinalDraft>";
    let mut project = read_v6(source).unwrap_or_else(|_| unreachable!()).project;
    project.elements.push(
        ScreenplayElement::new("unanchored", Role::Action, "new")
            .unwrap_or_else(|_| unreachable!()),
    );
    assert_eq!(
        write_v6(&project),
        Err(FdxError::SaveDiagnosticUnsupportedConstruct(5))
    );
}
