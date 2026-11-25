use crate::app::ScriptCraftApp;
use eframe::egui;
use scriptcraft_commands::{
    Command, CommandOutcome, CommandRejection, EditorSurface, ScriptNoteTextEditValue,
    ScriptNoteTextOperationValue, ScriptNoteTimestampValue,
};
use scriptcraft_model::Project;
use scriptcraft_session::editing::Selection;

use super::DesktopCommandOutcome;

impl ScriptCraftApp {
    pub(super) fn dispatch_clipboard_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        if self.script_note_workspace.focused
            && self.script_note_workspace.viewport_focused
            && self.session.script_note_selection_is_valid()
            && matches!(self.session.script_note_selection(),
                scriptcraft_session::editing::ScriptNoteSelection::Text { note, .. }
                    if self.script_note_workspace.open_note.as_ref() == Some(note))
        {
            return self.dispatch_script_note_clipboard_command(command, ctx);
        }
        if self.root_document_view == crate::app::RootDocumentView::BeatBoard && self.beat_text.focused {
            return self.dispatch_beat_clipboard_command(command, ctx);
        }
        match command {
            Command::EditCopy => {
                let selection = self.active_selection();
                match canonical_selection_text(
                    self.session.project(),
                    selection,
                    self.active_editor_surface(),
                ) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
                            self.clear_error();
                        }
                        Err(ClipboardAccessError::Unavailable) => self
                            .set_clipboard_notice("Clipboard write failed; screenplay unchanged"),
                    },
                    None => self.set_clipboard_notice(
                        "Copy unavailable: selection could not be read; screenplay unchanged",
                    ),
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::EditCut => {
                let selection = self.active_selection();
                match canonical_selection_text(
                    self.session.project(),
                    selection,
                    self.active_editor_surface(),
                ) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
                            let outcome = self.dispatch_internal_command(Command::EditDelete, ctx);
                            if matches!(
                                &outcome,
                                DesktopCommandOutcome::Editor(
                                    CommandOutcome::Changed | CommandOutcome::NoOp
                                )
                            ) {
                                self.clear_error();
                            }
                            outcome
                        }
                        Err(ClipboardAccessError::Unavailable) => {
                            self.set_clipboard_notice(
                                "Clipboard write failed; screenplay unchanged",
                            );
                            ctx.request_repaint();
                            DesktopCommandOutcome::EffectDispatched
                        }
                    },
                    None => {
                        self.set_clipboard_notice(
                            "Cut unavailable: selection could not be read; screenplay unchanged",
                        );
                        ctx.request_repaint();
                        DesktopCommandOutcome::EffectDispatched
                    }
                }
            }
            Command::EditPaste => match DesktopClipboard::read_text() {
                Ok(text) if text.is_empty() => {
                    self.set_clipboard_notice(
                        "Paste unavailable: clipboard text is empty; screenplay unchanged",
                    );
                    ctx.request_repaint();
                    DesktopCommandOutcome::EffectDispatched
                }
                Ok(text) => {
                    let outcome = self.dispatch_internal_command(
                        Command::EditorTextPaste {
                            text,
                            surface: self.active_editor_surface(),
                        },
                        ctx,
                    );
                    if matches!(
                        &outcome,
                        DesktopCommandOutcome::Editor(
                            CommandOutcome::Changed | CommandOutcome::NoOp
                        )
                    ) {
                        self.clear_error();
                    }
                    outcome
                }
                Err(ClipboardAccessError::Unavailable) => {
                    self.set_clipboard_notice(
                        "Paste unavailable: clipboard has no readable text; screenplay unchanged",
                    );
                    ctx.request_repaint();
                    DesktopCommandOutcome::EffectDispatched
                }
            },
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    fn dispatch_script_note_clipboard_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        match command {
            Command::EditCopy | Command::EditCut => {
                let text = self.session.selected_script_note_text().ok().flatten();
                let Some(text) = text.filter(|text| !text.is_empty()) else {
                    self.set_clipboard_notice("Clipboard action unavailable: no ScriptNote text selection; note unchanged");
                    ctx.request_repaint();
                    return DesktopCommandOutcome::EffectDispatched;
                };
                match DesktopClipboard::write_text(&text) {
                    Ok(()) if matches!(command, Command::EditCut) => {
                        match ScriptNoteTimestampValue::new(scriptcraft_fdx::current_timestamp()).ok() {
                            Some(modified) => self.dispatch_internal_command(
                                Command::ScriptNoteTextEdit {
                                    edit: ScriptNoteTextEditValue {
                                        operation: ScriptNoteTextOperationValue::DeleteForward,
                                        modified,
                                    },
                                },
                                ctx,
                            ),
                            None => {
                                self.set_clipboard_notice("ScriptNote timestamp could not be represented; note unchanged");
                                ctx.request_repaint();
                                DesktopCommandOutcome::EffectDispatched
                            }
                        }
                    }
                    Ok(()) => { self.clear_error(); ctx.request_repaint(); DesktopCommandOutcome::EffectDispatched }
                    Err(ClipboardAccessError::Unavailable) => {
                        self.set_clipboard_notice("Clipboard write failed; ScriptNote unchanged");
                        ctx.request_repaint();
                        DesktopCommandOutcome::EffectDispatched
                    }
                }
            }
            Command::EditPaste => match DesktopClipboard::read_text() {
                Ok(text) if text.is_empty() => {
                    self.set_clipboard_notice("Paste unavailable: clipboard text is empty; ScriptNote unchanged");
                    ctx.request_repaint(); DesktopCommandOutcome::EffectDispatched
                }
                Ok(text) => {
                    match ScriptNoteTimestampValue::new(scriptcraft_fdx::current_timestamp()).ok() {
                        Some(modified) => self.dispatch_internal_command(
                            Command::ScriptNoteTextEdit {
                                edit: ScriptNoteTextEditValue {
                                    operation: ScriptNoteTextOperationValue::Paste { text },
                                    modified,
                                },
                            },
                            ctx,
                        ),
                        None => {
                            self.set_clipboard_notice("ScriptNote timestamp could not be represented; note unchanged");
                            ctx.request_repaint();
                            DesktopCommandOutcome::EffectDispatched
                        }
                    }
                }
                Err(ClipboardAccessError::Unavailable) => {
                    self.set_clipboard_notice("Paste unavailable: clipboard has no readable text; ScriptNote unchanged");
                    ctx.request_repaint(); DesktopCommandOutcome::EffectDispatched
                }
            },
            other => DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id())),
        }
    }

    fn dispatch_beat_clipboard_command(&mut self, command: Command, ctx: &egui::Context) -> DesktopCommandOutcome {
        use scriptcraft_commands::{BeatTextEditValue, CommandOutcome};
        match command {
            Command::EditCopy | Command::EditCut => {
                let text = self.session.selected_beat_text().ok().flatten();
                let Some(text) = text.filter(|text| !text.is_empty()) else {
                    self.set_clipboard_notice("Clipboard action unavailable: no Beat text selection; document unchanged");
                    ctx.request_repaint();
                    return DesktopCommandOutcome::EffectDispatched;
                };
                match DesktopClipboard::write_text(&text) {
                    Ok(()) if matches!(command, Command::EditCut) => {
                        let result = self.dispatch_internal_command(Command::BeatBoardTextEdit { operation: BeatTextEditValue::DeleteForward }, ctx);
                        if matches!(result, DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp)) { self.clear_error(); }
                        result
                    }
                    Ok(()) => { self.clear_error(); ctx.request_repaint(); DesktopCommandOutcome::EffectDispatched }
                    Err(ClipboardAccessError::Unavailable) => {
                        self.set_clipboard_notice("Clipboard write failed; Beat text unchanged");
                        ctx.request_repaint();
                        DesktopCommandOutcome::EffectDispatched
                    }
                }
            }
            Command::EditPaste => match DesktopClipboard::read_text() {
                Ok(text) if text.is_empty() => {
                    self.set_clipboard_notice("Paste unavailable: clipboard text is empty; Beat text unchanged");
                    ctx.request_repaint(); DesktopCommandOutcome::EffectDispatched
                }
                Ok(text) => self.dispatch_internal_command(Command::BeatBoardTextEdit { operation: BeatTextEditValue::Paste { text } }, ctx),
                Err(ClipboardAccessError::Unavailable) => {
                    self.set_clipboard_notice("Paste unavailable: clipboard has no readable text; Beat text unchanged");
                    ctx.request_repaint(); DesktopCommandOutcome::EffectDispatched
                }
            },
            other => DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id())),
        }
    }

    fn set_clipboard_notice(&mut self, message: &str) {
        self.show_error(message);
    }
}

#[derive(Debug, Clone, Copy)]
enum ClipboardAccessError {
    Unavailable,
}

/// The single desktop-owned boundary for explicit system clipboard access.
struct DesktopClipboard;

impl DesktopClipboard {
    fn write_text(text: &str) -> Result<(), ClipboardAccessError> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|_| ClipboardAccessError::Unavailable)?;
        clipboard
            .set_text(text.to_owned())
            .map_err(|_| ClipboardAccessError::Unavailable)
    }

    fn read_text() -> Result<String, ClipboardAccessError> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|_| ClipboardAccessError::Unavailable)?;
        clipboard
            .get_text()
            .map_err(|_| ClipboardAccessError::Unavailable)
    }
}

/// Extracts the ordered selection directly from canonical body-element UTF-8.
/// Each crossed element boundary contributes exactly one newline; content is
/// otherwise copied byte-for-byte without export or layout transformations.
fn canonical_selection_text(
    project: &Project,
    selection: Selection,
    surface: EditorSurface,
) -> Option<String> {
    let (start, end) = if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    };
    if start == end {
        return None;
    }

    let paragraph_text = |index: usize| match surface {
        EditorSurface::Body => project
            .elements
            .get(index)
            .map(|element| element.text.as_str()),
        EditorSurface::TitlePage => project
            .title_page
            .get(index)
            .map(|paragraph| paragraph.text.as_str()),
    };
    let start_text = paragraph_text(start.element)?;
    if start.element == end.element {
        return Some(start_text.get(start.offset..end.offset)?.to_owned());
    }

    let mut text = String::new();
    text.push_str(start_text.get(start.offset..)?);
    for element_index in (start.element + 1)..=end.element {
        text.push('\n');
        let paragraph = paragraph_text(element_index)?;
        if element_index == end.element {
            text.push_str(paragraph.get(..end.offset)?);
        } else {
            text.push_str(paragraph);
        }
    }
    Some(text)
}
