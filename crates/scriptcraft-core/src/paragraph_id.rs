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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_collisions_and_reserves_allocated_values() {
        let first = "b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned();
        let second = "17ebd8dd-7497-4e6d-99fd-a88fb7a2c620".to_owned();
        let mut allocator = ParagraphIdAllocator::new([first.clone()]);
        let mut candidates = [first.clone(), second.clone()].into_iter();

        let allocated = allocator
            .allocate_with(|| Ok(candidates.next().unwrap_or_default()))
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(allocated, second);
        assert_eq!(allocator.reserved.len(), 2);
    }

    #[test]
    fn collision_exhaustion_and_entropy_failure_are_typed() {
        let collision = "b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned();
        let mut allocator = ParagraphIdAllocator::new([collision.clone()]);
        let mut attempts = 0;
        let result = allocator.allocate_with(|| {
            attempts += 1;
            Ok(collision.clone())
        });
        assert_eq!(result, Err(ParagraphIdAllocationError::CollisionLimit));
        assert_eq!(attempts, MAX_ALLOCATION_ATTEMPTS);

        let entropy = allocator.allocate_with(|| {
            Err(ParagraphIdAllocationError::EntropyUnavailable)
        });
        assert_eq!(entropy, Err(ParagraphIdAllocationError::EntropyUnavailable));
    }

    #[test]
    fn generated_ids_have_uuid_v4_shape_and_variant() {
        let mut allocator = ParagraphIdAllocator::new(std::iter::empty());
        let value = allocator.allocate().unwrap_or_else(|_| unreachable!());
        let parsed = uuid::Uuid::parse_str(&value).unwrap_or_else(|_| unreachable!());

        assert_eq!(value, value.to_ascii_lowercase());
        assert_eq!(value.len(), 36);
        assert_eq!(parsed.get_version(), Some(uuid::Version::Random));
        assert_eq!(parsed.get_variant(), uuid::Variant::RFC4122);
    }
}
