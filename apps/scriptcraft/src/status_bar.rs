use eframe::egui::{self, Color32, Rect, Sense, WidgetInfo, WidgetType};
use std::path::Path;

const COMPACT_STATUS_BREAKPOINT: f32 = 720.0;

fn uses_compact_status_layout(available_width: f32) -> bool {
    !available_width.is_finite() || available_width < COMPACT_STATUS_BREAKPOINT
}

#[allow(dead_code)] // WO-002 defines reusable interaction capabilities without assigning actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusItemKind {
    Text,
    ClickableText,
    ClickableIcon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StatusItem {
    pub(super) id: &'static str,
    pub(super) label: String,
    pub(super) accessible_name: String,
    pub(super) kind: StatusItemKind,
}

impl StatusItem {
    pub(super) fn text(id: &'static str, label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            id,
            accessible_name: label.clone(),
            label,
            kind: StatusItemKind::Text,
        }
    }

    #[allow(dead_code)]
    pub(super) fn clickable_text(
        id: &'static str,
        label: impl Into<String>,
        accessible_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            accessible_name: accessible_name.into(),
            kind: StatusItemKind::ClickableText,
        }
    }

    #[allow(dead_code)]
    pub(super) fn clickable_icon(
        id: &'static str,
        icon: impl Into<String>,
        accessible_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            label: icon.into(),
            accessible_name: accessible_name.into(),
            kind: StatusItemKind::ClickableIcon,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)] // WO-002 exposes interaction capabilities without inventing actions.
pub(super) struct StatusItemResponse {
    pub(super) response: egui::Response,
    pub(super) activated: bool,
}

fn interactive_response(
    ui: &egui::Ui,
    response: egui::Response,
    accessible_name: &str,
) -> StatusItemResponse {
    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Button,
            response.enabled,
            accessible_name.to_owned(),
        )
    });
    let keyboard_activated = response.has_focus()
        && ui.input(|input| {
            input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space)
        });
    let activated = response.clicked() || keyboard_activated;
    StatusItemResponse {
        response,
        activated,
    }
}

/// Render a status-item capability without assigning it an application action.
/// The caller may route the activation result; no action is stored or invented here.
pub(super) fn render_item(ui: &mut egui::Ui, item: &StatusItem) -> Option<StatusItemResponse> {
    match item.kind {
        StatusItemKind::Text => {
            ui.push_id(item.id, |ui| ui.label(&item.label));
            None
        }
        StatusItemKind::ClickableText => {
            let response = ui.push_id(item.id, |ui| ui.button(&item.label)).inner;
            Some(interactive_response(ui, response, &item.accessible_name))
        }
        StatusItemKind::ClickableIcon => {
            let response = ui
                .push_id(item.id, |ui| {
                    ui.add(
                        egui::Button::new(&item.label)
                            .frame(false)
                            .sense(Sense::click()),
                    )
                })
                .inner;
            Some(interactive_response(ui, response, &item.accessible_name))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VisiblePage {
    Available { one_based: usize, total: usize },
    Unavailable,
}

impl VisiblePage {
    pub(super) fn label(self) -> String {
        match self {
            Self::Available { one_based, total } => format!("Page {one_based} of {total}"),
            Self::Unavailable => "Page unavailable".to_owned(),
        }
    }
}

fn valid_rect(rect: Rect) -> bool {
    let min = rect.min;
    let max = rect.max;
    min.x.is_finite()
        && min.y.is_finite()
        && max.x.is_finite()
        && max.y.is_finite()
        && max.x > min.x
        && max.y > min.y
}

/// Select the rendered sheet with the greatest vertical overlap against the
/// actual editor clip. Empty overlap selects the nearest sheet interval.
pub(super) fn select_visible_page(page_rects: &[Rect], clip: Rect) -> VisiblePage {
    if page_rects.is_empty()
        || !valid_rect(clip)
        || page_rects.iter().any(|rect| !valid_rect(*rect))
    {
        return VisiblePage::Unavailable;
    }

    let clip_top = clip.top();
    let clip_bottom = clip.bottom();
    let mut best_index = 0;
    let mut best_overlap = 0.0_f32;

    for (index, page) in page_rects.iter().enumerate() {
        let overlap = (page.bottom().min(clip_bottom) - page.top().max(clip_top)).max(0.0);
        if overlap > best_overlap {
            best_overlap = overlap;
            best_index = index;
        }
    }

    if best_overlap == 0.0 {
        let distance = |page: &Rect| {
            if clip_bottom < page.top() {
                page.top() - clip_bottom
            } else if clip_top > page.bottom() {
                clip_top - page.bottom()
            } else {
                0.0
            }
        };
        let mut nearest = f32::INFINITY;
        for (index, page) in page_rects.iter().enumerate() {
            let candidate = distance(page);
            if candidate < nearest {
                nearest = candidate;
                best_index = index;
            }
        }
    }

    match best_index.checked_add(1) {
        Some(one_based) => VisiblePage::Available {
            one_based,
            total: page_rects.len(),
        },
        None => VisiblePage::Unavailable,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct GeometrySnapshot {
    published_frame: u64,
    layout_revision: u64,
    viewport_rect: Rect,
    scale: f32,
    page_rects: Vec<Rect>,
    clip: Rect,
}

/// Caches only actual page and scroll-clip geometry published by the editor.
/// The footer runs first, so it consumes a snapshot only on the immediately
/// following frame and only while its independent geometry inputs still match.
#[derive(Debug, Default)]
pub(super) struct ViewportPageTracker {
    published: Option<GeometrySnapshot>,
    repaint_after_publish: bool,
}

impl ViewportPageTracker {
    pub(super) fn visible_page(
        &mut self,
        frame: u64,
        layout_revision: u64,
        viewport_rect: Rect,
        scale: f32,
        invalidated: bool,
    ) -> VisiblePage {
        if invalidated {
            self.repaint_after_publish = true;
            return VisiblePage::Unavailable;
        }
        let Some(snapshot) = self.published.as_ref() else {
            self.repaint_after_publish = true;
            return VisiblePage::Unavailable;
        };
        if frame.checked_sub(snapshot.published_frame) != Some(1)
            || snapshot.layout_revision != layout_revision
            || snapshot.viewport_rect != viewport_rect
            || !scale.is_finite()
            || (snapshot.scale - scale).abs() > f32::EPSILON
            || snapshot.page_rects.is_empty()
        {
            self.repaint_after_publish = true;
            return VisiblePage::Unavailable;
        }
        select_visible_page(&snapshot.page_rects, snapshot.clip)
    }

    pub(super) fn publish(
        &mut self,
        frame: u64,
        layout_revision: u64,
        viewport_rect: Rect,
        scale: f32,
        page_rects: Vec<Rect>,
        clip: Rect,
    ) -> bool {
        let next = GeometrySnapshot {
            published_frame: frame,
            layout_revision,
            viewport_rect,
            scale,
            page_rects,
            clip,
        };
        let changed = self.published.as_ref().is_none_or(|old| {
            old.layout_revision != next.layout_revision
                || old.viewport_rect != next.viewport_rect
                || old.scale != next.scale
                || old.page_rects != next.page_rects
                || old.clip != next.clip
        });
        self.published = Some(next);
        let request_repaint = changed || self.repaint_after_publish;
        self.repaint_after_publish = false;
        request_repaint
    }

    pub(super) fn invalidate(&mut self) {
        self.published = None;
        self.repaint_after_publish = true;
    }

    pub(super) fn clip(&self) -> Option<Rect> {
        self.published.as_ref().map(|snapshot| snapshot.clip)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CenterProjection {
    pub(super) clean_label: &'static str,
    pub(super) frames_label: String,
    pub(super) path_label: String,
    pub(super) operation: String,
    pub(super) error: Option<String>,
}

pub(super) fn project_center(
    dirty: bool,
    frames: u64,
    path: Option<&Path>,
    operation: &str,
    error: Option<&str>,
) -> CenterProjection {
    CenterProjection {
        clean_label: if dirty {
            "Unsaved changes"
        } else if path.is_some() {
            "Clean"
        } else {
            "Clean (not saved)"
        },
        frames_label: format!("Frames: {frames}"),
        path_label: format!(
            "Path: {}",
            path.map_or("(not saved)".to_owned(), |p| { p.display().to_string() })
        ),
        operation: operation.to_owned(),
        error: error.map(str::to_owned),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DiagnosticProjection<'a> {
    pub(super) focused: bool,
    pub(super) focus_lock: bool,
    pub(super) dispatch_enabled: bool,
    pub(super) received_events: u32,
    pub(super) dispatched_frames: u32,
    pub(super) pointer_activations: u32,
    pub(super) last_event_category: &'a str,
    pub(super) performance_events: u64,
    pub(super) layout_builds: u64,
    pub(super) layout_cache_hits: u64,
    pub(super) painted_pages: u64,
    pub(super) repaint_requests: u64,
}

impl DiagnosticProjection<'_> {
    pub(super) fn safe_summary(self) -> String {
        format!(
            "focus={} lock={} dispatch={} received-events={} dispatched-frames={} pointer-activations={} last-category={} performance-events={} layouts={} cache-hits={} painted-pages={} repaint-requests={}",
            self.focused,
            self.focus_lock,
            self.dispatch_enabled,
            self.received_events,
            self.dispatched_frames,
            self.pointer_activations,
            if self.last_event_category.is_empty() {
                "none"
            } else {
                self.last_event_category
            },
            self.performance_events,
            self.layout_builds,
            self.layout_cache_hits,
            self.painted_pages,
            self.repaint_requests
        )
    }
}

pub(super) fn show(
    ui: &mut egui::Ui,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: DiagnosticProjection<'_>,
) {
    let compact = uses_compact_status_layout(ui.available_width());
    let page_item = StatusItem::text("visible-page", page.label());
    let clean_item = StatusItem::text("clean-state", center.clean_label);
    let frames_item = StatusItem::text("frame-count", &center.frames_label);
    let path_item = StatusItem::text("document-path", &center.path_label);
    let operation_item = StatusItem::text("operation-status", &center.operation);

    if compact {
        // Keep viewport and document state on the first compact row; keep the
        // potentially longer path/operation/error details on a second wrapped row.
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                render_item(ui, &page_item);
                render_item(ui, &clean_item);
                render_item(ui, &frames_item);
            });
            ui.horizontal_wrapped(|ui| {
                render_item(ui, &path_item);
                render_item(ui, &operation_item);
                if let Some(error) = center.error.as_deref() {
                    ui.colored_label(Color32::RED, error);
                }
            });
        });
    } else {
        ui.columns(3, |regions| {
            regions[0].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                render_item(ui, &page_item);
            });
            regions[1].horizontal_wrapped(|ui| {
                render_item(ui, &clean_item);
                render_item(ui, &frames_item);
                render_item(ui, &path_item);
                render_item(ui, &operation_item);
                if let Some(error) = center.error.as_deref() {
                    ui.colored_label(Color32::RED, error);
                }
            });
            // Reserve a neutral right region for WO-003 without adding zoom controls.
            regions[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(48.0);
            });
        });
    }
    egui::CollapsingHeader::new("Diagnostics")
        .id_salt("status-bar-diagnostics")
        .default_open(false)
        .show(ui, |ui| {
            ui.label(diagnostics.safe_summary());
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom))
    }

    #[test]
    fn status_bar_selector_chooses_greatest_overlap_and_earlier_exact_tie() {
        let pages = [rect(0.0, 0.0, 100.0, 100.0), rect(0.0, 110.0, 100.0, 210.0)];
        assert_eq!(
            select_visible_page(&pages, rect(0.0, 50.0, 100.0, 180.0)),
            VisiblePage::Available {
                one_based: 2,
                total: 2
            }
        );
        assert_eq!(
            select_visible_page(&pages, rect(0.0, 50.0, 100.0, 160.0)),
            VisiblePage::Available {
                one_based: 1,
                total: 2
            }
        );
    }

    #[test]
    fn status_bar_selector_uses_nearest_page_in_gap_and_earlier_distance_tie() {
        let pages = [rect(0.0, 0.0, 100.0, 100.0), rect(0.0, 120.0, 100.0, 220.0)];
        assert_eq!(
            select_visible_page(&pages, rect(0.0, 105.0, 100.0, 115.0)),
            VisiblePage::Available {
                one_based: 1,
                total: 2
            }
        );
        assert_eq!(
            select_visible_page(&pages, rect(0.0, 106.0, 100.0, 116.0)),
            VisiblePage::Available {
                one_based: 2,
                total: 2
            }
        );
    }

    #[test]
    fn status_bar_selector_clamps_bounds_and_counts_title_sheet() {
        let title_and_body = [
            rect(0.0, 0.0, 100.0, 100.0),
            rect(0.0, 112.0, 100.0, 212.0),
            rect(0.0, 224.0, 100.0, 324.0),
        ];
        assert_eq!(
            select_visible_page(&title_and_body, rect(0.0, -40.0, 100.0, -20.0)),
            VisiblePage::Available {
                one_based: 1,
                total: 3
            }
        );
        assert_eq!(
            select_visible_page(&title_and_body, rect(0.0, 350.0, 100.0, 380.0)),
            VisiblePage::Available {
                one_based: 3,
                total: 3
            }
        );
        assert_eq!(
            select_visible_page(&title_and_body, rect(0.0, 130.0, 100.0, 140.0)),
            VisiblePage::Available {
                one_based: 2,
                total: 3
            }
        );
    }

    #[test]
    fn status_bar_selector_returns_unavailable_for_empty_or_invalid_geometry() {
        assert_eq!(
            select_visible_page(&[], rect(0.0, 0.0, 100.0, 100.0)),
            VisiblePage::Unavailable
        );
        assert_eq!(
            select_visible_page(&[rect(0.0, 0.0, 100.0, 100.0)], rect(0.0, 0.0, 100.0, 0.0)),
            VisiblePage::Unavailable
        );
        assert_eq!(
            select_visible_page(
                &[rect(0.0, 0.0, f32::INFINITY, 100.0)],
                rect(0.0, 0.0, 100.0, 100.0)
            ),
            VisiblePage::Unavailable
        );
        assert_eq!(
            select_visible_page(&[Rect::NOTHING], rect(0.0, 0.0, 100.0, 100.0)),
            VisiblePage::Unavailable
        );
    }

    #[test]
    fn status_bar_tracker_requires_fresh_frame_and_matching_geometry_inputs() {
        let viewport = rect(0.0, 0.0, 400.0, 600.0);
        let pages = vec![rect(20.0, 0.0, 380.0, 500.0)];
        let clip = rect(0.0, 30.0, 400.0, 590.0);
        let mut tracker = ViewportPageTracker::default();
        assert!(tracker.publish(1, 8, viewport, 1.0, pages.clone(), clip));
        assert_eq!(
            tracker.visible_page(2, 8, viewport, 1.0, false),
            VisiblePage::Available {
                one_based: 1,
                total: 1
            }
        );
        for (frame, revision, size, scale, invalidated) in [
            (2, 9, viewport, 1.0, false),
            (2, 8, rect(0.0, 0.0, 401.0, 600.0), 1.0, false),
            (2, 8, viewport, 1.25, false),
            (3, 8, viewport, 1.0, false),
            (2, 8, viewport, 1.0, true),
        ] {
            assert_eq!(
                tracker.visible_page(frame, revision, size, scale, invalidated),
                VisiblePage::Unavailable
            );
        }
        assert!(tracker.publish(4, 9, viewport, 1.0, pages, clip));
        assert_eq!(
            tracker.visible_page(5, 9, viewport, 1.0, false),
            VisiblePage::Available {
                one_based: 1,
                total: 1
            }
        );
    }

    #[test]
    fn status_bar_tracker_invalidates_on_error_and_repaints_after_fresh_publish() {
        let viewport = rect(0.0, 0.0, 400.0, 600.0);
        let pages = vec![rect(20.0, 0.0, 380.0, 500.0)];
        let clip = rect(0.0, 30.0, 400.0, 590.0);
        let mut tracker = ViewportPageTracker::default();
        assert!(tracker.publish(1, 8, viewport, 1.0, pages.clone(), clip));
        tracker.invalidate();
        assert_eq!(tracker.clip(), None);
        assert_eq!(
            tracker.visible_page(2, 8, viewport, 1.0, false),
            VisiblePage::Unavailable
        );
        assert!(tracker.publish(2, 8, viewport, 1.0, pages, clip));
    }

    #[test]
    fn status_bar_item_model_separates_capabilities_without_actions() {
        let text = StatusItem::text("label", "Page 1 of 2");
        let clickable_text = StatusItem::clickable_text("link", "Details", "Show diagnostics");
        let clickable_icon = StatusItem::clickable_icon("icon", "⋯", "More status options");
        assert_eq!(text.kind, StatusItemKind::Text);
        assert_eq!(text.accessible_name, "Page 1 of 2");
        assert_eq!(clickable_text.kind, StatusItemKind::ClickableText);
        assert_eq!(clickable_text.accessible_name, "Show diagnostics");
        assert_eq!(clickable_icon.kind, StatusItemKind::ClickableIcon);
        assert_eq!(clickable_icon.accessible_name, "More status options");
        assert!(!text.label.contains("action"));
    }

    #[test]
    fn status_bar_uses_deterministic_compact_layout_below_breakpoint() {
        assert!(uses_compact_status_layout(320.0));
        assert!(uses_compact_status_layout(COMPACT_STATUS_BREAKPOINT - 0.1));
        assert!(!uses_compact_status_layout(COMPACT_STATUS_BREAKPOINT));
        assert!(!uses_compact_status_layout(1_200.0));
        assert!(uses_compact_status_layout(f32::NAN));
    }

    #[test]
    fn status_bar_center_projection_reports_clean_unsaved_path_and_errors() {
        let clean_unsaved = project_center(false, 17, None, "In-memory project", None);
        assert_eq!(clean_unsaved.clean_label, "Clean (not saved)");
        assert_eq!(clean_unsaved.frames_label, "Frames: 17");
        assert_eq!(clean_unsaved.path_label, "Path: (not saved)");

        let dirty = project_center(
            true,
            18,
            Some(Path::new("screenplay.json")),
            "Save failed",
            Some("unchanged destination"),
        );
        assert_eq!(dirty.clean_label, "Unsaved changes");
        assert_eq!(dirty.path_label, "Path: screenplay.json");
        assert_eq!(dirty.operation, "Save failed");
        assert_eq!(dirty.error.as_deref(), Some("unchanged destination"));
        assert_eq!(
            project_center(false, 19, Some(Path::new("saved.json")), "Saved", None).clean_label,
            "Clean"
        );
    }

    #[test]
    fn status_bar_diagnostic_projection_contains_only_categories_and_counters() {
        let diagnostics = DiagnosticProjection {
            focused: true,
            focus_lock: true,
            dispatch_enabled: true,
            received_events: 3,
            dispatched_frames: 2,
            pointer_activations: 1,
            last_event_category: "key",
            performance_events: 8,
            layout_builds: 2,
            layout_cache_hits: 6,
            painted_pages: 4,
            repaint_requests: 1,
        };
        let summary = diagnostics.safe_summary();
        assert!(summary.contains("last-category=key"));
        assert!(summary.contains("cache-hits=6"));
        assert!(!summary.contains("document"));
        assert!(!summary.contains("source text"));
    }

    #[test]
    fn status_bar_item_rendering_keeps_text_noninteractive_and_buttons_named() {
        let context = egui::Context::default();
        let mut text_interactive = None;
        let mut button_interactive = None;
        let mut icon_interactive = None;
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                let text = StatusItem::text("plain", "status");
                text_interactive = render_item(ui, &text).map(|_| true);
                let button = StatusItem::clickable_text("details", "Details", "Show diagnostics");
                button_interactive = render_item(ui, &button)
                    .map(|rendered| rendered.response.sense == Sense::click());
                let icon = StatusItem::clickable_icon("more", "⋯", "More status options");
                icon_interactive = render_item(ui, &icon)
                    .map(|response| response.response.sense == Sense::click());
            });
        });
        assert_eq!(text_interactive, None);
        assert_eq!(button_interactive, Some(true));
        assert_eq!(icon_interactive, Some(true));
    }

    #[test]
    fn status_bar_clickable_item_reports_focused_enter_and_space_activation() {
        let context = egui::Context::default();
        let item = StatusItem::clickable_text("keyboard", "Details", "Show diagnostics");
        let mut button_id = None;
        let mut activated = false;
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                let rendered = render_item(ui, &item);
                if let Some(rendered) = rendered {
                    rendered.response.request_focus();
                    button_id = Some(rendered.response.id);
                }
            });
        });
        assert!(button_id.is_some());
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        let _ = context.run(input, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                activated = render_item(ui, &item).is_some_and(|rendered| rendered.activated);
            });
        });
        assert!(activated);
    }
}
