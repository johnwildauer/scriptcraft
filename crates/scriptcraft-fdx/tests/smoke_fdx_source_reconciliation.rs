#![forbid(unsafe_code)]

use scriptcraft_fdx::{read_v6, write_v6};
use scriptcraft_model::dual_dialogue::active_dual_dialogue_projection;
use scriptcraft_model::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_model::{BodyOwner, ElementTypeKey, Project, ScreenplayElement};
use std::collections::BTreeSet;

const SOURCE_FIXTURE: &[u8] = include_bytes!("fixtures/source_reconciliation.fdx");

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| {
        panic!("source-reconciliation regression fixture is valid: {error:?}")
    })
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

fn child<'a>(element: &'a PreservedElement, local_name: &str) -> Option<&'a PreservedElement> {
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(candidate)
            if candidate.name.namespace_uri == element.name.namespace_uri
                && candidate.name.local_name == local_name =>
        {
            Some(candidate)
        }
        _ => None,
    })
}

fn child_local<'a>(
    element: &'a PreservedElement,
    local_name: &str,
) -> Option<&'a PreservedElement> {
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(candidate) if candidate.name.local_name == local_name => {
            Some(candidate)
        }
        _ => None,
    })
}

fn find_id<'a>(element: &'a PreservedElement, foreign_id: &str) -> Option<&'a PreservedElement> {
    if attribute(element, "id") == Some(foreign_id) {
        return Some(element);
    }
    element.children.iter().find_map(|node| match node {
        PreservedNode::Element(candidate) => find_id(candidate, foreign_id),
        _ => None,
    })
}

fn root(project: &Project) -> &PreservedElement {
    project
        .preservation
        .as_ref()
        .and_then(|preservation| {
            preservation.sections.iter().find_map(|section| {
                section.children.iter().find_map(|node| match node {
                    PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => {
                        Some(element)
                    }
                    _ => None,
                })
            })
        })
        .unwrap_or_else(|| unreachable!("fixture source root remains retained"))
}

fn source_by_id<'a>(project: &'a Project, foreign_id: &str) -> &'a PreservedElement {
    find_id(root(project), foreign_id)
        .unwrap_or_else(|| unreachable!("fixture foreign identity remains retained"))
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
fn source_reconciliation_fixture_roundtrips_managed_and_opaque_source() {
    let imported = valid(read_v6(SOURCE_FIXTURE));
    let project = imported.project;
    let original_bytes = project
        .preservation
        .as_ref()
        .map(|preservation| preservation.source.original_bytes.clone());

    assert_eq!(project.validate(), Ok(()));
    assert!(project
        .element_definitions()
        .get_by_label("Lyrics")
        .is_some());
    assert!(project
        .element_definitions()
        .get_by_label("lyrics")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("DirectOnlyTitle")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("HeaderTitle")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("FooterTitle")
        .is_none());
    assert!(project
        .element_definitions()
        .get_by_label("WrappedTitle")
        .is_none());
    assert_eq!(
        project
            .element_definitions()
            .iter()
            .take(3)
            .map(|definition| definition.key.as_str())
            .collect::<Vec<_>>(),
        vec!["Lyrics", "action", "Action"]
    );
    assert_eq!(project.title_page.len(), 2);

    let body = body_by_foreign_id(&project, "reconcile-body-lyrics")
        .unwrap_or_else(|| unreachable!("typed direct body paragraph is managed"));
    assert_eq!(body.element_type().as_str(), "Lyrics");
    assert_eq!(body.text, "First line");
    assert_eq!(body.runs.len(), 2);
    assert!(project.direct_paragraph_overrides.contains_key(&body.id));
    let title = title_by_foreign_id(&project, "reconcile-title-content")
        .unwrap_or_else(|| unreachable!("TitlePage Content title is managed"));
    assert_eq!(
        title.element_type.as_ref().map(ElementTypeKey::as_str),
        Some("titlefirst")
    );
    assert_eq!(title.runs.len(), 2);
    assert!(title_by_foreign_id(&project, "reconcile-title-typeless")
        .is_some_and(|paragraph| paragraph.element_type.is_none()));
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_some());

    for foreign_id in [
        "reconcile-title-direct",
        "reconcile-title-wrapped",
        "reconcile-header-title",
        "reconcile-footer-title",
    ] {
        assert!(matches!(
            &source_by_id(&project, foreign_id).ownership,
            SourceOwnership::Opaque
        ));
        assert!(source_by_id(&project, foreign_id).anchor.is_none());
    }
    let body_source = source_by_id(&project, "reconcile-body-lyrics");
    assert!(child(body_source, "SceneProperties").is_some());
    assert!(child_local(body_source, "BodyOpaque").is_some());
    assert!(child(body_source, "UserParagraphData").is_some());
    let title_source = source_by_id(&project, "reconcile-title-content");
    assert!(child(title_source, "UserParagraphData").is_some());
    assert!(child_local(title_source, "TitleOpaque").is_some());
    let settings_winner = root(&project)
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && attribute(element, "Type") == Some("Lyrics") =>
            {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("winning Lyrics settings remain available"));
    assert_eq!(attribute(settings_winner, "VendorSetting"), Some("keep"));
    assert!(child_local(settings_winner, "UnknownDefinition").is_some());
    let duplicate_settings = root(&project)
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && attribute(element, "Type") == Some("Lyrics") =>
            {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(duplicate_settings.len(), 2);
    assert!(duplicate_settings[0].anchor.is_some());
    assert!(duplicate_settings[1].anchor.is_none());
    assert_eq!(
        attribute(duplicate_settings[1], "VendorSetting"),
        Some("inert duplicate")
    );
    assert!(child_local(duplicate_settings[1], "DuplicatePayload").is_some());

    let output = valid(write_v6(&project));
    assert_eq!(
        project
            .preservation
            .as_ref()
            .map(|preservation| preservation.source.original_bytes.clone()),
        original_bytes,
        "writing must not mutate the imported byte snapshot"
    );
    let reopened = valid(read_v6(&output)).project;
    assert_eq!(
        reopened.element_definitions(),
        project.element_definitions()
    );
    assert_eq!(reopened.elements.len(), project.elements.len());
    assert_eq!(reopened.title_page.len(), project.title_page.len());
    let reopened_body = body_by_foreign_id(&reopened, "reconcile-body-lyrics")
        .unwrap_or_else(|| unreachable!("reopened body retains its foreign identity"));
    assert_eq!(reopened_body.id, body.id);
    assert_eq!(reopened_body.element_type(), body.element_type());
    assert_eq!(reopened_body.text, body.text);
    assert_eq!(reopened_body.runs, body.runs);
    assert!(child(
        source_by_id(&reopened, "reconcile-body-lyrics"),
        "SceneProperties"
    )
    .is_some());
    assert!(reopened
        .direct_paragraph_overrides
        .contains_key(&reopened_body.id));
    let reopened_title = title_by_foreign_id(&reopened, "reconcile-title-content")
        .unwrap_or_else(|| unreachable!("reopened Content title retains its foreign identity"));
    assert_eq!(reopened_title.id, title.id);
    assert_eq!(reopened_title.element_type, title.element_type);
    assert_eq!(reopened_title.text, title.text);
    assert_eq!(reopened_title.runs, title.runs);
    assert!(child_local(
        source_by_id(&reopened, "reconcile-title-content"),
        "TitleOpaque"
    )
    .is_some());
    assert!(
        active_dual_dialogue_projection(&reopened, &reopened.dual_dialogue_groups[0]).is_some()
    );
    assert_eq!(
        &source_by_id(&reopened, "reconcile-title-direct").ownership,
        &SourceOwnership::Opaque
    );
    assert_eq!(
        attribute(source_by_id(&reopened, "reconcile-title-direct"), "Type"),
        Some("DirectOnlyTitle")
    );
    assert_eq!(
        attribute(source_by_id(&reopened, "reconcile-header-title"), "Type"),
        Some("HeaderTitle")
    );
    assert_eq!(
        attribute(source_by_id(&reopened, "reconcile-footer-title"), "Type"),
        Some("FooterTitle")
    );
    assert!(child_local(
        source_by_id(&reopened, "reconcile-body-lyrics"),
        "BodyOpaque"
    )
    .is_some());
    let reopened_lyrics_settings = root(&reopened)
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && attribute(element, "Type") == Some("Lyrics") =>
            {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(reopened_lyrics_settings.len(), 2);
    assert!(reopened_lyrics_settings[1].anchor.is_none());
    assert!(child_local(reopened_lyrics_settings[1], "DuplicatePayload").is_some());
    assert!(child_local(reopened_lyrics_settings[0], "UnknownDefinition").is_some());
    assert!(source_by_id(&reopened, "reconcile-dual-wrapper")
        .anchor
        .is_some());
    assert!(source_by_id(&reopened, "reconcile-dual-left-cue")
        .anchor
        .is_some());
    assert!(source_by_id(&reopened, "reconcile-dual-right-line")
        .anchor
        .is_some());
}

#[test]
fn preservation_free_scene_metadata_uses_only_exact_capabilities() {
    let template = valid(scriptcraft_fdx::instantiate_builtin_template(
        scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
    ));
    let definitions = template
        .project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut project = valid(Project::empty_projection());
    valid(project.replace_element_definitions(definitions.clone()));
    let mut definitions = definitions;
    let mut lyrics = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("base Action definition is present"));
    lyrics.key = valid(ElementTypeKey::new("Lyrics"));
    lyrics.behavior.paginate_as = Some("Character".to_owned());
    definitions.push(lyrics);
    valid(project.replace_element_definitions(definitions));

    let element = |id: &str, element_type: &str, text: &str| {
        BodyOwner::shared(valid(ScreenplayElement::new(
            id,
            valid(ElementTypeKey::new(element_type)),
            text,
        )))
    };
    project.elements = vec![
        element("generated-scene-1", "Scene Heading", "INT. ROOM"),
        element("misleading-scene", "Shot", "EXT. NOT A SCENE"),
        element("misleading-cue", "Lyrics", "NOT A CHARACTER"),
        element("generated-cue", "Character", "  Mira (V.O.)  "),
        element("generated-scene-2", "Scene Heading", "INT. HALL"),
    ];

    let output = valid(write_v6(&project));
    let reopened = valid(read_v6(&output)).project;
    let first_heading = reopened
        .elements
        .iter()
        .find(|element| {
            element.element_type().as_str() == "Scene Heading" && element.text == "INT. ROOM"
        })
        .unwrap_or_else(|| unreachable!("first exact scene heading remains managed"));
    let second_heading = reopened
        .elements
        .iter()
        .find(|element| {
            element.element_type().as_str() == "Scene Heading" && element.text == "INT. HALL"
        })
        .unwrap_or_else(|| unreachable!("second exact scene heading remains managed"));
    let misleading = reopened
        .elements
        .iter()
        .find(|element| {
            element.element_type().as_str() == "Shot" && element.text == "EXT. NOT A SCENE"
        })
        .unwrap_or_else(|| unreachable!("arbitrary pagination type remains managed"));

    assert_eq!(
        scriptcraft_model::scene_metadata::scene_arc_names(&reopened, &first_heading.id),
        Ok(vec!["MIRA".to_owned()])
    );
    assert_eq!(
        scriptcraft_model::scene_metadata::scene_arc_names(&reopened, &second_heading.id),
        Ok(Vec::new())
    );
    let preservation = reopened
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("reread FDX has retained source"));
    let misleading_source = preservation
        .managed_element(&misleading.id)
        .unwrap_or_else(|error| panic!("managed source lookup succeeds: {error:?}"))
        .unwrap_or_else(|| unreachable!("Shot remains a managed owner"));
    assert!(child(misleading_source, "SceneProperties").is_none());
}

#[test]
fn scene_metadata_stays_opaque_while_scene_capability_is_removed() {
    let mut project = valid(read_v6(SOURCE_FIXTURE)).project;
    let heading_index = project
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("reconcile-body-lyrics")
        })
        .unwrap_or_else(|| unreachable!("fixture has its Lyrics paragraph with opaque metadata"));
    let heading_id = project.elements[heading_index].id.clone();
    let original_key = project.elements[heading_index].element_type();
    let scene_heading_key = valid(ElementTypeKey::new("Scene Heading"));
    let lyrics_key = valid(ElementTypeKey::new("Lyrics"));
    project
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("fixture has preservation state"))
        .change_managed_element_type(&heading_id, &original_key, &scene_heading_key)
        .unwrap_or_else(|error| panic!("exact source Type update is supported: {error:?}"));
    BodyOwner::update(&mut project.elements[heading_index], |element| {
        element.set_element_type(scene_heading_key.clone());
    });
    let original_heading_source = project
        .preservation
        .as_ref()
        .and_then(|state| state.managed_element(&heading_id).ok().flatten())
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture heading has retained source"));
    let original_properties = child(&original_heading_source, "SceneProperties")
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture heading has scene metadata"));

    project
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("fixture has preservation state"))
        .change_managed_element_type(&heading_id, &scene_heading_key, &lyrics_key)
        .unwrap_or_else(|error| panic!("exact source Type update is supported: {error:?}"));
    BodyOwner::update(&mut project.elements[heading_index], |element| {
        element.set_element_type(lyrics_key.clone());
    });
    assert!(scriptcraft_model::scene_metadata::scene_arc_names(&project, &heading_id).is_err());

    let _ = scriptcraft_model::scene_metadata::reconcile_project_candidate_with_scope(
        &mut project,
        None,
    )
    .unwrap_or_else(|error| panic!("unrelated supported scenes reconcile safely: {error:?}"));
    let still_opaque = project
        .preservation
        .as_ref()
        .and_then(|state| state.managed_element(&heading_id).ok().flatten())
        .and_then(|heading| child(heading, "SceneProperties"))
        .unwrap_or_else(|| unreachable!("removed scene capability does not remove its attachment"));
    assert_eq!(still_opaque, &original_properties);

    project
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("fixture has preservation state"))
        .change_managed_element_type(&heading_id, &lyrics_key, &scene_heading_key)
        .unwrap_or_else(|error| panic!("restoring exact source Type is supported: {error:?}"));
    BodyOwner::update(&mut project.elements[heading_index], |element| {
        element.set_element_type(scene_heading_key);
    });
    let requested = BTreeSet::from([heading_id.as_str().to_owned()]);
    let outcome = scriptcraft_model::scene_metadata::reconcile_project_candidate_with_scope(
        &mut project,
        Some(&requested),
    )
    .unwrap_or_else(|error| panic!("restored scene capability is eligible for refresh: {error:?}"));
    assert_eq!(outcome.scene_candidates, 1);
}
