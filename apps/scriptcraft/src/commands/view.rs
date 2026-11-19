use crate::app::ScriptCraftApp;
use eframe::egui;
use scriptcraft_commands::{Appearance, Command, CommandRejection, EditorSurface};

use super::DesktopCommandOutcome;

impl ScriptCraftApp {
    pub(super) fn dispatch_view_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        match command {
            Command::WindowTitlePageOpen => {
                self.open_title_page_window(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::WindowTitlePageClose => {
                self.close_title_page_window(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewZoomSet { percent } => {
                let next = percent.get();
                self.set_zoom_for_surface(self.active_editor_surface(), next, ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewZoomPinch { factor } => {
                let surface = self.active_editor_surface();
                if let Some(next) = crate::chrome::status_bar::zoom_after_native_factor(
                    self.pinch_zoom_for_surface(surface),
                    factor.get(),
                ) {
                    let next_zoom = crate::chrome::status_bar::normalized_zoom_percent(next);
                    self.set_zoom_for_surface(surface, next_zoom, ctx);
                    self.set_pinch_zoom_for_surface(surface, next);
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewScriptViewSet { mode } => {
                if self.script_view_mode != mode {
                    self.capture_pending_zoom_anchor(ctx);
                    self.script_view_mode = mode;
                    self.invalidate_surface(EditorSurface::Body);
                    ctx.request_repaint();
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewAppearanceSet { appearance } => {
                let appearance = match appearance {
                    Appearance::System => crate::chrome::theme::AppearanceMode::System,
                    Appearance::Day => crate::chrome::theme::AppearanceMode::Day,
                    Appearance::Night => crate::chrome::theme::AppearanceMode::Night,
                    Appearance::Midnight => crate::chrome::theme::AppearanceMode::Midnight,
                };
                if self.appearance_mode != appearance {
                    self.appearance_mode = appearance;
                    ctx.request_repaint();
                }
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ViewFocusModeToggle => {
                if let Some(request) = self.focus_mode.request(self.timer_now()) {
                    DesktopViewportEffects::fullscreen(ctx, request.target);
                }
                DesktopCommandOutcome::EffectDispatched
            }
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    pub(crate) fn restore_imported_window(
        &self,
        commands: [egui::ViewportCommand; 2],
        ctx: &egui::Context,
    ) {
        DesktopViewportEffects::send(ctx, commands);
    }
}

/// The viewport adapter owns native close/restore requests; imported geometry
/// never becomes semantic screenplay state or a command payload.
pub(crate) struct DesktopViewportEffects;

impl DesktopViewportEffects {
    pub(crate) fn title(ctx: &egui::Context, title: String) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    pub(super) fn close_root(ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn fullscreen(ctx: &egui::Context, target: bool) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(target));
    }

    fn send(ctx: &egui::Context, commands: [egui::ViewportCommand; 2]) {
        for command in commands {
            ctx.send_viewport_cmd(command);
        }
    }
}
