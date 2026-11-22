//! Bounded read-only extraction of Preview descriptions from built-in FDXT data.

use std::collections::BTreeSet;

use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::reader::NsReader;
use quick_xml::XmlVersion;

use crate::builtin_templates::BuiltinTemplateId;
use crate::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH, MAX_SOURCE_EVENTS,
    MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES,
};

/// Returns the optional Description on the first direct-root Preview element
/// in the selected embedded template.
pub fn builtin_template_preview_description(id: BuiltinTemplateId) -> Option<String> {
    read_preview_description(crate::builtin_templates::template_bytes(id))
}

fn read_preview_description(input: &[u8]) -> Option<String> {
    if input.len() > MAX_SOURCE_BYTES {
        return None;
    }

    let bytes = input.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(input);
    let mut reader = NsReader::from_reader(bytes);
    let config = reader.config_mut();
    config.trim_text(false);
    config.check_end_names = true;
    config.allow_unmatched_ends = false;
    config.check_comments = true;
    reader
        .resolver_mut()
        .set_max_declarations_per_element(MAX_ATTRIBUTES_PER_ELEMENT);

    let mut buffer = Vec::new();
    let mut event_count = 0usize;
    let mut text_bytes = 0usize;
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut root_namespace: Option<String> = None;

    loop {
        let event = reader.read_event_into(&mut buffer).ok()?;
        event_count = event_count.checked_add(1)?;
        if event_count > MAX_SOURCE_EVENTS {
            return None;
        }

        match event {
            Event::Start(start) => {
                if depth >= MAX_SOURCE_DEPTH {
                    return None;
                }
                let element = inspect_start(&reader, &start)?;
                if !root_seen {
                    if depth != 0 || element.local_name != "FinalDraft" {
                        return None;
                    }
                    root_namespace = element.namespace_uri;
                    root_seen = true;
                    depth = 1;
                } else {
                    if depth == 1 && is_preview_in_root_namespace(&element, &root_namespace) {
                        return element.description;
                    }
                    depth += 1;
                }
            }
            Event::Empty(start) => {
                let element = inspect_start(&reader, &start)?;
                if !root_seen {
                    if depth != 0 || element.local_name != "FinalDraft" {
                        return None;
                    }
                    return None;
                }
                if depth == 1 && is_preview_in_root_namespace(&element, &root_namespace) {
                    return element.description;
                }
            }
            Event::End(_) => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    return None;
                }
            }
            Event::Text(text) => {
                let decoded = text.decode().ok()?;
                let value = unescape(&decoded).ok()?;
                account_text(&mut text_bytes, value.len())?;
                crate::reader::validate_xml_chars(&value).ok()?;
                if depth == 0 && !value.chars().all(char::is_whitespace) {
                    return None;
                }
            }
            Event::GeneralRef(reference) => {
                let reference = reference.decode().ok()?;
                let encoded = format!("&{reference};");
                let value = unescape(&encoded).ok()?;
                account_text(&mut text_bytes, value.len())?;
                crate::reader::validate_xml_chars(&value).ok()?;
                if depth == 0 && !value.chars().all(char::is_whitespace) {
                    return None;
                }
            }
            Event::CData(text) => {
                let value = text.decode().ok()?;
                account_text(&mut text_bytes, value.len())?;
                crate::reader::validate_xml_chars(&value).ok()?;
                if depth == 0 && !value.chars().all(char::is_whitespace) {
                    return None;
                }
            }
            Event::Comment(comment) => {
                let value = comment.decode().ok()?;
                account_text(&mut text_bytes, value.len())?;
                crate::reader::validate_xml_chars(&value).ok()?;
            }
            Event::PI(instruction) => {
                if instruction.target().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
                    return None;
                }
                let target = std::str::from_utf8(instruction.target()).ok()?;
                let data = reader.decoder().decode(instruction.content()).ok()?;
                account_text(
                    &mut text_bytes,
                    target.len().checked_add(data.len())?,
                )?;
                crate::reader::validate_xml_chars(&data).ok()?;
            }
            Event::Decl(declaration) => {
                if declaration.as_ref().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
                    return None;
                }
                if let Some(encoding) = declaration.encoding() {
                    let encoding = encoding.ok()?;
                    let encoding = std::str::from_utf8(&encoding).ok()?;
                    if !encoding.eq_ignore_ascii_case("UTF-8") {
                        return None;
                    }
                }
            }
            Event::DocType(_) => return None,
            Event::Eof => return None,
        }

        buffer.clear();
    }
}

struct ElementMetadata {
    namespace_uri: Option<String>,
    local_name: String,
    description: Option<String>,
}

fn inspect_start<R: std::io::BufRead>(
    reader: &NsReader<R>,
    start: &BytesStart<'_>,
) -> Option<ElementMetadata> {
    if start.name().as_ref().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
        return None;
    }
    let (resolved, local) = reader.resolver().resolve_element(start.name());
    let (namespace_uri, local_name) = resolved_name(resolved, local.as_ref())?;

    let mut seen_raw_names = BTreeSet::new();
    let mut seen_expanded_names = BTreeSet::new();
    let mut description = None;
    let mut attribute_count = 0usize;

    for item in start.attributes().with_checks(false) {
        let attribute = item.ok()?;
        attribute_count = attribute_count.checked_add(1)?;
        if attribute_count > MAX_ATTRIBUTES_PER_ELEMENT
            || attribute.key.as_ref().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
            || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
        {
            return None;
        }

        let raw_name = std::str::from_utf8(attribute.key.as_ref()).ok()?.to_owned();
        if !seen_raw_names.insert(raw_name) {
            return None;
        }

        let (attribute_namespace, attribute_local) =
            reader.resolver().resolve_attribute(attribute.key);
        let (attribute_namespace, attribute_local) =
            resolved_name(attribute_namespace, attribute_local.as_ref())?;
        if !seen_expanded_names.insert((attribute_namespace.clone(), attribute_local.clone())) {
            return None;
        }

        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .ok()?;
        if value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
            return None;
        }
        crate::reader::validate_xml_chars(&value).ok()?;

        if attribute_namespace.is_none() && attribute_local == "Description" {
            if description.is_some() {
                return None;
            }
            description = Some(value.into_owned());
        }
    }

    Some(ElementMetadata {
        namespace_uri,
        local_name,
        description,
    })
}

fn resolved_name(
    resolved: ResolveResult<'_>,
    local_name: &[u8],
) -> Option<(Option<String>, String)> {
    if local_name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
        return None;
    }
    let namespace_uri = match resolved {
        ResolveResult::Bound(Namespace(uri)) => Some(std::str::from_utf8(uri).ok()?.to_owned()),
        ResolveResult::Unbound => None,
        ResolveResult::Unknown(_) => return None,
    };
    Some((namespace_uri, std::str::from_utf8(local_name).ok()?.to_owned()))
}

fn is_preview_in_root_namespace(
    element: &ElementMetadata,
    root_namespace: &Option<String>,
) -> bool {
    element.local_name == "Preview" && element.namespace_uri.as_ref() == root_namespace.as_ref()
}

fn account_text(total: &mut usize, bytes: usize) -> Option<()> {
    *total = total.checked_add(bytes)?;
    (*total <= MAX_SOURCE_TEXT_BYTES).then_some(())
}
