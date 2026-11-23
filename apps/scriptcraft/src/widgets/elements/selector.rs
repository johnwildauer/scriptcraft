//! Compact presentation for the ordered built-in template catalog.

use super::state::{
    selector_mode_can_apply, selector_mode_can_create_new, TemplateSelectorError,
    TemplateSelectorMode,
};
use super::presentation::{
    self, detail_frame, listbox_frame, selectable_row, DEFAULT_SELECTOR_SIZE,
    FOOTER_BUTTON_GAP, FOOTER_BUTTON_HEIGHT, FOOTER_BUTTON_WIDTH, MIN_PANE_WIDTH,
    MIN_SELECTOR_SIZE, SELECTOR_FOOTER_ALLOWANCE, WINDOW_CONTENT_HORIZONTAL_ALLOWANCE,
};
use crate::chrome::{status_bar, theme};
use eframe::egui;
use scriptcraft_fdx::{
    builtin_template_metadata, builtin_template_preview_description, builtin_templates,
    BuiltinTemplateId,
};

const CANCEL_LABEL: &str = "Cancel";
const APPLY_LABEL: &str = "Apply";
const NEW_LABEL: &str = "New";

pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("scriptcraft-elements-template-selector")
}

fn viewport_builder(mode: TemplateSelectorMode) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(selector_title(mode))
        .with_inner_size(DEFAULT_SELECTOR_SIZE)
        .with_min_inner_size(MIN_SELECTOR_SIZE)
        .with_resizable(true)
}

pub(crate) fn send_close(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Close);
}

pub(crate) fn send_focus(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
}

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

/// Shows the native catalog/preview viewport and returns an intent or unsupported outcome.
pub(crate) fn show_template_selector(
    root_ctx: &egui::Context,
    mode: TemplateSelectorMode,
    selected_template: Option<BuiltinTemplateId>,
    error: Option<TemplateSelectorError>,
    focus_requested: bool,
    appearance_mode: theme::AppearanceMode,
    applied_appearance: theme::AppliedAppearance,
) -> Result<(Option<TemplateSelectorIntent>, theme::AppliedAppearance), ()> {
    let mut outcome = None;
    root_ctx.show_viewport_immediate(viewport_id(), viewport_builder(mode), |viewport_ui, class| {
        if class != egui::ViewportClass::Immediate {
            outcome = Some(Err(()));
            return;
        }
        let child_ctx = viewport_ui.ctx().clone();
        if child_ctx.input(|input| input.viewport().close_requested()) {
            outcome = Some(Ok((Some(TemplateSelectorIntent::Cancel), applied_appearance)));
            return;
        }
        let escape_pressed = child_ctx.input(|input| input.key_pressed(egui::Key::Escape));
        if escape_pressed {
            outcome = Some(Ok((Some(TemplateSelectorIntent::Cancel), applied_appearance)));
            return;
        }

        let native_scale = child_ctx.input(|input| input.viewport().native_pixels_per_point);
        child_ctx.set_zoom_factor(status_bar::ui_zoom_factor(native_scale));
        let (_, applied, _) = theme::configure_appearance(
            &child_ctx,
            appearance_mode,
            applied_appearance,
        );
        egui::CentralPanel::default().show_inside(viewport_ui, |ui| {
            let mut selected = selected_template;
            let mut cancel_clicked = false;
            let mut primary_clicked = false;
            show_selector_content(
                ui,
                mode,
                error,
                focus_requested,
                &mut selected,
                &mut cancel_clicked,
                &mut primary_clicked,
            );
            outcome = Some(Ok((
                resolve_selector_intent(
                    true,
                    false,
                    cancel_clicked,
                    primary_clicked,
                    mode,
                    selected_template,
                    selected,
                ),
                applied,
            )));
        });
    });
    outcome.unwrap_or(Ok((None, applied_appearance)))
}

fn show_selector_content(
    ui: &mut egui::Ui,
    mode: TemplateSelectorMode,
    error: Option<TemplateSelectorError>,
    focus_requested: bool,
    selected: &mut Option<BuiltinTemplateId>,
    cancel_clicked: &mut bool,
    primary_clicked: &mut bool,
) {
    ui.set_max_width(
        (ui.available_width() - WINDOW_CONTENT_HORIZONTAL_ALLOWANCE).max(MIN_PANE_WIDTH),
    );
    let body_width = ui.available_width();
    let body_height = (ui.available_height() - SELECTOR_FOOTER_ALLOWANCE).max(MIN_PANE_WIDTH);
    ui.allocate_ui_with_layout(
        egui::vec2(body_width, body_height),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let pane_width = ui.available_width();
            let (list_width, pane_gap) = presentation::list_pane_widths(pane_width);
            let preview_width = (pane_width - list_width - pane_gap).max(MIN_PANE_WIDTH);
            ui.allocate_ui_with_layout(
                egui::vec2(list_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    listbox_frame(ui).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("elements-template-catalog")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = presentation::SELECTABLE_ROW_GAP;
                                for metadata in builtin_templates() {
                                    ui.push_id(metadata.id.as_str(), |ui| {
                                        let is_selected = *selected == Some(metadata.id);
                                        let response = selectable_row(
                                            ui,
                                            metadata.display_name,
                                            is_selected,
                                            is_selected && focus_requested,
                                        );
                                        if response.clicked() {
                                            *selected = Some(metadata.id);
                                        }
                                    });
                                }
                            });
                    });
                },
            );
            ui.add_space(pane_gap);
            ui.allocate_ui_with_layout(
                egui::vec2(preview_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    detail_frame(ui).show(ui, |ui| {
                        if let Some(id) = *selected {
                            let metadata = builtin_template_metadata(id);
                            ui.heading(metadata.display_name);
                            if let Some(description) = builtin_template_preview_description(id) {
                                egui::ScrollArea::vertical()
                                    .id_salt("elements-template-preview-description")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        let response = ui.add(
                                            egui::Label::new(description.as_str())
                                                .wrap()
                                                .selectable(true),
                                        );
                                        response.widget_info(|| {
                                            egui::WidgetInfo::labeled(
                                                egui::WidgetType::Label,
                                                response.enabled(),
                                                format!("Preview Description: {description}"),
                                            )
                                        });
                                    });
                            }
                        }
                    });
                },
            );
        },
    );

    if let Some(message) = selector_error_text(error) {
        ui.add_space(presentation::FOOTER_CONTENT_GAP);
        ui.colored_label(ui.visuals().error_fg_color, message);
    }
    ui.separator();
    let footer_width = ui.available_width();
    let footer_rect = ui.allocate_space(egui::vec2(footer_width, FOOTER_BUTTON_HEIGHT)).1;
    let (cancel_rect, primary_rect) = footer_button_rects(footer_rect);
    let primary_enabled = selector_can_apply(mode, *selected) || selector_can_new(mode, *selected);
    let mut cancel_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(cancel_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let cancel_response = cancel_ui.add_sized(cancel_rect.size(), egui::Button::new(CANCEL_LABEL));
    cancel_response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            cancel_response.enabled(),
            CANCEL_LABEL.to_owned(),
        )
    });
    *cancel_clicked = cancel_response.clicked();
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
    *primary_clicked = primary_response.clicked();
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

fn footer_button_rects(footer: egui::Rect) -> (egui::Rect, egui::Rect) {
    let button_gap = FOOTER_BUTTON_GAP
        .min((footer.width() - MIN_PANE_WIDTH * 2.0).max(0.0));
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
