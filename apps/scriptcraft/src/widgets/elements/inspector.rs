//! Compact, read-only Elements window.

use super::{presentation, state::ElementsTab, summary::ElementDefinitionSummary};
use crate::chrome::{status_bar, theme};
use eframe::egui;
use scriptcraft_model::ElementTypeKey;

pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("scriptcraft-elements-inspector")
}

fn viewport_builder() -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title("Elements")
        .with_inner_size(presentation::DEFAULT_INSPECTOR_SIZE)
        .with_min_inner_size(presentation::MIN_INSPECTOR_SIZE)
        .with_resizable(true)
}

pub(crate) fn send_close(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Close);
}

pub(crate) fn send_focus(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
}

/// Deferred inspector presentation and command intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElementsWindowIntent {
    SelectElement(ElementTypeKey),
    SelectTab(ElementsTab),
    OpenTemplateSelector,
    CloseElements,
}

/// Shows the native inspector and returns a deferred intent or unsupported outcome.
pub(crate) fn show_elements_window(
    root_ctx: &egui::Context,
    summaries: &[ElementDefinitionSummary],
    selected_key: Option<&ElementTypeKey>,
    selected_summary: Option<&ElementDefinitionSummary>,
    active_tab: ElementsTab,
    selector_is_topmost: bool,
    focus_requested: bool,
    appearance_mode: theme::AppearanceMode,
    applied_appearance: theme::AppliedAppearance,
) -> Result<(Option<ElementsWindowIntent>, theme::AppliedAppearance), ()> {
    let mut outcome = None;
    root_ctx.show_viewport_immediate(viewport_id(), viewport_builder(), |viewport_ui, class| {
        if class != egui::ViewportClass::Immediate {
            outcome = Some(Err(()));
            return;
        }

        let child_ctx = viewport_ui.ctx().clone();
        if selector_is_topmost
            && child_ctx.input(|input| input.viewport().focused) == Some(true)
        {
            super::focus_topmost_element_viewport_for_state(&child_ctx, true, true);
        }
        if child_ctx.input(|input| input.viewport().close_requested()) {
            if selector_is_topmost {
                child_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                super::focus_topmost_element_viewport_for_state(&child_ctx, true, true);
                return;
            }
            outcome = Some(Ok((Some(ElementsWindowIntent::CloseElements), applied_appearance)));
            return;
        }
        if !selector_is_topmost && child_ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            outcome = Some(Ok((Some(ElementsWindowIntent::CloseElements), applied_appearance)));
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
            let mut intent = None;
            let mut close_clicked = false;
            let mut action_clicked = false;
            ui.add_enabled_ui(!selector_is_topmost, |ui| {
                show_elements_content(
                    ui,
                    summaries,
                    selected_key,
                    selected_summary,
                    active_tab,
                    focus_requested,
                    &mut intent,
                    &mut action_clicked,
                    &mut close_clicked,
                );
            });
            outcome = Some(Ok((if close_clicked {
                Some(ElementsWindowIntent::CloseElements)
            } else if action_clicked {
                Some(ElementsWindowIntent::OpenTemplateSelector)
            } else {
                intent
            }, applied)));
        });
    });
    outcome.unwrap_or(Ok((None, applied_appearance)))
}

fn show_elements_content(
    ui: &mut egui::Ui,
    summaries: &[ElementDefinitionSummary],
    selected_key: Option<&ElementTypeKey>,
    selected_summary: Option<&ElementDefinitionSummary>,
    active_tab: ElementsTab,
    focus_requested: bool,
    presentation_intent: &mut Option<ElementsWindowIntent>,
    action_clicked: &mut bool,
    close_clicked: &mut bool,
) {
    let available = ui.available_size();
    let body_height = (available.y - presentation::INSPECTOR_FOOTER_RESERVED_HEIGHT)
        .max(presentation::MIN_PANE_WIDTH);
    let body_width = available.x.max(presentation::MIN_PANE_WIDTH);
    ui.allocate_ui_with_layout(
        egui::vec2(body_width, body_height),
        egui::Layout::left_to_right(egui::Align::Min),
        |body| {
            let (left_width, pane_gap) = presentation::list_pane_widths(body.available_width());
            body.allocate_ui_with_layout(
                egui::vec2(left_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |list_ui| {
                    presentation::listbox_frame(list_ui).show(list_ui, |pane| {
                        let pane_width = pane.available_width();
                        let pane_height = pane.available_height();
                        egui::ScrollArea::vertical()
                            .id_salt("elements-definition-list")
                            .max_width(pane_width)
                            .max_height(pane_height)
                            .auto_shrink([false, false])
                            .show(pane, |rows| {
                                rows.spacing_mut().item_spacing.y = presentation::SELECTABLE_ROW_GAP;
                                rows.set_min_width(pane_width);
                                for summary in summaries {
                                    let is_selected = selected_key.is_some_and(|key| key == &summary.key);
                                    let response = rows.push_id(&summary.key, |row_ui| {
                                        presentation::selectable_row(
                                            row_ui,
                                            summary.key.as_str(),
                                            is_selected,
                                            focus_requested && is_selected,
                                        )
                                    });
                                    if response.inner.clicked() {
                                        *presentation_intent = Some(
                                            ElementsWindowIntent::SelectElement(summary.key.clone()),
                                        );
                                    }
                                }
                            });
                    });
                },
            );
            body.add_space(pane_gap);
            let detail_width = body.available_width().max(presentation::MIN_PANE_WIDTH);
            body.allocate_ui_with_layout(
                egui::vec2(detail_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |detail_ui| {
                    show_detail_pane(detail_ui, selected_summary, active_tab, presentation_intent);
                },
            );
        },
    );

    ui.add_space(presentation::FOOTER_CONTENT_GAP);
    ui.separator();
    let footer_width = ui.available_width().max(1.0);
    ui.allocate_ui_with_layout(
        egui::vec2(footer_width, presentation::INSPECTOR_FOOTER_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |footer| {
            let apply = footer.button("Apply a Template");
            apply.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    apply.enabled(),
                    "Apply a Template".to_owned(),
                )
            });
            *action_clicked = apply.clicked();
            footer.with_layout(egui::Layout::right_to_left(egui::Align::Center), |footer| {
                let close = footer.button("Close");
                close.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        close.enabled(),
                        "Close".to_owned(),
                    )
                });
                *close_clicked = close.clicked();
            });
        },
    );
}

fn show_detail_pane(
    ui: &mut egui::Ui,
    selected: Option<&ElementDefinitionSummary>,
    active_tab: ElementsTab,
    intent: &mut Option<ElementsWindowIntent>,
) {
    presentation::detail_frame(ui).show(ui, |pane| {
        pane.with_layout(egui::Layout::top_down(egui::Align::Center), |pane| {
            pane.horizontal(|tabs| {
                for (tab, label) in [
                    (ElementsTab::Basic, "Basic"),
                    (ElementsTab::Font, "Font"),
                    (ElementsTab::Paragraph, "Paragraph"),
                ] {
                    let response = presentation::tab_button(tabs, label, active_tab == tab);
                    if response.clicked() {
                        *intent = Some(ElementsWindowIntent::SelectTab(tab));
                    }
                }
            });
        });
        pane.separator();
        let content_height = pane.available_height().max(1.0);
        egui::ScrollArea::vertical()
            .id_salt("elements-selected-details")
            .max_height(content_height)
            .auto_shrink([false, false])
            .show(pane, |content| {
                let Some(selected) = selected else {
                    return;
                };
                match active_tab {
                    ElementsTab::Basic => show_basic(content, selected),
                    ElementsTab::Font => show_font(content, selected),
                    ElementsTab::Paragraph => show_paragraph(content, selected),
                }
            });
    });
}

fn show_basic(ui: &mut egui::Ui, summary: &ElementDefinitionSummary) {
    ui.heading("Behavior");
    presentation::label_value_row(
        ui,
        "Start new page",
        yes_no(summary.paragraph.starts_new_page),
    );
    presentation::label_value_row(
        ui,
        "Paginate as",
        summary.behavior.paginate_as.as_deref().unwrap_or("Absent"),
    );

    ui.add_space(8.0);
    ui.heading("Shortcuts");
    presentation::label_value_row(
        ui,
        "Shortcut",
        summary.behavior.shortcut.as_deref().unwrap_or("None"),
    );
    presentation::label_value_row(
        ui,
        "Next Element Enter/Return",
        &summary.behavior.return_target,
    );
    presentation::label_value_row(
        ui,
        "Next Element Tab",
        &summary.behavior.tab_target,
    );

    if let Some(outline) = &summary.outline {
        ui.add_space(8.0);
        ui.heading("Outline");
        presentation::label_value_row(ui, "Outline", outline);
    }
}

fn show_font(ui: &mut egui::Ui, summary: &ElementDefinitionSummary) {
    ui.heading("Font description");
    presentation::label_value_row(ui, "Font", &summary.font.family);
    presentation::label_value_row(ui, "Point size", &format!("{} pt", summary.font.size));
    presentation::label_value_row(
        ui,
        "Color",
        summary.font.color.as_deref().unwrap_or("Theme default"),
    );
    let style = if summary.font.active_flags.is_empty() {
        "Regular".to_owned()
    } else {
        summary.font.active_flags.join(", ")
    };
    presentation::label_value_row(ui, "Style", &style);
}

fn show_paragraph(ui: &mut egui::Ui, summary: &ElementDefinitionSummary) {
    ui.heading("Paragraph");
    presentation::label_value_row(ui, "Alignment", summary.paragraph.alignment);
    presentation::label_value_row(ui, "Spacing", &summary.paragraph.spacing.to_string());
    presentation::label_value_row(ui, "Leading", &summary.paragraph.leading);
    presentation::label_value_row(
        ui,
        "Space before",
        &format!("{} pt", summary.paragraph.space_before),
    );

    ui.add_space(8.0);
    ui.heading("Indents");
    presentation::label_value_row(
        ui,
        "First",
        &format!("{} in", summary.paragraph.first_indent),
    );
    presentation::label_value_row(
        ui,
        "Left",
        &format!("{} in", summary.paragraph.left_indent),
    );
    presentation::label_value_row(
        ui,
        "Right",
        &format!("{} in", summary.paragraph.right_indent),
    );
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}
