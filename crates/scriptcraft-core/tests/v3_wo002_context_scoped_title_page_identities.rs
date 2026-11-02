use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::fdx::{read_v6, write_v6, FdxError};
use scriptcraft_core::preservation::{
    ForeignIdDomain, PreservedAttribute, PreservedElement, PreservedNode, PreservationState,
};
use scriptcraft_core::{Project, Role};

const REPEATED_TITLE_PAGE_IDS: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6">
<Content><Paragraph Type="Action" id="body-1"><Text>Canonical body.</Text></Paragraph></Content>
<TitlePage>
  <Paragraph Type="Action" id="managed-title-1"><Text>Managed title role</Text></Paragraph>
  <Content>
    <Paragraph Alignment="Left" Custom="first" id="opaque-repeat"><Text>First title line</Text></Paragraph>
    <Paragraph Alignment="Center" Custom="second" id="opaque-repeat"><Text Style="Bold">Second title line</Text></Paragraph>
    <Paragraph Alignment="Right" Custom="unique" id="opaque-unique"><Text>Distinct opaque title line</Text></Paragraph>
  </Content>
</TitlePage>
</FinalDraft>"#;

fn imported(source: &[u8]) -> Project {
    read_v6(source)
        .unwrap_or_else(|error| panic!("fixture imports: {error:?}"))
        .project
}

fn element_text(element: &PreservedElement) -> String {
    fn visit(nodes: &[PreservedNode], output: &mut String) {
        for node in nodes {
            match node {
                PreservedNode::Element(element) => visit(&element.children, output),
                PreservedNode::Text(text) | PreservedNode::CData(text) => output.push_str(text),
                PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {}
            }
        }
    }

    let mut output = String::new();
    visit(&element.children, &mut output);
    output
}

fn title_page_paragraphs(state: &PreservationState) -> Vec<(Vec<PreservedAttribute>, String)> {
    fn visit(
        nodes: &[PreservedNode],
        in_title_page: bool,
        output: &mut Vec<(Vec<PreservedAttribute>, String)>,
    ) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            let in_title_page = in_title_page || element.name.local_name == "TitlePage";
            if in_title_page && element.name.local_name == "Paragraph" {
                output.push((element.attributes.clone(), element_text(element)));
            }
            visit(&element.children, in_title_page, output);
        }
    }

    let mut output = Vec::new();
    for section in &state.sections {
        visit(&section.children, false, &mut output);
    }
    output
}

fn attribute<'a>(attributes: &'a [PreservedAttribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case(name)
        })
        .map(|attribute| attribute.value.as_str())
}

fn paragraph_ids(project: &Project) -> Vec<String> {
    project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("imported project has preservation state"))
        .identities
        .iter()
        .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
        .map(|identity| identity.value.clone())
        .collect()
}

fn root_element(state: &PreservationState) -> &PreservedElement {
    state
        .sections
        .iter()
        .flat_map(|section| &section.children)
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("preserved document root exists"))
}

fn opaque_top_level_sections(state: &PreservationState) -> Vec<PreservedElement> {
    root_element(state)
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element)
                if !matches!(
                    element.name.local_name.as_str(),
                    "Content" | "TitlePage" | "ElementSettings"
                ) =>
            {
                Some(element.clone())
            }
            _ => None,
        })
        .collect()
}

fn canonical_body(project: &Project) -> Vec<(Role, String)> {
    project
        .elements
        .iter()
        .map(|element| (element.role, element.text.clone()))
        .collect()
}

fn arc_names(project: &Project) -> Vec<String> {
    fn visit(nodes: &[PreservedNode], names: &mut Vec<String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "CharacterArcBeat" {
                if let Some(name) = attribute(&element.attributes, "Name") {
                    names.push(name.to_owned());
                }
            }
            visit(&element.children, names);
        }
    }

    let mut names = Vec::new();
    if let Some(state) = &project.preservation {
        for section in &state.sections {
            visit(&section.children, &mut names);
        }
    }
    names
}

fn arc_ids_by_name(project: &Project) -> Vec<(String, String)> {
    fn visit(nodes: &[PreservedNode], arcs: &mut Vec<(String, String)>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "CharacterArcBeat" {
                let name = attribute(&element.attributes, "Name").map(str::to_owned);
                let id = element.children.iter().find_map(|child| match child {
                    PreservedNode::Element(paragraph)
                        if paragraph.name.local_name == "Paragraph" =>
                    {
                        attribute(&paragraph.attributes, "id").map(str::to_owned)
                    }
                    _ => None,
                });
                if let (Some(name), Some(id)) = (name, id) {
                    arcs.push((name, id));
                }
            }
            visit(&element.children, arcs);
        }
    }

    let mut arcs = Vec::new();
    if let Some(state) = &project.preservation {
        for section in &state.sections {
            visit(&section.children, &mut arcs);
        }
    }
    arcs
}

#[test]
fn repeated_opaque_title_page_ids_round_trip_as_ordered_source_occurrences() {
    let project = imported(REPEATED_TITLE_PAGE_IDS);
    let state = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("import has preservation state"));
    let before = title_page_paragraphs(state);
    assert_eq!(before.len(), 4);
    assert_eq!(
        before.iter().map(|(_, text)| text.as_str()).collect::<Vec<_>>(),
        [
            "Managed title role",
            "First title line",
            "Second title line",
            "Distinct opaque title line"
        ]
    );
    assert_eq!(attribute(&before[1].0, "id"), Some("opaque-repeat"));
    assert_eq!(attribute(&before[2].0, "id"), Some("opaque-repeat"));
    assert_eq!(attribute(&before[1].0, "Custom"), Some("first"));
    assert_eq!(attribute(&before[2].0, "Alignment"), Some("Center"));
    assert_eq!(attribute(&before[3].0, "id"), Some("opaque-unique"));

    let ids = paragraph_ids(&project);
    assert!(ids.iter().any(|id| id == "body-1"));
    assert!(ids.iter().any(|id| id == "managed-title-1"));
    // A distinct opaque title-page ID keeps its prior global registration;
    // only the repeated opaque occurrences are excluded from that domain.
    assert!(ids.iter().any(|id| id == "opaque-unique"));
    assert!(!ids.iter().any(|id| id == "opaque-repeat"));

    let saved = write_v6(&project).unwrap_or_else(|error| panic!("candidate serializes: {error:?}"));
    let reopened = imported(&saved);
    let reopened_state = reopened
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("reopened project has preservation state"));
    assert_eq!(title_page_paragraphs(reopened_state), before);
    assert_eq!(paragraph_ids(&reopened), ids);
    assert_eq!(canonical_body(&reopened), canonical_body(&project));
}

#[test]
fn real_reference_round_trips_title_content_and_opaque_fdx_sections() {
    const REAL_REFERENCE: &[u8] = include_bytes!("../../../.reference/real_script_example.fdx");

    let original = imported(REAL_REFERENCE);
    let original_state = original
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("reference import has preservation state"));
    let before_title = title_page_paragraphs(original_state);
    let repeated_id = "ae7f1040-c536-4085-aeec-464de6be3eac";
    assert!(before_title.iter().filter(|(attrs, _)| attribute(attrs, "id") == Some(repeated_id)).count() > 1);
    let original_board_graph = original_state.board_graph.clone();
    let original_references = original_state.references.clone();
    let original_opaque_sections = opaque_top_level_sections(original_state);
    let original_body = canonical_body(&original);
    let original_title_roles = original
        .title_page
        .iter()
        .map(|element| (element.role, element.text.clone()))
        .collect::<Vec<_>>();

    let saved = write_v6(&original).unwrap_or_else(|error| panic!("reference candidate serializes: {error:?}"));
    let reopened = imported(&saved);
    let reopened_state = reopened
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("reopened reference has preservation state"));
    assert_eq!(canonical_body(&reopened), original_body);
    assert_eq!(
        reopened
            .title_page
            .iter()
            .map(|element| (element.role, element.text.clone()))
            .collect::<Vec<_>>(),
        original_title_roles
    );
    assert_eq!(title_page_paragraphs(reopened_state), before_title);
    assert_eq!(reopened_state.board_graph, original_board_graph);
    assert_eq!(reopened_state.references, original_references);
    assert_eq!(opaque_top_level_sections(reopened_state), original_opaque_sections);

    let original_helen_arc = arc_ids_by_name(&reopened)
        .into_iter()
        .find(|(name, _)| name == "HELEN MIRREN")
        .unwrap_or_else(|| unreachable!("reference contains Helen Mirren's derived arc"));
    let mut derived = reopened;
    correct_cast_lists(&mut derived).unwrap_or_else(|error| panic!("derived projection is bounded: {error:?}"));
    assert!(derived
        .elements
        .iter()
        .any(|element| element.role == Role::Character && element.text == "HELEN MIRREN (V.O.)"));
    assert!(arc_names(&derived).iter().any(|name| name == "HELEN MIRREN"));
    assert!(arc_ids_by_name(&derived).contains(&original_helen_arc));
}

#[test]
fn duplicate_managed_and_referenced_identities_remain_ambiguous() {
    let duplicate_managed_body = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" id="managed-duplicate"><Text>one</Text></Paragraph>
<Paragraph Type="Dialogue" id="managed-duplicate"><Text>two</Text></Paragraph>
</Content></FinalDraft>"#;
    assert_eq!(
        read_v6(duplicate_managed_body),
        Err(FdxError::AmbiguousIdentity)
    );

    let duplicate_referenced_list_item = br#"<FinalDraft DocumentType="Script" Version="6"><Content/>
<ListItems><ListItem ID="referenced-duplicate"/><ListItem ID="referenced-duplicate"/></ListItems>
<DisplayBoards><DisplayBoard ID="board-1"><Item id="referenced-duplicate"/></DisplayBoard></DisplayBoards>
</FinalDraft>"#;
    assert_eq!(
        read_v6(duplicate_referenced_list_item),
        Err(FdxError::AmbiguousIdentity)
    );
}

#[test]
fn opaque_repeated_id_cannot_shadow_a_globally_registered_paragraph() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6">
<Content><Paragraph Type="Action" id="shared"><Text>managed</Text></Paragraph></Content>
<TitlePage><Content><Paragraph id="shared"><Text>opaque title content</Text></Paragraph>
<Paragraph id="shared"><Text>repeated opaque title content</Text></Paragraph></Content></TitlePage>
</FinalDraft>"#;
    assert_eq!(read_v6(source), Err(FdxError::AmbiguousIdentity));
}
