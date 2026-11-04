use crate::focus_mode::FocusModeView;
use eframe::egui::{
    self, Align, Color32, Direction, FontSelection, Pos2, Rect, Sense, Vec2, WidgetInfo,
    WidgetText, WidgetType,
};
pub(super) const MIN_ZOOM_PERCENT: u16 = 75;
pub(super) const MAX_ZOOM_PERCENT: u16 = 300;
pub(super) const ZOOM_PRESETS: [u16; 9] = [75, 90, 100, 125, 150, 175, 200, 250, 300];

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
    pub(super) subdued_text: Color32,
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
                subdued_text: Color32::from_rgb(98, 98, 98),
                border: Color32::from_rgb(105, 105, 105),
                selection: Color32::from_rgb(56, 109, 177),
                selection_text: Color32::WHITE,
                caret: Color32::from_rgb(20, 80, 150),
                ime_text: Color32::from_rgb(76, 42, 122),
            },
            (egui::Theme::Dark, true) => Self {
                paper: Color32::from_rgb(12, 14, 18),
                text: Color32::from_rgb(238, 240, 244),
                subdued_text: Color32::from_rgb(156, 163, 173),
                border: Color32::from_rgb(112, 118, 130),
                selection: Color32::from_rgb(75, 117, 163),
                selection_text: Color32::WHITE,
                caret: Color32::from_rgb(85, 213, 255),
                ime_text: Color32::from_rgb(211, 194, 255),
            },
            (egui::Theme::Dark, false) => Self {
                paper: Color32::from_rgb(28, 30, 35),
                text: Color32::from_rgb(238, 238, 238),
                subdued_text: Color32::from_rgb(156, 163, 173),
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
        .unwrap_or(1.0)
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
    scroll_offset: Vec2,
}

/// The viewport-center point expressed relative to a bounded page coordinate.
/// `outside_page_offset_points` retains any gap or edge distance separately,
/// so a center in a gap never becomes a fabricated screenplay position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ViewportCenterAnchor {
    pub(super) page_index: usize,
    pub(super) page_local_document_point: Vec2,
    pub(super) outside_page_offset_points: Vec2,
    pub(super) viewport_center: Pos2,
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
    pub(super) fn capture_center_anchor(
        &self,
        frame: u64,
        layout_revision: u64,
        viewport_rect: Rect,
        scale: f32,
    ) -> Option<ViewportCenterAnchor> {
        let snapshot = self.published.as_ref()?;
        let age = frame.checked_sub(snapshot.published_frame)?;
        if age > 1
            || snapshot.layout_revision != layout_revision
            || snapshot.viewport_rect != viewport_rect
            || !scale.is_finite()
            || scale <= 0.0
            || (snapshot.scale - scale).abs() > f32::EPSILON
            || !valid_geometry_rect(snapshot.clip)
            || snapshot.page_rects.is_empty()
            || !snapshot.scroll_offset.x.is_finite()
            || !snapshot.scroll_offset.y.is_finite()
            || snapshot.scroll_offset.x < 0.0
            || snapshot.scroll_offset.y < 0.0
            || snapshot
                .page_rects
                .iter()
                .any(|rect| !valid_geometry_rect(*rect))
        {
            return None;
        }

        let viewport_center = snapshot.clip.center();
        let content_point = viewport_center + snapshot.scroll_offset;
        // egui 0.29 lays out ScrollArea children from `inner_rect.min - offset`,
        // so allocated rectangles move with the viewport. Normalize them into
        // content coordinates before comparing with the offset-adjusted center.
        let (page_index, page_rect) = snapshot
            .page_rects
            .iter()
            .enumerate()
            .map(|(index, rect)| (index, rect.translate(snapshot.scroll_offset)))
            .find(|(_, rect)| rect.contains(content_point))
            .or_else(|| {
                snapshot
                    .page_rects
                    .iter()
                    .enumerate()
                    .map(|(index, rect)| (index, rect.translate(snapshot.scroll_offset)))
                    .min_by(|(_, left), (_, right)| {
                        distance_squared_to_rect(content_point, *left)
                            .total_cmp(&distance_squared_to_rect(content_point, *right))
                    })
            })?;

        let bounded_point = Pos2::new(
            content_point.x.clamp(page_rect.left(), page_rect.right()),
            content_point.y.clamp(page_rect.top(), page_rect.bottom()),
        );
        Some(ViewportCenterAnchor {
            page_index,
            page_local_document_point: (bounded_point - page_rect.min) / scale,
            outside_page_offset_points: (content_point - bounded_point),
            viewport_center,
        })
    }

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

    // Snapshot publication keeps geometry fields explicit and avoids a wrapper
    // allocation on each editor frame.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn publish(
        &mut self,
        frame: u64,
        layout_revision: u64,
        viewport_rect: Rect,
        scale: f32,
        page_rects: Vec<Rect>,
        clip: Rect,
        scroll_offset: Vec2,
    ) -> bool {
        let next = GeometrySnapshot {
            published_frame: frame,
            layout_revision,
            viewport_rect,
            scale,
            page_rects,
            clip,
            scroll_offset,
        };
        let changed = self.published.as_ref().is_none_or(|old| {
            old.layout_revision != next.layout_revision
                || old.viewport_rect != next.viewport_rect
                || old.scale != next.scale
                || old.page_rects != next.page_rects
                || old.clip != next.clip
                || old.scroll_offset != next.scroll_offset
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

fn valid_geometry_rect(rect: Rect) -> bool {
    rect.min.x.is_finite()
        && rect.min.y.is_finite()
        && rect.max.x.is_finite()
        && rect.max.y.is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
}

fn distance_squared_to_rect(point: Pos2, rect: Rect) -> f32 {
    let dx = if point.x < rect.left() {
        rect.left() - point.x
    } else if point.x > rect.right() {
        point.x - rect.right()
    } else {
        0.0
    };
    let dy = if point.y < rect.top() {
        rect.top() - point.y
    } else if point.y > rect.bottom() {
        point.y - rect.bottom()
    } else {
        0.0
    };
    dx * dx + dy * dy
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CenterProjection {
    pub(super) operation: String,
    pub(super) error: Option<String>,
}

pub(super) fn project_center(operation: &str, error: Option<&str>) -> CenterProjection {
    CenterProjection {
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
    pub(super) performance_summary: &'a str,
}

impl DiagnosticProjection<'_> {
    pub(super) fn safe_summary(self) -> String {
        let summary = format!(
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
        );
        if self.performance_summary.is_empty() {
            summary
        } else {
            format!("{summary} {}", self.performance_summary)
        }
    }
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
    let mut zoom_changed = false;
    let mut appearance_changed = false;
    let mut timer_rect = None;
    let mut focus_rect = None;
    let mut timer_open = timer_open;
    let mut timer_just_opened = timer_just_opened;
    let mut focus_activated = focus_activated;
    let message = status_message(center);
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
                        status_rect,
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
