//! Same-process native title-page viewport and its child-only composition.
//!
//! This remains an ordinary top-level native window, not a native OS owned child.
//! The callback borrows the existing ScriptCraftApp synchronously through egui's
//! public immediate-viewport API; no application, process, session, document,
//! menu controller, or save path is created here. Embedded fallback callbacks are
//! rejected rather than rendered as an in-place editor.

use super::{app_commands::CommandInvocation, status_bar, theme, ScriptCraftApp};
use eframe::egui;
use scriptcraft_commands::{Command, CommandOrigin, EditorSurface};

pub(super) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("scriptcraft-title-page-editor")
}
pub(super) fn native_title(main_window_title: &str) -> String {
    format!("Title Page - {main_window_title}")
}

fn viewport_builder(main_window_title: &str) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(native_title(main_window_title))
        .with_inner_size([820.0, 680.0])
        .with_min_inner_size([520.0, 420.0])
        .with_resizable(true)
}

fn accepts_viewport_class(class: egui::ViewportClass) -> bool {
    class == egui::ViewportClass::Immediate
}

pub(super) fn send_close(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Close);
}

pub(super) fn show_if_open(app: &mut ScriptCraftApp, root_ctx: &egui::Context) {
    if !app.title_page_window_open || app.title_viewport_unsupported {
        return;
    }

    let focus_requested = std::mem::take(&mut app.title_page_focus_requested);
    root_ctx.show_viewport_immediate(
        viewport_id(),
        viewport_builder(&app.native_title),
        |ui, class| {
            if !accepts_viewport_class(class) {
                // egui 0.34 deliberately falls back to an embedded Window when
                // the renderer/backend cannot create another native viewport.
                // Do not render any editor content in that callback.
                app.title_viewport_unsupported = true;
                app.set_title_page_window_open(false);
                app.title_page_focus_requested = false;
                app.show_error(
                    "TITLE_PAGE_VIEWPORT_UNSUPPORTED: this renderer cannot create a separate native window; title editor remains closed",
                );
                return;
            }

            let child_ctx = ui.ctx().clone();
            if child_ctx.input(|input| input.viewport().close_requested()) {
                app.set_title_page_window_open(false);
                return;
            }
            if child_ctx.input(|input| input.viewport().focused) == Some(true) {
                app.set_active_editor_surface(EditorSurface::TitlePage);
            }

            app.swap_title_view_state();
            let native_scale =
                child_ctx.input(|input| input.viewport().native_pixels_per_point);
            let effective_scale = status_bar::resolved_ui_pixels_per_point(native_scale);
            let scale_changed = app.effective_pixels_per_point != Some(effective_scale);
            app.effective_pixels_per_point = Some(effective_scale);
            child_ctx.set_zoom_factor(status_bar::ui_zoom_factor(native_scale));
            if scale_changed {
                app.page_geometry.invalidate();
                app.page_scale = None;
            }
            let (_, applied, palette) =
                theme::configure_appearance(&child_ctx, app.appearance_mode, app.applied_appearance);
            app.applied_appearance = applied;
            app.theme_palette = palette;
            if app.pending_fdx_open.is_none() {
                app.handle_surface_input(&child_ctx, EditorSurface::TitlePage);
            }
            show_child_panels(app, ui, &child_ctx);
            app.swap_title_view_state();
        },
    );

    if !app.title_page_window_open {
        app.close_title_page_window(root_ctx);
        app.title_view = super::TitlePageViewState::default();
        app.title_view_reset_pending = false;
    }

    if focus_requested && app.title_page_window_open && !app.title_viewport_unsupported {
        root_ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
    }
}

fn show_child_panels(app: &mut ScriptCraftApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let now = app.timer_now();
    app.reconcile_sprint_timer(now);
    let palette = app.theme_palette;

    egui::Panel::top("title-page-toolbar")
        .frame(egui::Frame::NONE.fill(palette.toolbar.background))
        .show_separator_line(false)
        .exact_size(super::toolbar::toolbar_content_height(&ctx.global_style()))
        .show_inside(ui, |ui| {
            ui.scope(|ui| {
                app.toolbar_for_surface(ui, EditorSurface::TitlePage, true);
            });
            if palette.toolbar.border != egui::Color32::TRANSPARENT {
                let rect = ui.max_rect().intersect(ui.clip_rect());
                ui.painter().line_segment(
                    [egui::pos2(rect.left(), rect.bottom() - 0.5), egui::pos2(rect.right(), rect.bottom() - 0.5)],
                    egui::Stroke::new(super::toolbar::TOOLBAR_BOTTOM_BORDER_WIDTH, palette.toolbar.border),
                );
            }
        });

    let page = app.page_geometry.visible_page(
        app.performance.frames,
        app.session.revision(),
        ctx.content_rect(),
        app.page_scale.unwrap_or(f32::NAN),
        app.reveal_intent == super::RevealIntent::Conditional
            || super::page_geometry_input_invalidated(ctx, app.page_geometry.clip()),
    );
    let status_panel = egui::Panel::bottom("title-page-status")
        .frame(status_bar::status_panel_frame(ctx, palette.status))
        .default_size(status_bar::default_panel_height())
        .show_separator_line(false)
        .show_inside(ui, |ui| {
            let intents = status_bar::show_title_page(
                ui,
                palette.status,
                page,
                app.zoom_percent,
                app.appearance_mode,
            );
            let timer_button_rect = intents.timer_button_rect;
            for command in intents.commands {
                let opening_timer = matches!(command, Command::ToolsSprintTimerTogglePopup)
                    && !app.sprint_timer.popup_open;
                app.dispatch_command(
                    CommandInvocation::new(command, CommandOrigin::Pointer),
                    ctx,
                );
                if opening_timer {
                    app.sprint_timer.popup_anchor = timer_button_rect.unwrap_or(egui::Rect::NOTHING);
                    app.timer_popup_surface = EditorSurface::TitlePage;
                }
            }
        });
    status_bar::paint_panel_top_border(ui, status_panel.response.rect, palette.status);

    egui::CentralPanel::default()
        .frame(super::script_view::central_panel_frame(ctx, palette.script_workspace))
        .show_inside(ui, |ui| app.live_document_surface(ui, EditorSurface::TitlePage));
    if app.timer_popup_surface == EditorSurface::TitlePage {
        for command in super::sprint_timer::show_popup(ctx, &mut app.sprint_timer, now) {
            app.dispatch_command(
                CommandInvocation::new(command, CommandOrigin::Pointer),
                ctx,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{accepts_viewport_class, native_title, viewport_builder, viewport_id};
    use eframe::egui;

    #[test]
    fn native_viewport_identity_and_title_are_stable_and_bounded() {
        assert_eq!(
            viewport_id(),
            egui::ViewportId::from_hash_of("scriptcraft-title-page-editor"),
        );
        let title = native_title("Untitled Screenplay");
        assert_eq!(title, "Title Page - Untitled Screenplay");
        assert_eq!(native_title("Example.fdx"), "Title Page - Example.fdx");
        let builder = viewport_builder("Untitled Screenplay");
        assert_eq!(builder.title.as_deref(), Some("Title Page - Untitled Screenplay"));
        assert_eq!(builder.inner_size, Some([820.0, 680.0].into()));
        assert_eq!(builder.min_inner_size, Some([520.0, 420.0].into()));
    }

    #[test]
    fn embedded_fallback_is_rejected_and_title_zoom_starts_at_one_hundred_percent() {
        assert!(accepts_viewport_class(egui::ViewportClass::Immediate));
        assert!(!accepts_viewport_class(egui::ViewportClass::EmbeddedWindow));
        let app = super::super::ScriptCraftApp::new();
        assert_eq!(app.map(|app| app.title_view.zoom_percent), Ok(100));
    }
}
