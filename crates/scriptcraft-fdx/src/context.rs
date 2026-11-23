use crate::{
    diagnostics::{FdxError, SaveDiagnosticCode},
    document::{InvisiblesState, ScriptPanelDisplayMode, WindowState},
    text_state::TextState,
};
use std::collections::BTreeSet;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOperation {
    DirectSerialization,
    Save,
    SaveAs,
}

/// Save-scoped writer authority. Direct serialization has no lineage side
/// effects; transactional saves capture one timestamp before candidate work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveContext {
    operation: SaveOperation,
    timestamp: String,
    session_open_timestamp: Option<String>,
    window_state: WindowState,
    text_state: TextState,
    display_mode: ScriptPanelDisplayMode,
    invisibles_state: InvisiblesState,
}

impl SaveContext {
    pub fn new(operation: SaveOperation) -> Self {
        Self {
            operation,
            timestamp: current_timestamp(),
            session_open_timestamp: None,
            window_state: WindowState::DEFAULT,
            text_state: TextState::DEFAULT,
            display_mode: ScriptPanelDisplayMode::Page,
            invisibles_state: InvisiblesState::default(),
        }
    }

    pub fn with_session_open_timestamp(mut self, timestamp: String) -> Self {
        self.session_open_timestamp = Some(timestamp);
        self
    }

    pub fn with_window_state(mut self, window_state: WindowState) -> Self {
        self.window_state = window_state;
        self
    }

    pub fn with_text_state(mut self, text_state: TextState) -> Self {
        self.text_state = text_state;
        self
    }

    pub fn with_display_mode(mut self, display_mode: ScriptPanelDisplayMode) -> Self {
        self.display_mode = display_mode;
        self
    }

    pub fn with_invisibles_state(mut self, invisibles_state: InvisiblesState) -> Self {
        self.invisibles_state = invisibles_state;
        self
    }

    pub(crate) const fn operation(&self) -> SaveOperation {
        self.operation
    }

    pub(crate) fn timestamp(&self) -> &str {
        &self.timestamp
    }

    pub(crate) fn session_open_timestamp(&self) -> Option<&str> {
        self.session_open_timestamp.as_deref()
    }

    pub(crate) const fn window_state(&self) -> WindowState {
        self.window_state
    }

    pub(crate) const fn text_state(&self) -> TextState {
        self.text_state
    }

    pub(crate) const fn display_mode(&self) -> ScriptPanelDisplayMode {
        self.display_mode
    }

    pub(crate) const fn invisibles_state(&self) -> InvisiblesState {
        self.invisibles_state
    }

    pub(crate) fn allocate_lineage_id(
        &mut self,
        reserved: &mut BTreeSet<String>,
    ) -> Result<String, FdxError> {
        const MAX_ATTEMPTS: usize = 16;
        for _ in 0..MAX_ATTEMPTS {
            let mut bytes = [0; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| SaveDiagnosticCode::LineageIdentityEntropyUnavailable.error())?;
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            let candidate = uuid::Uuid::from_bytes(bytes).hyphenated().to_string();
            if is_lowercase_uuid_v4(&candidate) && reserved.insert(candidate.clone()) {
                return Ok(candidate);
            }
        }
        Err(SaveDiagnosticCode::LineageIdentityCollisionLimit.error())
    }
}

fn is_lowercase_uuid_v4(value: &str) -> bool {
    if value.len() != 36 || value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return false;
    }
    uuid::Uuid::parse_str(value).is_ok_and(|uuid| {
        uuid.hyphenated().to_string() == value
            && uuid.get_version() == Some(uuid::Version::Random)
            && uuid.get_variant() == uuid::Variant::RFC4122
    })
}

pub(crate) fn valid_document_ref_datetime(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 15
        || bytes[8] != b'T'
        || !bytes[..8].iter().all(u8::is_ascii_digit)
        || !bytes[9..].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let number = |range: std::ops::Range<usize>| value.get(range)?.parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0..4),
        number(4..6),
        number(6..8),
        number(9..11),
        number(11..13),
        number(13..15),
    ) else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

pub fn current_timestamp() -> String {
    let local = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}",
        local.year(),
        u8::from(local.month()),
        local.day(),
        local.hour(),
        local.minute(),
        local.second()
    )
}
