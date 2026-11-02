use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::ForeignIdDomain;
use scriptcraft_core::{Project, Role, ScreenplayElement};

fn is_canonical_uuid_v4(value: &str) -> bool {
    let Ok(parsed) = uuid::Uuid::parse_str(value) else {
        return false;
    };
    parsed.get_version() == Some(uuid::Version::Random)
        && parsed.get_variant() == uuid::Variant::RFC4122
        && value == parsed.hyphenated().to_string()
}

fn paragraph_ids(project: &Project) -> Vec<String> {
    project
        .preservation
        .as_ref()
        .map(|state| {
            state
                .identities
                .iter()
                .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
                .map(|identity| identity.value.clone())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn new_body_title_and_scene_arc_paragraphs_receive_distinct_uuid_v4_ids() {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    project.elements = vec![
        ScreenplayElement::new("heading-semantic", Role::SceneHeading, "INT. ROOM")
            .unwrap_or_else(|_| unreachable!()),
        ScreenplayElement::new("cue-semantic", Role::Character, "jOhN")
            .unwrap_or_else(|_| unreachable!()),
    ];
    project.title_page.push(
        ScreenplayElement::new("title-semantic", Role::Action, "Title")
            .unwrap_or_else(|_| unreachable!()),
    );

    let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
    let reread = read_v6(&bytes).unwrap_or_else(|_| unreachable!()).project;
    let ids = paragraph_ids(&reread);

    assert_eq!(ids.len(), 4);
    assert!(ids.iter().all(|id| is_canonical_uuid_v4(id)));
    assert_eq!(ids.iter().collect::<std::collections::BTreeSet<_>>().len(), ids.len());
}

#[test]
fn source_backed_enter_and_split_allocate_uuid_ids_without_migrating_source_ids() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" id="legacy-action"><Text>alpha beta</Text></Paragraph>
<Paragraph Type="Custom" id="scriptcraft-paragraph-1"><Text>opaque source</Text></Paragraph>
</Content></FinalDraft>"#;
    let project = read_v6(source).unwrap_or_else(|_| unreachable!()).project;
    let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());

    session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: 5,
        }))
        .unwrap_or_else(|_| unreachable!());
    session.split().unwrap_or_else(|_| unreachable!());
    session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!());
    session.enter().unwrap_or_else(|_| unreachable!());

    let ids = paragraph_ids(session.project());
    let generated = ids
        .iter()
        .filter(|id| is_canonical_uuid_v4(id))
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 2);
    assert_eq!(generated.iter().collect::<std::collections::BTreeSet<_>>().len(), 2);
    assert!(ids.contains(&"legacy-action".to_owned()));
    assert!(ids.contains(&"scriptcraft-paragraph-1".to_owned()));

    let bytes = write_v6(session.project()).unwrap_or_else(|_| unreachable!());
    let output = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(output.contains("id=\"legacy-action\""));
    assert!(output.contains("id=\"scriptcraft-paragraph-1\""));
    assert!(generated.iter().all(|id| output.contains(id.as_str())));
}

#[test]
fn imported_scene_arc_allocation_preserves_legacy_paragraph_ids() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" id="legacy-heading"><Text>INT. ROOM</Text></Paragraph>
<Paragraph Type="Character" id="legacy-character"><Text>ALICE</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut project = read_v6(source).unwrap_or_else(|_| unreachable!()).project;
    assert!(correct_cast_lists(&mut project).unwrap_or(false));

    let ids = paragraph_ids(&project);
    let generated = ids
        .iter()
        .filter(|id| is_canonical_uuid_v4(id))
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 1);
    assert!(ids.contains(&"legacy-heading".to_owned()));
    assert!(ids.contains(&"legacy-character".to_owned()));

    let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
    let reread_ids = paragraph_ids(&read_v6(&bytes).unwrap_or_else(|_| unreachable!()).project);
    assert!(generated.iter().all(|id| reread_ids.contains(id)));
    assert!(reread_ids.contains(&"legacy-heading".to_owned()));
    assert!(reread_ids.contains(&"legacy-character".to_owned()));
}
