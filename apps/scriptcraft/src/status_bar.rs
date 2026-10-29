use eframe::egui::{
    self, Align, Color32, Direction, FontSelection, Rect, Sense, WidgetInfo, WidgetText, WidgetType,
};
const COMPACT_STATUS_BREAKPOINT: f32 = 720.0;
pub(super) const MIN_ZOOM_PERCENT: u16 = 75;
pub(super) const MAX_ZOOM_PERCENT: u16 = 300;
pub(super) const ZOOM_PRESETS: [u16; 9] = [75, 90, 100, 125, 150, 175, 200, 250, 300];
const FALLBACK_UI_PIXELS_PER_POINT: f32 = 2.0;

pub(super) fn resolved_ui_pixels_per_point(native: Option<f32>) -> f32 {
    native
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(FALLBACK_UI_PIXELS_PER_POINT)
}

pub(super) fn ui_zoom_factor(native: Option<f32>) -> f32 {
    let valid_native = native.filter(|value| value.is_finite() && *value > 0.0);
    resolved_ui_pixels_per_point(valid_native) / valid_native.unwrap_or(1.0)
}

pub(super) fn zoom_after_native_factor(current_percent: f64, factor: f32) -> Option<f64> {
    if !current_percent.is_finite() || !factor.is_finite() || factor <= 0.0 {
        return None;
    }
    Some(
        (current_percent * f64::from(factor))
            .clamp(MIN_ZOOM_PERCENT as f64, MAX_ZOOM_PERCENT as f64),
    )
}

pub(super) fn normalized_zoom_percent(value: f64) -> u16 {
    if !value.is_finite() {
        return 100;
    }
    value
        .round()
        .clamp(MIN_ZOOM_PERCENT as f64, MAX_ZOOM_PERCENT as f64) as u16
}

fn zoom_controls(
    ui: &mut egui::Ui,
    zoom_percent: &mut u16,
    region_width: Option<f32>,
) -> (bool, Rect, Rect, egui::Id) {
    let old_value = *zoom_percent;
    let mut slider_rect = Rect::NOTHING;
    let mut presets_rect = Rect::NOTHING;
    let mut presets_id = egui::Id::NULL;
    ui.horizontal(|ui| {
        let available_width = region_width.unwrap_or_else(|| ui.available_width());
        let slider_label = if region_width.is_some() {
            "Zoom"
        } else {
            "Page zoom"
        };
        let label_reserve = if region_width.is_some() { 48.0 } else { 0.0 };
        let slider_width = (available_width - 64.0 - ui.spacing().item_spacing.x - label_reserve)
            .clamp(40.0, 165.0);
        slider_rect = zoom_slider(ui, slider_width, zoom_percent, slider_label).rect;
        let presets_response = egui::ComboBox::from_id_salt("status-page-zoom-presets")
            .selected_text(format!("{zoom_percent}%"))
            .width(64.0)
            .show_ui(ui, |ui| {
                for preset in ZOOM_PRESETS {
                    ui.selectable_value(zoom_percent, preset, format!("{preset}%"));
                }
            })
            .response;
        presets_rect = presets_response.rect;
        presets_id = presets_response.id;
    });
    (
        *zoom_percent != old_value,
        slider_rect,
        presets_rect,
        presets_id,
    )
}

fn zoom_slider(
    ui: &mut egui::Ui,
    width: f32,
    zoom_percent: &mut u16,
    label: &str,
) -> egui::Response {
    let response = ui.add_sized(
        [width, ui.spacing().interact_size.y],
        egui::Slider::new(zoom_percent, MIN_ZOOM_PERCENT..=MAX_ZOOM_PERCENT)
            .integer()
            .trailing_fill(true)
            .show_value(false)
            .text(label),
    );
    if response.has_focus() {
        if ui.input(|input| input.key_pressed(egui::Key::Home)) {
            *zoom_percent = MIN_ZOOM_PERCENT;
        } else if ui.input(|input| input.key_pressed(egui::Key::End)) {
            *zoom_percent = MAX_ZOOM_PERCENT;
        }
    }
    response
}

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
            ui.push_id(item.id, |ui| {
                paint_noninteractive_text(ui, item.id, &item.label, ui.visuals().text_color());
            });
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

/// Lay out and paint a label without creating an egui `Response` or pointer hit
/// target. Keep the same wrapping and text style as `Ui::label`, and publish an
/// equivalent AccessKit label node when the backend has accessibility enabled.
fn paint_noninteractive_text(
    ui: &mut egui::Ui,
    id_source: &str,
    text: &str,
    color: Color32,
) -> Rect {
    let mut layout_job = WidgetText::from(text).into_layout_job(
        ui.style(),
        FontSelection::Default,
        ui.text_valign(),
    );
    let available_width = ui.available_width();
    let (galley_pos, galley, bounds) = if ui.wrap_mode() == egui::TextWrapMode::Wrap
        && ui.layout().main_dir() == Direction::LeftToRight
        && ui.layout().main_wrap()
        && available_width.is_finite()
    {
        let cursor = ui.cursor();
        let first_row_indentation = available_width - ui.available_size_before_wrap().x;
        layout_job.wrap.max_width = available_width;
        layout_job.first_row_min_height = cursor.height();
        layout_job.halign = Align::Min;
        layout_job.justify = false;
        if let Some(first_section) = layout_job.sections.first_mut() {
            first_section.leading_space = first_row_indentation;
        }
        let galley = ui.fonts(|fonts| fonts.layout_job(layout_job));
        let pos = egui::pos2(ui.max_rect().left(), ui.cursor().top());
        let mut bounds = Rect::NOTHING;
        for row in &galley.rows {
            let row_rect = row.rect.translate(pos.to_vec2());
            ui.advance_cursor_after_rect(row_rect);
            bounds = bounds.union(row_rect);
        }
        (pos, galley, bounds)
    } else {
        match ui.wrap_mode() {
            egui::TextWrapMode::Extend => {
                layout_job.wrap.max_width = f32::INFINITY;
            }
            egui::TextWrapMode::Wrap => layout_job.wrap.max_width = available_width,
            egui::TextWrapMode::Truncate => {
                layout_job.wrap.max_width = available_width;
                layout_job.wrap.max_rows = 1;
                layout_job.wrap.break_anywhere = true;
            }
        }
        layout_job.halign = ui.layout().horizontal_placement();
        layout_job.justify = ui.layout().horizontal_justify();
        let galley = ui.fonts(|fonts| fonts.layout_job(layout_job));
        let (_, rect) = ui.allocate_space(galley.size());
        let pos = match galley.job.halign {
            Align::LEFT => rect.left_top(),
            Align::Center => rect.center_top(),
            Align::RIGHT => rect.right_top(),
        };
        (pos, galley, rect)
    };

    if ui.is_rect_visible(bounds) {
        ui.painter().galley(galley_pos, galley, color);
    }
    let accessibility_id = ui.make_persistent_id(("status-label", id_source));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_name(text.to_owned());
        node.set_bounds(egui::accesskit::Rect {
            x0: bounds.min.x.into(),
            y0: bounds.min.y.into(),
            x1: bounds.max.x.into(),
            y1: bounds.max.y.into(),
        });
    });
    bounds
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

    fn compact_label(self) -> String {
        match self {
            Self::Available { one_based, total } => format!("{one_based}/{total}"),
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
    pub(super) operation: String,
    pub(super) error: Option<String>,
}

pub(super) fn project_center(
    dirty: bool,
    operation: &str,
    error: Option<&str>,
) -> CenterProjection {
    CenterProjection {
        clean_label: if dirty { "Unsaved changes" } else { "Clean" },
        operation: operation.to_owned(),
        error: error.map(str::to_owned),
    }
}

fn one_line_region<R>(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::left_to_right(egui::Align::Center),
        |region| {
            region.set_clip_rect(region.clip_rect().intersect(region.max_rect()));
            region.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            contents(region)
        },
    )
}

fn paint_clipped_single_line(
    ui: &egui::Ui,
    id_source: &str,
    text: &str,
    accessible_name: &str,
    color: Color32,
    rect: Rect,
) {
    let mut layout_job =
        WidgetText::from(text).into_layout_job(ui.style(), FontSelection::Default, Align::Center);
    layout_job.wrap.max_width = rect.width();
    layout_job.wrap.max_rows = 1;
    layout_job.wrap.break_anywhere = true;
    layout_job.halign = Align::Min;
    let galley = ui.fonts(|fonts| fonts.layout_job(layout_job));
    let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5);
    if ui.is_rect_visible(rect) {
        ui.painter().with_clip_rect(rect).galley(pos, galley, color);
    }
    let accessibility_id = ui.make_persistent_id(("status-label", id_source));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_name(accessible_name.to_owned());
        node.set_bounds(egui::accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
    });
}

fn status_message(center: &CenterProjection) -> Option<String> {
    match (center.operation.is_empty(), center.error.as_deref()) {
        (true, None) => None,
        (true, Some(error)) => Some(error.to_owned()),
        (false, None) => Some(center.operation.clone()),
        (false, Some(error)) => Some(format!("{}: {error}", center.operation)),
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
    zoom_percent: &mut u16,
) -> bool {
    show_impl(ui, page, center, diagnostics, zoom_percent, None)
}

fn show_impl(
    ui: &mut egui::Ui,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: DiagnosticProjection<'_>,
    zoom_percent: &mut u16,
    diagnostics_open: Option<bool>,
) -> bool {
    let compact = uses_compact_status_layout(ui.available_width());
    let mut zoom_changed = false;
    let message = status_message(center);

    if compact {
        let available_width = ui.available_width();
        let row_height = ui.spacing().interact_size.y;
        let gaps = ui.spacing().item_spacing.x * 2.0;
        let zoom_width = (available_width * 0.4)
            .clamp(160.0, 200.0)
            .min(available_width);
        let content_width = (available_width - zoom_width - gaps).max(0.0);
        let page_width = content_width * 0.4;
        let state_width = (content_width - page_width).max(0.0);
        let clean_label = if center.clean_label == "Unsaved changes" {
            "Dirty"
        } else {
            center.clean_label
        };
        one_line_region(ui, egui::vec2(available_width, row_height), |row| {
            row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                one_line_region(ui, egui::vec2(zoom_width, row_height), |zoom_ui| {
                    zoom_changed = zoom_controls(zoom_ui, zoom_percent, Some(zoom_width)).0;
                });
                let visible_page = if zoom_changed {
                    VisiblePage::Unavailable
                } else {
                    page
                };
                let page_item = StatusItem {
                    accessible_name: visible_page.label(),
                    ..StatusItem::text("visible-page", visible_page.compact_label())
                };
                one_line_region(ui, egui::vec2(page_width, row_height), |region| {
                    render_item(region, &page_item);
                });
                one_line_region(ui, egui::vec2(state_width, row_height), |region| {
                    let state_rect = region.max_rect().intersect(region.clip_rect());
                    let clean_width = (state_rect.width() * 0.55).max(0.0);
                    let clean_rect = Rect::from_min_max(
                        state_rect.min,
                        egui::pos2(state_rect.left() + clean_width, state_rect.bottom()),
                    );
                    let message_rect = Rect::from_min_max(
                        egui::pos2(clean_rect.right(), state_rect.top()),
                        state_rect.max,
                    );
                    paint_clipped_single_line(
                        region,
                        "clean-state",
                        clean_label,
                        center.clean_label,
                        region.visuals().text_color(),
                        clean_rect,
                    );
                    if let Some(message) = message.as_deref() {
                        paint_clipped_single_line(
                            region,
                            "operation-status",
                            message,
                            message,
                            if center.error.is_some() {
                                Color32::RED
                            } else {
                                region.visuals().text_color()
                            },
                            message_rect,
                        );
                    }
                });
            });
        });
    } else {
        ui.columns(3, |regions| {
            // Render controls before the page label so a zoom change reports
            // unavailable until the newly scaled page geometry is published.
            zoom_changed = zoom_controls(&mut regions[2], zoom_percent, None).0;
            let status_width = regions[1].available_width();
            let page_row_height = regions[1].min_size().y.max(regions[2].min_size().y);
            one_line_region(
                &mut regions[1],
                egui::vec2(status_width, page_row_height),
                |ui| {
                    let status_rect = ui.max_rect().intersect(ui.clip_rect());
                    let clean_width = (status_rect.width() * 0.48).max(0.0);
                    let clean_rect = Rect::from_min_max(
                        status_rect.min,
                        egui::pos2(status_rect.left() + clean_width, status_rect.bottom()),
                    );
                    let message_rect = Rect::from_min_max(
                        egui::pos2(clean_rect.right(), status_rect.top()),
                        status_rect.max,
                    );
                    paint_clipped_single_line(
                        ui,
                        "clean-state",
                        center.clean_label,
                        center.clean_label,
                        ui.visuals().text_color(),
                        clean_rect,
                    );
                    if let Some(message) = message.as_deref() {
                        paint_clipped_single_line(
                            ui,
                            "operation-status",
                            message,
                            message,
                            if center.error.is_some() {
                                Color32::RED
                            } else {
                                ui.visuals().text_color()
                            },
                            message_rect,
                        );
                    }
                },
            );

            // Center the page label within the content row, not the column's
            // full available panel height. The latter grows with TopBottomPanel
            // feedback and becomes a growing minimum when collected by columns.
            let page_column_width = regions[0].available_width();
            one_line_region(
                &mut regions[0],
                egui::vec2(page_column_width, page_row_height),
                |ui| {
                    let page_item = StatusItem::text(
                        "visible-page",
                        if zoom_changed {
                            VisiblePage::Unavailable.label()
                        } else {
                            page.label()
                        },
                    );
                    render_item(ui, &page_item);
                },
            );
        });
    }
    egui::CollapsingHeader::new("Diagnostics")
        .id_salt("status-bar-diagnostics")
        .default_open(false)
        .open(diagnostics_open)
        .show(ui, |ui| {
            let summary = diagnostics.safe_summary();
            let summary_height = ui.text_style_height(&egui::TextStyle::Body);
            let summary_width = ui.available_width();
            one_line_region(
                ui,
                egui::vec2(summary_width, summary_height),
                |summary_ui| {
                    paint_noninteractive_text(
                        summary_ui,
                        "status-diagnostics-summary",
                        &summary,
                        summary_ui.visuals().text_color(),
                    );
                },
            );
        });
    zoom_changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_zoom_controls(
        context: &egui::Context,
        input: egui::RawInput,
        zoom_percent: &mut u16,
    ) -> (egui::FullOutput, bool, Rect, Rect, egui::Id) {
        let mut changed = false;
        let mut slider_rect = Rect::NOTHING;
        let mut presets_rect = Rect::NOTHING;
        let mut presets_id = egui::Id::NULL;
        let output = context.run(input, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                (changed, slider_rect, presets_rect, presets_id) =
                    zoom_controls(ui, zoom_percent, None);
            });
        });
        (output, changed, slider_rect, presets_rect, presets_id)
    }

    fn pointer_button_frame(position: egui::Pos2, pressed: bool) -> egui::RawInput {
        egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        }
    }

    fn text_center(output: &egui::FullOutput, expected: &str) -> Option<egui::Pos2> {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == expected => {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
    }

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom))
    }

    #[allow(clippy::too_many_arguments)] // Keeps the headless renderer fixture explicit at call sites.
    fn render_status_frame(
        context: &egui::Context,
        input: egui::RawInput,
        width: f32,
        page: VisiblePage,
        center: &CenterProjection,
        diagnostics: DiagnosticProjection<'_>,
        zoom_percent: &mut u16,
        diagnostics_open: Option<bool>,
    ) -> (egui::FullOutput, Rect, Rect, Rect) {
        let screen_rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 820.0));
        let next_time = context.input(|input| input.time + 0.1);
        let mut toolbar_rect = Rect::NOTHING;
        let mut status_rect = Rect::NOTHING;
        let mut editor_rect = Rect::NOTHING;
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                time: Some(next_time),
                ..input
            },
            |context| {
                toolbar_rect = egui::TopBottomPanel::top("status-dynamic-test-toolbar")
                    .exact_height(32.0)
                    .show(context, |ui| ui.label("Toolbar"))
                    .response
                    .rect;
                status_rect = egui::TopBottomPanel::bottom("status-dynamic-test-panel")
                    .show(context, |ui| {
                        show_impl(
                            ui,
                            page,
                            center,
                            diagnostics,
                            zoom_percent,
                            diagnostics_open,
                        );
                    })
                    .response
                    .rect;
                editor_rect = egui::CentralPanel::default()
                    .show(context, |ui| ui.label("Editor content"))
                    .response
                    .rect;
            },
        );
        (output, toolbar_rect, status_rect, editor_rect)
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
        let clean_unsaved = project_center(false, "", None);
        assert_eq!(clean_unsaved.clean_label, "Clean");
        assert_eq!(status_message(&clean_unsaved), None);

        let dirty = project_center(true, "Save failed", Some("unchanged destination"));
        assert_eq!(dirty.clean_label, "Unsaved changes");
        assert_eq!(dirty.operation, "Save failed");
        assert_eq!(dirty.error.as_deref(), Some("unchanged destination"));
        assert_eq!(
            status_message(&dirty).as_deref(),
            Some("Save failed: unchanged destination")
        );
        assert_eq!(project_center(false, "Saved", None).clean_label, "Clean");
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
        context.enable_accesskit();
        let mut button_interactive = None;
        let mut icon_interactive = None;
        let item = StatusItem::text("plain", "status text remains visible");
        let mut text_rect = Rect::NOTHING;
        let mut text_widget_id = None;
        let initial = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                ui.push_id(item.id, |ui| {
                    // `Label` would allocate its response with this next automatic
                    // ID; use it to verify the actual egui interaction snapshot.
                    text_widget_id = Some(ui.next_auto_id());
                    text_rect = paint_noninteractive_text(
                        ui,
                        item.id,
                        &item.label,
                        ui.visuals().text_color(),
                    );
                });
                let button = StatusItem::clickable_text("details", "Details", "Show diagnostics");
                button_interactive = render_item(ui, &button)
                    .map(|rendered| rendered.response.sense == Sense::click());
                let icon = StatusItem::clickable_icon("more", "⋯", "More status options");
                icon_interactive = render_item(ui, &icon)
                    .map(|response| response.response.sense == Sense::click());
            });
        });
        assert!(initial.shapes.iter().any(|shape| {
            matches!(
                &shape.shape,
                egui::epaint::Shape::Text(text)
                    if text.galley.text() == item.label
            )
        }));
        let accessible_nodes = initial
            .platform_output
            .accesskit_update
            .as_ref()
            .map(|update| &update.nodes);
        assert!(
            accessible_nodes.is_some_and(|nodes| {
                nodes.iter().any(|(_, node)| {
                    node.role() == egui::accesskit::Role::Label
                        && node.name() == Some(item.label.as_str())
                }) && nodes.iter().any(|(_, node)| {
                    node.role() == egui::accesskit::Role::Button
                        && node.name() == Some("Show diagnostics")
                }) && nodes.iter().any(|(_, node)| {
                    node.role() == egui::accesskit::Role::Button
                        && node.name() == Some("More status options")
                })
            }),
            "painter-only text and interactive controls retain their accessibility labels"
        );
        assert_eq!(button_interactive, Some(true));
        assert_eq!(icon_interactive, Some(true));

        let text_id = text_widget_id.unwrap_or_else(|| unreachable!());
        let pointer_input = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(text_rect.center()),
                egui::Event::PointerButton {
                    pos: text_rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        let mut adapter_returned_response = false;
        let _ = context.run(pointer_input, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                adapter_returned_response = render_item(ui, &item).is_some();
            });
        });
        let pointer_targets_text = context.interaction_snapshot(|snapshot| {
            snapshot.contains_pointer.contains(&text_id)
                || snapshot.hovered.contains(&text_id)
                || snapshot.clicked == Some(text_id)
                || snapshot.dragged == Some(text_id)
        });
        assert!(!adapter_returned_response);
        assert!(
            !pointer_targets_text,
            "painted noninteractive text must not register a pointer-sensing egui response"
        );
        assert_ne!(context.memory(|memory| memory.focused()), Some(text_id));
        assert_eq!(
            context.output(|output| output.cursor_icon),
            egui::CursorIcon::Default,
            "noninteractive text must not claim cursor ownership"
        );
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

    #[test]
    fn zoom_value_normalization_rounds_and_clamps_all_integer_values() {
        assert_eq!(normalized_zoom_percent(f64::NAN), 100);
        assert_eq!(normalized_zoom_percent(f64::INFINITY), 100);
        assert_eq!(normalized_zoom_percent(74.4), MIN_ZOOM_PERCENT);
        assert_eq!(normalized_zoom_percent(74.5), MIN_ZOOM_PERCENT);
        assert_eq!(normalized_zoom_percent(301.0), MAX_ZOOM_PERCENT);
        assert_eq!(normalized_zoom_percent(124.49), 124);
        assert_eq!(normalized_zoom_percent(124.5), 125);
        for percent in MIN_ZOOM_PERCENT..=MAX_ZOOM_PERCENT {
            assert_eq!(normalized_zoom_percent(percent as f64), percent);
        }
    }

    #[test]
    fn native_zoom_mapping_preserves_fractional_steps_and_clamps_inward_outward() {
        let mut value = 100.0;
        for _ in 0..3 {
            value = zoom_after_native_factor(value, 1.02).unwrap_or(value);
        }
        assert_eq!(normalized_zoom_percent(value), 106);
        assert!((zoom_after_native_factor(100.0, 0.8).unwrap_or_default() - 80.0).abs() < 0.00001);
        assert_eq!(
            zoom_after_native_factor(299.0, 2.0),
            Some(MAX_ZOOM_PERCENT as f64)
        );
        assert_eq!(
            zoom_after_native_factor(76.0, 0.5),
            Some(MIN_ZOOM_PERCENT as f64)
        );
        assert_eq!(zoom_after_native_factor(100.0, 0.0), None);
        assert_eq!(zoom_after_native_factor(100.0, -1.0), None);
        assert_eq!(zoom_after_native_factor(100.0, f32::NAN), None);
        assert_eq!(zoom_after_native_factor(100.0, f32::INFINITY), None);
    }

    #[test]
    fn native_ui_scale_prefers_valid_os_value_once_and_falls_back_to_two() {
        for native in [Some(1.0), Some(1.5), Some(2.0)] {
            assert_eq!(resolved_ui_pixels_per_point(native), native.unwrap_or(0.0));
            assert_eq!(ui_zoom_factor(native), 1.0);
            assert_eq!(
                ui_zoom_factor(native) * native.unwrap_or(1.0),
                resolved_ui_pixels_per_point(native)
            );
        }
        for invalid in [
            None,
            Some(0.0),
            Some(-1.0),
            Some(f32::NAN),
            Some(f32::INFINITY),
        ] {
            assert_eq!(resolved_ui_pixels_per_point(invalid), 2.0);
            assert_eq!(ui_zoom_factor(invalid), 2.0);
        }
        assert_eq!(ui_zoom_factor(None) * 1.5, 3.0);
        assert_eq!(ui_zoom_factor(Some(1.5)) * 1.5, 1.5);
    }

    #[test]
    fn zoom_preset_matrix_is_exact_and_ordered() {
        assert_eq!(ZOOM_PRESETS, [75, 90, 100, 125, 150, 175, 200, 250, 300]);
        assert_eq!(ZOOM_PRESETS.len(), 9);
        assert!(ZOOM_PRESETS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn zoom_slider_emits_blue_proportional_fill_and_circular_thumb() {
        for percent in [75, 137, 200, 300] {
            let context = egui::Context::default();
            let mut zoom_percent = percent;
            let mut slider_rect = Rect::NOTHING;
            let mut selection_fill = Color32::TRANSPARENT;
            let mut track_fill = Color32::TRANSPARENT;
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(640.0, 320.0),
                    )),
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        selection_fill = ui.visuals().selection.bg_fill;
                        track_fill = ui.visuals().widgets.inactive.bg_fill;
                        slider_rect = zoom_slider(ui, 220.0, &mut zoom_percent, "Page zoom").rect;
                    });
                },
            );

            let emitted_rects = output
                .shapes
                .iter()
                .enumerate()
                .filter_map(|(index, clipped)| match &clipped.shape {
                    egui::epaint::Shape::Rect(shape) => Some((index, shape)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let track_shapes = emitted_rects
                .iter()
                .copied()
                .filter(|(_, shape)| shape.fill == track_fill && shape.rect.is_positive())
                .collect::<Vec<_>>();
            let selected_shapes = emitted_rects
                .iter()
                .copied()
                .filter(|(_, shape)| shape.fill == selection_fill && shape.rect.is_positive())
                .collect::<Vec<_>>();
            let thumbs = output
                .shapes
                .iter()
                .enumerate()
                .filter_map(|(index, clipped)| match &clipped.shape {
                    egui::epaint::Shape::Circle(circle) => Some((index, circle)),
                    _ => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(
                track_shapes.len(),
                1,
                "zoom={percent}: expected emitted track"
            );
            assert_eq!(
                selected_shapes.len(),
                1,
                "zoom={percent}: expected emitted fill"
            );
            assert_eq!(
                thumbs.len(),
                1,
                "zoom={percent}: expected one circular thumb"
            );
            assert_ne!(
                selection_fill, track_fill,
                "selected and unselected track colors must differ"
            );
            assert!(
                selection_fill.b() > selection_fill.g() && selection_fill.g() > selection_fill.r(),
                "zoom={percent}: selected fill should be visibly blue, got {selection_fill:?}"
            );

            let (track_index, track) = track_shapes[0];
            let (fill_index, fill) = selected_shapes[0];
            let (thumb_index, thumb) = thumbs[0];
            let fill_fraction =
                (percent - MIN_ZOOM_PERCENT) as f32 / (MAX_ZOOM_PERCENT - MIN_ZOOM_PERCENT) as f32;
            let first_thumb_center = track.rect.left() + thumb.radius;
            let last_thumb_center = track.rect.right() - thumb.radius;
            let expected_thumb_x =
                first_thumb_center + fill_fraction * (last_thumb_center - first_thumb_center);
            let actual_fill_position = fill.rect.right() - track.rounding.nw;
            let actual_fill_fraction = (actual_fill_position - first_thumb_center)
                / (last_thumb_center - first_thumb_center);

            assert!(track.rect.is_positive());
            assert_eq!(fill.rect.left(), track.rect.left());
            assert_eq!(fill.rect.top(), track.rect.top());
            assert_eq!(fill.rect.bottom(), track.rect.bottom());
            assert_eq!(fill.rounding, track.rounding);
            assert!(
                (thumb.center.x - expected_thumb_x).abs() <= 0.01,
                "zoom={percent}: thumb x={} should map to {fill_fraction:.4} of usable rail at {expected_thumb_x}",
                thumb.center.x
            );
            assert!(
                (actual_fill_fraction - fill_fraction).abs() <= 0.01,
                "zoom={percent}: emitted selected fill maps to {actual_fill_fraction:.4}, expected {fill_fraction:.4}"
            );
            assert!((fill.rect.right() - (thumb.center.x + track.rounding.nw)).abs() <= 0.01);
            assert_eq!(thumb.center.y, track.rect.center().y);
            assert!(thumb.radius > 0.0);
            assert!(
                track_index < fill_index && fill_index < thumb_index,
                "track, selected fill, and circular thumb should be painted in that order"
            );
            assert!(slider_rect.contains(thumb.center));
        }
    }

    #[test]
    fn zoom_controls_select_presets_and_drag_slider_into_intermediate_value() {
        let viewport = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 320.0));
        let context = egui::Context::default();
        let mut zoom_percent = 137;
        let (initial, changed, slider_rect, presets_rect, _) = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
            &mut zoom_percent,
        );
        assert!(!changed);
        assert!(slider_rect.is_positive());
        assert!(presets_rect.is_positive());
        assert!(text_center(&initial, "137%").is_some());

        let press = egui::pos2(
            slider_rect.left() + slider_rect.width() * 0.2,
            slider_rect.center().y,
        );
        let drag = egui::pos2(
            slider_rect.left() + slider_rect.width() * 0.4,
            slider_rect.center().y,
        );
        let (_, press_changed, _, _, _) = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..pointer_button_frame(press, true)
            },
            &mut zoom_percent,
        );
        assert!(press_changed);
        assert!((MIN_ZOOM_PERCENT..=MAX_ZOOM_PERCENT).contains(&zoom_percent));
        assert!(!ZOOM_PRESETS.contains(&zoom_percent));
        let _ = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                events: vec![egui::Event::PointerMoved(drag)],
                ..Default::default()
            },
            &mut zoom_percent,
        );
        let (_, slider_changed, _, _, _) = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..pointer_button_frame(drag, false)
            },
            &mut zoom_percent,
        );
        assert!(slider_changed || press_changed);
        assert!((MIN_ZOOM_PERCENT..=MAX_ZOOM_PERCENT).contains(&zoom_percent));
        assert!(!ZOOM_PRESETS.contains(&zoom_percent));

        let context = egui::Context::default();
        zoom_percent = 137;
        let (_, _, _, presets_rect, presets_id) = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
            &mut zoom_percent,
        );
        let _ = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..pointer_button_frame(presets_rect.center(), true)
            },
            &mut zoom_percent,
        );
        let (_, _, _, _, _) = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..pointer_button_frame(presets_rect.center(), false)
            },
            &mut zoom_percent,
        );
        assert!(context.memory(|memory| memory.is_popup_open(presets_id.with("popup"))));
        let (opened_menu, _, _, _, _) = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
            &mut zoom_percent,
        );
        let preset_center =
            text_center(&opened_menu, "200%").unwrap_or(egui::Pos2::new(-1.0, -1.0));
        assert!(
            preset_center.x >= 0.0 && preset_center.y >= 0.0,
            "preset popup did not render at {:?}; rendered text: {:?}",
            presets_rect,
            opened_menu
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        );
        let _ = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..pointer_button_frame(preset_center, true)
            },
            &mut zoom_percent,
        );
        let (selected, preset_changed, _, _, _) = render_zoom_controls(
            &context,
            egui::RawInput {
                screen_rect: Some(viewport),
                ..pointer_button_frame(preset_center, false)
            },
            &mut zoom_percent,
        );
        assert!(preset_changed);
        assert_eq!(zoom_percent, 200);
        assert!(text_center(&selected, "200%").is_some());
    }

    #[test]
    fn zoom_slider_is_accessible_and_keyboard_adjusts_the_shared_integer_value() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut zoom_percent = 100;
        let mut slider_id = None;
        let initial = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                let response = zoom_slider(ui, 220.0, &mut zoom_percent, "Page zoom");
                response.request_focus();
                slider_id = Some(response.id);
            });
        });
        assert!(slider_id.is_some());
        let initial_slider = initial
            .platform_output
            .accesskit_update
            .as_ref()
            .and_then(|update| {
                update.nodes.iter().find_map(|(_, node)| {
                    (node.name() == Some("Page zoom")
                        && node.role() == egui::accesskit::Role::Slider)
                        .then_some(node)
                })
            });
        assert!(initial_slider.is_some());
        assert_eq!(
            initial_slider.and_then(|node| node.numeric_value()),
            Some(100.0)
        );
        assert_eq!(
            initial_slider.and_then(|node| node.min_numeric_value()),
            Some(MIN_ZOOM_PERCENT as f64)
        );
        assert_eq!(
            initial_slider.and_then(|node| node.max_numeric_value()),
            Some(MAX_ZOOM_PERCENT as f64)
        );

        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        let incremented = context.run(input, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                zoom_slider(ui, 220.0, &mut zoom_percent, "Page zoom");
            });
        });
        assert_eq!(zoom_percent, 101);
        assert_eq!(context.memory(|memory| memory.focused()), slider_id);
        assert!(incremented
            .platform_output
            .accesskit_update
            .as_ref()
            .is_some_and(|update| update.nodes.iter().any(|(_, node)| {
                node.role() == egui::accesskit::Role::Slider
                    && node.name() == Some("Page zoom")
                    && node.numeric_value() == Some(101.0)
            })));

        for (key, expected) in [
            (egui::Key::Home, MIN_ZOOM_PERCENT),
            (egui::Key::End, MAX_ZOOM_PERCENT),
        ] {
            let input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                }],
                ..Default::default()
            };
            let _ = context.run(input, |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    zoom_slider(ui, 220.0, &mut zoom_percent, "Page zoom");
                });
            });
            assert_eq!(zoom_percent, expected);
        }
    }

    #[test]
    fn status_bar_panel_height_stays_bounded_across_repeated_rendered_frames() {
        let diagnostics = DiagnosticProjection {
            focused: false,
            focus_lock: false,
            dispatch_enabled: false,
            received_events: 0,
            dispatched_frames: 0,
            pointer_activations: 0,
            last_event_category: "",
            performance_events: 0,
            layout_builds: 0,
            layout_cache_hits: 0,
            painted_pages: 0,
            repaint_requests: 0,
        };
        let center = project_center(true, "Save failed", Some("destination unchanged"));

        for (width, layout_name) in [(1_200.0, "normal"), (480.0, "compact")] {
            let context = egui::Context::default();
            let screen_rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 820.0));
            let mut zoom_percent = 137;
            let mut status_heights = Vec::with_capacity(60);
            let mut toolbar_rect = Rect::NOTHING;
            let mut central_rect = Rect::NOTHING;
            let mut final_output = None;

            for _frame in 1..=60 {
                let output = context.run(
                    egui::RawInput {
                        screen_rect: Some(screen_rect),
                        ..Default::default()
                    },
                    |context| {
                        toolbar_rect = egui::TopBottomPanel::top("status-test-toolbar")
                            .exact_height(32.0)
                            .show(context, |ui| ui.label("Toolbar"))
                            .response
                            .rect;
                        let status =
                            egui::TopBottomPanel::bottom("status-test-panel").show(context, |ui| {
                                show(
                                    ui,
                                    VisiblePage::Available {
                                        one_based: 2,
                                        total: 8,
                                    },
                                    &center,
                                    diagnostics,
                                    &mut zoom_percent,
                                );
                            });
                        status_heights.push(status.response.rect.height());
                        central_rect = egui::CentralPanel::default()
                            .show(context, |ui| ui.label("Editor content"))
                            .response
                            .rect;
                    },
                );
                final_output = Some(output);
            }

            let stable_heights = &status_heights[1..];
            let minimum = stable_heights.iter().copied().fold(f32::INFINITY, f32::min);
            let maximum = stable_heights
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
            let monotonic_growth = stable_heights.windows(2).all(|pair| pair[1] >= pair[0])
                && stable_heights.windows(2).any(|pair| pair[1] > pair[0]);
            assert!(
                maximum - minimum <= 1.0,
                "{layout_name} status height drifted across frames 2–60: min={minimum}, max={maximum}, heights={status_heights:?}"
            );
            assert!(
                !monotonic_growth,
                "{layout_name} status height must not grow monotonically"
            );
            assert!(
                toolbar_rect.bottom() < screen_rect.bottom() - status_heights[59],
                "{layout_name} status panel reached the toolbar"
            );
            assert!(
                central_rect.height() > 100.0,
                "{layout_name} status panel consumed the editor viewport"
            );

            let output = final_output.unwrap_or_else(|| unreachable!());
            let text_is_visible = |expected: &str| {
                output.shapes.iter().any(|clipped| {
                    matches!(
                        &clipped.shape,
                        egui::epaint::Shape::Text(text) if text.galley.text() == expected
                    )
                })
            };
            for expected in ["137%", "Diagnostics"] {
                assert!(
                    text_is_visible(expected),
                    "{layout_name} status renderer omitted {expected:?}; labels={:?}",
                    output
                        .shapes
                        .iter()
                        .filter_map(|clipped| match &clipped.shape {
                            egui::epaint::Shape::Text(text) =>
                                Some((text.galley.text().to_owned(), text.pos, clipped.clip_rect,)),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                );
            }
            assert!(
                text_is_visible(if layout_name == "normal" {
                    "Page 2 of 8"
                } else {
                    "2/8"
                }),
                "{layout_name} status renderer omitted the visible page"
            );
            assert!(
                output.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text)
                        if if layout_name == "normal" {
                            text.galley.text() == "Unsaved changes"
                        } else {
                            text.galley.text().starts_with("Dirty")
                        }
                )),
                "{layout_name} status renderer omitted its clean/unsaved state; labels={:?}",
                output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            );
            assert!(
                output.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text)
                        if if layout_name == "normal" {
                            text.galley.text() == "Save failed: destination unchanged"
                        } else {
                            text.galley.text().starts_with("Save failed")
                        }
                )),
                "{layout_name} status renderer omitted operation/error feedback"
            );
        }
    }

    #[test]
    fn status_bar_clips_dynamic_content_at_width_boundaries_without_panel_growth() {
        for width in [1_200.0, 721.0, 720.0, 480.0, 320.0] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let mut zoom_percent = 137;
            let initial_center = project_center(false, "", None);
            let initial_diagnostics = DiagnosticProjection {
                focused: false,
                focus_lock: false,
                dispatch_enabled: true,
                received_events: 0,
                dispatched_frames: 0,
                pointer_activations: 0,
                last_event_category: "",
                performance_events: 0,
                layout_builds: 0,
                layout_cache_hits: 0,
                painted_pages: 0,
                repaint_requests: 0,
            };
            let (mut collapsed_output, _, mut collapsed_rect, _) = render_status_frame(
                &context,
                egui::RawInput::default(),
                width,
                VisiblePage::Available {
                    one_based: 1,
                    total: 8,
                },
                &initial_center,
                initial_diagnostics,
                &mut zoom_percent,
                None,
            );
            for _ in 0..4 {
                let (output, _, rect, _) = render_status_frame(
                    &context,
                    egui::RawInput::default(),
                    width,
                    VisiblePage::Available {
                        one_based: 1,
                        total: 8,
                    },
                    &initial_center,
                    initial_diagnostics,
                    &mut zoom_percent,
                    None,
                );
                collapsed_output = output;
                collapsed_rect = rect;
            }
            assert!(
                collapsed_output.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text)
                        if text.galley.text().contains("Diagnostics")
                )),
                "width={width}: Diagnostics disclosure header missing; panel={collapsed_rect:?}; labels={:?}",
                collapsed_output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Text(text) => Some((
                            text.galley.text().to_owned(),
                            clipped.clip_rect,
                            text.pos,
                        )),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            );
            let (_, _, mut expanded_rect, _) = render_status_frame(
                &context,
                egui::RawInput::default(),
                width,
                VisiblePage::Available {
                    one_based: 1,
                    total: 8,
                },
                &initial_center,
                initial_diagnostics,
                &mut zoom_percent,
                Some(true),
            );
            let mut expanded_output = egui::FullOutput::default();
            for _ in 0..12 {
                let (output, _, rect, _) = render_status_frame(
                    &context,
                    egui::RawInput::default(),
                    width,
                    VisiblePage::Available {
                        one_based: 1,
                        total: 8,
                    },
                    &initial_center,
                    initial_diagnostics,
                    &mut zoom_percent,
                    Some(true),
                );
                expanded_rect = rect;
                expanded_output = output;
            }
            assert!(
                expanded_rect.height() > collapsed_rect.height(),
                "width={width}: Diagnostics disclosure should add its intentional summary row; collapsed={}, expanded={}, labels={:?}",
                collapsed_rect.height(),
                expanded_rect.height(),
                expanded_output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            );
            assert!(
                expanded_rect.height() - collapsed_rect.height() <= 32.0,
                "width={width}: Diagnostics should add only one fixed row"
            );
            assert!(
                expanded_output.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text().starts_with("focus=")
                )),
                "width={width}: opening Diagnostics should render its summary"
            );

            let mut heights = Vec::with_capacity(60);
            let mut last_output = None;
            let mut last_toolbar = Rect::NOTHING;
            let mut last_editor = Rect::NOTHING;
            for frame in 1..=60 {
                let long = frame % 2 == 0;
                let center = if long {
                    project_center(
                        true,
                        "Operation status with a deliberately variable-width payload",
                        Some("destination unchanged after a detailed recoverable failure"),
                    )
                } else {
                    project_center(false, "Saved", None)
                };
                let diagnostics = DiagnosticProjection {
                    focused: frame % 2 == 0,
                    focus_lock: frame % 3 == 0,
                    dispatch_enabled: frame % 2 == 1,
                    received_events: frame,
                    dispatched_frames: frame.saturating_mul(2),
                    pointer_activations: frame / 2,
                    last_event_category: if frame % 2 == 0 { "pointer" } else { "key" },
                    performance_events: u64::from(frame).saturating_mul(10_003),
                    layout_builds: u64::from(frame).saturating_mul(997),
                    layout_cache_hits: u64::from(frame).saturating_mul(65_537),
                    painted_pages: u64::from(frame).saturating_mul(101),
                    repaint_requests: u64::from(frame).saturating_mul(17),
                };
                let page = VisiblePage::Available {
                    one_based: (frame as usize) * 3,
                    total: if long { 999 } else { 8 },
                };
                let (output, toolbar, panel, editor) = render_status_frame(
                    &context,
                    egui::RawInput::default(),
                    width,
                    page,
                    &center,
                    diagnostics,
                    &mut zoom_percent,
                    Some(true),
                );
                heights.push(panel.height());
                last_output = Some(output);
                last_toolbar = toolbar;
                last_editor = editor;
            }

            let min_height = heights.iter().copied().fold(f32::INFINITY, f32::min);
            let max_height = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert!(
                max_height - min_height <= 1.0,
                "width={width}: dynamic text/counters changed stable footer height: {heights:?}"
            );
            assert!(last_toolbar.bottom() < expanded_rect.top());
            assert!(last_editor.height() > 100.0);

            let output = last_output.unwrap_or_else(|| unreachable!());
            let text_shapes = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::epaint::Shape::Text(text) => Some((clipped, text)),
                    _ => None,
                });
            for (clipped, text) in text_shapes {
                let label = text.galley.text();
                assert!(!label.starts_with("Frames:"));
                assert!(!label.starts_with("Path:"));
                assert!(!label.contains("(not saved)"));
                assert!(!label.contains("In-memory demo project"));
                if label.starts_with("Operation status") || label.starts_with("Save failed") {
                    assert_eq!(text.galley.rows.len(), 1, "width={width}: message wrapped");
                    assert!(
                        clipped.clip_rect.width() < width * 0.5,
                        "width={width}: status message was not clipped to its region: {:?}",
                        clipped.clip_rect
                    );
                }
                if label.starts_with("focus=") {
                    assert_eq!(
                        text.galley.rows.len(),
                        1,
                        "width={width}: diagnostics wrapped"
                    );
                    assert!(clipped.clip_rect.width() < width);
                }
            }
            assert!(
                output.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text)
                        if text.galley.text().starts_with("Operation status")
                )),
                "width={width}: long operation feedback was not rendered"
            );
            assert!(
                output.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text().starts_with("focus=")
                )),
                "width={width}: expanded diagnostics summary was not rendered"
            );
            assert!(
                output.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text() == "137%"
                )),
                "width={width}: zoom value disappeared; labels={:?}",
                output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn narrow_status_layout_keeps_zoom_page_error_and_project_state_rendered() {
        let context = egui::Context::default();
        let viewport = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 320.0));
        let center = project_center(true, "Save failed", Some("destination unchanged"));
        let diagnostics = DiagnosticProjection {
            focused: false,
            focus_lock: false,
            dispatch_enabled: false,
            received_events: 0,
            dispatched_frames: 0,
            pointer_activations: 0,
            last_event_category: "",
            performance_events: 0,
            layout_builds: 0,
            layout_cache_hits: 0,
            painted_pages: 0,
            repaint_requests: 0,
        };
        let mut zoom_percent = 137;
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    assert!(!show(
                        ui,
                        VisiblePage::Available {
                            one_based: 2,
                            total: 8,
                        },
                        &center,
                        diagnostics,
                        &mut zoom_percent,
                    ));
                });
            },
        );
        let text_is_visible = |expected: &str| {
            output.shapes.iter().any(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text() == expected
                )
            })
        };
        for expected in ["137%", "2/8", "Dirty"] {
            assert!(
                text_is_visible(expected),
                "missing compact status text: {expected}"
            );
        }
        assert!(output.shapes.iter().any(|clipped| matches!(
            &clipped.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.text().starts_with("Save failed")
        )));
    }
}
