//! Compact, read-only Elements window.

use super::{
    presentation,
    state::ElementsTab,
    summary::ElementDefinitionSummary,
};
use eframe::egui;
use scriptcraft_model::ElementTypeKey;

const FOOTER_RESERVED_HEIGHT: f32 = 52.0;
const FOOTER_HEIGHT: f32 = 34.0;

/// Deferred inspector presentation and command intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElementsWindowIntent {
    SelectElement(ElementTypeKey),
    SelectTab(ElementsTab),
    OpenTemplateSelector,
    CloseElements,
}

/// Shows a bounded root-context inspector and returns at most one deferred intent.
pub(crate) fn show_elements_window(
    ctx: &egui::Context,
    summaries: &[ElementDefinitionSummary],
    selected_key: Option<&ElementTypeKey>,
    selected_summary: Option<&ElementDefinitionSummary>,
    active_tab: ElementsTab,
    selector_is_topmost: bool,
    focus_requested: bool,
) -> Option<ElementsWindowIntent> {
    let viewport = ctx.input(|input| input.viewport_rect());
    let (default_size, max_size) = window_sizes(viewport.size());
    let escape_pressed =
        !selector_is_topmost && ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let mut open = true;
    let mut close_clicked = false;
    let mut action_clicked = false;
    let mut presentation_intent = None;

    egui::Window::new("Elements")
        .id(egui::Id::new("elements-inspector-window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .default_size(default_size)
        .max_size(max_size)
        .show(ctx, |ui| {
            let available = ui.available_size();
            let body_height = (available.y - FOOTER_RESERVED_HEIGHT).max(1.0);
            let body_width = available.x.max(1.0);
            ui.allocate_ui_with_layout(
                egui::vec2(body_width, body_height),
                egui::Layout::left_to_right(egui::Align::Min),
                |body| {
                    let left_width = (body.available_width() * 0.31)
                        .clamp(
                            presentation::LIST_COLUMN_MIN_WIDTH,
                            presentation::LIST_COLUMN_MAX_WIDTH,
                        )
                        .min((body.available_width() - presentation::PANE_GAP - 1.0).max(1.0));
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
                                        rows.set_min_width(pane_width);
                                        for summary in summaries {
                                            let is_selected = selected_key
                                                .is_some_and(|key| key == &summary.key);
                                            let response = rows.push_id(
                                                &summary.key,
                                                |row_ui| {
                                                    presentation::selectable_row(
                                                        row_ui,
                                                        summary.key.as_str(),
                                                        is_selected,
                                                        focus_requested && is_selected,
                                                    )
                                                },
                                            );
                                            if response.inner.clicked() {
                                                presentation_intent = Some(
                                                    ElementsWindowIntent::SelectElement(
                                                        summary.key.clone(),
                                                    ),
                                                );
                                            }
                                        }
                                    });
                            });
                        },
                    );
                    body.add_space(presentation::PANE_GAP);
                    let detail_width = body.available_width().max(1.0);
                    body.allocate_ui_with_layout(
                        egui::vec2(detail_width, body_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |detail_ui| {
                            show_detail_pane(
                                detail_ui,
                                selected_summary,
                                active_tab,
                                &mut presentation_intent,
                            );
                        },
                    );
                },
            );

            ui.add_space(4.0);
            ui.separator();
            let footer_width = ui.available_width().max(1.0);
            ui.allocate_ui_with_layout(
                egui::vec2(footer_width, FOOTER_HEIGHT),
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
                    action_clicked = apply.clicked();
                    footer.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |footer| {
                            let close = footer.button("Close");
                            close.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    close.enabled(),
                                    "Close".to_owned(),
                                )
                            });
                            close_clicked = close.clicked();
                        },
                    );
                },
            );
        });

    // Window close/Escape outrank the same-frame footer or presentation input.
    if !open || escape_pressed || close_clicked {
        Some(ElementsWindowIntent::CloseElements)
    } else if action_clicked {
        Some(ElementsWindowIntent::OpenTemplateSelector)
    } else {
        presentation_intent
    }
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

fn window_sizes(viewport: egui::Vec2) -> (egui::Vec2, egui::Vec2) {
    let max_size = egui::vec2(
        (viewport.x - presentation::VIEWPORT_MARGIN).max(1.0),
        (viewport.y - presentation::VIEWPORT_MARGIN).max(1.0),
    );
    let default_size = egui::vec2(
        presentation::DEFAULT_INSPECTOR_SIZE.x.min(max_size.x),
        presentation::DEFAULT_INSPECTOR_SIZE.y.min(max_size.y),
    );
    (default_size, max_size)
}
