//! Ordered keyboard, text, paste, and Beat-local IME event routing.

use super::text_state::BeatTextState;
use eframe::egui::{self, Key};
use scriptcraft_commands::{BeatCardIdValue, BeatNavigationValue, BeatSelectionValue, BeatTextEditValue, BeatTextFieldValue, BeatTextPositionValue, Command, HorizontalDirection, LineEdge};
use scriptcraft_layout::{BeatCardLayout, Caret};
use scriptcraft_session::editing::{BeatSelection, BeatTextField, BeatTextPosition};

fn selection_command(card: &scriptcraft_model::StableId, field: BeatTextField, anchor: BeatTextPosition, focus: BeatTextPosition) -> Command {
    Command::BeatBoardSelectionSet { selection: BeatSelectionValue::Text {
        card: BeatCardIdValue::from_core(card.clone()),
        field: match field { BeatTextField::Title => BeatTextFieldValue::Title, BeatTextField::Body => BeatTextFieldValue::Body },
        anchor: BeatTextPositionValue { paragraph: anchor.paragraph as u32, offset: anchor.offset as u32 },
        focus: BeatTextPositionValue { paragraph: focus.paragraph as u32, offset: focus.offset as u32 },
    }}
}

pub(super) fn handle(
    ctx: &egui::Context,
    selection: &BeatSelection,
    board: &scriptcraft_model::BeatBoard,
    layout: Option<&BeatCardLayout>,
    state: &mut BeatTextState,
) -> Vec<Command> {
        if !state.focused { return Vec::new(); }
    let Some((card_id, field)) = state.field.clone() else { return Vec::new() };
    let Some(card) = board.cards.iter().find(|card| card.id == card_id) else { state.blur(); return Vec::new() };
    let Some((_, _, anchor, focus)) = (match selection { BeatSelection::Text { card, field, anchor, focus } if card == &card_id => Some((card, field, anchor, focus)), _ => None }) else { return Vec::new() };
    let (events, modifiers, now) = ctx.input(|input| (input.events.clone(), input.modifiers, input.time));
    let ime_frame = events.iter().any(|event| matches!(event, egui::Event::Ime(_)));
    let mut commands = Vec::new();
    for (index, event) in events.iter().enumerate() {
        match event {
            egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                state.ime_active = true;
                state.ime_preedit.clone_from(text);
                ctx.request_repaint();
            }
            egui::Event::Ime(egui::ImeEvent::Enabled) => state.ime_active = true,
            egui::Event::Ime(egui::ImeEvent::Disabled) => { state.ime_active = false; state.ime_preedit.clear(); }
            egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                state.ime_active = false;
                state.ime_preedit.clear();
                state.preferred_x = None;
                if !text.is_empty() {
                    state.suppress_text_commit = Some(text.clone());
                    commands.push(Command::BeatBoardTextEdit { operation: if text.contains('\r') || text.contains('\n') {
                        BeatTextEditValue::Paste { text: text.clone() }
                    } else { BeatTextEditValue::Insert { text: text.clone() } } });
                }
            }
            egui::Event::Text(text) if !text.is_empty() => {
                if state.suppress_text_commit.as_ref() == Some(text) {
                    state.suppress_text_commit = None;
                } else if !state.ime_active && !ime_frame && !modifiers.command && !(modifiers.ctrl && !cfg!(target_os = "macos"))
                    && text.chars().all(|character| !character.is_control())
                    && !events[index + 1..].iter().any(|event| matches!(event, egui::Event::Ime(egui::ImeEvent::Commit(commit)) if commit == text)) {
                    commands.push(Command::BeatBoardTextEdit { operation: BeatTextEditValue::Insert { text: text.clone() } });
                    state.preferred_x = None;
                }
            }
            egui::Event::Paste(text) if !text.is_empty() => {
                state.preferred_x = None;
                commands.push(Command::BeatBoardTextEdit { operation: BeatTextEditValue::Paste { text: text.clone() } });
            }
            egui::Event::Key { key, pressed: true, modifiers: key_modifiers, .. } => {
                if state.ime_active || ime_frame { continue; }
                let shortcut = key_modifiers.command || (key_modifiers.ctrl && !cfg!(target_os = "macos"));
                if shortcut {
                    match key {
                        Key::A => commands.push(Command::EditSelectAll),
                        Key::C => commands.push(Command::EditCopy),
                        Key::X => commands.push(Command::EditCut),
                        Key::V => commands.push(Command::EditPaste),
                        _ => {}
                    }
                    continue;
                }
                match key {
                    Key::Escape => {
                        if let Some(id) = state.viewport_id { ctx.memory_mut(|memory| memory.surrender_focus(id)); }
                        state.blur();
                        commands.push(Command::BeatBoardSelectionSet { selection: BeatSelectionValue::Card { card: BeatCardIdValue::from_core(card_id.clone()) } });
                        break;
                    }
                    Key::Backspace => { state.preferred_x = None; commands.push(Command::BeatBoardTextEdit { operation: BeatTextEditValue::DeleteBackward }); }
                    Key::Delete => { state.preferred_x = None; commands.push(Command::BeatBoardTextEdit { operation: BeatTextEditValue::DeleteForward }); }
                    Key::Enter => { state.preferred_x = None; commands.push(Command::BeatBoardTextEdit { operation: BeatTextEditValue::Enter }); }
                    Key::ArrowLeft => { state.preferred_x = None; commands.push(Command::BeatBoardNavigation { target: BeatNavigationValue::Horizontal { direction: HorizontalDirection::Left }, extend: key_modifiers.shift }); }
                    Key::ArrowRight => { state.preferred_x = None; commands.push(Command::BeatBoardNavigation { target: BeatNavigationValue::Horizontal { direction: HorizontalDirection::Right }, extend: key_modifiers.shift }); }
                    Key::Home | Key::End => {
                        let edge = if *key == Key::Home { LineEdge::Start } else { LineEdge::End };
                        let target = if let Some(layout) = layout {
                            match field {
                                BeatTextField::Title => BeatTextPosition { paragraph: 0, offset: if edge == LineEdge::Start { 0 } else { card.title.len() } },
                                BeatTextField::Body => {
                                    let line_index = layout.body_lines.iter().position(|line| line.element == focus.paragraph && line.source.start <= focus.offset && focus.offset <= line.source.end).unwrap_or(0);
                                    let line = &layout.body_lines[line_index];
                                    let offset = if edge == LineEdge::Start { line.source.start } else { line.source.end };
                                    BeatTextPosition { paragraph: focus.paragraph, offset }
                                }
                            }
                        } else { *focus };
                        commands.push(selection_command(&card_id, field, if key_modifiers.shift { *anchor } else { target }, target));
                    }
                    Key::ArrowUp | Key::ArrowDown if field == BeatTextField::Body => {
                        if let Some(layout) = layout {
                            let line_index = layout.body_lines.iter().position(|line| line.element == focus.paragraph && line.source.start <= focus.offset && focus.offset <= line.source.end).unwrap_or(0);
                            let line = &layout.body_lines[line_index];
                            let caret = layout.body_caret_geometry(line_index, &Caret { element: focus.paragraph, offset: focus.offset }).ok();
                            if let Some(caret) = caret {
                                let x = state.preferred_x.get_or_insert(caret.x.to_points_f32());
                                let delta: isize = if *key == Key::ArrowUp { -1 } else { 1 };
                                let target_index = (line_index as isize + delta).clamp(0, layout.body_lines.len().saturating_sub(1) as isize) as usize;
                                let target = layout.body_caret_for_x(target_index, scriptcraft_model::formatting::FixedPoint::from_points_f64(f64::from(*x)).unwrap_or_default());
                                if let Some(target) = target {
                                    let position = BeatTextPosition { paragraph: target.element, offset: target.offset };
                                    commands.push(selection_command(&card_id, field, if key_modifiers.shift { *anchor } else { position }, position));
                                }
                            }
                            let _ = line;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    state.suppress_text_commit = None;
    if !commands.is_empty() || state.ime_active { state.caret_reset_time = now; }
    commands
}
