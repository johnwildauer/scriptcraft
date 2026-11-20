//! Settings-only Apply Template as one ordinary session candidate.

use super::template_plan::plan_element_template_application;
use super::EditingSession;
use scriptcraft_fdx::{
    extract_element_template_settings, instantiate_builtin_template, BuiltinTemplateId,
    ElementTemplateSettings,
};
use std::fmt;

/// The publication result of one Apply Template attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyTemplateOutcome {
    /// The generic definition candidate changed and was published once.
    Changed,
    /// The complete candidate was equal to the current session state.
    NoOp,
}

/// Bounded, privacy-safe failure categories for built-in and lower-level Apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyTemplateError {
    /// The selected built-in could not be loaded through the common reader.
    TargetLoadFailed,
    /// The loaded built-in could not be reduced to bounded template settings.
    TargetExtractionFailed,
    /// The current and target definition/type projections could not be planned.
    PlanningFailed,
    /// The detached generic candidate failed source projection or validation.
    CandidateRejected,
}

impl fmt::Display for ApplyTemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TargetLoadFailed => "built-in template loading failed",
            Self::TargetExtractionFailed => "built-in template extraction failed",
            Self::PlanningFailed => "template application planning failed",
            Self::CandidateRejected => "template application candidate was rejected",
        })
    }
}

impl std::error::Error for ApplyTemplateError {}

impl EditingSession {
    /// Loads and applies one built-in template through the common FDX loader
    /// and settings-only extraction path. The full target document is dropped
    /// before the current session is planned or mutated.
    pub fn apply_builtin_template(
        &mut self,
        id: BuiltinTemplateId,
    ) -> Result<ApplyTemplateOutcome, ApplyTemplateError> {
        let template = {
            let document = instantiate_builtin_template(id)
                .map_err(|_| ApplyTemplateError::TargetLoadFailed)?;
            extract_element_template_settings(&document)
                .map_err(|_| ApplyTemplateError::TargetExtractionFailed)?
        };

        self.apply_element_template_settings(&template)
    }

    /// Applies detached template settings through one ordinary undoable session
    /// candidate. The current clean baseline, selections, storage target, and
    /// filesystem are not replaced or otherwise changed by this operation.
    pub fn apply_element_template_settings(
        &mut self,
        template: &ElementTemplateSettings,
    ) -> Result<ApplyTemplateOutcome, ApplyTemplateError> {
        let mut candidate =
            plan_element_template_application(&self.project, template.definitions())
                .map_err(|_| ApplyTemplateError::PlanningFailed)?;

        if self.project.preservation.is_some() {
            candidate.retained_settings_replacement = Some(template.retained_settings().to_vec());
        }

        let previous_mutation_token = self.mutation_token;
        self.replace_element_definitions_and_remap_types(candidate)
            .map_err(|_| ApplyTemplateError::CandidateRejected)?;

        if self.mutation_token == previous_mutation_token {
            Ok(ApplyTemplateOutcome::NoOp)
        } else {
            Ok(ApplyTemplateOutcome::Changed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{EditingSession, Position, Selection};
    use scriptcraft_fdx::{extract_element_template_settings, instantiate_builtin_template};
    use scriptcraft_model::{
        element::BodyOwner,
        title_page::{TitlePageParagraph, TitlePageParagraphFormatting},
        DocumentElementDefinitions, ElementTypeKey, Project, ScreenplayElement,
    };

    fn valid<T, E>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|_| unreachable!("Apply Template test input is valid"))
    }

    fn template(id: scriptcraft_fdx::BuiltinTemplateId) -> ElementTemplateSettings {
        let document = valid(instantiate_builtin_template(id));
        valid(extract_element_template_settings(&document))
    }

    fn musical_project() -> Project {
        valid(instantiate_builtin_template(
            scriptcraft_fdx::BuiltinTemplateId::DramatistsGuildMusical,
        ))
        .project
    }

    fn caret(element: usize, offset: usize) -> Selection {
        Selection::caret(Position { element, offset })
    }

    #[test]
    fn clean_apply_is_one_undoable_candidate_and_preserves_selection_and_baseline() {
        let original = musical_project();
        let target = template(scriptcraft_fdx::BuiltinTemplateId::Screenplay);
        let mut session = valid(EditingSession::new(original.clone()));

        // Seed redo without leaving the session dirty, then set distinct surface selections.
        valid(session.set_selection(caret(0, 0)));
        valid(session.insert("temporary history branch"));
        valid(session.undo());
        assert!(session.can_redo());
        let body_selection = caret(0, 0);
        let title_selection = caret(12, 1);
        valid(session.set_selection(body_selection));
        valid(session.set_title_selection(title_selection));

        let revision = session.revision();
        let token = session.mutation_token();
        let clean_baseline = session.clean_project.clone();
        let body_typing_style = session.body_typing_style();
        let title_typing_style = session.title_typing_style();
        let history_len = session.history.len();
        let outcome = valid(session.apply_element_template_settings(&target));

        assert_eq!(outcome, ApplyTemplateOutcome::Changed);
        assert_eq!(session.revision(), revision + 1);
        assert_eq!(session.mutation_token(), token + 1);
        assert_eq!(session.history.len(), history_len + 1);
        assert!(session.dirty());
        assert!(session.can_undo());
        assert!(!session.can_redo());
        assert!(session.pending_derived_work_summary().complete);
        assert_eq!(session.selection(), body_selection);
        assert_eq!(session.title_selection(), title_selection);
        assert_eq!(session.body_typing_style(), body_typing_style);
        assert_eq!(session.title_typing_style(), title_typing_style);
        assert_eq!(session.clean_project, clean_baseline);
        assert_eq!(
            session.project().element_definitions(),
            target.definitions()
        );

        valid(session.undo());
        assert_eq!(session.project(), &original);
        assert!(!session.dirty());
        valid(session.redo());
        assert_eq!(
            session.project().element_definitions(),
            target.definitions()
        );
        assert!(session.dirty());
    }

    #[test]
    fn dirty_apply_is_ordinary_edit_and_undo_restores_the_prior_dirty_state() {
        let target = template(scriptcraft_fdx::BuiltinTemplateId::Screenplay);
        let mut session = valid(EditingSession::new(musical_project()));
        valid(session.set_selection(caret(0, 0)));
        valid(session.insert("existing unsaved edit"));
        let prior_project = session.project().clone();
        let clean_baseline = session.clean_project.clone();
        let revision = session.revision();
        let token = session.mutation_token();
        let history_len = session.history.len();

        assert_eq!(
            valid(session.apply_element_template_settings(&target)),
            ApplyTemplateOutcome::Changed
        );
        assert!(session.dirty());
        assert_eq!(session.revision(), revision + 1);
        assert_eq!(session.mutation_token(), token + 1);
        assert_eq!(session.history.len(), history_len + 1);
        assert_eq!(session.clean_project, clean_baseline);

        valid(session.undo());
        assert_eq!(session.project(), &prior_project);
        assert!(session.dirty());
        assert_eq!(session.clean_project, clean_baseline);
    }

    #[test]
    fn preservation_free_apply_uses_the_same_policy_without_fabricating_source() {
        let target = template(scriptcraft_fdx::BuiltinTemplateId::Screenplay);
        let musical = musical_project();
        let definitions = valid(DocumentElementDefinitions::new(
            musical.element_definitions().iter().cloned(),
        ));
        let mut project = valid(Project::empty_projection());
        valid(project.replace_element_definitions(definitions.iter().cloned().collect()));
        let lyrics = ScreenplayElement::new(
            "source-less-lyrics",
            valid(ElementTypeKey::new("Lyrics")),
            "current lyrics",
        );
        let lyrics = valid(lyrics);
        project.elements.push(BodyOwner::shared(lyrics));
        let mut typed_title = valid(TitlePageParagraph::new(
            "source-less-typed-title",
            "current typed title",
            TitlePageParagraphFormatting::default(),
        ));
        typed_title.element_type = Some(valid(ElementTypeKey::new("Lyrics")));
        project.title_page.push(typed_title);
        let action = valid(ScreenplayElement::new(
            "source-less-exact-action",
            valid(ElementTypeKey::new("Action")),
            "exact current Action",
        ));
        project.elements.push(BodyOwner::shared(action));
        let mut session = valid(EditingSession::new(project));

        assert_eq!(
            valid(session.apply_element_template_settings(&target)),
            ApplyTemplateOutcome::Changed
        );
        assert!(session.project().preservation.is_none());
        assert_eq!(
            session.project().elements[0].element_type().as_str(),
            "Action"
        );
        assert_eq!(session.project().elements[0].text, "current lyrics");
        assert_eq!(
            session.project().title_page[0].element_type(),
            Some(valid(ElementTypeKey::new("Action")))
        );
        assert_eq!(session.project().title_page[0].text, "current typed title");
        assert_eq!(
            session.project().elements[1].element_type().as_str(),
            "Action"
        );
        assert_eq!(session.project().elements[1].text, "exact current Action");
        assert_eq!(
            session.project().element_definitions(),
            target.definitions()
        );
    }

    #[test]
    fn equal_candidate_is_a_true_no_op() {
        let target = template(scriptcraft_fdx::BuiltinTemplateId::Screenplay);
        let mut project = valid(Project::empty_projection());
        valid(project.replace_element_definitions(target.definitions().iter().cloned().collect()));
        let action = ScreenplayElement::new(
            "source-less-action",
            valid(ElementTypeKey::new("Action")),
            "unchanged action",
        );
        let action = valid(action);
        project.elements.push(BodyOwner::shared(action));
        let mut session = valid(EditingSession::new(project));
        let before = session.clone();
        let token = session.mutation_token();

        assert_eq!(
            valid(session.apply_element_template_settings(&target)),
            ApplyTemplateOutcome::NoOp
        );
        assert_eq!(session, before);
        assert_eq!(session.mutation_token(), token);
        assert!(!session.can_undo());
        assert!(!session.can_redo());
    }

    #[test]
    fn planning_and_source_candidate_failures_leave_the_complete_session_unchanged() {
        let valid_target = template(scriptcraft_fdx::BuiltinTemplateId::Screenplay);
        let mut session = valid(EditingSession::new(musical_project()));
        valid(session.set_selection(caret(0, 0)));
        valid(session.insert("dirty state"));
        valid(session.undo());

        let mut invalid_owner = session.project.elements[0].as_element().clone();
        invalid_owner.set_element_type(valid(ElementTypeKey::new("unregistered-current-key")));
        session.project.elements[0] = BodyOwner::shared(invalid_owner);
        let before = session.clone();
        let token = session.mutation_token();

        assert_eq!(
            session.apply_element_template_settings(&valid_target),
            Err(ApplyTemplateError::PlanningFailed)
        );
        assert_eq!(session, before);
        assert_eq!(session.mutation_token(), token);

        let mut source_failure = valid(EditingSession::new(musical_project()));
        source_failure
            .project
            .preservation
            .as_mut()
            .unwrap_or_else(|| unreachable!("built-in project should be source-backed"))
            .sections
            .clear();
        let source_before = source_failure.clone();
        let source_token = source_failure.mutation_token();
        assert_eq!(
            source_failure.apply_element_template_settings(&valid_target),
            Err(ApplyTemplateError::CandidateRejected)
        );
        assert_eq!(source_failure, source_before);
        assert_eq!(source_failure.mutation_token(), source_token);
        assert!(!source_failure.can_undo());
        assert!(!source_failure.can_redo());
        assert!(session.can_redo());
    }

    #[test]
    fn type_less_title_content_is_not_retyped_by_apply() {
        let target = template(scriptcraft_fdx::BuiltinTemplateId::Screenplay);
        let mut project = musical_project();
        let mut title = valid(TitlePageParagraph::new(
            "source-less-typeless-title",
            "unchanged title",
            TitlePageParagraphFormatting::default(),
        ));
        title.element_type = None;
        project.title_page.push(title.clone());
        let mut session = valid(EditingSession::new(project));

        valid(session.apply_element_template_settings(&target));
        assert_eq!(session.project().title_page.last(), Some(&title));
    }

    #[test]
    fn public_builtin_service_uses_each_catalog_id_and_preserves_current_content() {
        let mut session = valid(EditingSession::new(musical_project()));
        let body_text = session
            .project()
            .elements
            .iter()
            .map(|owner| owner.as_element().text.clone())
            .collect::<Vec<_>>();
        let title_text = session
            .project()
            .title_page
            .iter()
            .map(|paragraph| paragraph.text.clone())
            .collect::<Vec<_>>();
        let clean_baseline = session.clean_project.clone();
        let original_bytes = session
            .project()
            .preservation
            .as_ref()
            .map(|preservation| preservation.source.original_bytes.clone());

        for (id, expected_key) in [
            (BuiltinTemplateId::Stageplay, "Notations"),
            (BuiltinTemplateId::DramatistsGuildMusical, "Lyrics"),
            (BuiltinTemplateId::Screenplay, "Scene Heading"),
        ] {
            assert_eq!(
                session.apply_builtin_template(id),
                Ok(ApplyTemplateOutcome::Changed)
            );
            assert!(session
                .project()
                .element_definitions()
                .get_by_label(expected_key)
                .is_some());
            assert_eq!(
                session
                    .project()
                    .elements
                    .iter()
                    .map(|owner| owner.as_element().text.clone())
                    .collect::<Vec<_>>(),
                body_text
            );
            assert_eq!(
                session
                    .project()
                    .title_page
                    .iter()
                    .map(|paragraph| paragraph.text.clone())
                    .collect::<Vec<_>>(),
                title_text
            );
            assert_eq!(session.clean_project, clean_baseline);
            assert_eq!(
                session
                    .project()
                    .preservation
                    .as_ref()
                    .map(|preservation| preservation.source.original_bytes.clone()),
                original_bytes
            );
            assert!(session.dirty());
        }
    }

    #[test]
    fn public_builtin_service_maps_equal_candidate_to_no_op() {
        let screenplay = template(BuiltinTemplateId::Screenplay);
        let mut project = valid(Project::empty_projection());
        valid(
            project.replace_element_definitions(screenplay.definitions().iter().cloned().collect()),
        );
        let action = ScreenplayElement::new(
            "source-less-action",
            valid(ElementTypeKey::new("Action")),
            "unchanged action",
        );
        let action = valid(action);
        project.elements.push(BodyOwner::shared(action));
        let mut session = valid(EditingSession::new(project));
        let before = session.clone();

        assert_eq!(
            session.apply_builtin_template(BuiltinTemplateId::Screenplay),
            Ok(ApplyTemplateOutcome::NoOp)
        );
        assert_eq!(session, before);
    }

    #[test]
    fn applying_same_builtin_to_source_backed_project_is_a_complete_no_op() {
        let project = valid(instantiate_builtin_template(BuiltinTemplateId::Screenplay)).project;
        let mut session = valid(EditingSession::new(project));
        let before = session.clone();
        let revision = session.revision();
        let token = session.mutation_token();
        let history_len = session.history.len();

        assert_eq!(
            session.apply_builtin_template(BuiltinTemplateId::Screenplay),
            Ok(ApplyTemplateOutcome::NoOp)
        );
        assert_eq!(session, before);
        assert_eq!(session.revision(), revision);
        assert_eq!(session.mutation_token(), token);
        assert_eq!(session.history.len(), history_len);
    }
}
