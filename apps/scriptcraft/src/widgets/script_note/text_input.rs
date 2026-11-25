//! Ordered native-note key, text, paste, and IME event translation.

use super::state::ScriptNoteWorkspaceState;
use eframe::egui::{self, Key};
use scriptcraft_commands::{
    Command, HorizontalDirection, LineEdge, ScriptNoteFieldValue, ScriptNoteIdValue,
    ScriptNoteNavigationValue, ScriptNoteSelectionValue, ScriptNoteTextEditValue,
    ScriptNoteTextOperationValue, ScriptNoteTextPositionValue, ScriptNoteTimestampValue,
};
use scriptcraft_layout::{Caret, ScriptNoteLayout};
use scriptcraft_model::StableId;
use scriptcraft_session::editing::{ScriptNoteField, ScriptNoteSelection, ScriptNoteTextPosition};

#[derive(Debug, Default)]
pub(crate) struct InputResult {
    pub(crate) commands: Vec<Command>,
    pub(crate) timestamp_error: bool,
    pub(crate) dismiss: bool,
}

fn push_text_edit(result: &mut InputResult, operation: ScriptNoteTextOperationValue) {
    let modified = ScriptNoteTimestampValue::new(scriptcraft_fdx::current_timestamp()).ok();
    match modified {
        Some(modified) => result.commands.push(Command::ScriptNoteTextEdit {
            edit: ScriptNoteTextEditValue { operation, modified },
        }),
        None => result.timestamp_error = true,
    }
}

fn field_value(field: ScriptNoteField) -> ScriptNoteFieldValue {
    match field { ScriptNoteField::Title => ScriptNoteFieldValue::Title, ScriptNoteField::Body => ScriptNoteFieldValue::Body }
}

fn selection_command(note: &StableId, field: ScriptNoteField, anchor: ScriptNoteTextPosition, focus: ScriptNoteTextPosition) -> Command {
    Command::ScriptNoteSelectionSet { selection: ScriptNoteSelectionValue::Text {
        note: ScriptNoteIdValue::from_core(note.clone()), field: field_value(field),
        anchor: ScriptNoteTextPositionValue { paragraph: anchor.paragraph as u32, offset: anchor.offset as u32 },
        focus: ScriptNoteTextPositionValue { paragraph: focus.paragraph as u32, offset: focus.offset as u32 },
    }}
}

pub(crate) fn select_all(note: &StableId, field: ScriptNoteField, value: &scriptcraft_model::ScriptNote) -> Command {
    let end = match field {
        ScriptNoteField::Title => ScriptNoteTextPosition { paragraph: 0, offset: value.title.len() },
        ScriptNoteField::Body => {
            let paragraph = value.paragraphs.len().saturating_sub(1);
            ScriptNoteTextPosition { paragraph, offset: value.paragraphs.last().map_or(0, |item| item.text.len()) }
        }
    };
    selection_command(note, field, ScriptNoteTextPosition { paragraph: 0, offset: 0 }, end)
}

pub(crate) fn handle(
    ctx: &egui::Context,
    note: &StableId,
    value: &scriptcraft_model::ScriptNote,
    selection: &ScriptNoteSelection,
    layout: &ScriptNoteLayout,
    state: &mut ScriptNoteWorkspaceState,
) -> InputResult {
    let mut result = InputResult::default();
    if !state.focused { return result; }
    let Some(field) = state.field else { return result };
    let Some((selection_note, selection_field, anchor, focus)) = (match selection {
        ScriptNoteSelection::Text { note, field, anchor, focus } if note == &value.id => Some((note, field, anchor, focus)),
        _ => None,
    }) else { return result };
    if selection_note != note || *selection_field != field { return result; }
    let (events, modifiers, now) = ctx.input(|input| (input.events.clone(), input.modifiers, input.time));
    let ime_frame = events.iter().any(|event| matches!(event, egui::Event::Ime(_)));
    for (index, event) in events.iter().enumerate() {
        match event {
            egui::Event::Ime(egui::ImeEvent::Enabled) => state.ime_active = true,
            egui::Event::Ime(egui::ImeEvent::Disabled) => { state.ime_active = false; state.ime_preedit.clear(); }
            egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                state.ime_active = true;
                state.ime_preedit.clone_from(text);
                ctx.request_repaint();
            }
            egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                state.ime_active = false;
                state.ime_preedit.clear();
                if !text.is_empty() {
                    state.suppress_text_commit = Some(text.clone());
                    push_text_edit(&mut result, ScriptNoteTextOperationValue::Insert {
                        text: text.clone(),
                    });
                }
            }
            egui::Event::Text(text) if !text.is_empty() => {
                if state.suppress_text_commit.as_ref() == Some(text) {
                    state.suppress_text_commit = None;
                } else if !state.ime_active && !ime_frame && !modifiers.command
                    && !(modifiers.ctrl && !cfg!(target_os = "macos"))
                    && text.chars().all(|ch| !ch.is_control())
                    && !events[index + 1..].iter().any(|next| matches!(next, egui::Event::Ime(egui::ImeEvent::Commit(commit)) if commit == text))
                {
                    push_text_edit(&mut result, ScriptNoteTextOperationValue::Insert {
                        text: text.clone(),
                    });
                }
            }
            egui::Event::Paste(text)
                if !text.is_empty()
                    && !events.iter().any(|event| matches!(event,
                        egui::Event::Key { key: Key::V, pressed: true, modifiers, .. }
                            if modifiers.command || (modifiers.ctrl && !cfg!(target_os = "macos")))) =>
            {
                push_text_edit(
                    &mut result,
                    ScriptNoteTextOperationValue::Paste { text: text.clone() },
                );
            }
            egui::Event::Key { key, pressed: true, modifiers: key_modifiers, .. } => {
                if state.ime_active || ime_frame { continue; }
                let shortcut = key_modifiers.command || (key_modifiers.ctrl && !cfg!(target_os = "macos"));
                if shortcut {
                    match key {
                        Key::A => result.commands.push(select_all(note, field, value)),
                        Key::C => result.commands.push(Command::EditCopy),
                        Key::X => result.commands.push(Command::EditCut),
                        Key::V => result.commands.push(Command::EditPaste),
                        _ => {}
                    }
                    continue;
                }
                match key {
                    Key::Escape => { result.dismiss = true; break; }
                    Key::Backspace | Key::Delete | Key::Enter => {
                        if *key == Key::Enter && field == ScriptNoteField::Title {
                            let target = ScriptNoteTextPosition { paragraph: 0, offset: 0 };
                            result.commands.push(selection_command(note, ScriptNoteField::Body, target, target));
                            state.field = Some(ScriptNoteField::Body);
                        } else {
                            let operation = match key {
                                Key::Backspace => Some(ScriptNoteTextOperationValue::DeleteBackward),
                                Key::Delete => Some(ScriptNoteTextOperationValue::DeleteForward),
                                Key::Enter => Some(ScriptNoteTextOperationValue::Enter),
                                _ => None,
                            };
                            if let Some(operation) = operation {
                                push_text_edit(&mut result, operation);
                            }
                        }
                        state.preferred_x = None;
                    }
                    Key::ArrowLeft | Key::ArrowRight => {
                        state.preferred_x = None;
                        result.commands.push(Command::ScriptNoteTextNavigate {
                            target: ScriptNoteNavigationValue::Horizontal { direction: if *key == Key::ArrowLeft { HorizontalDirection::Left } else { HorizontalDirection::Right } },
                            extend: key_modifiers.shift,
                        });
                    }
                    Key::Home | Key::End => {
                        state.preferred_x = None;
                        let edge = if *key == Key::Home { LineEdge::Start } else { LineEdge::End };
                        let target = match field {
                            ScriptNoteField::Title => ScriptNoteTextPosition { paragraph: 0, offset: if edge == LineEdge::Start { 0 } else { value.title.len() } },
                            ScriptNoteField::Body => {
                                let paragraph = focus.paragraph;
                                let line_index = layout.body_lines.iter().position(|line| line.element == paragraph && line.source.start <= focus.offset && focus.offset <= line.source.end).unwrap_or(0);
                                let line = &layout.body_lines[line_index];
                                ScriptNoteTextPosition { paragraph, offset: if edge == LineEdge::Start { line.source.start } else { line.source.end } }
                            }
                        };
                        result.commands.push(selection_command(note, field, if key_modifiers.shift { *anchor } else { target }, target));
                    }
                    Key::ArrowUp | Key::ArrowDown if field == ScriptNoteField::Body => {
                        let line_index = layout.body_lines.iter().position(|line| line.element == focus.paragraph && line.source.start <= focus.offset && focus.offset <= line.source.end).unwrap_or(0);
                        let line = &layout.body_lines[line_index];
                        let caret = layout.body_caret_geometry(line_index, &Caret { element: focus.paragraph, offset: focus.offset }).ok();
                        if let Some(caret) = caret {
                            let x = *state.preferred_x.get_or_insert(caret.x.to_points_f32());
                            let target_index = if *key == Key::ArrowUp { line_index.saturating_sub(1) } else { (line_index + 1).min(layout.body_lines.len().saturating_sub(1)) };
                            if let Some(target) = layout.body_caret_for_x(target_index, scriptcraft_model::formatting::FixedPoint::from_points_f64(f64::from(x)).unwrap_or_default()) {
                                let position = ScriptNoteTextPosition { paragraph: target.element, offset: target.offset };
                                result.commands.push(selection_command(note, field, if key_modifiers.shift { *anchor } else { position }, position));
                            }
                        }
                        let _ = line;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    state.suppress_text_commit = None;
    if !result.commands.is_empty() || state.ime_active { state.caret_reset_time = now; }
    result
}
