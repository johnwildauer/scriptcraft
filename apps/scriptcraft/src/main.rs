#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use eframe::egui::{self, Color32, FontId, Key, RichText, Sense, Stroke, Vec2};
use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::exports;
use scriptcraft_core::layout::{Caret, ScreenplayLayout, CHAR_WIDTH, LINE_HEIGHT};
use scriptcraft_core::persistence::{self, FileSystem, StdFileSystem};
use scriptcraft_core::{Project, Role};
use std::path::PathBuf;
use std::time::Duration;

const EDITOR_SURFACE_ID: &str = "page-editor-surface";
const CARET_WIDTH: f32 = 1.5;
const CURRENT_DEFAULT_PAGE_WIDTH: f32 = 360.0;
const DEFAULT_PRESENTATION_SCALE_MULTIPLIER: f32 = 1.6;
const PAGE_HORIZONTAL_PADDING: f32 = 16.0;
const MINIMUM_PAGE_WIDTH: f32 = 240.0;

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
    reveal_caret: bool,
    preferred_vertical_x: Option<u32>,
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
            reveal_caret: false,
            preferred_vertical_x: None,
        })
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

    fn export_text(&mut self) {
        match exports::export_text(self.session.project()) {
            Ok(result) => match StdFileSystem
                .write(&PathBuf::from("scriptcraft.txt"), result.text.as_bytes())
            {
                Ok(()) => {
                    self.status = format!(
                        "Exported lossy text to scriptcraft.txt — {}",
                        result.loss.message
                    );
                    self.clear_error();
                }
                Err(error) => self.show_error(error),
            },
            Err(error) => self.show_error(error),
        }
    }

    fn show_error<E: std::fmt::Debug>(&mut self, error: E) {
        self.error = Some(format!("{error:?}"));
        self.status = "Operation failed; document unchanged".into();
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

    fn mutate(&mut self, result: Result<(), EditError>, success: &str) {
        match result {
            Ok(()) => {
                self.preferred_vertical_x = None;
                self.status = success.into();
                self.clear_error();
                self.rendered_revision = None;
                self.reveal_caret = true;
            }
            Err(error) => self.show_error(error),
        }
    }

    fn role_controls(&mut self, ui: &mut egui::Ui) {
        let element = self.session.selection().focus.element;
        ui.horizontal_wrapped(|ui| {
            ui.label("Role:");
            for role in roles() {
                let selected = self.session.project().elements[element].role == role;
                if ui.selectable_label(selected, role_label(role)).clicked() {
                    let result = self.session.change_role(role);
                    self.mutate(result, "Role changed through EditingSession");
                }
            }
        });
    }

    fn caret_from_pointer(
        &self,
        layout: &ScreenplayLayout,
        page: usize,
        page_rect: egui::Rect,
        scale: f32,
        pointer: egui::Pos2,
    ) -> Option<Position> {
        let local_x = ((pointer.x - page_rect.left()) / scale).max(0.0) as u32;
        let local_y = ((pointer.y - page_rect.top()) / scale).max(0.0) as u32;
        layout
            .hit_test(page, local_x, local_y)
            .map(|caret| Position {
                element: caret.element,
                offset: caret.offset,
            })
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
            self.reveal_caret = true;
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
        let page_height = layout.profile.paper_height as f32 * scale;
        let page_rect = ui.allocate_space(Vec2::new(page_width, page_height)).1;
        let painter = ui.painter_at(page_rect);
        let char_width = CHAR_WIDTH as f32 * scale;
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
                        Vec2::new(width.max(1.0), LINE_HEIGHT as f32 * scale),
                    ),
                    0.0,
                    color,
                );
            }
            paint_fixed_metric_text(
                &painter,
                egui::pos2(x, y),
                &line.text,
                FontId::monospace((10.0 * scale).max(5.0)),
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
            ));
            if let Ok(rect) = layout.caret_rect(caret) {
                let blink_active = self.surface_focused;
                let blink_phase = ui.ctx().animate_bool_with_time(blink_id, blink_active, 0.5);
                if blink_active {
                    ui.ctx().request_repaint_after(Duration::from_millis(500));
                }
                if caret_should_paint(blink_active, true, blink_phase) {
                    painter.rect_filled(
                        caret_bar_rect(page_rect, rect, scale),
                        0.0,
                        Color32::from_rgb(20, 80, 150),
                    );
                }
                if !self.ime_preedit.is_empty() {
                    let x = page_rect.left() + rect.x as f32 * scale + char_width;
                    let y = page_rect.top() + rect.y as f32 * scale;
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        &self.ime_preedit,
                        FontId::monospace((10.0 * scale).max(5.0)),
                        Color32::from_rgb(80, 80, 160),
                    );
                }
            }
        }

        page_rect
    }

    fn live_document_surface(&mut self, ui: &mut egui::Ui) {
        let layout = match ScreenplayLayout::build(self.session.project()) {
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
        let reveal_caret = self.reveal_caret;
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
                if reveal_caret {
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
                            ui.scroll_to_rect(caret_rect, Some(egui::Align::Center));
                        }
                    }
                }
            });
        self.reveal_caret = false;

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
                if let Some(page_index) = page_rects.iter().position(|rect| rect.contains(pointer))
                {
                    if !layout.pages[page_index].title_page {
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
        }
        self.surface_focused = response.has_focus();
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
                egui::Event::Paste(text) if !text.is_empty() => Some(self.session.insert(text)),
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
                self.mutate(result, "Edited through EditingSession");
            }
        }
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
        self.native_input_diagnostics.dispatched_frames = self
            .native_input_diagnostics
            .dispatched_frames
            .saturating_add(1);
        let events = ctx.input(|input| input.events.clone());
        let modifiers = ctx.input(|input| input.modifiers);
        self.dispatch_surface_events(&events, modifiers);
        ctx.request_repaint();
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
        let layout = ScreenplayLayout::build(self.session.project())
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
        let Some(target_index) = target_index else {
            self.preferred_vertical_x = None;
            return self.session.set_selection(selection);
        };
        let current_line = lines[current_index];
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

fn fixed_metric_x(origin: f32, column: usize, char_width: f32) -> f32 {
    origin + column as f32 * char_width
}

fn caret_should_paint(focused: bool, collapsed: bool, blink_phase: f32) -> bool {
    focused && collapsed && blink_phase > 0.5
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
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
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
                if ui.button("Open").clicked() {
                    self.open_native();
                }
                if ui.button("Save").clicked() {
                    self.save_native();
                }
                if ui.button("Save As").clicked() {
                    self.save_as_native();
                }
                if ui.button("Export PDF").clicked() {
                    self.export_pdf();
                }
                if ui.button("Export TXT (lossy)").clicked() {
                    self.export_text();
                }
                if ui
                    .add_enabled(self.session.can_undo(), egui::Button::new("Undo"))
                    .clicked()
                {
                    let result = self.session.undo();
                    self.mutate(result, "Undo");
                }
                if ui
                    .add_enabled(self.session.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                {
                    let result = self.session.redo();
                    self.mutate(result, "Redo");
                }
                ui.separator();
                ui.label("Page layout live document");
            });
            self.role_controls(ui);
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
    }
}

fn roles() -> [Role; 7] {
    [
        Role::SceneHeading,
        Role::Action,
        Role::Character,
        Role::Dialogue,
        Role::Parenthetical,
        Role::Transition,
        Role::Note,
    ]
}

fn role_label(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "Scene heading",
        Role::Action => "Action",
        Role::Character => "Character",
        Role::Dialogue => "Dialogue",
        Role::Parenthetical => "Parenthetical",
        Role::Transition => "Transition",
        Role::Note => "Note",
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

    #[test]
    fn app_has_one_live_surface_and_no_ui_text_buffer() {
        let app = ScriptCraftApp::new().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements.len(), 1);
        assert!(!app.surface_focused);
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
                offset: 4,
            }))
            .unwrap_or_else(|_| unreachable!());
        app.session.enter().unwrap_or_else(|_| unreachable!());
        assert_eq!(app.session.project().elements[1].role, Role::Action);
        assert_eq!(app.session.selection().focus.element, 1);
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
        assert_eq!(layout.pages[0].content_bounds.x, 90);
        assert_eq!(layout.pages[0].content_bounds.y, 72);
    }

    #[test]
    fn caret_blink_is_focused_collapsed_and_transient() {
        assert!(caret_should_paint(true, true, 0.75));
        assert!(!caret_should_paint(false, true, 0.75));
        assert!(!caret_should_paint(true, false, 0.75));
        assert!(!caret_should_paint(true, true, 0.25));
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
