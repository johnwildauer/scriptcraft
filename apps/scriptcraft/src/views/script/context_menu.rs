//! Body Script View context popup, composed on the editor's single response.

use super::ScriptCraftApp;
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
            let max_height = (ui.ctx().content_rect().height() - 24.0).max(0.0);
            egui::ScrollArea::vertical()
                .max_height(max_height)
                .min_scrolled_height(0.0)
                .auto_shrink([true, true])
                .show(ui, |ui| {
                    for choice in projection.choices() {
                        let checked = projection.focused_type_key.as_ref() == Some(&choice.key);
                        let label = if checked {
                            format!("✓ {}", choice.label())
                        } else {
                            choice.label().to_owned()
                        };
                        let response = ui.button(label);
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
