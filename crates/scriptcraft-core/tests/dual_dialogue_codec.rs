#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::fdx::{read_v6, write_v6, FdxError, FdxDocument};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{Project, Role};

const BASIC_FIXTURE: &str = include_str!("fixtures/dual_dialogue_basic.fdx");
const EMPTY_SIDES_FIXTURE: &str = include_str!("fixtures/dual_dialogue_empty_sides.fdx");

fn import(source: &str) -> Option<FdxDocument> {
    let result = read_v6(source.as_bytes());
    assert!(result.is_ok(), "expected the authored FDX source to import");
    result.ok()
}

fn find_elements<'a>(nodes: &'a [PreservedNode], output: &mut Vec<&'a PreservedElement>) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            output.push(element);
            find_elements(&element.children, output);
        }
    }
}

fn attr<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.local_name == name)
        .map(|attribute| attribute.value.as_str())
}

fn document(body: &str, extra: &str) -> String {
    format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content>{body}</Content>{extra}</FinalDraft>"
    )
}

fn member_sequence(seed: &str, left_extra: &str, right_extra: &str) -> String {
    format!(
        "<Paragraph Type=\"Character\" id=\"{seed}-lc\"><Text>Left</Text></Paragraph>\
         <Paragraph Type=\"Dialogue\" id=\"{seed}-ld\"><Text>left words</Text></Paragraph>{left_extra}\
         <Paragraph Type=\"Character\" id=\"{seed}-rc\"><Text>Right</Text></Paragraph>\
         <Paragraph Type=\"Dialogue\" id=\"{seed}-rd\"><Text>right words</Text></Paragraph>{right_extra}"
    )
}

fn wrapper(seed: &str, members: &str) -> String {
    format!(
        "<Paragraph Type=\"General\" id=\"{seed}-outer\" LeftIndent=\"1.50\" RightIndent=\"7.50\" SpaceBefore=\"12\" StartsNewPage=\"No\" vendor=\"kept\"><DualDialogue><SpellCheckIgnoreLists><IgnoredRanges/><IgnoredWords/></SpellCheckIgnoreLists>{members}</DualDialogue></Paragraph>"
    )
}

fn contains_opaque(element: &PreservedElement, name: &str) -> bool {
    if element.name.local_name == name && element.ownership == SourceOwnership::Opaque {
        return true;
    }
    element.children.iter().any(|child| match child {
        PreservedNode::Element(child) => contains_opaque(child, name),
        _ => false,
    })
}

fn find_attr_element<'a>(
    nodes: &'a [PreservedNode],
    name: &str,
    value: &str,
) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if attr(element, name) == Some(value) {
            return Some(element);
        }
        if let Some(found) = find_attr_element(&element.children, name, value) {
            return Some(found);
        }
    }
    None
}

fn count_named(nodes: &[PreservedNode], name: &str) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            PreservedNode::Element(element) => {
                usize::from(element.name.local_name == name) + count_named(&element.children, name)
            }
            _ => 0,
        })
        .sum()
}

fn semantic_projection(project: &Project) -> Vec<(scriptcraft_core::StableId, Role, String, Vec<scriptcraft_core::TextRun>)> {
    project.elements.iter().map(|item| {
        (item.id.clone(), item.role, item.text.clone(), item.runs.clone())
    }).collect()
}

fn group_shapes(project: &Project) -> Vec<(Vec<Role>, Vec<Role>)> {
    project.dual_dialogue_groups.iter().map(|group| {
        let left = std::iter::once(&group.left.character)
            .chain(group.left.paragraphs.iter())
            .filter_map(|id| project.elements.iter().find(|item| &item.id == id).map(|item| item.role))
            .collect();
        let right = std::iter::once(&group.right.character)
            .chain(group.right.paragraphs.iter())
            .filter_map(|id| project.elements.iter().find(|item| &item.id == id).map(|item| item.role))
            .collect();
        (left, right)
    }).collect()
}

fn preservation_free(mut project: Project) -> Project {
    project.preservation = None;
    for group in &mut project.dual_dialogue_groups {
        group.wrapper.outer_foreign_id = None;
        group.wrapper.source = None;
        group.wrapper.preservation_anchor = None;
    }
    project
}

#[test]
fn imports_the_core_owned_reference_shape_as_flat_members_and_wrapper_metadata() {
    let Some(imported) = import(BASIC_FIXTURE) else {
        return;
    };
    let project = &imported.project;
    assert_eq!(project.elements.len(), 8);
    assert_eq!(project.dual_dialogue_groups.len(), 1);
    assert_eq!(project.elements[0].role, Role::Action);
    assert_eq!(project.elements[1].role, Role::Character);
    assert_eq!(project.elements[2].role, Role::Dialogue);
    assert_eq!(project.elements[3].role, Role::Parenthetical);
    assert_eq!(project.elements[4].role, Role::Character);
    assert_eq!(project.elements[5].role, Role::Dialogue);
    assert_eq!(project.elements[6].role, Role::Parenthetical);
    assert_eq!(project.elements[7].role, Role::Action);
    assert_eq!(project.elements[2].text, "First half—then the rest.");
    assert_eq!(project.elements[2].runs.len(), 2);
    assert_eq!(project.elements[5].text, "你好, world.");
    assert_eq!(project.elements[6].text, "");

    let group = &project.dual_dialogue_groups[0];
    assert_eq!(group.left.paragraphs.len(), 2);
    assert_eq!(group.right.paragraphs.len(), 2);
    assert_eq!(group.wrapper.outer_foreign_id.as_deref(), Some("outer-dual-001"));
    assert_eq!(
        group.wrapper.source.as_ref().and_then(|source| source.foreign_role.as_deref()),
        Some("General")
    );
    assert!(group.wrapper.preservation_anchor.is_some());
    let wrapper_fields = project.dual_dialogue_paragraph_overrides.get(&group.id);
    assert!(wrapper_fields.is_some_and(|fields| {
        fields.paragraph.left_indent.is_some()
            && fields.paragraph.right_indent.is_some()
            && fields.paragraph.space_before.is_some()
            && fields.starts_new_page == Some(false)
    }));

    let preservation = project.preservation.as_ref();
    assert!(preservation.is_some());
    let Some(preservation) = preservation else {
        return;
    };
    let mut elements = Vec::new();
    find_elements(&preservation.sections[0].children, &mut elements);
    assert!(elements.iter().any(|element| {
        element.name.local_name == "Paragraph"
            && element.ownership == SourceOwnership::DualDialogueWrapper(group.id.clone())
            && attr(element, "vendor-outer") == Some("retained")
    }));
    let managed = elements
        .iter()
        .filter(|element| matches!(element.ownership, SourceOwnership::Managed(_)))
        .count();
    assert_eq!(managed, 8);
    assert!(!project.elements.iter().any(|element| element.role == Role::General));
}

#[test]
fn preserves_both_spell_check_scopes_as_distinct_opaque_source_subtrees() {
    let Some(imported) = import(BASIC_FIXTURE) else {
        return;
    };
    let preservation = imported.project.preservation.as_ref();
    assert!(preservation.is_some());
    let Some(preservation) = preservation else {
        return;
    };
    let mut elements = Vec::new();
    find_elements(&preservation.sections[0].children, &mut elements);
    let spell_blocks = elements
        .iter()
        .filter(|element| element.name.local_name == "SpellCheckIgnoreLists")
        .collect::<Vec<_>>();
    assert_eq!(spell_blocks.len(), 2);
    assert!(spell_blocks.iter().all(|element| element.ownership == SourceOwnership::Opaque));
    assert!(spell_blocks.iter().any(|element| {
        attr(element, "vendor-scope") == Some("group")
            && contains_opaque(element, "Range")
            && contains_opaque(element, "Word")
    }));
    assert!(spell_blocks.iter().any(|element| {
        attr(element, "vendor-scope") == Some("document")
            && contains_opaque(element, "Range")
            && contains_opaque(element, "Word")
    }));
}

#[test]
fn imports_adjacent_groups_and_additional_side_paragraphs_in_order() {
    let left_extra = "<Paragraph Type=\"Parenthetical\" id=\"{seed}-lp\"><Text>(aside)</Text></Paragraph>";
    let right_extra = "<Paragraph Type=\"Dialogue\" id=\"{seed}-rd2\"><Text>follow-up</Text></Paragraph>";
    let first_members = member_sequence("one", &left_extra.replace("{seed}", "one"), &right_extra.replace("{seed}", "one"));
    let second_members = member_sequence("two", &left_extra.replace("{seed}", "two"), &right_extra.replace("{seed}", "two"));
    let body = format!("{}{}", wrapper("one", &first_members), wrapper("two", &second_members));
    let Some(imported) = import(&document(&body, "")) else {
        return;
    };
    assert_eq!(imported.project.dual_dialogue_groups.len(), 2);
    assert_eq!(imported.project.elements.len(), 12);
    assert_eq!(imported.project.dual_dialogue_groups[0].left.paragraphs.len(), 2);
    assert_eq!(imported.project.dual_dialogue_groups[0].right.paragraphs.len(), 2);
    assert_eq!(imported.project.dual_dialogue_groups[1].left.paragraphs.len(), 2);
    assert_eq!(imported.project.dual_dialogue_groups[1].right.paragraphs.len(), 2);
}

#[test]
fn empty_side_fixture_imports_and_source_backed_write_reread_preserve_members_and_wrapper_ownership() {
    let Some(imported) = import(EMPTY_SIDES_FIXTURE) else { return };
    let project = &imported.project;
    assert_eq!(project.dual_dialogue_groups.len(), 6);
    assert_eq!(project.elements.len(), 18);
    assert_eq!(group_shapes(project), vec![
        (vec![Role::Character], vec![Role::Character]),
        (vec![Role::Character], vec![Role::Character, Role::Dialogue]),
        (vec![Role::Character, Role::Dialogue], vec![Role::Character]),
        (vec![Role::Character], vec![Role::Character, Role::Parenthetical]),
        (vec![Role::Character, Role::Parenthetical], vec![Role::Character]),
        (vec![Role::Character, Role::Dialogue], vec![Role::Character, Role::Dialogue]),
    ]);
    assert_eq!(project.dual_dialogue_groups.iter().map(|group| group.wrapper.outer_foreign_id.as_deref()).collect::<Vec<_>>(),
        vec![Some("empty-cc-outer"), Some("empty-ccd-outer"), Some("empty-cdc-outer"),
             Some("empty-ccp-outer"), Some("empty-cpc-outer"), Some("valid-cdcd-outer")]);
    let cc_overrides = project.dual_dialogue_paragraph_overrides.get(&project.dual_dialogue_groups[0].id);
    assert!(cc_overrides.is_some_and(|overrides| {
        overrides.paragraph.left_indent.is_some()
            && overrides.paragraph.right_indent.is_some()
            && overrides.paragraph.space_before.is_some()
            && overrides.starts_new_page == Some(false)
    }));
    assert!(!project.elements.iter().any(|element| element.role == Role::General));
    for (group, expected) in project.dual_dialogue_groups.iter().zip([
        (0, 0), (0, 1), (1, 0), (0, 1), (1, 0), (1, 1),
    ]) {
        assert_eq!((group.left.paragraphs.len(), group.right.paragraphs.len()), expected);
    }
    let Some(source) = project.preservation.as_ref() else { return };
    assert_eq!(count_named(&source.sections[0].children, "DualDialogue"), 6);
    assert_eq!(count_named(&source.sections[0].children, "SpellCheckIgnoreLists"), 7);
    for id in ["empty-cc", "empty-ccd", "empty-cdc", "empty-ccp", "empty-cpc", "valid-cdcd"] {
        assert!(find_attr_element(&source.sections[0].children, "vendor-wrapper", id).is_some());
        assert!(find_attr_element(&source.sections[0].children, "vendor-dual", id).is_some());
    }
    let mut source_elements = Vec::new();
    find_elements(&source.sections[0].children, &mut source_elements);
    let member_ids = source_elements.iter().filter(|element| {
        element.name.local_name == "Paragraph" && matches!(element.ownership, SourceOwnership::Managed(_))
    }).filter_map(|element| attr(element, "id")).collect::<Vec<_>>();
    for id in [
        "empty-cc-left", "empty-cc-right", "empty-ccd-left", "empty-ccd-right", "empty-ccd-dialogue",
        "empty-cdc-left", "empty-cdc-dialogue", "empty-cdc-right", "empty-ccp-left", "empty-ccp-right",
        "empty-ccp-parenthetical", "empty-cpc-left", "empty-cpc-parenthetical", "empty-cpc-right",
        "valid-cdcd-left", "valid-cdcd-left-dialogue", "valid-cdcd-right", "valid-cdcd-right-dialogue",
    ] {
        assert!(member_ids.contains(&id));
    }

    let serialized = write_v6(project);
    assert!(serialized.is_ok(), "source-backed empty-side output must pass the existing writer gates");
    let Some(serialized) = serialized.ok() else { return };
    let reopened = read_v6(&serialized);
    assert!(reopened.is_ok(), "source-backed empty-side output must re-read");
    let Some(reopened) = reopened.ok() else { return };
    assert_eq!(semantic_projection(&reopened.project), semantic_projection(project));
    assert_eq!(group_shapes(&reopened.project), group_shapes(project));
    assert_eq!(reopened.project.dual_dialogue_groups, project.dual_dialogue_groups);
    for (before, after) in project.dual_dialogue_groups.iter().zip(&reopened.project.dual_dialogue_groups) {
        assert_eq!(project.dual_dialogue_paragraph_overrides.get(&before.id),
            reopened.project.dual_dialogue_paragraph_overrides.get(&after.id));
    }
    let Some(reopened_source) = reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&reopened_source.sections[0].children, "DualDialogue"), 6);
    assert_eq!(count_named(&reopened_source.sections[0].children, "SpellCheckIgnoreLists"), 7);
    for id in ["empty-cc", "empty-ccd", "empty-cdc", "empty-ccp", "empty-cpc", "valid"] {
        assert!(find_attr_element(&reopened_source.sections[0].children, "vendor-wrapper", id).is_some());
        assert!(find_attr_element(&reopened_source.sections[0].children, "vendor-dual", id).is_some());
        assert!(find_attr_element(&reopened_source.sections[0].children, "vendor-scope", id).is_some());
    }
    assert!(find_attr_element(&reopened_source.sections[0].children, "vendor-scope", "document").is_some());
    let mut reopened_elements = Vec::new();
    find_elements(&reopened_source.sections[0].children, &mut reopened_elements);
    let reopened_ids = reopened_elements.iter().filter(|element| {
        element.name.local_name == "Paragraph" && matches!(element.ownership, SourceOwnership::Managed(_))
    }).filter_map(|element| attr(element, "id")).collect::<Vec<_>>();
    for id in member_ids { assert!(reopened_ids.contains(&id)); }
}

#[test]
fn preservation_free_generation_emits_only_actual_empty_side_members_and_rereads() {
    let Some(imported) = import(EMPTY_SIDES_FIXTURE) else { return };
    let project = preservation_free(imported.project);
    let expected_shapes = group_shapes(&project);
    let expected_projection = project.elements.iter().map(|item| (item.role, item.text.clone(), item.runs.clone())).collect::<Vec<_>>();
    let serialized = write_v6(&project);
    assert!(serialized.is_ok(), "preservation-free empty-side groups must serialize");
    let Some(serialized) = serialized.ok() else { return };
    let reopened = read_v6(&serialized);
    assert!(reopened.is_ok(), "generated empty-side wrappers must re-read");
    let Some(reopened) = reopened.ok() else { return };
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 6);
    assert_eq!(group_shapes(&reopened.project), expected_shapes);
    assert_eq!(reopened.project.elements.iter().map(|item| (item.role, item.text.clone(), item.runs.clone())).collect::<Vec<_>>(), expected_projection);
    assert_eq!(reopened.project.elements.len(), 18);
    let Some(source) = reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&source.sections[0].children, "DualDialogue"), 6);
    assert_eq!(count_named(&source.sections[0].children, "SpellCheckIgnoreLists"), 7);
    let mut elements = Vec::new();
    find_elements(&source.sections[0].children, &mut elements);
    assert_eq!(elements.iter().filter(|element| {
        element.name.local_name == "Paragraph" && matches!(element.ownership, SourceOwnership::Managed(_))
    }).count(), 18);
    assert!(!reopened.project.elements.iter().any(|element| element.role == Role::General));
}

#[test]
fn safe_stops_malformed_patterns_namespaces_mixed_content_and_nested_wrappers() {
    let good = member_sequence("bad", "", "");
    let malformed = [
        wrapper("one-cue", "<Paragraph Type=\"Character\" id=\"one-cue\"><Text/></Paragraph>"),
        wrapper("third-cue", "<Paragraph Type=\"Character\" id=\"tc-1\"><Text>A</Text></Paragraph><Paragraph Type=\"Dialogue\" id=\"tc-2\"><Text>x</Text></Paragraph><Paragraph Type=\"Character\" id=\"tc-3\"><Text>B</Text></Paragraph><Paragraph Type=\"Dialogue\" id=\"tc-4\"><Text>y</Text></Paragraph><Paragraph Type=\"Character\" id=\"tc-5\"><Text>C</Text></Paragraph>"),
        wrapper("wrong-role", "<Paragraph Type=\"Character\" id=\"wr-1\"><Text>A</Text></Paragraph><Paragraph Type=\"Action\" id=\"wr-2\"><Text>x</Text></Paragraph><Paragraph Type=\"Character\" id=\"wr-3\"><Text>B</Text></Paragraph><Paragraph Type=\"Dialogue\" id=\"wr-4\"><Text>y</Text></Paragraph>"),
        wrapper("attachment", &format!("{good}<Unknown/>")),
        wrapper("mixed", &format!("{good}non-whitespace")),
        wrapper("nested", &format!("<Paragraph Type=\"Character\" id=\"n-1\"><Text>A</Text></Paragraph><Paragraph Type=\"Dialogue\" id=\"n-2\"><Text>x</Text></Paragraph><Paragraph Type=\"Character\" id=\"n-3\"><Text>B</Text></Paragraph><Paragraph Type=\"Dialogue\" id=\"n-4\"><Text>y</Text></Paragraph><Nested><DualDialogue/></Nested>")),
    ];
    for source_wrapper in malformed {
        let result = read_v6(document(&source_wrapper, "").as_bytes());
        assert_eq!(result.err(), Some(FdxError::UnsupportedConstruct));
    }

    let mixed_valid_and_invalid = format!(
        "{}{}",
        wrapper("valid-before-invalid", &member_sequence("valid-before-invalid", "", "")),
        wrapper("third-cue-after-valid", "<Paragraph Type=\"Character\" id=\"tcv-1\"><Text/></Paragraph><Paragraph Type=\"Dialogue\" id=\"tcv-2\"><Text/></Paragraph><Paragraph Type=\"Character\" id=\"tcv-3\"><Text/></Paragraph><Paragraph Type=\"Dialogue\" id=\"tcv-4\"><Text/></Paragraph><Paragraph Type=\"Character\" id=\"tcv-5\"><Text/></Paragraph>"),
    );
    assert_eq!(read_v6(document(&mixed_valid_and_invalid, "").as_bytes()).err(), Some(FdxError::UnsupportedConstruct));

    let wrong_namespace = wrapper("namespace", &good).replace(
        "<DualDialogue>",
        "<DualDialogue xmlns=\"urn:unsupported\">",
    );
    assert_eq!(
        read_v6(document(&wrong_namespace, "").as_bytes()).err(),
        Some(FdxError::UnsupportedConstruct)
    );
    let multiple_containers = wrapper("multiple", &member_sequence("multiple", "", ""))
        .replace(
            "</DualDialogue>",
            "</DualDialogue><DualDialogue><SpellCheckIgnoreLists><IgnoredRanges/><IgnoredWords/></SpellCheckIgnoreLists></DualDialogue>",
        );
    assert_eq!(
        read_v6(document(&multiple_containers, "").as_bytes()).err(),
        Some(FdxError::UnsupportedConstruct)
    );
    let duplicate_id = wrapper("duplicate", &member_sequence("duplicate", "", ""))
        .replace("id=\"duplicate-lc\"", "id=\"duplicate-outer\"");
    assert_eq!(
        read_v6(document(&duplicate_id, "").as_bytes()).err(),
        Some(FdxError::AmbiguousIdentity)
    );
    let unsupported_run = wrapper(
        "run",
        "<Paragraph Type=\"Character\" id=\"run-1\"><Text>A</Text></Paragraph><Paragraph Type=\"Dialogue\" id=\"run-2\"><Text><Inline>x</Inline></Text></Paragraph><Paragraph Type=\"Character\" id=\"run-3\"><Text>B</Text></Paragraph><Paragraph Type=\"Dialogue\" id=\"run-4\"><Text>y</Text></Paragraph>",
    );
    assert_eq!(
        read_v6(document(&unsupported_run, "").as_bytes()).err(),
        Some(FdxError::UnsupportedConstruct)
    );
    let title_occurrence = document("", &format!("<TitlePage><DualDialogue>{good}</DualDialogue></TitlePage>"));
    assert_eq!(
        read_v6(title_occurrence.as_bytes()).err(),
        Some(FdxError::UnsupportedConstruct)
    );
    let wrong_ancestry = document(
        &format!("<Paragraph Type=\"Action\" id=\"outer-action\">{}</Paragraph>", wrapper("nested-general", &good)),
        "",
    );
    assert_eq!(read_v6(wrong_ancestry.as_bytes()).err(), Some(FdxError::UnsupportedConstruct));
}

#[test]
fn preservation_free_generation_writes_one_complete_wrapper_per_group() {
    let Some(imported) = import(BASIC_FIXTURE) else {
        return;
    };
    let mut one_group = preservation_free(imported.project);
    let member_id = one_group.dual_dialogue_groups[0].left.character.clone();
    let mut member_overrides = scriptcraft_core::formatting::SourceParagraphOverrides::default();
    member_overrides.paragraph.alignment = Some(scriptcraft_core::ParagraphAlignment::Center);
    member_overrides.paragraph.left_indent = scriptcraft_core::FixedPoint::parse_inches("1.25");
    member_overrides.starts_new_page = Some(true);
    one_group
        .direct_paragraph_overrides
        .insert(member_id.clone(), member_overrides);
    let generated = write_v6(&one_group);
    assert!(generated.is_ok(), "a valid semantic group must serialize");
    let Some(generated) = generated.ok() else {
        return;
    };
    let reopened = read_v6(&generated);
    assert!(reopened.is_ok(), "generated grouped FDX must re-import");
    let Some(reopened) = reopened.ok() else {
        return;
    };
    assert_eq!(reopened.project.elements.len(), one_group.elements.len());
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 1);
    assert_eq!(
        reopened.project.direct_paragraph_overrides.get(&reopened.project.elements[1].id),
        Some(&member_overrides)
    );
    assert!(reopened.project.dual_dialogue_groups[0]
        .wrapper
        .outer_foreign_id
        .is_some());
    let Some(preservation) = reopened.project.preservation.as_ref() else {
        return;
    };
    assert_eq!(count_named(&preservation.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&preservation.sections[0].children, "SpellCheckIgnoreLists"), 1);
    assert_eq!(count_named(&preservation.sections[0].children, "IgnoredRanges"), 1);
    assert_eq!(count_named(&preservation.sections[0].children, "IgnoredWords"), 1);
    let mut elements = Vec::new();
    find_elements(&preservation.sections[0].children, &mut elements);
    let outer_ids = elements
        .iter()
        .filter(|element| {
            element.name.local_name == "Paragraph"
                && element.source_type.as_deref() == Some("General")
        })
        .filter_map(|element| attr(element, "id"))
        .collect::<Vec<_>>();
    assert_eq!(outer_ids.len(), 1);
    assert!(preservation.identities.iter().any(|identity| {
        identity.domain == scriptcraft_core::preservation::ForeignIdDomain::Paragraph
            && Some(identity.value.as_str()) == outer_ids.first().copied()
    }));

    let two_members = member_sequence("generated-two", "", "");
    let two_source = document(
        &format!("{}{}", wrapper("generated-one", &member_sequence("generated-one", "", "")), wrapper("generated-two", &two_members)),
        "",
    );
    let Some(mut two_imported) = import(&two_source) else {
        return;
    };
    two_imported.project = preservation_free(two_imported.project);
    let multiple_generated = write_v6(&two_imported.project);
    assert!(multiple_generated.is_ok(), "adjacent groups must generate independently");
    let Some(multiple_generated) = multiple_generated.ok() else {
        return;
    };
    let multiple_reopened = read_v6(&multiple_generated);
    assert!(multiple_reopened.is_ok(), "multiple generated groups must re-import");
    let Some(multiple_reopened) = multiple_reopened.ok() else {
        return;
    };
    assert_eq!(multiple_reopened.project.dual_dialogue_groups.len(), 2);
    let Some(multiple_state) = multiple_reopened.project.preservation.as_ref() else {
        return;
    };
    assert_eq!(count_named(&multiple_state.sections[0].children, "DualDialogue"), 2);
    assert_eq!(count_named(&multiple_state.sections[0].children, "SpellCheckIgnoreLists"), 2);
}

#[test]
fn source_backed_writer_round_trips_nested_text_and_both_spell_check_scopes() {
    let Some(mut imported) = import(BASIC_FIXTURE) else {
        return;
    };
    let original_state = imported.project.preservation.as_ref();
    assert!(original_state.is_some());
    let Some(original_state) = original_state else {
        return;
    };
    let original_document_spell = find_attr_element(
        &original_state.sections[0].children,
        "vendor-scope",
        "document",
    )
    .cloned();
    let original_nested_spell = find_attr_element(
        &original_state.sections[0].children,
        "vendor-scope",
        "group",
    )
    .cloned();
    assert!(original_document_spell.is_some());
    assert!(original_nested_spell.is_some());

    let edited = &mut imported.project.elements[2];
    assert_eq!(edited.runs.len(), 2);
    edited.runs[1].text.push_str(" 再見✨");
    edited.text = edited.runs.iter().map(|run| run.text.as_str()).collect();
    let serialized = write_v6(&imported.project);
    assert!(serialized.is_ok(), "nested managed text edits must reconcile");
    let Some(serialized) = serialized.ok() else {
        return;
    };
    let reopened = read_v6(&serialized);
    assert!(reopened.is_ok(), "source-backed grouped output must re-import");
    let Some(reopened) = reopened.ok() else {
        return;
    };
    assert_eq!(reopened.project.elements[2].text, "First half—then the rest. 再見✨");
    assert_eq!(reopened.project.elements[2].runs[1].attributes, imported.project.elements[2].runs[1].attributes);
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 1);
    assert_eq!(reopened.project.elements.len(), imported.project.elements.len());
    let Some(reopened_state) = reopened.project.preservation.as_ref() else {
        return;
    };
    assert_eq!(
        find_attr_element(&reopened_state.sections[0].children, "vendor-scope", "document"),
        original_document_spell.as_ref()
    );
    assert_eq!(
        find_attr_element(&reopened_state.sections[0].children, "vendor-scope", "group"),
        original_nested_spell.as_ref()
    );
    assert_eq!(count_named(&reopened_state.sections[0].children, "DualDialogue"), 1);
    let mut source_elements = Vec::new();
    find_elements(&reopened_state.sections[0].children, &mut source_elements);
    assert!(source_elements.iter().any(|element| {
        element.name.local_name == "Paragraph"
            && element.source_type.as_deref() == Some("General")
            && attr(element, "vendor-outer") == Some("retained")
    }));
    assert!(source_elements.iter().any(|element| {
        element.name.local_name == "DualDialogue"
            && attr(element, "vendor-container") == Some("retained")
    }));
    let group = &reopened.project.dual_dialogue_groups[0];
    assert!(reopened_state.anchors.iter().any(|anchor| {
        anchor.target == scriptcraft_core::preservation::AnchorTarget::DualDialogueWrapper(group.id.clone())
    }));
    assert!(reopened_state.identities.iter().any(|identity| {
        identity.domain == scriptcraft_core::preservation::ForeignIdDomain::Paragraph
            && Some(identity.value.as_str()) == group.wrapper.outer_foreign_id.as_deref()
    }));
}

#[test]
fn source_backed_toggle_generates_reference_wrapper_and_reopens_with_both_spell_scopes() {
    let source = document(
        &member_sequence("toggle", "", ""),
        "<SpellCheckIgnoreLists><IgnoredRanges/><IgnoredWords/></SpellCheckIgnoreLists>",
    );
    let Some(imported) = import(&source) else {
        return;
    };
    let original_project = imported.project.clone();
    let original_members = imported.project.elements.clone();
    let Ok(mut session) = EditingSession::new(imported.project) else {
        return;
    };
    assert!(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })).is_ok());
    assert!(session.toggle_dual_dialogue().is_ok());
    assert_eq!(session.project().elements, original_members);
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    let group = &session.project().dual_dialogue_groups[0];
    let Some(preservation) = session.project().preservation.as_ref() else {
        return;
    };
    assert_eq!(count_named(&preservation.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&preservation.sections[0].children, "SpellCheckIgnoreLists"), 2);
    let mut elements = Vec::new();
    find_elements(&preservation.sections[0].children, &mut elements);
    let wrapper = elements.iter().find(|element| {
        element.name.local_name == "Paragraph"
            && element.ownership == SourceOwnership::DualDialogueWrapper(group.id.clone())
    });
    assert!(wrapper.is_some());
    let Some(wrapper) = wrapper else {
        return;
    };
    let outer_id = attr(wrapper, "id").map(str::to_owned);
    assert!(outer_id.is_some());
    assert!(preservation.identities.iter().all(|identity| {
        identity.domain != scriptcraft_core::preservation::ForeignIdDomain::Paragraph
            || Some(identity.value.as_str()) != outer_id.as_deref()
            || Some(identity.value.as_str()) == group.wrapper.outer_foreign_id.as_deref()
    }));
    let nested_spell = wrapper.children.iter().find_map(|node| match node {
        PreservedNode::Element(dual) if dual.name.local_name == "DualDialogue" => {
            dual.children.iter().find_map(|nested| match nested {
                PreservedNode::Element(spell) if spell.name.local_name == "SpellCheckIgnoreLists" => Some(spell),
                _ => None,
            })
        }
        _ => None,
    });
    assert!(nested_spell.is_some_and(|spell| {
        spell.children.iter().filter_map(|node| match node {
            PreservedNode::Element(element) => Some(element.name.local_name.as_str()),
            _ => None,
        }).collect::<Vec<_>>() == ["IgnoredRanges", "IgnoredWords"]
    }));

    let wrapped_snapshot = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &original_project);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &wrapped_snapshot);

    let serialized = write_v6(session.project());
    assert!(serialized.is_ok());
    let Some(serialized) = serialized.ok() else {
        return;
    };
    let reopened = read_v6(&serialized);
    assert!(reopened.is_ok());
    let Some(reopened) = reopened.ok() else {
        return;
    };
    assert_eq!(reopened.project.elements, original_members);
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 1);
    assert_eq!(reopened.project.dual_dialogue_groups[0].wrapper.outer_foreign_id.as_deref(), outer_id.as_deref());
    let Some(reopened_source) = reopened.project.preservation.as_ref() else {
        return;
    };
    assert_eq!(count_named(&reopened_source.sections[0].children, "SpellCheckIgnoreLists"), 2);
}

#[test]
fn source_backed_two_cue_group_unwraps_and_rewraps_exactly_two_members() {
    let Some(imported) = import(EMPTY_SIDES_FIXTURE) else { return };
    let project = imported.project;
    let cue_only = project.dual_dialogue_groups.iter().find(|group| {
        group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()
    }).cloned();
    assert!(cue_only.is_some());
    let Some(cue_only) = cue_only else { return };
    let member_ids = cue_only.member_ids().cloned().collect::<Vec<_>>();
    let Some(first_index) = project.elements.iter().position(|element| element.id == member_ids[0]) else {
        return;
    };
    let Ok(mut session) = EditingSession::new(project.clone()) else { return };
    assert!(session.set_selection(Selection::caret(Position { element: first_index, offset: 0 })).is_ok());
    assert!(session.toggle_dual_dialogue().is_ok());
    assert!(session.project().dual_dialogue_groups.iter().all(|group| group.id != cue_only.id));
    assert_eq!(session.project().elements.iter().filter(|element| member_ids.contains(&element.id)).count(), 2);
    let unwrapped = session.project().clone();

    let current_first = session.project().elements.iter().position(|element| element.id == member_ids[0]);
    assert!(current_first.is_some());
    let Some(current_first) = current_first else { return };
    assert!(session.set_selection(Selection::caret(Position { element: current_first, offset: 0 })).is_ok());
    assert!(session.toggle_dual_dialogue().is_ok());
    let wrapped = session.project().clone();
    let restored_group = wrapped.dual_dialogue_groups.iter().find(|group| {
        group.left.character == member_ids[0] && group.right.character == member_ids[1]
    });
    assert!(restored_group.is_some());
    assert_eq!(restored_group.map(|group| group.member_ids().cloned().collect::<Vec<_>>()), Some(member_ids.clone()));

    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &unwrapped);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &wrapped);
    let Some(bytes) = write_v6(session.project()).ok() else { return };
    let Some(reopened) = read_v6(&bytes).ok() else { return };
    let reopened_group = reopened.project.dual_dialogue_groups.iter().find(|group| {
        group.left.character == member_ids[0] && group.right.character == member_ids[1]
    });
    assert!(reopened_group.is_some());
    assert_eq!(reopened_group.map(|group| group.member_ids().cloned().collect::<Vec<_>>()), Some(member_ids));
    let Some(preservation) = reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&preservation.sections[0].children, "DualDialogue"), 5);
}

#[test]
fn nested_structural_edits_round_trip_split_insert_join_and_delete() {
    let Some(imported) = import(BASIC_FIXTURE) else { return };
    let Ok(mut split) = EditingSession::new(imported.project.clone()) else { return };
    assert!(split.set_selection(Selection::caret(Position {
        element: 2,
        offset: "First half—".len(),
    })).is_ok());
    assert!(split.split().is_ok());
    let Some(split_bytes) = write_v6(split.project()).ok() else { return };
    let Some(split_reopened) = read_v6(&split_bytes).ok() else { return };
    assert_eq!(semantic_projection(&split_reopened.project), semantic_projection(split.project()));
    assert_eq!(split_reopened.project.dual_dialogue_groups, split.project().dual_dialogue_groups);
    let Some(split_source) = split_reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&split_source.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&split_source.sections[0].children, "SpellCheckIgnoreLists"), 2);

    let Ok(mut inserted_parenthetical) = EditingSession::with_policy(
        imported.project.clone(),
        scriptcraft_core::editing::EditPolicy {
            auto_parenthetical: true,
            ..scriptcraft_core::editing::EditPolicy::default()
        },
    ) else { return };
    assert!(inserted_parenthetical.set_selection(Selection::caret(Position { element: 3, offset: 0 })).is_ok());
    assert!(inserted_parenthetical.enter().is_ok());
    let Some(inserted_bytes) = write_v6(inserted_parenthetical.project()).ok() else { return };
    let Some(inserted_reopened) = read_v6(&inserted_bytes).ok() else { return };
    assert_eq!(semantic_projection(&inserted_reopened.project), semantic_projection(inserted_parenthetical.project()));
    assert_eq!(inserted_reopened.project.dual_dialogue_groups, inserted_parenthetical.project().dual_dialogue_groups);
    let Some(inserted_source) = inserted_reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&inserted_source.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&inserted_source.sections[0].children, "SpellCheckIgnoreLists"), 2);

    let Ok(mut joined) = EditingSession::new(imported.project.clone()) else { return };
    assert!(joined.set_selection(Selection::caret(Position {
        element: 2,
        offset: joined.project().elements[2].text.len(),
    })).is_ok());
    assert!(joined.join_with_next().is_ok());
    let Some(joined_bytes) = write_v6(joined.project()).ok() else { return };
    let Some(joined_reopened) = read_v6(&joined_bytes).ok() else { return };
    assert_eq!(semantic_projection(&joined_reopened.project), semantic_projection(joined.project()));
    assert_eq!(joined_reopened.project.dual_dialogue_groups, joined.project().dual_dialogue_groups);
    let Some(joined_source) = joined_reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&joined_source.sections[0].children, "DualDialogue"), 1);

    let Ok(mut deleted) = EditingSession::new(imported.project.clone()) else { return };
    let right_end = deleted.project().elements[5].text.len();
    assert!(deleted.set_selection(Selection {
        anchor: Position { element: 5, offset: right_end },
        focus: Position { element: 6, offset: 0 },
    }).is_ok());
    assert!(deleted.delete().is_ok());
    let Some(deleted_bytes) = write_v6(deleted.project()).ok() else { return };
    let Some(deleted_reopened) = read_v6(&deleted_bytes).ok() else { return };
    assert_eq!(semantic_projection(&deleted_reopened.project), semantic_projection(deleted.project()));
    assert_eq!(deleted_reopened.project.dual_dialogue_groups, deleted.project().dual_dialogue_groups);
    let Some(deleted_source) = deleted_reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&deleted_source.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&deleted_source.sections[0].children, "SpellCheckIgnoreLists"), 2);
}

#[test]
fn final_right_enter_round_trips_action_after_retained_wrapper_and_spell_payloads() {
    let source = BASIC_FIXTURE.replace(
        "<Paragraph Type=\"Parenthetical\" id=\"right-parenthetical-001\"/>",
        "",
    );
    let Some(imported) = import(&source) else { return };
    let original_document_spell = imported.project.preservation.as_ref().and_then(|state|
        find_attr_element(&state.sections[0].children, "vendor-scope", "document")).cloned();
    let original_nested_spell = imported.project.preservation.as_ref().and_then(|state|
        find_attr_element(&state.sections[0].children, "vendor-scope", "group")).cloned();
    let group = imported.project.dual_dialogue_groups.first();
    let Some(group) = group else { return };
    let final_member = group.member_ids().last();
    let Some(final_member) = final_member else { return };
    let Some(final_index) = imported.project.elements.iter().position(|element| &element.id == final_member) else { return };
    let offset = imported.project.elements[final_index].text.len();
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    assert!(session.set_selection(Selection::caret(Position { element: final_index, offset })).is_ok());
    assert!(session.enter().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    let action_index = final_index + 1;
    assert_eq!(session.project().elements[action_index].role, Role::Action);
    let serialized = write_v6(session.project());
    assert!(serialized.is_ok(), "the retained wrapper and following Action must serialize together");
    let Some(serialized) = serialized.ok() else { return };
    let reopened = read_v6(&serialized);
    assert!(reopened.is_ok(), "the final-right Enter result must re-import");
    let Some(reopened) = reopened.ok() else { return };
    assert_eq!(semantic_projection(&reopened.project), semantic_projection(session.project()));
    assert_eq!(reopened.project.dual_dialogue_groups, session.project().dual_dialogue_groups);
    assert_eq!(reopened.project.elements[action_index].role, Role::Action);
    let Some(reopened_state) = reopened.project.preservation.as_ref() else { return };
    assert_eq!(count_named(&reopened_state.sections[0].children, "DualDialogue"), 1);
    assert_eq!(count_named(&reopened_state.sections[0].children, "SpellCheckIgnoreLists"), 2);
    assert_eq!(find_attr_element(&reopened_state.sections[0].children, "vendor-scope", "document"),
        original_document_spell.as_ref());
    assert_eq!(find_attr_element(&reopened_state.sections[0].children, "vendor-scope", "group"),
        original_nested_spell.as_ref());
}
