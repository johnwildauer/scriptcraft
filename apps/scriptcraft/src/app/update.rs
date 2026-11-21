//! Root eframe adapter, input handling, and application update composition.

use crate::app;
use crate::app::{measure_root_other_bucket, responsiveness, ScriptCraftApp};
use crate::chrome::status_bar::status_modifiers_changed;
use crate::chrome::status_bar::DiagnosticProjection;
use crate::chrome::toolbar::toolbar_content_height;
use crate::commands::CommandInvocation;
use crate::{
    chrome::{menu, shortcuts, status_bar, theme},
    documents::open::{show_fdx_migration_dialog, show_startup_open_failure, StagedOpenOutcome},
    views::{
        script::{self as script_view, zoom_scroll::page_geometry_input_invalidated, RevealIntent},
        title_page,
    },
    widgets::{elements, sprint_timer},
};
use eframe::egui;
use scriptcraft_commands::{Command, CommandId, CommandOrigin, EditorSurface, ZoomFactor};
use std::time::Instant;

fn chrome_frame(palette: theme::ToolbarPalette) -> egui::Frame {
    // The semantic divider is painted along the toolbar's lower edge; keep the
    // frame stroke-free so no top or vertical borders are introduced.
    egui::Frame::NONE
        .fill(palette.background)
        .stroke(egui::Stroke::NONE)
}

fn show_application_panels(app: &mut ScriptCraftApp, ui: &mut egui::Ui) -> egui::Rect {
    let ctx = ui.ctx().clone();
    let _ = app.refresh_dynamic_controls();
    let now = app.timer_now();
    app.reconcile_sprint_timer(now);
    let fullscreen = ctx.input(|input| input.viewport().fullscreen);
    app.focus_mode.observe(fullscreen, now);
    let (_resolved_theme, applied_appearance, theme_palette) = theme::configure_appearance(
        &ctx,
        app.appearance_mode,
        app.appearance_applications
            .get(theme::AppearanceContext::Root),
    );
    app.appearance_applications
        .set(theme::AppearanceContext::Root, applied_appearance);
    app.theme_palette = theme_palette;

    if app.menu_policy.renders_in_window() {
        let availability = app.app_command_availability();
        let menu_panel = egui::Panel::top("application-menu")
            .frame(chrome_frame(app.theme_palette.toolbar))
            .show_separator_line(false)
            .exact_size(menu::MENU_PANEL_HEIGHT)
            .show_inside(ui, |ui| {
                menu::show_menu_bar(
                    ui,
                    availability,
                    app.script_view_mode,
                    app.dual_dialogue_menu_should_say_undual(),
                    &mut app.pending_commands,
                );
            });
        let rect = menu_panel.response.rect;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.bottom() - 0.5),
                egui::pos2(rect.right(), rect.bottom() - 0.5),
            ],
            egui::Stroke::new(1.0, app.theme_palette.toolbar.border),
        );
    }

    egui::Panel::top("titlebar-toolbar")
        .frame(chrome_frame(app.theme_palette.toolbar))
        .show_separator_line(false)
        .exact_size(toolbar_content_height(&ctx.global_style()))
        .show_inside(ui, |ui| app.toolbar(ui));
    let viewport_rect = ctx.content_rect();
    let page_number = app.page_geometry.visible_page(
        app.performance.frames,
        app.session.revision(),
        viewport_rect,
        app.page_scale.unwrap_or(f32::NAN),
        app.reveal_intent == RevealIntent::Conditional
            || page_geometry_input_invalidated(&ctx, app.page_geometry.clip()),
    );
    let (modifiers, ime_event_in_frame) = ctx.input(|input| {
        (
            input.modifiers,
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Ime(_))),
        )
    });
    let availability = app.app_command_availability();
    let element_windows_own_input = app.element_windows.owns_input();
    let body_focused = app.surface_focused
        && app.active_editor_surface() == EditorSurface::Body
        && !element_windows_own_input
        && !availability.replacement_pending
        && !availability.modal_blocked
        && !availability.native_output_active
        && !(app.menu_policy.in_window_shortcuts_owned() && menu::in_window_menu_popup_open(&ctx));
    let _ = app.refresh_dynamic_controls();
    let controls = app
        .dynamic_controls
        .as_ref()
        .expect("current controls refreshed before status composition");
    let center = status_bar::project_center(
        &app.session,
        body_focused,
        app.ime_active || ime_event_in_frame,
        element_windows_own_input
            || availability.replacement_pending
            || availability.modal_blocked
            || availability.native_output_active,
        modifiers,
        shortcuts::ShortcutPlatform::current(),
        &controls.shortcuts,
    );
    let diagnostics_calculation_started = app.development_mode.then(Instant::now);
    let performance_summary = app
        .development_mode
        .then(|| app.responsiveness.safe_summary());
    let phase_summary = app
        .development_mode
        .then(|| app.responsiveness.phase_summary());
    let command_detail_summaries = app
        .development_mode
        .then(|| app.responsiveness.command_detail_summaries());
    let aligned_edit_update_summaries = app
        .development_mode
        .then(|| app.responsiveness.aligned_edit_update_summaries());
    let aligned_edit_update_other_detail_summary = app.development_mode.then(|| {
        app.responsiveness
            .aligned_edit_update_other_detail_summary()
    });
    let layout_build_detail_summary = app
        .development_mode
        .then(|| app.responsiveness.layout_build_detail_summary());
    if let Some(started) = diagnostics_calculation_started {
        app.responsiveness
            .record_diagnostics_calculation(started.elapsed());
    }
    let diagnostic_message = if app.development_mode {
        app.diagnostic_message
            .as_ref()
            .map(|message| message.as_str().to_owned())
    } else {
        None
    };
    let diagnostics = match (
        performance_summary.as_deref(),
        phase_summary.as_deref(),
        command_detail_summaries.as_ref(),
        aligned_edit_update_summaries.as_ref(),
        aligned_edit_update_other_detail_summary.as_deref(),
        layout_build_detail_summary.as_deref(),
    ) {
        (
            Some(performance_summary),
            Some(phase_summary),
            Some(command_details),
            Some(aligned_details),
            Some(aligned_edit_update_other_detail_summary),
            Some(layout_build_detail_summary),
        ) => Some(DiagnosticProjection {
            performance_summary,
            phase_summary,
            command_detail_summaries: [
                command_details[0].as_str(),
                command_details[1].as_str(),
                command_details[2].as_str(),
            ],
            aligned_edit_update_summaries: [
                aligned_details[0].as_str(),
                aligned_details[1].as_str(),
            ],
            aligned_edit_update_other_detail_summary,
            layout_build_detail_summary,
            diagnostic_message: diagnostic_message.as_deref(),
        }),
        _ => None,
    };
    let status_panel = egui::Panel::bottom("status")
        .frame(status_bar::status_panel_frame(
            &ctx,
            app.theme_palette.status,
        ))
        .default_size(status_bar::default_panel_height())
        .show_separator_line(false)
        .show_inside(ui, |ui| {
            status_bar::apply_status_palette(ui, app.theme_palette.status);
            let focus_view = app.focus_mode.view();
            let status_intents = status_bar::show_with_focus_mode(
                ui,
                app.theme_palette.status,
                page_number,
                &center,
                diagnostics,
                app.development_mode,
                app.zoom_percent,
                app.appearance_mode,
                &focus_view,
            );
            let timer_button_rect = status_intents.timer_button_rect;
            for command in status_intents.commands {
                let opening_timer = matches!(&command, Command::ToolsSprintTimerTogglePopup)
                    && !app.sprint_timer.popup_open;
                app.dispatch_command(
                    CommandInvocation::new(command, CommandOrigin::Pointer),
                    &ctx,
                );
                if opening_timer {
                    app.sprint_timer.popup_anchor =
                        timer_button_rect.unwrap_or(egui::Rect::NOTHING);
                    app.timer_popup_surface = EditorSurface::Body;
                    app.capture_sprint_timer_host_geometry(EditorSurface::Body, &ctx);
                }
            }
        });
    status_bar::paint_panel_top_border(ui, status_panel.response.rect, app.theme_palette.status);
    let central = egui::CentralPanel::default()
        .frame(script_view::central_panel_frame(
            &ctx,
            app.script_view_mode,
            app.theme_palette.script_workspace,
        ))
        .show_inside(ui, |ui| app.live_document_surface(ui, EditorSurface::Body));
    let timer_commands = if app.timer_popup_surface == EditorSurface::Body
        && !app.element_windows.owns_input()
    {
        let host_geometry = app.sprint_timer_host_geometry(EditorSurface::Body);
        sprint_timer::show_popup_with_host_geometry(&ctx, &mut app.sprint_timer, now, host_geometry)
    } else {
        Vec::new()
    };
    for command in timer_commands {
        app.dispatch_command(
            CommandInvocation::new(command, CommandOrigin::Pointer),
            &ctx,
        );
    }
    if app.timer_popup_surface == EditorSurface::Body
        && app.sprint_timer.take_popup_close_completion()
    {
        app.finalize_sprint_timer_popup(&ctx);
    }
    central.response.rect
}

impl eframe::App for ScriptCraftApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        script_view::chrome_clear_color(self.script_view_mode, self.theme_palette.script_workspace)
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if status_modifiers_changed(self.last_status_modifiers, raw_input.modifiers) {
            self.last_status_modifiers = raw_input.modifiers;
            // Modifier-only transitions may not produce a semantic editor
            // command or a key event; keep the live status projection current.
            self.request_repaint(ctx);
        }
        // egui computes total pixels-per-point as zoom_factor × native scale.
        // Resolve the factor against this frame's native scale before begin_pass
        // so moving between monitors never briefly multiplies a stale UI factor
        // by the newly reported OS scale.
        let viewport = raw_input.viewports.get_mut(&raw_input.viewport_id);
        let native_scale = viewport.and_then(|viewport| {
            if viewport
                .native_pixels_per_point
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                viewport.native_pixels_per_point = None;
            }
            viewport.native_pixels_per_point
        });
        let effective_pixels_per_point = status_bar::resolved_ui_pixels_per_point(native_scale);
        ctx.set_zoom_factor(status_bar::ui_zoom_factor(native_scale));
        let scale_changed = self.effective_pixels_per_point != Some(effective_pixels_per_point);
        self.effective_pixels_per_point = Some(effective_pixels_per_point);

        // eframe invokes this hook for each viewport. Keep the middle-button
        // gesture state viewport-local while routing both windows through the
        // same pinch command and anchor/repaint implementation.
        let title_page_viewport = raw_input.viewport_id == title_page::viewport_id();
        let viewport_close_requested = raw_input
            .viewports
            .get(&raw_input.viewport_id)
            .is_some_and(|viewport| viewport.close_requested());
        let viewport_height_points = raw_input
            .screen_rect
            .map_or_else(|| ctx.content_rect().height(), |rect| rect.height());
        let element_dialog_open = self.element_windows.owns_input();
        let wheel_factors = {
            let adapter = if title_page_viewport {
                &mut self.title_view.wheel_zoom
            } else {
                &mut self.wheel_zoom
            };
            if viewport_close_requested || element_dialog_open {
                adapter.reset();
                Vec::new()
            } else {
                adapter.adapt_events(&mut raw_input.events, viewport_height_points)
            }
        };
        let surface = if title_page_viewport {
            EditorSurface::TitlePage
        } else {
            EditorSurface::Body
        };
        if title_page_viewport && !self.title_page_window_open {
            self.title_view.wheel_zoom.reset();
        } else {
            self.dispatch_wheel_zoom_factors(surface, ctx, wheel_factors);
        }

        for event in &raw_input.events {
            if !element_dialog_open {
                if let egui::Event::Zoom(factor) = event {
                    self.record_native_event("zoom");
                    if let Ok(factor) = ZoomFactor::new(*factor) {
                        self.dispatch_command(
                            CommandInvocation::new(
                                Command::ViewZoomPinch { factor },
                                CommandOrigin::InternalFollowUp,
                            ),
                            ctx,
                        );
                    }
                }
            }
        }
        // The app has consumed the event for page zoom. Keep it out of egui's
        // later input-state zoom delta so no other surface can apply it again.
        if !element_dialog_open {
            raw_input
                .events
                .retain(|event| !matches!(event, egui::Event::Zoom(_)));
        }

        // The active platform backend owns accelerators: native macOS while the
        // native menu is installed, otherwise the in-window backend on Windows.
        // Consume only recognized pressed keys; preserve unrelated input.
        let ime_event_in_frame = raw_input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)));
        if self.menu_policy.in_window_shortcuts_owned()
            && dialog_menu_shortcuts_allowed(
                self.ime_active,
                self.title_view.ime_active,
                ime_event_in_frame,
            )
        {
            let inspector_open = self.element_windows.elements_open();
            let selector_open = self.element_windows.selector_open();
            let dialog_availability = self.app_command_availability();
            let root_close_enabled = dialog_availability.is_enabled(CommandId::FileClose);
            let application_quit_enabled =
                dialog_availability.is_enabled(CommandId::ApplicationQuit);
            let mut retained_events = Vec::with_capacity(raw_input.events.len());
            for event in std::mem::take(&mut raw_input.events) {
                let command = menu::command_for_pressed_key_event(
                    &event,
                    shortcuts::ShortcutPlatform::current(),
                );
                if let Some(command) = command.filter(|command| {
                    !element_dialog_open
                        || element_dialog_shortcut_allowed(
                            command.id(),
                            inspector_open,
                            selector_open,
                            root_close_enabled,
                            application_quit_enabled,
                        )
                }) {
                    self.pending_commands
                        .enqueue(CommandInvocation::new(command, CommandOrigin::Shortcut));
                } else {
                    retained_events.push(event);
                }
            }
            raw_input.events = retained_events;
        }
        if scale_changed {
            self.page_geometry.invalidate();
            self.page_scale = None;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(target_os = "windows")]
        self.update_windows_print_session(ui, frame);
        ScriptCraftApp::update(self, ui, frame);
    }
}

fn element_dialog_shortcut_allowed(
    command: CommandId,
    inspector_open: bool,
    selector_open: bool,
    root_close_enabled: bool,
    application_quit_enabled: bool,
) -> bool {
    match command {
        CommandId::FileClose => root_close_enabled,
        CommandId::ApplicationQuit => application_quit_enabled,
        // Repeated menu activation may focus the already-open root dialog, but
        // cannot move ownership back to an underlying editor.
        CommandId::FormatElementsOpen => inspector_open,
        CommandId::FormatTemplateSelectorOpen => inspector_open || selector_open,
        _ => false,
    }
}

fn dialog_menu_shortcuts_allowed(
    body_ime_active: bool,
    title_ime_active: bool,
    ime_event_in_frame: bool,
) -> bool {
    shortcuts::shortcut_input_allowed(body_ime_active || title_ime_active, ime_event_in_frame)
}

impl ScriptCraftApp {
    fn update(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        self.finalize_initial_document_slot();
        let ctx = ui.ctx().clone();
        let frame_started = Instant::now();
        if self.development_mode {
            self.responsiveness.begin_update();
        }
        self.performance.frames = self.performance.frames.saturating_add(1);
        let root_close_allowed =
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
                app.begin_root_close_update(&ctx, frame)
            });
        // Acknowledgements and process exit are settled before availability or
        // input projection. Matching Opened uses the same WO-001 root Close path.
        let replacement_close_continuation =
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
                app.poll_pending_replacement(&ctx, Instant::now())
            });
        measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
            app.update_native_window_state(&ctx);
            if ctx.input(|input| input.viewport().focused) == Some(true) {
                app.set_active_editor_surface(EditorSurface::Body);
            }
        });
        let startup_outcome =
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
                let outcome = app.consume_startup_document_handoff();
                if let Some(outcome) = outcome {
                    app.emit_startup_handoff_result(outcome);
                }
                #[cfg(target_os = "macos")]
                app.drain_native_open_documents();
                if outcome == Some(StagedOpenOutcome::Failed) {
                    show_startup_open_failure(StagedOpenOutcome::Failed);
                }
                outcome
            });
        #[cfg(target_os = "macos")]
        measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
            if !replacement_close_continuation && app.menu_policy.installs_native_macos() {
                if let Some(native_menu) = &app.native_menu {
                    native_menu.drain_events(&mut app.pending_commands);
                }
            }
            if !replacement_close_continuation
                    && app
                        .native_lifecycle
                        .as_ref()
                        .is_some_and(
                            crate::platform::macos::macos_lifecycle::NativeLifecycleController::take_quit_request,
                        )
                {
                    app.pending_commands.enqueue(CommandInvocation::new(
                        Command::ApplicationQuit,
                        CommandOrigin::NativeOs,
                    ));
                }
        });
        // Menu activations rendered in the previous pass are dispatched once
        // before ordinary editor input, with command availability rechecked.
        if !replacement_close_continuation {
            measure_root_other_bucket(
                self,
                app::responsiveness::RootOtherBucket::InputDispatch,
                |app| app.dispatch_pending_commands(&ctx),
            );
        }
        measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
            app.process_staged_replacement_open(&ctx, frame)
        });
        self.notification_sink.set_repaint_context(&ctx);
        let replacement_pending = self.replacement_open_pending();
        let migration_pending = self.pending_fdx_open.is_some() && !replacement_pending;
        if !migration_pending && !replacement_pending && !replacement_close_continuation {
            // Route events using the editor-focus state established by the prior
            // surface frame, then let this frame's layout and paint observe edits.
            measure_root_other_bucket(
                self,
                app::responsiveness::RootOtherBucket::InputDispatch,
                |app| app.handle_surface_input(&ctx, EditorSurface::Body),
            );
            // Scheduled derived work sees every accepted same-frame edit and
            // never delays already-queued editor input on its deadline frame.
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Derived, |app| {
                app.refresh_cast_lists_on_schedule(&ctx, Instant::now())
            });
        }
        let timer_now = self.timer_now();
        self.reconcile_sprint_timer(timer_now);
        if replacement_close_continuation {
            // The authorized Close is delivered at the next root-update
            // boundary. Keep this intervening frame noninteractive.
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let _ = ui.allocate_space(ui.available_size());
            });
        } else if migration_pending {
            let status_panel = egui::Panel::bottom("status")
                .frame(status_bar::status_panel_frame(
                    &ctx,
                    self.theme_palette.status,
                ))
                .default_size(status_bar::default_panel_height())
                .show_separator_line(false)
                .show_inside(ui, |ui| {
                    status_bar::show_modal_ready_status(ui, self.theme_palette.status);
                });
            status_bar::paint_panel_top_border(
                ui,
                status_panel.response.rect,
                self.theme_palette.status,
            );
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let _ = ui.allocate_space(ui.available_size());
            });
            show_fdx_migration_dialog(self, &ctx);
        } else {
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Chrome, |app| {
                show_application_panels(app, ui)
            });
            // Dialogs are root-context surfaces and appear only after ordinary
            // panels, never over migration or replacement ownership.
            if !replacement_pending {
                measure_root_other_bucket(
                    self,
                    app::responsiveness::RootOtherBucket::Host,
                    |app| elements::show_elements_windows(app, &ctx),
                );
            }
        }
        if root_close_allowed {
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
                app.close_title_page_window(&ctx);
                if app.timer_popup_surface == EditorSurface::Body {
                    app.finalize_sprint_timer_popup(&ctx);
                }
            });
        } else if !replacement_close_continuation {
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
                title_page::show_if_open(app, &ctx)
            });
        }
        if replacement_pending {
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |_| {
                show_pending_replacement_indicator(&ctx)
            });
        }
        measure_root_other_bucket(
            self,
            app::responsiveness::RootOtherBucket::InputDispatch,
            |app| app.flush_pending_diagnostic_commands(&ctx),
        );
        self.schedule_sprint_timer_repaint(&ctx, self.timer_now());
        self.schedule_focus_mode_repaint(&ctx, self.timer_now());
        #[cfg(target_os = "macos")]
        measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
            if app.menu_policy.installs_native_macos() {
                let projection = app.app_command_projection();
                if let Some(native_menu) = &mut app.native_menu {
                    native_menu.synchronize(
                        projection.availability,
                        app.script_view_mode,
                        projection.say_undual,
                    );
                }
            }
        });
        measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
            app.sync_native_title(&ctx)
        });
        if self.development_mode {
            self.responsiveness
                .record_update(frame_started, Instant::now());
        }
        self.performance.frame_nanos = self
            .performance
            .frame_nanos
            .saturating_add(frame_started.elapsed().as_nanos());
    }
}

/// A bounded, nonclosable status card over the still-painted document. It has
/// no action affordance and does not imply that the child has activated.
fn show_pending_replacement_indicator(ctx: &egui::Context) {
    egui::Area::new(egui::Id::new("replacement-open-pending-indicator"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.label(
                        egui::RichText::new("Opening selected document in a new instance").strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        "The current document is temporarily locked while the new instance opens.",
                    );
                });
        });
}
