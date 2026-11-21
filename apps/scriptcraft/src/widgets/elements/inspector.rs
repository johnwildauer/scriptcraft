//! Compact, read-only Elements window.

use super::summary::{ElementDefinitionSummary, ParagraphSummary};
use eframe::egui;

const DEFAULT_WINDOW_SIZE: egui::Vec2 = egui::vec2(720.0, 620.0);
const VIEWPORT_MARGIN: f32 = 32.0;
const WINDOW_FOOTER_ALLOWANCE: f32 = 116.0;

/// The inspector exposes only close and selector-open presentation intents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementsWindowIntent {
    OpenTemplateSelector,
    CloseElements,
}

/// Shows a bounded root-context inspector and returns at most one deferred intent.
pub(crate) fn show_elements_window(
    ctx: &egui::Context,
    summaries: &[ElementDefinitionSummary],
    selector_is_topmost: bool,
    focus_requested: bool,
) -> Option<ElementsWindowIntent> {
    let viewport = ctx.input(|input| input.viewport_rect());
    let (default_size, max_size) = window_sizes(viewport.size());
    let escape_pressed =
        !selector_is_topmost && ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let mut open = true;
    let mut action_clicked = false;

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
            ui.set_max_width((max_size.x - 24.0).max(1.0));
            let scroll_height = (max_size.y - WINDOW_FOOTER_ALLOWANCE).max(1.0);
            egui::ScrollArea::vertical()
                .id_salt("elements-definition-list")
                .max_height(scroll_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (index, summary) in summaries.iter().enumerate() {
                        if index > 0 {
                            ui.separator();
                        }
                        show_definition_summary(ui, summary);
                    }
                });

            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let response = ui.button("Apply a Template");
                if focus_requested {
                    response.request_focus();
                }
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        response.enabled(),
                        "Apply a Template".to_owned(),
                    )
                });
                action_clicked = response.clicked();
            });
        });

    // A title close or Escape closes only Elements; it takes precedence over a
    // same-frame button response and never mutates dialog state reentrantly.
    if !open || escape_pressed {
        Some(ElementsWindowIntent::CloseElements)
    } else if action_clicked {
        Some(ElementsWindowIntent::OpenTemplateSelector)
    } else {
        None
    }
}

fn show_definition_summary(ui: &mut egui::Ui, summary: &ElementDefinitionSummary) {
    ui.heading(&summary.key);
    ui.label(format!(
        "Font: {} {} pt{}; color {}",
        summary.font.family,
        summary.font.size,
        if summary.font.active_flags.is_empty() {
            String::new()
        } else {
            format!(", {}", summary.font.active_flags.join(", "))
        },
        summary.font.color.as_deref().unwrap_or("Theme default")
    ));
    ui.label(paragraph_summary_label(&summary.paragraph));
    ui.label(format!(
        "Behavior: PaginateAs {}; Return {}; Tab {}; platform shortcut {}",
        summary.behavior.paginate_as.as_deref().unwrap_or("Absent"),
        summary.behavior.return_target,
        summary.behavior.tab_target,
        summary.behavior.shortcut.as_deref().unwrap_or("None"),
    ));
    if let Some(outline) = &summary.outline {
        ui.label(format!("Outline: {outline}"));
    }
}

fn paragraph_summary_label(paragraph: &ParagraphSummary) -> String {
    format!(
        "Paragraph: {}; first/left/right indent {} / {} / {} inches; leading {}; spacing {}; space before {} pt; starts new page {}",
        paragraph.alignment,
        paragraph.first_indent,
        paragraph.left_indent,
        paragraph.right_indent,
        paragraph.leading,
        paragraph.spacing,
        paragraph.space_before,
        yes_no(paragraph.starts_new_page),
    )
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
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
