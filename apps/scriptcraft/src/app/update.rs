//! Root eframe adapter, input handling, and application update composition.

use crate::app;
use crate::app::{measure_root_other_bucket, responsiveness, RootDocumentView, ScriptCraftApp};
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
        let projection = app.app_command_projection();
        let dynamic_elements = app.dynamic_element_menu_projection();
        let formatting = crate::chrome::formatting_menu::project_for_target(
            &app.session,
            app.active_editor_surface,
            app.root_document_view == RootDocumentView::BeatBoard,
            app.script_note_body_owns_input(),
        );
        let view_menu = crate::chrome::menu_items::ViewMenuProjection {
            zoom_percent: app.active_zoom(),
            appearance_mode: app.appearance_mode,
            beat_board: app.root_document_view == RootDocumentView::BeatBoard,
        };
        let menu_panel = egui::Panel::top("application-menu")
            .frame(chrome_frame(app.theme_palette.toolbar))
            .show_separator_line(false)
            .exact_size(menu::MENU_PANEL_HEIGHT)
            .show_inside(ui, |ui| {
                menu::show_menu_bar(
                    ui,
                    projection.availability,
                    &dynamic_elements,
                    &formatting,
                    view_menu,
                    app.script_view_mode,
                    projection.say_undual,
                    projection.invisibles_visible,
                    projection.ruler_visible,
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
    let body_focused = app.root_document_view == RootDocumentView::Script && app.surface_focused
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
    let alt_outcome_summary = app
        .development_mode
        .then(|| app.responsiveness.alt_outcome_summary());
    let alt_pointer_summary = app
        .development_mode
        .then(|| app.responsiveness.alt_pointer_summary());
    let font_preparation_summary = app
        .development_mode
        .then(|| app.responsiveness.font_preparation_summary());
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
        alt_outcome_summary.as_deref(),
        alt_pointer_summary.as_deref(),
        font_preparation_summary.as_deref(),
    ) {
        (
            Some(performance_summary),
            Some(phase_summary),
            Some(command_details),
            Some(aligned_details),
            Some(aligned_edit_update_other_detail_summary),
            Some(layout_build_detail_summary),
            Some(alt_outcome_summary),
            Some(alt_pointer_summary),
            Some(font_preparation_summary),
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
            alt_outcome_summary,
            alt_pointer_summary,
            font_preparation_summary,
            diagnostic_message: diagnostic_message.as_deref(),
        }),
        _ => None,
    };
    let mut timer_button_rect = None;
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
            let status_intents = if app.root_document_view == RootDocumentView::BeatBoard {
                status_bar::show_beat(ui, app.theme_palette.status, page_number, diagnostics,
                    app.development_mode, app.active_zoom(), app.appearance_mode, &focus_view,
                    app.active_zoom_policy())
            } else { status_bar::show_with_focus_mode(
                ui,
                app.theme_palette.status,
                page_number,
                &center,
                diagnostics,
                app.development_mode,
                app.zoom_percent,
                app.appearance_mode,
                &focus_view,
            ) };
            timer_button_rect = status_intents.timer_button_rect;
            for command in status_intents.commands {
                let opening_timer = matches!(&command, Command::ToolsSprintTimerTogglePopup)
                    && !app.sprint_timer.popup_open;
                app.dispatch_command(
                    CommandInvocation::new(command, CommandOrigin::Pointer),
                    &ctx,
                );
                if opening_timer {
                    app.attach_sprint_timer_popup_to_surface(
                        timer_button_rect.unwrap_or(egui::Rect::NOTHING),
                        EditorSurface::Body,
                        &ctx,
                    );
                }
            }
        });
    status_bar::paint_panel_top_border(ui, status_panel.response.rect, app.theme_palette.status);
    let frame = if app.root_document_view == RootDocumentView::BeatBoard {
        egui::Frame::NONE.fill(app.theme_palette.script_workspace.background)
    } else {
        script_view::central_panel_frame(&ctx, app.script_view_mode, app.theme_palette.script_workspace)
    };
    let central = egui::CentralPanel::default()
        .frame(frame)
        .show_inside(ui, |ui| {
            if app.root_document_view == RootDocumentView::BeatBoard {
                app.show_beat_board(ui);
            } else {
                let editor_scrollbar = app.theme_palette.script_workspace.editor_scrollbar;
                script_view::with_editor_surface_scrollbar_presentation(ui, editor_scrollbar, |ui| {
                    app.live_document_surface(ui, EditorSurface::Body)
                });
            }
        });
    if app.timer_popup_surface == EditorSurface::TitlePage
        && (!app.title_page_window_open || app.title_viewport_unsupported)
    {
        app.timer_popup_surface = EditorSurface::Body;
        app.timer_title_host_geometry = None;
    }
    if !app.element_windows.owns_input() {
        app.open_pending_sprint_timer_completion(EditorSurface::Body, timer_button_rect, &ctx);
    }
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
        if self.root_document_view == RootDocumentView::BeatBoard {
            self.theme_palette.script_workspace.background.to_normalized_gamma_f32()
        } else {
            script_view::chrome_clear_color(self.script_view_mode, self.theme_palette.script_workspace)
        }
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
        let title_page_viewport = raw_input.viewport_id == title_page::viewport_id();
        let script_note_viewport = raw_input.viewport_id == crate::widgets::script_note::viewport::viewport_id()
            || raw_input.viewport_id == crate::widgets::script_note::type_dialog::viewport_id()
            || raw_input.viewport_id == crate::widgets::script_note::properties::color_viewport_id()
            || raw_input.viewport_id == crate::widgets::script_note::properties::type_viewport_id();
        let type_dialog_open = self.script_note_workspace.new_type_dialog_open;
        let note_family_open = self.script_note_workspace.open_note.is_some() || type_dialog_open;
        if raw_input.viewport_id == egui::ViewportId::ROOT && note_family_open {
            let root_pointer_click = raw_input.events.iter().any(|event| {
                matches!(event, egui::Event::PointerButton { pressed: true, .. })
            });
            if root_pointer_click {
                let root_origin = raw_input.viewports.get(&raw_input.viewport_id)
                    .and_then(|viewport| viewport.outer_rect).map_or(egui::Pos2::ZERO, |rect| rect.min);
                let content_origin = raw_input.screen_rect.map_or(egui::Pos2::ZERO, |rect| rect.min);
                let regions = self.script_note_workspace.native_position
                    .map(crate::widgets::script_note::geometry::CardRegions::at);
                for event in &raw_input.events {
                    let egui::Event::PointerButton { pos, pressed: true, .. } = event else { continue };
                    let screen_pos = root_origin + (*pos - content_origin);
                    if self.script_note_workspace.color_menu_open
                        && !regions.is_some_and(|regions| regions.color_region.contains(screen_pos))
                    {
                        self.script_note_workspace.color_menu_open = false;
                        self.script_note_workspace.color_menu_close_pending = true;
                    }
                    if self.script_note_workspace.type_menu_open
                        && !regions.is_some_and(|regions| regions.type_region.contains(screen_pos))
                    {
                        self.script_note_workspace.type_menu_open = false;
                        self.script_note_workspace.type_menu_close_pending = true;
                    }
                }
            }
            let outside_note_click = raw_input.events.iter().any(|event| {
                let egui::Event::PointerButton { pos, pressed: true, .. } = event else { return false };
                let root_origin = raw_input.viewports.get(&raw_input.viewport_id)
                    .and_then(|viewport| viewport.outer_rect).map_or(egui::Pos2::ZERO, |rect| rect.min);
                let content_origin = raw_input.screen_rect.map_or(egui::Pos2::ZERO, |rect| rect.min);
                let screen_pos = root_origin + (*pos - content_origin);
                let note_rect = self.script_note_workspace.native_position.map(|origin| {
                    egui::Rect::from_min_size(origin, crate::widgets::script_note::geometry::CARD_SIZE)
                });
                let type_rect = crate::widgets::script_note::type_dialog::rect(&self.script_note_workspace);
                (!note_rect.is_some_and(|rect| rect.contains(screen_pos))
                    && !type_rect.is_some_and(|rect| rect.contains(screen_pos)))
            });
            if outside_note_click {
                self.script_note_workspace.dismiss();
                self.script_note_workspace.viewport_close_pending = true;
                ctx.request_repaint();
            }
        }
        let element_viewport = raw_input.viewport_id
            == crate::widgets::elements::inspector::viewport_id()
            || raw_input.viewport_id == crate::widgets::elements::selector::viewport_id();
        let scale_changed = !element_viewport && !script_note_viewport && !type_dialog_open
            && self.effective_pixels_per_point != Some(effective_pixels_per_point);
        if !element_viewport && !script_note_viewport && !type_dialog_open {
            self.effective_pixels_per_point = Some(effective_pixels_per_point);
        }

        // Keep the middle-button gesture state viewport-local while routing both windows through the
        // same pinch command and anchor/repaint implementation.
        let viewport_close_requested = raw_input
            .viewports
            .get(&raw_input.viewport_id)
            .is_some_and(|viewport| viewport.close_requested());
        let viewport_height_points = raw_input
            .screen_rect
            .map_or_else(|| ctx.content_rect().height(), |rect| rect.height());
        let element_dialog_open = self.element_windows.owns_input();
        let beat_root_input = !title_page_viewport && !element_viewport && !script_note_viewport
            && self.root_document_view == RootDocumentView::BeatBoard;
        if beat_root_input {
            self.handle_beat_navigation_input(ctx, raw_input, viewport_close_requested);
        } else {
        let wheel_factors = {
            let adapter = if title_page_viewport {
                &mut self.title_view.wheel_zoom
            } else {
                &mut self.wheel_zoom
            };
            if viewport_close_requested || element_dialog_open || element_viewport || script_note_viewport || type_dialog_open {
                adapter.reset();
                Vec::new()
            } else {
                adapter.adapt_events(&mut raw_input.events, viewport_height_points)
            }
        };
        let surface = if title_page_viewport {
            Some(EditorSurface::TitlePage)
        } else if script_note_viewport {
            None
        } else if element_viewport {
            None
        } else {
            Some(EditorSurface::Body)
        };
        if title_page_viewport && !self.title_page_window_open {
            self.title_view.wheel_zoom.reset();
        } else if let Some(surface) = surface {
            self.dispatch_wheel_zoom_factors(surface, ctx, wheel_factors);
        }

        for event in &raw_input.events {
            if !element_dialog_open && !element_viewport {
                    if !script_note_viewport && !type_dialog_open {
                    if let egui::Event::Zoom(factor) = event {
                        if let Some(surface) = surface { self.set_active_editor_surface(surface); }
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
        }
        // The app has consumed the event for page zoom. Keep it out of egui's
        // later input-state zoom delta so no other surface can apply it again.
        if element_dialog_open || element_viewport || script_note_viewport || type_dialog_open {
            raw_input
                .events
                .retain(|event| !matches!(event, egui::Event::Zoom(_)));
        }
        }

        // The active platform backend owns accelerators: native macOS while the
        // native menu is installed, otherwise the in-window backend on Windows.
        // Consume only recognized pressed keys; preserve unrelated input.
        let ime_event_in_frame = raw_input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)));
        if !script_note_viewport && !type_dialog_open && self.menu_policy.in_window_shortcuts_owned()
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
                let activation = menu::command_for_pressed_key_event(
                    &event,
                    shortcuts::ShortcutPlatform::current(),
                );
                if let Some(activation) = activation.filter(|activation| match activation {
                    menu::MenuActivation::Command(command) => {
                        !element_dialog_open
                            || element_dialog_shortcut_allowed(
                                command.id(),
                                inspector_open,
                                selector_open,
                                root_close_enabled,
                                application_quit_enabled,
                            )
                    }
                    menu::MenuActivation::ZoomStep(_) => !element_dialog_open,
                }) {
                    self.pending_commands
                        .enqueue_activation(activation, CommandOrigin::Shortcut);
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
    /// Process Beat wheel and pinch in raw event order, before egui accumulates
    /// smooth scrolling and before queued Save commands capture view state.
    fn handle_beat_navigation_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput, closing: bool) {
        let availability = self.app_command_availability();
        let blocked = closing || self.element_windows.owns_input()
            || raw.viewports.get(&raw.viewport_id).is_some_and(|viewport| viewport.focused == Some(false))
            || availability.replacement_pending || availability.modal_blocked
            || availability.native_output_active
            || (self.menu_policy.in_window_shortcuts_owned() && menu::in_window_menu_popup_open(ctx));
        if blocked {
            self.wheel_zoom.reset();
            return;
        }
        let mut pointer = self.beat_viewport.and_then(|geometry| geometry.pointer_screen);
        let mut retained = Vec::with_capacity(raw.events.len());
        for event in std::mem::take(&mut raw.events) {
            match &event {
                egui::Event::PointerMoved(pos) | egui::Event::PointerButton { pos, .. } => pointer = Some(*pos),
                egui::Event::PointerGone | egui::Event::WindowFocused(false) => {
                    pointer = None;
                    self.wheel_zoom.reset();
                }
                _ => {}
            }
            self.beat_viewport = self.beat_viewport.map(|geometry| geometry.with_pointer(pointer));
            let inside = self.beat_viewport.is_some_and(|geometry| pointer.is_some_and(|pos|
                geometry.clip.contains(pos) && !(self.sprint_timer.popup_open && self.sprint_timer.popup_rect.contains(pos))));
            let owns_pinch = self.beat_viewport.is_some() && (inside || pointer.is_none());
            if let egui::Event::Zoom(factor) = &event {
                if owns_pinch {
                    self.set_active_editor_surface(EditorSurface::Body);
                    self.record_native_event("zoom");
                    if let Ok(factor) = ZoomFactor::new(*factor) {
                        self.dispatch_command(CommandInvocation::new(Command::ViewZoomPinch { factor }, CommandOrigin::InternalFollowUp), ctx);
                    }
                    continue;
                }
            }
            if !inside {
                self.wheel_zoom.reset();
                retained.push(event);
                continue;
            }
            let viewport_size = self.beat_viewport.map(|geometry| geometry.viewport.size()).unwrap_or(egui::Vec2::ZERO);
            let mut single = vec![event];
            let factors = self.wheel_zoom.adapt_events(&mut single, viewport_size.y);
            self.dispatch_wheel_zoom_factors(EditorSurface::Body, ctx, factors);
            for event in single {
                if let egui::Event::MouseWheel { unit, delta, modifiers, .. } = &event {
                    let multiplier = match unit {
                        egui::MouseWheelUnit::Point => egui::Vec2::splat(1.0),
                        egui::MouseWheelUnit::Line => egui::Vec2::splat(crate::views::beat_board::WHEEL_POINTS_PER_LINE),
                        egui::MouseWheelUnit::Page => viewport_size,
                    };
                    let mut points = egui::vec2(delta.x * multiplier.x, delta.y * multiplier.y);
                    if modifiers.shift {
                        points.x += points.y;
                        points.y = 0.0;
                    }
                    if points.x.is_finite() && points.y.is_finite() {
                        if points.y != 0.0 && !modifiers.shift
                            && pointer.is_some_and(|pointer| self.scroll_focused_beat_body(pointer, points.y))
                        {
                            ctx.request_repaint();
                            continue;
                        }
                        if let Some(geometry) = self.beat_viewport {
                            let origin = geometry.pan(points);
                            if let Some(view) = scriptcraft_fdx::BeatBoardViewState::new(origin, self.beat_board_view_state.zoom_milli_percent()) {
                                self.beat_board_view_state = view;
                                self.beat_viewport = Some(geometry.with_view(view));
                                ctx.request_repaint();
                            }
                        }
                    }
                    // Consume even invalid deltas so egui cannot scroll a second time.
                    continue;
                }
                retained.push(event);
            }
        }
        raw.events = retained;
    }

    fn scroll_focused_beat_body(&mut self, pointer: egui::Pos2, wheel_delta_y: f32) -> bool {
        use crate::views::beat_board::geometry::{pixels, scale};
        use scriptcraft_session::editing::BeatTextField;
        if !self.beat_text.focused || self.beat_text.field.as_ref().is_none_or(|(_, field)| *field != BeatTextField::Body) {
            return false;
        }
        let Some(geometry) = self.beat_viewport else { return false };
        let Some(fonts) = self.prepared_document_fonts.as_ref() else { return false };
        let Some((card_id, _)) = self.beat_text.field.as_ref() else { return false };
        let Some(card) = self.session.project().beat_board().cards.iter().find(|card| &card.id == card_id) else { return false };
        let environment = scriptcraft_layout::LayoutEnvironment::new(
            self.session.layout_inputs().page_only(), fonts.environment.clone(),
        );
        let Ok(layouts) = self.beat_layout_cache.get_or_build(card, &environment, fonts, scriptcraft_layout::BeatCardMetrics::default()) else { return false };
        let zoom = scale(geometry.view) as f32;
        let origin = geometry.board_to_screen(pixels(card.rect.left), pixels(card.rect.top));
        let body = layouts.card.body_rect;
        let rect = egui::Rect::from_min_size(
            origin + egui::vec2(body.x.to_points_f32() * zoom, body.y.to_points_f32() * zoom),
            egui::vec2(body.width.to_points_f32() * zoom, body.height.to_points_f32() * zoom),
        );
        if !rect.contains(pointer) { return false; }
        let max = (layouts.card.body_content_height.to_points_f32() - body.height.to_points_f32()).max(0.0);
        let next = (self.beat_text.body_offset - wheel_delta_y / zoom).clamp(0.0, max);
        if (next - self.beat_text.body_offset).abs() < f32::EPSILON { return false; }
        self.beat_text.body_offset = next;
        true
    }

    fn update(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        let ctx = ui.ctx().clone();
        let native_pixels_per_point = ctx.input(|input| input.viewport().native_pixels_per_point);
        self.script_note_workspace.work_bounds = current_root_work_area(frame, native_pixels_per_point);
        #[cfg(target_os = "macos")]
        {
            self.native_operation_coordinator.refresh_root_owner(frame, &ctx);
            self.poll_native_operation_completion(&ctx);
        }
        self.finalize_initial_document_slot();
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
                if app.element_windows.owns_input() {
                    elements::focus_topmost_element_viewport(app, &ctx);
                } else {
                    app.set_active_editor_surface(EditorSurface::Body);
                }
            }
        });
        let startup_outcome =
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
                let acknowledged_replacement_child = app.acknowledged_replacement_startup();
                let outcome = app.consume_startup_document_handoff();
                if let Some(outcome) = outcome {
                    app.emit_startup_handoff_result(outcome);
                }
                #[cfg(target_os = "macos")]
                app.drain_native_open_documents();
                if outcome == Some(StagedOpenOutcome::Failed) {
                    show_startup_open_failure(
                        StagedOpenOutcome::Failed,
                        acknowledged_replacement_child,
                    );
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
            if self.root_document_view == RootDocumentView::Script
                && !self.script_note_workspace.new_type_dialog_open
            {
                measure_root_other_bucket(
                    self,
                    app::responsiveness::RootOtherBucket::InputDispatch,
                    |app| app.handle_surface_input(&ctx, EditorSurface::Body),
                );
            }
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
            self.consume_pending_element_focus_return(&ctx);
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
                app.script_note_workspace.reset_document_activation();
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
        if self.script_note_workspace.viewport_close_pending {
            if self.script_note_workspace.open_note.is_none() {
                crate::widgets::script_note::viewport::close(&ctx);
                crate::widgets::script_note::type_dialog::close(&ctx);
            } else {
                crate::widgets::script_note::type_dialog::close(&ctx);
            }
            self.script_note_workspace.type_dialog_close_pending = false;
            self.script_note_workspace.viewport_close_pending = false;
        }
        if self.script_note_workspace.open_note.is_some() {
            measure_root_other_bucket(self, app::responsiveness::RootOtherBucket::Host, |app| {
                crate::widgets::script_note::viewport::show(app, &ctx)
            });
        }
        crate::widgets::script_note::properties::flush_pending_closes(
            &ctx,
            &mut self.script_note_workspace,
        );
        let property_action = self.script_note_workspace.open_note.as_ref()
            .and_then(|note_id| self.session.project().script_notes().get(note_id).cloned())
            .map(|note| {
                let types = crate::widgets::script_note::properties::used_types(
                    self.session.project().script_notes(),
                );
                crate::widgets::script_note::properties::show_native_menus(
                    &ctx,
                    &mut self.script_note_workspace,
                    &note,
                    &types,
                )
            })
            .flatten();
        crate::widgets::script_note::properties::flush_pending_closes(
            &ctx,
            &mut self.script_note_workspace,
        );
        if let Some(action) = property_action {
            crate::widgets::script_note::viewport::apply_property_menu_action(self, &ctx, action);
        }
        let type_dialog_action = crate::widgets::script_note::type_dialog::show(self, &ctx);
        if let Some(action) = type_dialog_action {
            use crate::widgets::script_note::type_dialog::TypeDialogAction;
            match action {
                TypeDialogAction::Cancel => {
                    let return_focus = self.script_note_workspace.type_return_focus.take();
                    self.script_note_workspace.new_type_dialog_open = false;
                    self.script_note_workspace.type_draft.clear();
                    self.script_note_workspace.type_dialog_error = false;
                    self.script_note_workspace.type_dialog_focused = false;
                    crate::widgets::script_note::type_dialog::close(&ctx);
                    if let Some(field) = return_focus {
                        self.script_note_workspace.field = Some(field);
                        self.script_note_workspace.focused = true;
                        ctx.send_viewport_cmd_to(
                            crate::widgets::script_note::viewport::viewport_id(),
                            egui::ViewportCommand::Focus,
                        );
                    }
                }
                TypeDialogAction::Apply(draft) => {
                    let requested_type = if draft.is_empty() || draft == "None" { None } else { Some(draft) };
                    let current_note = self.script_note_workspace.open_note.as_ref()
                        .and_then(|id| self.session.project().script_notes().get(id))
                        .map(|note| (
                            note.id.clone(),
                            crate::widgets::script_note::properties::resolve_color(
                                note.color_projection(),
                                self.theme_palette.script_workspace.page.text,
                            ).default_color,
                        ));
                    if let Some((note_id, color)) = current_note {
                        let modified = scriptcraft_commands::ScriptNoteTimestampValue::new(
                            scriptcraft_fdx::current_timestamp(),
                        ).ok();
                        let outcome = modified.map(|modified| self.dispatch_command(
                            CommandInvocation::new(Command::ScriptNotePropertiesSet {
                                properties: scriptcraft_commands::ScriptNotePropertiesValue {
                                    note: scriptcraft_commands::ScriptNoteIdValue::from_core(note_id.clone()),
                                    color: color.and_then(scriptcraft_commands::ScriptNoteColorValue::from_core),
                                    note_type: requested_type.clone(),
                                    modified,
                                },
                            }, CommandOrigin::Pointer),
                            &ctx,
                        ).outcome);
                        if matches!(outcome,
                            Some(crate::commands::DesktopCommandOutcome::Editor(
                                scriptcraft_commands::CommandOutcome::Changed
                                    | scriptcraft_commands::CommandOutcome::NoOp
                            )))
                        {
                            self.script_note_workspace.new_type_dialog_open = false;
                            self.script_note_workspace.type_draft.clear();
                            self.script_note_workspace.type_dialog_error = false;
                            self.script_note_workspace.type_return_focus = None;
                            self.script_note_workspace.type_dialog_focused = false;
                            crate::widgets::script_note::type_dialog::close(&ctx);
                            self.script_note_workspace.field = Some(scriptcraft_session::editing::ScriptNoteField::Body);
                            self.script_note_workspace.focused = true;
                            let body_position = match self.session.script_note_selection() {
                                scriptcraft_session::editing::ScriptNoteSelection::Text {
                                    note, field: scriptcraft_session::editing::ScriptNoteField::Body, focus, ..
                                } if note == &note_id => *focus,
                                _ => scriptcraft_session::editing::ScriptNoteTextPosition { paragraph: 0, offset: 0 },
                            };
                            let selection = scriptcraft_commands::ScriptNoteSelectionValue::Text {
                                note: scriptcraft_commands::ScriptNoteIdValue::from_core(note_id),
                                field: scriptcraft_commands::ScriptNoteFieldValue::Body,
                                anchor: scriptcraft_commands::ScriptNoteTextPositionValue {
                                    paragraph: body_position.paragraph as u32,
                                    offset: body_position.offset as u32,
                                },
                                focus: scriptcraft_commands::ScriptNoteTextPositionValue {
                                    paragraph: body_position.paragraph as u32,
                                    offset: body_position.offset as u32,
                                },
                            };
                            let _ = self.dispatch_command(
                                CommandInvocation::new(
                                    Command::ScriptNoteSelectionSet { selection },
                                    CommandOrigin::InternalFollowUp,
                                ),
                                &ctx,
                            );
                            ctx.send_viewport_cmd_to(
                                crate::widgets::script_note::viewport::viewport_id(),
                                egui::ViewportCommand::Focus,
                            );
                        } else {
                            self.script_note_workspace.type_dialog_error = true;
                        }
                    } else {
                        self.script_note_workspace.type_dialog_error = true;
                    }
                }
            }
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
                let dynamic_elements = app.dynamic_element_menu_projection();
                let formatting = crate::chrome::formatting_menu::project_for_target(
                    &app.session,
                    app.active_editor_surface,
                    app.root_document_view == RootDocumentView::BeatBoard,
                    app.script_note_body_owns_input(),
                );
                let view_menu = crate::chrome::menu_items::ViewMenuProjection {
                    zoom_percent: app.active_zoom(),
                    appearance_mode: app.appearance_mode,
                    beat_board: app.root_document_view == RootDocumentView::BeatBoard,
                };
                if let Some(native_menu) = &mut app.native_menu {
                    native_menu.synchronize(
                        projection.availability,
                        app.script_view_mode,
                        projection.say_undual,
                        &dynamic_elements,
                        &formatting,
                        view_menu,
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

fn current_root_work_area(
    frame: &eframe::Frame,
    native_pixels_per_point: Option<f32>,
) -> Option<crate::widgets::script_note::state::WorkAreaBounds> {
    #[cfg(target_os = "macos")]
    {
        let _ = native_pixels_per_point;
        crate::platform::macos::macos_work_area::current_work_area(frame)
    }
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::windows_work_area::current_work_area(frame, native_pixels_per_point)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (frame, native_pixels_per_point);
        None
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
