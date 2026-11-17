use crate::ModelError;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(String);

impl StableId {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ModelError::EmptyId);
        }
        if value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\')
        {
            return Err(ModelError::InvalidId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Allocates deterministic IDs that do not collide with any reserved ID.
/// Foreign source IDs must be kept in their own namespace and are not reserved here.
#[derive(Debug, Clone)]
pub struct StableIdAllocator {
    reserved: BTreeSet<StableId>,
    next_serial: u64,
}

impl Default for StableIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl StableIdAllocator {
    pub fn new() -> Self {
        Self {
            reserved: BTreeSet::new(),
            next_serial: 1,
        }
    }

    pub fn reserve(&mut self, id: StableId) -> Result<(), ModelError> {
        if !self.reserved.insert(id) {
            return Err(ModelError::DuplicateId);
        }
        Ok(())
    }

    pub fn allocate(&mut self, prefix: &str) -> Result<StableId, ModelError> {
        loop {
            let serial = self.next_serial;
            self.next_serial = self
                .next_serial
                .checked_add(1)
                .ok_or(ModelError::InvalidNumber)?;
            let candidate = StableId::new(format!("{prefix}-{serial}"))?;
            if self.reserved.insert(candidate.clone()) {
                return Ok(candidate);
            }
        }
    }
}
