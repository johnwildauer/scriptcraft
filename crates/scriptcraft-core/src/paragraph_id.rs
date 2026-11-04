//! Shared allocator for newly generated FDX paragraph identities.

use std::collections::BTreeSet;

const MAX_ALLOCATION_ATTEMPTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParagraphIdAllocationError {
    EntropyUnavailable,
    CollisionLimit,
}

#[derive(Debug, Clone)]
pub(crate) struct ParagraphIdAllocator {
    reserved: BTreeSet<String>,
}

impl ParagraphIdAllocator {
    pub(crate) fn new(reserved: impl IntoIterator<Item = String>) -> Self {
        Self {
            reserved: reserved.into_iter().collect(),
        }
    }

    pub(crate) fn allocate(&mut self) -> Result<String, ParagraphIdAllocationError> {
        self.allocate_with(|| {
            let mut bytes = [0; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| ParagraphIdAllocationError::EntropyUnavailable)?;
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            Ok(uuid::Uuid::from_bytes(bytes).hyphenated().to_string())
        })
    }

    fn allocate_with(
        &mut self,
        mut generate: impl FnMut() -> Result<String, ParagraphIdAllocationError>,
    ) -> Result<String, ParagraphIdAllocationError> {
        for _ in 0..MAX_ALLOCATION_ATTEMPTS {
            let candidate = generate()?;
            if self.reserved.insert(candidate.clone()) {
                return Ok(candidate);
            }
        }
        Err(ParagraphIdAllocationError::CollisionLimit)
    }
}
