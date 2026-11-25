//! Detached retained-source reconciliation and preservation-free ScriptNotes output.

use crate::content::fixed_inches_string;
use crate::diagnostics::FdxError;
use crate::tree::{new_lineage_element, plain_source_name};
use scriptcraft_model::preservation::{PreservedAttribute, PreservedElement, PreservedNode, PreservedSection, SourceName};
use scriptcraft_model::{Project, ScriptNote, ScriptNoteParagraph, TextRun};
use std::collections::{BTreeMap, BTreeSet};

fn child_index(parent: &PreservedElement, name: &str) -> Option<usize> {
    parent.children.iter().position(|node| matches!(node,
        PreservedNode::Element(child) if child.name.local_name == name
            && child.name.namespace_uri == parent.name.namespace_uri))
}

fn child_mut(parent: &mut PreservedElement, index: usize) -> Result<&mut PreservedElement, FdxError> {
    match parent.children.get_mut(index) {
        Some(PreservedNode::Element(element)) => Ok(element),
        _ => Err(FdxError::InvalidValue),
    }
}

fn raw_attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    let mut found = None;
    for attribute in &element.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == name {
            if found.is_some() { return None; }
            found = Some(attribute.value.as_str());
        }
    }
    found
}

fn identity<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    raw_attribute(element, name)
}

fn set_attribute(element: &mut PreservedElement, name: &str, value: &str) -> Result<(), FdxError> {
    let indexes = element.attributes.iter().enumerate().filter_map(|(index, attribute)| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name == name).then_some(index)
    }).collect::<Vec<_>>();
    match indexes.as_slice() {
        [] => element.attributes.push(PreservedAttribute { name: plain_source_name(name), value: value.to_owned() }),
        [index] => element.attributes[*index].value = value.to_owned(),
        _ => return Err(FdxError::AmbiguousIdentity),
    }
    Ok(())
}

fn set_single_attribute(element: &mut PreservedElement, name: &str, value: &str) {
    let indexes = element.attributes.iter().enumerate().filter_map(|(index, attribute)| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name == name).then_some(index)
    }).collect::<Vec<_>>();
    if let Some(index) = indexes.first().copied() {
        element.attributes[index].value = value.to_owned();
        for index in indexes.iter().skip(1).rev() { element.attributes.remove(*index); }
    } else {
        element.attributes.push(PreservedAttribute { name: plain_source_name(name), value: value.to_owned() });
    }
}

fn generated_run(parent: &SourceName, run: &TextRun) -> PreservedElement {
    let mut text = new_lineage_element(parent, "Text", Vec::new());
    text.attributes = run.attributes.iter().map(|attribute| PreservedAttribute {
        name: plain_source_name(&attribute.name), value: attribute.value.clone(),
    }).collect();
    if !run.text.is_empty() { text.children.push(PreservedNode::Text(run.text.clone().into())); }
    text
}

fn generated_note_indent(value: scriptcraft_model::formatting::FixedPoint) -> String {
    if value.raw() == 0 { "0.00".into() } else { fixed_inches_string(value) }
}

fn update_run_shell(text: &mut PreservedElement, run: &TextRun) {
    text.attributes = run.attributes.iter().map(|attribute| PreservedAttribute {
        name: text.attributes.iter().find(|source| {
            source.name.qualified_name.as_deref().unwrap_or(&source.name.local_name) == attribute.name
        }).map(|source| source.name.clone()).unwrap_or_else(|| plain_source_name(&attribute.name)),
        value: attribute.value.clone(),
    }).collect();
    let mut inserted = false;
    let mut children = Vec::new();
    for child in std::mem::take(&mut text.children) {
        match child {
            PreservedNode::Text(_) | PreservedNode::CData(_) => {
                if !inserted && !run.text.is_empty() {
                    children.push(PreservedNode::Text(run.text.clone().into()));
                    inserted = true;
                }
            }
            other => children.push(other),
        }
    }
    if !inserted && !run.text.is_empty() { children.insert(0, PreservedNode::Text(run.text.clone().into())); }
    text.children = children.into();
}

fn reconcile_text(paragraph: &mut PreservedElement, semantic: &ScriptNoteParagraph) -> Result<(), FdxError> {
    let raw = crate::beat_board_output::raw_element(paragraph);
    let source_runs = crate::content::semantic_text_runs(&raw, false, false)?;
    if source_runs == semantic.runs { return Ok(()); }
    let namespace = paragraph.name.namespace_uri.clone();
    let mut runs = semantic.runs.iter();
    let mut children = Vec::new();
    for node in std::mem::take(&mut paragraph.children) {
        let PreservedNode::Element(mut text) = node else { children.push(node); continue };
        if text.name.local_name != "Text" || text.name.namespace_uri != namespace {
            children.push(PreservedNode::Element(text));
            continue;
        }
        if let Some(run) = runs.next() {
            update_run_shell(&mut text, run);
            children.push(PreservedNode::Element(text));
        } else {
            text.children = text.children.into_iter().filter(|node| !matches!(node,
                PreservedNode::Text(_) | PreservedNode::CData(_))).collect::<Vec<_>>().into();
            if !text.children.is_empty() || !text.attributes.is_empty() || !text.namespace_bindings.is_empty() {
                children.push(PreservedNode::Element(text));
            }
        }
    }
    for run in runs { children.push(PreservedNode::Element(generated_run(&paragraph.name, run))); }
    paragraph.children = children.into();
    Ok(())
}

fn reconcile_setting<T: PartialEq>(
    element: &mut PreservedElement,
    name: &str,
    old: Option<T>,
    new: Option<T>,
    format: impl FnOnce(T) -> String,
) -> Result<(), FdxError> {
    if old == new { return Ok(()); }
    match new {
        Some(value) => set_attribute(element, name, &format(value)),
        None => {
            let matches = element.attributes.iter().enumerate().filter_map(|(index, attribute)|
                (attribute.name.namespace_uri.is_none() && attribute.name.local_name == name).then_some(index)
            ).collect::<Vec<_>>();
            if matches.len() > 1 { return Err(FdxError::AmbiguousIdentity); }
            if let Some(index) = matches.first() { element.attributes.remove(*index); }
            Ok(())
        }
    }
}

fn reconcile_settings(element: &mut PreservedElement, paragraph: &ScriptNoteParagraph) -> Result<(), FdxError> {
    let old = crate::content::direct_paragraph_overrides(&crate::beat_board_output::raw_element(element)).unwrap_or_default();
    let new = paragraph.direct_settings;
    reconcile_setting(element, "Alignment", old.paragraph.alignment, new.paragraph.alignment, |value| value.fdx_value().into())?;
    reconcile_setting(element, "FirstIndent", old.paragraph.first_indent, new.paragraph.first_indent, fixed_inches_string)?;
    reconcile_setting(element, "Leading", old.paragraph.leading, new.paragraph.leading, |value| value.fdx_value().into())?;
    reconcile_setting(element, "LeftIndent", old.paragraph.left_indent, new.paragraph.left_indent, fixed_inches_string)?;
    reconcile_setting(element, "RightIndent", old.paragraph.right_indent, new.paragraph.right_indent, fixed_inches_string)?;
    reconcile_setting(element, "SpaceBefore", old.paragraph.space_before, new.paragraph.space_before, |value| value.decimal_string())?;
    reconcile_setting(element, "Spacing", old.paragraph.spacing, new.paragraph.spacing, |value| value.fdx_value().into())?;
    reconcile_setting(element, "StartsNewPage", old.starts_new_page, new.starts_new_page, |value| if value { "Yes".into() } else { "No".into() })?;
    Ok(())
}

fn generated_paragraph(parent: &SourceName, paragraph: &ScriptNoteParagraph) -> Result<PreservedElement, FdxError> {
    let settings = paragraph.direct_settings;
    let spec = settings.paragraph;
    let (Some(alignment), Some(first), Some(leading), Some(left), Some(right), Some(before), Some(spacing), Some(new_page)) = (
        spec.alignment, spec.first_indent, spec.leading, spec.left_indent, spec.right_indent,
        spec.space_before, spec.spacing, settings.starts_new_page,
    ) else { return Err(FdxError::InvalidValue); };
    let mut result = new_lineage_element(parent, "Paragraph", vec![
        ("Alignment".into(), alignment.fdx_value().into()),
        ("FirstIndent".into(), generated_note_indent(first)),
        ("Leading".into(), leading.fdx_value().into()),
        ("LeftIndent".into(), generated_note_indent(left)),
        ("OutlineLevel".into(), "0".into()),
        ("RightIndent".into(), if right.raw() == 61_200 { "8.50".into() } else { generated_note_indent(right) }),
        ("SpaceBefore".into(), before.decimal_string()),
        ("Spacing".into(), spacing.fdx_value().into()),
        ("StartsNewPage".into(), if new_page { "Yes".into() } else { "No".into() }),
        ("id".into(), paragraph.durable_id.clone().ok_or(FdxError::AmbiguousIdentity)?),
    ]);
    for run in &paragraph.runs { result.children.push(PreservedNode::Element(generated_run(&result.name, run))); }
    Ok(result)
}

fn raw_note_identity(element: &PreservedElement) -> (Option<&str>, Option<&str>) {
    (identity(element, "RefId"), identity(element, "Id"))
}

fn note_matches(source: &PreservedElement, baseline: &ScriptNote) -> bool {
    let (ref_id, numeric_id) = raw_note_identity(source);
    if let Some(expected) = baseline.ref_id.as_deref().filter(|value| !value.is_empty()) {
        if ref_id.is_some() { return ref_id == Some(expected); }
    }
    baseline.numeric_id.as_deref().filter(|value| !value.is_empty()).is_some_and(|expected| numeric_id == Some(expected))
}

fn paragraph_id(element: &PreservedElement) -> Option<&str> {
    let mut found = None;
    for attribute in &element.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id") {
            if found.is_some() { return None; }
            found = Some(attribute.value.as_str());
        }
    }
    found
}

fn reconcile_note(source: &mut PreservedElement, note: &ScriptNote, baseline: &ScriptNote) -> Result<(), FdxError> {
    reconcile_setting(
        source,
        "Color",
        (!baseline.color.is_empty()).then_some(baseline.color.as_str()),
        (!note.color.is_empty()).then_some(note.color.as_str()),
        str::to_owned,
    )?;
    if note.date_modified != baseline.date_modified { set_attribute(source, "DateModified", &note.date_modified)?; }
    if note.date_time != baseline.date_time { set_attribute(source, "DateTime", &note.date_time)?; }
    if note.numeric_id != baseline.numeric_id {
        set_attribute(source, "Id", note.numeric_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?)?;
    }
    if note.title != baseline.title { set_attribute(source, "Name", &note.title)?; }
    if note.range != baseline.range {
        set_attribute(source, "Range", &format!("{},{}", note.range.start(), note.range.end()))?;
    }
    if note.ref_id != baseline.ref_id {
        set_single_attribute(source, "RefId", note.ref_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?);
    }
    if note.note_type != baseline.note_type {
        set_attribute(source, "Type", note.note_type.as_deref().unwrap_or(""))?;
    } else if note.note_type.is_none() && raw_attribute(source, "Type").is_none() {
        set_attribute(source, "Type", "")?;
    }
    let mut source_type = None;
    for attribute in &source.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type" {
            if source_type.is_some() { return Err(FdxError::AmbiguousIdentity); }
            source_type = Some(attribute.value.clone());
        }
    }
    source.source_type = source_type;
    if note.writer_id != baseline.writer_id { set_attribute(source, "WriterID", &note.writer_id)?; }
    if note.writer_name != baseline.writer_name { set_attribute(source, "WriterName", &note.writer_name)?; }

    let namespace = source.name.namespace_uri.clone();
    let paragraph_slots = source.children.iter().enumerate().filter_map(|(index, node)| match node {
        PreservedNode::Element(element) if element.name.local_name == "Paragraph" && element.name.namespace_uri == namespace => Some(index),
        _ => None,
    }).collect::<Vec<_>>();
    let mut source_by_id = BTreeMap::<String, PreservedElement>::new();
    let mut idless = BTreeMap::<usize, PreservedElement>::new();
    for (ordinal, index) in paragraph_slots.iter().enumerate() {
        let PreservedNode::Element(paragraph) = &source.children[*index] else { return Err(FdxError::InvalidValue); };
        if let Some(id) = paragraph_id(paragraph) {
            if source_by_id.insert(id.to_owned(), paragraph.clone()).is_some() { return Err(FdxError::AmbiguousIdentity); }
        } else { idless.insert(ordinal, paragraph.clone()); }
    }
    let mut used_ordinals = BTreeSet::new();
    let mut desired = Vec::new();
    for paragraph in &note.paragraphs {
        let durable = paragraph.durable_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?;
        let mut retained = source_by_id.remove(durable);
        if retained.is_none() && paragraph.source_paragraph_ordinal.is_some() {
            let ordinal = paragraph.source_paragraph_ordinal.ok_or(FdxError::AmbiguousIdentity)?;
            if !used_ordinals.insert(ordinal) { return Err(FdxError::AmbiguousIdentity); }
            let mut shell = idless.remove(&ordinal).ok_or(FdxError::AmbiguousIdentity)?;
            set_attribute(&mut shell, "id", durable)?;
            retained = Some(shell);
        }
        let node = if let Some(mut shell) = retained {
            reconcile_settings(&mut shell, paragraph)?;
            reconcile_text(&mut shell, paragraph)?;
            shell
        } else {
            generated_paragraph(&source.name, paragraph)?
        };
        desired.push(PreservedNode::Element(node));
    }
    // A surviving note may remove only direct paragraphs represented by its prior
    // managed semantic projection; opaque descendants remain attached to shells.
    let slot_set = paragraph_slots.iter().copied().collect::<BTreeSet<_>>();
    let mut replacements = desired.into_iter();
    let mut children = Vec::new();
    for (index, child) in std::mem::take(&mut source.children).into_iter().enumerate() {
        if slot_set.contains(&index) {
            if let Some(replacement) = replacements.next() { children.push(replacement); }
        } else { children.push(child); }
        if Some(&index) == paragraph_slots.last() { children.extend(replacements.by_ref()); }
    }
    if paragraph_slots.is_empty() { children.extend(replacements); }
    source.children = children.into();
    Ok(())
}

fn generated_note(parent: &SourceName, note: &ScriptNote) -> Result<PreservedElement, FdxError> {
    let note_type = note.note_type.clone().unwrap_or_default();
    let mut result = new_lineage_element(parent, "ScriptNote", vec![
        ("DateModified".into(), note.date_modified.clone()),
        ("DateTime".into(), note.date_time.clone()),
        ("Id".into(), note.numeric_id.clone().ok_or(FdxError::AmbiguousIdentity)?),
        ("Name".into(), note.title.clone()),
        ("Range".into(), format!("{},{}", note.range.start(), note.range.end())),
        ("RefId".into(), note.ref_id.clone().ok_or(FdxError::AmbiguousIdentity)?),
        ("Type".into(), note_type.clone()),
        ("WriterID".into(), note.writer_id.clone()),
        ("WriterName".into(), note.writer_name.clone()),
    ]);
    result.source_type = Some(note_type);
    if !note.color.is_empty() {
        result.attributes.insert(0, PreservedAttribute {
            name: plain_source_name("Color"),
            value: note.color.clone(),
        });
    }
    for paragraph in &note.paragraphs {
        result.children.push(PreservedNode::Text("\n      ".into()));
        result.children.push(PreservedNode::Element(generated_paragraph(&result.name, paragraph)?));
    }
    if !note.paragraphs.is_empty() { result.children.push(PreservedNode::Text("\n    ".into())); }
    Ok(result)
}

/// Reconcile only the first selected note container, retaining every opaque child.
pub(crate) fn project_script_notes_source(
    sections: &[PreservedSection],
    project: &Project,
    original: &Project,
) -> Result<Vec<PreservedSection>, FdxError> {
    let mut sections = sections.to_vec();
    let root = sections.iter_mut().flat_map(|section| &mut section.children).find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => Some(element),
        _ => None,
    }).ok_or(FdxError::InvalidRoot)?;
    let namespace = root.name.namespace_uri.clone();
    let container_index = root.children.iter().position(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "ScriptNotes" && element.name.namespace_uri == namespace));
    if container_index.is_none() {
        if project.script_notes().is_empty() { return Ok(sections); }
        let mut container = new_lineage_element(&root.name, "ScriptNotes", Vec::new());
        for note in project.script_notes().iter() { container.children.push(PreservedNode::Element(generated_note(&container.name, note)?)); }
        // Place after Beat structures while leaving all unrelated source order intact.
        let position = root.children.iter().rposition(|node| matches!(node,
            PreservedNode::Element(element) if element.name.namespace_uri == namespace
                && matches!(element.name.local_name.as_str(), "ListItems" | "DisplayBoards"))).map_or(root.children.len(), |index| index + 1);
        root.children.insert(position, PreservedNode::Element(container));
        return Ok(sections);
    }
    let container_index = container_index.ok_or(FdxError::InvalidValue)?;
    let container = child_mut(root, container_index)?;
    let container_namespace = container.name.namespace_uri.clone();
    let originals = original.script_notes().iter().collect::<Vec<_>>();
    let current = project.script_notes().iter().collect::<Vec<_>>();
    let mut used_current = BTreeSet::new();
    let mut children = Vec::new();
    let mut source_ordinal = 0usize;
    for child in std::mem::take(&mut container.children) {
        let PreservedNode::Element(mut source_note) = child else { children.push(child); continue };
        if source_note.name.local_name != "ScriptNote" || source_note.name.namespace_uri != container_namespace {
            children.push(PreservedNode::Element(source_note));
            continue;
        }
        let ordinal = source_ordinal;
        source_ordinal = source_ordinal.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        let source_model = originals.iter().copied().find(|note| note.source_note_ordinal == Some(ordinal));
        let Some(source_model) = source_model else {
            children.push(PreservedNode::Element(source_note));
            continue;
        };
        let matching_indices = if note_matches(&source_note, source_model) {
            current.iter().enumerate().filter_map(|(index, note)|
                (note.id == source_model.id).then_some(index)
            ).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let matched_index = if matching_indices.len() == 1 {
            matching_indices.first().copied()
        } else if matching_indices.len() > 1 {
            return Err(FdxError::AmbiguousIdentity);
        } else if source_model.durable_identity().is_none() {
            let fallback = current.iter().enumerate().filter(|(_, note)| note.source_note_ordinal == Some(ordinal))
                .map(|(index, _)| index).collect::<Vec<_>>();
            if fallback.len() > 1 { return Err(FdxError::AmbiguousIdentity); }
            fallback.first().copied()
        } else { None };
        if let Some(index) = matched_index {
            if !used_current.insert(index) { return Err(FdxError::AmbiguousIdentity); }
            reconcile_note(&mut source_note, current[index], source_model)?;
            children.push(PreservedNode::Element(source_note));
        }
        // A supported source note absent from the current semantic collection
        // was explicitly removed. Unsupported source subtrees never enter here.
    }
    for (index, note) in current.iter().enumerate() {
        if used_current.contains(&index) { continue; }
        if note.source_note_ordinal.is_some() { return Err(FdxError::AmbiguousIdentity); }
        children.push(PreservedNode::Element(generated_note(&container.name, note)?));
    }
    container.children = children.into();
    Ok(sections)
}

/// Emit a deterministic ScriptNotes container in preservation-free output.
pub(crate) fn append_generated_script_notes(root: &mut PreservedElement, project: &Project) -> Result<(), FdxError> {
    let mut container = new_lineage_element(&root.name, "ScriptNotes", Vec::new());
    for note in project.script_notes().iter() {
        container.children.push(PreservedNode::Element(generated_note(&container.name, note)?));
    }
    root.children.push(PreservedNode::Element(container));
    Ok(())
}
