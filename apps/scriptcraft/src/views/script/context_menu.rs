//! Body Script View context popup, composed on the editor's single response.

use super::ScriptCraftApp;
use crate::chrome::menu::{
    available_popup_inner_viewport_height, menu_check_atom, menu_popup_row_height,
    popup_rows_content_height, render_popup_content,
};
use crate::chrome::element_choices::DynamicElementMenuProjection;
use crate::commands::{command_for_id, CommandInvocation};
use eframe::egui;
use scriptcraft_commands::{
    Command, CommandId, CommandOrigin, EditorSurface, ElementTypeValue,
};

impl ScriptCraftApp {
    pub(super) fn show_body_context_menu(
        &mut self,
        response: &egui::Response,
        ctx: &egui::Context,
    ) {
        let availability = self.app_command_availability();
        // Paste is the ordinary editing command whose shared availability gates
        // include modal, replacement, output, and element-window ownership.
        if !availability.is_enabled(CommandId::EditPaste) {
            return;
        }
        let elements = self.dynamic_element_menu_projection();
        let mut selected_command = None;
        response.context_menu(|ui| {
            for (label, command_id) in [
                ("Cut", CommandId::EditCut),
                ("Copy", CommandId::EditCopy),
                ("Paste", CommandId::EditPaste),
                ("Delete", CommandId::EditDelete),
            ] {
                context_command_row(ui, label, command_id, availability, &mut selected_command);
            }
            context_command_row(
                ui,
                "ScriptNote",
                CommandId::ScriptNoteCreate,
                availability,
                &mut selected_command,
            );
            ui.separator();
            for (label, command_id) in [
                ("Undo", CommandId::EditUndo),
                ("Redo", CommandId::EditRedo),
            ] {
                context_command_row(ui, label, command_id, availability, &mut selected_command);
            }
            ui.separator();
            context_command_row(
                ui,
                "Select All",
                CommandId::EditSelectAll,
                availability,
                &mut selected_command,
            );
            ui.separator();
            render_element_submenu(ui, &elements, availability, &mut selected_command);
        });

        if let Some(command) = selected_command {
            self.dispatch_command(
                CommandInvocation::new(command, CommandOrigin::Pointer),
                ctx,
            );
        }
    }
}

fn context_command_row(
    ui: &mut egui::Ui,
    label: &str,
    command_id: CommandId,
    availability: crate::commands::AppCommandAvailability,
    selected_command: &mut Option<Command>,
) {
    let enabled = availability.is_enabled(command_id);
    let response = ui.add_enabled(enabled, egui::Button::new(label));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label.to_owned())
    });
    if response.clicked() {
        *selected_command = command_for_id(command_id);
        ui.close();
    }
}

fn render_element_submenu(
    ui: &mut egui::Ui,
    projection: &DynamicElementMenuProjection,
    availability: crate::commands::AppCommandAvailability,
    selected_command: &mut Option<Command>,
) {
    let enabled = availability.is_enabled(CommandId::EditorElementChange)
        && !projection.choices().is_empty();
    ui.add_enabled_ui(enabled, |ui| {
        ui.menu_button("Change Element To", |ui| {
            let choices = projection.choices();
            let row_height = menu_popup_row_height();
            let row_gap = ui.spacing().item_spacing.y;
            let content_height = popup_rows_content_height(choices.len(), row_height, row_gap);
            let margin = ui.spacing().menu_margin;
            let vertical_margin =
                f32::from(margin.top) + f32::from(margin.bottom);
            let available_height = available_popup_inner_viewport_height(ui, vertical_margin);
            render_popup_content(ui, content_height, available_height, |ui| {
                    for choice in choices {
                        let checked = projection.focused_type_key.as_ref() == Some(&choice.key);
                        let mut label_atom = egui::Atom::from(choice.label());
                        label_atom.grow = true;
                        label_atom.align = egui::Align2::LEFT_CENTER;
                        let response = ui.add(
                            egui::Button::new((menu_check_atom(checked), label_atom))
                                .min_size(egui::vec2(0.0, row_height)),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                choice.label().to_owned(),
                            )
                        });
                        if response.clicked() {
                            *selected_command = Some(Command::EditorElementChange {
                                element_type: ElementTypeValue::from_key(choice.key.clone()),
                                surface: EditorSurface::Body,
                            });
                            ui.close();
                        }
                    }
                });
        });
    });
}
