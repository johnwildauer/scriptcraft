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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::elements::summary::ParagraphSummary;

    #[test]
    fn paragraph_label_distinguishes_inches_from_points() {
        let paragraph = ParagraphSummary {
            alignment: "Left",
            first_indent: "-0.125".to_owned(),
            left_indent: "1".to_owned(),
            right_indent: "2.5".to_owned(),
            leading: "Regular".to_owned(),
            spacing: 1,
            space_before: "4.5".to_owned(),
            starts_new_page: false,
        };

        assert_eq!(
            paragraph_summary_label(&paragraph),
            "Paragraph: Left; first/left/right indent -0.125 / 1 / 2.5 inches; leading Regular; spacing 1; space before 4.5 pt; starts new page No"
        );
    }

    #[test]
    fn inspector_intent_surface_is_read_only_and_has_one_template_entry_point() {
        let allowed = [
            ElementsWindowIntent::OpenTemplateSelector,
            ElementsWindowIntent::CloseElements,
        ];
        assert_eq!(allowed.len(), 2);
        assert_eq!(yes_no(true), "Yes");
        assert_eq!(yes_no(false), "No");
    }

    #[test]
    fn window_default_and_outer_bounds_fit_the_available_viewport() {
        let viewport = egui::vec2(900.0, 700.0);
        let (default_size, max_size) = window_sizes(viewport);
        assert_eq!(max_size, egui::vec2(868.0, 668.0));
        assert_eq!(default_size, egui::vec2(720.0, 620.0));
        assert!(default_size.x <= max_size.x);
        assert!(default_size.y <= max_size.y);

        let short_viewport = egui::vec2(360.0, 260.0);
        let (small_default, small_max) = window_sizes(short_viewport);
        assert!(small_default.x <= small_max.x);
        assert!(small_default.y <= small_max.y);
        assert!(small_max.x < viewport.x);
        assert!(small_max.y < viewport.y);

        let tiny_viewport = egui::vec2(200.0, 120.0);
        let (tiny_default, tiny_max) = window_sizes(tiny_viewport);
        assert_eq!(tiny_max, egui::vec2(168.0, 88.0));
        assert!(tiny_default.x <= tiny_max.x);
        assert!(tiny_default.y <= tiny_max.y);
    }
}
