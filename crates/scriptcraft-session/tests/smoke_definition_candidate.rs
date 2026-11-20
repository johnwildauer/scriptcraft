#![forbid(unsafe_code)]

use scriptcraft_fdx::{read_v6, reconcile_project_element_settings};
use scriptcraft_model::dual_dialogue::DualDialogueGroup;
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{
    BodyOwner, ElementDefinition, ElementTypeKey, Position, Project, ScreenplayElement, Selection,
    StableId,
};
use scriptcraft_session::editing::{
    EditError, EditingSession, ElementDefinitionCandidate, ElementTypeRemapping,
};

const OPEN_FIXTURE: &[u8] = include_bytes!("fixtures/open_element_settings.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("definition candidate smoke input is valid"))
}

fn key(label: &str) -> ElementTypeKey {
    valid(ElementTypeKey::new(label))
}

fn unqualified_type(element: &PreservedElement) -> Option<&str> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        })
        .map(|attribute| attribute.value.as_str())
}

fn find_foreign_id<'a>(element: &'a PreservedElement, id: &str) -> Option<&'a PreservedElement> {
    if element.attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name == "id"
            && attribute.value == id
    }) {
        return Some(element);
    }
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child) => find_foreign_id(child, id),
        _ => None,
    })
}

fn find_document_root(element: &PreservedElement) -> Option<&PreservedElement> {
    if element.name.local_name == "FinalDraft" {
        return Some(element);
    }
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(child) => find_document_root(child),
        _ => None,
    })
}

fn retained_by_id<'a>(project: &'a Project, id: &str) -> &'a PreservedElement {
    project
        .preservation
        .as_ref()
        .and_then(|preservation| {
            preservation.sections.iter().find_map(|section| {
                section.children.iter().find_map(|node| match node {
                    PreservedNode::Element(element) => find_foreign_id(element, id),
                    _ => None,
                })
            })
        })
        .unwrap_or_else(|| unreachable!("fixture source identity remains retained"))
}

fn winning_settings_order(project: &Project) -> Vec<String> {
    project
        .preservation
        .as_ref()
        .and_then(|preservation| {
            preservation.sections.iter().find_map(|section| {
                section.children.iter().find_map(|node| match node {
                    PreservedNode::Element(element) => find_document_root(element),
                    _ => None,
                })
            })
        })
        .map(|root| {
            root.children
                .iter()
                .filter_map(|node| match node {
                    PreservedNode::Element(element)
                        if element.name.local_name == "ElementSettings"
                            && element.anchor.is_some() =>
                    {
                        element.source_type.clone()
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn rename_definition(definitions: &mut [ElementDefinition], from: &str, to: &str) {
    let definition = definitions
        .iter_mut()
        .find(|definition| definition.key.as_str() == from)
        .unwrap_or_else(|| unreachable!("fixture contains the source definition"));
    definition.key = key(to);
}

#[test]
fn source_backed_definition_candidate_remaps_body_and_content_titles_atomically() {
    let project = valid(read_v6(OPEN_FIXTURE)).project;
    let original_bytes = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source.original_bytes.clone())
        .unwrap_or_default();
    let body_index = project
        .elements
        .iter()
        .position(|owner| {
            owner
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-body-lyrics")
        })
        .unwrap_or_else(|| unreachable!("fixture Lyrics paragraph is managed"));
    let title_index = project
        .title_page
        .iter()
        .position(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-title-content")
        })
        .unwrap_or_else(|| unreachable!("TitlePage Content paragraph is managed"));
    let typeless_title_index = project
        .title_page
        .iter()
        .position(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-title-typeless")
        })
        .unwrap_or_else(|| unreachable!("type-less Content paragraph is managed"));

    let mut session = valid(EditingSession::new(project));
    let body_selection = Selection::caret(Position {
        element: body_index,
        offset: 3,
    });
    let title_selection = Selection::caret(Position {
        element: title_index,
        offset: 4,
    });
    valid(session.set_selection(body_selection));
    valid(session.insert("temporary history branch"));
    valid(session.undo());
    assert!(session.can_redo());
    valid(session.set_title_selection(title_selection));
    let before_body = session.project().elements[body_index].as_element().clone();
    let before_title = session.project().title_page[title_index].clone();
    let before_typeless = session.project().title_page[typeless_title_index].clone();
    let before_direct_overrides = session.project().direct_paragraph_overrides.clone();
    let before_dual_overrides = session.project().dual_dialogue_paragraph_overrides.clone();
    let before_body_typing = session.body_typing_style();
    let before_title_typing = session.title_typing_style();
    let before_body_children = retained_by_id(session.project(), "open-body-lyrics")
        .children
        .clone();
    let opaque_direct_title = retained_by_id(session.project(), "open-title-direct").clone();
    let opaque_header = retained_by_id(session.project(), "open-header-title").clone();
    let opaque_footer = retained_by_id(session.project(), "open-footer-title").clone();
    let opaque_wrapper_title = retained_by_id(session.project(), "open-title-opaque").clone();

    let mut definitions = session
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let original_order = definitions
        .iter()
        .map(|definition| definition.key.as_str().to_owned())
        .collect::<Vec<_>>();
    rename_definition(&mut definitions, "Lyrics", "Song");
    rename_definition(&mut definitions, "TitleContent", "Credit");
    definitions.reverse();
    let target_order = definitions
        .iter()
        .map(|definition| definition.key.as_str().to_owned())
        .collect::<Vec<_>>();
    let mut candidate = ElementDefinitionCandidate::new(definitions);
    candidate
        .body_remappings
        .push(ElementTypeRemapping::new(key("Lyrics"), key("Song")));
    candidate.title_remappings.push(ElementTypeRemapping::new(
        key("TitleContent"),
        key("Credit"),
    ));
    let initial_revision = session.revision();

    valid(session.replace_element_definitions_and_remap_types(candidate));

    let changed_body = session.project().elements[body_index].as_element();
    assert_eq!(changed_body.element_type(), key("Song"));
    assert_eq!(changed_body.id, before_body.id);
    assert_eq!(changed_body.text, before_body.text);
    assert_eq!(changed_body.runs, before_body.runs);
    assert_eq!(
        changed_body
            .source_provenance
            .as_ref()
            .and_then(|source| source.foreign_role.as_deref()),
        Some("Song")
    );
    let changed_title = &session.project().title_page[title_index];
    assert_eq!(changed_title.element_type(), Some(key("Credit")));
    assert_eq!(changed_title.id, before_title.id);
    assert_eq!(changed_title.text, before_title.text);
    assert_eq!(changed_title.runs, before_title.runs);
    assert_eq!(
        session.project().title_page[typeless_title_index],
        before_typeless
    );
    assert_eq!(session.selection(), body_selection);
    assert_eq!(session.title_selection(), title_selection);
    assert_eq!(session.body_typing_style(), before_body_typing);
    assert_eq!(session.title_typing_style(), before_title_typing);
    assert_eq!(
        session.project().direct_paragraph_overrides,
        before_direct_overrides
    );
    assert_eq!(
        session.project().dual_dialogue_paragraph_overrides,
        before_dual_overrides
    );
    assert_eq!(session.revision(), initial_revision + 1);
    assert!(session.dirty());
    assert!(!session.can_redo());
    assert!(session.pending_derived_work_summary().complete);
    assert_eq!(
        unqualified_type(retained_by_id(session.project(), "open-body-lyrics")),
        Some("Song")
    );
    assert_eq!(
        unqualified_type(retained_by_id(session.project(), "open-title-content")),
        Some("Credit")
    );
    assert_eq!(
        retained_by_id(session.project(), "open-body-lyrics").children,
        before_body_children
    );
    assert_eq!(
        retained_by_id(session.project(), "open-title-direct"),
        &opaque_direct_title
    );
    assert_eq!(
        retained_by_id(session.project(), "open-header-title"),
        &opaque_header
    );
    assert_eq!(
        retained_by_id(session.project(), "open-footer-title"),
        &opaque_footer
    );
    assert_eq!(
        retained_by_id(session.project(), "open-title-opaque"),
        &opaque_wrapper_title
    );
    assert_eq!(
        session
            .project()
            .preservation
            .as_ref()
            .map(|preservation| preservation.source.original_bytes.clone()),
        Some(original_bytes)
    );
    assert_eq!(
        session
            .project()
            .element_definitions()
            .iter()
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>(),
        target_order.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(winning_settings_order(session.project()), target_order);
    assert_ne!(target_order, original_order);

    let changed = session.clone();
    let revision = session.revision();
    let no_op = ElementDefinitionCandidate::new(
        session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect(),
    );
    valid(session.replace_element_definitions_and_remap_types(no_op));
    assert_eq!(session, changed);
    assert_eq!(session.revision(), revision);

    valid(session.undo());
    assert_eq!(
        session.project().elements[body_index]
            .element_type()
            .as_str(),
        "Lyrics"
    );
    assert_eq!(
        session.project().title_page[title_index]
            .element_type()
            .as_ref()
            .map(ElementTypeKey::as_str),
        Some("TitleContent")
    );
    assert!(!session.dirty());
    valid(session.redo());
    assert_eq!(session.project(), changed.project());
    assert_eq!(session.selection(), body_selection);
    assert_eq!(session.title_selection(), title_selection);
    assert!(session.dirty());
}

#[test]
fn definition_candidate_rejects_incomplete_and_ambiguous_requests_atomically() {
    let mut session = valid(EditingSession::new(valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    )));
    let baseline = session.clone();

    let missing_target = session
        .project()
        .element_definitions()
        .iter()
        .filter(|definition| definition.key.as_str() != "Scene Heading")
        .cloned()
        .collect::<Vec<_>>();
    let missing = ElementDefinitionCandidate::new(missing_target);
    assert_eq!(
        session.replace_element_definitions_and_remap_types(missing),
        Err(EditError::InvalidDefinitionCandidate)
    );
    assert_eq!(session, baseline);

    let mut duplicate = ElementDefinitionCandidate::new(
        session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect(),
    );
    duplicate.body_remappings.extend([
        ElementTypeRemapping::new(key("Scene Heading"), key("Action")),
        ElementTypeRemapping::new(key("Scene Heading"), key("Dialogue")),
    ]);
    assert_eq!(
        session.replace_element_definitions_and_remap_types(duplicate),
        Err(EditError::InvalidDefinitionCandidate)
    );
    assert_eq!(session, baseline);

    let mut source_replacement = ElementDefinitionCandidate::new(
        session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect(),
    );
    source_replacement.retained_settings_replacement = Some(Vec::new());
    assert_eq!(
        session.replace_element_definitions_and_remap_types(source_replacement),
        Err(EditError::InvalidDefinitionCandidate)
    );
    assert_eq!(session, baseline);
}

#[test]
fn definition_replacement_deactivates_dual_projection_without_unwrapping() {
    let mut project = valid(
        scriptcraft_fdx::instantiate_builtin_template(scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID)
            .map(|document| document.project),
    );
    project.preservation = None;
    let cue_left = valid(ScreenplayElement::new(
        "cue-left",
        scriptcraft_model::ElementTypeKey::new("Character").expect("valid fixture label"),
        "LEFT",
    ));
    let dialogue_left = valid(ScreenplayElement::new(
        "dialogue-left",
        scriptcraft_model::ElementTypeKey::new("Dialogue").expect("valid fixture label"),
        "left",
    ));
    let cue_right = valid(ScreenplayElement::new(
        "cue-right",
        scriptcraft_model::ElementTypeKey::new("Character").expect("valid fixture label"),
        "RIGHT",
    ));
    let dialogue_right = valid(ScreenplayElement::new(
        "dialogue-right",
        scriptcraft_model::ElementTypeKey::new("Dialogue").expect("valid fixture label"),
        "right",
    ));
    project.elements = vec![
        BodyOwner::shared(cue_left),
        BodyOwner::shared(dialogue_left),
        BodyOwner::shared(cue_right),
        BodyOwner::shared(dialogue_right),
    ];
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: valid(StableId::new("definition-dual")),
        members: ["cue-left", "dialogue-left", "cue-right", "dialogue-right"]
            .into_iter()
            .map(|id| valid(StableId::new(id)))
            .collect(),
        wrapper: Default::default(),
    });
    assert_eq!(project.validate(), Ok(()));
    let before = project.clone();
    let mut session = valid(EditingSession::new(project));
    let mut definitions = session
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    rename_definition(&mut definitions, "Character", "SpeakerCue");
    let mut candidate = ElementDefinitionCandidate::new(definitions);
    candidate.body_remappings.push(ElementTypeRemapping::new(
        key("Character"),
        key("SpeakerCue"),
    ));

    valid(session.replace_element_definitions_and_remap_types(candidate));

    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(
        session.project().dual_dialogue_groups[0].members,
        before.dual_dialogue_groups[0].members
    );
    assert!(
        scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(
            session.project(),
            &session.project().dual_dialogue_groups[0]
        )
        .is_none()
    );
    assert_eq!(
        session.project().elements[0].element_type(),
        key("SpeakerCue")
    );
    valid(session.undo());
    assert_eq!(session.project(), &before);
    assert!(
        scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(
            session.project(),
            &session.project().dual_dialogue_groups[0]
        )
        .is_some()
    );
    valid(session.redo());
    assert_eq!(
        session.project().elements[2].element_type(),
        key("SpeakerCue")
    );
}

#[test]
fn source_backed_candidate_accepts_ordered_retained_settings_replacement() {
    let mut project = valid(read_v6(OPEN_FIXTURE)).project;
    valid(reconcile_project_element_settings(&mut project));
    let definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let settings = project
        .preservation
        .as_ref()
        .and_then(|preservation| {
            preservation.sections.iter().find_map(|section| {
                section.children.iter().find_map(|node| match node {
                    PreservedNode::Element(element) => find_document_root(element),
                    _ => None,
                })
            })
        })
        .map(|root| {
            let mut winners = Vec::new();
            for definition in &definitions {
                let winner = root.children.iter().find_map(|node| match node {
                    PreservedNode::Element(element)
                        if element.name.local_name == "ElementSettings"
                            && unqualified_type(element) == Some(definition.key.as_str()) =>
                    {
                        Some(element.clone())
                    }
                    _ => None,
                });
                if let Some(mut winner) = winner {
                    winner.anchor = None;
                    winners.push(winner);
                }
            }
            winners
        })
        .unwrap_or_default();
    assert_eq!(settings.len(), definitions.len());

    let mut session = valid(EditingSession::new(project));
    let original_bytes = session
        .project()
        .preservation
        .as_ref()
        .map(|preservation| preservation.source.original_bytes.clone())
        .unwrap_or_default();
    let mut candidate = ElementDefinitionCandidate::new(definitions);
    candidate.retained_settings_replacement = Some(settings);

    valid(session.replace_element_definitions_and_remap_types(candidate));

    assert_eq!(
        session.project().element_definitions().len(),
        session
            .project()
            .preservation
            .as_ref()
            .map(|preservation| preservation
                .anchors
                .iter()
                .filter(|anchor| {
                    matches!(
                        anchor.target,
                        scriptcraft_model::preservation::AnchorTarget::ElementDefinition(_)
                    )
                })
                .count())
            .unwrap_or_default()
    );
    assert_eq!(
        session
            .project()
            .preservation
            .as_ref()
            .map(|preservation| preservation.source.original_bytes.clone()),
        Some(original_bytes)
    );
    assert!(session.project().validate().is_ok());

    let baseline = session.clone();
    let mut invalid = ElementDefinitionCandidate::new(
        session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect(),
    );
    invalid.retained_settings_replacement = Some(Vec::new());
    assert_eq!(
        session.replace_element_definitions_and_remap_types(invalid),
        Err(EditError::InvalidDefinitionCandidate)
    );
    assert_eq!(session, baseline);
}
