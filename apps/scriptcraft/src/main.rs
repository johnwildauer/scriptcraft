#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use eframe::egui::{self, Color32, FontFamily, FontId, Key, RichText, Sense, Stroke, Vec2};
use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::exports;
use scriptcraft_core::layout::{Caret, LayoutCache, ScreenplayLayout};
use scriptcraft_core::persistence::{self, FileSystem, StdFileSystem};
use scriptcraft_core::{Project, Role};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const EDITOR_SURFACE_ID: &str = "page-editor-surface";
const CARET_WIDTH: f32 = 1.5;
const CURRENT_DEFAULT_PAGE_WIDTH: f32 = 360.0;
const DEFAULT_PRESENTATION_SCALE_MULTIPLIER: f32 = 1.6;
const PAGE_HORIZONTAL_PADDING: f32 = 16.0;
const MINIMUM_PAGE_WIDTH: f32 = 240.0;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const TOOLBAR_NORMAL_ITEM_WIDTH: f32 = 88.0;
const TOOLBAR_OVERFLOW_WIDTH: f32 = 32.0;
const TOOLBAR_ROW_PADDING: f32 = 4.0;
const TOOLBAR_ITEM_GAP: f32 = 4.0;

#[derive(Debug, Default)]
struct NativeInputDiagnostics {
    pointer_activations: u32,
    received_events: u32,
    dispatched_frames: u32,
    last_event_category: &'static str,
    focused: bool,
    focus_lock_installed: bool,
    dispatch_enabled: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct PerformanceDiagnostics {
    frames: u64,
    events: u64,
    layout_builds: u64,
    layout_cache_hits: u64,
    painted_pages: u64,
    repaint_requests: u64,
    frame_nanos: u128,
    event_nanos: u128,
    layout_nanos: u128,
    paint_nanos: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevealIntent {
    None,
    Conditional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    New,
    Open,
    Save,
    SaveAs,
    ExportPdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarItemKind {
    Action(ToolbarAction),
    Elements,
    Space,
    FlexibleSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolbarItem {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    kind: ToolbarItemKind,
}

impl ToolbarItem {
    const fn action(
        id: &'static str,
        label: &'static str,
        icon: &'static str,
        action: ToolbarAction,
    ) -> Self {
        Self {
            id,
            label,
            icon,
            kind: ToolbarItemKind::Action(action),
        }
    }

    const fn elements() -> Self {
        Self {
            id: "elements",
            label: "Elements",
            icon: "▾",
            kind: ToolbarItemKind::Elements,
        }
    }

    const fn flexible_space(id: &'static str) -> Self {
        Self {
            id,
            label: "Flexible Space",
            icon: "",
            kind: ToolbarItemKind::FlexibleSpace,
        }
    }

    pub const fn space(id: &'static str) -> Self {
        Self {
            id,
            label: "Space",
            icon: "",
            kind: ToolbarItemKind::Space,
        }
    }
}

fn toolbar_action_text(item: ToolbarItem) -> String {
    format!("{}\n{}", item.icon, item.label)
}

const SUPPORTED_SCREENPLAY_ROLES: [(Role, &str); 7] = [
    (Role::SceneHeading, "Scene Heading"),
    (Role::Action, "Action"),
    (Role::Character, "Character"),
    (Role::Dialogue, "Dialogue"),
    (Role::Parenthetical, "Parenthetical"),
    (Role::Transition, "Transition"),
    (Role::Note, "Note"),
];

fn screenplay_role_label(role: Role) -> &'static str {
    SUPPORTED_SCREENPLAY_ROLES
        .iter()
        .find_map(|(supported, label)| (*supported == role).then_some(*label))
        .unwrap_or("Unknown")
}

fn default_toolbar_items() -> [ToolbarItem; 8] {
    [
        ToolbarItem::action("new", "New", "＋", ToolbarAction::New),
        ToolbarItem::action("open", "Open", "↥", ToolbarAction::Open),
        ToolbarItem::action("save", "Save", "▣", ToolbarAction::Save),
        ToolbarItem::action("save-as", "Save As", "⇩", ToolbarAction::SaveAs),
        ToolbarItem::flexible_space("flexible-space-before-elements"),
        ToolbarItem::elements(),
        ToolbarItem::flexible_space("flexible-space-after-elements"),
        ToolbarItem::action("export-pdf", "Export PDF", "⇱", ToolbarAction::ExportPdf),
    ]
}

#[derive(Debug, Clone, PartialEq)]
struct ToolbarAllocation {
    visible: Vec<(ToolbarItem, f32)>,
    overflow: Vec<ToolbarItem>,
    overflow_width: f32,
}

fn is_flexible_toolbar_space(item: ToolbarItem) -> bool {
    matches!(item.kind, ToolbarItemKind::FlexibleSpace)
}

fn toolbar_content_width(widths: &[(ToolbarItem, f32)], overflow_width: f32) -> f32 {
    let child_count = widths.len() as f32 + f32::from(overflow_width > 0.0);
    widths.iter().map(|(_, width)| width).sum::<f32>()
        + TOOLBAR_ROW_PADDING * 2.0
        + (child_count - 1.0).max(0.0) * TOOLBAR_ITEM_GAP
        + overflow_width
}

/// Allocate the transient toolbar without measuring or mutating project state.
/// Normal items, including `Space`, have one fixed slot. Flexible spaces share
/// only the remaining width once every normal item fits.
fn allocate_toolbar(items: &[ToolbarItem], available_width: f32) -> ToolbarAllocation {
    let normal_count = items
        .iter()
        .filter(|item| !is_flexible_toolbar_space(**item))
        .count();
    let fixed_width = normal_count as f32 * TOOLBAR_NORMAL_ITEM_WIDTH;
    let full_fixed_width = fixed_width
        + TOOLBAR_ROW_PADDING * 2.0
        + (items.len().saturating_sub(1) as f32) * TOOLBAR_ITEM_GAP;
    let flexible_count = items
        .iter()
        .filter(|item| matches!(item.kind, ToolbarItemKind::FlexibleSpace))
        .count();

    if full_fixed_width <= available_width {
        let flexible_width = if flexible_count == 0 {
            0.0
        } else {
            ((available_width - full_fixed_width) / flexible_count as f32).max(0.0)
        };
        return ToolbarAllocation {
            visible: items
                .iter()
                .map(|item| {
                    let width = if is_flexible_toolbar_space(*item) {
                        flexible_width
                    } else {
                        TOOLBAR_NORMAL_ITEM_WIDTH
                    };
                    (*item, width)
                })
                .collect(),
            overflow: Vec::new(),
            overflow_width: 0.0,
        };
    }

    let mut visible = Vec::new();
    let mut overflow = Vec::new();
    let mut fitting = true;
    for item in items.iter().copied() {
        if is_flexible_toolbar_space(item) {
            continue;
        }
        if matches!(item.kind, ToolbarItemKind::Space) {
            let mut candidate = visible.clone();
            candidate.push((item, TOOLBAR_NORMAL_ITEM_WIDTH));
            if fitting
                && toolbar_content_width(&candidate, TOOLBAR_OVERFLOW_WIDTH) <= available_width
            {
                visible.push((item, TOOLBAR_NORMAL_ITEM_WIDTH));
            }
            continue;
        }
        let mut candidate = visible.clone();
        candidate.push((item, TOOLBAR_NORMAL_ITEM_WIDTH));
        if fitting && toolbar_content_width(&candidate, TOOLBAR_OVERFLOW_WIDTH) <= available_width {
            visible.push((item, TOOLBAR_NORMAL_ITEM_WIDTH));
        } else {
            fitting = false;
            overflow.push(item);
        }
    }

    ToolbarAllocation {
        visible,
        overflow,
        overflow_width: TOOLBAR_OVERFLOW_WIDTH,
    }
}

struct ScriptCraftApp {
    session: EditingSession,
    surface_focused: bool,
    drag_anchor: Option<Position>,
    ime_preedit: String,
    ime_active: bool,
    rendered_revision: Option<u64>,
    status: String,
    error: Option<String>,
    native_path: Option<PathBuf>,
    native_input_diagnostics: NativeInputDiagnostics,
    editor_surface_id: Option<egui::Id>,
    editor_surface_rect: Option<egui::Rect>,
    reveal_intent: RevealIntent,
    preferred_vertical_x: Option<u32>,
    caret_blink_focus_generation: u64,
    caret_blink_reset_time: f64,
    layout_cache: LayoutCache,
    performance: PerformanceDiagnostics,
}

impl ScriptCraftApp {
    fn new() -> Result<Self, String> {
        let project = Project::new("demo-project", "Untitled Screenplay", "Writer")
            .map_err(|error| format!("new project failed: {error:?}"))?;
        let session =
            EditingSession::new(project).map_err(|error| format!("session failed: {error:?}"))?;
        Ok(Self {
            session,
            surface_focused: false,
            drag_anchor: None,
            ime_preedit: String::new(),
            ime_active: false,
            rendered_revision: None,
            status: "In-memory demo project".into(),
            error: None,
            native_path: None,
            native_input_diagnostics: NativeInputDiagnostics::default(),
            editor_surface_id: None,
            editor_surface_rect: None,
            reveal_intent: RevealIntent::None,
            preferred_vertical_x: None,
            caret_blink_focus_generation: 0,
            caret_blink_reset_time: 0.0,
            layout_cache: LayoutCache::default(),
            performance: PerformanceDiagnostics::default(),
        })
    }

    fn request_repaint(&mut self, ctx: &egui::Context) {
        self.performance.repaint_requests = self.performance.repaint_requests.saturating_add(1);
        ctx.request_repaint();
    }

    fn layout_for_render(
        &mut self,
    ) -> Result<std::sync::Arc<ScreenplayLayout>, scriptcraft_core::layout::LayoutError> {
        let revision = self.session.revision();
        let started = Instant::now();
        let (layout, hit) = self
            .layout_cache
            .get_or_build(revision, self.session.project())?;
        if hit {
            self.performance.layout_cache_hits =
                self.performance.layout_cache_hits.saturating_add(1);
        } else {
            self.performance.layout_builds = self.performance.layout_builds.saturating_add(1);
            self.performance.layout_nanos = self
                .performance
                .layout_nanos
                .saturating_add(started.elapsed().as_nanos());
        }
        Ok(layout)
    }

    fn save_native(&mut self) {
        let Some(path) = self.native_path.clone() else {
            self.save_as_native();
            return;
        };
        self.save_native_at(&path);
    }

    fn save_native_at(&mut self, path: &std::path::Path) {
        match persistence::atomic_save(&StdFileSystem, path, self.session.project()) {
            Ok(()) => {
                self.session.mark_saved();
                self.native_path = Some(path.to_owned());
                self.status = format!("Saved {}", path.display());
                self.clear_error();
            }
            Err(error) => self.show_error(error),
        }
    }

    fn open_native(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("ScriptCraft project", &["json"])
            .pick_file()
        {
            self.open_native_at(&path);
        }
    }

    fn open_native_at(&mut self, path: &std::path::Path) {
        if self.session.dirty() {
            self.error = None;
            self.status = "Open cancelled: save the unsaved project first".into();
            return;
        }
        let result = StdFileSystem
            .read(path)
            .and_then(|bytes| persistence::deserialize(&bytes))
            .map_err(|error| format!("open failed: {error:?}"));
        match result {
            Ok(project) => match self.session.replace_with_saved(project) {
                Ok(()) => {
                    self.native_path = Some(path.to_owned());
                    self.status = format!("Opened {}", path.display());
                    self.clear_error();
                }
                Err(error) => self.show_error(error),
            },
            Err(error) => self.show_error(error),
        }
    }

    fn save_as_native(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("ScriptCraft project", &["json"])
            .set_file_name("Untitled Screenplay.json")
            .save_file()
        {
            self.save_native_at(&path);
        }
    }

    fn export_pdf(&mut self) {
        match exports::export_pdf(self.session.project()) {
            Ok(bytes) => match StdFileSystem.write(&PathBuf::from("scriptcraft.pdf"), &bytes) {
                Ok(()) => {
                    self.status = "Exported fixed-layout PDF to scriptcraft.pdf".into();
                    self.clear_error();
                }
                Err(error) => self.show_error(error),
            },
            Err(error) => self.show_error(error),
        }
    }

    fn dispatch_toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::New => {
                match Project::new("demo-project", "Untitled Screenplay", "Writer")
                    .map_err(|error| format!("new project failed: {error:?}"))
                    .and_then(|project| {
                        EditingSession::new(project)
                            .map_err(|error| format!("session failed: {error:?}"))
                    }) {
                    Ok(session) => {
                        self.session = session;
                        self.surface_focused = false;
                        self.native_path = None;
                        self.status = "New in-memory project".into();
                        self.clear_error();
                    }
                    Err(error) => self.show_error(error),
                }
            }
            ToolbarAction::Open => self.open_native(),
            ToolbarAction::Save => self.save_native(),
            ToolbarAction::SaveAs => self.save_as_native(),
            ToolbarAction::ExportPdf => self.export_pdf(),
        }
    }

    fn toolbar_action_button(&mut self, ui: &mut egui::Ui, item: ToolbarItem, width: f32) {
        let button = egui::Button::new(RichText::new(toolbar_action_text(item)))
            .frame(false)
            .sense(Sense::click());
        let response = ui
            .add_sized(Vec2::new(width, ui.spacing().interact_size.y * 2.0), button)
            .on_hover_text(item.label)
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            if let ToolbarItemKind::Action(action) = item.kind {
                self.dispatch_toolbar_action(action);
            }
        }
    }

    fn focused_role(&self) -> Option<Role> {
        self.session
            .project()
            .elements
            .get(self.session.selection().focus.element)
            .map(|element| element.role)
    }

    fn change_role_from_toolbar(&mut self, role: Role) {
        let result = self.session.change_role(role);
        self.mutate(result, "Changed focused element role", RevealIntent::None);
    }

    fn elements_toolbar_item(&mut self, ui: &mut egui::Ui, item: ToolbarItem, width: f32) {
        let selected_role = self.focused_role();
        let mut next_role = selected_role.unwrap_or(Role::Action);
        ui.allocate_ui_with_layout(
            Vec2::new(width, ui.spacing().interact_size.y * 2.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                egui::ComboBox::from_id_salt(item.id)
                    .selected_text(screenplay_role_label(next_role))
                    .show_ui(ui, |ui| {
                        for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
                            ui.selectable_value(&mut next_role, role, label);
                        }
                    });
                ui.label(item.label);
            },
        );
        if selected_role != Some(next_role) {
            self.change_role_from_toolbar(next_role);
        }
    }

    fn toolbar_overflow_menu(&mut self, ui: &mut egui::Ui, items: &[ToolbarItem]) {
        let mut chosen_action = None;
        let mut chosen_role = None;
        ui.menu_button(">>", |ui| {
            for item in items.iter().copied() {
                match item.kind {
                    ToolbarItemKind::Action(action) => {
                        if ui.button(item.label).clicked() {
                            chosen_action = Some(action);
                            ui.close_menu();
                        }
                    }
                    ToolbarItemKind::Elements => {
                        let selected_role = self.focused_role();
                        ui.menu_button(item.label, |ui| {
                            for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
                                if ui
                                    .selectable_label(selected_role == Some(role), label)
                                    .clicked()
                                {
                                    chosen_role = Some(role);
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                    ToolbarItemKind::Space | ToolbarItemKind::FlexibleSpace => {}
                }
            }
        });
        if let Some(action) = chosen_action {
            self.dispatch_toolbar_action(action);
        }
        if let Some(role) = chosen_role {
            self.change_role_from_toolbar(role);
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let allocation = allocate_toolbar(&default_toolbar_items(), ui.available_width());
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.horizontal(|ui| {
                ui.add_space(TOOLBAR_ROW_PADDING);
                let mut first = true;
                for (item, width) in allocation.visible.iter().copied() {
                    if !first {
                        ui.add_space(TOOLBAR_ITEM_GAP);
                    }
                    first = false;
                    ui.push_id(item.id, |ui| match item.kind {
                        ToolbarItemKind::Action(_) => self.toolbar_action_button(ui, item, width),
                        ToolbarItemKind::Elements => self.elements_toolbar_item(ui, item, width),
                        ToolbarItemKind::Space | ToolbarItemKind::FlexibleSpace => {
                            ui.add_space(width);
                        }
                    });
                }
                if !allocation.overflow.is_empty() {
                    if !first {
                        ui.add_space(TOOLBAR_ITEM_GAP);
                    }
                    ui.allocate_ui(
                        Vec2::new(allocation.overflow_width, ui.spacing().interact_size.y),
                        |ui| self.toolbar_overflow_menu(ui, &allocation.overflow),
                    );
                }
                ui.add_space(TOOLBAR_ROW_PADDING);
            });
        });
    }

    fn show_error<E: std::fmt::Debug>(&mut self, error: E) {
        let detail = format!("{error:?}");
        let health = self.session_health();
        self.error = Some(detail.clone());
        self.status = format!("Operation failed; document unchanged ({detail}); {health}");
    }

    fn session_health(&self) -> String {
        let validation = match self.session.validate() {
            Ok(()) => "session valid".to_owned(),
            Err(error) => format!("session invalid: {error:?}"),
        };
        let json = match self.session.project().to_json() {
            Ok(serialized) => format!("live JSON valid ({} bytes)", serialized.len()),
            Err(error) => format!("live JSON invalid: {error:?}"),
        };
        format!("{validation}; {json}")
    }

    fn clear_error(&mut self) {
        self.error = None;
    }

    fn record_native_event(&mut self, category: &'static str) {
        self.native_input_diagnostics.received_events = self
            .native_input_diagnostics
            .received_events
            .saturating_add(1);
        self.native_input_diagnostics.last_event_category = category;
    }

    fn mutate(
        &mut self,
        result: Result<(), EditError>,
        success: &str,
        reveal_intent: RevealIntent,
    ) {
        match result {
            Ok(()) => {
                self.preferred_vertical_x = None;
                self.status = success.into();
                self.clear_error();
                self.rendered_revision = None;
                self.reveal_intent = reveal_intent;
            }
            Err(error) => self.show_error(error),
        }
    }

    fn caret_from_pointer(
        &self,
        layout: &ScreenplayLayout,
        page: usize,
        page_rect: egui::Rect,
        scale: f32,
        pointer: egui::Pos2,
    ) -> Option<Position> {
        let local_x = ((pointer.x - page_rect.left()) / scale).round() as i64;
        let local_y = ((pointer.y - page_rect.top()) / scale).round() as i64;
        layout
            .nearest_line_hit_test(page, local_x, local_y)
            .map(|caret| Position {
                element: caret.element,
                offset: caret.offset,
            })
    }

    fn pointer_body_page(
        layout: &ScreenplayLayout,
        page_rects: &[egui::Rect],
        pointer: egui::Pos2,
    ) -> Option<usize> {
        if page_rects
            .iter()
            .enumerate()
            .any(|(index, rect)| rect.contains(pointer) && layout.pages[index].title_page)
        {
            // Title-page ownership is explicit: a title click never becomes a
            // body selection, even when a body line is globally nearest.
            return None;
        }
        page_rects
            .iter()
            .enumerate()
            .filter(|(index, _)| !layout.pages[*index].title_page)
            .min_by_key(|(index, rect)| {
                let distance = if pointer.y < rect.top() {
                    rect.top() - pointer.y
                } else if pointer.y > rect.bottom() {
                    pointer.y - rect.bottom()
                } else {
                    0.0
                };
                (distance.to_bits(), *index)
            })
            .map(|(index, _)| index)
    }

    fn apply_pointer_selection(
        &mut self,
        position: Position,
        drag_started: bool,
        dragged: bool,
    ) -> Result<(), EditError> {
        let selection = if drag_started {
            self.drag_anchor = Some(position);
            Selection::caret(position)
        } else if dragged {
            Selection {
                anchor: self.drag_anchor.unwrap_or(self.session.selection().anchor),
                focus: position,
            }
        } else {
            self.drag_anchor = None;
            Selection::caret(position)
        };
        let result = self.session.set_selection(selection);
        if result.is_ok() {
            self.preferred_vertical_x = None;
            self.rendered_revision = None;
            // Pointer selection is authoritative: it must not inherit a reveal
            // request queued by an earlier edit or navigation command.
            self.reveal_intent = RevealIntent::None;
        }
        result
    }

    fn selection_color(
        &self,
        line_start: usize,
        line_end: usize,
        element: usize,
    ) -> Option<Color32> {
        let selection = self.session.selection();
        let (start, end) = ordered_selection(selection);
        if start.element > element || end.element < element {
            return None;
        }
        let from = if start.element == element {
            start.offset
        } else {
            line_start
        };
        let to = if end.element == element {
            end.offset
        } else {
            line_end
        };
        (from < line_end && to > line_start).then_some(Color32::from_rgb(170, 205, 245))
    }

    fn paint_page(
        &mut self,
        ui: &mut egui::Ui,
        layout: &ScreenplayLayout,
        page_index: usize,
        page_width: f32,
        scale: f32,
        editor_id: egui::Id,
    ) -> egui::Rect {
        let paint_started = Instant::now();
        self.performance.painted_pages = self.performance.painted_pages.saturating_add(1);
        let page_height = layout.profile.paper_height as f32 * scale;
        let page_rect = ui.allocate_space(Vec2::new(page_width, page_height)).1;
        let painter = ui.painter_at(page_rect);
        let char_width = layout.char_width as f32 * scale;
        painter.rect_filled(page_rect, 2.0, Color32::from_gray(246));
        painter.rect_stroke(page_rect, 2.0, Stroke::new(1.0_f32, Color32::GRAY));

        for line in &layout.pages[page_index].lines {
            let x = page_rect.left() + line.rect.x as f32 * scale;
            let y = page_rect.top() + line.rect.y as f32 * scale;
            if let Some(color) =
                self.selection_color(line.source.start, line.source.end, line.element)
            {
                let (start, end) = ordered_selection(self.session.selection());
                let from = if start.element == line.element {
                    start.offset.max(line.source.start)
                } else {
                    line.source.start
                };
                let to = if end.element == line.element {
                    end.offset.min(line.source.end)
                } else {
                    line.source.end
                };
                let left =
                    x + line.text[..from - line.source.start].chars().count() as f32 * char_width;
                let width = line.text[from - line.source.start..to - line.source.start]
                    .chars()
                    .count() as f32
                    * char_width;
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(left, y),
                        Vec2::new(width.max(1.0), layout.line_height as f32 * scale),
                    ),
                    0.0,
                    color,
                );
            }
            paint_fixed_metric_text(
                &painter,
                egui::pos2(x, y),
                &line.text,
                renderer_font(layout, scale),
                Color32::DARK_GRAY,
                char_width,
            );
        }

        let selection = self.session.selection();
        if selection.anchor == selection.focus && !layout.pages[page_index].title_page {
            let caret = Caret {
                element: selection.focus.element,
                offset: selection.focus.offset,
            };
            let blink_id = editor_id.with((
                "caret-blink",
                self.session.revision(),
                caret.element,
                caret.offset,
                self.caret_blink_focus_generation,
            ));
            if let Ok(rect) = layout.caret_rect(caret) {
                let blink_active = self.surface_focused;
                if caret_blink_visible(
                    ui.ctx(),
                    blink_id,
                    blink_active,
                    true,
                    self.caret_blink_reset_time,
                ) {
                    painter.rect_filled(
                        caret_bar_rect(page_rect, rect, scale),
                        0.0,
                        Color32::from_rgb(20, 80, 150),
                    );
                }
                if self.surface_focused {
                    self.performance.repaint_requests =
                        self.performance.repaint_requests.saturating_add(1);
                }
                if !self.ime_preedit.is_empty() {
                    let x = page_rect.left() + rect.x as f32 * scale + char_width;
                    let y = page_rect.top() + rect.y as f32 * scale;
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        &self.ime_preedit,
                        renderer_font(layout, scale),
                        Color32::from_rgb(80, 80, 160),
                    );
                }
            }
        }

        self.performance.paint_nanos = self
            .performance
            .paint_nanos
            .saturating_add(paint_started.elapsed().as_nanos());
        page_rect
    }

    fn live_document_surface(&mut self, ui: &mut egui::Ui) {
        let layout = match self.layout_for_render() {
            Ok(layout) => layout,
            Err(error) => {
                self.show_error(error.clone());
                ui.colored_label(Color32::RED, format!("Layout error: {error:?}"));
                return;
            }
        };
        self.rendered_revision = Some(self.session.revision());
        let available = ui.available_size();
        let (page_width, horizontal_padding, scale) =
            presentation_geometry(available.x, layout.profile.paper_width as f32);
        let editor_id = ui.id().with(EDITOR_SURFACE_ID);
        let mut page_rects = Vec::with_capacity(layout.pages.len());
        let reveal_intent = self.reveal_intent;
        let scroll_output = egui::ScrollArea::vertical()
            .id_salt("page-editor-scroll")
            .show(ui, |ui| {
                for page_index in 0..layout.pages.len() {
                    let mut page_rect = None;
                    ui.horizontal(|ui| {
                        ui.add_space(horizontal_padding);
                        page_rect = Some(
                            self.paint_page(ui, &layout, page_index, page_width, scale, editor_id),
                        );
                        ui.add_space(horizontal_padding);
                    });
                    page_rects.push(page_rect.unwrap_or_else(|| {
                        egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::ZERO)
                    }));
                    ui.add_space(12.0);
                }
                if reveal_intent == RevealIntent::Conditional {
                    let focus = self.session.selection().focus;
                    if let Some(page_index) = layout.pages.iter().position(|page| {
                        page.lines.iter().any(|line| {
                            line.element == focus.element
                                && focus.offset >= line.source.start
                                && focus.offset <= line.source.end
                        })
                    }) {
                        if let Ok(caret) = layout.caret_rect(Caret {
                            element: focus.element,
                            offset: focus.offset,
                        }) {
                            let page_rect = page_rects[page_index];
                            let caret_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    page_rect.left() + caret.x as f32 * scale,
                                    page_rect.top() + caret.y as f32 * scale,
                                ),
                                Vec2::new(CARET_WIDTH, caret.height as f32 * scale),
                            );
                            let visible_clip = ui.clip_rect();
                            if caret_needs_reveal(caret_rect, visible_clip) {
                                // `None` asks egui for nearest/minimal scrolling;
                                // centering would override the writer's viewport.
                                ui.scroll_to_rect(caret_rect, None);
                            }
                        }
                    }
                }
            });
        self.reveal_intent = RevealIntent::None;

        // This is the only interactive response in the painted editor. Pages remain
        // source-mapped geometry, not independent focusable widgets.
        self.editor_surface_id = Some(editor_id);
        let response = ui.interact(scroll_output.inner_rect, editor_id, Sense::click_and_drag());
        self.editor_surface_rect = Some(response.rect);
        if let Some(pointer) = response.interact_pointer_pos() {
            let pointer_active = response.clicked()
                || response.drag_started()
                || response.dragged()
                || (response.hovered() && ui.input(|input| input.pointer.primary_down()));
            if pointer_active {
                response.request_focus();
                self.native_input_diagnostics.pointer_activations = self
                    .native_input_diagnostics
                    .pointer_activations
                    .saturating_add(1);
                if let Some(page_index) = Self::pointer_body_page(&layout, &page_rects, pointer) {
                    if let Some(position) = self.caret_from_pointer(
                        &layout,
                        page_index,
                        page_rects[page_index],
                        scale,
                        pointer,
                    ) {
                        if let Err(error) = self.apply_pointer_selection(
                            position,
                            response.drag_started(),
                            response.dragged(),
                        ) {
                            self.show_error(error);
                        } else {
                            self.clear_error();
                        }
                    }
                }
            }
        }
        let surface_focused = response.has_focus();
        if surface_focused != self.surface_focused {
            self.caret_blink_focus_generation = self.caret_blink_focus_generation.wrapping_add(1);
            if surface_focused {
                self.caret_blink_reset_time = ui.ctx().input(|input| input.time.max(0.0));
            }
            // The next frame must use the new focus-generation identity. This is
            // a transition-only repaint, not an unfocused blink loop.
            self.request_repaint(ui.ctx());
        }
        self.surface_focused = surface_focused;
        self.native_input_diagnostics.focused = self.surface_focused;
        if self.surface_focused {
            ui.ctx().memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    editor_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
            self.native_input_diagnostics.focus_lock_installed = true;
        } else {
            self.native_input_diagnostics.focus_lock_installed = false;
        }
    }

    fn dispatch_surface_events(&mut self, events: &[egui::Event], modifiers: egui::Modifiers) {
        let started = Instant::now();
        self.performance.events = self.performance.events.saturating_add(events.len() as u64);
        for (index, event) in events.iter().enumerate() {
            self.record_native_event(match event {
                egui::Event::Text(_) => "text",
                egui::Event::Paste(_) => "paste",
                egui::Event::Ime(egui::ImeEvent::Preedit(_)) => "ime-preedit",
                egui::Event::Ime(egui::ImeEvent::Commit(_)) => "ime-commit",
                egui::Event::Ime(egui::ImeEvent::Enabled) => "ime-enabled",
                egui::Event::Ime(egui::ImeEvent::Disabled) => "ime-disabled",
                egui::Event::Key { pressed: true, .. } => "key-pressed",
                egui::Event::Key { pressed: false, .. } => "key-released",
                _ => "other",
            });
            let result = match event {
                egui::Event::Text(text)
                    if !text.is_empty()
                        && !self.ime_active
                        && !events[index + 1..].iter().any(|next| {
                            matches!(next, egui::Event::Ime(egui::ImeEvent::Commit(commit)) if commit == text)
                        })
                        && !modifiers.command
                        && !(modifiers.ctrl && !cfg!(target_os = "macos"))
                        && text.chars().all(|character| !character.is_control()) =>
                {
                    Some(self.session.insert(text))
                }
                egui::Event::Paste(text) if !text.is_empty() => {
                    Some(self.session.insert_paste(text))
                }
                egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                    self.ime_active = true;
                    self.ime_preedit.clone_from(text);
                    self.rendered_revision = None;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    (!text.is_empty()).then(|| self.session.insert(text))
                }
                egui::Event::Ime(egui::ImeEvent::Enabled) => {
                    self.ime_active = true;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Disabled) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    None
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => self.key_event(*key, *modifiers),
                _ => continue,
            };
            if let Some(result) = result {
                self.mutate(
                    result,
                    "Edited through EditingSession",
                    RevealIntent::Conditional,
                );
            }
        }
        self.performance.event_nanos = self
            .performance
            .event_nanos
            .saturating_add(started.elapsed().as_nanos());
    }

    fn key_event(&mut self, key: Key, modifiers: egui::Modifiers) -> Option<Result<(), EditError>> {
        if modifiers.command || (modifiers.ctrl && !cfg!(target_os = "macos")) {
            return match key {
                Key::Z if modifiers.shift => Some(self.session.redo()),
                Key::Z => Some(self.session.undo()),
                Key::Y => Some(self.session.redo()),
                Key::A => Some(self.session.set_selection(Selection {
                    anchor: Position {
                        element: 0,
                        offset: 0,
                    },
                    focus: self.document_end(),
                })),
                _ => None,
            };
        }
        match key {
            Key::Backspace => Some(self.session.backspace()),
            Key::Delete => Some(self.session.delete()),
            Key::Enter => Some(self.session.enter()),
            Key::Tab => Some(self.session.tab()),
            Key::ArrowLeft => Some(self.move_horizontal(false, modifiers.shift)),
            Key::ArrowRight => Some(self.move_horizontal(true, modifiers.shift)),
            Key::ArrowUp => Some(self.move_vertical(false, modifiers.shift)),
            Key::ArrowDown => Some(self.move_vertical(true, modifiers.shift)),
            Key::Home => Some(self.move_to_edge(false, modifiers.shift)),
            Key::End => Some(self.move_to_edge(true, modifiers.shift)),
            _ => None,
        }
    }

    fn handle_surface_input(&mut self, ctx: &egui::Context) {
        self.native_input_diagnostics.dispatch_enabled = self.surface_focused;
        if !self.surface_focused {
            return;
        }
        if ctx.input(|input| input.raw_scroll_delta != Vec2::ZERO) {
            self.reveal_intent = RevealIntent::None;
        }
        self.native_input_diagnostics.dispatched_frames = self
            .native_input_diagnostics
            .dispatched_frames
            .saturating_add(1);
        let events = ctx.input(|input| input.events.clone());
        let modifiers = ctx.input(|input| input.modifiers);
        self.dispatch_surface_events(&events, modifiers);
        if !events.is_empty() || ctx.input(|input| input.raw_scroll_delta != Vec2::ZERO) {
            self.request_repaint(ctx);
        }
    }

    fn document_end(&self) -> Position {
        let element = self.session.project().elements.len().saturating_sub(1);
        Position {
            element,
            offset: self.session.project().elements[element].text.len(),
        }
    }

    fn move_horizontal(&mut self, right: bool, extend: bool) -> Result<(), EditError> {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        if selection.anchor != selection.focus && !extend {
            let position = if right {
                ordered_selection(selection).1
            } else {
                ordered_selection(selection).0
            };
            return self.session.set_selection(Selection::caret(position));
        }
        let position = selection.focus;
        let item = &self.session.project().elements[position.element];
        let next = if right {
            if position.offset < item.text.len() {
                Position {
                    offset: item.text[position.offset..]
                        .chars()
                        .next()
                        .map_or(position.offset, |c| position.offset + c.len_utf8()),
                    ..position
                }
            } else if position.element + 1 < self.session.project().elements.len() {
                Position {
                    element: position.element + 1,
                    offset: 0,
                }
            } else {
                position
            }
        } else if position.offset > 0 {
            Position {
                offset: item.text[..position.offset]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(at, _)| at),
                ..position
            }
        } else if position.element > 0 {
            let previous = &self.session.project().elements[position.element - 1];
            Position {
                element: position.element - 1,
                offset: previous.text.len(),
            }
        } else {
            position
        };
        self.session.set_selection(if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    fn move_vertical(&mut self, down: bool, extend: bool) -> Result<(), EditError> {
        let selection = self.session.selection();
        let current = selection.focus;
        let layout = self
            .layout_for_render()
            .map_err(|_| EditError::InvalidElement)?;
        let lines: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .collect();
        let current_index = lines.iter().position(|line| {
            line.element == current.element
                && current.offset >= line.source.start
                && current.offset <= line.source.end
        });
        let Some(current_index) = current_index else {
            self.preferred_vertical_x = None;
            return self.session.set_selection(selection);
        };
        let target_index = if down {
            current_index
                .checked_add(1)
                .filter(|index| *index < lines.len())
        } else {
            current_index.checked_sub(1)
        };
        let current_line = lines[current_index];
        let Some(target_index) = target_index else {
            self.preferred_vertical_x = None;
            let next = Position {
                element: current_line.element,
                offset: if down {
                    current_line.source.end
                } else {
                    current_line.source.start
                },
            };
            return self.session.set_selection(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        };
        let goal = self.preferred_vertical_x.unwrap_or_else(|| {
            let columns = current_line.text[..current.offset - current_line.source.start]
                .chars()
                .count() as u32;
            current_line.rect.x + columns.saturating_mul(layout.char_width)
        });
        self.preferred_vertical_x = Some(goal);
        let target_line = lines[target_index];
        let columns = goal
            .saturating_sub(target_line.rect.x)
            .saturating_add(layout.char_width / 2)
            / layout.char_width.max(1);
        let offset_in_line = target_line
            .text
            .char_indices()
            .nth(columns as usize)
            .map_or(target_line.text.len(), |(offset, _)| offset);
        let next = Position {
            element: target_line.element,
            offset: target_line.source.start + offset_in_line,
        };
        self.session.set_selection(if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    fn move_to_edge(&mut self, end: bool, extend: bool) -> Result<(), EditError> {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        let position = if end {
            Position {
                element: selection.focus.element,
                offset: self.session.project().elements[selection.focus.element]
                    .text
                    .len(),
            }
        } else {
            Position {
                element: selection.focus.element,
                offset: 0,
            }
        };
        self.session.set_selection(if extend {
            Selection {
                anchor: selection.anchor,
                focus: position,
            }
        } else {
            Selection::caret(position)
        })
    }
}

fn presentation_geometry(available_width: f32, paper_width: f32) -> (f32, f32, f32) {
    let default_scale =
        CURRENT_DEFAULT_PAGE_WIDTH * DEFAULT_PRESENTATION_SCALE_MULTIPLIER / paper_width;
    let default_page_width = paper_width * default_scale;
    let usable_width = (available_width - PAGE_HORIZONTAL_PADDING * 2.0).max(0.0);
    let page_width = default_page_width.min(usable_width.max(MINIMUM_PAGE_WIDTH));
    let content_width = available_width.max(page_width);
    let horizontal_padding = ((content_width - page_width) / 2.0).max(0.0);
    (page_width, horizontal_padding, page_width / paper_width)
}

fn paint_fixed_metric_text(
    painter: &egui::Painter,
    origin: egui::Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    char_width: f32,
) {
    for (column, character) in text.chars().enumerate() {
        painter.text(
            egui::pos2(fixed_metric_x(origin.x, column, char_width), origin.y),
            egui::Align2::LEFT_TOP,
            character.to_string(),
            font.clone(),
            color,
        );
    }
}

fn renderer_font(layout: &ScreenplayLayout, scale: f32) -> FontId {
    FontId::new(
        (layout.profile.font_size as f32 * scale).max(5.0),
        // The persisted profile names the Courier-family target. egui's
        // built-in monospace family is the deterministic headless fallback;
        // viewport scale changes the display transform, not the base point size.
        FontFamily::Monospace,
    )
}

fn fixed_metric_x(origin: f32, column: usize, char_width: f32) -> f32 {
    origin + column as f32 * char_width
}

fn caret_blink_visible(
    ctx: &egui::Context,
    blink_id: egui::Id,
    focused: bool,
    collapsed: bool,
    reset_time: f64,
) -> bool {
    if !focused || !collapsed {
        return false;
    }

    // egui owns both the clock and animation entry. A zero-duration animation
    // makes each 500 ms phase switch discrete rather than introducing a fade.
    // The caller's identity includes revision and caret position, so changes
    // begin a distinct transient entry without model or thread state.
    let time = ctx.input(|input| input.time.max(0.0));
    let elapsed = (time - reset_time).max(0.0);
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (elapsed / interval).floor() as u64;
    let target_visible = phase.is_multiple_of(2);
    let phase_value = ctx.animate_bool_with_time(blink_id, target_visible, 0.0);
    ctx.request_repaint_after(caret_blink_repaint_delay(elapsed));
    phase_value > 0.5
}

fn caret_blink_repaint_delay(time: f64) -> Duration {
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (time.max(0.0) / interval).floor() as u64;
    Duration::from_secs_f64(((phase + 1) as f64 * interval - time.max(0.0)).max(0.001))
}

fn caret_bar_rect(
    page_rect: egui::Rect,
    caret: scriptcraft_core::layout::Rect,
    scale: f32,
) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            page_rect.left() + caret.x as f32 * scale,
            page_rect.top() + caret.y as f32 * scale,
        ),
        Vec2::new(CARET_WIDTH, caret.height as f32 * scale),
    )
}

fn caret_needs_reveal(caret_rect: egui::Rect, visible_clip: egui::Rect) -> bool {
    !visible_clip.contains_rect(caret_rect)
}

fn ordered_selection(selection: Selection) -> (Position, Position) {
    if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}

impl eframe::App for ScriptCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_started = Instant::now();
        self.performance.frames = self.performance.frames.saturating_add(1);
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.toolbar(ui);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(&self.status);
                ui.label(format!(
                    "Path: {}",
                    self.native_path
                        .as_deref()
                        .map_or("(unsaved)".to_owned(), |path| path.display().to_string())
                ));
                let performance = self.performance;
                ui.label(format!(
                    "Performance: frames={} events={} layouts={} hits={} pages={} repaint={}",
                    performance.frames,
                    performance.events,
                    performance.layout_builds,
                    performance.layout_cache_hits,
                    performance.painted_pages,
                    performance.repaint_requests
                ));
                if self.session.dirty() {
                    ui.label(RichText::new("● unsaved").color(Color32::YELLOW));
                }
                if let Some(error) = &self.error {
                    ui.colored_label(Color32::RED, error);
                }
                let diagnostics = &self.native_input_diagnostics;
                ui.label(format!(
                    "Native input diagnostics: focus={} lock={} dispatch={} events={} frames={} activations={} last={}",
                    diagnostics.focused,
                    diagnostics.focus_lock_installed,
                    diagnostics.dispatch_enabled,
                    diagnostics.received_events,
                    diagnostics.dispatched_frames,
                    diagnostics.pointer_activations,
                    if diagnostics.last_event_category.is_empty() {
                        "none"
                    } else {
                        diagnostics.last_event_category
                    }
                ));
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&self.session.project().metadata.title);
            ui.separator();
            self.live_document_surface(ui);
        });
        self.handle_surface_input(ctx);
        self.performance.frame_nanos = self
            .performance
            .frame_nanos
            .saturating_add(frame_started.elapsed().as_nanos());
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "ScriptCraft",
        options,
        Box::new(|_cc| {
            ScriptCraftApp::new()
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|error| error.into())
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptcraft_core::layout::{CHAR_WIDTH, LINE_HEIGHT};
    use scriptcraft_core::Role;

    #[test]
    fn default_toolbar_has_approved_order_and_labels() {
        let items = default_toolbar_items();
        let labels: Vec<_> = items.iter().map(|item| item.label).collect();
        assert_eq!(
            labels,
            vec![
                "New",
                "Open",
                "Save",
                "Save As",
                "Flexible Space",
                "Elements",
                "Flexible Space",
                "Export PDF",
            ]
        );
        assert!(matches!(
            items[0].kind,
            ToolbarItemKind::Action(ToolbarAction::New)
        ));
        assert!(matches!(items[4].kind, ToolbarItemKind::FlexibleSpace));
        assert!(matches!(items[5].kind, ToolbarItemKind::Elements));
        assert!(items.iter().all(|item| !item.label.is_empty()));
        assert!(items
            .iter()
            .filter(|item| matches!(item.kind, ToolbarItemKind::Action(_)))
            .all(|item| !item.icon.is_empty()));
    }

    #[test]
    fn toolbar_model_keeps_normal_and_flexible_spacing_typed() {
        assert!(matches!(
            ToolbarItem::space("space").kind,
            ToolbarItemKind::Space
        ));
        assert!(matches!(
            ToolbarItem::flexible_space("flex").kind,
            ToolbarItemKind::FlexibleSpace
        ));
    }

    #[test]
    fn toolbar_action_presentation_stacks_icon_above_label() {
        for item in default_toolbar_items()
            .iter()
            .copied()
            .filter(|item| matches!(item.kind, ToolbarItemKind::Action(_)))
        {
            assert_eq!(
                toolbar_action_text(item),
                format!("{}\n{}", item.icon, item.label)
            );
            assert!(!toolbar_action_text(item).contains(&format!("{} {}", item.icon, item.label)));
        }
    }

    #[test]
    fn elements_item_keeps_selected_role_and_presentation_label() {
        let item = ToolbarItem::elements();
        assert_eq!(item.label, "Elements");
        assert_eq!(screenplay_role_label(Role::Dialogue), "Dialogue");
        assert!(matches!(item.kind, ToolbarItemKind::Elements));
    }

    #[test]
    fn toolbar_allocation_wide_and_exact_fit_are_deterministic() {
        let items = default_toolbar_items();
        let wide = allocate_toolbar(&items, 800.0);
        assert!(wide.overflow.is_empty());
        assert_eq!(wide.visible.len(), items.len());
        assert_eq!(wide.visible[4].1, 118.0);
        assert_eq!(wide.visible[6].1, 118.0);

        let exact_width = toolbar_content_width(
            &items
                .iter()
                .copied()
                .map(|item| {
                    (
                        item,
                        if is_flexible_toolbar_space(item) {
                            0.0
                        } else {
                            TOOLBAR_NORMAL_ITEM_WIDTH
                        },
                    )
                })
                .collect::<Vec<_>>(),
            0.0,
        );
        let exact = allocate_toolbar(&items, exact_width);
        assert!(exact.overflow.is_empty());
        assert_eq!(exact.visible.len(), items.len());
        assert_eq!(toolbar_content_width(&exact.visible, 0.0), exact_width);
    }

    #[test]
    fn toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu() {
        let allocation = allocate_toolbar(&default_toolbar_items(), 300.0);
        let visible: Vec<_> = allocation
            .visible
            .iter()
            .map(|(item, _)| item.label)
            .collect();
        let overflow: Vec<_> = allocation.overflow.iter().map(|item| item.label).collect();
        assert_eq!(visible, vec!["New", "Open"]);
        assert_eq!(overflow, vec!["Save", "Save As", "Elements", "Export PDF"]);
        assert_eq!(allocation.overflow_width, TOOLBAR_OVERFLOW_WIDTH);
    }

    #[test]
    fn toolbar_allocation_two_flexible_spaces_split_remaining_width_equally() {
        let items = [
            ToolbarItem::action("a", "A", "", ToolbarAction::New),
            ToolbarItem::flexible_space("flex-a"),
            ToolbarItem::action("b", "B", "", ToolbarAction::Save),
            ToolbarItem::flexible_space("flex-b"),
            ToolbarItem::action("c", "C", "", ToolbarAction::ExportPdf),
        ];
        let allocation = allocate_toolbar(&items, 500.0);
        assert_eq!(allocation.visible[1].1, 106.0);
        assert_eq!(allocation.visible[3].1, 106.0);
        assert!(allocation.overflow.is_empty());
    }

    #[test]
    fn toolbar_geometry_uses_half_default_gap_and_stays_within_bounds() {
        assert_eq!(TOOLBAR_ITEM_GAP, 8.0 / 2.0);
        let items = default_toolbar_items();
        let allocation = allocate_toolbar(&items, 800.0);
        assert!(toolbar_content_width(&allocation.visible, allocation.overflow_width) <= 800.0);
        assert_eq!(
            allocation.visible.last().map(|(item, _)| item.label),
            Some("Export PDF")
        );
    }

    #[test]
    fn toolbar_allocation_wide_maximized_equivalent_keeps_export_pdf_fully_allocated() {
        let items = default_toolbar_items();
        let allocation = allocate_toolbar(&items, 600.0);
        assert!(allocation.overflow.is_empty());
        assert_eq!(
            allocation.visible.last().map(|(item, _)| item.label),
            Some("Export PDF")
        );
        assert!(toolbar_content_width(&allocation.visible, 0.0) <= 600.0);
    }

    #[test]
    fn overflow_entries_route_actions_and_elements_submenu_choices() {
        let allocation = allocate_toolbar(&default_toolbar_items(), 300.0);
        assert!(allocation
            .overflow
            .iter()
            .any(|item| matches!(item.kind, ToolbarItemKind::Elements)));

        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.dispatch_toolbar_action(ToolbarAction::New);
        assert_eq!(app.status, "New in-memory project");
        app.change_role_from_toolbar(Role::Dialogue);
        assert_eq!(app.focused_role(), Some(Role::Dialogue));
        assert!(app.error.is_none());
    }

    #[test]
    fn toolbar_new_dispatch_replaces_session_without_persisting_path() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.native_path = Some(PathBuf::from("existing.json"));
        app.session
            .insert("draft")
            .unwrap_or_else(|_| unreachable!());

        app.dispatch_toolbar_action(ToolbarAction::New);

        assert_eq!(app.session.project().metadata.title, "Untitled Screenplay");
        assert_eq!(app.session.project().elements[0].text, "");
        assert_eq!(app.native_path, None);
        assert_eq!(app.status, "New in-memory project");
        assert!(app.error.is_none());
    }

    #[test]
    fn elements_role_labels_cover_all_supported_roles() {
        let expected = [
            (Role::SceneHeading, "Scene Heading"),
            (Role::Action, "Action"),
            (Role::Character, "Character"),
            (Role::Dialogue, "Dialogue"),
            (Role::Parenthetical, "Parenthetical"),
            (Role::Transition, "Transition"),
            (Role::Note, "Note"),
        ];
        assert_eq!(SUPPORTED_SCREENPLAY_ROLES, expected);
        for (role, label) in expected {
            assert_eq!(screenplay_role_label(role), label);
        }
    }

    #[test]
    fn elements_role_choices_change_the_current_caret_element() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("first")
            .unwrap_or_else(|_| unreachable!());
        app.session.enter().unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 1,
                offset: 0,
            }))
            .unwrap_or_else(|_| unreachable!());

        for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
            app.change_role_from_toolbar(role);
            assert_eq!(app.focused_role(), Some(role), "{label}");
            assert_eq!(screenplay_role_label(role), label);
            assert_eq!(app.session.selection().focus.element, 1);
            assert_eq!(app.session.project().elements[0].role, Role::Action);
            assert!(app.error.is_none());
        }
    }

    #[test]
    fn rejected_elements_role_change_preserves_session_and_reports_error() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("unchanged")
            .unwrap_or_else(|_| unreachable!());
        app.session.enter().unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection {
                anchor: Position {
                    element: 0,
                    offset: 0,
                },
                focus: Position {
                    element: 1,
                    offset: 0,
                },
            })
            .unwrap_or_else(|_| unreachable!());
        let before = app.session.clone();

        app.change_role_from_toolbar(Role::Dialogue);

        assert_eq!(app.session, before);
        assert!(app.error.is_some());
        assert!(app.status.contains("document unchanged"));
    }

    #[test]
    fn app_has_one_live_surface_and_no_ui_text_buffer() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements.len(), 1);
        assert!(!app.surface_focused);
    }

    #[test]
    fn session_health_reports_valid_live_model_and_json() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let health = app.session_health();
        assert!(health.contains("session valid"));
        assert!(health.contains("live JSON valid"));
    }

    #[test]
    fn layout_cache_hits_unchanged_reads_and_misses_revision_or_profile_changes() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let first = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        let second = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        assert_eq!(first.as_ref(), second.as_ref());
        assert_eq!(app.performance.layout_builds, 1);
        assert_eq!(app.performance.layout_cache_hits, 1);

        let mut changed_profile = app.session.project().clone();
        changed_profile.page_profile.line_advance += 1;
        app.session
            .replace_with_saved(changed_profile)
            .unwrap_or_else(|_| unreachable!());
        let _ = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.performance.layout_builds, 2);

        app.session
            .insert("revision change")
            .unwrap_or_else(|_| unreachable!());
        let _ = app.layout_for_render().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.performance.layout_builds, 3);
    }

    #[test]
    fn presentation_geometry_scales_default_page_and_centers_it() {
        let (page_width, padding, scale) = presentation_geometry(800.0, 612.0);
        assert!((page_width - 576.0).abs() < f32::EPSILON);
        assert!((padding - 112.0).abs() < f32::EPSILON);
        assert!((scale - (576.0 / 612.0)).abs() < f32::EPSILON);

        let (narrow_width, narrow_padding, narrow_scale) = presentation_geometry(400.0, 612.0);
        assert!((narrow_width - 368.0).abs() < f32::EPSILON);
        assert!((narrow_padding - 16.0).abs() < f32::EPSILON);
        assert!(narrow_scale < scale);
    }

    #[test]
    fn conditional_reveal_only_requests_outside_or_partial_carets() {
        let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        assert!(!caret_needs_reveal(
            egui::Rect::from_min_max(egui::pos2(20.0, 20.0), egui::pos2(21.0, 40.0)),
            clip
        ));
        assert!(caret_needs_reveal(
            egui::Rect::from_min_max(egui::pos2(20.0, -2.0), egui::pos2(21.0, 10.0)),
            clip
        ));
        assert!(caret_needs_reveal(
            egui::Rect::from_min_max(egui::pos2(20.0, 95.0), egui::pos2(21.0, 105.0)),
            clip
        ));
    }

    #[test]
    fn pointer_selection_clears_pending_reveal_intent() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.reveal_intent = RevealIntent::Conditional;
        app.apply_pointer_selection(
            Position {
                element: 0,
                offset: 0,
            },
            true,
            false,
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(app.reveal_intent, RevealIntent::None);
    }

    #[test]
    fn pointer_drag_preserves_initial_hit_as_selection_anchor() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("abcdef")
            .unwrap_or_else(|_| unreachable!());
        let start = Position {
            element: 0,
            offset: 1,
        };
        let end = Position {
            element: 0,
            offset: 5,
        };

        app.apply_pointer_selection(start, true, false)
            .unwrap_or_else(|_| unreachable!());
        app.apply_pointer_selection(end, false, true)
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(
            app.session.selection(),
            Selection {
                anchor: start,
                focus: end,
            }
        );
    }

    #[test]
    fn direct_adapter_operations_use_core_selection_and_role_transition() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.surface_focused = true;
        app.session
            .insert("INT. OFFICE")
            .unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::SceneHeading)
            .unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: "INT. OFFICE".len(),
            }))
            .unwrap_or_else(|_| unreachable!());
        app.session.enter().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[1].role, Role::Action);
        assert_eq!(app.session.selection().focus.element, 1);
    }

    #[test]
    fn enter_key_dispatch_splits_interior_dialogue_without_role_drift() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::Dialogue)
            .unwrap_or_else(|_| unreachable!());
        app.session
            .insert("hello🙂world")
            .unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: "hello".len(),
            }))
            .unwrap_or_else(|_| unreachable!());
        let result = app
            .key_event(Key::Enter, egui::Modifiers::default())
            .unwrap_or_else(|| unreachable!());
        app.mutate(result, "Enter", RevealIntent::Conditional);
        assert_eq!(app.session.project().elements[0].text, "hello");
        assert_eq!(app.session.project().elements[1].text, "🙂world");
        assert_eq!(app.session.project().elements[1].role, Role::Dialogue);
        assert_eq!(
            app.session.selection(),
            Selection::caret(Position {
                element: 1,
                offset: 0
            })
        );
    }

    #[test]
    fn vertical_navigation_uses_wrapped_lines_and_preserves_shift_anchor() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::Dialogue)
            .unwrap_or_else(|_| unreachable!());
        app.session
            .insert(&"abcdefghij ".repeat(5))
            .unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 12,
            }))
            .unwrap_or_else(|_| unreachable!());
        app.move_vertical(true, false)
            .unwrap_or_else(|_| unreachable!());
        assert!(app.session.selection().focus.offset > 12);
        app.move_vertical(false, true)
            .unwrap_or_else(|_| unreachable!());
        assert_ne!(
            app.session.selection().anchor.offset,
            app.session.selection().focus.offset
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vertical_navigation_clamps_single_line_document_edges() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session.insert("first and last").unwrap();

        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 7,
            }))
            .unwrap();
        app.move_vertical(false, false).unwrap();
        assert_eq!(
            app.session.selection(),
            Selection::caret(Position {
                element: 0,
                offset: 0,
            })
        );

        app.session
            .set_selection(Selection::caret(Position {
                element: 0,
                offset: 7,
            }))
            .unwrap();
        app.move_vertical(true, false).unwrap();
        assert_eq!(
            app.session.selection(),
            Selection::caret(Position {
                element: 0,
                offset: "first and last".len(),
            })
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vertical_navigation_clamps_wrapped_first_and_last_lines() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let text = "🙂abcdefghij".repeat(12);
        app.session.insert(&text).unwrap();
        let layout = ScreenplayLayout::build(app.session.project()).unwrap();
        let lines: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .collect();
        assert!(lines.len() > 1);

        let first = lines[0];
        let first_offset = first.source.start + first.text.chars().next().unwrap().len_utf8();
        app.session
            .set_selection(Selection::caret(Position {
                element: first.element,
                offset: first_offset,
            }))
            .unwrap();
        app.move_vertical(false, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, first.source.start);

        let last = lines.last().unwrap();
        let last_offset = last.source.start + last.text.chars().next().unwrap().len_utf8();
        app.session
            .set_selection(Selection::caret(Position {
                element: last.element,
                offset: last_offset,
            }))
            .unwrap();
        app.move_vertical(true, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, last.source.end);
        assert!(text.is_char_boundary(app.session.selection().focus.offset));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vertical_edge_clamps_reset_goal_repeat_safely_and_preserve_shift_anchor() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session.insert(&"a🙂b".repeat(24)).unwrap();
        let layout = ScreenplayLayout::build(app.session.project()).unwrap();
        let lines: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .collect();
        assert!(lines.len() > 1);

        let first = lines[0];
        let first_focus = Position {
            element: first.element,
            offset: first.source.start + first.text.chars().next().unwrap().len_utf8(),
        };
        app.session
            .set_selection(Selection::caret(first_focus))
            .unwrap();
        app.move_vertical(false, true).unwrap();
        let anchor = app.session.selection().anchor;
        assert_eq!(app.session.selection().focus.offset, first.source.start);
        app.move_vertical(false, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, first.source.start);
        assert_eq!(app.preferred_vertical_x, None);
        assert_eq!(anchor, first_focus);

        let last = lines.last().unwrap();
        app.session
            .set_selection(Selection::caret(Position {
                element: last.element,
                offset: last.source.start + last.text.chars().next().unwrap().len_utf8(),
            }))
            .unwrap();
        app.move_vertical(true, true).unwrap();
        assert_eq!(app.session.selection().focus.offset, last.source.end);
        app.move_vertical(true, false).unwrap();
        assert_eq!(app.session.selection().focus.offset, last.source.end);
        assert_eq!(app.preferred_vertical_x, None);
    }

    #[test]
    fn tab_key_is_semantic_and_does_not_insert_literal_tab() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .change_role(Role::Note)
            .unwrap_or_else(|_| unreachable!());
        let before = app.session.clone();
        let result = app.key_event(Key::Tab, egui::Modifiers::default());
        assert!(result.is_some());
        result
            .unwrap_or_else(|| unreachable!())
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "");
        assert_eq!(app.session.project().elements[0].role, Role::Note);
        assert_eq!(app.session.can_undo(), before.can_undo());
    }

    #[test]
    fn rendered_parenthetical_inner_end_routes_pointer_and_both_keys() {
        for key in [Key::Tab, Key::Enter] {
            let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
            app.session
                .insert("(words)")
                .unwrap_or_else(|_| unreachable!());
            app.session
                .change_role(Role::Parenthetical)
                .unwrap_or_else(|_| unreachable!());

            let context = egui::Context::default();
            let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
            let layout =
                ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
            let line = &layout.pages[0].lines[0];
            let inner_end_pointer = |surface_rect: egui::Rect| {
                let (_, horizontal_padding, scale) =
                    presentation_geometry(surface_rect.width(), layout.profile.paper_width as f32);
                egui::pos2(
                    surface_rect.left()
                        + horizontal_padding
                        + (line.rect.x as f32
                            + ("(words".chars().count() as f32 * layout.char_width as f32))
                            * scale,
                    surface_rect.top() + (line.rect.y as f32 + 1.0) * scale,
                )
            };
            let pointer_click = |pos| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            };

            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(screen_rect),
                    focused: true,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.live_document_surface(ui);
                    });
                },
            );
            let surface_rect = app.editor_surface_rect.unwrap_or_else(|| unreachable!());
            let pointer = inner_end_pointer(surface_rect);

            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(screen_rect),
                    focused: true,
                    events: vec![egui::Event::PointerMoved(pointer), pointer_click(pointer)],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.live_document_surface(ui);
                    });
                    app.handle_surface_input(ctx);
                },
            );

            assert_eq!(
                app.session.selection(),
                Selection::caret(Position {
                    element: 0,
                    offset: "(words".len(),
                })
            );
            let key_event = egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            };
            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(screen_rect),
                    focused: true,
                    events: vec![key_event],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.live_document_surface(ui);
                    });
                    app.handle_surface_input(ctx);
                },
            );

            assert_eq!(app.session.project().elements[0].text, "(words)");
            assert_eq!(app.session.project().elements[1].role, Role::Dialogue);
            assert_eq!(
                app.session.selection().focus,
                Position {
                    element: 1,
                    offset: 0
                }
            );
        }
    }

    #[test]
    fn direct_surface_editing_supports_selection_replacement_deletion_and_backspace() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session.insert("abc").unwrap_or_else(|_| unreachable!());
        app.session
            .set_selection(Selection {
                anchor: Position {
                    element: 0,
                    offset: 1,
                },
                focus: Position {
                    element: 0,
                    offset: 2,
                },
            })
            .unwrap_or_else(|_| unreachable!());
        app.session.insert("🙂").unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "a🙂c");
        app.session.delete().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "a🙂");
        app.session.backspace().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[0].text, "a");
    }

    #[test]
    fn focused_event_dispatch_inserts_unicode_and_invalidates_render_state() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.surface_focused = true;
        app.rendered_revision = Some(app.session.revision());
        app.dispatch_surface_events(
            &[egui::Event::Text("café🙂".into())],
            egui::Modifiers::default(),
        );
        assert_eq!(app.session.project().elements[0].text, "café🙂");
        assert_eq!(app.session.selection().focus.offset, "café🙂".len());
        assert_eq!(app.rendered_revision, None);
        assert_eq!(app.native_input_diagnostics.received_events, 1);
        assert_eq!(app.native_input_diagnostics.last_event_category, "text");
        assert!(app.performance.event_nanos > 0);
    }

    #[test]
    fn unfocused_surface_does_not_consume_keyboard_events() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let context = egui::Context::default();
        let _ = context.run(
            egui::RawInput {
                focused: true,
                events: vec![egui::Event::Text("blocked".into())],
                ..Default::default()
            },
            |ctx| app.handle_surface_input(ctx),
        );
        assert_eq!(app.session.project().elements[0].text, "");
        assert_eq!(app.native_input_diagnostics.received_events, 0);
        assert!(!app.native_input_diagnostics.dispatch_enabled);
    }

    #[test]
    fn rendered_page_editor_retains_focus_for_later_frame_input() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.session
            .insert("seed")
            .unwrap_or_else(|_| unreachable!());
        let context = egui::Context::default();
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let (_, horizontal_padding, scale) =
            presentation_geometry(800.0, layout.profile.paper_width as f32);
        let line = &layout.pages[0].lines[0];
        let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let pointer_click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: Vec::new(),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                app.handle_surface_input(ctx);
            },
        );
        let editor_id = app.editor_surface_id.unwrap_or_else(|| unreachable!());
        assert!(!app.surface_focused);
        assert!(!context.memory(|memory| memory.has_focus(editor_id)));

        let surface_rect = app.editor_surface_rect.unwrap_or_else(|| unreachable!());
        let pointer = egui::pos2(
            surface_rect.left() + horizontal_padding + (line.rect.x as f32 + 1.0) * scale,
            surface_rect.top() + (line.rect.y as f32 + 1.0) * scale,
        );
        let blank_pointer = egui::pos2(surface_rect.right() - 4.0, surface_rect.top() + 4.0);
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::PointerMoved(blank_pointer),
                    pointer_click(blank_pointer, true),
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
            },
        );
        assert!(app.surface_focused);
        assert!(context.memory(|memory| memory.has_focus(editor_id)));
        assert_eq!(app.native_input_diagnostics.pointer_activations, 1);
        assert_eq!(app.session.selection().focus.offset, "seed".len());

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::PointerMoved(pointer),
                    pointer_click(pointer, true),
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                app.handle_surface_input(ctx);
            },
        );
        assert!(app.surface_focused);
        assert!(context.memory(|memory| memory.has_focus(editor_id)));
        assert!(app.native_input_diagnostics.focus_lock_installed);
        assert_eq!(app.native_input_diagnostics.pointer_activations, 2);

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::PointerMoved(pointer),
                    pointer_click(pointer, false),
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                assert!(app.surface_focused);
                assert!(context.memory(|memory| memory.has_focus(editor_id)));
                app.handle_surface_input(ctx);
            },
        );

        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                focused: true,
                events: vec![
                    egui::Event::Text("later".into()),
                    egui::Event::Key {
                        key: Key::Backspace,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.live_document_surface(ui);
                });
                assert!(app.surface_focused);
                app.handle_surface_input(ctx);
            },
        );

        assert_eq!(app.session.project().elements[0].text.chars().count(), 8);
        assert_ne!(app.session.project().elements[0].text, "seed");
    }

    #[test]
    fn ime_preedit_is_transient_and_commit_is_inserted_once() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.dispatch_surface_events(
            &[
                egui::Event::Ime(egui::ImeEvent::Preedit("に".into())),
                egui::Event::Text("に".into()),
                egui::Event::Ime(egui::ImeEvent::Commit("日本".into())),
            ],
            egui::Modifiers::default(),
        );
        assert_eq!(app.session.project().elements[0].text, "日本");
        assert!(app.ime_preedit.is_empty());
        assert!(!app.ime_active);
    }

    #[test]
    fn editing_key_dispatch_covers_enter_delete_backspace_undo_redo_and_shift_selection() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.dispatch_surface_events(
            &[egui::Event::Text("abc".into())],
            egui::Modifiers::default(),
        );
        let plain = egui::Modifiers::default();
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    shift: true,
                    ..plain
                },
            }],
            plain,
        );
        assert_ne!(
            app.session.selection().anchor,
            app.session.selection().focus
        );
        app.dispatch_surface_events(
            &[
                egui::Event::Key {
                    key: Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: plain,
                },
                egui::Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: plain,
                },
            ],
            plain,
        );
        assert_eq!(app.session.project().elements.len(), 2);
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    command: true,
                    ..plain
                },
            }],
            plain,
        );
        assert_eq!(app.session.project().elements.len(), 1);
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::Y,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    command: true,
                    ..plain
                },
            }],
            plain,
        );
        assert_eq!(app.session.project().elements.len(), 2);
        app.dispatch_surface_events(
            &[egui::Event::Key {
                key: Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: plain,
            }],
            plain,
        );
    }

    #[test]
    fn caret_selection_mapping_is_source_mapped() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let line = &layout.pages[0].lines[0];
        assert_eq!(
            layout.hit_test(0, line.rect.x, line.rect.y),
            Some(Caret {
                element: 0,
                offset: 0
            })
        );
        assert!(layout
            .caret_rect(Caret {
                element: 0,
                offset: 0
            })
            .is_ok());
    }

    #[test]
    fn fixed_metric_rendering_and_caret_bar_do_not_accumulate_drift() {
        let columns = 120;
        let scale = 360.0 / 612.0;
        let origin = 90.0 * scale;
        let expected = origin + columns as f32 * CHAR_WIDTH as f32 * scale;
        assert_eq!(
            fixed_metric_x(origin, columns, CHAR_WIDTH as f32 * scale),
            expected
        );

        let bar = caret_bar_rect(
            egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(360.0, 460.0)),
            scriptcraft_core::layout::Rect {
                x: 90 + (columns as u32) * CHAR_WIDTH,
                y: 72,
                width: 1,
                height: LINE_HEIGHT,
            },
            scale,
        );
        assert_eq!(bar.width(), CARET_WIDTH);
        assert!((bar.left() - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn viewport_scale_only_transforms_fixed_print_geometry() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let line = &layout.pages[0].lines[0];
        let (_, _, small_scale) = presentation_geometry(400.0, 612.0);
        let (_, _, large_scale) = presentation_geometry(1_200.0, 612.0);

        assert_eq!(layout.pages.len(), 1);
        assert_eq!(line.rect.y, 72);
        assert_eq!(line.rect.height, LINE_HEIGHT);
        assert_eq!(line.rect.y as f32 * small_scale, 72.0 * small_scale);
        assert_eq!(line.rect.y as f32 * large_scale, 72.0 * large_scale);
        assert_ne!(small_scale, large_scale);
        assert_eq!(layout.pages[0].content_bounds.x, 108);
        assert_eq!(layout.pages[0].content_bounds.y, 72);
    }

    #[test]
    fn renderer_uses_profile_point_size_and_deterministic_monospace_fallback() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let layout =
            ScreenplayLayout::build(app.session.project()).unwrap_or_else(|_| unreachable!());
        let font = renderer_font(&layout, 0.5);
        assert_eq!(layout.profile.font_family, "Courier");
        assert_eq!(layout.profile.font_size, 12);
        assert_eq!(font.size, 6.0);
        assert_eq!(font.family, FontFamily::Monospace);
    }

    #[test]
    fn rendered_caret_blink_alternates_on_controlled_egui_time() {
        let context = egui::Context::default();
        let id = egui::Id::new("wo002-caret");
        let frame = |time: f64| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, true, true, 0.0),
            );
            (visible, output)
        };

        assert!(frame(0.0).0);
        assert!(frame(0.499).0);
        assert!(!frame(0.5).0);
        assert!(!frame(0.999).0);
        assert!(frame(1.0).0);
    }

    #[test]
    fn rendered_caret_blink_schedules_only_focused_collapsed_phases() {
        let context = egui::Context::default();
        let id = egui::Id::new("wo002-repaint");
        let run = |time: f64, focused: bool, collapsed: bool| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, focused, collapsed, 0.0),
            );
            (visible, output)
        };

        let _ = run(2.0, true, true);
        let focused = run(2.1, true, true);
        assert!(focused.0);
        assert!(focused
            .1
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|output| output.repaint_delay)
            .is_some_and(|delay| delay <= CARET_BLINK_INTERVAL));
        assert_eq!(caret_blink_repaint_delay(2.1), Duration::from_millis(400));
        let unfocused = run(2.6, false, true);
        assert!(!unfocused.0);
        assert_eq!(caret_blink_repaint_delay(2.6), Duration::from_millis(400));
        assert!(!run(2.6, true, false).0);
    }

    #[test]
    fn rendered_caret_blink_identity_change_uses_a_distinct_entry() {
        let context = egui::Context::default();
        let first = egui::Id::new("wo002-first-caret");
        let second = egui::Id::new("wo002-second-caret");
        let run = |id: egui::Id, time: f64| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, true, true, 0.0),
            );
            (visible, output)
        };

        assert!(run(first, 0.0).0);
        assert!(!run(first, 0.5).0);
        assert!(!run(second, 0.5).0);
        assert!(run(second, 1.0).0);
    }

    #[test]
    fn rendered_caret_blink_focus_regain_starts_at_initial_visible_phase() {
        let context = egui::Context::default();
        let focused_id = egui::Id::new("wo002-focus-generation-0");
        let regained_id = egui::Id::new("wo002-focus-generation-1");
        let run = |id: egui::Id, time: f64, focused: bool, reset_time: f64| {
            let mut visible = false;
            let output = context.run(
                egui::RawInput {
                    time: Some(time),
                    predicted_dt: 0.5,
                    focused: true,
                    ..Default::default()
                },
                |ctx| visible = caret_blink_visible(ctx, id, focused, true, reset_time),
            );
            (visible, output)
        };

        assert!(run(focused_id, 0.0, true, 0.0).0);
        assert!(!run(focused_id, 0.75, true, 0.0).0);

        let unfocused = run(focused_id, 0.75, false, 0.0);
        assert!(!unfocused.0);
        assert_eq!(
            unfocused
                .1
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map(|output| output.repaint_delay),
            Some(Duration::MAX)
        );

        // Focus loss changes the animation identity; regain establishes a new
        // origin so a stale hidden phase cannot leak across the boundary.
        assert!(run(regained_id, 0.75, true, 0.75).0);
        assert!(run(regained_id, 1.249, true, 0.75).0);
        assert!(!run(regained_id, 1.25, true, 0.75).0);
    }

    #[test]
    fn named_save_and_open_update_path_and_clean_state() {
        let root = std::env::temp_dir().join(format!("scriptcraft-wo010-{}", std::process::id()));
        let first = root.join("first.json");
        let second = root.join("second.json");
        let _ = std::fs::create_dir_all(&root);

        let mut source = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        source
            .session
            .insert("named project")
            .unwrap_or_else(|_| unreachable!());
        source.save_native_at(&first);
        assert_eq!(source.native_path, Some(first.clone()));
        assert!(!source.session.dirty());

        let mut replacement = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        replacement
            .session
            .insert("second project")
            .unwrap_or_else(|_| unreachable!());
        replacement.save_native_at(&second);

        source.open_native_at(&second);
        assert_eq!(source.native_path, Some(second));
        assert!(!source.session.dirty());
        assert_eq!(source.session.project().elements[0].text, "second project");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_named_save_preserves_path_file_and_dirty_state() {
        let root =
            std::env::temp_dir().join(format!("scriptcraft-wo010-failure-{}", std::process::id()));
        let destination = root.join("project.json");
        let _ = std::fs::create_dir_all(&root);
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        app.save_native_at(&destination);
        app.session
            .insert("unsaved change")
            .unwrap_or_else(|_| unreachable!());
        let before = std::fs::read(&destination).unwrap_or_default();
        let failed_path = root.join("missing").join("project.json");

        app.save_native_at(&failed_path);

        assert_eq!(app.native_path, Some(destination));
        assert!(app.session.dirty());
        assert_eq!(
            std::fs::read(root.join("project.json")).unwrap_or_default(),
            before
        );
        assert!(app.error.is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_open_does_not_replace_named_project() {
        let mut app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        let original_path = PathBuf::from("known-project.json");
        let original_text = app.session.project().elements[0].text.clone();
        app.native_path = Some(original_path.clone());

        app.open_native_at(std::path::Path::new("does-not-exist.json"));

        assert_eq!(app.native_path, Some(original_path));
        assert_eq!(app.session.project().elements[0].text, original_text);
        assert!(!app.session.dirty());
        assert!(app.error.is_some());
    }
}
