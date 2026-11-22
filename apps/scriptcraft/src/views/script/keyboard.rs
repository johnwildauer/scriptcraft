//! Script View focused keyboard, text, paste, and IME routing.

use super::{zoom_scroll::has_nonzero_mouse_wheel, ScriptCraftApp};
use crate::app::add_performance_counter;
use crate::chrome::shortcuts::{
    editor_command_for_key_event, editor_command_for_text_event, shortcut_input_allowed,
    ShortcutPlatform,
};
use crate::commands::CommandInvocation;
use crate::views::script::RevealIntent;
use eframe::egui::{self, Key};
use scriptcraft_commands::{
    Command, CommandId, CommandOrigin, EditorSurface, HorizontalDirection, LineEdge,
    VerticalDirection,
};
use std::time::Instant;

impl ScriptCraftApp {
    fn dispatch_surface_events(
        &mut self,
        events: &[egui::Event],
        modifiers: egui::Modifiers,
        ctx: &egui::Context,
        surface: EditorSurface,
    ) {
        let started = Instant::now();
        let ime_event_in_frame = events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)));
        self.performance.events = self.performance.events.saturating_add(events.len() as u64);
        for (index, event) in events.iter().enumerate() {
            self.record_native_event(match event {
                egui::Event::Text(_) => "text",
                egui::Event::Paste(_) => "paste",
                egui::Event::Ime(egui::ImeEvent::Preedit(_)) => "ime-preedit",
                egui::Event::Ime(egui::ImeEvent::Commit(_)) => "ime-commit",
                egui::Event::Ime(egui::ImeEvent::Enabled) => "ime-enabled",
                egui::Event::Ime(egui::ImeEvent::Disabled) => "ime-disabled",
                egui::Event::Zoom(_) => "zoom",
                egui::Event::Key { pressed: true, .. } => "key-pressed",
                egui::Event::Key { pressed: false, .. } => "key-released",
                _ => "other",
            });
            let mut command = None;
            let result = match event {
                egui::Event::Zoom(factor) => {
                    if let Ok(factor) = scriptcraft_commands::ZoomFactor::new(*factor) {
                        command = Some(CommandInvocation::new(
                            Command::ViewZoomPinch { factor },
                            CommandOrigin::InternalFollowUp,
                        ));
                    }
                    None
                }
                egui::Event::Text(text) if !text.is_empty() => {
                    if shortcut_input_allowed(self.ime_active, ime_event_in_frame)
                        && self.dynamic_shortcuts_allowed(surface)
                    {
                        let controls = self.refresh_dynamic_controls();
                        command = editor_command_for_text_event(
                            text,
                            modifiers,
                            ShortcutPlatform::current(),
                            &controls.shortcuts.bindings,
                        )
                        .map(|command| CommandInvocation::new(command, CommandOrigin::Shortcut));
                    }
                    if command.is_none()
                        && !self.ime_active
                        && !events[index + 1..].iter().any(|next| {
                            matches!(next, egui::Event::Ime(egui::ImeEvent::Commit(commit)) if commit == text)
                        })
                        && !modifiers.command
                        && !(modifiers.ctrl && !cfg!(target_os = "macos"))
                        && text.chars().all(|character| !character.is_control())
                    {
                        command = Some(CommandInvocation::new(
                            Command::EditorTextInsert { text: text.clone(), surface },
                            CommandOrigin::Keyboard,
                        ));
                    }
                    None
                }
                egui::Event::Paste(text) if !text.is_empty() => {
                    command = Some(CommandInvocation::new(
                        Command::EditorTextPaste {
                            text: text.clone(),
                            surface,
                        },
                        CommandOrigin::Keyboard,
                    ));
                    None
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
                    if !text.is_empty() {
                        command = Some(CommandInvocation::new(
                            Command::EditorTextInsert {
                                text: text.clone(),
                                surface,
                            },
                            CommandOrigin::ImeCommit,
                        ));
                    }
                    None
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
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if crate::chrome::shortcuts::shortcut_input_allowed(
                        self.ime_active,
                        ime_event_in_frame,
                    ) {
                        if surface == EditorSurface::TitlePage
                            && self.menu_policy.in_window_shortcuts_owned()
                        {
                            command = crate::chrome::menu::command_for_key_event_command(
                                *key, *modifiers,
                            )
                            .map(|command| {
                                CommandInvocation::new(command, CommandOrigin::Shortcut)
                            });
                        }
                        if command.is_none() {
                            command = crate::chrome::shortcuts::dual_dialogue_toggle_for_focused_key_event(
                                *key,
                                *physical_key,
                                *modifiers,
                                ShortcutPlatform::current(),
                                self.menu_policy.in_window_shortcuts_owned(),
                            )
                            .filter(|_| surface == EditorSurface::Body)
                            .map(|command| {
                                CommandInvocation::new(command, CommandOrigin::Shortcut)
                            });
                        }
                        if command.is_none() {
                            let dynamic_enabled = self.dynamic_shortcuts_allowed(surface);
                            let controls = self.refresh_dynamic_controls();
                            let resolved_command = editor_command_for_key_event(
                                *key,
                                *physical_key,
                                *modifiers,
                                ShortcutPlatform::current(),
                                if dynamic_enabled {
                                    &controls.shortcuts.bindings
                                } else {
                                    &[]
                                },
                            );
                            command = resolved_command
                            .filter(|command| {
                                surface == EditorSurface::Body
                                    || !matches!(
                                        command,
                                        Command::EditorElementChange { .. }
                                            | Command::EditorParagraphTab
                                    )
                            })
                            .map(|command| {
                                let origin = match command.id() {
                                    CommandId::EditorParagraphEnter
                                    | CommandId::EditorParagraphTab
                                    | CommandId::EditorElementChange => CommandOrigin::Shortcut,
                                    _ => CommandOrigin::Keyboard,
                                };
                                CommandInvocation::new(command, origin)
                            });
                        }
                    }
                    if command.is_none() {
                        command =
                            self.navigation_key_event(*key, *modifiers, surface)
                                .map(|command| {
                                    CommandInvocation::new(command, CommandOrigin::Keyboard)
                                });
                    }
                    None
                }
                _ => continue,
            };
            if let Some(command) = command {
                self.dispatch_command(command, ctx);
                continue;
            }
            if let Some(result) = result {
                self.mutate(result, RevealIntent::Conditional);
            }
        }
        self.performance.event_nanos = self
            .performance
            .event_nanos
            .saturating_add(started.elapsed().as_nanos());
    }

    fn navigation_key_event(
        &self,
        key: Key,
        modifiers: egui::Modifiers,
        surface: EditorSurface,
    ) -> Option<Command> {
        match key {
            Key::ArrowLeft => Some(Command::EditorNavigationHorizontal {
                direction: HorizontalDirection::Left,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowRight => Some(Command::EditorNavigationHorizontal {
                direction: HorizontalDirection::Right,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowUp => Some(Command::EditorNavigationVertical {
                direction: VerticalDirection::Up,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowDown => Some(Command::EditorNavigationVertical {
                direction: VerticalDirection::Down,
                extend: modifiers.shift,
                surface,
            }),
            Key::Home => Some(Command::EditorNavigationLineEdge {
                edge: LineEdge::Start,
                extend: modifiers.shift,
                surface,
            }),
            Key::End => Some(Command::EditorNavigationLineEdge {
                edge: LineEdge::End,
                extend: modifiers.shift,
                surface: EditorSurface::Body,
            }),
            _ => None,
        }
    }

    fn dynamic_shortcuts_allowed(&mut self, surface: EditorSurface) -> bool {
        if !dynamic_shortcut_surface_allowed(
            surface,
            self.surface_focused,
            self.active_editor_surface(),
            self.replacement_open_pending(),
        ) {
            return false;
        }
        let availability = self.app_command_availability();
        dynamic_shortcut_availability_allowed(
            availability.modal_blocked,
            availability.replacement_pending,
            availability.native_output_active,
            availability.body_only,
        )
    }

    pub(crate) fn handle_surface_input(&mut self, ctx: &egui::Context, surface: EditorSurface) {
        if self.replacement_open_pending() {
            // Do not consume Body or Title Page semantic input while a selected
            // replacement is pending; the frame's events are intentionally not
            // queued for replay after a later safe resume.
            self.native_input_diagnostics.dispatch_enabled = false;
            return;
        }
        if self.element_windows.owns_input() {
            // Do not consume text, paste, navigation, shortcut, or IME events
            // while a root-context element window owns the keyboard. Leaving the
            // events in egui lets the topmost dialog handle its own controls.
            self.native_input_diagnostics.dispatch_enabled = false;
            return;
        }
        self.native_input_diagnostics.dispatch_enabled = self.surface_focused;
        if ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            // Explicit user scrolling wins over a not-yet-applied zoom request.
            // It must not be pulled back to the old viewport center afterward.
            if self.pending_zoom_anchor.is_some() {
                add_performance_counter(&mut self.performance.zoom_manual_scroll_cancellations, 1);
            }
            self.pending_zoom_anchor = None;
            self.reveal_intent = RevealIntent::None;
        }
        if !surface_input_is_dispatchable(
            self.surface_focused,
            self.active_editor_surface(),
            surface,
            self.element_windows.owns_input(),
        ) {
            return;
        }
        if self.menu_policy.in_window_shortcuts_owned()
            && crate::chrome::menu::in_window_menu_popup_open(ctx)
        {
            return;
        }
        let _ = self.refresh_dynamic_controls();
        self.native_input_diagnostics.dispatched_frames = self
            .native_input_diagnostics
            .dispatched_frames
            .saturating_add(1);
        let events = ctx.input(|input| input.events.clone());
        let modifiers = ctx.input(|input| input.modifiers);
        self.dispatch_surface_events(&events, modifiers, ctx, surface);
        if !events.is_empty() || ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            self.request_repaint(ctx);
        }
    }
}

fn surface_input_is_dispatchable(
    surface_focused: bool,
    active_surface: EditorSurface,
    requested_surface: EditorSurface,
    element_windows_own_input: bool,
) -> bool {
    !element_windows_own_input && surface_focused && active_surface == requested_surface
}

fn dynamic_shortcut_surface_allowed(
    requested_surface: EditorSurface,
    surface_focused: bool,
    active_surface: EditorSurface,
    replacement_pending: bool,
) -> bool {
    requested_surface == EditorSurface::Body
        && surface_focused
        && active_surface == EditorSurface::Body
        && !replacement_pending
}

fn dynamic_shortcut_availability_allowed(
    modal_blocked: bool,
    replacement_pending: bool,
    native_output_active: bool,
    body_only: bool,
) -> bool {
    !modal_blocked && !replacement_pending && !native_output_active && body_only
}
