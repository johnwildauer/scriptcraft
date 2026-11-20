#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, Position, Project, ScreenplayElement, Selection,
};
use scriptcraft_session::editing::{EditPolicy, EditingSession};

const OPEN_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/open_element_settings.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("dynamic Parenthetical source uses valid bounded input"))
}

fn key(label: &str) -> ElementTypeKey {
    valid(ElementTypeKey::new(label))
}

fn ensure_definition(definitions: &mut Vec<scriptcraft_model::ElementDefinition>, label: &str) {
    if definitions
        .iter()
        .any(|definition| definition.key.as_str() == label)
    {
        return;
    }
    let mut definition = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("the Project contains its required Action definition"));
    definition.key = key(label);
    definitions.push(definition);
}

fn project_with_element_type(label: &str, text: &str) -> Project {
    let mut project = valid(scriptcraft_fdx::new_project());
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ensure_definition(&mut definitions, label);
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Parenthetical")
        .unwrap_or_else(|| unreachable!("the required Parenthetical definition exists"))
        .behavior
        .return_key = Some(key("Dialogue"));
    valid(project.replace_element_definitions(definitions));
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "parenthetical-owner",
        key(label),
        text,
    )))];
    project
}

fn project_with_parenthetical_tab_target() -> Project {
    let mut project = project_with_element_type("Verse", "");
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Verse")
        .unwrap_or_else(|| unreachable!("the current arbitrary source definition exists"))
        .behavior
        .tab_key = Some(key("Parenthetical"));
    valid(project.replace_element_definitions(definitions));
    project
}

fn retained_paragraph<'a>(
    element: &'a PreservedElement,
    foreign_id: &str,
) -> Option<&'a PreservedElement> {
    if element.name.local_name == "Paragraph"
        && element.attributes.iter().any(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
                && attribute.value == foreign_id
        })
    {
        return Some(element);
    }
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child) => retained_paragraph(child, foreign_id),
        _ => None,
    })
}

#[test]
fn exact_type_assignment_does_not_generate_parenthetical_text() {
    let mut assigned = valid(EditingSession::new(project_with_element_type("Verse", "")));
    valid(assigned.change_element_type(key("Parenthetical")));
    assert_eq!(
        assigned.project().elements[0].element_type(),
        key("Parenthetical")
    );
    assert_eq!(assigned.project().elements[0].text, "");

    valid(assigned.undo());
    assert_eq!(assigned.project().elements[0].element_type(), key("Verse"));
    valid(assigned.redo());
    assert_eq!(assigned.project().elements[0].text, "");
}

#[test]
fn edited_or_reconstructed_pairs_are_not_mistaken_for_generated_pairs() {
    let mut session = valid(EditingSession::new(project_with_parenthetical_tab_target()));
    valid(session.tab());
    valid(session.insert("x"));
    valid(session.backspace());
    assert_eq!(session.project().elements[0].text, "()");
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 1,
    })));
    valid(session.enter());
    assert_eq!(session.project().elements[0].text, "()");
    assert_eq!(
        session.project().elements[1].element_type(),
        key("Dialogue")
    );

    let mut remapped = valid(EditingSession::new(project_with_parenthetical_tab_target()));
    valid(remapped.tab());
    valid(remapped.change_element_type(key("Action")));
    assert_eq!(remapped.project().elements[0].text, "()");
    valid(remapped.change_element_type(key("Parenthetical")));
    valid(remapped.set_selection(Selection::caret(Position {
        element: 0,
        offset: 1,
    })));
    valid(remapped.enter());
    assert_eq!(remapped.project().elements[0].text, "()");
}

#[test]
fn disabled_automation_and_generic_type_changes_never_clear_authored_delimiters() {
    let mut session = valid(EditingSession::new(project_with_element_type("Verse", "")));
    session.set_policy(EditPolicy {
        auto_parenthetical: false,
        ..EditPolicy::default()
    });
    valid(session.change_element_type(key("Parenthetical")));
    assert_eq!(session.project().elements[0].text, "");
    valid(session.insert("()"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 1,
    })));
    valid(session.enter());
    assert_eq!(session.project().elements[0].text, "()");

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    valid(session.change_element_type(key("Action")));
    assert_eq!(session.project().elements[0].text, "()");

    let mut generated = valid(EditingSession::new(project_with_parenthetical_tab_target()));
    valid(generated.tab());
    generated.set_policy(EditPolicy {
        auto_parenthetical: false,
        ..EditPolicy::default()
    });
    valid(generated.enter());
    assert_eq!(generated.project().elements[0].text, "()");
}

#[test]
fn delimiter_tab_uses_the_parenthetical_definitions_custom_tab_key() {
    let mut project = project_with_element_type("Parenthetical", "()");
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ensure_definition(&mut definitions, "Aside");
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Parenthetical")
        .unwrap_or_else(|| unreachable!("the required Parenthetical definition exists"))
        .behavior
        .tab_key = Some(key("Aside"));
    valid(project.replace_element_definitions(definitions));

    let mut session = valid(EditingSession::new(project));
    valid(session.tab());
    assert_eq!(session.project().elements[0].element_type(), key("Aside"));
    assert_eq!(session.project().elements[0].text, "");
}

#[test]
fn behavior_transition_to_parenthetical_capability_generates_only_when_enabled() {
    let mut project = project_with_element_type("Verse", "");
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Verse")
        .unwrap_or_else(|| unreachable!("the current arbitrary source definition exists"))
        .behavior
        .tab_key = Some(key("Parenthetical"));
    valid(project.replace_element_definitions(definitions));

    let mut enabled = valid(EditingSession::new(project.clone()));
    valid(enabled.tab());
    assert_eq!(
        enabled.project().elements[0].element_type(),
        key("Parenthetical")
    );
    assert_eq!(enabled.project().elements[0].text, "()");
    assert_eq!(
        enabled.selection(),
        Selection::caret(Position {
            element: 0,
            offset: 1
        })
    );

    let mut disabled = valid(EditingSession::new(project));
    disabled.set_policy(EditPolicy {
        auto_parenthetical: false,
        ..EditPolicy::default()
    });
    valid(disabled.tab());
    assert_eq!(
        disabled.project().elements[0].element_type(),
        key("Parenthetical")
    );
    assert_eq!(disabled.project().elements[0].text, "");

    let mut misleading = project_with_element_type("Verse", "");
    let mut definitions = misleading
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ensure_definition(&mut definitions, "parenthetical");
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Verse")
        .unwrap_or_else(|| unreachable!("the arbitrary source definition exists"))
        .behavior
        .tab_key = Some(key("parenthetical"));
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "parenthetical")
        .unwrap_or_else(|| unreachable!("case-distinct target definition exists"))
        .behavior
        .paginate_as = Some("Parenthetical".to_owned());
    valid(misleading.replace_element_definitions(definitions));
    let mut no_guessed_target = valid(EditingSession::new(misleading));
    valid(no_guessed_target.tab());
    assert_eq!(
        no_guessed_target.project().elements[0].element_type(),
        key("parenthetical")
    );
    assert_eq!(no_guessed_target.project().elements[0].text, "");
}

#[test]
fn delimiter_only_delete_uses_capability_fallback_and_preserves_source_attachment() {
    let mut project = valid(read_v6(OPEN_FIXTURE)).project;
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ensure_definition(&mut definitions, "dialogue");
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "dialogue")
        .unwrap_or_else(|| unreachable!("case-distinct Dialogue-like definition exists"))
        .behavior
        .paginate_as = Some("Dialogue".to_owned());
    valid(project.replace_element_definitions(definitions));
    let owner_index = project
        .elements
        .iter()
        .position(|owner| {
            owner
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-body-general-derived")
        })
        .unwrap_or_else(|| unreachable!("fixture contains the attached managed paragraph"));
    let owner_id = project.elements[owner_index].id.clone();
    let original_overrides = project.direct_paragraph_overrides.clone();
    let foreign_id = "open-body-general-derived";
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position {
        element: owner_index,
        offset: 0,
    })));
    valid(session.change_element_type(key("Parenthetical")));
    let text_len = session.project().elements[owner_index].text.len();
    valid(session.set_selection(Selection {
        anchor: Position {
            element: owner_index,
            offset: 0,
        },
        focus: Position {
            element: owner_index,
            offset: text_len,
        },
    }));
    valid(session.insert("()"));
    valid(session.set_selection(Selection::caret(Position {
        element: owner_index,
        offset: 0,
    })));
    valid(session.delete());

    let survivor = &session.project().elements[owner_index];
    assert_eq!(survivor.id, owner_id);
    assert_eq!(survivor.element_type(), key("Dialogue"));
    assert_eq!(survivor.text, "");
    assert_eq!(
        session.project().direct_paragraph_overrides,
        original_overrides
    );
    let source = retained_paragraph(
        session
            .project()
            .preservation
            .as_ref()
            .and_then(|preservation| {
                preservation.sections.iter().find_map(|section| {
                    section.children.iter().find_map(|node| match node {
                        PreservedNode::Element(element) => retained_paragraph(element, foreign_id),
                        _ => None,
                    })
                })
            })
            .unwrap_or_else(|| unreachable!("fixture source remains attached")),
        foreign_id,
    )
    .unwrap_or_else(|| unreachable!("surviving source paragraph remains managed"));
    assert!(source.children.iter().any(
        |node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "UserParagraphData")
    ));
    assert!(source.attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name == "Type"
            && attribute.value == "Dialogue"
    }));
}

#[test]
fn delimiter_delete_is_capability_gated_and_after_close_uses_ordinary_delete() {
    let mut custom = project_with_element_type("parenthetical", "()");
    let mut definitions = custom
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "parenthetical")
        .unwrap_or_else(|| unreachable!("case-distinct type exists"))
        .behavior
        .paginate_as = Some("Parenthetical".to_owned());
    valid(custom.replace_element_definitions(definitions));
    let mut custom_session = valid(EditingSession::new(custom));
    valid(custom_session.delete());
    assert_eq!(
        custom_session.project().elements[0].element_type(),
        key("parenthetical")
    );
    assert_eq!(custom_session.project().elements[0].text, ")");

    for (text, offset, expected) in [("()", 2, "()following"), (")", 1, ")following")] {
        let mut exact = project_with_element_type("Parenthetical", text);
        exact
            .elements
            .push(BodyOwner::shared(valid(ScreenplayElement::new(
                "next-parenthetical",
                key("Parenthetical"),
                "following",
            ))));
        let mut exact_session = valid(EditingSession::new(exact));
        valid(exact_session.set_selection(Selection::caret(Position { element: 0, offset })));
        valid(exact_session.delete());
        assert_eq!(exact_session.project().elements.len(), 1);
        assert_eq!(exact_session.project().elements[0].text, expected);
        assert_eq!(
            exact_session.project().elements[0].element_type(),
            key("Parenthetical")
        );
    }
}

#[test]
fn delimiter_delete_covers_all_special_shapes_offsets_predecessor_and_range_paths() {
    for (text, offset) in [("(", 0), ("(", 1), ("()", 0), ("()", 1), (")", 0)] {
        let mut session = valid(EditingSession::new(project_with_element_type(
            "Parenthetical",
            text,
        )));
        valid(session.set_selection(Selection::caret(Position { element: 0, offset })));
        valid(session.delete());
        assert_eq!(session.project().elements.len(), 1);
        assert_eq!(
            session.project().elements[0].element_type(),
            key("Dialogue")
        );
        assert_eq!(session.project().elements[0].text, "");
        assert_eq!(
            session.selection(),
            Selection::caret(Position {
                element: 0,
                offset: 0
            })
        );
        valid(session.undo());
        assert_eq!(session.project().elements[0].text, text);
        valid(session.redo());
        assert_eq!(session.project().elements[0].text, "");
    }

    let mut predecessor_project = project_with_element_type("Parenthetical", "before");
    predecessor_project
        .elements
        .push(BodyOwner::shared(valid(ScreenplayElement::new(
            "later-parenthetical",
            key("Parenthetical"),
            "()",
        ))));
    let mut predecessor = valid(EditingSession::new(predecessor_project));
    valid(predecessor.set_selection(Selection::caret(Position {
        element: 1,
        offset: 1,
    })));
    valid(predecessor.delete());
    assert_eq!(predecessor.project().elements.len(), 1);
    assert_eq!(predecessor.project().elements[0].text, "before");
    assert_eq!(
        predecessor.selection(),
        Selection::caret(Position {
            element: 0,
            offset: 6
        })
    );

    let mut range = valid(EditingSession::new(project_with_element_type(
        "Parenthetical",
        "()",
    )));
    valid(range.set_selection(Selection {
        anchor: Position {
            element: 0,
            offset: 0,
        },
        focus: Position {
            element: 0,
            offset: 1,
        },
    }));
    valid(range.delete());
    assert_eq!(
        range.project().elements[0].element_type(),
        key("Parenthetical")
    );
    assert_eq!(range.project().elements[0].text, ")");
}

#[test]
fn unresolved_parenthetical_repair_target_is_a_complete_noop() {
    let mut project = project_with_element_type("Parenthetical", "(untouched");
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == "Parenthetical")
        .unwrap_or_else(|| unreachable!("the required Parenthetical definition exists"))
        .behavior
        .tab_key = Some(key("MissingTabTarget"));
    valid(project.replace_element_definitions(definitions));
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: "(untouched".len(),
    })));
    let before = session.clone();
    let revision = session.revision();
    let mutation_token = session.mutation_token();
    valid(session.tab());
    assert_eq!(session, before);
    assert_eq!(session.revision(), revision);
    assert_eq!(session.mutation_token(), mutation_token);
    assert!(!session.can_undo());
}
