//! Bounded projection of the supported direct-root ScriptNotes region.

use crate::content::direct_paragraph_overrides;
use crate::diagnostics::{map_model_error, FdxError};
use crate::reader::{attribute, RawElement, RawNode};
use scriptcraft_model::body_offsets::{flat_document_len, flat_offset_to_position};
use scriptcraft_model::formatting::SourceParagraphOverrides;
use scriptcraft_model::script_notes::{ScriptNote, ScriptNoteParagraph, ScriptNoteRange, ScriptNotes};
use scriptcraft_model::{Project, RunAttribute, StableId, TextRun};
use std::collections::BTreeMap;

fn is_fdx_element(element: &RawElement, name: &str, namespace: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == namespace
}

fn direct_children<'a>(parent: &'a RawElement, name: &'a str, namespace: Option<&'a str>) -> impl Iterator<Item = &'a RawElement> {
    parent.children.iter().filter_map(move |node| match node {
        RawNode::Element(element) if is_fdx_element(element, name, namespace) => Some(element),
        _ => None,
    })
}

fn unique_attribute<'a>(element: &'a RawElement, name: &str) -> Option<&'a str> {
    let mut found = None;
    for attribute in &element.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == name {
            if found.is_some() { return None; }
            found = Some(attribute.value.as_str());
        }
    }
    found
}

fn attribute_count(element: &RawElement, name: &str) -> usize {
    element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    }).count()
}

fn parse_range(value: &str) -> Option<(usize, usize)> {
    let (start, end) = value.split_once(',')?;
    if end.contains(',') || start.is_empty() || end.is_empty()
        || !start.bytes().all(|byte| byte.is_ascii_digit())
        || !end.bytes().all(|byte| byte.is_ascii_digit())
    { return None; }
    Some((start.parse().ok()?, end.parse().ok()?))
}

pub(crate) fn has_supported_range(note: &RawElement, project: &Project) -> bool {
    let Some(length) = flat_document_len(project) else { return false; };
    (attribute_count(note, "Range") == 1)
        .then(|| unique_attribute(note, "Range"))
        .flatten()
        .and_then(parse_range)
        .is_some_and(|(start, end)| start <= end && end <= length
            && flat_offset_to_position(project, start).is_some()
            && flat_offset_to_position(project, end).is_some())
}

fn text_runs(paragraph: &RawElement, namespace: Option<&str>) -> Vec<TextRun> {
    let mut runs = Vec::new();
    for text_element in direct_children(paragraph, "Text", namespace) {
        let mut text = String::new();
        for child in &text_element.children {
            match child {
                RawNode::Text(value) | RawNode::CData(value) => text.push_str(value),
                RawNode::Element(_) | RawNode::Comment(_) | RawNode::ProcessingInstruction { .. } => {}
            }
        }
        if text.is_empty() { continue; }
        let attributes = text_element.attributes.iter().map(|attribute| RunAttribute {
            name: attribute.name.qualified_name.clone().unwrap_or_else(|| attribute.name.local_name.clone()),
            value: attribute.value.clone(),
        }).collect();
        runs.push(TextRun { text, attributes });
    }
    runs
}

/// Install supported direct notes only after source preservation and Body projection exist.
pub(crate) fn import(root: &RawElement, project: &mut Project) -> Result<(), FdxError> {
    let namespace = root.name.namespace_uri.as_deref();
    let Some(container) = direct_children(root, "ScriptNotes", namespace).next() else {
        return Ok(());
    };
    let candidates = direct_children(container, "ScriptNote", namespace).enumerate().collect::<Vec<_>>();
    let mut refs = BTreeMap::<String, usize>::new();
    let mut numeric = BTreeMap::<String, usize>::new();
    let mut ranges = Vec::with_capacity(candidates.len());
    for (_, note) in &candidates {
        let parsed = has_supported_range(note, project).then(|| unique_attribute(note, "Range"))
            .flatten().and_then(parse_range);
        if parsed.is_some() {
            if let Some(id) = unique_attribute(note, "RefId").filter(|id| !id.is_empty()) {
                let count = refs.entry(id.to_owned()).or_default();
                *count = count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
            }
            if let Some(id) = unique_attribute(note, "Id").filter(|id| !id.is_empty()) {
                let count = numeric.entry(id.to_owned()).or_default();
                *count = count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
            }
        }
        ranges.push(parsed);
    }

    let mut allocator = project.stable_id_allocator().map_err(map_model_error)?;
    let mut notes = Vec::new();
    for ((source_ordinal, note), range) in candidates.into_iter().zip(ranges) {
        let Some((start, end)) = range else { continue; };
        let ref_id = unique_attribute(note, "RefId").filter(|id| !id.is_empty())
            .filter(|id| refs.get(*id) == Some(&1)).map(str::to_owned);
        let numeric_id = unique_attribute(note, "Id").filter(|id| !id.is_empty())
            .filter(|id| numeric.get(*id) == Some(&1)).map(str::to_owned);
        let note_id = allocator.allocate("script-note").map_err(map_model_error)?;
        let mut paragraphs = Vec::new();
        for (paragraph_ordinal, paragraph) in direct_children(note, "Paragraph", namespace).enumerate() {
            let runs = text_runs(paragraph, namespace);
            let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
            let paragraph_id = allocator.allocate("script-note-paragraph").map_err(map_model_error)?;
            let durable_id = (identity_attribute_count(paragraph) <= 1)
                .then(|| unique_attribute_case_insensitive(paragraph, "id"))
                .flatten().filter(|id| !id.is_empty()).map(str::to_owned);
            paragraphs.push(ScriptNoteParagraph {
                id: paragraph_id,
                durable_id,
                source_paragraph_ordinal: Some(paragraph_ordinal),
                text,
                runs,
                direct_settings: direct_paragraph_overrides(paragraph).unwrap_or_default(),
            });
        }
        if paragraphs.is_empty() {
            paragraphs.push(ScriptNoteParagraph::empty(
                allocator.allocate("script-note-paragraph").map_err(map_model_error)?,
                None,
                None,
                SourceParagraphOverrides::default(),
            ));
        }
        let note = ScriptNote {
            id: note_id,
            numeric_id,
            ref_id,
            source_note_ordinal: Some(source_ordinal),
            range: ScriptNoteRange::new(start, end).ok_or(FdxError::InvalidValue)?,
            color: attribute(note, "Color").unwrap_or("").to_owned(),
            title: attribute(note, "Name").unwrap_or("").to_owned(),
            note_type: attribute(note, "Type").filter(|value| !value.is_empty()).map(str::to_owned),
            writer_id: attribute(note, "WriterID").unwrap_or("").to_owned(),
            writer_name: attribute(note, "WriterName").unwrap_or("").to_owned(),
            date_time: attribute(note, "DateTime").unwrap_or("").to_owned(),
            date_modified: attribute(note, "DateModified").unwrap_or("").to_owned(),
            paragraphs,
        };
        notes.push(note);
    }
    project.replace_script_notes(ScriptNotes::new(notes)).map_err(map_model_error)
}

fn unique_attribute_case_insensitive<'a>(element: &'a RawElement, name: &str) -> Option<&'a str> {
    let mut found = None;
    for attribute in &element.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case(name) {
            if found.is_some() { return None; }
            found = Some(attribute.value.as_str());
        }
    }
    found
}

fn identity_attribute_count(element: &RawElement) -> usize {
    element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id")
    }).count()
}
