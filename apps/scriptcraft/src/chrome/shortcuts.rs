//! Shared logical shortcut definitions plus the focused editor binding resolver.

use super::element_shortcuts::ElementEditorBinding;
use eframe::egui;
use scriptcraft_commands::{Command, EditorSurface, ElementTypeValue};

/// Platform is explicit so the focused editor resolver can be exercised for
/// either supported desktop platform independently of the host OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutPlatform {
    MacOS,
    Windows,
    Other,
}

impl ShortcutPlatform {
    pub(crate) const fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::MacOS
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Self::Other
        }
    }
}

/// Display/native accelerator metadata. `primary` means Command on macOS and
/// Control on Windows; `control` is the additional literal modifier slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LogicalShortcut {
    pub(crate) key: ShortcutKey,
    pub(crate) primary: bool,
    pub(crate) control: bool,
    pub(crate) option: bool,
    pub(crate) shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ShortcutKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Comma,
    Semicolon,
    Colon,
    Period,
    Enter,
    Tab,
    Backspace,
    Delete,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    BracketLeft,
    BracketRight,
    Backslash,
    Pipe,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    Home,
    End,
}

pub(super) const fn primary_shortcut(key: ShortcutKey, shift: bool) -> LogicalShortcut {
    LogicalShortcut {
        key,
        primary: true,
        control: false,
        option: false,
        shift,
    }
}

pub(super) const fn display_shortcut(
    key: ShortcutKey,
    primary: bool,
    control: bool,
    option: bool,
    shift: bool,
) -> LogicalShortcut {
    LogicalShortcut {
        key,
        primary,
        control,
        option,
        shift,
    }
}

pub(super) fn key_for_shortcut(key: ShortcutKey) -> Option<egui::Key> {
    Some(match key {
        ShortcutKey::A => egui::Key::A,
        ShortcutKey::B => egui::Key::B,
        ShortcutKey::C => egui::Key::C,
        ShortcutKey::D => egui::Key::D,
        ShortcutKey::E => egui::Key::E,
        ShortcutKey::F => egui::Key::F,
        ShortcutKey::G => egui::Key::G,
        ShortcutKey::H => egui::Key::H,
        ShortcutKey::I => egui::Key::I,
        ShortcutKey::J => egui::Key::J,
        ShortcutKey::K => egui::Key::K,
        ShortcutKey::L => egui::Key::L,
        ShortcutKey::M => egui::Key::M,
        ShortcutKey::N => egui::Key::N,
        ShortcutKey::O => egui::Key::O,
        ShortcutKey::P => egui::Key::P,
        ShortcutKey::Q => egui::Key::Q,
        ShortcutKey::R => egui::Key::R,
        ShortcutKey::S => egui::Key::S,
        ShortcutKey::T => egui::Key::T,
        ShortcutKey::U => egui::Key::U,
        ShortcutKey::V => egui::Key::V,
        ShortcutKey::W => egui::Key::W,
        ShortcutKey::X => egui::Key::X,
        ShortcutKey::Y => egui::Key::Y,
        ShortcutKey::Z => egui::Key::Z,
        ShortcutKey::Comma => return None,
        ShortcutKey::Semicolon => return None,
        ShortcutKey::Colon => return None,
        ShortcutKey::Period => return None,
        ShortcutKey::Enter => egui::Key::Enter,
        ShortcutKey::Tab => egui::Key::Tab,
        ShortcutKey::Backspace => egui::Key::Backspace,
        ShortcutKey::Delete => egui::Key::Delete,
        ShortcutKey::Digit0 => egui::Key::Num0,
        ShortcutKey::Digit1 => egui::Key::Num1,
        ShortcutKey::Digit2 => egui::Key::Num2,
        ShortcutKey::Digit3 => egui::Key::Num3,
        ShortcutKey::Digit4 => egui::Key::Num4,
        ShortcutKey::Digit5 => egui::Key::Num5,
        ShortcutKey::Digit6 => egui::Key::Num6,
        ShortcutKey::Digit7 => egui::Key::Num7,
        ShortcutKey::Digit8 => egui::Key::Num8,
        ShortcutKey::Digit9 => egui::Key::Num9,
        ShortcutKey::BracketLeft => return None,
        ShortcutKey::BracketRight => return None,
        ShortcutKey::Backslash => return None,
        ShortcutKey::Pipe => return None,
        ShortcutKey::ArrowRight => egui::Key::ArrowRight,
        ShortcutKey::ArrowLeft => egui::Key::ArrowLeft,
        ShortcutKey::ArrowUp => egui::Key::ArrowUp,
        ShortcutKey::ArrowDown => egui::Key::ArrowDown,
        ShortcutKey::Home => egui::Key::Home,
        ShortcutKey::End => egui::Key::End,
    })
}

pub(super) fn shortcut_matches(
    shortcut: LogicalShortcut,
    key: egui::Key,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
) -> bool {
    key_for_shortcut(shortcut.key) == Some(key)
        && shortcut_modifiers_match(shortcut, modifiers, platform)
}

fn literal_control_modifier(modifiers: egui::Modifiers, platform: ShortcutPlatform) -> bool {
    match platform {
        ShortcutPlatform::MacOS => modifiers.ctrl,
        ShortcutPlatform::Windows | ShortcutPlatform::Other => modifiers.mac_cmd,
    }
}

pub(crate) fn shortcut_modifiers_match(
    shortcut: LogicalShortcut,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
) -> bool {
    modifiers.alt == shortcut.option
        && literal_control_modifier(modifiers, platform) == shortcut.control
        && modifiers.command == shortcut.primary
        && modifiers.shift == shortcut.shift
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementHintChord {
    Base,
    Extended,
}

/// Status lists are limited to the documented primary and extended modifier
/// shapes; they are populated only by current enabled bindings.
pub(crate) fn element_hint_chord(
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
) -> Option<ElementHintChord> {
    let base = primary_shortcut(ShortcutKey::Digit0, false);
    let extended = match platform {
        ShortcutPlatform::MacOS => LogicalShortcut {
            control: true,
            ..base
        },
        ShortcutPlatform::Windows => LogicalShortcut {
            shift: true,
            ..base
        },
        ShortcutPlatform::Other => return None,
    };
    if shortcut_modifiers_match(base, modifiers, platform) {
        Some(ElementHintChord::Base)
    } else if shortcut_modifiers_match(extended, modifiers, platform) {
        Some(ElementHintChord::Extended)
    } else {
        None
    }
}

pub(crate) fn binding_uses_hint_chord(
    binding: &ElementEditorBinding,
    hint: ElementHintChord,
    platform: ShortcutPlatform,
) -> bool {
    let base = primary_shortcut(ShortcutKey::Digit0, false);
    let expected = match hint {
        ElementHintChord::Base => base,
        ElementHintChord::Extended => match platform {
            ShortcutPlatform::MacOS => LogicalShortcut {
                control: true,
                ..base
            },
            ShortcutPlatform::Windows => LogicalShortcut {
                shift: true,
                ..base
            },
            ShortcutPlatform::Other => return false,
        },
    };
    binding.chord.primary == expected.primary
        && binding.chord.control == expected.control
        && binding.chord.option == expected.option
        && binding.chord.shift == expected.shift
}

pub(crate) fn element_shortcut_label(
    shortcut: LogicalShortcut,
    platform: ShortcutPlatform,
) -> String {
    let mut parts = Vec::with_capacity(4);
    if shortcut.primary {
        parts.push(match platform {
            ShortcutPlatform::MacOS => "Cmd",
            ShortcutPlatform::Windows | ShortcutPlatform::Other => "Ctrl",
        });
    }
    if shortcut.control {
        parts.push("Control");
    }
    if shortcut.option {
        parts.push("Alt");
    }
    if shortcut.shift {
        parts.push("Shift");
    }
    parts.push(match shortcut.key {
        ShortcutKey::A => "A",
        ShortcutKey::B => "B",
        ShortcutKey::C => "C",
        ShortcutKey::D => "D",
        ShortcutKey::E => "E",
        ShortcutKey::F => "F",
        ShortcutKey::G => "G",
        ShortcutKey::H => "H",
        ShortcutKey::I => "I",
        ShortcutKey::J => "J",
        ShortcutKey::K => "K",
        ShortcutKey::L => "L",
        ShortcutKey::M => "M",
        ShortcutKey::N => "N",
        ShortcutKey::O => "O",
        ShortcutKey::P => "P",
        ShortcutKey::Q => "Q",
        ShortcutKey::R => "R",
        ShortcutKey::S => "S",
        ShortcutKey::T => "T",
        ShortcutKey::U => "U",
        ShortcutKey::V => "V",
        ShortcutKey::W => "W",
        ShortcutKey::X => "X",
        ShortcutKey::Y => "Y",
        ShortcutKey::Z => "Z",
        ShortcutKey::Digit0 => "0",
        ShortcutKey::Digit1 => "1",
        ShortcutKey::Digit2 => "2",
        ShortcutKey::Digit3 => "3",
        ShortcutKey::Digit4 => "4",
        ShortcutKey::Digit5 => "5",
        ShortcutKey::Digit6 => "6",
        ShortcutKey::Digit7 => "7",
        ShortcutKey::Digit8 => "8",
        ShortcutKey::Digit9 => "9",
        ShortcutKey::Semicolon | ShortcutKey::Colon => ";",
        ShortcutKey::Comma => ",",
        ShortcutKey::Period => ".",
        ShortcutKey::Enter => "Enter",
        ShortcutKey::Tab => "Tab",
        ShortcutKey::Backspace => "Backspace",
        ShortcutKey::Delete => "Delete",
        ShortcutKey::ArrowRight => "ArrowRight",
        ShortcutKey::ArrowLeft => "ArrowLeft",
        ShortcutKey::ArrowUp => "ArrowUp",
        ShortcutKey::ArrowDown => "ArrowDown",
        ShortcutKey::Home => "Home",
        ShortcutKey::End => "End",
        ShortcutKey::BracketLeft => "[",
        ShortcutKey::BracketRight => "]",
        ShortcutKey::Backslash => "\\",
        ShortcutKey::Pipe => "|",
    });
    parts.join("+")
}

/// Resolves editor-local bindings without consulting the menu catalog. The
/// caller supplies a pressed key event and suspends this resolver during IME.
pub(crate) fn editor_command_for_key_event(
    key: egui::Key,
    physical_key: Option<egui::Key>,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
    bindings: &[ElementEditorBinding],
) -> Option<Command> {
    match key {
        // Deletion key modifiers have historically not altered their editing
        // operation; retain that behavior while routing through typed commands.
        egui::Key::Backspace => return Some(Command::EditorDeleteBackward),
        egui::Key::Delete => return Some(Command::EditDelete),
        egui::Key::Enter if modifiers_are_none(modifiers) => {
            return Some(Command::EditorParagraphEnter)
        }
        egui::Key::Tab if modifiers_are_none(modifiers) => {
            return Some(Command::EditorParagraphTab)
        }
        _ => {}
    }

    // Egui's logical key is the requested top-row digit. Requiring the same
    // physical key avoids treating a keypad-produced logical digit as an alias.
    if physical_key != Some(key) {
        return None;
    }
    for binding in bindings {
        if key_for_shortcut(binding.chord.key) == Some(key)
            && shortcut_modifiers_match(binding.chord, modifiers, platform)
        {
            return Some(Command::EditorElementChange {
                element_type: ElementTypeValue::from_key(binding.element_type.clone()),
                surface: EditorSurface::Body,
            });
        }
    }
    None
}

/// Resolve one supported punctuation Text event before ordinary text insertion.
pub(crate) fn editor_command_for_text_event(
    text: &str,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
    bindings: &[ElementEditorBinding],
) -> Option<Command> {
    if text != ";" && text != ":" {
        return None;
    }
    let shift = text == ":";
    bindings.iter().find_map(|binding| {
        (binding.chord.key == ShortcutKey::Semicolon
            && binding.chord.shift == shift
            && shortcut_modifiers_match(binding.chord, modifiers, platform))
        .then(|| Command::EditorElementChange {
            element_type: ElementTypeValue::from_key(binding.element_type.clone()),
            surface: EditorSurface::Body,
        })
    })
}

/// Focused fallback for the Windows-owned in-window dual-dialogue accelerator.
/// macOS deliberately declines here because its native menu is the sole owner.
pub(crate) fn dual_dialogue_toggle_for_focused_key_event(
    key: egui::Key,
    physical_key: Option<egui::Key>,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
    in_window_shortcuts_owned: bool,
) -> Option<Command> {
    if !in_window_shortcuts_owned || key != egui::Key::D || physical_key != Some(key) {
        return None;
    }
    let shortcut = match platform {
        ShortcutPlatform::MacOS => primary_shortcut(ShortcutKey::D, false),
        ShortcutPlatform::Windows => LogicalShortcut {
            option: true,
            ..primary_shortcut(ShortcutKey::D, false)
        },
        ShortcutPlatform::Other => return None,
    };
    shortcut_matches(shortcut, key, modifiers, platform)
        .then_some(Command::EditorDualDialogueToggle)
}

pub(crate) fn modifiers_are_none(modifiers: egui::Modifiers) -> bool {
    !modifiers.alt
        && !modifiers.ctrl
        && !modifiers.command
        && !modifiers.mac_cmd
        && !modifiers.shift
}

pub(crate) const fn shortcut_input_allowed(ime_active: bool, ime_event_in_frame: bool) -> bool {
    !ime_active && !ime_event_in_frame
}

#[cfg(test)]
mod dynamic_editor_shortcut_tests {
    use super::*;
    use crate::chrome::element_shortcuts::ElementEditorBinding;
    use scriptcraft_model::ElementTypeKey;

    fn binding(
        label: &str,
        key: ShortcutKey,
        shift: bool,
    ) -> Result<ElementEditorBinding, scriptcraft_model::ModelError> {
        Ok(ElementEditorBinding {
            definition_ordinal: 0,
            element_type: ElementTypeKey::new(label)?,
            chord: LogicalShortcut {
                key,
                primary: true,
                control: false,
                option: false,
                shift,
            },
        })
    }

    fn modifiers(command: bool, ctrl: bool, shift: bool) -> egui::Modifiers {
        egui::Modifiers {
            command,
            ctrl,
            shift,
            ..egui::Modifiers::default()
        }
    }

    #[test]
    fn dynamic_named_keys_require_physical_match_and_exact_modifiers() {
        let Ok(binding) = binding("Lyrics", ShortcutKey::Digit2, false) else {
            assert!(false, "test open key should be valid");
            return;
        };
        let command = editor_command_for_key_event(
            egui::Key::Num2,
            Some(egui::Key::Num2),
            modifiers(true, false, false),
            ShortcutPlatform::MacOS,
            std::slice::from_ref(&binding),
        );
        assert!(matches!(
            command,
            Some(Command::EditorElementChange { element_type, .. })
                if element_type.as_str() == "Lyrics"
        ));
        assert!(editor_command_for_key_event(
            egui::Key::Num2,
            None,
            modifiers(true, false, false),
            ShortcutPlatform::MacOS,
            std::slice::from_ref(&binding),
        )
        .is_none());
        assert!(editor_command_for_key_event(
            egui::Key::Num2,
            Some(egui::Key::Num2),
            modifiers(true, false, true),
            ShortcutPlatform::MacOS,
            &[binding],
        )
        .is_none());
    }

    #[test]
    fn semicolon_and_colon_text_chords_are_distinct_and_consumable() {
        let Ok(semicolon) = binding("Lyrics", ShortcutKey::Semicolon, false) else {
            assert!(false, "test open key should be valid");
            return;
        };
        let Ok(colon) = binding("Notations", ShortcutKey::Semicolon, true) else {
            assert!(false, "test open key should be valid");
            return;
        };
        let bindings = [semicolon, colon];
        assert!(matches!(
            editor_command_for_text_event(
                ";",
                modifiers(true, false, false),
                ShortcutPlatform::MacOS,
                &bindings,
            ),
            Some(Command::EditorElementChange { element_type, .. })
                if element_type.as_str() == "Lyrics"
        ));
        assert!(matches!(
            editor_command_for_text_event(
                ":",
                modifiers(true, false, true),
                ShortcutPlatform::MacOS,
                &bindings,
            ),
            Some(Command::EditorElementChange { element_type, .. })
                if element_type.as_str() == "Notations"
        ));
        assert!(editor_command_for_text_event(
            ";",
            modifiers(true, false, true),
            ShortcutPlatform::MacOS,
            &bindings,
        )
        .is_none());
        assert!(editor_command_for_text_event(
            "é",
            modifiers(true, false, false),
            ShortcutPlatform::MacOS,
            &bindings,
        )
        .is_none());
    }

    #[test]
    fn fixed_deletion_ownership_precedes_dynamic_resolution() {
        let Ok(binding) = binding("Unsafe", ShortcutKey::Delete, false) else {
            assert!(false, "test open key should be valid");
            return;
        };
        assert!(matches!(
            editor_command_for_key_event(
                egui::Key::Delete,
                Some(egui::Key::Delete),
                modifiers(true, false, false),
                ShortcutPlatform::MacOS,
                &[binding],
            ),
            Some(Command::EditDelete)
        ));
    }

    #[test]
    fn ime_preedit_or_same_frame_ime_event_disables_shortcut_gate() {
        assert!(shortcut_input_allowed(false, false));
        assert!(!shortcut_input_allowed(true, false));
        assert!(!shortcut_input_allowed(false, true));
    }
}
