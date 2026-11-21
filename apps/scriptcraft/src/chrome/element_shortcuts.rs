//! Pure parsing and deterministic resolution of document-owned element shortcuts.

use std::collections::BTreeMap;

use scriptcraft_model::{
    DocumentElementDefinitions, ElementDefinition, ElementTypeKey,
    MAX_ELEMENT_DEFINITION_STRING_BYTES,
};

use super::shortcuts::{LogicalShortcut, ShortcutKey, ShortcutPlatform};

const MAX_RETAINED_ISSUES: usize = 32;

/// Bounded parse failures. Variants intentionally carry no source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementShortcutParseError {
    ValueTooLong,
    NonAscii,
    EmptyComponent,
    DuplicateModifier,
    MissingKey,
    MultipleKeys,
    UnknownToken,
    UnsupportedPunctuation,
}

impl ElementShortcutParseError {
    fn issue_kind(self) -> ElementShortcutIssueKind {
        match self {
            Self::ValueTooLong
            | Self::NonAscii
            | Self::UnknownToken
            | Self::UnsupportedPunctuation => ElementShortcutIssueKind::Unsupported,
            Self::EmptyComponent
            | Self::DuplicateModifier
            | Self::MissingKey
            | Self::MultipleKeys => ElementShortcutIssueKind::Malformed,
        }
    }
}

/// Injected exact and wildcard reservations, independent of menu/event routing.
///
/// The Phase 6.6 owner supplies active menu accelerators (including aliases and
/// disabled placeholders) and fixed-editor wildcard keys. Menu chords are
/// canonicalized here so colon and Shift+semicolon have the same identity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReservedElementChords {
    exact: std::collections::BTreeSet<LogicalShortcut>,
    wildcard_keys: std::collections::BTreeSet<ShortcutKey>,
}

impl ReservedElementChords {
    /// Builds exact reservations from logical menu accelerators. Callers pass
    /// both the active platform chord and any platform-specific alias.
    pub(crate) fn from_menu_shortcuts(
        shortcuts: impl IntoIterator<Item = LogicalShortcut>,
    ) -> Self {
        let mut reserved = Self::default();
        for shortcut in shortcuts {
            reserved.reserve_menu_shortcut(shortcut);
        }
        reserved
    }

    /// Adds a menu accelerator after normalizing logical punctuation aliases.
    pub(crate) fn reserve_menu_shortcut(&mut self, shortcut: LogicalShortcut) {
        self.exact.insert(canonicalize_shortcut(shortcut));
    }

    /// Adds another exact reservation, using the same canonical chord rules.
    pub(crate) fn reserve_exact(&mut self, shortcut: LogicalShortcut) {
        self.exact.insert(canonicalize_shortcut(shortcut));
    }

    /// Reserves a key for every modifier combination used by fixed editor
    /// behavior whose current operation ignores or owns the modifiers.
    pub(crate) fn reserve_wildcard_key(&mut self, key: ShortcutKey) {
        self.wildcard_keys.insert(canonicalize_key(key).0);
    }

    pub(crate) fn contains(&self, shortcut: LogicalShortcut) -> bool {
        let shortcut = canonicalize_shortcut(shortcut);
        self.exact.contains(&shortcut) || self.wildcard_keys.contains(&shortcut.key)
    }
}

/// Stable, privacy-safe reason why one document definition has no binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementShortcutIssueKind {
    Malformed,
    Unsupported,
    Reserved,
    Conflict,
}

/// Coarse normalized chord category; it never contains source spelling or a
/// document label. The ordinal is zero-based in the current definition order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ElementShortcutIssue {
    pub(crate) definition_ordinal: usize,
    pub(crate) kind: ElementShortcutIssueKind,
    pub(crate) chord_category: Option<ElementShortcutChordCategory>,
}

/// Finite category used only for bounded conflict/reservation diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementShortcutChordCategory {
    Letter,
    Digit,
    NamedKey,
    Semicolon,
}

/// One enabled binding to the exact document-owned open element key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElementEditorBinding {
    pub(crate) definition_ordinal: usize,
    pub(crate) element_type: ElementTypeKey,
    pub(crate) chord: LogicalShortcut,
}

/// Complete immutable result of resolving one ordered definition registry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolvedElementShortcuts {
    pub(crate) bindings: Vec<ElementEditorBinding>,
    pub(crate) issues: Vec<ElementShortcutIssue>,
    pub(crate) suppressed_issue_count: usize,
}

/// Parses a single platform-specific source value without rewriting it.
///
/// A bare key implies Command on macOS or Control on Windows. On macOS `Ctrl`
/// adds literal Control; on Windows it repeats the already-implied primary.
/// `Alt` and `Option` are one slot. Components are plus-separated, with zero or
/// more modifiers before exactly one key. Only surrounding ASCII whitespace is
/// trimmed. Colon canonicalizes to Shift+semicolon before returning.
pub(crate) fn parse_element_shortcut(
    raw: &str,
    platform: ShortcutPlatform,
) -> Result<Option<LogicalShortcut>, ElementShortcutParseError> {
    if platform == ShortcutPlatform::Other {
        return Ok(None);
    }
    if raw.len() > MAX_ELEMENT_DEFINITION_STRING_BYTES {
        return Err(ElementShortcutParseError::ValueTooLong);
    }

    let source = raw.trim_matches(|character: char| character.is_ascii_whitespace());
    if source.is_empty() {
        return Ok(None);
    }
    if !source.is_ascii() {
        return Err(ElementShortcutParseError::NonAscii);
    }

    let components = source.split('+').collect::<Vec<_>>();
    if components.iter().any(|component| component.is_empty()) {
        return Err(ElementShortcutParseError::EmptyComponent);
    }

    let mut control = false;
    let mut option = false;
    let mut shift = false;
    let key_component_index = components.len() - 1;
    for component in components.iter().take(key_component_index) {
        match modifier_token(component) {
            Some(ModifierSlot::Control) => {
                if control {
                    return Err(ElementShortcutParseError::DuplicateModifier);
                }
                control = true;
            }
            Some(ModifierSlot::Option) => {
                if option {
                    return Err(ElementShortcutParseError::DuplicateModifier);
                }
                option = true;
            }
            Some(ModifierSlot::Shift) => {
                if shift {
                    return Err(ElementShortcutParseError::DuplicateModifier);
                }
                shift = true;
            }
            None => match parse_key_component(component) {
                Ok(_) => return Err(ElementShortcutParseError::MultipleKeys),
                Err(error) => return Err(error),
            },
        }
    }

    let key_component = components[key_component_index];
    if modifier_token(key_component).is_some() {
        return Err(ElementShortcutParseError::MissingKey);
    }
    let (key, colon_implies_shift) = parse_key_component(key_component)?;
    let shortcut = LogicalShortcut {
        key,
        primary: true,
        control: control && platform == ShortcutPlatform::MacOS,
        option,
        shift: shift || colon_implies_shift,
    };
    Ok(Some(canonicalize_shortcut(shortcut)))
}

/// Resolves only the current platform's source field for each definition.
/// Every unsafe chord is disabled as a group; output and issue order follow the
/// registry, while the conflict map itself is keyed by canonical chord.
pub(crate) fn resolve_element_shortcuts(
    definitions: &DocumentElementDefinitions,
    platform: ShortcutPlatform,
    reserved: &ReservedElementChords,
) -> ResolvedElementShortcuts {
    if platform == ShortcutPlatform::Other {
        return ResolvedElementShortcuts::default();
    }

    let ordered = definitions.iter().collect::<Vec<_>>();
    let mut chords = vec![None; ordered.len()];
    let mut issue_by_ordinal = vec![None; ordered.len()];
    let mut groups = BTreeMap::<LogicalShortcut, Vec<usize>>::new();

    for (ordinal, definition) in ordered.iter().enumerate() {
        let source = match platform {
            ShortcutPlatform::MacOS => definition.behavior.shortcut.as_deref(),
            ShortcutPlatform::Windows => definition.behavior.win_shortcut.as_deref(),
            ShortcutPlatform::Other => None,
        };
        let Some(source) = source else {
            continue;
        };
        match parse_element_shortcut(source, platform) {
            Ok(Some(chord)) => {
                let chord = canonicalize_shortcut(chord);
                chords[ordinal] = Some(chord);
                groups.entry(chord).or_default().push(ordinal);
            }
            Ok(None) => {}
            Err(error) => {
                issue_by_ordinal[ordinal] = Some(ElementShortcutIssue {
                    definition_ordinal: ordinal,
                    kind: error.issue_kind(),
                    chord_category: None,
                });
            }
        }
    }

    for (chord, ordinals) in groups {
        let issue_kind = if reserved.contains(chord) {
            Some(ElementShortcutIssueKind::Reserved)
        } else if ordinals.len() > 1 {
            Some(ElementShortcutIssueKind::Conflict)
        } else {
            None
        };
        if let Some(kind) = issue_kind {
            let category = chord_category(chord.key);
            for ordinal in ordinals {
                issue_by_ordinal[ordinal] = Some(ElementShortcutIssue {
                    definition_ordinal: ordinal,
                    kind,
                    chord_category: Some(category),
                });
            }
        }
    }

    let mut result = ResolvedElementShortcuts::default();
    for (ordinal, definition) in ordered.into_iter().enumerate() {
        if let Some(issue) = issue_by_ordinal[ordinal] {
            if result.issues.len() < MAX_RETAINED_ISSUES {
                result.issues.push(issue);
            } else {
                result.suppressed_issue_count = result.suppressed_issue_count.saturating_add(1);
            }
        } else if let Some(chord) = chords[ordinal] {
            result
                .bindings
                .push(binding_for(ordinal, definition, chord));
        }
    }
    result
}

fn binding_for(
    definition_ordinal: usize,
    definition: &ElementDefinition,
    chord: LogicalShortcut,
) -> ElementEditorBinding {
    ElementEditorBinding {
        definition_ordinal,
        element_type: definition.key.clone(),
        chord,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModifierSlot {
    Control,
    Option,
    Shift,
}

fn modifier_token(component: &str) -> Option<ModifierSlot> {
    if component.eq_ignore_ascii_case("Ctrl") {
        Some(ModifierSlot::Control)
    } else if component.eq_ignore_ascii_case("Alt") || component.eq_ignore_ascii_case("Option") {
        Some(ModifierSlot::Option)
    } else if component.eq_ignore_ascii_case("Shift") {
        Some(ModifierSlot::Shift)
    } else {
        None
    }
}

fn parse_key_component(component: &str) -> Result<(ShortcutKey, bool), ElementShortcutParseError> {
    if component.len() == 1 {
        let byte = component.as_bytes()[0];
        if byte.is_ascii_alphabetic() {
            return Ok((letter_key(byte.to_ascii_uppercase()), false));
        }
        if byte.is_ascii_digit() {
            return Ok((digit_key(byte - b'0'), false));
        }
        return match byte {
            b';' => Ok((ShortcutKey::Semicolon, false)),
            b':' => Ok((ShortcutKey::Semicolon, true)),
            byte if byte.is_ascii_punctuation() => {
                Err(ElementShortcutParseError::UnsupportedPunctuation)
            }
            _ => Err(ElementShortcutParseError::UnknownToken),
        };
    }

    let named_key = if component.eq_ignore_ascii_case("Enter") {
        Some(ShortcutKey::Enter)
    } else if component.eq_ignore_ascii_case("Tab") {
        Some(ShortcutKey::Tab)
    } else if component.eq_ignore_ascii_case("Backspace") {
        Some(ShortcutKey::Backspace)
    } else if component.eq_ignore_ascii_case("Delete") {
        Some(ShortcutKey::Delete)
    } else if component.eq_ignore_ascii_case("ArrowRight") {
        Some(ShortcutKey::ArrowRight)
    } else {
        None
    };
    if let Some(key) = named_key {
        return Ok((key, false));
    }
    if component.bytes().any(|byte| byte.is_ascii_punctuation()) {
        Err(ElementShortcutParseError::UnsupportedPunctuation)
    } else {
        Err(ElementShortcutParseError::UnknownToken)
    }
}

fn letter_key(letter: u8) -> ShortcutKey {
    match letter {
        b'A' => ShortcutKey::A,
        b'B' => ShortcutKey::B,
        b'C' => ShortcutKey::C,
        b'D' => ShortcutKey::D,
        b'E' => ShortcutKey::E,
        b'F' => ShortcutKey::F,
        b'G' => ShortcutKey::G,
        b'H' => ShortcutKey::H,
        b'I' => ShortcutKey::I,
        b'J' => ShortcutKey::J,
        b'K' => ShortcutKey::K,
        b'L' => ShortcutKey::L,
        b'M' => ShortcutKey::M,
        b'N' => ShortcutKey::N,
        b'O' => ShortcutKey::O,
        b'P' => ShortcutKey::P,
        b'Q' => ShortcutKey::Q,
        b'R' => ShortcutKey::R,
        b'S' => ShortcutKey::S,
        b'T' => ShortcutKey::T,
        b'U' => ShortcutKey::U,
        b'V' => ShortcutKey::V,
        b'W' => ShortcutKey::W,
        b'X' => ShortcutKey::X,
        b'Y' => ShortcutKey::Y,
        _ => ShortcutKey::Z,
    }
}

fn digit_key(digit: u8) -> ShortcutKey {
    match digit {
        0 => ShortcutKey::Digit0,
        1 => ShortcutKey::Digit1,
        2 => ShortcutKey::Digit2,
        3 => ShortcutKey::Digit3,
        4 => ShortcutKey::Digit4,
        5 => ShortcutKey::Digit5,
        6 => ShortcutKey::Digit6,
        7 => ShortcutKey::Digit7,
        8 => ShortcutKey::Digit8,
        _ => ShortcutKey::Digit9,
    }
}

fn canonicalize_shortcut(mut shortcut: LogicalShortcut) -> LogicalShortcut {
    let (key, colon_implies_shift) = canonicalize_key(shortcut.key);
    shortcut.key = key;
    shortcut.shift |= colon_implies_shift;
    shortcut
}

fn canonicalize_key(key: ShortcutKey) -> (ShortcutKey, bool) {
    if key == ShortcutKey::Colon {
        (ShortcutKey::Semicolon, true)
    } else {
        (key, false)
    }
}

fn chord_category(key: ShortcutKey) -> ElementShortcutChordCategory {
    match key {
        ShortcutKey::A
        | ShortcutKey::B
        | ShortcutKey::C
        | ShortcutKey::D
        | ShortcutKey::E
        | ShortcutKey::F
        | ShortcutKey::G
        | ShortcutKey::H
        | ShortcutKey::I
        | ShortcutKey::J
        | ShortcutKey::K
        | ShortcutKey::L
        | ShortcutKey::M
        | ShortcutKey::N
        | ShortcutKey::O
        | ShortcutKey::P
        | ShortcutKey::Q
        | ShortcutKey::R
        | ShortcutKey::S
        | ShortcutKey::T
        | ShortcutKey::U
        | ShortcutKey::V
        | ShortcutKey::W
        | ShortcutKey::X
        | ShortcutKey::Y
        | ShortcutKey::Z => ElementShortcutChordCategory::Letter,
        ShortcutKey::Digit0
        | ShortcutKey::Digit1
        | ShortcutKey::Digit2
        | ShortcutKey::Digit3
        | ShortcutKey::Digit4
        | ShortcutKey::Digit5
        | ShortcutKey::Digit6
        | ShortcutKey::Digit7
        | ShortcutKey::Digit8
        | ShortcutKey::Digit9 => ElementShortcutChordCategory::Digit,
        ShortcutKey::Semicolon | ShortcutKey::Colon => ElementShortcutChordCategory::Semicolon,
        _ => ElementShortcutChordCategory::NamedKey,
    }
}

