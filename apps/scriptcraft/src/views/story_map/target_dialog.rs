//! Story Map-owned native Target Script Length dialog and transient state.

use crate::app::ScriptCraftApp;
use crate::chrome::theme::{self, AppearanceContext};
use crate::commands::{CommandInvocation, DesktopCommandOutcome};
use eframe::egui::{self, Pos2, Rect, Stroke, StrokeKind, Vec2};
use scriptcraft_commands::{Command, CommandOrigin, CommandOutcome, CommandRejection, StoryMapTargetPagesValue};

use super::constants::{STORY_MAP_TARGET_DIALOG_HEIGHT, STORY_MAP_TARGET_DIALOG_WIDTH};

const VIEWPORT_NAME: &str = "scriptcraft-story-map-target-length";
const CONTENT_PADDING: f32 = 12.0;
const CAPTION_HEIGHT: f32 = 20.0;
const INPUT_HEIGHT: f32 = 24.0;
const BUTTON_SIZE: Vec2 = Vec2::new(80.0, 20.0);
const BUTTON_GAP: f32 = 12.0;
const DIALOG_RADIUS: u8 = 8;
const INPUT_ERROR: &str = "Enter a whole page count from 1 to 10,000.";
const APPLY_ERROR: &str = "Unable to change target script length. The document was not changed.";

const DIALOG_SIZE: Vec2 = Vec2::new(STORY_MAP_TARGET_DIALOG_WIDTH, STORY_MAP_TARGET_DIALOG_HEIGHT);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetDialogError {
    InvalidInput,
    MinimumPages(u32),
    Unavailable,
}

impl TargetDialogError {
    fn text(self) -> String {
        match self {
            Self::InvalidInput => INPUT_ERROR.to_owned(),
            Self::MinimumPages(pages) => format!(
                "Target length must be at least {pages} pages to contain placed Beats."
            ),
            Self::Unavailable => APPLY_ERROR.to_owned(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct StoryMapTargetDialogState {
    pub(crate) open: bool,
    draft: String,
    error: Option<TargetDialogError>,
    close_pending: bool,
    focused: bool,
    anchor: Option<Pos2>,
}

impl StoryMapTargetDialogState {
    pub(crate) fn owns_input(&self) -> bool {
        self.open || self.close_pending
    }

    pub(crate) fn reset(&mut self) {
        let was_open = self.open;
        self.open = false;
        self.draft.clear();
        self.error = None;
        self.focused = false;
        self.anchor = None;
        self.close_pending |= was_open;
    }

    pub(crate) fn dismiss(&mut self) {
        self.open = false;
        self.draft.clear();
        self.error = None;
        self.focused = false;
        self.anchor = None;
        self.close_pending = true;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetDialogAction {
    Cancel,
    Apply(String),
}

pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of(VIEWPORT_NAME)
}

pub(crate) fn close(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Close);
}

/// Opens from either Story Map presentation entry point without touching semantic state.
pub(crate) fn open(app: &mut ScriptCraftApp, ctx: &egui::Context, anchor: Pos2) -> bool {
    let availability = app.app_command_availability();
    if availability.modal_blocked
        || availability.replacement_pending
        || availability.native_output_active
        || app.element_windows.owns_input()
        || app.pending_fdx_open.is_some()
        || app.title_view.surface_focused
        || app.title_view_state_swapped
        || app.active_editor_surface == scriptcraft_commands::EditorSurface::TitlePage
        || app.title_page_window_open
        || app.script_note_workspace.open_note.is_some()
        || app.script_note_workspace.new_type_dialog_open
        || app.sprint_timer.popup_open
        || (app.menu_policy.in_window_shortcuts_owned()
            && crate::chrome::menu::in_window_menu_popup_open(ctx))
        || ctx.input(|input| input.viewport().focused == Some(false))
    {
        return false;
    }
    let root_origin = ctx
        .input(|input| input.viewport().outer_rect.map(|rect| rect.min))
        .unwrap_or(Pos2::ZERO);
    let content_origin = ctx.content_rect().min;
    app.story_map_target_dialog.open = true;
    app.story_map_target_dialog.draft = app.session.project().story_map().target_pages.to_string();
    app.story_map_target_dialog.error = None;
    app.story_map_target_dialog.close_pending = false;
    app.story_map_target_dialog.focused = false;
    app.story_map_target_dialog.anchor = Some(root_origin + (anchor - content_origin));
    app.story_map_preview.close();
    app.story_map_context_menu.close();
    app.command_projection_cache = None;
    ctx.request_repaint();
    true
}

pub(crate) fn show(app: &mut ScriptCraftApp, root_ctx: &egui::Context) -> Option<TargetDialogAction> {
    let availability = app.app_command_availability();
    let higher_priority_owner = availability.modal_blocked
        || availability.replacement_pending
        || availability.native_output_active
        || app.element_windows.owns_input()
        || app.pending_fdx_open.is_some()
        || app.title_view.surface_focused
        || app.title_view_state_swapped
        || app.active_editor_surface == scriptcraft_commands::EditorSurface::TitlePage
        || app.title_page_window_open
        || app.script_note_workspace.open_note.is_some()
        || app.script_note_workspace.new_type_dialog_open
        || app.sprint_timer.popup_open;
    if higher_priority_owner {
        if app.story_map_target_dialog.open {
            app.story_map_target_dialog.reset();
            app.command_projection_cache = None;
        }
        if app.story_map_target_dialog.close_pending {
            close(root_ctx);
            app.story_map_target_dialog.close_pending = false;
        }
        app.story_map_context_menu.close();
        return None;
    }
    if !app.story_map_target_dialog.open {
        if app.story_map_target_dialog.close_pending {
            close(root_ctx);
            app.story_map_target_dialog.close_pending = false;
        }
        return None;
    }

    let position = app.story_map_target_dialog.anchor.map(|anchor| {
        let bounds = root_ctx
            .input(|input| input.viewport().outer_rect)
            .unwrap_or_else(|| root_ctx.content_rect());
        clamp_origin(
            anchor + Vec2::new(12.0, 14.0),
            DIALOG_SIZE + Vec2::new(0.0, 32.0),
            bounds,
        )
    });
    let mut builder = egui::ViewportBuilder::default()
        .with_title("Target Script Length")
        .with_inner_size(DIALOG_SIZE)
        .with_min_inner_size(DIALOG_SIZE)
        .with_max_inner_size(DIALOG_SIZE)
        .with_resizable(false)
        .with_decorations(true)
        .with_title_shown(true)
        .with_titlebar_shown(true)
        .with_titlebar_buttons_shown(false);
    if let Some(position) = position {
        builder = builder.with_position(position);
    }

    let request_focus = !app.story_map_target_dialog.focused;
    let mut action = None;
    let mut focused = false;
    let mut unsupported = false;
    root_ctx.show_viewport_immediate(viewport_id(), builder, |viewport_ui, class| {
        if class != egui::ViewportClass::Immediate {
            unsupported = true;
            return;
        }
        let ctx = viewport_ui.ctx().clone();
        focused = ctx.input(|input| input.viewport().focused) == Some(true);
        let native_scale = ctx.input(|input| input.viewport().native_pixels_per_point);
        ctx.set_zoom_factor(crate::chrome::status_bar::ui_zoom_factor(native_scale));
        let (_, _, palette) = theme::configure_appearance(
            &ctx,
            app.appearance_mode,
            app.appearance_applications.get(AppearanceContext::Root),
        );
        let dialog_rect = Rect::from_min_size(Pos2::ZERO, DIALOG_SIZE);
        let mut ui = viewport_ui.new_child(egui::UiBuilder::new().max_rect(dialog_rect));
        ui.set_min_size(DIALOG_SIZE);
        let painter = ui.painter();
        painter.rect_filled(
            dialog_rect,
            egui::CornerRadius::same(DIALOG_RADIUS),
            palette.overlays.window_background,
        );
        painter.rect_stroke(
            dialog_rect,
            egui::CornerRadius::same(DIALOG_RADIUS),
            Stroke::new(1.0, palette.overlays.window_border),
            StrokeKind::Inside,
        );

        let content_left = CONTENT_PADDING;
        let content_width = DIALOG_SIZE.x - CONTENT_PADDING * 2.0;
        let caption_rect = Rect::from_min_size(
            Pos2::new(content_left, CONTENT_PADDING),
            Vec2::new(content_width, CAPTION_HEIGHT),
        );
        painter.text(
            caption_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            "Pages:",
            ui.style().text_styles[&egui::TextStyle::Body].clone(),
            ui.visuals().text_color(),
        );
        let input_rect = Rect::from_min_size(
            Pos2::new(content_left, caption_rect.bottom() + 2.0),
            Vec2::new(content_width, INPUT_HEIGHT),
        );
        let response = ui.put(
            input_rect,
            egui::TextEdit::singleline(&mut app.story_map_target_dialog.draft)
                .id(egui::Id::new("story-map-target-pages-input"))
                .desired_width(content_width)
                .vertical_align(egui::Align::Center),
        );
        if request_focus {
            response.request_focus();
        }
        let button_top = DIALOG_SIZE.y - CONTENT_PADDING - BUTTON_SIZE.y;
        if let Some(error) = app.story_map_target_dialog.error {
            let error_rect = Rect::from_min_max(
                Pos2::new(content_left, input_rect.bottom() + 3.0),
                Pos2::new(DIALOG_SIZE.x - CONTENT_PADDING, button_top - 3.0),
            );
            ui.put(
                error_rect,
                egui::Label::new(
                    egui::RichText::new(error.text())
                        .size(11.0)
                        .color(palette.feedback.error),
                )
                .truncate()
                .halign(egui::Align::Min),
            );
        }
        if (response.has_focus() || response.lost_focus())
            && ui.input(|input| input.key_pressed(egui::Key::Enter))
        {
            action = Some(TargetDialogAction::Apply(
                app.story_map_target_dialog.draft.clone(),
            ));
        }
        if ui.input(|input| input.key_pressed(egui::Key::Escape))
            || ui.input(|input| input.viewport().close_requested())
        {
            action = Some(TargetDialogAction::Cancel);
        }

        let ok_left = DIALOG_SIZE.x - CONTENT_PADDING - BUTTON_SIZE.x;
        let cancel_left = ok_left - BUTTON_GAP - BUTTON_SIZE.x;
        let cancel_rect = Rect::from_min_size(Pos2::new(cancel_left, button_top), BUTTON_SIZE);
        let ok_rect = Rect::from_min_size(Pos2::new(ok_left, button_top), BUTTON_SIZE);
        let cancel_clicked = ui.scope(|ui| {
            set_application_control_visuals(ui, palette);
            ui.put(
                cancel_rect,
                egui::Button::new(egui::RichText::new("Cancel").size(13.0))
                    .min_size(BUTTON_SIZE),
            )
        }).inner.clicked();
        if cancel_clicked {
            action = Some(TargetDialogAction::Cancel);
        }
        let ok_clicked = ui.scope(|ui| {
            set_application_control_visuals(ui, palette);
            ui.put(
                ok_rect,
                egui::Button::new(egui::RichText::new("OK").size(13.0))
                    .min_size(BUTTON_SIZE),
            )
        }).inner.clicked();
        if ok_clicked {
            action = Some(TargetDialogAction::Apply(
                app.story_map_target_dialog.draft.clone(),
            ));
        }
        ctx.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                egui::Id::new(viewport_id()),
                egui::EventFilter { tab: true, escape: true, ..Default::default() },
            );
        });
    });
    app.story_map_target_dialog.focused = focused;
    if unsupported {
        app.story_map_target_dialog.error = Some(TargetDialogError::Unavailable);
    }
    action
}

pub(crate) fn apply(app: &mut ScriptCraftApp, ctx: &egui::Context, draft: String) {
    let requested = draft.trim().parse::<u32>().ok()
        .and_then(StoryMapTargetPagesValue::new);
    let Some(target_pages) = requested else {
        app.story_map_target_dialog.error = Some(TargetDialogError::InvalidInput);
        ctx.request_repaint();
        return;
    };
    let result = app.dispatch_command(
        CommandInvocation::new(
            Command::StoryMapTargetSet { target_pages },
            CommandOrigin::Pointer,
        ),
        ctx,
    );
    match result.outcome {
        DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp) => {
            app.story_map_target_dialog.dismiss();
            app.command_projection_cache = None;
        }
        DesktopCommandOutcome::Rejected(CommandRejection::InvalidStoryMapTarget) => {
            app.story_map_target_dialog.error = minimum_required_pages(app)
                .map(TargetDialogError::MinimumPages)
                .or(Some(TargetDialogError::Unavailable));
        }
        _ => app.story_map_target_dialog.error = Some(TargetDialogError::Unavailable),
    }
    ctx.request_repaint();
}

pub(crate) fn cancel(app: &mut ScriptCraftApp) {
    app.story_map_target_dialog.dismiss();
    app.command_projection_cache = None;
}

fn minimum_required_pages(app: &ScriptCraftApp) -> Option<u32> {
    let maximum_end = app.session.project().beat_board().cards.iter()
        .filter_map(|card| card.story_map_placement.as_ref())
        .try_fold(0_u64, |maximum, placement| {
            placement.checked_end_eighths().map(|end| maximum.max(end))
        })?;
    let pages = maximum_end / 8 + if maximum_end % 8 == 0 { 0 } else { 1 };
    u32::try_from(pages).ok()
}

fn set_application_control_visuals(ui: &mut egui::Ui, palette: theme::ApplicationPalette) {
    let controls = palette.controls;
    let visuals = &mut ui.style_mut().visuals.widgets;
    visuals.inactive.bg_fill = controls.inactive_fill;
    visuals.inactive.weak_bg_fill = controls.inactive_fill;
    visuals.inactive.fg_stroke.color = controls.inactive_foreground;
    visuals.inactive.bg_stroke = Stroke::new(1.0, controls.inactive_border);
    visuals.hovered.bg_fill = controls.hovered_fill;
    visuals.hovered.weak_bg_fill = controls.hovered_fill;
    visuals.hovered.fg_stroke.color = controls.inactive_foreground;
    visuals.hovered.bg_stroke = Stroke::new(1.0, controls.hovered_border);
    visuals.active.bg_fill = controls.active_fill;
    visuals.active.weak_bg_fill = controls.active_fill;
    visuals.active.fg_stroke.color = controls.inactive_foreground;
    visuals.active.bg_stroke = Stroke::new(1.0, controls.active_border);
    visuals.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.active.corner_radius = egui::CornerRadius::same(6);
}

fn clamp_origin(anchor: Pos2, size: Vec2, bounds: Rect) -> Pos2 {
    let maximum = bounds.max - size;
    Pos2::new(
        anchor.x.clamp(bounds.left(), maximum.x.max(bounds.left())),
        anchor.y.clamp(bounds.top(), maximum.y.max(bounds.top())),
    )
}
