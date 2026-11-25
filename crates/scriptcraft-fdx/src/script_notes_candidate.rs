//! Detached durable-identity completion for ScriptNote serialization candidates.

use crate::diagnostics::{map_model_error, map_paragraph_id_error, FdxError};
use scriptcraft_model::formatting::{
    FixedPoint, ParagraphAlignment, ParagraphLeading, ParagraphSpacing, SourceParagraphOverrides,
};
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::preservation::{ForeignIdDomain, ForeignIdentity};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{Project, RunAttribute, ScriptNote, ScriptNotes, TextRun};
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub(crate) struct ScriptNoteIdentityChanges {
    pub(crate) added: Vec<ForeignIdentity>,
    pub(crate) removed: BTreeSet<ForeignIdentity>,
}

fn note_identity(value: &str) -> Result<ForeignIdentity, FdxError> {
    ForeignIdentity::new(ForeignIdDomain::Other("ScriptNote".to_owned()), value.to_owned())
        .map_err(crate::diagnostics::map_preservation_error)
}

fn paragraph_identity(value: &str) -> Result<ForeignIdentity, FdxError> {
    ForeignIdentity::new(ForeignIdDomain::Paragraph, value.to_owned())
        .map_err(crate::diagnostics::map_preservation_error)
}

fn checked_numeric(value: &str) -> Result<Option<u64>, FdxError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) { return Ok(None); }
    value.parse::<u64>().map(Some).map_err(|_| FdxError::InvalidValue)
}

fn collect_raw_ids(nodes: &[PreservedNode], ids: &mut BTreeSet<String>) {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue; };
        for attribute in &element.attributes {
            if attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id") {
                ids.insert(attribute.value.clone());
            }
        }
        collect_raw_ids(&element.children, ids);
    }
}

fn collect_note_identity_values(nodes: &[PreservedNode], ids: &mut BTreeSet<String>, refs: &mut BTreeSet<String>) {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue; };
        if element.name.local_name == "ScriptNote" {
            for attribute in &element.attributes {
                if attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Id" {
                    ids.insert(attribute.value.clone());
                }
                if attribute.name.namespace_uri.is_none() && attribute.name.local_name == "RefId" {
                    refs.insert(attribute.value.clone());
                }
            }
        }
        collect_note_identity_values(&element.children, ids, refs);
    }
}

fn selected_source_note_paragraph_ids(project: &Project, paragraph_identities: &BTreeSet<String>, note_identities: &BTreeSet<String>) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let Some(preservation) = &project.preservation else { return result; };
    for section in preservation.sections.iter() {
        for node in section.children.iter() {
            let PreservedNode::Element(root) = node else { continue; };
            if root.name.local_name != "FinalDraft" { continue; }
            let namespace = root.name.namespace_uri.clone();
            let Some(container) = root.children.iter().find_map(|node| match node {
                PreservedNode::Element(element) if element.name.local_name == "ScriptNotes"
                    && element.name.namespace_uri == namespace => Some(element),
                _ => None,
            }) else { continue; };
            for (ordinal, note) in container.children.iter().filter_map(|node| match node {
                PreservedNode::Element(element) if element.name.local_name == "ScriptNote"
                    && element.name.namespace_uri == container.name.namespace_uri => Some(element),
                _ => None,
            }).enumerate() {
                let numeric_id = note.attributes.iter().find(|attribute| attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name == "Id").map(|attribute| attribute.value.as_str());
                let semantically_known = note_identities.iter().any(|id| Some(id.as_str()) == numeric_id)
                    || project.script_notes().iter().any(|semantic| semantic.source_note_ordinal == Some(ordinal));
                let in_bounds = crate::beat_board_output::raw_element(note);
                let supported = semantically_known || crate::script_notes::has_supported_range(&in_bounds, project);
                if !supported { continue; }
                for paragraph in note.children.iter().filter_map(|child| match child {
                    PreservedNode::Element(element) if element.name.local_name == "Paragraph"
                        && element.name.namespace_uri == note.name.namespace_uri => Some(element),
                    _ => None,
                }) {
                    let mut ids = paragraph.attributes.iter().filter(|attribute| attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id"));
                    if let Some(id) = ids.next().map(|attribute| attribute.value.as_str()) {
                        if ids.next().is_none() && paragraph_identities.contains(id) { result.insert(id.to_owned()); }
                    }
                }
            }
        }
    }
    result
}

fn generated_settings() -> SourceParagraphOverrides {
    let mut settings = SourceParagraphOverrides::default();
    settings.paragraph.alignment = Some(ParagraphAlignment::Left);
    settings.paragraph.first_indent = Some(FixedPoint::from_raw(0));
    settings.paragraph.left_indent = Some(FixedPoint::from_raw(0));
    settings.paragraph.right_indent = Some(FixedPoint::from_raw(61_200));
    settings.paragraph.leading = Some(ParagraphLeading::Regular);
    settings.paragraph.space_before = Some(FixedPoint::from_raw(0));
    settings.paragraph.spacing = Some(ParagraphSpacing::Single);
    settings.starts_new_page = Some(false);
    settings
}

fn default_generated_run(run: &mut TextRun) {
    if run.attributes.is_empty() {
        run.attributes = vec![
            RunAttribute { name: "AdornmentStyle".into(), value: "0".into() },
            RunAttribute { name: "Font".into(), value: "Arial".into() },
            RunAttribute { name: "RevisionID".into(), value: "0".into() },
            RunAttribute { name: "Size".into(), value: "12".into() },
            RunAttribute { name: "Style".into(), value: String::new() },
        ];
    }
}

/// Reserve source/current durable values before allocating, then complete the
/// borrowed Project on a detached validated clone.
pub fn finalize_script_notes_candidate(project: &Project) -> Result<Project, FdxError> {
    finalize_script_notes_candidate_with_changes(project).map(|(candidate, _)| candidate)
}

pub(crate) fn finalize_script_notes_candidate_with_changes(
    project: &Project,
) -> Result<(Project, ScriptNoteIdentityChanges), FdxError> {
    let source_identities = project.preservation.as_ref().map(|state| state.identities.as_slice()).unwrap_or(&[]);
    let source_note_ids = source_identities.iter().filter(|identity| {
        identity.domain == ForeignIdDomain::Other("ScriptNote".to_owned())
    }).map(|identity| identity.value.clone()).collect::<BTreeSet<_>>();
    let all_source_paragraph_ids = source_identities.iter().filter(|identity| {
        identity.domain == ForeignIdDomain::Paragraph
    }).map(|identity| identity.value.clone()).collect::<BTreeSet<_>>();
    let source_paragraph_ids = selected_source_note_paragraph_ids(project, &all_source_paragraph_ids, &source_note_ids);

    let mut all_raw_ids = BTreeSet::new();
    let mut raw_note_ids = BTreeSet::new();
    let mut raw_note_ref_ids = BTreeSet::new();
    if let Some(preservation) = &project.preservation {
        for section in preservation.sections.iter() {
            collect_raw_ids(&section.children, &mut all_raw_ids);
            collect_note_identity_values(&section.children, &mut raw_note_ids, &mut raw_note_ref_ids);
        }
    }
    let mut used_numeric = source_note_ids.clone();
    used_numeric.extend(raw_note_ids.iter().cloned());
    let mut maximum = None;
    for id in &source_note_ids {
        if let Some(value) = checked_numeric(id)? { maximum = maximum.max(Some(value)); }
    }
    let mut reserved_uuid = source_note_ids.clone();
    reserved_uuid.extend(source_paragraph_ids.iter().cloned());
    reserved_uuid.extend(all_raw_ids.iter().cloned());
    reserved_uuid.extend(raw_note_ref_ids);
    for note in project.script_notes().iter() {
        if let Some(id) = note.numeric_id.as_ref().filter(|id| !id.is_empty()) {
            used_numeric.insert(id.clone());
            if let Some(value) = checked_numeric(id)? { maximum = maximum.max(Some(value)); }
            reserved_uuid.insert(id.clone());
        }
        if let Some(id) = &note.ref_id { reserved_uuid.insert(id.clone()); }
        for paragraph in &note.paragraphs {
            if let Some(id) = &paragraph.durable_id {
                reserved_uuid.insert(id.clone());
                used_numeric.insert(id.clone());
            }
        }
    }
    let mut uuid = ParagraphIdAllocator::new(reserved_uuid);
    let mut candidate = project.clone();
    let mut notes = candidate.script_notes().iter().cloned().collect::<Vec<ScriptNote>>();
    let source_owned_numeric_ids = notes.iter().filter(|note| note.source_note_ordinal.is_some())
        .filter_map(|note| note.numeric_id.as_deref())
        .filter(|id| source_note_ids.contains(*id))
        .map(str::to_owned).collect::<BTreeSet<_>>();
    let mut changes = ScriptNoteIdentityChanges::default();
    let mut live_note_ids = BTreeSet::new();
    let mut live_ref_ids = BTreeSet::new();
    let mut live_paragraph_ids = BTreeSet::new();

    for note in &mut notes {
        let generated_numeric = if note.numeric_id.as_deref().is_none_or(str::is_empty) {
            let mut next = maximum.map_or(0, |value| value.checked_add(1).unwrap_or(u64::MAX));
            loop {
                if next == u64::MAX && maximum == Some(u64::MAX) { return Err(FdxError::InvalidValue); }
                let lexical = next.to_string();
                if used_numeric.insert(lexical.clone()) {
                    maximum = Some(next);
                    note.numeric_id = Some(lexical.clone());
                    break Some(lexical);
                }
                next = next.checked_add(1).ok_or(FdxError::InvalidValue)?;
            }
        } else { None };
        let numeric_id = note.numeric_id.as_deref().filter(|id| !id.is_empty()).ok_or(FdxError::InvalidValue)?;
        if (source_note_ids.contains(numeric_id) || raw_note_ids.contains(numeric_id))
            && !source_owned_numeric_ids.contains(numeric_id)
        { return Err(FdxError::AmbiguousIdentity); }
        if !live_note_ids.insert(numeric_id.to_owned()) { return Err(FdxError::AmbiguousIdentity); }
        if let Some(id) = generated_numeric { used_numeric.insert(id); }
        if note.ref_id.as_deref().is_none_or(str::is_empty) {
            let id = uuid.allocate().map_err(map_paragraph_id_error)?;
            note.ref_id = Some(id);
        }
        let ref_id = note.ref_id.as_deref().filter(|id| !id.is_empty()).ok_or(FdxError::InvalidValue)?;
        if !live_ref_ids.insert(ref_id.to_owned()) { return Err(FdxError::AmbiguousIdentity); }
        for paragraph in &mut note.paragraphs {
            let generated = paragraph.source_paragraph_ordinal.is_none();
            if paragraph.durable_id.as_deref().is_none_or(str::is_empty) {
                paragraph.durable_id = Some(uuid.allocate().map_err(map_paragraph_id_error)?);
            }
            let durable_id = paragraph.durable_id.clone().ok_or(FdxError::InvalidValue)?;
            if all_raw_ids.contains(&durable_id) && !source_paragraph_ids.contains(&durable_id) {
                return Err(FdxError::AmbiguousIdentity);
            }
            if !live_paragraph_ids.insert(durable_id.clone()) { return Err(FdxError::AmbiguousIdentity); }
            if generated {
                paragraph.direct_settings = generated_settings();
            }
            if generated && !paragraph.text.is_empty() {
                for run in &mut paragraph.runs { default_generated_run(run); }
            }
            paragraph.normalize().map_err(|_| FdxError::InvalidValue)?;
            if !source_paragraph_ids.contains(&durable_id) {
                changes.added.push(paragraph_identity(&durable_id)?);
            }
        }
        if !source_note_ids.contains(numeric_id) {
            changes.added.push(note_identity(numeric_id)?);
        }
    }
    for id in source_note_ids.difference(&live_note_ids) {
        changes.removed.insert(note_identity(id)?);
    }
    for id in source_paragraph_ids.difference(&live_paragraph_ids) {
        changes.removed.insert(paragraph_identity(id)?);
    }
    candidate.replace_script_notes(ScriptNotes::new(notes)).map_err(map_model_error)?;
    candidate.validate().map_err(map_model_error)?;
    Ok((candidate, changes))
}
