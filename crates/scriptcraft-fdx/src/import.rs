use crate::document::{
    FdxImportCandidate, FdxImportOutcome, FdxMigrationConsent, FdxMigrationKind,
    FdxVersionDirection, FdxVersionStatus,
};
use crate::{preflight_version, read_v6, write_v6, FdxError};

/// Import Version 6 directly, or attempt a typed migration after the caller has
/// shown the required warning and supplied explicit consent. Older files retain
/// only TitlePage and Content, with unrecognized paragraph types converted to
/// General. Newer files use the Version 6 importer best-effort and retain source
/// sections where representable. Neither path mutates the source or a session.
pub fn import_fdx(
    input: &[u8],
    consent: FdxMigrationConsent,
) -> Result<FdxImportOutcome, FdxError> {
    match preflight_version(input)? {
        FdxVersionStatus::Version6 => Ok(FdxImportOutcome::Candidate(Box::new(
            FdxImportCandidate {
                document: read_v6(input)?,
                migration: None,
            },
        ))),
        FdxVersionStatus::ExplicitNon6 {
            declared,
            direction,
        } => {
            if consent == FdxMigrationConsent::Cancel {
                return Ok(FdxImportOutcome::Cancelled {
                    declared_version: declared,
                });
            }
            let migrated = crate::version::prepare_non_v6_source(input, direction)?;
            let document = read_v6(&migrated)?;
            // Verify the complete bounded Version 6 projection before publishing
            // the staged migration candidate.
            write_v6(&document.project)?;
            let migration = match direction {
                FdxVersionDirection::Older => FdxMigrationKind::OlderLossy { declared },
                FdxVersionDirection::Newer => FdxMigrationKind::NewerBestEffort { declared },
            };
            Ok(FdxImportOutcome::Candidate(Box::new(FdxImportCandidate {
                document,
                migration: Some(migration),
            })))
        }
        FdxVersionStatus::Missing => Err(FdxError::MissingVersion),
        FdxVersionStatus::Malformed => Err(FdxError::InvalidVersion),
        FdxVersionStatus::Duplicate => Err(FdxError::DuplicateVersion),
    }
}
