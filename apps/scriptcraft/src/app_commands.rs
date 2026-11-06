use super::{EditingSession, LayoutCache, Project, ScriptCraftApp, Selection};
use eframe::egui;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppCommand {
    New,
    Open,
    Close,
    Save,
    SaveAs,
    ExportPdf,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AppCommandAvailability {
    pub(super) modal_blocked: bool,
    pub(super) undo: bool,
    pub(super) redo: bool,
    pub(super) delete: bool,
    pub(super) select_all: bool,
    pub(super) cut: bool,
    pub(super) copy: bool,
    pub(super) paste: bool,
}

impl ScriptCraftApp {
    /// Returns egui focus to the one screenplay interaction surface after a
    /// menu Edit command, without making the command target depend on that focus.
    pub(super) fn restore_editor_focus(&mut self, ctx: &egui::Context) {
        if let Some(editor_id) = self.editor_surface_id {
            ctx.memory_mut(|memory| memory.request_focus(editor_id));
            ctx.request_repaint();
        }
    }

    pub(super) fn app_command_availability(&self) -> AppCommandAvailability {
        let migration_pending = self.pending_fdx_open.is_some();
        let selection = self.session.selection();
        let delete_target_exists = if selection.anchor != selection.focus {
            true
        } else {
            self.session
                .project()
                .elements
                .get(selection.focus.element)
                .is_some_and(|element| {
                    selection.focus.offset < element.text.len()
                        || (element.role == super::Role::Parenthetical
                            && matches!(element.text.as_str(), "(" | "()" | ")"))
                })
        };

        AppCommandAvailability {
            modal_blocked: migration_pending,
            undo: !migration_pending && self.session.can_undo(),
            redo: !migration_pending && self.session.can_redo(),
            delete: !migration_pending && delete_target_exists,
            // EditingSession projects a document with at least one body element.
            select_all: !migration_pending && !self.session.project().elements.is_empty(),
            cut: !migration_pending && selection.anchor != selection.focus,
            copy: !migration_pending && selection.anchor != selection.focus,
            // Clipboard state is read only when Paste is activated, never per frame.
            paste: !migration_pending,
        }
    }

    pub(super) fn dispatch_app_command(
        &mut self,
        command: AppCommand,
        ctx: &egui::Context,
    ) {
        let availability = self.app_command_availability();
        if !availability.is_enabled(command) {
            return;
        }
        match command {
            AppCommand::New => {
                match Project::new()
                    .map_err(|error| format!("new project failed: {error:?}"))
                    .and_then(|project| {
                        EditingSession::with_policy(project, self.session.policy())
                            .map_err(|error| format!("session failed: {error:?}"))
                    }) {
                    Ok(session) => {
                        self.session = session;
                        self.page_geometry.reset_for_document_replacement();
                        self.layout_cache = LayoutCache::default();
                        self.surface_focused = false;
                        self.native_path = None;
                        self.fdx_baseline = None;
                        self.pending_fdx_open = None;
                        self.next_cast_list_refresh =
                            Instant::now() + super::CAST_LIST_REFRESH_INTERVAL;
                        self.rendered_revision = None;
                        self.status = "New in-memory project".into();
                        self.clear_error();
                    }
                    Err(error) => self.show_error(error),
                }
            }
            AppCommand::Open => self.open_native(),
            AppCommand::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            AppCommand::Save => self.save_native(),
            AppCommand::SaveAs => self.save_as_native(),
            AppCommand::ExportPdf => self.export_pdf(),
            AppCommand::Undo if availability.undo => {
                let result = self.session.undo();
                self.mutate(
                    result,
                    "Undid last edit",
                    super::RevealIntent::Conditional,
                );
                ctx.request_repaint();
            }
            AppCommand::Redo if availability.redo => {
                let result = self.session.redo();
                self.mutate(
                    result,
                    "Redid last edit",
                    super::RevealIntent::Conditional,
                );
                ctx.request_repaint();
            }
            AppCommand::Delete if availability.delete => {
                let result = self.session.delete();
                self.mutate(
                    result,
                    "Deleted forward",
                    super::RevealIntent::Conditional,
                );
                ctx.request_repaint();
            }
            AppCommand::SelectAll if availability.select_all => {
                let end_element = self.session.project().elements.len().saturating_sub(1);
                let end = super::Position {
                    element: end_element,
                    offset: self.session.project().elements[end_element].text.len(),
                };
                let result = self.session.set_selection(super::Selection {
                    anchor: super::Position {
                        element: 0,
                        offset: 0,
                    },
                    focus: end,
                });
                self.mutate(
                    result,
                    "Selected all screenplay text",
                    super::RevealIntent::Conditional,
                );
                ctx.request_repaint();
            }
            AppCommand::Copy if availability.copy => {
                match canonical_selection_text(self.session.project(), self.session.selection()) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
                            self.status = "Copied selection to clipboard".into();
                            self.clear_error();
                        }
                        Err(ClipboardAccessError::Unavailable) => self.set_clipboard_notice(
                            "Clipboard write failed; screenplay unchanged",
                        ),
                    },
                    None => self.set_clipboard_notice(
                        "Copy unavailable: selection could not be read; screenplay unchanged",
                    ),
                }
                ctx.request_repaint();
            }
            AppCommand::Cut if availability.cut => {
                match canonical_selection_text(self.session.project(), self.session.selection()) {
                    Some(text) => match DesktopClipboard::write_text(&text) {
                        Ok(()) => {
                            let result = self.session.delete();
                            self.mutate(
                                result,
                                "Cut selection to clipboard",
                                super::RevealIntent::Conditional,
                            );
                        }
                        Err(ClipboardAccessError::Unavailable) => self.set_clipboard_notice(
                            "Clipboard write failed; screenplay unchanged",
                        ),
                    },
                    None => self.set_clipboard_notice(
                        "Cut unavailable: selection could not be read; screenplay unchanged",
                    ),
                }
                ctx.request_repaint();
            }
            AppCommand::Paste if availability.paste => {
                match DesktopClipboard::read_text() {
                    Ok(text) if text.is_empty() => self.set_clipboard_notice(
                        "Paste unavailable: clipboard text is empty; screenplay unchanged",
                    ),
                    Ok(text) => {
                        let result = self.session.insert_paste(&text);
                        self.mutate(
                            result,
                            "Pasted clipboard text",
                            super::RevealIntent::Conditional,
                        );
                    }
                    Err(ClipboardAccessError::Unavailable) => self.set_clipboard_notice(
                        "Paste unavailable: clipboard has no readable text; screenplay unchanged",
                    ),
                }
                ctx.request_repaint();
            }
            AppCommand::Undo
            | AppCommand::Redo
            | AppCommand::Delete
            | AppCommand::SelectAll
            | AppCommand::Cut
            | AppCommand::Copy
            | AppCommand::Paste => {}
        }
    }

    fn set_clipboard_notice(&mut self, message: &str) {
        self.status = message.into();
        self.error = Some(message.into());
    }
}

impl AppCommandAvailability {
    /// One command-state projection consumed by both menu presentations and
    /// checked again at dispatch time to reject stale or modal-blocked input.
    pub(super) const fn is_enabled(self, command: AppCommand) -> bool {
        if self.modal_blocked {
            return false;
        }
        match command {
            AppCommand::Undo => self.undo,
            AppCommand::Redo => self.redo,
            AppCommand::Delete => self.delete,
            AppCommand::SelectAll => self.select_all,
            AppCommand::Cut => self.cut,
            AppCommand::Copy => self.copy,
            AppCommand::Paste => self.paste,
            AppCommand::New
            | AppCommand::Open
            | AppCommand::Close
            | AppCommand::Save
            | AppCommand::SaveAs
            | AppCommand::ExportPdf => true,
        }
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
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|_| ClipboardAccessError::Unavailable)?;
        clipboard
            .set_text(text.to_owned())
            .map_err(|_| ClipboardAccessError::Unavailable)
    }

    fn read_text() -> Result<String, ClipboardAccessError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|_| ClipboardAccessError::Unavailable)?;
        clipboard
            .get_text()
            .map_err(|_| ClipboardAccessError::Unavailable)
    }
}

/// Extracts the ordered selection directly from canonical body-element UTF-8.
/// Each crossed element boundary contributes exactly one newline; content is
/// otherwise copied byte-for-byte without export or layout transformations.
fn canonical_selection_text(project: &Project, selection: Selection) -> Option<String> {
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

    let start_element = project.elements.get(start.element)?;
    if start.element == end.element {
        return Some(start_element.text.get(start.offset..end.offset)?.to_owned());
    }

    let mut text = String::new();
    text.push_str(start_element.text.get(start.offset..)?);
    for element_index in (start.element + 1)..=end.element {
        text.push('\n');
        let element = project.elements.get(element_index)?;
        if element_index == end.element {
            text.push_str(element.text.get(..end.offset)?);
        } else {
            text.push_str(&element.text);
        }
    }
    Some(text)
}
