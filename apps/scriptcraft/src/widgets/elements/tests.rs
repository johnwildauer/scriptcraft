#![allow(clippy::unwrap_used)]

use super::{state::TemplateSelectorMode, summary::element_definition_summaries};
use crate::{
    app::ScriptCraftApp,
    chrome::shortcuts::ShortcutPlatform,
    commands::{CommandInvocation, DesktopCommandOutcome},
};
use eframe::egui;
use scriptcraft_commands::{
    BuiltinTemplateValue, Command, CommandOrigin, CommandOutcome, EditorSurface,
};
use scriptcraft_fdx::BuiltinTemplateId;
use scriptcraft_model::{ElementTypeKey, Project, ScreenplayElement};
use scriptcraft_session::editing::EditingSession;

const ELEMENTS_UI_WORKFLOW_FDX: &[u8] =
    include_bytes!("../../../tests/fixtures/elements_ui_workflow.fdx");

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("prepared Elements UI fixture failed: {error:?}"))
}

fn fixture_project() -> Project {
    valid(scriptcraft_fdx::read_v6(ELEMENTS_UI_WORKFLOW_FDX)).project
}

fn app_with_fixture() -> ScriptCraftApp {
    let mut app = valid(ScriptCraftApp::new_for_template_lifecycle_tests());
    app.session = valid(EditingSession::new(fixture_project()));
    app
}

fn body_element<'a>(project: &'a Project, foreign_id: &str) -> &'a ScreenplayElement {
    project
        .elements
        .iter()
        .map(|owner| owner.as_element())
        .find(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some(foreign_id)
        })
        .unwrap_or_else(|| panic!("fixture body owner {foreign_id} is present"))
}

fn title_element<'a>(
    project: &'a Project,
    foreign_id: &str,
) -> &'a scriptcraft_model::TitlePageParagraph {
    project
        .title_page
        .iter()
        .find(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some(foreign_id)
        })
        .unwrap_or_else(|| panic!("fixture title owner {foreign_id} is present"))
}

fn unrelated_root_nodes(project: &Project) -> Vec<scriptcraft_model::preservation::PreservedNode> {
    let preservation = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| panic!("the UI workflow fixture retains FDX source"));
    let root = preservation
        .sections
        .iter()
        .flat_map(|section| section.children.iter())
        .find_map(|node| match node {
            scriptcraft_model::preservation::PreservedNode::Element(element)
                if element.name.local_name == "FinalDraft" =>
            {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("the fixture has a FinalDraft root"));
    root.children
        .iter()
        .filter(|node| match node {
            scriptcraft_model::preservation::PreservedNode::Element(element) => !matches!(
                element.name.local_name.as_str(),
                "ElementSettings" | "Content" | "TitlePage"
            ),
            scriptcraft_model::preservation::PreservedNode::Text(_) => false,
            scriptcraft_model::preservation::PreservedNode::CData(_)
            | scriptcraft_model::preservation::PreservedNode::Comment(_)
            | scriptcraft_model::preservation::PreservedNode::ProcessingInstruction { .. } => true,
        })
        .cloned()
        .collect()
}

fn preserved_marker_exists(
    nodes: &[scriptcraft_model::preservation::PreservedNode],
    marker: &str,
) -> bool {
    nodes.iter().any(|node| match node {
        scriptcraft_model::preservation::PreservedNode::Element(element) => {
            element
                .attributes
                .iter()
                .any(|attribute| attribute.value == marker)
                || preserved_marker_exists(&element.children, marker)
        }
        scriptcraft_model::preservation::PreservedNode::Text(_)
        | scriptcraft_model::preservation::PreservedNode::CData(_)
        | scriptcraft_model::preservation::PreservedNode::Comment(_)
        | scriptcraft_model::preservation::PreservedNode::ProcessingInstruction { .. } => false,
    })
}

#[test]
fn compile_time_workflow_fixture_projects_the_live_ordered_registry_and_complete_summaries() {
    let project = fixture_project();
    let keys = project
        .element_definitions()
        .iter()
        .map(|definition| definition.key.as_str())
        .collect::<Vec<_>>();
    assert!(keys.contains(&"Lyrics"));
    assert!(keys.contains(&"lyrics"));
    assert!(keys.contains(&"action"));
    assert!(keys.contains(&"Action"));

    let summaries = element_definition_summaries(&project, ShortcutPlatform::MacOS);
    let lyric = summaries
        .iter()
        .find(|summary| summary.key == "Lyrics")
        .unwrap_or_else(|| panic!("the fixture has a Musical Lyrics definition"));
    assert_eq!(lyric.font.family, "Times New Roman");
    assert_eq!(lyric.font.size, 12);
    assert_eq!(lyric.font.active_flags, vec!["italic".to_owned()]);
    assert_eq!(lyric.font.color, None);
    assert_eq!(lyric.paragraph.alignment, "Left");
    assert_eq!(lyric.paragraph.first_indent, "0");
    assert_eq!(lyric.paragraph.left_indent, "1");
    assert_eq!(lyric.paragraph.right_indent, "7");
    assert_eq!(lyric.paragraph.leading, "Regular");
    assert_eq!(lyric.paragraph.spacing, 1);
    assert_eq!(lyric.paragraph.space_before, "0");
    assert!(!lyric.paragraph.starts_new_page);
    assert_eq!(lyric.behavior.return_target, "Musical Cue");
    assert_eq!(lyric.behavior.tab_target, "Action");
    assert_eq!(lyric.behavior.shortcut, None);
    assert_eq!(lyric.behavior.paginate_as.as_deref(), Some("General"));
    assert_eq!(lyric.outline, Some("level 1".to_owned()));

    let cue = summaries
        .iter()
        .find(|summary| summary.key == "Musical Cue")
        .unwrap_or_else(|| panic!("the fixture has its current-only cue definition"));
    assert_eq!(cue.font.family, "Times New Roman");
    assert_eq!(cue.font.size, 13);
    assert_eq!(cue.font.active_flags, vec!["bold".to_owned()]);
    assert_eq!(cue.font.color.as_deref(), Some("#123456789ABC"));
    assert_eq!(cue.paragraph.alignment, "Right");
    assert_eq!(cue.paragraph.first_indent, "-1.25");
    assert_eq!(cue.paragraph.left_indent, "2.5");
    assert_eq!(cue.paragraph.right_indent, "3.75");
    assert_eq!(cue.paragraph.space_before, "4.5");
    assert_eq!(cue.paragraph.spacing, 2);
    assert!(cue.paragraph.starts_new_page);
    assert_eq!(
        cue.behavior.return_target,
        "Missing Current Target (unresolved)"
    );
    assert_eq!(cue.behavior.tab_target, "Action");
    assert_eq!(cue.behavior.paginate_as.as_deref(), Some("Action"));
    assert_eq!(cue.behavior.shortcut.as_deref(), Some("Ctrl+M"));
    assert_eq!(cue.outline.as_deref(), Some("level 2, hide supported"));

    assert_eq!(
        element_definition_summaries(&project, ShortcutPlatform::MacOS)
            .iter()
            .map(|summary| summary.key.as_str())
            .collect::<Vec<_>>(),
        keys
    );
}

#[test]
fn selector_cancel_leaves_the_current_session_unchanged() {
    let mut app = app_with_fixture();
    let original = app.session.project().clone();
    let revision = app.session.revision();
    let token = app.session.mutation_token();
    let dirty = app.session.dirty();
    let ctx = egui::Context::default();

    app.element_windows.open_elements(EditorSurface::Body);
    app.element_windows
        .open_selector(TemplateSelectorMode::ApplyToCurrent);
    assert!(app
        .element_windows
        .select_template(BuiltinTemplateId::DramatistsGuildMusical));
    let result = app.dispatch_command(
        CommandInvocation::new(
            Command::FormatTemplateSelectorCancel,
            CommandOrigin::InternalFollowUp,
        ),
        &ctx,
    );
    assert_eq!(result.outcome, DesktopCommandOutcome::EffectDispatched);
    assert!(app.element_windows.elements_open());
    assert!(!app.element_windows.selector_open());
    assert_eq!(app.session.project(), &original);
    assert_eq!(app.session.revision(), revision);
    assert_eq!(app.session.mutation_token(), token);
    assert_eq!(app.session.dirty(), dirty);
    assert!(app.native_path.is_none());
    assert!(app.fdx_baseline.is_none());
}

#[test]
fn selector_apply_routes_musical_lyrics_through_the_real_executor_and_one_session_candidate() {
    let mut app = app_with_fixture();
    let before = app.session.project().clone();
    let before_body = before
        .elements
        .iter()
        .map(|owner| owner.as_element().clone())
        .collect::<Vec<_>>();
    assert_eq!(before_body.len(), before.elements.len());
    let before_title = before.title_page.clone();
    let before_unrelated_root_nodes = unrelated_root_nodes(&before);
    let before_body_overrides = before.direct_paragraph_overrides.clone();
    let before_wrapper_overrides = before.dual_dialogue_paragraph_overrides.clone();
    let before_dual_groups = before.dual_dialogue_groups.clone();
    let revision = app.session.revision();
    let token = app.session.mutation_token();
    let ctx = egui::Context::default();
    app.rendered_revision = Some(revision);
    app.title_view.rendered_revision = Some(revision);
    app.page_scale = Some(1.0);
    app.title_view.page_scale = Some(1.0);

    app.element_windows.open_elements(EditorSurface::Body);
    app.element_windows
        .open_selector(TemplateSelectorMode::ApplyToCurrent);
    assert!(app
        .element_windows
        .select_template(BuiltinTemplateId::Screenplay));
    let result = app.dispatch_command(
        CommandInvocation::new(
            Command::FormatTemplateApply {
                template: BuiltinTemplateValue::Screenplay,
            },
            CommandOrigin::InternalFollowUp,
        ),
        &ctx,
    );

    assert_eq!(
        result.outcome,
        DesktopCommandOutcome::Editor(CommandOutcome::Changed)
    );
    assert_eq!(app.session.revision(), revision + 1);
    assert_eq!(app.session.mutation_token(), token + 1);
    assert!(app.session.dirty());
    assert!(app.session.can_undo());
    assert!(!app.session.can_redo());
    assert!(!app.element_windows.selector_open());
    assert!(app.element_windows.elements_open());
    assert!(app.native_path.is_none());
    assert!(app.fdx_baseline.is_none());
    assert!(app.prepared_document_fonts.is_none());
    assert_eq!(app.rendered_revision, None);
    assert_eq!(app.title_view.rendered_revision, None);
    assert_eq!(app.page_scale, None);
    assert_eq!(app.title_view.page_scale, None);

    let after_project = app.session.project().clone();
    let after = &after_project;
    assert_eq!(before_body.len(), after.elements.len());
    for (old, new) in before_body
        .iter()
        .zip(after.elements.iter().map(|owner| owner.as_element()))
    {
        assert_eq!(old.id, new.id);
        assert_eq!(old.text, new.text);
        assert_eq!(old.runs, new.runs);
        assert_eq!(
            old.source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref()),
            new.source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
        );
        let expected_type = if matches!(
            old.element_type.as_str(),
            "General"
                | "Scene Heading"
                | "Action"
                | "Character"
                | "Parenthetical"
                | "Dialogue"
                | "Transition"
                | "Shot"
                | "Cast List"
                | "New Act"
                | "Sequence"
                | "End of Act"
                | "Summary"
                | "Outline 1"
                | "Outline 2"
                | "Outline 3"
                | "Note"
        ) {
            old.element_type.clone()
        } else {
            ElementTypeKey::new("Action").expect("the built-in Action key is valid")
        };
        assert_eq!(new.element_type, expected_type);
    }
    assert_eq!(
        body_element(after, "apply-body-lyrics")
            .element_type
            .as_str(),
        "Action"
    );
    assert_eq!(
        body_element(after, "apply-body-lower-action")
            .element_type
            .as_str(),
        "Action"
    );
    assert_eq!(
        body_element(after, "apply-body-action")
            .element_type
            .as_str(),
        "Action"
    );
    assert_eq!(
        body_element(after, "apply-body-lower-lyrics")
            .element_type
            .as_str(),
        "Action"
    );
    assert_eq!(
        body_element(after, "apply-body-custom")
            .element_type
            .as_str(),
        "Action"
    );

    assert_eq!(before_title.len(), after.title_page.len());
    for (old, new) in before_title.iter().zip(&after.title_page) {
        assert_eq!(old.id, new.id);
        assert_eq!(old.text, new.text);
        assert_eq!(old.runs, new.runs);
        assert_eq!(old.formatting, new.formatting);
        let expected_type = old.element_type.as_ref().map(|key| {
            if matches!(key.as_str(), "Character") {
                key.clone()
            } else {
                ElementTypeKey::new("Action").expect("the built-in Action key is valid")
            }
        });
        assert_eq!(new.element_type, expected_type);
    }
    assert_eq!(
        title_element(after, "apply-title-typeless").element_type,
        None
    );
    assert_eq!(
        title_element(after, "apply-title-character")
            .element_type
            .as_ref()
            .map(ElementTypeKey::as_str),
        Some("Character")
    );
    assert_eq!(after.direct_paragraph_overrides, before_body_overrides);
    assert_eq!(
        after.dual_dialogue_paragraph_overrides,
        before_wrapper_overrides
    );
    assert_eq!(after.dual_dialogue_groups, before_dual_groups);
    assert_eq!(unrelated_root_nodes(after), before_unrelated_root_nodes);
    assert_eq!(
        after
            .preservation
            .as_ref()
            .expect("source-backed Apply keeps preservation")
            .source,
        before
            .preservation
            .as_ref()
            .expect("fixture begins source-backed")
            .source
    );
    let after_source = &after
        .preservation
        .as_ref()
        .expect("source-backed Apply keeps preservation")
        .sections;
    for marker in [
        "retain-body-attachment",
        "retain-body-opaque",
        "retain-title-attachment",
        "retain-title-opaque",
        "retain-typeless",
        "opaque-current-root",
    ] {
        assert!(preserved_marker_exists(
            &after_source
                .iter()
                .flat_map(|section| section.children.iter())
                .cloned()
                .collect::<Vec<_>>(),
            marker
        ));
    }

    let current_rows = element_definition_summaries(after, ShortcutPlatform::MacOS);
    let current_keys = after
        .element_definitions()
        .iter()
        .map(|definition| definition.key.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        current_rows
            .iter()
            .map(|summary| summary.key.clone())
            .collect::<Vec<_>>(),
        current_keys
    );
    assert!(!current_keys.iter().any(|key| key == "Lyrics"));
    assert!(!current_keys.iter().any(|key| key == "Musical Cue"));
    let refreshed_controls = app.refresh_dynamic_controls();
    assert_eq!(
        refreshed_controls
            .choices
            .iter()
            .map(|choice| choice.key.as_str().to_owned())
            .collect::<Vec<_>>(),
        current_keys
    );

    let applied = after_project.clone();
    valid(app.session.undo());
    assert_eq!(app.session.project(), &before);
    assert!(!app.session.dirty());
    assert_eq!(
        app.refresh_dynamic_controls()
            .choices
            .iter()
            .map(|choice| choice.key.as_str().to_owned())
            .collect::<Vec<_>>(),
        before
            .element_definitions()
            .iter()
            .map(|definition| definition.key.as_str().to_owned())
            .collect::<Vec<_>>()
    );
    valid(app.session.redo());
    assert_eq!(app.session.project(), &applied);
    assert!(app.session.dirty());
}
