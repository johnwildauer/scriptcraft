//! Compact presentation for the ordered built-in template catalog.

use super::state::{
    selector_mode_can_apply, selector_mode_can_create_new, TemplateSelectorError,
    TemplateSelectorMode,
};
use eframe::egui;
use scriptcraft_fdx::{builtin_templates, BuiltinTemplateId};

const DEFAULT_WINDOW_SIZE: egui::Vec2 = egui::vec2(520.0, 390.0);
const VIEWPORT_MARGIN: f32 = 32.0;
const WINDOW_FOOTER_ALLOWANCE: f32 = 112.0;
const FOOTER_BUTTON_WIDTH: f32 = 88.0;
const FOOTER_BUTTON_HEIGHT: f32 = 30.0;
const FOOTER_BUTTON_GAP: f32 = 8.0;

const CANCEL_LABEL: &str = "Cancel";
const APPLY_LABEL: &str = "Apply";
const NEW_LABEL: &str = "New";

/// A deferred selector interaction. None of these intents loads or constructs a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemplateSelectorIntent {
    SelectionChanged(BuiltinTemplateId),
    Cancel,
    Apply(BuiltinTemplateId),
    New(BuiltinTemplateId),
}

/// Apply is reachable only in the current-document mode with a catalog selection.
pub(crate) const fn selector_can_apply(
    mode: TemplateSelectorMode,
    selected_template: Option<BuiltinTemplateId>,
) -> bool {
    selector_mode_can_apply(mode, selected_template)
}

pub(crate) const fn selector_can_new(
    mode: TemplateSelectorMode,
    selected_template: Option<BuiltinTemplateId>,
) -> bool {
    selector_mode_can_create_new(mode, selected_template)
}

/// Renders the catalog metadata and returns at most one deferred intent.
pub(crate) fn show_template_selector(
    ctx: &egui::Context,
    mode: TemplateSelectorMode,
    selected_template: Option<BuiltinTemplateId>,
    error: Option<TemplateSelectorError>,
    focus_requested: bool,
) -> Option<TemplateSelectorIntent> {
    let viewport = ctx.input(|input| input.viewport_rect());
    let (default_size, max_size) = window_sizes(viewport.size());
    let escape_pressed = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let mut open = true;
    let mut selected = selected_template;
    let mut cancel_clicked = false;
    let mut primary_clicked = false;

    egui::Window::new(selector_title(mode))
        .id(egui::Id::new("elements-template-selector-window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .default_size(default_size)
        .max_size(max_size)
        .show(ctx, |ui| {
            ui.set_max_width((max_size.x - 24.0).max(1.0));
            let list_height = (max_size.y - WINDOW_FOOTER_ALLOWANCE).max(1.0);
            egui::ScrollArea::vertical()
                .id_salt("elements-template-catalog")
                .max_height(list_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (index, metadata) in builtin_templates().iter().enumerate() {
                        ui.push_id(metadata.id.as_str(), |ui| {
                            let response = ui.radio_value(
                                &mut selected,
                                Some(metadata.id),
                                metadata.display_name,
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::RadioButton,
                                    response.enabled(),
                                    selected == Some(metadata.id),
                                    template_row_accessible_name(
                                        metadata.display_name,
                                        metadata.description,
                                    ),
                                )
                            });
                            if index == 0 && focus_requested {
                                response.request_focus();
                            }
                            ui.label(metadata.description);
                        });
                    }
                });

            if let Some(message) = selector_error_text(error) {
                ui.add_space(4.0);
                ui.colored_label(ui.visuals().error_fg_color, message);
            }

            ui.separator();
            // Place controls explicitly so keyboard traversal follows Cancel,
            // then the enabled primary action, while the visual order remains
            // bottom-right with the primary action at the far right.
            let footer_width = ui.available_width();
            let footer_rect = ui
                .allocate_space(egui::vec2(footer_width, FOOTER_BUTTON_HEIGHT))
                .1;
            let (cancel_rect, primary_rect) = footer_button_rects(footer_rect);
            let primary_enabled =
                selector_can_apply(mode, selected) || selector_can_new(mode, selected);

            let mut cancel_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(cancel_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            let cancel_response =
                cancel_ui.add_sized(cancel_rect.size(), egui::Button::new(CANCEL_LABEL));
            cancel_response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    cancel_response.enabled(),
                    CANCEL_LABEL.to_owned(),
                )
            });
            cancel_clicked = cancel_response.clicked();

            let mut primary_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(primary_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            let primary_response = primary_ui.add_enabled(
                primary_enabled,
                egui::Button::new(primary_label(mode)).min_size(primary_rect.size()),
            );
            primary_response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    primary_response.enabled(),
                    primary_label(mode).to_owned(),
                )
            });
            primary_clicked = primary_response.clicked();
        });

    // Terminal selector actions outrank any underlying Elements-window close.
    // Escape and title-button close intentionally share the Cancel intent.
    resolve_selector_intent(
        open,
        escape_pressed,
        cancel_clicked,
        primary_clicked,
        mode,
        selected_template,
        selected,
    )
}

fn resolve_selector_intent(
    window_open: bool,
    escape_pressed: bool,
    cancel_clicked: bool,
    primary_clicked: bool,
    mode: TemplateSelectorMode,
    previous_selection: Option<BuiltinTemplateId>,
    current_selection: Option<BuiltinTemplateId>,
) -> Option<TemplateSelectorIntent> {
    if !window_open || escape_pressed || cancel_clicked {
        Some(TemplateSelectorIntent::Cancel)
    } else if primary_clicked {
        match mode {
            TemplateSelectorMode::ApplyToCurrent if selector_can_apply(mode, current_selection) => {
                current_selection.map(TemplateSelectorIntent::Apply)
            }
            TemplateSelectorMode::NewDocument if selector_can_new(mode, current_selection) => {
                current_selection.map(TemplateSelectorIntent::New)
            }
            TemplateSelectorMode::ApplyToCurrent | TemplateSelectorMode::NewDocument => None,
        }
    } else if current_selection != previous_selection {
        current_selection.map(TemplateSelectorIntent::SelectionChanged)
    } else {
        None
    }
}

fn selector_title(mode: TemplateSelectorMode) -> &'static str {
    match mode {
        TemplateSelectorMode::ApplyToCurrent => "Apply a Template",
        TemplateSelectorMode::NewDocument => "New from Template",
    }
}

fn primary_label(mode: TemplateSelectorMode) -> &'static str {
    match mode {
        TemplateSelectorMode::ApplyToCurrent => APPLY_LABEL,
        TemplateSelectorMode::NewDocument => NEW_LABEL,
    }
}

fn selector_error_text(error: Option<TemplateSelectorError>) -> Option<&'static str> {
    match error? {
        TemplateSelectorError::TargetLoad => {
            Some("The selected template could not be loaded. The current document was not changed.")
        }
        TemplateSelectorError::CurrentDocumentEdit => {
            Some("The template could not be applied. The current document was not changed.")
        }
        TemplateSelectorError::NewDocumentLaunch => Some("The new document could not be opened."),
    }
}

fn window_sizes(viewport: egui::Vec2) -> (egui::Vec2, egui::Vec2) {
    let max_size = egui::vec2(
        (viewport.x - VIEWPORT_MARGIN).max(1.0),
        (viewport.y - VIEWPORT_MARGIN).max(1.0),
    );
    let default_size = egui::vec2(
        DEFAULT_WINDOW_SIZE.x.min(max_size.x),
        DEFAULT_WINDOW_SIZE.y.min(max_size.y),
    );
    (default_size, max_size)
}

fn footer_button_rects(footer: egui::Rect) -> (egui::Rect, egui::Rect) {
    let button_gap = FOOTER_BUTTON_GAP.min((footer.width() - 2.0).max(0.0));
    let button_width = ((footer.width() - button_gap).max(0.0) * 0.5).min(FOOTER_BUTTON_WIDTH);
    let cancel = egui::Rect::from_min_size(
        egui::pos2(
            footer.right() - button_width * 2.0 - button_gap,
            footer.center().y - FOOTER_BUTTON_HEIGHT * 0.5,
        ),
        egui::vec2(button_width, FOOTER_BUTTON_HEIGHT),
    );
    let primary = egui::Rect::from_min_size(
        egui::pos2(
            footer.right() - button_width,
            footer.center().y - FOOTER_BUTTON_HEIGHT * 0.5,
        ),
        egui::vec2(button_width, FOOTER_BUTTON_HEIGHT),
    );
    (cancel, primary)
}

fn template_row_accessible_name(display_name: &str, description: &str) -> String {
    format!("{display_name} — {description}")
}
