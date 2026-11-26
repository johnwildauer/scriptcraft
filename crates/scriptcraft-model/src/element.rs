use crate::{
    dialogue_alts::DialogueAlternatives, element_type::ElementTypeKey, identity::StableId,
    validation::validate_element, ModelError,
    MAX_TEXT_BYTES,
};
use std::collections::BTreeSet;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_BODY_OWNER_REVISION: AtomicU64 = AtomicU64::new(1);

fn next_body_owner_revision() -> u64 {
    NEXT_BODY_OWNER_REVISION.fetch_add(1, Ordering::Relaxed)
}

pub const MAX_RUNS_PER_ELEMENT: usize = 1_024;
pub const MAX_RUN_ATTRIBUTES: usize = 128;
pub const MAX_RUN_ATTRIBUTE_BYTES: usize = 4_096;
pub const MAX_PROVENANCE_STRING_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub text: String,
    /// Ordered source attributes are retained as bounded opaque name/value pairs.
    pub attributes: Vec<RunAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunAttribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Fdx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSection {
    Content,
    TitlePage,
}

/// Minimal source coordinates for later ordered preservation; this is not the
/// source tree or a claim that the source can yet be safely rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProvenance {
    pub format: SourceFormat,
    pub section: SourceSection,
    pub ordinal: u32,
    pub foreign_id: Option<String>,
    pub foreign_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayElement {
    pub id: StableId,
    /// Canonical exact, document-owned element-type identity.
    pub element_type: ElementTypeKey,
    /// Plain-text compatibility projection used by existing editing/layout clients.
    pub text: String,
    /// Empty for legacy plain-text elements; populated runs remain source ordered.
    pub runs: Vec<TextRun>,
    /// Inactive Dialogue variants travel with this owner and remain dormant
    /// when its current element type is not Dialogue-capable.
    pub dialogue_alternatives: Option<DialogueAlternatives>,
    pub source_provenance: Option<SourceProvenance>,
}

impl ScreenplayElement {
    /// Returns the canonical exact element-type key.
    pub fn element_type(&self) -> ElementTypeKey {
        self.element_type.clone()
    }

    /// Sets the canonical key without changing any other paragraph value.
    pub fn set_element_type(&mut self, element_type: ElementTypeKey) {
        self.element_type = element_type;
    }

    pub fn new(
        id: impl Into<String>,
        element_type: impl Into<ElementTypeKey>,
        text: impl Into<String>,
    ) -> Result<Self, ModelError> {
        let id = StableId::new(id)?;
        let text = text.into();
        if text.len() > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        Ok(Self {
            id,
            element_type: element_type.into(),
            text,
            runs: Vec::new(),
            dialogue_alternatives: None,
            source_provenance: None,
        })
    }

    pub fn with_runs(
        id: impl Into<String>,
        element_type: impl Into<ElementTypeKey>,
        runs: Vec<TextRun>,
        source_provenance: Option<SourceProvenance>,
    ) -> Result<Self, ModelError> {
        if runs.len() > MAX_RUNS_PER_ELEMENT {
            return Err(ModelError::TooManyRuns);
        }
        let attribute_count = runs.iter().fold(0usize, |count, run| {
            count.saturating_add(run.attributes.len())
        });
        if attribute_count > MAX_RUN_ATTRIBUTES {
            return Err(ModelError::TooManyRunAttributes);
        }
        let mut text = String::new();
        for run in &runs {
            if text.len().saturating_add(run.text.len()) > MAX_TEXT_BYTES {
                return Err(ModelError::TextTooLong);
            }
            text.push_str(&run.text);
        }
        let element = Self {
            id: StableId::new(id)?,
            element_type: element_type.into(),
            text,
            runs,
            dialogue_alternatives: None,
            source_provenance,
        };
        validate_element(&element)?;
        Ok(element)
    }

    pub fn is_boundary(&self, byte_offset: usize) -> bool {
        self.text.is_char_boundary(byte_offset)
    }

    pub fn validate_boundary(&self, byte_offset: usize) -> Result<(), ModelError> {
        if self.is_boundary(byte_offset) {
            Ok(())
        } else {
            Err(ModelError::InvalidUtf8Boundary)
        }
    }
}

/// Immutable canonical Body content plus its ephemeral local revision.
///
/// `Project` stores each owner in an `Arc`, so project/candidate/history clones
/// share allocations. There is deliberately no mutable dereference: semantic
/// updates go through `BodyOwner::update`, which allocates only the changed
/// owner. Revisions never participate in equality or serialization.
#[derive(Debug)]
pub struct BodyOwner {
    element: ScreenplayElement,
    revision: u64,
}

impl BodyOwner {
    pub fn new(element: ScreenplayElement) -> Self {
        Self {
            element,
            revision: next_body_owner_revision(),
        }
    }

    pub fn shared(element: ScreenplayElement) -> Arc<Self> {
        Arc::new(Self::new(element))
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn as_element(&self) -> &ScreenplayElement {
        &self.element
    }

    /// Applies copy-on-write to one shared owner. A no-op retains both pointer
    /// identity and revision; a semantic change receives a fresh allocation.
    pub fn update<R>(owner: &mut Arc<Self>, update: impl FnOnce(&mut ScreenplayElement) -> R) -> R {
        let mut candidate = owner.element.clone();
        let result = update(&mut candidate);
        if candidate != owner.element {
            *owner = Self::shared(candidate);
        }
        result
    }

    /// Direct Body formatting lives in the project's override maps rather than
    /// in `ScreenplayElement`; callers invoke this only when that state changed.
    pub(crate) fn revise(owner: &mut Arc<Self>) {
        *owner = Self::shared(owner.element.clone());
    }
}

impl Deref for BodyOwner {
    type Target = ScreenplayElement;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

impl From<ScreenplayElement> for BodyOwner {
    fn from(element: ScreenplayElement) -> Self {
        Self::new(element)
    }
}

impl PartialEq for BodyOwner {
    fn eq(&self, other: &Self) -> bool {
        self.element == other.element
    }
}

impl Eq for BodyOwner {}

pub fn body_owners_equal(left: &[Arc<BodyOwner>], right: &[Arc<BodyOwner>]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| Arc::ptr_eq(left, right) || left == right)
}
