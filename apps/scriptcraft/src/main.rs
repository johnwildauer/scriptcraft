#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use eframe::egui::{self, Color32, FontId, Key, RichText, Sense, Stroke, TextStyle, Vec2};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports;
use scriptcraft_core::layout::ScreenplayLayout;
use scriptcraft_core::persistence::{self, FileSystem, StdFileSystem};
use scriptcraft_core::{Project, Role};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Continuous,
    Pages,
}

struct ScriptCraftApp {
    session: EditingSession,
    view: ViewMode,
    selected: usize,
    editor_text: String,
    editor_element: usize,
    status: String,
    error: Option<String>,
    native_path: PathBuf,
}

impl ScriptCraftApp {
    fn new() -> Result<Self, String> {
        let project = Project::new("demo-project", "Untitled Screenplay", "Writer")
            .map_err(|error| format!("new project failed: {error:?}"))?;
        let session =
            EditingSession::new(project).map_err(|error| format!("session failed: {error:?}"))?;
        Ok(Self {
            editor_text: session.project().elements[0].text.clone(),
            editor_element: 0,
            session,
            view: ViewMode::Continuous,
            selected: 0,
            status: "In-memory demo project".into(),
            error: None,
            native_path: PathBuf::from("scriptcraft.json"),
        })
    }

    fn save_native(&mut self) {
        let fs = StdFileSystem;
        match persistence::atomic_save(&fs, &self.native_path, self.session.project()) {
            Ok(()) => {
                self.session.mark_saved();
                self.status = format!("Saved {}", self.native_path.display());
                self.clear_error();
            }
            Err(error) => self.show_error(error),
        }
    }

    fn open_native(&mut self) {
        let fs = StdFileSystem;
        let result = fs
            .read(&self.native_path)
            .and_then(|bytes| persistence::deserialize(&bytes))
            .map_err(|error| format!("open failed: {error:?}"));
        match result {
            Ok(project) => match self.session.replace_with_saved(project) {
                Ok(()) => {
                    self.selected = 0;
                    self.editor_element = 0;
                    self.editor_text = self.session.project().elements[0].text.clone();
                    self.status = format!("Opened {}", self.native_path.display());
                    self.clear_error();
                }
                Err(error) => self.show_error(error),
            },
            Err(error) => self.show_error(error),
        }
    }

    fn inspect_recovery(&mut self) {
        let fs = StdFileSystem;
        match persistence::discover_recovery(
            &fs,
            std::path::Path::new("."),
            self.session.revision(),
        ) {
            Ok(candidates) => {
                self.status = if candidates.is_empty() {
                    "No newer recovery candidate".into()
                } else {
                    format!(
                        "{} recovery candidate(s) available; open is explicit",
                        candidates.len()
                    )
                }
            }
            Err(error) => self.show_error(error),
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

    fn select(&mut self, index: usize) {
        if index >= self.session.project().elements.len() {
            return;
        }
        self.selected = index;
        let text = self.session.project().elements[index].text.clone();
        let offset = text.len();
        self.editor_element = index;
        self.editor_text = text;
        self.clear_error();
        if let Err(error) = self.session.set_selection(Selection::caret(Position {
            element: index,
            offset,
        })) {
            self.show_error(error);
        }
    }

    fn mutate(&mut self, result: Result<(), scriptcraft_core::editing::EditError>, success: &str) {
        match result {
            Ok(()) => {
                self.status = success.into();
                self.clear_error();
            }
            Err(error) => self.show_error(error),
        }
    }

    fn replace_selected_text(&mut self) {
        let Some(element) = self.session.project().elements.get(self.selected) else {
            return;
        };
        let selection = Selection {
            anchor: Position {
                element: self.selected,
                offset: 0,
            },
            focus: Position {
                element: self.selected,
                offset: element.text.len(),
            },
        };
        if let Err(error) = self.session.set_selection(selection) {
            self.show_error(error);
            return;
        }
        let text = self.editor_text.clone();
        let result = self.session.insert(&text);
        self.mutate(result, "Edited through EditingSession");
    }

    fn show_error<E: std::fmt::Debug>(&mut self, error: E) {
        self.error = Some(format!("{error:?}"));
        self.status = "Edit rejected; document unchanged".into();
    }

    fn clear_error(&mut self) {
        self.error = None;
    }

    fn role_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Role:");
            for role in roles() {
                let selected = self.session.project().elements[self.selected].role == role;
                if ui.selectable_label(selected, role_label(role)).clicked() {
                    let result = self.session.change_role(role);
                    self.mutate(result, "Role changed through EditingSession");
                }
            }
        });
    }

    fn continuous_view(&mut self, ui: &mut egui::Ui) {
        let count = self.session.project().elements.len();
        for index in 0..count {
            let item = &self.session.project().elements[index];
            let selected = index == self.selected;
            let response = ui.add_sized(
                [ui.available_width(), 28.0],
                egui::SelectableLabel::new(
                    selected,
                    RichText::new(format!("{}  {}", role_label(item.role), item.text))
                        .text_style(TextStyle::Body),
                ),
            );
            if response.clicked() {
                self.select(index);
            }
            if selected {
                self.role_controls(ui);
                let changed = ui
                    .add(
                        egui::TextEdit::multiline(&mut self.editor_text)
                            .desired_rows(3)
                            .hint_text("Type screenplay text…"),
                    )
                    .changed();
                if changed {
                    self.replace_selected_text();
                }
            }
            ui.add_space(6.0);
        }
    }

    fn page_view(&mut self, ui: &mut egui::Ui) {
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
        let page_height = layout.profile.paper_height as f32 * scale;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for page in &layout.pages {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(page_width, page_height), Sense::click());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 2.0, Color32::from_gray(246));
                painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, Color32::GRAY));
                for line in &page.lines {
                    let x = rect.left() + line.rect.x as f32 * scale;
                    let y = rect.top() + line.rect.y as f32 * scale;
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        &line.text,
                        FontId::monospace((10.0 * scale).max(5.0)),
                        if line.element == self.selected {
                            Color32::from_rgb(20, 80, 150)
                        } else {
                            Color32::DARK_GRAY
                        },
                    );
                }
                if response.clicked() {
                    self.status = format!("Page {} preview", page.number + 1);
                }
                ui.add_space(12.0);
            }
        });
    }
}

impl eframe::App for ScriptCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.key_pressed(Key::Z) && input.modifiers.command) {
            let result = self.session.undo();
            self.mutate(result, "Undo");
            self.editor_text = self.session.project().elements[self.selected].text.clone();
        }
        if ctx.input(|input| input.key_pressed(Key::Y) && input.modifiers.command) {
            let result = self.session.redo();
            self.mutate(result, "Redo");
            self.editor_text = self.session.project().elements[self.selected].text.clone();
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    let result = Project::new("demo-project", "Untitled Screenplay", "Writer")
                        .map_err(|error| format!("new project failed: {error:?}"))
                        .and_then(|project| {
                            EditingSession::new(project)
                                .map_err(|error| format!("session failed: {error:?}"))
                        });
                    match result {
                        Ok(session) => {
                            self.session = session;
                            self.selected = 0;
                            self.editor_element = 0;
                            self.editor_text.clear();
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
                if ui.button("Recovery").clicked() {
                    self.inspect_recovery();
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
                if ui.button("Enter → next role").clicked() {
                    let result = self.session.enter();
                    self.mutate(result, "Role transition through EditingSession");
                    let selection = self.session.selection();
                    self.selected = selection.focus.element;
                    self.editor_element = self.selected;
                    self.editor_text = self.session.project().elements[self.selected].text.clone();
                }
                ui.separator();
                ui.selectable_value(&mut self.view, ViewMode::Continuous, "Continuous");
                ui.selectable_value(&mut self.view, ViewMode::Pages, "Page preview");
                ui.separator();
                ui.label(format!(
                    "Role: {}",
                    role_label(self.session.project().elements[self.selected].role)
                ));
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(&self.status);
                if self.session.dirty() {
                    ui.label(RichText::new("● unsaved (memory only)").color(Color32::YELLOW));
                }
                if let Some(error) = &self.error {
                    ui.colored_label(Color32::RED, error);
                }
                ui.label("Persistence host boundary: not connected");
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            ViewMode::Continuous => {
                ui.heading(&self.session.project().metadata.title);
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| self.continuous_view(ui));
            }
            ViewMode::Pages => self.page_view(ui),
        });
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
    fn new_app_has_editable_core_element() {
        let result = ScriptCraftApp::new();
        assert!(result.is_ok());
        if let Ok(app) = result {
            assert_eq!(app.session.project().elements.len(), 1);
            assert_eq!(app.session.project().elements[0].role, Role::Action);
            assert_eq!(app.editor_text, "");
        }
    }

    #[test]
    fn text_adapter_routes_replacement_through_session() {
        let result = ScriptCraftApp::new();
        assert!(result.is_ok());
        if let Ok(mut app) = result {
            app.editor_text = "INT. OFFICE - DAY".into();
            app.replace_selected_text();
            assert_eq!(app.session.project().elements[0].text, "INT. OFFICE - DAY");
            assert!(app.session.dirty());
            assert!(app.session.can_undo());
        }
    }
}
