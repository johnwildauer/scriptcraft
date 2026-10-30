use crate::focus_mode::FocusModeView;
use eframe::egui::{
    self, Align, Color32, Direction, FontSelection, Rect, Sense, WidgetInfo, WidgetText, WidgetType,
};
const COMPACT_STATUS_BREAKPOINT: f32 = 720.0;
pub(super) const MIN_ZOOM_PERCENT: u16 = 75;
pub(super) const MAX_ZOOM_PERCENT: u16 = 300;
pub(super) const ZOOM_PRESETS: [u16; 9] = [75, 90, 100, 125, 150, 175, 200, 250, 300];
const FALLBACK_UI_PIXELS_PER_POINT: f32 = 2.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum AppearanceMode {
    #[default]
    System,
    Day,
    Night,
    Midnight,
}

impl AppearanceMode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Day => "Day Mode",
            Self::Night => "Night Mode",
            Self::Midnight => "Midnight Mode",
        }
    }

    fn theme_preference(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Day => egui::ThemePreference::Light,
            Self::Night | Self::Midnight => egui::ThemePreference::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PagePalette {
    pub(super) paper: Color32,
    pub(super) text: Color32,
    pub(super) border: Color32,
    pub(super) selection: Color32,
    pub(super) selection_text: Color32,
    pub(super) caret: Color32,
    pub(super) ime_text: Color32,
}

impl PagePalette {
    pub(super) fn for_theme(mode: AppearanceMode, theme: egui::Theme) -> Self {
        match (theme, mode == AppearanceMode::Midnight) {
            (egui::Theme::Light, _) => Self {
                paper: Color32::from_rgb(246, 246, 246),
                text: Color32::from_rgb(32, 32, 32),
                border: Color32::from_rgb(105, 105, 105),
                selection: Color32::from_rgb(56, 109, 177),
                selection_text: Color32::WHITE,
                caret: Color32::from_rgb(20, 80, 150),
                ime_text: Color32::from_rgb(76, 42, 122),
            },
            (egui::Theme::Dark, true) => Self {
                paper: Color32::from_rgb(12, 14, 18),
                text: Color32::from_rgb(238, 240, 244),
                border: Color32::from_rgb(112, 118, 130),
                selection: Color32::from_rgb(75, 117, 163),
                selection_text: Color32::WHITE,
                caret: Color32::from_rgb(85, 213, 255),
                ime_text: Color32::from_rgb(211, 194, 255),
            },
            (egui::Theme::Dark, false) => Self {
                paper: Color32::from_rgb(28, 30, 35),
                text: Color32::from_rgb(238, 238, 238),
                border: Color32::from_rgb(128, 130, 138),
                selection: Color32::from_rgb(75, 117, 163),
                selection_text: Color32::WHITE,
                caret: Color32::from_rgb(85, 213, 255),
                ime_text: Color32::from_rgb(211, 194, 255),
            },
        }
    }
}

pub(super) fn configure_appearance(
    ctx: &egui::Context,
    mode: AppearanceMode,
    currently_applied: Option<(AppearanceMode, egui::Theme)>,
) -> (egui::Theme, Option<(AppearanceMode, egui::Theme)>) {
    let system_theme = ctx
        .system_theme()
        .unwrap_or_else(|| ctx.options(|options| options.fallback_theme));
    let resolved_theme = match mode {
        AppearanceMode::System => system_theme,
        AppearanceMode::Day => egui::Theme::Light,
        AppearanceMode::Night | AppearanceMode::Midnight => egui::Theme::Dark,
    };
    let applied = Some((mode, resolved_theme));

    if currently_applied != applied {
        let light = egui::Visuals::light();
        let mut dark = egui::Visuals::dark();
        if mode == AppearanceMode::Midnight {
            dark.panel_fill = Color32::from_rgb(12, 14, 18);
            dark.window_fill = Color32::from_rgb(15, 17, 22);
            dark.faint_bg_color = Color32::from_rgb(20, 22, 27);
            dark.extreme_bg_color = Color32::from_rgb(8, 10, 13);
            dark.code_bg_color = Color32::from_rgb(16, 18, 23);
            dark.widgets.noninteractive.bg_fill = Color32::from_rgb(14, 16, 20);
            dark.widgets.inactive.bg_fill = Color32::from_rgb(25, 28, 34);
            dark.widgets.hovered.bg_fill = Color32::from_rgb(38, 43, 52);
            dark.widgets.active.bg_fill = Color32::from_rgb(48, 56, 68);
            dark.widgets.open.bg_fill = Color32::from_rgb(22, 25, 31);
        }
        ctx.set_visuals_of(egui::Theme::Light, light);
        ctx.set_visuals_of(egui::Theme::Dark, dark);
        ctx.set_theme(mode.theme_preference());
    }

    (resolved_theme, applied)
}

#[allow(dead_code)] // Stable slots for the later timer and focus work orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum RightStatusItemId {
    SprintTimer,
    FocusMode,
    Appearance,
    Zoom,
}

impl RightStatusItemId {
    fn order(self) -> u8 {
        match self {
            Self::SprintTimer => 0,
            Self::FocusMode => 1,
            Self::Appearance => 2,
            Self::Zoom => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RightStatusItemWidth {
    pub(super) id: RightStatusItemId,
    pub(super) width: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RightStatusLayout {
    pub(super) inline: Vec<RightStatusItemId>,
    pub(super) overflow: Vec<RightStatusItemId>,
}

pub(super) fn plan_right_status_items(
    available_width: f32,
    item_spacing: f32,
    overflow_trigger_width: f32,
    items: &[RightStatusItemWidth],
) -> RightStatusLayout {
    let mut ordered = items
        .iter()
        .copied()
        .filter(|item| item.width.is_finite() && item.width > 0.0)
        .collect::<Vec<_>>();
    ordered.sort_by_key(|item| item.id.order());
    ordered.dedup_by_key(|item| item.id);

    let available_width = if available_width.is_finite() {
        available_width.max(0.0)
    } else {
        0.0
    };
    let spacing = if item_spacing.is_finite() {
        item_spacing.max(0.0)
    } else {
        0.0
    };
    let total = ordered.iter().map(|item| item.width).sum::<f32>()
        + spacing * ordered.len().saturating_sub(1) as f32;
    if total <= available_width {
        return RightStatusLayout {
            inline: ordered.iter().map(|item| item.id).collect(),
            overflow: Vec::new(),
        };
    }

    let trigger = if overflow_trigger_width.is_finite() {
        overflow_trigger_width.max(0.0)
    } else {
        0.0
    };
    let inline_budget = (available_width - trigger - spacing).max(0.0);
    let mut inline = Vec::new();
    let mut used = 0.0;
    for item in &ordered {
        let candidate = used + if inline.is_empty() { 0.0 } else { spacing } + item.width;
        if candidate > inline_budget {
            break;
        }
        inline.push(item.id);
        used = candidate;
    }
    let overflow = ordered
        .iter()
        .skip(inline.len())
        .map(|item| item.id)
        .collect();
    RightStatusLayout { inline, overflow }
}

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
        let slider_label = "Page zoom";
        let label_reserve = if region_width.is_some() { 64.0 } else { 0.0 };
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
                    ui.add_sized(
                        ui.spacing().interact_size,
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

#[cfg(test)]
pub(super) fn show(
    ui: &mut egui::Ui,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: DiagnosticProjection<'_>,
    zoom_percent: &mut u16,
) -> bool {
    show_impl(ui, page, center, diagnostics, zoom_percent, None)
}

#[allow(clippy::too_many_arguments)] // Keep transient timer ownership at the app caller.
pub(super) fn show_with_focus_mode(
    ui: &mut egui::Ui,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: DiagnosticProjection<'_>,
    zoom_percent: &mut u16,
    appearance: &mut AppearanceMode,
    timer_open: &mut bool,
    timer_just_opened: &mut bool,
    focus_view: &FocusModeView,
    focus_activated: &mut bool,
) -> (bool, bool, Option<Rect>, Option<Rect>) {
    show_impl_with_appearance(
        ui,
        page,
        center,
        diagnostics,
        zoom_percent,
        appearance,
        None,
        Some(timer_open),
        Some(timer_just_opened),
        Some(focus_view),
        Some(focus_activated),
    )
}

fn measured_button_width(ui: &egui::Ui, text: &str) -> f32 {
    let text_width = ui.fonts(|fonts| {
        fonts
            .layout_no_wrap(
                text.to_owned(),
                egui::TextStyle::Button.resolve(ui.style()),
                Color32::WHITE,
            )
            .size()
            .x
    });
    text_width + ui.spacing().button_padding.x * 2.0
}

fn appearance_menu(ui: &mut egui::Ui, appearance: &mut AppearanceMode) -> bool {
    let previous = *appearance;
    let menu = ui.menu_button("Appearance", |ui| {
        ui.set_min_width(156.0);
        for choice in [
            AppearanceMode::System,
            AppearanceMode::Day,
            AppearanceMode::Night,
            AppearanceMode::Midnight,
        ] {
            ui.selectable_value(appearance, choice, choice.label());
        }
    });
    menu.response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Button,
            menu.response.enabled,
            "Appearance mode".to_owned(),
        )
    });
    *appearance != previous
}

fn appearance_choices(ui: &mut egui::Ui, appearance: &mut AppearanceMode) -> bool {
    let previous = *appearance;
    for choice in [
        AppearanceMode::System,
        AppearanceMode::Day,
        AppearanceMode::Night,
        AppearanceMode::Midnight,
    ] {
        ui.selectable_value(appearance, choice, choice.label());
    }
    *appearance != previous
}

#[allow(clippy::too_many_arguments)] // Keep each transient status action owned by its desktop caller.
fn render_right_item(
    ui: &mut egui::Ui,
    item: RightStatusItemId,
    width: f32,
    zoom_percent: &mut u16,
    appearance: &mut AppearanceMode,
    timer_open: Option<&mut bool>,
    timer_just_opened: Option<&mut bool>,
    focus_view: Option<&FocusModeView>,
    focus_activated: Option<&mut bool>,
    focus_width: f32,
) -> (bool, bool, Option<Rect>, Option<Rect>) {
    ui.push_id(item, |ui| match item {
        RightStatusItemId::SprintTimer => {
            let mut activated = false;
            let mut timer_rect = None;
            ui.allocate_ui_with_layout(
                egui::vec2(width, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let timer_item =
                        StatusItem::clickable_icon("sprint-timer-status-item", "⏱", "Sprint Timer");
                    if let Some(rendered) = render_item(ui, &timer_item) {
                        rendered.response.clone().on_hover_text("Sprint Timer");
                        timer_rect = Some(rendered.response.rect);
                        activated = rendered.activated;
                    }
                },
            );
            if activated {
                if let Some(open) = timer_open {
                    *open = !*open;
                    if *open {
                        if let Some(just_opened) = timer_just_opened {
                            *just_opened = true;
                        }
                    }
                }
            }
            (false, false, timer_rect, None)
        }
        RightStatusItemId::Appearance => {
            ui.allocate_ui_with_layout(
                egui::vec2(width, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| (false, appearance_menu(ui, appearance), None, None),
            )
            .inner
        }
        RightStatusItemId::Zoom => {
            ui.allocate_ui_with_layout(
                egui::vec2(width, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    (
                        zoom_controls(ui, zoom_percent, Some(width)).0,
                        false,
                        None,
                        None,
                    )
                },
            )
            .inner
        }
        RightStatusItemId::FocusMode => {
            let mut focus_rect = None;
            ui.allocate_ui_with_layout(
                egui::vec2(focus_width, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    if let Some(view) = focus_view {
                        let item = StatusItem::clickable_text(
                            "focus-mode-status-item",
                            view.button_text(),
                            view.accessible_name(),
                        );
                        let rendered = ui
                            .add_enabled_ui(view.enabled(), |ui| {
                                let response = ui
                                    .push_id(item.id, |ui| {
                                        ui.add_sized(
                                            egui::vec2(focus_width, ui.spacing().interact_size.y),
                                            egui::Button::new(&item.label),
                                        )
                                    })
                                    .inner;
                                Some(interactive_response(ui, response, &item.accessible_name))
                            })
                            .inner;
                        if let Some(rendered) = rendered {
                            rendered
                                .response
                                .clone()
                                .on_hover_text(view.accessible_name());
                            focus_rect = Some(rendered.response.rect);
                            if rendered.activated {
                                if let Some(activated) = focus_activated {
                                    *activated = true;
                                }
                            }
                        }
                    }
                    (false, false, None, focus_rect)
                },
            )
            .inner
        }
    })
    .inner
}

#[allow(clippy::too_many_arguments)] // Each live control state remains explicitly caller-owned.
fn overflow_menu(
    ui: &mut egui::Ui,
    overflow: &[RightStatusItemId],
    page: VisiblePage,
    center: &CenterProjection,
    zoom_percent: &mut u16,
    appearance: &mut AppearanceMode,
    timer_open: &mut Option<&mut bool>,
    timer_just_opened: &mut Option<&mut bool>,
    focus_view: Option<&FocusModeView>,
    focus_activated: Option<&mut bool>,
) -> (bool, bool, Option<Rect>, Option<Rect>) {
    let mut zoom_changed = false;
    let mut appearance_changed = false;
    let mut timer_rect = None;
    let mut focus_rect = None;
    let mut focus_activated = focus_activated;
    let trigger = if center.error.is_some() {
        "! More"
    } else {
        "More"
    };
    let accessible_name = if center.error.is_some() {
        "Error. More status controls and details"
    } else {
        "More status controls and details"
    };
    let menu = ui.menu_button(trigger, |ui| {
        ui.set_min_width(288.0);
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                ui.label(page.label());
                ui.label(center.clean_label);
                if let Some(message) = status_message(center) {
                    ui.label(message);
                }
                ui.separator();
                for item in overflow {
                    match item {
                        RightStatusItemId::Appearance => {
                            appearance_changed |= appearance_choices(ui, appearance);
                        }
                        RightStatusItemId::Zoom => {
                            zoom_changed |= zoom_controls(ui, zoom_percent, Some(270.0)).0;
                        }
                        RightStatusItemId::SprintTimer => {
                            let response = ui
                                .push_id(RightStatusItemId::SprintTimer, |ui| {
                                    ui.button("⏱  Sprint Timer")
                                })
                                .inner;
                            response.clone().on_hover_text("Sprint Timer");
                            timer_rect = Some(response.rect);
                            response.widget_info(|| {
                                WidgetInfo::labeled(
                                    WidgetType::Button,
                                    response.enabled,
                                    "Sprint Timer".to_owned(),
                                )
                            });
                            if response.clicked() {
                                if let Some(open) = timer_open.as_deref_mut() {
                                    *open = !*open;
                                    if *open {
                                        if let Some(just_opened) = timer_just_opened.as_deref_mut()
                                        {
                                            *just_opened = true;
                                        }
                                    }
                                }
                                ui.close_menu();
                            }
                        }
                        RightStatusItemId::FocusMode => {
                            if let Some(view) = focus_view {
                                let response = ui
                                    .add_enabled_ui(view.enabled(), |ui| {
                                        ui.push_id(RightStatusItemId::FocusMode, |ui| {
                                            ui.button(view.button_text())
                                        })
                                        .inner
                                    })
                                    .inner;
                                response.clone().on_hover_text(view.accessible_name());
                                response.widget_info(|| {
                                    WidgetInfo::labeled(
                                        WidgetType::Button,
                                        response.enabled,
                                        view.accessible_name(),
                                    )
                                });
                                focus_rect = Some(response.rect);
                                if response.clicked() {
                                    if let Some(activated) = focus_activated.as_deref_mut() {
                                        *activated = true;
                                    }
                                    ui.close_menu();
                                }
                            }
                        }
                    }
                }
            });
    });
    menu.response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Button,
            menu.response.enabled,
            accessible_name.to_owned(),
        )
    });
    (zoom_changed, appearance_changed, timer_rect, focus_rect)
}

#[cfg(test)]
fn show_impl(
    ui: &mut egui::Ui,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: DiagnosticProjection<'_>,
    zoom_percent: &mut u16,
    diagnostics_open: Option<bool>,
) -> bool {
    let mut appearance = AppearanceMode::System;
    show_impl_with_appearance(
        ui,
        page,
        center,
        diagnostics,
        zoom_percent,
        &mut appearance,
        diagnostics_open,
        None,
        None,
        None,
        None,
    )
    .0
}

#[allow(clippy::too_many_arguments)] // Explicit projections and caller-owned state avoid hidden status globals.
fn show_impl_with_appearance(
    ui: &mut egui::Ui,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: DiagnosticProjection<'_>,
    zoom_percent: &mut u16,
    appearance: &mut AppearanceMode,
    diagnostics_open: Option<bool>,
    timer_open: Option<&mut bool>,
    timer_just_opened: Option<&mut bool>,
    focus_view: Option<&FocusModeView>,
    focus_activated: Option<&mut bool>,
) -> (bool, bool, Option<Rect>, Option<Rect>) {
    let available_width = ui.available_width().max(0.0);
    let mut zoom_changed = false;
    let mut appearance_changed = false;
    let mut timer_rect = None;
    let mut focus_rect = None;
    let mut timer_open = timer_open;
    let mut timer_just_opened = timer_just_opened;
    let mut focus_activated = focus_activated;
    let message = status_message(center);
    let clean_label =
        if uses_compact_status_layout(available_width) && center.clean_label == "Unsaved changes" {
            "Dirty"
        } else {
            center.clean_label
        };
    let row_height = ui.spacing().interact_size.y;
    let timer_width = ui.spacing().interact_size.x.max(36.0);
    ui.columns(3, |regions| {
        let right_width = regions[2].available_width().max(0.0);
        let appearance_width = measured_button_width(&regions[2], "Appearance");
        let zoom_width = 270.0;
        let trigger_label = if center.error.is_some() {
            "! More"
        } else {
            "More"
        };
        let trigger_width = measured_button_width(&regions[2], trigger_label);
        let mut right_items = Vec::with_capacity(4);
        if timer_open.is_some() {
            right_items.push(RightStatusItemWidth {
                id: RightStatusItemId::SprintTimer,
                width: timer_width,
            });
        }
        let focus_width = measured_button_width(&regions[2], "⛶ Focus Mode").max(112.0);
        if focus_view.is_some() {
            right_items.push(RightStatusItemWidth {
                id: RightStatusItemId::FocusMode,
                width: focus_width,
            });
        }
        right_items.extend([
            RightStatusItemWidth {
                id: RightStatusItemId::Appearance,
                width: appearance_width,
            },
            RightStatusItemWidth {
                id: RightStatusItemId::Zoom,
                width: zoom_width,
            },
        ]);
        let plan = plan_right_status_items(
            right_width,
            regions[2].spacing().item_spacing.x,
            trigger_width,
            &right_items,
        );

        // Controls are processed first so a zoom edit makes this frame's page
        // projection unavailable before the page label is painted.
        one_line_region(
            &mut regions[2],
            egui::vec2(right_width, row_height),
            |right_ui| {
                right_ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |right_ui| {
                        if !plan.overflow.is_empty() {
                            let (zoom, mode, timer_button_rect, focus_button_rect) = overflow_menu(
                                right_ui,
                                &plan.overflow,
                                page,
                                center,
                                zoom_percent,
                                appearance,
                                &mut timer_open,
                                &mut timer_just_opened,
                                focus_view,
                                focus_activated.as_deref_mut(),
                            );
                            zoom_changed |= zoom;
                            appearance_changed |= mode;
                            timer_rect = timer_button_rect;
                            focus_rect = focus_button_rect;
                        }
                        for item in plan.inline.iter().rev() {
                            let width = match item {
                                RightStatusItemId::Appearance => appearance_width,
                                RightStatusItemId::Zoom => zoom_width,
                                RightStatusItemId::SprintTimer => timer_width,
                                RightStatusItemId::FocusMode => focus_width,
                            };
                            let (zoom, mode, timer_button_rect, focus_button_rect) =
                                render_right_item(
                                    right_ui,
                                    *item,
                                    width.min(right_ui.available_width()),
                                    zoom_percent,
                                    appearance,
                                    timer_open.as_deref_mut(),
                                    timer_just_opened.as_deref_mut(),
                                    focus_view,
                                    focus_activated.as_deref_mut(),
                                    focus_width,
                                );
                            zoom_changed |= zoom;
                            appearance_changed |= mode;
                            timer_rect = timer_button_rect.or(timer_rect);
                            focus_rect = focus_button_rect.or(focus_rect);
                        }
                    },
                );
            },
        );

        let page_width = regions[0].available_width().max(0.0);
        one_line_region(
            &mut regions[0],
            egui::vec2(page_width, row_height),
            |page_ui| {
                let visible_page = if zoom_changed {
                    VisiblePage::Unavailable
                } else {
                    page
                };
                let item = StatusItem {
                    accessible_name: visible_page.label(),
                    ..StatusItem::text("visible-page", visible_page.label())
                };
                render_item(page_ui, &item);
            },
        );

        let status_width = regions[1].available_width().max(0.0);
        one_line_region(
            &mut regions[1],
            egui::vec2(status_width, row_height),
            |region| {
                let status_rect = region.max_rect().intersect(region.clip_rect());
                let clean_width = (status_rect.width() * 0.4).max(0.0);
                let clean_rect = Rect::from_min_max(
                    status_rect.min,
                    egui::pos2(status_rect.left() + clean_width, status_rect.bottom()),
                );
                let message_rect = Rect::from_min_max(
                    egui::pos2(clean_rect.right(), status_rect.top()),
                    status_rect.max,
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
            },
        );
    });
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
    (zoom_changed, appearance_changed, timer_rect, focus_rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wo004_focus_action_supports_enter_space_and_reports_accessible_state() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut controller = crate::focus_mode::FocusModeController::default();
        controller.observe(Some(false), std::time::Duration::ZERO);
        let view = controller.view();
        let item = StatusItem::clickable_text(
            "focus-mode-status-item",
            view.button_text(),
            view.accessible_name(),
        );
        let mut widget_id = None;
        let mut activated = false;
        let render =
            |input: egui::RawInput, widget_id: &mut Option<egui::Id>, activated: &mut bool| {
                context.run(input, |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        let rendered = ui
                            .add_enabled_ui(view.enabled(), |ui| {
                                let response = ui
                                    .push_id(item.id, |ui| {
                                        ui.add_sized(
                                            egui::vec2(144.0, ui.spacing().interact_size.y),
                                            egui::Button::new(&item.label),
                                        )
                                    })
                                    .inner;
                                Some(interactive_response(ui, response, &item.accessible_name))
                            })
                            .inner;
                        if let Some(rendered) = rendered {
                            *widget_id = Some(rendered.response.id);
                            *activated = rendered.activated;
                        }
                    });
                })
            };
        let initial = render(egui::RawInput::default(), &mut widget_id, &mut activated);
        let focus_id = widget_id.unwrap_or_else(|| unreachable!("Focus Mode widget was rendered"));
        let accesskit = initial.platform_output.accesskit_update;
        assert!(accesskit.is_some_and(|update| {
            update.nodes.iter().any(|(_, node)| {
                node.role() == egui::accesskit::Role::Button
                    && node.name() == Some(view.accessible_name().as_str())
            })
        }));

        for key in [egui::Key::Enter, egui::Key::Space] {
            context.memory_mut(|memory| memory.request_focus(focus_id));
            activated = false;
            let output = render(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::default(),
                    }],
                    ..Default::default()
                },
                &mut widget_id,
                &mut activated,
            );
            assert!(activated, "keyboard activation failed for {key:?}");
            assert!(output.platform_output.accesskit_update.is_some());
        }
    }

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

    #[allow(clippy::too_many_arguments)]
    fn open_status_overflow(
        context: &egui::Context,
        width: f32,
        page: VisiblePage,
        center: &CenterProjection,
        diagnostics: DiagnosticProjection<'_>,
        zoom_percent: &mut u16,
        diagnostics_open: Option<bool>,
    ) -> egui::FullOutput {
        let (initial, _, _, _) = render_status_frame(
            context,
            egui::RawInput::default(),
            width,
            page,
            center,
            diagnostics,
            zoom_percent,
            diagnostics_open,
        );
        let Some(trigger) = initial
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.text() == "More" || text.galley.text() == "! More" =>
                {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
        else {
            return initial;
        };
        for pressed in [true, false] {
            let _ = render_status_frame(
                context,
                pointer_button_frame(trigger, pressed),
                width,
                page,
                center,
                diagnostics,
                zoom_percent,
                diagnostics_open,
            );
        }
        render_status_frame(
            context,
            egui::RawInput::default(),
            width,
            page,
            center,
            diagnostics,
            zoom_percent,
            diagnostics_open,
        )
        .0
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
    fn wo002_right_item_plan_preserves_order_and_moves_only_a_suffix_to_overflow() {
        let items = [
            RightStatusItemWidth {
                id: RightStatusItemId::Zoom,
                width: 250.0,
            },
            RightStatusItemWidth {
                id: RightStatusItemId::Appearance,
                width: 110.0,
            },
            RightStatusItemWidth {
                id: RightStatusItemId::FocusMode,
                width: 70.0,
            },
            RightStatusItemWidth {
                id: RightStatusItemId::SprintTimer,
                width: 70.0,
            },
        ];
        let all = plan_right_status_items(600.0, 6.0, 60.0, &items);
        assert_eq!(
            all.inline,
            [
                RightStatusItemId::SprintTimer,
                RightStatusItemId::FocusMode,
                RightStatusItemId::Appearance,
                RightStatusItemId::Zoom,
            ]
        );
        assert!(all.overflow.is_empty());

        let suffix = plan_right_status_items(400.0, 6.0, 60.0, &items);
        assert_eq!(
            suffix.inline,
            [
                RightStatusItemId::SprintTimer,
                RightStatusItemId::FocusMode,
                RightStatusItemId::Appearance,
            ]
        );
        assert_eq!(suffix.overflow, [RightStatusItemId::Zoom]);

        let narrow = plan_right_status_items(280.0, 6.0, 60.0, &items);
        assert_eq!(
            narrow.inline,
            [RightStatusItemId::SprintTimer, RightStatusItemId::FocusMode]
        );
        assert_eq!(
            narrow.overflow,
            [RightStatusItemId::Appearance, RightStatusItemId::Zoom]
        );
        let below_minimum = plan_right_status_items(40.0, 6.0, 60.0, &items);
        assert!(below_minimum.inline.is_empty());
        assert_eq!(below_minimum.overflow.len(), 4);
    }

    #[test]
    fn wo002_right_status_fixture_items_render_inline_and_in_overflow() {
        let items = [
            RightStatusItemWidth {
                id: RightStatusItemId::Zoom,
                width: 250.0,
            },
            RightStatusItemWidth {
                id: RightStatusItemId::Appearance,
                width: 110.0,
            },
            RightStatusItemWidth {
                id: RightStatusItemId::FocusMode,
                width: 70.0,
            },
            RightStatusItemWidth {
                id: RightStatusItemId::SprintTimer,
                width: 70.0,
            },
        ];
        let label = |id| match id {
            RightStatusItemId::SprintTimer => "Sprint Timer",
            RightStatusItemId::FocusMode => "Focus Mode",
            RightStatusItemId::Appearance => "Appearance",
            RightStatusItemId::Zoom => "Zoom",
        };

        for width in [600.0, 400.0, 280.0] {
            let context = egui::Context::default();
            let screen = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 180.0));
            let mut plan = None;
            let render = |input: egui::RawInput, plan: &mut Option<RightStatusLayout>| {
                context.run(input, |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        let layout = plan_right_status_items(
                            ui.available_width(),
                            ui.spacing().item_spacing.x,
                            measured_button_width(ui, "More"),
                            &items,
                        );
                        *plan = Some(layout.clone());
                        let row_height = ui.spacing().interact_size.y;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !layout.overflow.is_empty() {
                                ui.menu_button("More", |ui| {
                                    for id in &layout.overflow {
                                        ui.label(label(*id));
                                    }
                                });
                            }
                            for id in layout.inline.iter().rev() {
                                let width = items
                                    .iter()
                                    .find(|item| item.id == *id)
                                    .map_or(0.0, |item| item.width);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(width, row_height),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.label(label(*id));
                                    },
                                );
                            }
                        });
                    });
                })
            };

            let mut output = render(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                &mut plan,
            );
            let layout = plan.take().unwrap_or_else(|| unreachable!());
            let inline_positions = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::epaint::Shape::Text(text) => {
                        let name = text.galley.text();
                        layout
                            .inline
                            .iter()
                            .any(|id| label(*id) == name)
                            .then_some((name.to_owned(), text.pos.x))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let mut sorted_inline = inline_positions.clone();
            sorted_inline.sort_by(|first, second| first.1.total_cmp(&second.1));
            assert_eq!(
                sorted_inline
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>(),
                layout
                    .inline
                    .iter()
                    .map(|id| label(*id))
                    .collect::<Vec<_>>(),
                "inline fixture render order at {width} points"
            );

            if !layout.overflow.is_empty() {
                let trigger = output
                    .shapes
                    .iter()
                    .find_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Text(text) if text.galley.text() == "More" => {
                            Some(text.pos + text.galley.size() * 0.5)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| unreachable!("overflow trigger must be rendered"));
                for pressed in [true, false] {
                    let _ = render(
                        egui::RawInput {
                            screen_rect: Some(screen),
                            events: vec![
                                egui::Event::PointerMoved(trigger),
                                egui::Event::PointerButton {
                                    pos: trigger,
                                    button: egui::PointerButton::Primary,
                                    pressed,
                                    modifiers: egui::Modifiers::default(),
                                },
                            ],
                            ..Default::default()
                        },
                        &mut plan,
                    );
                }
                output = render(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    &mut plan,
                );
                let mut overflow_positions = output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Text(text) => {
                            let name = text.galley.text();
                            layout
                                .overflow
                                .iter()
                                .any(|id| label(*id) == name)
                                .then_some((name.to_owned(), text.pos.y))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                overflow_positions.sort_by(|first, second| first.1.total_cmp(&second.1));
                assert_eq!(
                    overflow_positions
                        .iter()
                        .map(|(name, _)| name.as_str())
                        .collect::<Vec<_>>(),
                    layout
                        .overflow
                        .iter()
                        .map(|id| label(*id))
                        .collect::<Vec<_>>(),
                    "overflow fixture render order at {width} points"
                );
            }
        }
    }

    fn relative_luminance(color: Color32, background: Color32) -> f64 {
        let alpha = f64::from(color.a()) / 255.0;
        let channel = |foreground: u8, behind: u8| {
            let composited =
                (f64::from(foreground) * alpha + f64::from(behind) * (1.0 - alpha)) / 255.0;
            if composited <= 0.04045 {
                composited / 12.92
            } else {
                ((composited + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r(), background.r())
            + 0.7152 * channel(color.g(), background.g())
            + 0.0722 * channel(color.b(), background.b())
    }

    fn contrast_ratio(first: Color32, second: Color32) -> f64 {
        let first_luminance = relative_luminance(first, second);
        let second_luminance = relative_luminance(second, second);
        let lighter = first_luminance.max(second_luminance);
        let darker = first_luminance.min(second_luminance);
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn wo002_page_palettes_meet_text_selection_caret_ime_and_border_contrast() {
        for (mode, theme) in [
            (AppearanceMode::System, egui::Theme::Light),
            (AppearanceMode::Day, egui::Theme::Light),
            (AppearanceMode::System, egui::Theme::Dark),
            (AppearanceMode::Night, egui::Theme::Dark),
            (AppearanceMode::Midnight, egui::Theme::Dark),
        ] {
            let palette = PagePalette::for_theme(mode, theme);
            assert!(contrast_ratio(palette.text, palette.paper) >= 4.5);
            assert!(contrast_ratio(palette.ime_text, palette.paper) >= 4.5);
            assert!(contrast_ratio(palette.selection_text, palette.selection) >= 4.5);
            assert!(contrast_ratio(palette.selection, palette.paper) >= 3.0);
            assert!(contrast_ratio(palette.caret, palette.paper) >= 3.0);
            assert!(contrast_ratio(palette.border, palette.paper) >= 3.0);
        }
    }

    #[test]
    fn wo002_system_tracks_resolved_theme_and_explicit_modes_ignore_system_changes() {
        let context = egui::Context::default();
        context.options_mut(|options| options.fallback_theme = egui::Theme::Light);
        let mut applied = None;
        for (system_theme, expected) in [
            (Some(egui::Theme::Light), egui::Theme::Light),
            (Some(egui::Theme::Dark), egui::Theme::Dark),
            (None, egui::Theme::Light),
            (Some(egui::Theme::Light), egui::Theme::Light),
        ] {
            let _ = context.run(
                egui::RawInput {
                    system_theme,
                    ..Default::default()
                },
                |context| {
                    let (resolved, next) =
                        configure_appearance(context, AppearanceMode::System, applied);
                    applied = next;
                    assert_eq!(resolved, expected);
                    assert_eq!(context.theme(), expected);
                    let palette = PagePalette::for_theme(AppearanceMode::System, resolved);
                    assert_eq!(
                        palette.paper,
                        PagePalette::for_theme(
                            if expected == egui::Theme::Dark {
                                AppearanceMode::Night
                            } else {
                                AppearanceMode::Day
                            },
                            expected,
                        )
                        .paper
                    );
                },
            );
        }

        for (mode, expected) in [
            (AppearanceMode::Day, egui::Theme::Light),
            (AppearanceMode::Night, egui::Theme::Dark),
            (AppearanceMode::Midnight, egui::Theme::Dark),
        ] {
            let mut current = None;
            for system_theme in [Some(egui::Theme::Dark), Some(egui::Theme::Light)] {
                let _ = context.run(
                    egui::RawInput {
                        system_theme,
                        ..Default::default()
                    },
                    |context| {
                        let (resolved, next) = configure_appearance(context, mode, current);
                        current = next;
                        assert_eq!(resolved, expected);
                        assert_eq!(context.theme(), expected);
                    },
                );
            }
        }

        let mut night_visuals = egui::Visuals::dark();
        let mut applied = None;
        let _ = context.run(
            egui::RawInput {
                system_theme: Some(egui::Theme::Dark),
                ..Default::default()
            },
            |context| {
                let (_, next) = configure_appearance(context, AppearanceMode::Night, applied);
                applied = next;
                night_visuals = context.style_of(egui::Theme::Dark).visuals.clone();
                let (_, _) = configure_appearance(context, AppearanceMode::Midnight, applied);
                let midnight = context.style_of(egui::Theme::Dark).visuals.clone();
                assert!(
                    relative_luminance(midnight.panel_fill, Color32::BLACK)
                        < relative_luminance(night_visuals.panel_fill, Color32::BLACK)
                );
                assert!(
                    relative_luminance(midnight.window_fill, Color32::BLACK)
                        < relative_luminance(night_visuals.window_fill, Color32::BLACK)
                );
            },
        );
    }

    #[test]
    fn wo002_appearance_menu_has_exact_choices_and_keyboard_accessible_widget() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let screen = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 320.0));
        let mut mode = AppearanceMode::System;
        let render = |context: &egui::Context, input: egui::RawInput, mode: &mut AppearanceMode| {
            context.run(input, |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    appearance_menu(ui, mode);
                });
            })
        };
        let initial = render(
            &context,
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            &mut mode,
        );
        assert!(initial
            .platform_output
            .accesskit_update
            .as_ref()
            .is_some_and(|update| update.nodes.iter().any(|(_, node)| {
                node.role() == egui::accesskit::Role::Button
                    && node.name() == Some("Appearance mode")
            })));
        let button = text_center(&initial, "Appearance").unwrap_or(egui::Pos2::ZERO);
        for pressed in [true, false] {
            let _ = render(
                &context,
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..pointer_button_frame(button, pressed)
                },
                &mut mode,
            );
        }
        let menu = render(
            &context,
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            &mut mode,
        );
        for label in ["System", "Day Mode", "Night Mode", "Midnight Mode"] {
            assert!(text_center(&menu, label).is_some(), "missing {label}");
        }
        let night = text_center(&menu, "Night Mode").unwrap_or(egui::Pos2::ZERO);
        for pressed in [true, false] {
            let _ = render(
                &context,
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..pointer_button_frame(night, pressed)
                },
                &mut mode,
            );
        }
        assert_eq!(mode, AppearanceMode::Night);
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
    fn native_ui_scale_prefers_valid_os_value_once_and_uses_2_3_fallback() {
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
            Some(f32::NEG_INFINITY),
        ] {
            assert_eq!(resolved_ui_pixels_per_point(invalid), 2.3);
            assert_eq!(ui_zoom_factor(invalid), 2.3);
        }
        assert!((ui_zoom_factor(None) * 1.5 - 3.45).abs() < 0.000_001);
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
            let expected = "Diagnostics";
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
            if layout_name == "normal" {
                assert!(text_is_visible("137%"));
            } else {
                assert!(text_is_visible("More") || text_is_visible("! More"));
                let overflow = open_status_overflow(
                    &context,
                    width,
                    VisiblePage::Available {
                        one_based: 2,
                        total: 8,
                    },
                    &center,
                    diagnostics,
                    &mut zoom_percent,
                    None,
                );
                assert!(overflow.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text() == "137%"
                )));
            }
            assert!(
                text_is_visible("Page 2 of 8"),
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
            if !output.shapes.iter().any(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text() == "137%"
                )
            }) {
                let center = project_center(
                    true,
                    "Operation status with a deliberately variable-width payload",
                    Some("destination unchanged after a detailed recoverable failure"),
                );
                let diagnostics = DiagnosticProjection {
                    focused: true,
                    focus_lock: false,
                    dispatch_enabled: false,
                    received_events: 60,
                    dispatched_frames: 120,
                    pointer_activations: 30,
                    last_event_category: "pointer",
                    performance_events: 600_180,
                    layout_builds: 59_820,
                    layout_cache_hits: 3_932_220,
                    painted_pages: 6_060,
                    repaint_requests: 1_020,
                };
                let overflow = open_status_overflow(
                    &context,
                    width,
                    VisiblePage::Available {
                        one_based: 180,
                        total: 999,
                    },
                    &center,
                    diagnostics,
                    &mut zoom_percent,
                    Some(true),
                );
                assert!(
                    overflow.shapes.iter().any(|clipped| matches!(
                        &clipped.shape,
                        egui::epaint::Shape::Text(text) if text.galley.text() == "137%"
                    )),
                    "width={width}: Zoom is not reachable in status overflow; popup labels={:?}",
                    overflow
                        .shapes
                        .iter()
                        .filter_map(|clipped| match &clipped.shape {
                            egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                );
            }
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
        }
    }

    #[test]
    fn narrow_status_layout_keeps_zoom_page_error_and_project_state_rendered() {
        let context = egui::Context::default();
        let width = 320.0;
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
        let page = VisiblePage::Available {
            one_based: 2,
            total: 8,
        };
        let (output, _, _, _) = render_status_frame(
            &context,
            egui::RawInput::default(),
            width,
            page,
            &center,
            diagnostics,
            &mut zoom_percent,
            None,
        );
        let text_is_visible = |expected: &str| {
            output.shapes.iter().any(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text() == expected
                )
            })
        };
        for expected in ["Page 2 of 8", "Dirty", "! More"] {
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
        let overflow = open_status_overflow(
            &context,
            width,
            page,
            &center,
            diagnostics,
            &mut zoom_percent,
            None,
        );
        for expected in ["137%", "Page 2 of 8", "Unsaved changes", "Save failed"] {
            assert!(
                overflow.shapes.iter().any(|clipped| matches!(
                    &clipped.shape,
                    egui::epaint::Shape::Text(text) if text.galley.text().contains(expected)
                )),
                "overflow omitted {expected}"
            );
        }
    }
}
