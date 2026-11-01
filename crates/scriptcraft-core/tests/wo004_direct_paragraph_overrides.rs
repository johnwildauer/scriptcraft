use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::formatting::SourceParagraphOverrides;
use scriptcraft_core::layout::{Caret, LayoutCache, LayoutPurpose, ScreenplayLayout};
use scriptcraft_core::preservation::PreservationError;
use scriptcraft_core::{FixedPoint, LayoutInputs, ParagraphAlignment, Role};

const PRECEDENCE_SOURCE: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6">
<ElementSettings Type="Summary"><ParagraphSpec Type="Summary" Alignment="Center" FirstIndent="0.05" Leading="Regular" LeftIndent="0.88" RightIndent="7.50" SpaceBefore="12" Spacing="1" StartsNewPage="No"/></ElementSettings>
<Content>
<Paragraph Type="Action" id="before"><Text>before</Text></Paragraph>
<Paragraph Alignment="Right" FirstIndent="-0.10" Leading="Regular" LeftIndent="1.12" RightIndent="7.25" SpaceBefore="18" Spacing="1" StartsNewPage="Yes" Type="Summary" id="direct-one" Keep="unchanged"><Text>one</Text></Paragraph>
<Paragraph Alignment="Bogus" FirstIndent="bad" Leading="Expanded" LeftIndent="bad" RightIndent="bad" SpaceBefore="-1" Spacing="2" StartsNewPage="Maybe" Type="Summary" id="direct-two"><Text>two</Text></Paragraph>
<Paragraph SpaceBefore="20" Type="Summary" id="direct-three"><Text>three</Text></Paragraph>
<Paragraph StartsNewPage="No" Type="New Act" id="direct-no-start"><Text>ACT TWO</Text></Paragraph>
</Content></FinalDraft>"#;

fn imported(source: &[u8]) -> scriptcraft_core::Project {
    read_v6(source)
        .unwrap_or_else(|_| unreachable!("bounded direct-format fixture must import"))
        .project
}

fn layout(project: &scriptcraft_core::Project) -> ScreenplayLayout {
    ScreenplayLayout::build_with_inputs_for_purpose(
        project,
        &LayoutInputs::default(),
        LayoutPurpose::Editor,
    )
    .unwrap_or_else(|_| unreachable!("bounded fixture must have layout"))
}

fn line<'a>(layout: &'a ScreenplayLayout, text: &str) -> &'a scriptcraft_core::layout::LayoutLine {
    layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .find(|line| line.text.trim_end() == text.trim_end())
        .unwrap_or_else(|| unreachable!("fixture line must be present in editor layout"))
}

fn summary_position(project: &scriptcraft_core::Project, text: &str) -> usize {
    project
        .elements
        .iter()
        .position(|element| element.text.trim_end() == text.trim_end())
        .unwrap_or_else(|| unreachable!("summary fixture paragraph must exist"))
}

#[test]
fn every_supported_direct_field_projects_by_semantic_identity_and_wins_per_field() {
    let project = imported(PRECEDENCE_SOURCE);
    let state = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("import retains its source state"));
    let first = &project.elements[summary_position(&project, "one")];
    let overrides = state
        .source_layout_settings()
        .direct_paragraph_overrides
        .get(&first.id)
        .copied()
        .unwrap_or_else(|| unreachable!("managed summary receives direct projection"));
    assert_eq!(
        overrides,
        SourceParagraphOverrides {
            paragraph: scriptcraft_core::SourceParagraphSpec {
                alignment: Some(ParagraphAlignment::Right),
                first_indent: FixedPoint::parse_inches("-0.10"),
                left_indent: FixedPoint::parse_inches("1.12"),
                right_indent: FixedPoint::parse_inches("7.25"),
                leading_regular: Some(true),
                space_before: FixedPoint::parse_points("18"),
                spacing_single: Some(true),
            },
            starts_new_page: Some(true),
        }
    );

    let layout = layout(&project);
    let first = line(&layout, "one");
    assert_eq!(first.paragraph_bounds.x, FixedPoint::parse_inches("1.12").unwrap_or_default());
    assert_eq!(
        first.paragraph_bounds.width,
        FixedPoint::parse_inches("7.25")
            .zip(FixedPoint::parse_inches("1.12"))
            .map(|(right, left)| FixedPoint::from_raw(right.raw() - left.raw()))
            .unwrap_or_default()
    );
    assert_eq!(first.text_style.size, 12);
    assert!(first.text_style.hidden_text);
    assert_eq!(
        first.text_style.declared_family,
        "Arial",
        "direct paragraph geometry does not replace Summary typography"
    );

    let before = line(&layout, "before");
    assert!(first.page > before.page, "direct StartsNewPage applies to its paragraph");
    assert_eq!(first.paragraph_bounds.y, FixedPoint::from_points(72));
    assert_eq!(line(&layout, "ACT TWO").page, line(&layout, "three").page);
    let first_index = summary_position(&project, "one");
    assert!(layout
        .caret_geometry(Caret {
            element: first_index,
            offset: 0,
        })
        .is_ok_and(|caret| caret.x == first.geometry.x));
}

#[test]
fn invalid_direct_values_fall_back_to_role_and_sparse_direct_values_leave_other_fields_inherited() {
    let project = imported(PRECEDENCE_SOURCE);
    let state = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("import retains its source state"));
    let invalid = &project.elements[summary_position(&project, "two")];
    let invalid_projection = state
        .source_layout_settings()
        .direct_paragraph_overrides
        .get(&invalid.id)
        .copied()
        .unwrap_or_default();
    assert_eq!(invalid_projection, SourceParagraphOverrides::default());
    let preserved_invalid = write_v6(&project)
        .unwrap_or_else(|_| unreachable!("unsupported direct values remain serializable"));
    assert!(std::str::from_utf8(&preserved_invalid)
        .unwrap_or("")
        .contains("Alignment=\"Bogus\" FirstIndent=\"bad\""));

    let layout = layout(&project);
    let two = line(&layout, "two");
    assert_eq!(two.paragraph_bounds.x, FixedPoint::parse_inches("0.88").unwrap_or_default());
    assert_eq!(two.paragraph_bounds.width, FixedPoint::from_raw(47_664));
    assert_eq!(two.page, line(&layout, "one").page);

    let three_index = summary_position(&project, "three");
    let three = &project.elements[three_index];
    let sparse = state
        .source_layout_settings()
        .direct_paragraph_overrides
        .get(&three.id)
        .copied()
        .unwrap_or_else(|| unreachable!("SpaceBefore-only source value is projected"));
    assert_eq!(sparse.paragraph.space_before, FixedPoint::parse_points("20"));
    assert_eq!(sparse.paragraph.alignment, None);
    assert_eq!(sparse.paragraph.left_indent, None);
    assert_eq!(sparse.starts_new_page, None);
    let three_line = line(&layout, "three");
    let two_line = line(&layout, "two");
    assert_eq!(three_line.paragraph_bounds.x, two_line.paragraph_bounds.x);
    assert_eq!(
        three_line.paragraph_bounds.y.raw()
            - two_line.paragraph_bounds.y.raw()
            - two_line.paragraph_bounds.height.raw(),
        FixedPoint::parse_points("20").map_or(0, FixedPoint::raw)
    );
}

#[test]
fn populated_reference_summary_overrides_remain_attached_to_their_semantic_paragraphs() {
    let project = imported(include_bytes!("../../../.reference/reference2.fdx"));
    let layout = layout(&project);
    for (text, expected_left, expected_space) in [
        ("Summary of outline 2. ", "1.12", "14"),
        ("Summary of outline 3. ", "1.38", "14"),
    ] {
        let index = summary_position(&project, text);
        let element = &project.elements[index];
        let state = project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("reference import retains its source tree"));
        let overrides = state
            .source_layout_settings()
            .direct_paragraph_overrides
            .get(&element.id)
            .unwrap_or_else(|| unreachable!("reference paragraph direct settings are identity-bound"));
        assert_eq!(
            overrides.paragraph.left_indent,
            FixedPoint::parse_inches(expected_left)
        );
        assert_eq!(
            overrides.paragraph.space_before,
            FixedPoint::parse_points(expected_space)
        );
        let rendered = line(&layout, text);
        assert_eq!(rendered.paragraph_bounds.x, FixedPoint::parse_inches(expected_left).unwrap_or_default());
        assert!(rendered.text_style.hidden_text);
        assert_eq!(rendered.text_style.declared_family, "Arial");
    }
}

#[test]
fn source_attributes_survive_role_change_and_new_split_paragraph_does_not_inherit_xml() {
    let mut project = imported(PRECEDENCE_SOURCE);
    let index = summary_position(&project, "one");
    let mut session = EditingSession::new(project.clone())
        .unwrap_or_else(|_| unreachable!("fixture session must construct"));
    session
        .set_selection(Selection::caret(Position { element: index, offset: 0 }))
        .unwrap_or_else(|_| unreachable!("paragraph start is a valid caret"));
    assert_eq!(session.change_role(Role::Action), Ok(()));
    let saved = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("role change keeps source attributes serializable"));
    let xml = std::str::from_utf8(&saved).unwrap_or("");
    assert!(xml.contains(
        "Alignment=\"Right\" FirstIndent=\"-0.10\" Leading=\"Regular\" LeftIndent=\"1.12\" RightIndent=\"7.25\" SpaceBefore=\"18\" Spacing=\"1\" StartsNewPage=\"Yes\" Type=\"Action\" id=\"direct-one\" Keep=\"unchanged\""
    ));
    project = read_v6(&saved)
        .unwrap_or_else(|_| unreachable!("role-changed source reopens"))
        .project;
    let changed = &project.elements[index];
    assert!(project
        .preservation
        .as_ref()
        .is_some_and(|state| state.source_layout_settings().direct_paragraph_overrides.contains_key(&changed.id)));

    let split_source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Alignment="Right" FirstIndent="-0.10" LeftIndent="1.12" RightIndent="7.25" SpaceBefore="18" Type="Summary" id="split-target"><Text>one two</Text></Paragraph>
</Content></FinalDraft>"#;
    let split = imported(split_source);
    let index = summary_position(&split, "one two");
    let mut session = EditingSession::new(split)
        .unwrap_or_else(|_| unreachable!("simple managed paragraph session must construct"));
    session
        .set_selection(Selection::caret(Position { element: index, offset: 4 }))
        .unwrap_or_else(|_| unreachable!("interior split position is valid"));
    assert_eq!(session.split(), Ok(()));
    let saved = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("split retains its source paragraph mapping"));
    let reopened = read_v6(&saved)
        .unwrap_or_else(|_| unreachable!("split output reopens"))
        .project;
    let state = reopened
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("split output retains preservation state"));
    let left = &reopened.elements[index];
    let right = &reopened.elements[index + 1];
    assert!(state
        .source_layout_settings()
        .direct_paragraph_overrides
        .contains_key(&left.id));
    assert!(!state
        .source_layout_settings()
        .direct_paragraph_overrides
        .contains_key(&right.id));
    let xml = std::str::from_utf8(&saved).unwrap_or("");
    assert_eq!(xml.matches("SpaceBefore=\"18\"").count(), 1);
}

#[test]
fn joining_a_directly_formatted_right_paragraph_stops_transactionally_and_cache_keys_include_overrides() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" id="left"><Text>left</Text></Paragraph>
<Paragraph SpaceBefore="24" Type="Action" id="right"><Text>right</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut session = EditingSession::new(imported(source))
        .unwrap_or_else(|_| unreachable!("join fixture session must construct"));
    session
        .set_selection(Selection::caret(Position { element: 0, offset: 4 }))
        .unwrap_or_else(|_| unreachable!("join boundary is valid"));
    let before = session.clone();
    assert_eq!(
        session.join_with_next(),
        Err(EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(session, before);

    let first = imported(source);
    let second_source = source
        .windows(b"SpaceBefore=\"24\"".len())
        .position(|window| window == b"SpaceBefore=\"24\"")
        .map(|offset| {
            let mut changed = source.to_vec();
            changed.splice(offset..offset + b"SpaceBefore=\"24\"".len(), b"SpaceBefore=\"30\"".iter().copied());
            changed
        })
        .unwrap_or_else(|| unreachable!("fixture has one direct SpaceBefore"));
    let second = imported(&second_source);
    let mut cache = LayoutCache::default();
    let inputs = LayoutInputs::default();
    assert!(cache
        .get_or_build_with_inputs_for_purpose(7, &first, &inputs, LayoutPurpose::Editor)
        .is_ok_and(|(_, cached)| !cached));
    assert!(cache
        .get_or_build_with_inputs_for_purpose(7, &second, &inputs, LayoutPurpose::Editor)
        .is_ok_and(|(_, cached)| !cached));
}
