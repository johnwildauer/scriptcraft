//! Shared logical shortcut definitions plus the focused editor binding resolver.

use eframe::egui;
use scriptcraft_commands::{Command, EditorSurface, RoleValue};
use scriptcraft_core::Role;

/// Platform is explicit so the focused editor resolver can be exercised for
/// either supported desktop platform independently of the host OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShortcutPlatform {
    MacOS,
    Windows,
    Other,
}

impl ShortcutPlatform {
    pub(super) const fn current() -> Self {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LogicalShortcut {
    pub(super) key: ShortcutKey,
    pub(super) primary: bool,
    pub(super) control: bool,
    pub(super) option: bool,
    pub(super) shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShortcutKey {
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

fn shortcut_modifiers_match(
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
pub(super) enum ElementHintChord {
    Base,
    Extended,
}

/// Recognize only the exact modifier shapes accepted by the editor binding
/// resolver so status guidance cannot advertise a rejected chord.
pub(super) fn element_hint_chord(
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

/// Project rows from the shared binding table. The extended role ordering is a
/// presentation-only sequence; every digit still comes from the binding table.
pub(super) fn element_hint_bindings(extended: bool) -> Vec<(u8, Role)> {
    let role_order: Vec<Role> = if extended {
        vec![
            Role::Summary,
            Role::Outline1,
            Role::Outline2,
            Role::Outline3,
            Role::Note,
            Role::Sequence,
            Role::EndOfAct,
        ]
    } else {
        EDITOR_ELEMENT_BINDINGS
            .iter()
            .filter(|binding| !binding.extended)
            .map(|binding| binding.role)
            .collect()
    };
    role_order
        .into_iter()
        .filter_map(|role| {
            EDITOR_ELEMENT_BINDINGS
                .iter()
                .find(|binding| binding.extended == extended && binding.role == role)
                .and_then(|binding| digit_value(binding.digit).map(|digit| (digit, role)))
        })
        .collect()
}

fn digit_value(digit: ShortcutKey) -> Option<u8> {
    Some(match digit {
        ShortcutKey::Digit0 => 0,
        ShortcutKey::Digit1 => 1,
        ShortcutKey::Digit2 => 2,
        ShortcutKey::Digit3 => 3,
        ShortcutKey::Digit4 => 4,
        ShortcutKey::Digit5 => 5,
        ShortcutKey::Digit6 => 6,
        ShortcutKey::Digit7 => 7,
        ShortcutKey::Digit8 => 8,
        ShortcutKey::Digit9 => 9,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy)]
struct ElementBinding {
    digit: ShortcutKey,
    role: Role,
    extended: bool,
}

const EDITOR_ELEMENT_BINDINGS: [ElementBinding; 17] = [
    ElementBinding { digit: ShortcutKey::Digit0, role: Role::General, extended: false },
    ElementBinding { digit: ShortcutKey::Digit1, role: Role::SceneHeading, extended: false },
    ElementBinding { digit: ShortcutKey::Digit2, role: Role::Action, extended: false },
    ElementBinding { digit: ShortcutKey::Digit3, role: Role::Character, extended: false },
    ElementBinding { digit: ShortcutKey::Digit4, role: Role::Parenthetical, extended: false },
    ElementBinding { digit: ShortcutKey::Digit5, role: Role::Dialogue, extended: false },
    ElementBinding { digit: ShortcutKey::Digit6, role: Role::Transition, extended: false },
    ElementBinding { digit: ShortcutKey::Digit7, role: Role::Shot, extended: false },
    ElementBinding { digit: ShortcutKey::Digit8, role: Role::CastList, extended: false },
    ElementBinding { digit: ShortcutKey::Digit9, role: Role::NewAct, extended: false },
    ElementBinding { digit: ShortcutKey::Digit8, role: Role::Sequence, extended: true },
    ElementBinding { digit: ShortcutKey::Digit9, role: Role::EndOfAct, extended: true },
    ElementBinding { digit: ShortcutKey::Digit0, role: Role::Summary, extended: true },
    ElementBinding { digit: ShortcutKey::Digit1, role: Role::Outline1, extended: true },
    ElementBinding { digit: ShortcutKey::Digit2, role: Role::Outline2, extended: true },
    ElementBinding { digit: ShortcutKey::Digit3, role: Role::Outline3, extended: true },
    ElementBinding { digit: ShortcutKey::Digit4, role: Role::Note, extended: true },
];

/// Resolves editor-local bindings without consulting the menu catalog. The
/// caller supplies a pressed key event and suspends this resolver during IME.
pub(super) fn editor_command_for_key_event(
    key: egui::Key,
    physical_key: Option<egui::Key>,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
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
    let digit = logical_digit(key)?;
    for binding in EDITOR_ELEMENT_BINDINGS {
        if binding.digit != digit {
            continue;
        }
        let shortcut = match (platform, binding.extended) {
            (ShortcutPlatform::MacOS, false) => primary_shortcut(binding.digit, false),
            (ShortcutPlatform::MacOS, true) => LogicalShortcut {
                control: true,
                ..primary_shortcut(binding.digit, false)
            },
            (ShortcutPlatform::Windows, false) => primary_shortcut(binding.digit, false),
            (ShortcutPlatform::Windows, true) => LogicalShortcut {
                shift: true,
                ..primary_shortcut(binding.digit, false)
            },
            (ShortcutPlatform::Other, _) => return None,
        };
        if shortcut_matches(shortcut, key, modifiers, platform) {
            return Some(Command::EditorElementChange {
                role: RoleValue::new(binding.role),
                surface: EditorSurface::Body,
            });
        }
    }
    None
}

/// Focused fallback for the Windows-owned in-window dual-dialogue accelerator.
/// macOS deliberately declines here because its native menu is the sole owner.
pub(super) fn dual_dialogue_toggle_for_focused_key_event(
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

fn logical_digit(key: egui::Key) -> Option<ShortcutKey> {
    Some(match key {
        egui::Key::Num0 => ShortcutKey::Digit0,
        egui::Key::Num1 => ShortcutKey::Digit1,
        egui::Key::Num2 => ShortcutKey::Digit2,
        egui::Key::Num3 => ShortcutKey::Digit3,
        egui::Key::Num4 => ShortcutKey::Digit4,
        egui::Key::Num5 => ShortcutKey::Digit5,
        egui::Key::Num6 => ShortcutKey::Digit6,
        egui::Key::Num7 => ShortcutKey::Digit7,
        egui::Key::Num8 => ShortcutKey::Digit8,
        egui::Key::Num9 => ShortcutKey::Digit9,
        _ => return None,
    })
}

pub(super) fn modifiers_are_none(modifiers: egui::Modifiers) -> bool {
    !modifiers.alt
        && !modifiers.ctrl
        && !modifiers.command
        && !modifiers.mac_cmd
        && !modifiers.shift
}

pub(super) const fn shortcut_input_allowed(ime_active: bool, ime_event_in_frame: bool) -> bool {
    !ime_active && !ime_event_in_frame
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifiers(
        platform: ShortcutPlatform,
        primary: bool,
        control: bool,
        shift: bool,
    ) -> egui::Modifiers {
        egui::Modifiers {
            alt: false,
            ctrl: platform == ShortcutPlatform::MacOS && control,
            command: primary,
            mac_cmd: platform != ShortcutPlatform::MacOS && control,
            shift,
        }
    }

    #[test]
    fn element_bindings_cover_all_roles_and_require_exact_platform_chords() {
        let mut mapped_roles = EDITOR_ELEMENT_BINDINGS.map(|binding| binding.role);
        mapped_roles.sort_unstable();
        let mut expected_roles = Role::ALL;
        expected_roles.sort_unstable();
        assert_eq!(mapped_roles, expected_roles);

        for platform in [ShortcutPlatform::MacOS, ShortcutPlatform::Windows] {
            for binding in EDITOR_ELEMENT_BINDINGS {
                let key = match binding.digit {
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
                    _ => continue,
                };
                let (control, shift) = match (platform, binding.extended) {
                    (ShortcutPlatform::MacOS, true) => (true, false),
                    (ShortcutPlatform::Windows, true) => (false, true),
                    _ => (false, false),
                };
                let command = editor_command_for_key_event(
                    key,
                    Some(key),
                    modifiers(platform, true, control, shift),
                    platform,
                );
                assert_eq!(
                    command,
                    Some(Command::EditorElementChange {
                        role: RoleValue::new(binding.role),
                        surface: EditorSurface::Body,
                    })
                );
                assert!(editor_command_for_key_event(
                    key,
                    Some(key),
                    modifiers(platform, true, control, !shift),
                    platform,
                )
                .is_none());
                let mut with_option = modifiers(platform, true, control, shift);
                with_option.alt = true;
                assert!(editor_command_for_key_event(
                    key,
                    Some(key),
                    with_option,
                    platform,
                )
                .is_none());
            }
        }
    }

    #[test]
    fn guidance_chord_projection_matches_host_resolver_modifiers_and_shared_bindings() {
        for (platform, base, extended, wrong, extra) in [
            (
                ShortcutPlatform::MacOS,
                modifiers(ShortcutPlatform::MacOS, true, false, false),
                modifiers(ShortcutPlatform::MacOS, true, true, false),
                modifiers(ShortcutPlatform::MacOS, true, false, true),
                modifiers(ShortcutPlatform::MacOS, false, true, false),
            ),
            (
                ShortcutPlatform::Windows,
                modifiers(ShortcutPlatform::Windows, true, false, false),
                modifiers(ShortcutPlatform::Windows, true, false, true),
                modifiers(ShortcutPlatform::Windows, true, true, false),
                modifiers(ShortcutPlatform::Windows, false, false, true),
            ),
        ] {
            assert_eq!(element_hint_chord(base, platform), Some(ElementHintChord::Base));
            assert_eq!(
                element_hint_chord(extended, platform),
                Some(ElementHintChord::Extended)
            );
            assert_eq!(element_hint_chord(wrong, platform), None);
            assert_eq!(element_hint_chord(extra, platform), None);
            let mut with_alt = base;
            with_alt.alt = true;
            assert_eq!(element_hint_chord(with_alt, platform), None);
        }
        assert_eq!(
            element_hint_chord(
                modifiers(ShortcutPlatform::Other, true, false, false),
                ShortcutPlatform::Other,
            ),
            None
        );

        let base = element_hint_bindings(false);
        let extended = element_hint_bindings(true);
        assert_eq!(base.len(), 10);
        assert_eq!(extended.len(), 7);
        assert_eq!(base.first(), Some(&(0, Role::General)));
        assert!(extended.contains(&(0, Role::Summary)));
        assert!(extended.contains(&(8, Role::Sequence)));
        assert!(extended.contains(&(9, Role::EndOfAct)));
    }

    #[test]
    fn paragraph_keys_are_unmodified_only_and_numpad_is_not_added() {
        assert_eq!(
            editor_command_for_key_event(
                egui::Key::Enter,
                None,
                egui::Modifiers::NONE,
                ShortcutPlatform::MacOS,
            ),
            Some(Command::EditorParagraphEnter)
        );
        assert_eq!(
            editor_command_for_key_event(
                egui::Key::Tab,
                None,
                egui::Modifiers::NONE,
                ShortcutPlatform::Windows,
            ),
            Some(Command::EditorParagraphTab)
        );
        assert!(editor_command_for_key_event(
            egui::Key::Enter,
            None,
            egui::Modifiers::CTRL,
            ShortcutPlatform::Windows,
        )
        .is_none());
        assert!(editor_command_for_key_event(
            egui::Key::Num8,
            None,
            modifiers(ShortcutPlatform::MacOS, true, false, false),
            ShortcutPlatform::Windows,
        )
        .is_none());
        assert!(editor_command_for_key_event(
            egui::Key::Num8,
            Some(egui::Key::K),
            modifiers(ShortcutPlatform::MacOS, true, false, false),
            ShortcutPlatform::MacOS,
        )
        .is_none());
    }

    #[test]
    fn dual_dialogue_focused_fallback_requires_exact_host_chord_and_in_window_ownership() {
        let mac = egui::Modifiers {
            command: true,
            ..egui::Modifiers::NONE
        };
        assert_eq!(
            dual_dialogue_toggle_for_focused_key_event(
                egui::Key::D,
                Some(egui::Key::D),
                mac,
                ShortcutPlatform::MacOS,
                true,
            ),
            Some(Command::EditorDualDialogueToggle),
        );
        assert!(dual_dialogue_toggle_for_focused_key_event(
            egui::Key::D,
            Some(egui::Key::D),
            mac,
            ShortcutPlatform::MacOS,
            false,
        )
        .is_none());

        let windows = egui::Modifiers {
            command: true,
            ctrl: true,
            alt: true,
            ..egui::Modifiers::NONE
        };
        assert_eq!(
            dual_dialogue_toggle_for_focused_key_event(
                egui::Key::D,
                Some(egui::Key::D),
                windows,
                ShortcutPlatform::Windows,
                true,
            ),
            Some(Command::EditorDualDialogueToggle),
        );
        for mut wrong in [windows, mac] {
            wrong.shift = true;
            assert!(dual_dialogue_toggle_for_focused_key_event(
                egui::Key::D,
                Some(egui::Key::D),
                wrong,
                if wrong.ctrl { ShortcutPlatform::Windows } else { ShortcutPlatform::MacOS },
                true,
            )
            .is_none());
        }
        let mut missing_alt = windows;
        missing_alt.alt = false;
        assert!(dual_dialogue_toggle_for_focused_key_event(
            egui::Key::D,
            Some(egui::Key::D),
            missing_alt,
            ShortcutPlatform::Windows,
            true,
        )
        .is_none());
        assert!(dual_dialogue_toggle_for_focused_key_event(
            egui::Key::D,
            Some(egui::Key::K),
            windows,
            ShortcutPlatform::Windows,
            true,
        )
        .is_none());
        assert!(dual_dialogue_toggle_for_focused_key_event(
            egui::Key::D,
            Some(egui::Key::D),
            windows,
            ShortcutPlatform::Other,
            true,
        )
        .is_none());
    }

    #[test]
    fn ime_state_or_same_frame_ime_event_suppresses_shortcut_dispatch() {
        assert!(shortcut_input_allowed(false, false));
        assert!(!shortcut_input_allowed(true, false));
        assert!(!shortcut_input_allowed(false, true));
        assert!(!shortcut_input_allowed(true, true));
    }
}
