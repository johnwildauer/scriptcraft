//! Session-aware layout lookup for desktop rendering.

use super::{diagnostics::add_performance_nanos, responsiveness, ScriptCraftApp};
use scriptcraft_commands::EditorSurface;
use scriptcraft_layout::{
    BodyEditorPresentation, LayoutBuildDiagnostics, LayoutEnvironment, LayoutErrorKind,
    ScreenplayLayout,
};
use std::time::Instant;

impl ScriptCraftApp {
    pub(crate) fn show_beat_board(&mut self, ui: &mut eframe::egui::Ui) {
        let prepared = match self.document_font_store.prepare_document(self.session.project()) {
            Ok(fonts) => {
                self.prepared_document_fonts = Some(fonts.clone());
                match self.document_font_store.register_prepared_fonts(ui.ctx(), &fonts) {
                    Ok(true) => Some(fonts),
                    Ok(false) => {
                        // Families queued with add_font become available on the next pass.
                        ui.ctx().request_repaint();
                        None
                    }
                    Err(_) => {
                        self.show_error("FONT_PREPARATION_FAILED: resolved face registration failed; document unchanged");
                        None
                    }
                }
            }
            Err(_) => {
                self.show_error("FONT_PREPARATION_FAILED: no prepared environment; document unchanged");
                None
            }
        };
        let environment = prepared.as_ref().map(|fonts| LayoutEnvironment::new(
            self.session.layout_inputs().page_only(), fonts.environment.clone(),
        ));
        let command_availability = self.app_command_availability();
        let menu_popup_blocked = self.menu_policy.in_window_shortcuts_owned()
            && crate::chrome::menu::in_window_menu_popup_open(ui.ctx());
        let sprint_popup_blocked = self.sprint_timer.popup_open
            && ui.ctx().input(|i| i.pointer.hover_pos().is_some_and(|p| self.sprint_timer.popup_rect.contains(p)));
        let available = command_availability.is_enabled(scriptcraft_commands::CommandId::BeatBoardSelectionSet)
            && !menu_popup_blocked && !sprint_popup_blocked;
        let ime_event_in_frame = ui.ctx().input(|input| input.events.iter().any(|event| matches!(event, eframe::egui::Event::Ime(_))));
        let card_delete_keyboard_enabled = command_availability.beat_card_delete
            && !menu_popup_blocked && !sprint_popup_blocked
            && crate::chrome::shortcuts::shortcut_input_allowed(self.ime_active, ime_event_in_frame);
        let document_identity = self.session.project().layout_identity().clone();
        let (success, command, beat_auto_scroll_owns_input) = crate::views::beat_board::show(
            ui,
            self.session.project().beat_board(),
            &mut self.beat_board_view_state,
            &mut self.beat_viewport,
            self.theme_palette.script_workspace,
            &mut self.beat_layout_cache,
            environment.as_ref().zip(prepared.as_ref()),
            self.session.beat_selection(),
            self.session.mutation_token(),
            document_identity,
            &mut self.beat_interaction,
            &mut self.beat_auto_scroll,
            &mut self.beat_text,
            &mut self.beat_drag_coordinator,
            available,
            card_delete_keyboard_enabled,
        );
        for command in command {
            self.dispatch_command(crate::commands::CommandInvocation::new(command, scriptcraft_commands::CommandOrigin::Pointer), ui.ctx());
        }
        if !beat_auto_scroll_owns_input && self.beat_text.focused {
            match self.session.beat_selection() {
                scriptcraft_session::editing::BeatSelection::Text { card, field, .. } => {
                    let changed_target = self.beat_text.field.as_ref().is_none_or(|(old_card, old_field)| old_card != card || old_field != field);
                    if changed_target { self.beat_text.focus(card.clone(), *field, ui.ctx().input(|input| input.time)); }
                }
                _ => self.beat_text.blur(),
            }
        }
        if !success {
            self.show_error("BEAT_LAYOUT_OR_PAINT_FAILED: card presentation unavailable; document unchanged");
        }
    }

    pub(crate) fn layout_for_render(
        &mut self,
        surface: EditorSurface,
    ) -> Result<std::sync::Arc<ScreenplayLayout>, scriptcraft_layout::LayoutError> {
        let started = Instant::now();
        if surface == EditorSurface::Body {
            self.reconcile_body_active_group();
        }
        let (prepared_fonts, font_observation) = if self.development_mode {
            let (prepared, observation) = self
                .document_font_store
                .prepare_document_observed(self.session.project())
                .map_err(|_| scriptcraft_layout::LayoutError {
                    kind: LayoutErrorKind::NoGeometry,
                })?;
            (prepared, Some(observation))
        } else {
            (
                self.document_font_store
                    .prepare_document(self.session.project())
                    .map_err(|_| scriptcraft_layout::LayoutError {
                        kind: LayoutErrorKind::NoGeometry,
                    })?,
                None,
            )
        };
        if let Some(observation) = font_observation {
            self.responsiveness.record_font_preparation(
                observation.signature_duration,
                observation.cache_hit,
                observation.miss_preparation_duration,
                observation.cache_clear_count,
            );
        }
        let layout_environment = LayoutEnvironment::new(
            self.session.layout_inputs().page_only(),
            prepared_fonts.environment.clone(),
        );
        self.prepared_document_fonts = Some(prepared_fonts);
        let cache = match surface {
            EditorSurface::Body => &mut self.layout_cache,
            EditorSurface::TitlePage => &mut self.title_layout_cache,
        };
        let mut build_diagnostics = self.development_mode.then(LayoutBuildDiagnostics::default);
        let (layout, hit) = if surface == EditorSurface::Body {
            let presentation = BodyEditorPresentation::for_project(
                self.session.project(),
                self.active_body_dual_dialogue_group.clone(),
            );
            if let Some(diagnostics) = build_diagnostics.as_mut() {
                cache.get_or_build_body_editor_for_session_with_environment_and_diagnostics(
                    &self.session,
                    &presentation,
                    &layout_environment,
                    diagnostics,
                )?
            } else {
                cache.get_or_build_body_editor_for_session_with_environment(
                    &self.session,
                    &presentation,
                    &layout_environment,
                )?
            }
        } else if let Some(diagnostics) = build_diagnostics.as_mut() {
            cache.get_or_build_title_page_editor_for_session_with_environment_and_diagnostics(
                &self.session,
                &layout_environment,
                diagnostics,
            )?
        } else {
            cache.get_or_build_title_page_editor_for_session_with_environment(
                &self.session,
                &layout_environment,
            )?
        };
        let elapsed = started.elapsed();
        add_performance_nanos(
            &mut self.performance.layout_lookup_nanos,
            elapsed.as_nanos(),
        );
        if self.development_mode {
            self.responsiveness.record_layout(elapsed, hit);
            if !hit {
                if let Some(observation) = build_diagnostics.as_ref() {
                    self.responsiveness.record_layout_build(observation);
                }
            }
        }
        if hit {
            self.performance.layout_cache_hits =
                self.performance.layout_cache_hits.saturating_add(1);
        } else {
            self.performance.layout_builds = self.performance.layout_builds.saturating_add(1);
            self.performance.layout_nanos = self
                .performance
                .layout_nanos
                .saturating_add(elapsed.as_nanos());
        }
        let identity_changed = self.last_layout_identity.as_ref().is_some_and(|previous| {
            previous
                .upgrade()
                .is_none_or(|previous| !std::sync::Arc::ptr_eq(&previous, &layout))
        });
        if identity_changed {
            self.performance.layout_identity_changes =
                self.performance.layout_identity_changes.saturating_add(1);
        }
        self.last_layout_identity = Some(std::sync::Arc::downgrade(&layout));
        Ok(layout)
    }
}
