#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use eframe::egui::{self, Color32, FontId, Key, RichText, Sense, Stroke, Vec2};
use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::exports;
use scriptcraft_core::layout::{Caret, ScreenplayLayout, CHAR_WIDTH, LINE_HEIGHT};
use scriptcraft_core::persistence::{self, FileSystem, StdFileSystem};
use scriptcraft_core::{Project, Role};
use std::path::PathBuf;

struct ScriptCraftApp {
    session: EditingSession,
    surface_focused: bool,
    drag_anchor: Option<Position>,
    status: String,
    error: Option<String>,
    native_path: Option<PathBuf>,
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
            status: "In-memory demo project".into(),
            error: None,
            native_path: None,
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

    fn mutate(&mut self, result: Result<(), EditError>, success: &str) {
        match result {
            Ok(()) => {
                self.status = success.into();
                self.clear_error();
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
        self.session.set_selection(selection)
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
    ) {
        let page_height = layout.profile.paper_height as f32 * scale;
        let (page_rect, response) =
            ui.allocate_exact_size(Vec2::new(page_width, page_height), Sense::click_and_drag());
        let painter = ui.painter_at(page_rect);
        painter.rect_filled(page_rect, 2.0, Color32::from_gray(246));
        painter.rect_stroke(page_rect, 2.0, Stroke::new(1.0_f32, Color32::GRAY));

        if let Some(pointer) = response.interact_pointer_pos() {
            if (response.clicked() || response.drag_started() || response.dragged())
                && !layout.pages[page_index].title_page
            {
                if let Some(position) =
                    self.caret_from_pointer(layout, page_index, page_rect, scale, pointer)
                {
                    if let Err(error) = self.apply_pointer_selection(
                        position,
                        response.drag_started(),
                        response.dragged(),
                    ) {
                        self.show_error(error);
                    } else {
                        self.surface_focused = true;
                        self.clear_error();
                    }
                }
            }
        }

        for line in &layout.pages[page_index].lines {
            let x = page_rect.left() + line.rect.x as f32 * scale;
            let y = page_rect.top() + line.rect.y as f32 * scale;
            let char_width = CHAR_WIDTH as f32 * scale;
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
            painter.text(
                egui::pos2(x, y),
                egui::Align2::LEFT_TOP,
                &line.text,
                FontId::monospace((10.0 * scale).max(5.0)),
                Color32::DARK_GRAY,
            );
        }

        let selection = self.session.selection();
        if selection.anchor == selection.focus && !layout.pages[page_index].title_page {
            let caret = Caret {
                element: selection.focus.element,
                offset: selection.focus.offset,
            };
            if let Ok(rect) = layout.caret_rect(caret) {
                let x = page_rect.left() + rect.x as f32 * scale;
                let y = page_rect.top() + rect.y as f32 * scale;
                painter.line_segment(
                    [
                        egui::pos2(x, y),
                        egui::pos2(x, y + rect.height as f32 * scale),
                    ],
                    Stroke::new(1.5_f32, Color32::from_rgb(20, 80, 150)),
                );
            }
        }
    }

    fn live_document_surface(&mut self, ui: &mut egui::Ui) {
        let layout = match ScreenplayLayout::build(self.session.project()) {
            Ok(layout) => layout,
            Err(error) => {
                ui.colored_label(Color32::RED, format!("Layout error: {error:?}"));
                return;
            }
        };
        let available = ui.available_size();
        let page_width = 360.0_f32.min(available.x.max(240.0));
        let scale = page_width / layout.profile.paper_width as f32;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for page_index in 0..layout.pages.len() {
                self.paint_page(ui, &layout, page_index, page_width, scale);
                ui.add_space(12.0);
            }
        });
    }

    fn handle_surface_input(&mut self, ctx: &egui::Context) {
        if !self.surface_focused {
            return;
        }
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            let result = match event {
                egui::Event::Text(text) if !text.is_empty() => self.session.insert(&text),
                egui::Event::Key {
                    key: Key::Backspace,
                    pressed: true,
                    ..
                } => self.session.backspace(),
                egui::Event::Key {
                    key: Key::Delete,
                    pressed: true,
                    ..
                } => self.session.delete(),
                egui::Event::Key {
                    key: Key::Enter,
                    pressed: true,
                    ..
                } => self.session.enter(),
                egui::Event::Key {
                    key: Key::ArrowLeft,
                    pressed: true,
                    ..
                } => self.move_horizontal(false),
                egui::Event::Key {
                    key: Key::ArrowRight,
                    pressed: true,
                    ..
                } => self.move_horizontal(true),
                egui::Event::Key {
                    key: Key::Z,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => self.session.undo(),
                egui::Event::Key {
                    key: Key::Y,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => self.session.redo(),
                _ => continue,
            };
            self.mutate(result, "Edited through EditingSession");
        }
    }

    fn move_horizontal(&mut self, right: bool) -> Result<(), EditError> {
        let selection = self.session.selection();
        if selection.anchor != selection.focus {
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
        self.session.set_selection(Selection::caret(next))
    }
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
