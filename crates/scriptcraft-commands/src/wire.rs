use std::{collections::BTreeMap, fmt};

use scriptcraft_model::Project;
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::catalog::validate_descriptor_catalog;
use super::{CatalogError, Command, EditorSurface, ElementTypeValue, Selection, SelectionError};

/// Current version of the command-envelope contract.
pub const COMMAND_WIRE_VERSION: u16 = 2;
/// Previous command-envelope version accepted for command 22 only.
pub const COMMAND_WIRE_VERSION_1: u16 = 1;

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
#[derive(Debug, Clone, PartialEq)]
pub struct CommandEnvelope {
    pub version: u16,
    pub origin: CommandOrigin,
    pub command: Command,
}

impl Serialize for CommandEnvelope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut envelope = serializer.serialize_struct("CommandEnvelope", 3)?;
        // Serialization is always the current canonical wire representation,
        // including when a decoded v1 envelope is forwarded by a producer.
        envelope.serialize_field("version", &COMMAND_WIRE_VERSION)?;
        envelope.serialize_field("origin", &self.origin)?;
        envelope.serialize_field("command", &self.command)?;
        envelope.end()
    }
}

impl<'de> Deserialize<'de> for CommandEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnvelopeVisitor;

        impl<'de> Visitor<'de> for EnvelopeVisitor {
            type Value = CommandEnvelope;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a versioned command envelope")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut version = None;
                let mut origin = None;
                let mut command = None;

                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "version" => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            version = Some(map.next_value()?);
                        }
                        "origin" => {
                            if origin.is_some() {
                                return Err(de::Error::duplicate_field("origin"));
                            }
                            origin = Some(map.next_value()?);
                        }
                        "command" => {
                            if command.is_some() {
                                return Err(de::Error::duplicate_field("command"));
                            }
                            command = Some(map.next_value::<RawCommand>()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &field,
                                &["version", "origin", "command"],
                            ))
                        }
                    }
                }

                let version = version.ok_or_else(|| de::Error::missing_field("version"))?;
                let origin = origin.ok_or_else(|| de::Error::missing_field("origin"))?;
                let command = command.ok_or_else(|| de::Error::missing_field("command"))?;
                let command = command.into_command(version).map_err(de::Error::custom)?;
                Ok(CommandEnvelope {
                    version,
                    origin,
                    command,
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let version = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let origin = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let command: RawCommand = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let command = command.into_command(version).map_err(de::Error::custom)?;
                Ok(CommandEnvelope {
                    version,
                    origin,
                    command,
                })
            }
        }

        deserializer.deserialize_struct(
            "CommandEnvelope",
            &["version", "origin", "command"],
            EnvelopeVisitor,
        )
    }
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
        if !matches!(self.version, COMMAND_WIRE_VERSION_1 | COMMAND_WIRE_VERSION) {
            return Err(EnvelopeError::UnsupportedVersion(self.version));
        }
        validate_descriptor_catalog().map_err(EnvelopeError::InvalidCatalog)?;
        Ok(())
    }

    /// Performs document-dependent validation before command execution.
    pub fn validate_for_project(&self, project: &Project) -> Result<(), EnvelopeError> {
        self.validate()?;
        match &self.command {
            Command::EditorElementChange { element_type, .. }
                if project
                    .element_definitions()
                    .get(element_type.as_key())
                    .is_none() =>
            {
                Err(EnvelopeError::InvalidElementTarget)
            }
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

/// Captures the small command discriminator and parameter map without
/// duplicating the stable command catalog. Parameter entries remain ordered
/// here so command 22 can reject repeated representations before conversion.
struct RawCommand {
    id: String,
    params: Option<RawParams>,
}

impl<'de> Deserialize<'de> for RawCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RawCommandVisitor;

        impl<'de> Visitor<'de> for RawCommandVisitor {
            type Value = RawCommand;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a tagged command with an id and optional params")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut id = None;
                let mut params = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "id" => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        "params" => {
                            if params.is_some() {
                                return Err(de::Error::duplicate_field("params"));
                            }
                            params = Some(map.next_value()?);
                        }
                        _ => return Err(de::Error::unknown_field(&field, &["id", "params"])),
                    }
                }
                Ok(RawCommand {
                    id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                    params,
                })
            }
        }

        deserializer.deserialize_map(RawCommandVisitor)
    }
}

struct CheckedValue(serde_value::Value);

impl<'de> Deserialize<'de> for CheckedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CheckedValueVisitor;

        impl<'de> Visitor<'de> for CheckedValueVisitor {
            type Value = CheckedValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a Serde value without duplicate map keys")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::Bool(value)))
            }

            fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::U8(value)))
            }

            fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::U16(value)))
            }

            fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::U32(value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::U64(value)))
            }

            fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::I8(value)))
            }

            fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::I16(value)))
            }

            fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::I32(value)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::I64(value)))
            }

            fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::F32(value)))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::F64(value)))
            }

            fn visit_char<E>(self, value: char) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::Char(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_string(value.to_owned())
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::String(value)))
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_byte_buf(value.to_owned())
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::Bytes(value)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::Unit))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(CheckedValue(serde_value::Value::Option(None)))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = CheckedValue::deserialize(deserializer)?.0;
                Ok(CheckedValue(serde_value::Value::Option(Some(Box::new(
                    value,
                )))))
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = CheckedValue::deserialize(deserializer)?.0;
                Ok(CheckedValue(serde_value::Value::Newtype(Box::new(value))))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<CheckedValue>()? {
                    values.push(value.0);
                }
                Ok(CheckedValue(serde_value::Value::Seq(values)))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = BTreeMap::new();
                while let Some(key) = map.next_key::<serde_value::Value>()? {
                    let value = map.next_value::<CheckedValue>()?.0;
                    if values.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate nested command map key"));
                    }
                }
                Ok(CheckedValue(serde_value::Value::Map(values)))
            }
        }

        deserializer.deserialize_any(CheckedValueVisitor)
    }
}

struct RawParams(Vec<(String, CheckedValue)>);

impl<'de> Deserialize<'de> for RawParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RawParamsVisitor;

        impl<'de> Visitor<'de> for RawParamsVisitor {
            type Value = RawParams;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a command parameter map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    entries.push((key, map.next_value()?));
                }
                Ok(RawParams(entries))
            }
        }

        deserializer.deserialize_map(RawParamsVisitor)
    }
}

impl RawCommand {
    fn into_command(self, version: u16) -> Result<Command, String> {
        if self.id == "editor.element.change" {
            return self.into_element_change(version);
        }

        let mut command = BTreeMap::new();
        command.insert(
            serde_value::Value::String("id".to_owned()),
            serde_value::Value::String(self.id),
        );
        if let Some(params) = self.params {
            command.insert(
                serde_value::Value::String("params".to_owned()),
                serde_value::Value::Map(params.into_map()?),
            );
        }
        Command::deserialize(serde_value::Value::Map(command)).map_err(|error| error.to_string())
    }

    fn into_element_change(self, version: u16) -> Result<Command, String> {
        let params = self
            .params
            .ok_or_else(|| "editor.element.change requires params".to_owned())?;
        let mut element_type = None;
        let mut role = None;
        let mut surface = EditorSurface::Body;
        let mut surface_seen = false;

        for (field, value) in params.0 {
            match field.as_str() {
                "element_type" => {
                    if element_type.is_some() {
                        return Err("duplicate field `element_type`".to_owned());
                    }
                    element_type = Some(
                        ElementTypeValue::deserialize(value.0)
                            .map_err(|error| error.to_string())?,
                    );
                }
                "role" => {
                    if role.is_some() {
                        return Err("duplicate field `role`".to_owned());
                    }
                    role = Some(String::deserialize(value.0).map_err(|error| error.to_string())?);
                }
                "surface" => {
                    if surface_seen {
                        return Err("duplicate field `surface`".to_owned());
                    }
                    surface =
                        EditorSurface::deserialize(value.0).map_err(|error| error.to_string())?;
                    surface_seen = true;
                }
                _ => return Err(format!("unknown field `{field}` in editor.element.change")),
            }
        }

        let element_type = match version {
            COMMAND_WIRE_VERSION_1 if element_type.is_none() => {
                let old_role = role.ok_or_else(|| {
                    "version 1 editor.element.change requires exactly one `role`".to_owned()
                })?;
                decode_version_one_role(&old_role)
                    .ok_or_else(|| "unknown version 1 editor element role".to_owned())?
            }
            COMMAND_WIRE_VERSION if role.is_none() => element_type.ok_or_else(|| {
                "version 2 editor.element.change requires exactly one `element_type`".to_owned()
            })?,
            COMMAND_WIRE_VERSION_1 => {
                return Err("version 1 accepts `role` only for command 22".to_owned())
            }
            COMMAND_WIRE_VERSION => {
                return Err("version 2 accepts `element_type` only for command 22".to_owned())
            }
            _ => return Err(format!("unsupported command wire version: {version}")),
        };

        Ok(Command::EditorElementChange {
            element_type,
            surface,
        })
    }
}

/// Historical command-wire v1 vocabulary. Kept private to envelope decoding;
/// it is not a model registry or runtime Role authority.
fn decode_version_one_role(value: &str) -> Option<ElementTypeValue> {
    let label = match value {
        "scene_heading" => "Scene Heading",
        "action" => "Action",
        "character" => "Character",
        "dialogue" => "Dialogue",
        "parenthetical" => "Parenthetical",
        "transition" => "Transition",
        "note" => "Note",
        "new_act" => "New Act",
        "end_of_act" => "End of Act",
        "cast_list" => "Cast List",
        "general" => "General",
        "shot" => "Shot",
        "sequence" => "Sequence",
        "outline_1" => "Outline 1",
        "outline_2" => "Outline 2",
        "outline_3" => "Outline 3",
        "summary" => "Summary",
        _ => return None,
    };
    ElementTypeValue::new(label).ok()
}

impl RawParams {
    fn into_map(self) -> Result<BTreeMap<serde_value::Value, serde_value::Value>, String> {
        let mut values = BTreeMap::new();
        for (key, value) in self.0 {
            let key = serde_value::Value::String(key);
            if values.insert(key, value.0).is_some() {
                return Err("duplicate command parameter field".to_owned());
            }
        }
        Ok(values)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    UnsupportedVersion(u16),
    InvalidCatalog(CatalogError),
    /// The exact command target is absent from the current project registry.
    InvalidElementTarget,
    InvalidSelection(SelectionError),
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported command wire version: {version}")
            }
            Self::InvalidCatalog(error) => write!(formatter, "invalid command catalog: {error}"),
            Self::InvalidElementTarget => {
                formatter.write_str("element target is not registered in the current project")
            }
            Self::InvalidSelection(error) => {
                write!(formatter, "invalid command selection: {error}")
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}
