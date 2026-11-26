//! Separate native New Type dialog for the ScriptNote card.

use super::{geometry, state::ScriptNoteWorkspaceState};
use crate::app::ScriptCraftApp;
use crate::chrome::theme::{self, AppearanceContext};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};

const VIEWPORT_NAME: &str = "scriptcraft-script-note-type";
const DIALOG_SIZE: Vec2 = Vec2::new(320.0, 124.0);
const CONTENT_PADDING: f32 = 12.0;
const LABEL_HEIGHT: f32 = 28.0;
const INPUT_HEIGHT: f32 = 24.0;
const BUTTON_SIZE: Vec2 = Vec2::new(80.0, 20.0);
const BUTTON_GAP: f32 = 16.0;
const INPUT_BUTTON_GAP: f32 = 22.0;
const DIALOG_RADIUS: u8 = 8;
const ERROR_TEXT: &str = "Unable to apply Type. The note was not changed.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TypeDialogAction {
    Cancel,
    Apply(String),
}

pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of(VIEWPORT_NAME)
}

pub(crate) fn close(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Close);
}

pub(crate) fn position(state: &ScriptNoteWorkspaceState) -> Option<Pos2> {
    let note = state.native_position?;
    let centered = Pos2::new(
        note.x + (geometry::CARD_SIZE.x - DIALOG_SIZE.x) * 0.5,
        note.y + (geometry::CARD_SIZE.y - DIALOG_SIZE.y) * 0.5,
    );
    Some(geometry::clamp_origin(
        centered,
        DIALOG_SIZE,
        state.work_bounds.map(|bounds| bounds.rect()),
    ))
}

pub(crate) fn rect(state: &ScriptNoteWorkspaceState) -> Option<Rect> {
    position(state).map(|position| Rect::from_min_size(position, DIALOG_SIZE))
}

fn set_button_visuals(
    ui: &mut egui::Ui,
    inactive_fill: Color32,
    hovered_fill: Color32,
    active_fill: Color32,
    foreground: Color32,
    border: Color32,
    hovered_border: Color32,
    active_border: Color32,
) {
    let visuals = &mut ui.style_mut().visuals.widgets;
    visuals.inactive.bg_fill = inactive_fill;
    visuals.inactive.weak_bg_fill = inactive_fill;
    visuals.inactive.fg_stroke.color = foreground;
    visuals.inactive.bg_stroke = Stroke::new(1.0, border);
    visuals.hovered.bg_fill = hovered_fill;
    visuals.hovered.weak_bg_fill = hovered_fill;
    visuals.hovered.fg_stroke.color = foreground;
    visuals.hovered.bg_stroke = Stroke::new(1.0, hovered_border);
    visuals.active.bg_fill = active_fill;
    visuals.active.weak_bg_fill = active_fill;
    visuals.active.fg_stroke.color = foreground;
    visuals.active.bg_stroke = Stroke::new(1.0, active_border);
    visuals.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.active.corner_radius = egui::CornerRadius::same(6);
}

pub(crate) fn show(
    app: &mut ScriptCraftApp,
    root_ctx: &egui::Context,
) -> Option<TypeDialogAction> {
    if !app.script_note_workspace.new_type_dialog_open {
        if app.script_note_workspace.type_dialog_close_pending {
            close(root_ctx);
            app.script_note_workspace.type_dialog_close_pending = false;
        }
        return None;
    }
    let position = position(&app.script_note_workspace);
    let builder = egui::ViewportBuilder::default()
        .with_title("New ScriptNote Type")
        .with_inner_size(DIALOG_SIZE)
        .with_min_inner_size(DIALOG_SIZE)
        .with_max_inner_size(DIALOG_SIZE)
        .with_resizable(false)
        .with_decorations(true)
        .with_title_shown(true)
        .with_titlebar_shown(true)
        .with_titlebar_buttons_shown(false);
    let builder = position.map_or(builder.clone(), |position| builder.with_position(position));
    let mut action = None;
    let mut focused = false;
    let mut request_input_focus = false;
    root_ctx.show_viewport_immediate(viewport_id(), builder, |viewport_ui, class| {
        if class != egui::ViewportClass::Immediate {
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
        let mut ui = viewport_ui.new_child(
            egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::ZERO, DIALOG_SIZE)),
        );
        ui.set_min_size(DIALOG_SIZE);
        let dialog_rect = Rect::from_min_size(Pos2::ZERO, DIALOG_SIZE);
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
        let label_rect = Rect::from_min_size(
            Pos2::new(content_left, CONTENT_PADDING),
            Vec2::new(content_width, LABEL_HEIGHT),
        );
        painter.text(
            label_rect.left_center(), 
            egui::Align2::LEFT_CENTER, 
            "Enter Type:", 
            ui.style()
            .text_styles[&egui::TextStyle::Body].clone(), 
            ui.visuals().text_color());
        // ui.put(
        //     label_rect,
        //     egui::Label::new("Enter Type:").halign(egui::Align::Min),
        // );
        let input_rect = Rect::from_min_size(
            Pos2::new(content_left, label_rect.bottom() + 4.0),
            Vec2::new(content_width, INPUT_HEIGHT),
        );
        let response = ui.put(
            input_rect,
            egui::TextEdit::singleline(&mut app.script_note_workspace.type_draft)
                .id(egui::Id::new("scriptcraft-script-note-type-input"))
                .desired_width(content_width)
                .vertical_align(egui::Align::Center),
        );
        let button_top = input_rect.bottom() + INPUT_BUTTON_GAP;
        if !app.script_note_workspace.type_dialog_focused {
            request_input_focus = true;
        }
        if request_input_focus {
            response.request_focus();
        }
        if (response.has_focus() || response.lost_focus())
            && ui.input(|input| input.key_pressed(egui::Key::Enter))
        {
            action = Some(TypeDialogAction::Apply(app.script_note_workspace.type_draft.clone()));
        }
        if ui.input(|input| input.key_pressed(egui::Key::Escape))
            || ui.input(|input| input.viewport().close_requested())
        {
            action = Some(TypeDialogAction::Cancel);
        }
        if app.script_note_workspace.type_dialog_error {
            let error_top = input_rect.bottom() + 3.0;
            let error_rect = Rect::from_min_max(
                Pos2::new(content_left, error_top),
                Pos2::new(DIALOG_SIZE.x - CONTENT_PADDING, button_top - 3.0),
            );
            ui.put(
                error_rect,
                egui::Label::new(
                    egui::RichText::new(ERROR_TEXT)
                        .size(11.0)
                        .color(palette.feedback.error),
                )
                .truncate()
                .halign(egui::Align::Min),
            );
        }
        let ok_left = DIALOG_SIZE.x - CONTENT_PADDING - BUTTON_SIZE.x;
        let cancel_left = ok_left - BUTTON_GAP - BUTTON_SIZE.x;
        let cancel_rect = Rect::from_min_size(Pos2::new(cancel_left, button_top), BUTTON_SIZE);
        let ok_rect = Rect::from_min_size(Pos2::new(ok_left, button_top), BUTTON_SIZE);
        let cancel_clicked = ui.scope(|ui| {
            set_button_visuals(
                ui,
                palette.controls.inactive_fill,
                palette.controls.hovered_fill,
                palette.controls.active_fill,
                palette.controls.inactive_foreground,
                palette.controls.inactive_border,
                palette.controls.hovered_border,
                palette.controls.active_border,
            );
            ui.put(
                cancel_rect,
                egui::Button::new(egui::RichText::new("Cancel").size(13.0))
                    .min_size(BUTTON_SIZE),
            )
        }).inner.clicked();
        if cancel_clicked {
            action = Some(TypeDialogAction::Cancel);
        }
        let ok_clicked = ui.scope(|ui| {
            set_button_visuals(
                ui,
                Color32::from_rgb(0, 122, 255),
                Color32::from_rgb(0, 105, 225),
                Color32::from_rgb(0, 88, 195),
                Color32::WHITE,
                Color32::from_rgb(0, 105, 225),
                Color32::from_rgb(0, 88, 195),
                Color32::from_rgb(0, 72, 165),
            );
            ui.put(
                ok_rect,
                egui::Button::new(egui::RichText::new("OK").size(13.0).color(Color32::WHITE))
                    .min_size(BUTTON_SIZE),
            )
        }).inner.clicked();
        if ok_clicked {
            action = Some(TypeDialogAction::Apply(app.script_note_workspace.type_draft.clone()));
        }
        ctx.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                egui::Id::new(viewport_id()),
                egui::EventFilter { tab: true, escape: true, ..Default::default() },
            );
        });
    });
    app.script_note_workspace.type_dialog_focused = focused;
    action
}
