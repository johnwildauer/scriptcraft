//! Separate native New Type dialog for the ScriptNote card.

use super::{geometry, state::ScriptNoteWorkspaceState};
use crate::app::ScriptCraftApp;
use eframe::egui::{self, Pos2, Rect, Vec2};

const VIEWPORT_NAME: &str = "scriptcraft-script-note-type";
const DIALOG_SIZE: Vec2 = Vec2::new(320.0, 170.0);

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
        .with_resizable(false);
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
        let mut ui = viewport_ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::ZERO, DIALOG_SIZE)));
        ui.set_min_size(DIALOG_SIZE);
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.heading("New ScriptNote Type");
        });
        ui.add_space(7.0);
        ui.label("Enter Type:");
        let response = ui.add_sized(
            Vec2::new(296.0, 26.0),
            egui::TextEdit::singleline(&mut app.script_note_workspace.type_draft)
                .id(egui::Id::new("scriptcraft-script-note-type-input")),
        );
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
            ui.colored_label(ui.visuals().error_fg_color, "Unable to apply Type. The note was not changed.");
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = Some(TypeDialogAction::Cancel);
                }
                if ui.button("OK").clicked() {
                    action = Some(TypeDialogAction::Apply(app.script_note_workspace.type_draft.clone()));
                }
            });
        });
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
