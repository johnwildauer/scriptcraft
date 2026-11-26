//! Semantic values and detached operations for alternate Dialogue variants.

use crate::{
    element::TextRun,
    element_definition::ElementDefinition,
    formatted_text::{sparse_alt_run_projection, FormattedTextError},
};
use std::{fmt, str::FromStr};

/// Maximum complete variant count on one Dialogue owner.
pub const MAX_DIALOGUE_ALT_VARIANTS_PER_OWNER: usize = 10_000;
/// Maximum complete variant count across a Project.
pub const MAX_DIALOGUE_ALT_VARIANTS_PER_PROJECT: usize = 100_000;
/// Aggregate inactive text budget, matching the retained-source text budget.
pub const MAX_DIALOGUE_ALT_TEXT_BYTES_PER_PROJECT: usize = 8 * 1024 * 1024;
/// Aggregate inactive run and attribute budgets across a Project.
pub const MAX_DIALOGUE_ALT_RUN_VALUES_PER_PROJECT: usize = 100_000;
/// Aggregate run-attribute name/value byte budget across a Project.
pub const MAX_DIALOGUE_ALT_ATTRIBUTE_BYTES_PER_PROJECT: usize = 8 * 1024 * 1024;

/// Nonnegative numeric identity used only within Dialogue Alt semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DialogueAltId(u64);

impl DialogueAltId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn checked_increment(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Parses an exact, nonempty ASCII decimal value within the `u64` bound.
    pub fn parse_decimal(value: &str) -> Option<Self> {
        if value.is_empty() || value.len() > 20 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse::<u64>().ok().map(Self)
    }
}

impl fmt::Display for DialogueAltId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for DialogueAltId {
    type Err = DialogueAltIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_decimal(value).ok_or(DialogueAltIdParseError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogueAltIdParseError;

impl fmt::Display for DialogueAltIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid Dialogue Alt numeric ID")
    }
}

impl std::error::Error for DialogueAltIdParseError {}

/// One inactive variant in preserved inline order. Runs are sparse relative to
/// the current exact Dialogue definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueAlternative {
    pub id: DialogueAltId,
    pub text: String,
    pub runs: Vec<TextRun>,
}

impl DialogueAlternative {
    pub fn new(id: DialogueAltId, text: impl Into<String>, runs: Vec<TextRun>) -> Self {
        Self {
            id,
            text: text.into(),
            runs,
        }
    }
}

/// Direction for numeric Previous/Next navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueAltDirection {
    Previous,
    Next,
}

/// One owner's active numeric identity and inline-ordered inactive variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueAlternatives {
    pub current: DialogueAltId,
    pub inactive: Vec<DialogueAlternative>,
}

impl DialogueAlternatives {
    pub fn new(
        current: DialogueAltId,
        inactive: Vec<DialogueAlternative>,
    ) -> Result<Self, DialogueAltError> {
        let alternatives = Self { current, inactive };
        alternatives.validate_ids()?;
        Ok(alternatives)
    }

    pub fn total_count(&self) -> usize {
        self.inactive.len().checked_add(1).unwrap_or(usize::MAX)
    }

    /// One-based position of Current in the numerically sorted full ID set.
    pub fn active_ordinal(&self) -> usize {
        self.inactive
            .iter()
            .filter(|alternative| alternative.id < self.current)
            .count()
            .checked_add(1)
            .unwrap_or(usize::MAX)
    }

    pub fn variant(&self, id: DialogueAltId) -> Option<&DialogueAlternative> {
        self.inactive.iter().find(|alternative| alternative.id == id)
    }

    pub fn has_inactive_variants(&self) -> bool {
        !self.inactive.is_empty()
    }

    pub fn can_navigate(&self) -> bool {
        self.total_count() > 1
    }

    pub fn can_delete_current(&self) -> bool {
        self.total_count() > 1
    }

    pub(crate) fn validate_ids(&self) -> Result<(), DialogueAltError> {
        if self.total_count() > MAX_DIALOGUE_ALT_VARIANTS_PER_OWNER {
            return Err(DialogueAltError::TooManyVariants);
        }
        let mut ids = std::collections::BTreeSet::new();
        ids.insert(self.current);
        for alternative in &self.inactive {
            if !ids.insert(alternative.id) {
                return Err(DialogueAltError::DuplicateId);
            }
        }
        Ok(())
    }

    /// Adds a fresh empty Current. Without prior state, `demoted_current` is
    /// required for the old active text; with prior state it must be `None`.
    pub fn add(
        existing: Option<&Self>,
        active_text: &str,
        active_runs: &[TextRun],
        ids: DialogueAltAddIds,
        dialogue_definition: &ElementDefinition,
    ) -> Result<DialogueAltEdit, DialogueAltError> {
        if let Some(existing) = existing {
            existing.validate_ids()?;
            if ids.demoted_current.is_some() {
                return Err(DialogueAltError::UnexpectedDemotedId);
            }
            if existing.current == ids.fresh_current
                || existing.variant(ids.fresh_current).is_some()
            {
                return Err(DialogueAltError::DuplicateId);
            }
        } else if ids.demoted_current.is_none() || ids.demoted_current == Some(ids.fresh_current) {
            return Err(DialogueAltError::DuplicateId);
        }

        let demoted_id = existing
            .map(|alternatives| alternatives.current)
            .or(ids.demoted_current)
            .ok_or(DialogueAltError::MissingDemotedId)?;
        let demoted_runs = sparse_alt_run_projection(
            dialogue_definition,
            active_text,
            active_runs,
        )?;
        let mut inactive = existing.map_or_else(Vec::new, |value| value.inactive.clone());
        inactive.push(DialogueAlternative::new(
            demoted_id,
            active_text,
            demoted_runs,
        ));
        let alternatives = Self::new(ids.fresh_current, inactive)?;
        Ok(DialogueAltEdit {
            alternatives: Some(alternatives),
            active_id: ids.fresh_current,
            text: String::new(),
            runs: Vec::new(),
        })
    }

    /// Switches to the nearest numeric Previous/Next target, wrapping at the
    /// numeric boundary and appending the demoted Current in inline order.
    pub fn navigate(
        &self,
        direction: DialogueAltDirection,
        active_text: &str,
        active_runs: &[TextRun],
        dialogue_definition: &ElementDefinition,
    ) -> Result<DialogueAltEdit, DialogueAltError> {
        self.validate_ids()?;
        if !self.can_navigate() {
            return Err(DialogueAltError::RequiresMultipleVariants);
        }
        let target = match direction {
            DialogueAltDirection::Previous => self
                .inactive
                .iter()
                .filter(|alternative| alternative.id < self.current)
                .max_by_key(|alternative| alternative.id)
                .or_else(|| self.inactive.iter().max_by_key(|alternative| alternative.id)),
            DialogueAltDirection::Next => self
                .inactive
                .iter()
                .filter(|alternative| alternative.id > self.current)
                .min_by_key(|alternative| alternative.id)
                .or_else(|| self.inactive.iter().min_by_key(|alternative| alternative.id)),
        }
        .ok_or(DialogueAltError::MissingTarget)?
        .clone();

        let demoted_runs = sparse_alt_run_projection(
            dialogue_definition,
            active_text,
            active_runs,
        )?;
        let promoted_runs = sparse_alt_run_projection(
            dialogue_definition,
            &target.text,
            &target.runs,
        )?;
        let mut inactive = self
            .inactive
            .iter()
            .filter(|alternative| alternative.id != target.id)
            .cloned()
            .collect::<Vec<_>>();
        inactive.push(DialogueAlternative::new(
            self.current,
            active_text,
            demoted_runs,
        ));
        let alternatives = Self::new(target.id, inactive)?;
        Ok(DialogueAltEdit {
            alternatives: Some(alternatives),
            active_id: target.id,
            text: target.text,
            runs: promoted_runs,
        })
    }

    /// Deletes Current and promotes greatest-lower, otherwise least-greater,
    /// inactive ID. The final inactive entry removes semantic Alt state.
    pub fn delete_current(
        &self,
        dialogue_definition: &ElementDefinition,
    ) -> Result<DialogueAltEdit, DialogueAltError> {
        self.validate_ids()?;
        if !self.can_delete_current() {
            return Err(DialogueAltError::RequiresMultipleVariants);
        }
        let target = self
            .inactive
            .iter()
            .filter(|alternative| alternative.id < self.current)
            .max_by_key(|alternative| alternative.id)
            .or_else(|| {
                self.inactive
                    .iter()
                    .filter(|alternative| alternative.id > self.current)
                    .min_by_key(|alternative| alternative.id)
            })
            .ok_or(DialogueAltError::MissingTarget)?
            .clone();
        let remaining = self
            .inactive
            .iter()
            .filter(|alternative| alternative.id != target.id)
            .cloned()
            .collect::<Vec<_>>();
        let alternatives = if remaining.is_empty() {
            None
        } else {
            Some(Self::new(target.id, remaining)?)
        };
        let runs = sparse_alt_run_projection(
            dialogue_definition,
            &target.text,
            &target.runs,
        )?;
        Ok(DialogueAltEdit {
            alternatives,
            active_id: target.id,
            text: target.text,
            runs,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogueAltAddIds {
    pub demoted_current: Option<DialogueAltId>,
    pub fresh_current: DialogueAltId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueAltEdit {
    pub alternatives: Option<DialogueAlternatives>,
    pub active_id: DialogueAltId,
    pub text: String,
    pub runs: Vec<TextRun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueAltError {
    DuplicateId,
    MissingDemotedId,
    UnexpectedDemotedId,
    TooManyVariants,
    RequiresMultipleVariants,
    MissingTarget,
    RunProjection(FormattedTextError),
}

impl From<FormattedTextError> for DialogueAltError {
    fn from(error: FormattedTextError) -> Self {
        Self::RunProjection(error)
    }
}
