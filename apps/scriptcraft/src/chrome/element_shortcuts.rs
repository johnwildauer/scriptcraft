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

#[cfg(test)]
mod tests {
    use super::*;
    use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment};
    use scriptcraft_model::{
        ElementBehavior, ElementFontSpec, ElementOutline, ElementParagraphSpec,
    };

    const DYNAMIC_CONTROLS_FDX: &[u8] = include_bytes!("../../tests/fixtures/dynamic_controls.fdx");

    fn definition(
        key: &str,
        mac: Option<&str>,
        windows: Option<&str>,
    ) -> Result<ElementDefinition, scriptcraft_model::ModelError> {
        Ok(ElementDefinition {
            key: ElementTypeKey::new(key)?,
            font: ElementFontSpec {
                family: "Courier Final Draft".to_owned(),
                size: 12,
                bold: false,
                italic: false,
                underline: false,
                all_caps: false,
                hidden_text: false,
                color: None,
                adornment_style: 0,
                revision_id: 0,
            },
            paragraph: ElementParagraphSpec {
                alignment: ParagraphAlignment::Left,
                first_indent: FixedPoint::from_points(0),
                left_indent: FixedPoint::from_points(0),
                right_indent: FixedPoint::from_points(0),
                leading: "Regular".to_owned(),
                space_before: FixedPoint::from_points(0),
                spacing: 1,
                starts_new_page: false,
            },
            behavior: ElementBehavior {
                paginate_as: None,
                return_key: None,
                shortcut: mac.map(str::to_owned),
                tab_key: None,
                win_shortcut: windows.map(str::to_owned),
            },
            outline: ElementOutline {
                can_hide: false,
                level: 1,
            },
        })
    }

    fn definitions(
        values: &[(&str, Option<&str>, Option<&str>)],
    ) -> Result<DocumentElementDefinitions, scriptcraft_model::ModelError> {
        DocumentElementDefinitions::new(
            values
                .iter()
                .map(|(key, mac, windows)| definition(key, *mac, *windows))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    fn chord(key: ShortcutKey, control: bool, option: bool, shift: bool) -> LogicalShortcut {
        LogicalShortcut {
            key,
            primary: true,
            control,
            option,
            shift,
        }
    }

    #[test]
    fn platform_parser_keeps_primary_and_literal_modifier_semantics_distinct() {
        assert_eq!(
            parse_element_shortcut("  Ctrl+Alt+Option+Shift+7 \t", ShortcutPlatform::MacOS),
            Err(ElementShortcutParseError::DuplicateModifier)
        );
        assert_eq!(
            parse_element_shortcut(" Ctrl+Alt+Shift+7 ", ShortcutPlatform::MacOS),
            Ok(Some(chord(ShortcutKey::Digit7, true, true, true)))
        );
        assert_eq!(
            parse_element_shortcut("cTrL+oPtIoN+SHIFT+7", ShortcutPlatform::Windows),
            Ok(Some(chord(ShortcutKey::Digit7, false, true, true)))
        );
        assert_eq!(
            parse_element_shortcut("7", ShortcutPlatform::Windows),
            Ok(Some(chord(ShortcutKey::Digit7, false, false, false)))
        );
        assert_eq!(
            parse_element_shortcut("ArrowRight", ShortcutPlatform::MacOS),
            Ok(Some(chord(ShortcutKey::ArrowRight, false, false, false)))
        );
        assert_eq!(
            parse_element_shortcut("Enter", ShortcutPlatform::Windows),
            Ok(Some(chord(ShortcutKey::Enter, false, false, false)))
        );
    }

    #[test]
    fn blank_values_are_absent_and_platform_uses_only_its_own_field() {
        assert_eq!(
            parse_element_shortcut(" \t\n", ShortcutPlatform::MacOS),
            Ok(None)
        );
        assert_eq!(
            parse_element_shortcut("not parsed on this host", ShortcutPlatform::Other),
            Ok(None)
        );
        let Ok(definitions) = definitions(&[("Lyrics", Some(""), Some("Alt+L"))]) else {
            assert!(false, "test registry should be valid");
            return;
        };
        assert!(resolve_element_shortcuts(
            &definitions,
            ShortcutPlatform::MacOS,
            &ReservedElementChords::default()
        )
        .bindings
        .is_empty());
        assert_eq!(
            resolve_element_shortcuts(
                &definitions,
                ShortcutPlatform::Windows,
                &ReservedElementChords::default()
            )
            .bindings[0]
                .element_type
                .as_str(),
            "Lyrics"
        );
    }

    #[test]
    fn punctuation_is_logical_and_colon_conflicts_with_shift_semicolon() {
        assert_eq!(
            parse_element_shortcut(";", ShortcutPlatform::MacOS),
            Ok(Some(chord(ShortcutKey::Semicolon, false, false, false)))
        );
        assert_eq!(
            parse_element_shortcut(":", ShortcutPlatform::MacOS),
            Ok(Some(chord(ShortcutKey::Semicolon, false, false, true)))
        );
        assert_eq!(
            parse_element_shortcut("Shift+;", ShortcutPlatform::MacOS),
            Ok(Some(chord(ShortcutKey::Semicolon, false, false, true)))
        );
        let Ok(definitions) = definitions(&[
            ("Colon", Some(":"), None),
            ("Shift semicolon", Some("Shift+;"), None),
        ]) else {
            assert!(false, "test registry should be valid");
            return;
        };
        let resolved = resolve_element_shortcuts(
            &definitions,
            ShortcutPlatform::MacOS,
            &ReservedElementChords::default(),
        );
        assert!(resolved.bindings.is_empty());
        assert_eq!(resolved.issues.len(), 2);
        assert!(resolved
            .issues
            .iter()
            .all(|issue| issue.kind == ElementShortcutIssueKind::Conflict));
    }

    #[test]
    fn malformed_unsupported_reserved_and_duplicate_groups_are_disabled() {
        for (raw, expected) in [
            ("Ctrl++A", ElementShortcutParseError::EmptyComponent),
            ("Ctrl+Ctrl+A", ElementShortcutParseError::DuplicateModifier),
            ("A+B", ElementShortcutParseError::MultipleKeys),
            ("Ctrl+", ElementShortcutParseError::EmptyComponent),
            ("Ctrl+Comma", ElementShortcutParseError::UnknownToken),
            ("Alt+,", ElementShortcutParseError::UnsupportedPunctuation),
            ("Ctrl+é", ElementShortcutParseError::NonAscii),
        ] {
            assert_eq!(
                parse_element_shortcut(raw, ShortcutPlatform::MacOS),
                Err(expected)
            );
        }
        let oversized = "A".repeat(MAX_ELEMENT_DEFINITION_STRING_BYTES + 1);
        assert_eq!(
            parse_element_shortcut(&oversized, ShortcutPlatform::MacOS),
            Err(ElementShortcutParseError::ValueTooLong)
        );

        let reserved_chord = chord(ShortcutKey::A, false, false, false);
        let reserved = ReservedElementChords::from_menu_shortcuts([reserved_chord]);
        let Ok(definitions) = definitions(&[
            ("reserved", Some("A"), None),
            ("duplicate one", Some("B"), None),
            ("duplicate two", Some("b"), None),
            ("duplicate three", Some("B"), None),
            ("wildcard", Some("Ctrl+Delete"), None),
        ]) else {
            assert!(false, "test registry should be valid");
            return;
        };
        let mut reserved = reserved;
        reserved.reserve_wildcard_key(ShortcutKey::Delete);
        let resolved = resolve_element_shortcuts(&definitions, ShortcutPlatform::MacOS, &reserved);
        assert!(resolved.bindings.is_empty());
        assert_eq!(resolved.issues.len(), 5);
        assert_eq!(resolved.issues[0].kind, ElementShortcutIssueKind::Reserved);
        assert!(resolved.issues[1..4]
            .iter()
            .all(|issue| issue.kind == ElementShortcutIssueKind::Conflict));
        assert_eq!(resolved.issues[4].kind, ElementShortcutIssueKind::Reserved);
    }

    #[test]
    fn duplicate_outcome_does_not_depend_on_registry_position() {
        let Ok(first) = definitions(&[("One", Some("K"), None), ("Two", Some("k"), None)]) else {
            assert!(false, "test registry should be valid");
            return;
        };
        let Ok(reversed) = definitions(&[("Two", Some("k"), None), ("One", Some("K"), None)])
        else {
            assert!(false, "reversed test registry should be valid");
            return;
        };
        for registry in [&first, &reversed] {
            let resolved = resolve_element_shortcuts(
                registry,
                ShortcutPlatform::MacOS,
                &ReservedElementChords::default(),
            );
            assert!(resolved.bindings.is_empty());
            assert_eq!(resolved.issues.len(), 2);
            assert!(resolved
                .issues
                .iter()
                .all(|issue| issue.kind == ElementShortcutIssueKind::Conflict));
        }
    }

    #[test]
    fn diagnostics_are_capped_and_other_platforms_have_no_active_bindings() {
        let labels = (0..40)
            .map(|index| format!("Type {index}"))
            .collect::<Vec<_>>();
        let rows = labels
            .iter()
            .map(|label| (label.as_str(), Some("?"), None))
            .collect::<Vec<_>>();
        let Ok(definitions) = definitions(&rows) else {
            assert!(false, "bounded test registry should be valid");
            return;
        };
        let resolved = resolve_element_shortcuts(
            &definitions,
            ShortcutPlatform::MacOS,
            &ReservedElementChords::default(),
        );
        assert_eq!(resolved.issues.len(), MAX_RETAINED_ISSUES);
        assert_eq!(resolved.suppressed_issue_count, 8);
        assert!(resolved.bindings.is_empty());
        let issue_summary = format!("{:?}", resolved.issues);
        assert!(!issue_summary.contains("Type"));
        assert!(!issue_summary.contains('?'));
        assert_eq!(
            resolve_element_shortcuts(
                &definitions,
                ShortcutPlatform::Other,
                &ReservedElementChords::default()
            ),
            ResolvedElementShortcuts::default()
        );
    }

    #[test]
    fn menu_colon_reservation_uses_the_same_canonical_chord() {
        let reserved = ReservedElementChords::from_menu_shortcuts([LogicalShortcut {
            key: ShortcutKey::Colon,
            primary: true,
            control: false,
            option: false,
            shift: false,
        }]);
        let Ok(definitions) = definitions(&[("Punctuation", Some("Shift+;"), None)]) else {
            assert!(false, "test registry should be valid");
            return;
        };
        let resolved = resolve_element_shortcuts(&definitions, ShortcutPlatform::MacOS, &reserved);
        assert!(resolved.bindings.is_empty());
        assert_eq!(resolved.issues[0].kind, ElementShortcutIssueKind::Reserved);
    }

    #[test]
    fn fixture_platform_bindings_disable_every_reserved_and_ambiguous_member() {
        let project = scriptcraft_fdx::read_v6(DYNAMIC_CONTROLS_FDX)
            .unwrap_or_else(|_| unreachable!("checked-in dynamic controls fixture is valid"))
            .project;
        let definitions = project.element_definitions();
        let mac = resolve_element_shortcuts(
            definitions,
            ShortcutPlatform::MacOS,
            &super::super::menu::reserved_element_chords(ShortcutPlatform::MacOS),
        );
        let windows = resolve_element_shortcuts(
            definitions,
            ShortcutPlatform::Windows,
            &super::super::menu::reserved_element_chords(ShortcutPlatform::Windows),
        );

        let has_binding = |resolved: &ResolvedElementShortcuts, label: &str| {
            resolved
                .bindings
                .iter()
                .any(|binding| binding.element_type.as_str() == label)
        };
        let issue_kind_for = |resolved: &ResolvedElementShortcuts, label: &str| {
            let ordinal = definitions
                .iter()
                .position(|definition| definition.key.as_str() == label)
                .unwrap_or_else(|| unreachable!("fixture definition is present"));
            resolved
                .issues
                .iter()
                .find(|issue| issue.definition_ordinal == ordinal)
                .map(|issue| issue.kind)
        };

        for resolved in [&mac, &windows] {
            assert!(has_binding(resolved, "Lyrics"));
            assert!(has_binding(resolved, "lyrics"));
            assert!(!has_binding(resolved, "Blank Binding"));
            for label in ["Conflict One", "Conflict Two", "Conflict Three"] {
                assert!(!has_binding(resolved, label));
                assert_eq!(
                    issue_kind_for(resolved, label),
                    Some(ElementShortcutIssueKind::Conflict)
                );
            }
            assert!(has_binding(resolved, "Punctuation"));
            for label in ["Colon Alias", "Shift Semicolon Alias"] {
                assert!(!has_binding(resolved, label));
                assert_eq!(
                    issue_kind_for(resolved, label),
                    Some(ElementShortcutIssueKind::Conflict)
                );
            }
        }
        assert_eq!(
            issue_kind_for(&mac, "Mac Quit"),
            Some(ElementShortcutIssueKind::Reserved)
        );
        assert_eq!(
            issue_kind_for(&windows, "Windows Dual"),
            Some(ElementShortcutIssueKind::Reserved)
        );
        assert_eq!(
            definitions
                .get_by_label("Blank Binding")
                .and_then(|definition| definition.behavior.shortcut.as_deref()),
            Some("  ")
        );
        assert_eq!(
            definitions
                .get_by_label("Blank Binding")
                .and_then(|definition| definition.behavior.win_shortcut.as_deref()),
            Some("   ")
        );
    }
}
