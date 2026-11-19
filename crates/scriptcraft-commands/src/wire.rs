use std::fmt;

use scriptcraft_model::Project;
use serde::{Deserialize, Serialize};

use super::catalog::validate_descriptor_catalog;
use super::{CatalogError, Command, Selection, SelectionError};

/// Current version of the command-envelope contract.
pub const COMMAND_WIRE_VERSION: u16 = 1;

/// Diagnostic/routing source. Origins never change command semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOrigin {
    Menu,
    Toolbar,
    NativeMenu,
    Shortcut,
    Keyboard,
    Pointer,
    ImeCommit,
    InternalFollowUp,
    Cli,
    Mcp,
    Test,
    NativeOs,
}

/// Serializable command plus diagnostic origin and wire version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEnvelope {
    pub version: u16,
    pub origin: CommandOrigin,
    pub command: Command,
}

impl CommandEnvelope {
    pub const fn new(origin: CommandOrigin, command: Command) -> Self {
        Self {
            version: COMMAND_WIRE_VERSION,
            origin,
            command,
        }
    }

    pub fn validate(&self) -> Result<(), EnvelopeError> {
        if self.version != COMMAND_WIRE_VERSION {
            return Err(EnvelopeError::UnsupportedVersion(self.version));
        }
        validate_descriptor_catalog().map_err(EnvelopeError::InvalidCatalog)?;
        Ok(())
    }

    /// Performs document-dependent validation before command execution.
    pub fn validate_for_project(&self, project: &Project) -> Result<(), EnvelopeError> {
        self.validate()?;
        match &self.command {
            Command::EditorSelectionSet { selection, surface } => selection
                .validate_for_surface(project, *surface)
                .map_err(EnvelopeError::InvalidSelection),
            Command::EditorSelectionPointer {
                position, surface, ..
            } => Selection {
                anchor: *position,
                focus: *position,
            }
            .validate_for_surface(project, *surface)
            .map_err(EnvelopeError::InvalidSelection),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    UnsupportedVersion(u16),
    InvalidCatalog(CatalogError),
    InvalidSelection(SelectionError),
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported command wire version: {version}")
            }
            Self::InvalidCatalog(error) => write!(formatter, "invalid command catalog: {error}"),
            Self::InvalidSelection(error) => {
                write!(formatter, "invalid command selection: {error}")
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}
