//! Model-owned operations for managed preservation text projection and topology.

use super::*;
use crate::formatting::ParagraphAlignment;
use crate::{element::{Role, RunAttribute, ScreenplayElement, TextRun}, identity::StableId, title_page::TitlePageParagraph};
use std::collections::{BTreeMap, BTreeSet};
use super::navigation::find_managed_element_counted;
use super::title::is_title_content_location;
use super::title::source_attribute_matches_run;
use super::reconciliation::plain_source_name;

impl PreservationState {
    /// Updates exactly one unqualified managed Paragraph attribute while
    /// retaining its original ordered slot and every unrelated source value.
    pub fn set_managed_paragraph_attribute(
        &mut self,
        id: &StableId,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), PreservationError> {
        let paragraph = self.managed_element_mut_indexed(id)?;
        if paragraph.name.local_name != "Paragraph" {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let matching = paragraph
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, attribute)| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matching.len() > 1 {
            return Err(PreservationError::AmbiguousDirectAttribute);
        }
        match (matching.first().copied(), value) {
            (Some(index), Some(value)) => paragraph.attributes[index].value = value.to_owned(),
            (Some(index), None) => {
                paragraph.attributes.remove(index);
            }
            (None, Some(value)) => paragraph.attributes.push(PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: name.to_owned(),
                    qualified_name: Some(name.to_owned()),
                },
                value: value.to_owned(),
            }),
            (None, None) => {}
        }
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

    /// Body run editing has no semantic slot for an opaque child between
    /// source Text elements. Reject that shape instead of moving or merging
    /// styled text across the preserved child.
    pub fn ensure_run_range_is_contiguous(
        &self,
        id: &StableId,
        range: std::ops::Range<usize>,
    ) -> Result<(), PreservationError> {
        let paragraph = self
            .managed_element(id)?
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut text_indexes = Vec::new();
        for (index, child) in paragraph.children.iter().enumerate() {
            if let PreservedNode::Element(element) = child {
                if is_text_element(element, paragraph) {
                    text_indexes.push(index);
                }
            }
        }
        let total_text = text_indexes.iter().filter_map(|index| match &paragraph.children[*index] {
            PreservedNode::Element(element) => Some(element.children.iter().filter_map(|content| match content {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.len()),
                _ => None,
            }).sum::<usize>()),
            _ => None,
        }).sum::<usize>();
        let clear_all_text = range.start == 0 && range.end == total_text;
        let mut boundary = 0usize;
        for (slot_index, index) in text_indexes.iter().enumerate() {
            let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
                return Err(PreservationError::UnsupportedRunTopology);
            };
            let run_start = boundary;
            let mut local_offset = 0usize;
            for child in &text_element.children {
                match child {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        local_offset = local_offset.saturating_add(text.len());
                    }
                    _ if !clear_all_text => {
                        let opaque_boundary = run_start.saturating_add(local_offset);
                        if (range.start < opaque_boundary && range.end > opaque_boundary)
                            || (range.start == range.end && range.start == opaque_boundary)
                        {
                            return Err(PreservationError::UnsupportedRunTopology);
                        }
                    }
                    _ => {}
                }
            }
            boundary = boundary.saturating_add(local_offset);
            if let Some(next_index) = text_indexes.get(slot_index + 1) {
                let gap = &paragraph.children[*index + 1..*next_index];
                let has_opaque_gap = gap.iter().any(|node| match node {
                    PreservedNode::Element(element) => !(element.name.local_name == "Alts"
                        && element.name.namespace_uri == paragraph.name.namespace_uri),
                    PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                    _ => true,
                });
                if !clear_all_text && has_opaque_gap
                    && ((range.start < boundary && range.end > boundary)
                        || (range.start == range.end && range.start == boundary))
                {
                    return Err(PreservationError::UnsupportedRunTopology);
                }
            }
        }
        Ok(())
    }

    /// Replaces the ordered source Text-node projection with an exact semantic
    /// run projection while retaining each node's expanded names, namespace
    /// bindings, and all run attributes. A non-Text child or source-text
    /// mismatch is a safe-stop; callers stage this against a detached project.
    pub fn sync_managed_text_runs(
        &mut self,
        id: &StableId,
        expected_text: &str,
        runs: &[TextRun],
    ) -> Result<(), PreservationError> {
        self.ensure_managed_paragraph_index();
        let title_source = self.indexed_location(id)?
            .is_some_and(|location| is_title_content_location(&self.sections, location));
        let paragraph = self.managed_element_mut(id)?;
        if title_source && !is_plain_managed_paragraph(paragraph) {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        if expected_text.is_empty() && runs.is_empty() {
            let mut remove = BTreeSet::new();
            let paragraph_name = paragraph.name.clone();
            for (index, node) in paragraph.children.iter_mut().enumerate() {
                let PreservedNode::Element(element) = node else { continue };
                if element.name.local_name != "Text"
                    || element.name.namespace_uri != paragraph_name.namespace_uri
                {
                    continue;
                }
                let wholly_managed = element.attributes.is_empty()
                    && element.namespace_bindings.is_empty()
                    && element.children.iter().all(|child| matches!(child,
                        PreservedNode::Text(_) | PreservedNode::CData(_)
                    ));
                if wholly_managed {
                    remove.insert(index);
                } else {
                    set_direct_text_content_preserving_opaque(element, "")?;
                }
            }
            paragraph.children = paragraph.children.drain(..).enumerate()
                .filter_map(|(index, child)| (!remove.contains(&index)).then_some(child)).collect();
            return Ok(());
        }
        let text_elements = paragraph.children.iter().enumerate().filter_map(|(index, node)| {
            match node {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => Some((index, element)),
                _ => None,
            }
        }).collect::<Vec<_>>();
        let source_text = text_elements.iter().flat_map(|(_, element)| element.children.iter()).filter_map(|node| {
            match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }
        }).collect::<String>();
        if source_text != expected_text {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        if text_elements.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let effective = if runs.is_empty() {
            vec![TextRun { text: expected_text.to_owned(), attributes: Vec::new() }]
        } else {
            runs.to_vec()
        };
        if effective.iter().map(|run| run.text.as_str()).collect::<String>() != expected_text {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut replacements = BTreeMap::<usize, Vec<PreservedNode>>::new();
        let mut text_offset = 0usize;
        for (index, template) in &text_elements {
            let segment_text = template.children.iter().filter_map(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>();
            let end = text_offset.saturating_add(segment_text.len());
            let mut segment_runs = slice_text_runs(&effective, text_offset..end)?;
            if segment_runs.is_empty() && !segment_text.is_empty() {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let mut replacement = Vec::with_capacity(segment_runs.len());
            let has_opaque_descendants = template.children.iter().any(|node| !matches!(node,
                PreservedNode::Text(_) | PreservedNode::CData(_)
            ));
            if has_opaque_descendants {
                let unchanged_run = match segment_runs.as_slice() {
                    [run] => run.text == segment_text && template.attributes.iter().all(|source| {
                        run.attributes.iter().any(|attribute| source_attribute_matches_run(source, attribute))
                    }) && run.attributes.len() == template.attributes.len(),
                    [] => segment_text.is_empty(),
                    _ => false,
                };
                if !unchanged_run {
                    return Err(PreservationError::UnsupportedRunTopology);
                }
                replacement.push(PreservedNode::Element((*template).clone()));
            } else {
                for run in segment_runs.drain(..) {
                    replacement.push(PreservedNode::Element(sync_text_element_attributes(template, &run)?));
                }
            }
            replacements.insert(*index, replacement);
            text_offset = end;
        }
        if text_offset != expected_text.len() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut rebuilt = Vec::with_capacity(paragraph.children.len().saturating_add(effective.len()));
        for (index, child) in paragraph.children.iter().enumerate() {
            if let Some(replacement) = replacements.remove(&index) {
                rebuilt.extend(replacement);
            } else {
                rebuilt.push(child.clone());
            }
        }
        paragraph.children = rebuilt.into();
        Ok(())
    }

    pub fn set_direct_paragraph_alignment(
        &mut self,
        id: &StableId,
        alignment: ParagraphAlignment,
    ) {
        let overrides = self
            .source_layout_settings
            .direct_paragraph_overrides
            .entry(id.clone())
            .or_default();
        overrides.paragraph.alignment = Some(alignment);
    }

    /// Reconciles a join or cross-paragraph deletion while retaining the
    /// surviving owner's Alts in place. The left source is trimmed through its
    /// mapped Text runs, and only the retained suffix Text runs of the final
    /// removed Paragraph are copied; removed Alts and AltCollection are never
    /// transferred or pruned.
    pub(super) fn reconcile_joined_text_nodes(
        &mut self,
        survivor_id: &StableId,
        original_survivor_text: &str,
        survivor_prefix_len: usize,
        final_removed_id: &StableId,
        final_removed_text: &str,
        final_suffix_start: usize,
        expected_result: &str,
        expected_runs: Option<&[String]>,
    ) -> Result<(), PreservationError> {
        let survivor_location = self.indexed_location(survivor_id)?
            .cloned().ok_or(PreservationError::ManagedAnchorMissing)?;
        let removed_location = self.indexed_location(final_removed_id)?
            .cloned().ok_or(PreservationError::ManagedAnchorMissing)?;
        let survivor = self.managed_element_at(&survivor_location)
            .filter(|element| matches!(&element.ownership, SourceOwnership::Managed(id) if id == survivor_id))
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let removed = self.managed_element_at(&removed_location)
            .filter(|element| matches!(&element.ownership, SourceOwnership::Managed(id) if id == final_removed_id))
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let survivor_runs = paragraph_text_nodes_with_supported_attachments(&survivor)?;
        let removed_runs = paragraph_text_nodes_with_supported_attachments(&removed)?;
        if text_of_nodes(&survivor_runs) != original_survivor_text
            || text_of_nodes(&removed_runs) != final_removed_text
            || survivor_prefix_len > original_survivor_text.len()
            || final_suffix_start > final_removed_text.len()
            || !original_survivor_text.is_char_boundary(survivor_prefix_len)
            || !final_removed_text.is_char_boundary(final_suffix_start)
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let suffix_runs = slice_source_text_nodes(&removed_runs, final_suffix_start)?;
        let suffix_text = text_of_nodes(&suffix_runs);
        if format!("{}{}", &original_survivor_text[..survivor_prefix_len], suffix_text)
            != expected_result
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        self.replace_managed_text(
            survivor_id,
            original_survivor_text,
            survivor_prefix_len,
            original_survivor_text.len(),
            "",
            expected_runs,
        )?;
        let target = self.managed_element_mut(survivor_id)?;
        target.children.extend(suffix_runs);
        Ok(())
    }

    /// Trims the right part of a split from the original Paragraph's mapped
    /// Text nodes. Opaque children, including Alts, stay on the original/left
    /// owner and are never copied to the new Paragraph.
    pub(super) fn trim_split_text(
        &mut self,
        id: &StableId,
        original_text: &str,
        split_offset: usize,
        expected_runs: Option<&[String]>,
    ) -> Result<(), PreservationError> {
        self.replace_managed_text(
            id,
            original_text,
            split_offset,
            original_text.len(),
            "",
            expected_runs,
        )
    }

    /// Applies a UTF-8 byte-range edit to the exact ordered text children of one
    /// managed paragraph. If semantic runs are present, each run must correspond
    /// one-to-one with a source text child; otherwise the source mapping is not
    /// sufficiently explicit to mutate safely. At a source-child boundary,
    /// insertion uses right affinity except at end-of-text, which uses left
    /// affinity; a replacement crossing an intervening opaque child is rejected.
    pub fn replace_managed_text(
        &mut self,
        id: &StableId,
        old_text: &str,
        start: usize,
        end: usize,
        inserted: &str,
        expected_runs: Option<&[String]>,
    ) -> Result<(), PreservationError> {
        if start > end
            || end > old_text.len()
            || !old_text.is_char_boundary(start)
            || !old_text.is_char_boundary(end)
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        self.ensure_managed_paragraph_index();
        let title_source = self.indexed_location(id)?
            .is_some_and(|location| is_title_content_location(&self.sections, location));
        let element = self.managed_element_mut(id)?;
        if title_source && !is_plain_managed_paragraph(element) {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        if element.children.iter().any(
            |node| matches!(node, PreservedNode::Element(child) if is_text_element(child, element)),
        ) {
            return replace_text_element_children(
                element,
                old_text,
                start,
                end,
                inserted,
                expected_runs,
            )
            .map_err(|error| {
                categorize_managed_text_mismatch(error, ManagedTextMismatchSite::TextReplacement)
            });
        }
        let indexes = direct_text_child_indexes(element);
        if indexes.is_empty() {
            return replace_text_element_children(
                element,
                old_text,
                start,
                end,
                inserted,
                expected_runs,
            )
            .map_err(|error| {
                categorize_managed_text_mismatch(error, ManagedTextMismatchSite::TextReplacement)
            });
        }
        let source_segments = indexes
            .iter()
            .map(|index| match &element.children[*index] {
                PreservedNode::Text(text) | PreservedNode::CData(text) => text.to_string(),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        if source_segments.concat() != old_text {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::TextReplacement,
            ));
        }
        if expected_runs.is_some_and(|runs| runs != source_segments.as_slice()) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut boundary = 0usize;
        let clear_all_text = start == 0 && end == old_text.len() && inserted.is_empty();
        for pair in indexes.windows(2) {
            let previous = pair[0];
            let next = pair[1];
            if element.children[previous + 1..next]
                .iter()
                .any(|node| !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_)))
            {
                boundary = boundary.saturating_add(match &element.children[previous] {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => text.len(),
                    _ => unreachable!(),
                });
                if start < boundary && end > boundary && !clear_all_text {
                    return Err(PreservationError::UnsupportedTextMapping);
                }
            } else {
                boundary = boundary.saturating_add(match &element.children[previous] {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => text.len(),
                    _ => unreachable!(),
                });
            }
        }
        let updated = replace_segments(&source_segments, start, end, inserted)?;
        for (index, text) in indexes.into_iter().zip(updated) {
            match &mut element.children[index] {
                PreservedNode::Text(value) | PreservedNode::CData(value) => *value = text.into(),
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// Replaces the complete plain-text projection of a managed FDX paragraph
    /// whose text is represented by ordered `<Text>` elements. The caller must
    /// establish that those runs carry no formatting; opaque or mixed children
    /// are rejected rather than moved or discarded.
    pub fn replace_managed_text_elements(
        &mut self,
        id: &StableId,
        expected_runs: &[String],
        replacement: &str,
    ) -> Result<(), PreservationError> {
        if replacement.len() > crate::MAX_TEXT_BYTES {
            return Err(PreservationError::TextTooLarge);
        }
        let paragraph = self.managed_element_mut(id)?;
        let text_elements = validate_managed_text_elements_replacement(paragraph, expected_runs)?;
        if expected_runs.is_empty() {
            let indexes = text_elements.into_iter().collect::<BTreeSet<_>>();
            paragraph.children = paragraph
                .children
                .drain(..)
                .enumerate()
                .filter_map(|(index, child)| (!indexes.contains(&index)).then_some(child))
                .collect();
            if !replacement.is_empty() {
                let (name, namespace_bindings) = paragraph_text_name(paragraph);
                let text_node = PreservedNode::Element(PreservedElement {
                        name,
                        attributes: Vec::new(),
                        namespace_bindings,
                        source_type: None,
                        ownership: SourceOwnership::Opaque,
                        dependency_confidence: DependencyConfidence::Unknown,
                        anchor: None,
                        children: vec![PreservedNode::Text(replacement.to_owned().into())].into(),
                    });
                let insertion_index = paragraph.children.iter().position(|node| !matches!(node,
                    PreservedNode::Element(element) if is_text_element(element, paragraph)
                )).unwrap_or(paragraph.children.len());
                paragraph.children.insert(insertion_index, text_node);
            }
            return Ok(());
        }

        if replacement.is_empty() {
            let indexes = text_elements.into_iter().collect::<BTreeSet<_>>();
            paragraph.children = paragraph
                .children
                .drain(..)
                .enumerate()
                .filter_map(|(index, child)| (!indexes.contains(&index)).then_some(child))
                .collect();
            return Ok(());
        }

        for (run_index, index) in text_elements.into_iter().enumerate() {
            let PreservedNode::Element(text_element) = &mut paragraph.children[index] else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            if let Some(first) = text_element.children.first_mut() {
                match first {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        *text = if run_index == 0 {
                            replacement.to_owned().into()
                        } else {
                            String::new().into()
                        };
                    }
                    _ => return Err(PreservationError::UnsupportedTextMapping),
                }
                for child in text_element.children.iter_mut().skip(1) {
                    match child {
                        PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                        _ => return Err(PreservationError::UnsupportedTextMapping),
                    }
                }
            } else if run_index == 0 {
                text_element
                    .children
                    .push(PreservedNode::Text(replacement.to_owned().into()));
            }
        }
        Ok(())
    }

    /// Checks that a complete plain-text replacement has an exact, supported
    /// source mapping without mutating the retained tree. Derived correction
    /// uses this to defer an unsupported scene before reconciling its arcs.
    pub fn validate_managed_text_elements_replacement(
        &self,
        id: &StableId,
        expected_runs: &[String],
    ) -> Result<(), PreservationError> {
        let paragraph = if let Some(location) = self.indexed_location(id)? {
            self.managed_element_at(location)
        } else {
            find_managed_element_counted(&self.sections, id)
        }
        .ok_or(PreservationError::ManagedAnchorMissing)?;
        validate_managed_text_elements_replacement(paragraph, expected_runs).map(|_| ())
    }

    /// Updates only a recognized paragraph Type mapping. All other source
    /// attributes and nested attachments remain byte/structure-owned by source.
    pub fn change_managed_role(
        &mut self,
        id: &StableId,
        old_role: &str,
        new_role: &str,
    ) -> Result<(), PreservationError> {
        let element = self.managed_element_mut(id)?;
        if element.source_type.as_deref() != Some(old_role) {
            return Err(PreservationError::UnsupportedRoleMapping);
        }
        let mut type_index = None;
        for (index, attribute) in element.attributes.iter().enumerate() {
            if attribute.name.local_name == "Type" {
                if type_index.is_some() || attribute.value != old_role {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
                type_index = Some(index);
            }
        }
        let Some(type_index) = type_index else {
            return Err(PreservationError::UnsupportedRoleMapping);
        };
        element.attributes[type_index].value = new_role.to_owned();
        element.source_type = Some(new_role.to_owned());
        Ok(())
    }

pub(super) fn replace_managed_projection(
        &mut self,
        semantic: &ScreenplayElement,
    ) -> Result<(), PreservationError> {
        let paragraph = self.managed_element_mut(&semantic.id)?;
        if !is_plain_managed_paragraph(paragraph) {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        let templates = paragraph
            .children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => {
                    Some(element.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if templates.iter().any(|template| template.children.iter().any(|node| {
            !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
        })) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let text_indexes = paragraph.children.iter().enumerate().filter_map(|(index, node)| {
            matches!(node, PreservedNode::Element(element) if is_text_element(element, paragraph))
                .then_some(index)
        }).collect::<Vec<_>>();
        if text_indexes.windows(2).any(|pair| {
            paragraph.children[pair[0] + 1..pair[1]].iter().any(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                _ => true,
            })
        }) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let runs = semantic_text_runs(semantic);
        let mut rebuilt = Vec::with_capacity(paragraph.children.len().saturating_add(runs.len()));
        let mut inserted = false;
        let mut template_cursor = 0usize;
        for child in &paragraph.children {
            match child {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => {
                    if !inserted {
                        for run in &runs {
                            let template = templates
                                .iter()
                                .find(|candidate| text_element_matches_run(candidate, run))
                                .or_else(|| templates.get(template_cursor))
                                .ok_or(PreservationError::UnsupportedTextMapping)?;
                            template_cursor = template_cursor.saturating_add(1);
                            rebuilt
                                .push(PreservedNode::Element(text_element_for_run(template, run)));
                        }
                        inserted = true;
                    }
                }
                PreservedNode::Text(text) | PreservedNode::CData(text)
                    if text.trim().is_empty() =>
                {
                    rebuilt.push(child.clone())
                }
                PreservedNode::Element(element) if !is_text_element(element, paragraph) => {
                    rebuilt.push(child.clone())
                }
                PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {
                    rebuilt.push(child.clone())
                }
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if !inserted && !runs.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        paragraph.children = rebuilt.into();
        Ok(())
    }
}

fn validate_managed_text_elements_replacement(
    paragraph: &PreservedElement,
    expected_runs: &[String],
) -> Result<Vec<usize>, PreservationError> {
    let mut text_elements = Vec::new();
    for (index, node) in paragraph.children.iter().enumerate() {
        match node {
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                text_elements.push(index);
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            PreservedNode::Element(_) | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => {}
            _ => return Err(PreservationError::UnsupportedTextMapping),
        }
    }

    if expected_runs.is_empty() {
        for index in &text_elements {
            let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            if !text_element.attributes.is_empty()
                || text_element
                    .children
                    .iter()
                    .any(|child| !matches!(child, PreservedNode::Text(text) if text.is_empty()))
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
        }
        return Ok(text_elements);
    }

    if text_elements.is_empty() || text_elements.len() != expected_runs.len() {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    for (index, expected) in text_elements.iter().zip(expected_runs) {
        let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        if !text_element.attributes.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut actual = String::new();
        for child in &text_element.children {
            match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => actual.push_str(text),
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if &actual != expected {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::TextElementsReplacement,
            ));
        }
    }
    Ok(text_elements)
}

pub(super) fn source_name_from_qname(name: &str) -> SourceName {
    let local_name = name.rsplit(':').next().unwrap_or(name).to_owned();
    SourceName {
        namespace_uri: None,
        local_name,
        qualified_name: Some(name.to_owned()),
    }
}

pub(super) fn fdx_role_name(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "Scene Heading",
        Role::Action => "Action",
        Role::Character => "Character",
        Role::Dialogue => "Dialogue",
        Role::Parenthetical => "Parenthetical",
        Role::Transition => "Transition",
        Role::Note => "Note",
        Role::NewAct => "New Act",
        Role::EndOfAct => "End of Act",
        Role::CastList => "Cast List",
        Role::General => "General",
        Role::Shot => "Shot",
        Role::Sequence => "Sequence",
        Role::Outline1 => "Outline 1",
        Role::Outline2 => "Outline 2",
        Role::Outline3 => "Outline 3",
        Role::Summary => "Summary",
    }
}

pub(super) fn semantic_text_runs(element: &ScreenplayElement) -> Vec<TextRun> {
    if !element.runs.is_empty() {
        element.runs.clone()
    } else if element.text.is_empty() {
        Vec::new()
    } else {
        vec![TextRun {
            text: element.text.clone(),
            attributes: Vec::new(),
        }]
    }
}

pub(super) fn is_text_element(element: &PreservedElement, paragraph: &PreservedElement) -> bool {
    element.name.local_name == "Text" && element.name.namespace_uri == paragraph.name.namespace_uri
}

pub(super) fn is_paragraph_id_attribute(attribute: &PreservedAttribute) -> bool {
    attribute.name.namespace_uri.is_none()
        && attribute.name.local_name.eq_ignore_ascii_case("id")
}

pub(super) fn is_managed_paragraph_attribute(attribute: &PreservedAttribute) -> bool {
    attribute.name.namespace_uri.is_none()
        && matches!(attribute.name.local_name.as_str(),
            "Type" | "Alignment" | "FirstIndent" | "Leading" | "LeftIndent"
            | "RightIndent" | "SpaceBefore" | "Spacing" | "StartsNewPage")
}








pub(super) fn is_plain_managed_paragraph(paragraph: &PreservedElement) -> bool {
    paragraph.name.local_name == "Paragraph"
        && paragraph.children.iter().all(|node| match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) => text.trim().is_empty(),
            // Recognized direct Text elements are managed run slots. All other
            // direct children are owner-opaque and retained by exact comparison.
            PreservedNode::Element(element) if is_text_element(element, paragraph) => true,
            _ => true,
        })
}

pub(super) fn same_direct_owner_opaque_children(
    before: &PreservedElement,
    after: &PreservedElement,
) -> bool {
    fn opaque_text_topology(paragraph: &PreservedElement) -> Vec<PreservedElement> {
        paragraph.children.iter().filter_map(|node| match node {
            PreservedNode::Element(element) if is_text_element(element, paragraph)
                && element.children.iter().any(|child| !matches!(child,
                    PreservedNode::Text(_) | PreservedNode::CData(_)
                )) => {
                let mut normalized = element.clone();
                for child in &mut normalized.children {
                    match child {
                        PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                        _ => {}
                    }
                }
                Some(normalized)
            }
            _ => None,
        }).collect()
    }
    let direct_opaque = |paragraph: &PreservedElement| {
        let mut text_slots = 0usize;
        let mut opaque = Vec::new();
        for node in &paragraph.children {
            if matches!(node, PreservedNode::Element(element) if is_text_element(element, paragraph)) {
                text_slots = text_slots.saturating_add(1);
            } else {
                opaque.push((text_slots, node.clone()));
            }
        }
        (text_slots, opaque)
    };
    let (before_slots, before_opaque) = direct_opaque(before);
    let (after_slots, after_opaque) = direct_opaque(after);
    (if before_slots == after_slots {
        before_opaque == after_opaque
    } else {
        before_opaque.iter().map(|(_, node)| node).eq(after_opaque.iter().map(|(_, node)| node))
    }) && opaque_text_topology(before) == opaque_text_topology(after)
}

fn paragraph_text_nodes_with_supported_attachments(
    paragraph: &PreservedElement,
) -> Result<Vec<PreservedNode>, PreservationError> {
    let mut alts_count = 0usize;
    let mut user_data_count = 0usize;
    let mut text_nodes = Vec::new();
    for node in &paragraph.children {
        match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri
                    && element.children.iter().all(|child| matches!(
                        child,
                        PreservedNode::Text(_) | PreservedNode::CData(_)
                    )) => text_nodes.push(node.clone()),
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                return Err(PreservationError::UnsupportedRunTopology);
            }
            PreservedNode::Element(element)
                if element.name.local_name == "Alts"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                alts_count = alts_count.saturating_add(1);
                if alts_count > 1 {
                    return Err(PreservationError::StructuralAttachmentConflict);
                }
            }
            PreservedNode::Element(element)
                if is_user_paragraph_data_name(&element.name, &paragraph.name) =>
            {
                user_data_count = user_data_count.saturating_add(1);
                if user_data_count > 1 {
                    return Err(PreservationError::StructuralAttachmentConflict);
                }
            }
            // Unknown direct siblings belong to their paragraph owner. They are
            // intentionally omitted from the returned managed Text slot list.
            _ => {}
        }
    }
    Ok(text_nodes)
}

fn text_of_nodes(nodes: &[PreservedNode]) -> String {
    nodes.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Text" => Some(
            element.children.iter().filter_map(|child| match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>(),
        ),
        _ => None,
    }).fold(String::new(), |mut value, text| {
        value.push_str(&text);
        value
    })
}

fn slice_source_text_nodes(
    nodes: &[PreservedNode],
    start: usize,
) -> Result<Vec<PreservedNode>, PreservationError> {
    let total = text_of_nodes(nodes);
    if start > total.len() || !total.is_char_boundary(start) {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut offset = 0usize;
    let mut result = Vec::new();
    for node in nodes {
        let PreservedNode::Element(original) = node else { continue };
        let segment = original.children.iter().filter_map(|child| match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        }).collect::<String>();
        let segment_end = offset.saturating_add(segment.len());
        if segment_end > start {
            let local_start = start.saturating_sub(offset);
            if !segment.is_char_boundary(local_start) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let mut clone = original.clone();
            let mut child_offset = 0usize;
            let mut children = Vec::new();
            for child in &original.children {
                match child {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        let child_end = child_offset.saturating_add(text.len());
                        let keep_from = local_start.max(child_offset);
                        if child_end > keep_from {
                            let local_from = keep_from - child_offset;
                            if !text.is_char_boundary(local_from) {
                                return Err(PreservationError::UnsupportedTextMapping);
                            }
                            let sliced = &text[local_from..];
                            children.push(match child {
                                PreservedNode::Text(_) => PreservedNode::Text(sliced.to_owned().into()),
                                PreservedNode::CData(_) => PreservedNode::CData(sliced.to_owned().into()),
                                _ => return Err(PreservationError::UnsupportedTextMapping),
                            });
                        }
                        child_offset = child_end;
                    }
                    _ => return Err(PreservationError::UnsupportedTextMapping),
                }
            }
            let retained_text = children.iter().filter_map(|child| match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>();
            clone.children = children.into();
            if !retained_text.is_empty() {
                result.push(PreservedNode::Element(clone));
            }
        }
        offset = segment_end;
    }
    Ok(result)
}

pub(super) fn text_element_matches_run(element: &PreservedElement, run: &TextRun) -> bool {
    element.attributes.len() == run.attributes.len()
        && element
            .attributes
            .iter()
            .zip(&run.attributes)
            .all(|(source, model)| {
                source
                    .name
                    .qualified_name
                    .as_deref()
                    .unwrap_or(source.name.local_name.as_str())
                    == model.name
                    && source.value == model.value
            })
}

fn slice_text_runs(
    runs: &[TextRun],
    range: std::ops::Range<usize>,
) -> Result<Vec<TextRun>, PreservationError> {
    let total = runs.iter().map(|run| run.text.len()).sum::<usize>();
    if range.start > range.end || range.end > total {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut result = Vec::new();
    let mut offset = 0usize;
    for run in runs {
        let end = offset.saturating_add(run.text.len());
        let start = range.start.max(offset);
        let finish = range.end.min(end);
        if start < finish {
            let from = start - offset;
            let to = finish - offset;
            if !run.text.is_char_boundary(from) || !run.text.is_char_boundary(to) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            result.push(TextRun {
                text: run.text[from..to].to_owned(),
                attributes: run.attributes.clone(),
            });
        }
        offset = end;
    }
    if range.is_empty() && total == 0 {
        if let Some(run) = runs.first() {
            result.push(run.clone());
        }
    }
    Ok(result)
}

pub(super) fn text_element_for_run(template: &PreservedElement, run: &TextRun) -> PreservedElement {
    let mut element = template.clone();
    element.attributes = run
        .attributes
        .iter()
        .map(|attribute| PreservedAttribute {
            name: source_name_from_qname(&attribute.name),
            value: attribute.value.clone(),
        })
        .collect();
    let existing_text = element.children.iter().filter_map(|child| match child {
        PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
        _ => None,
    }).collect::<String>();
    if existing_text != run.text {
        element.children = vec![PreservedNode::Text(run.text.clone().into())].into();
    }
    element
}

fn sync_text_element_attributes(
    template: &PreservedElement,
    run: &TextRun,
) -> Result<PreservedElement, PreservationError> {
    let mut element = template.clone();
    for source in &mut element.attributes {
        let source_name = source.name.qualified_name.as_deref()
            .unwrap_or(source.name.local_name.as_str());
        let Some(attribute) = run.attributes.iter().find(|attribute| attribute.name == source_name) else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        source.value.clone_from(&attribute.value);
    }
    for attribute in &run.attributes {
        let found = element.attributes.iter().any(|source| {
            source.name.qualified_name.as_deref()
                .unwrap_or(source.name.local_name.as_str()) == attribute.name
        });
        if !found {
            if attribute.name != "Style" {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            element.attributes.push(PreservedAttribute {
                name: plain_source_name("Style"),
                value: attribute.value.clone(),
            });
        }
    }
    element.children = vec![PreservedNode::Text(run.text.clone().into())].into();
    Ok(element)
}

fn replace_text_element_children(
    paragraph: &mut PreservedElement,
    old_text: &str,
    start: usize,
    end: usize,
    inserted: &str,
    expected_runs: Option<&[String]>,
) -> Result<(), PreservationError> {
    if start > end
        || end > old_text.len()
        || !old_text.is_char_boundary(start)
        || !old_text.is_char_boundary(end)
    {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut indexes = paragraph
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element) if is_text_element(element, paragraph) => Some(index),
            _ => None,
        })
        .collect::<Vec<_>>();
    if indexes.is_empty() {
        if !old_text.is_empty() || start != 0 || end != 0 {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        if inserted.is_empty() {
            return Ok(());
        }
        let (name, namespace_bindings) = paragraph_text_name(paragraph);
        let text_node = PreservedNode::Element(PreservedElement {
                name,
                attributes: Vec::new(),
                namespace_bindings,
                source_type: None,
                ownership: SourceOwnership::Opaque,
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: None,
                children: SharedVec::default(),
            });
        let insertion_index = paragraph.children.iter().position(|node| !matches!(node,
            PreservedNode::Element(element) if is_text_element(element, paragraph)
        )).unwrap_or(paragraph.children.len());
        paragraph.children.insert(insertion_index, text_node);
        indexes.push(insertion_index);
    }
    let mut mixed_segments = Vec::with_capacity(indexes.len());
    let mut segment_offsets = Vec::with_capacity(indexes.len());
    let mut running_offset = 0usize;
    let source_segments = indexes.iter().map(|index| {
        let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        let mut text = String::new();
        let mut mixed = false;
        for child in &text_element.children {
            match child {
                PreservedNode::Text(value) | PreservedNode::CData(value) => text.push_str(value),
                _ => mixed = true,
            }
        }
        segment_offsets.push(running_offset);
        running_offset = running_offset.saturating_add(text.len());
        mixed_segments.push(mixed);
        Ok(text)
    }).collect::<Result<Vec<_>, _>>()?;
    if source_segments.concat() != old_text {
        return Err(PreservationError::ManagedTextMismatch);
    }
    if expected_runs.is_some_and(|runs| runs != source_segments.as_slice()) {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut boundary = 0usize;
    for pair in indexes.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        boundary = boundary.saturating_add(
            source_segments[indexes
                .iter()
                .position(|index| *index == previous)
                .ok_or(PreservationError::UnsupportedTextMapping)?]
            .len(),
        );
        let has_opaque_gap = paragraph.children[previous + 1..next]
            .iter()
            .any(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                _ => true,
            });
        let clear_all_text = start == 0 && end == old_text.len() && inserted.is_empty();
        if has_opaque_gap && !clear_all_text
            && ((start < boundary && end > boundary) || (start == end && start == boundary))
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
    }
    let updated = replace_segments(&source_segments, start, end, inserted)?;
    let clear_all_text = start == 0 && end == old_text.len() && inserted.is_empty();
    let mut remove_indexes = BTreeSet::new();
    for (run_index, (index, text)) in indexes.into_iter().zip(updated).enumerate() {
        let PreservedNode::Element(text_element) = &mut paragraph.children[index] else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        if mixed_segments[run_index] && text != source_segments[run_index] {
            if clear_all_text {
                set_direct_text_content_preserving_opaque(text_element, "")?;
                continue;
            }
            let run_start = segment_offsets[run_index];
            let run_end = run_start.saturating_add(source_segments[run_index].len());
            if start < run_start || end > run_end {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            replace_mixed_text_run(text_element, start - run_start, end - run_start, inserted)?;
            continue;
        }
        let removable = text.is_empty()
            && text_element.attributes.is_empty()
            && text_element.namespace_bindings.is_empty()
            && text_element.children.iter().all(|child| matches!(child,
                PreservedNode::Text(_) | PreservedNode::CData(_)
            ));
        if removable {
            remove_indexes.insert(index);
        } else if text != source_segments[run_index] || clear_all_text {
            set_direct_text_content_preserving_opaque(text_element, &text)?;
        }
    }
    if !remove_indexes.is_empty() {
        paragraph.children = paragraph.children.drain(..).enumerate()
            .filter_map(|(index, child)| (!remove_indexes.contains(&index)).then_some(child))
            .collect();
    }
    Ok(())
}

fn replace_mixed_text_run(
    element: &mut PreservedElement,
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<(), PreservationError> {
    let mut segments = Vec::new();
    let mut opaque_boundaries = Vec::new();
    let mut offset = 0usize;
    let mut saw_text = false;
    let mut previous_was_text = false;
    for child in &element.children {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                segments.push(text.to_string());
                offset = offset.saturating_add(text.len());
                saw_text = true;
                previous_was_text = true;
            }
            _ => {
                if previous_was_text || !saw_text {
                    opaque_boundaries.push(offset);
                }
                previous_was_text = false;
            }
        }
    }
    let clear_all = start == 0 && end == offset && inserted.is_empty();
    if !clear_all && opaque_boundaries.iter().any(|boundary| {
        (start < *boundary && end > *boundary)
            || (start == end && start == *boundary)
    }) {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let updated = replace_segments(&segments, start, end, inserted)?;
    let mut update_index = 0usize;
    for child in &mut element.children {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                *text = updated.get(update_index)
                    .ok_or(PreservationError::UnsupportedTextMapping)?
                    .clone().into();
                update_index += 1;
            }
            _ => {}
        }
    }
    if update_index != updated.len() {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok(())
}

fn set_direct_text_content_preserving_opaque(
    element: &mut PreservedElement,
    replacement: &str,
) -> Result<(), PreservationError> {
    let mut first = true;
    for child in &mut element.children {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                if first {
                    *text = if replacement.is_empty() { String::new().into() } else { replacement.to_owned().into() };
                    first = false;
                } else {
                    text.clear();
                }
            }
            _ => {}
        }
    }
    if first && !replacement.is_empty() {
        if element.children.iter().any(|child| !matches!(child,
            PreservedNode::Text(_) | PreservedNode::CData(_)
        )) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        element.children.push(PreservedNode::Text(replacement.to_owned().into()));
    }
    Ok(())
}

fn paragraph_text_name(paragraph: &PreservedElement) -> (SourceName, Vec<NamespaceBinding>) {
    let prefix = paragraph
        .name
        .qualified_name
        .as_deref()
        .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
    let qualified_name =
        prefix.map_or_else(|| "Text".to_owned(), |prefix| format!("{prefix}:Text"));
    (
        SourceName {
            namespace_uri: paragraph.name.namespace_uri.clone(),
            local_name: "Text".to_owned(),
            qualified_name: Some(qualified_name),
        },
        paragraph.namespace_bindings.clone(),
    )
}

fn replace_text_element_content(
    element: &mut PreservedElement,
    replacement: &str,
) -> Result<(), PreservationError> {
    if element
        .children
        .iter()
        .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
    {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    if let Some(first) = element.children.first_mut() {
        match first {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                *text = replacement.to_owned().into();
            }
            _ => return Err(PreservationError::UnsupportedTextMapping),
        }
        for child in element.children.iter_mut().skip(1) {
            match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
    } else if !replacement.is_empty() {
        element
            .children
            .push(PreservedNode::Text(replacement.to_owned().into()));
    }
    Ok(())
}

fn direct_text_child_indexes(element: &PreservedElement) -> Vec<usize> {
    element
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            // Whitespace-only direct text around generated scene containers is
            // element-only XML formatting, not part of the screenplay text
            // projection. Keep those nodes in place, but do not compare or edit
            // them as semantic text runs.
            matches!(node, PreservedNode::Text(text) | PreservedNode::CData(text) if !text.trim().is_empty())
                .then_some(index)
        })
        .collect()
}

fn replace_segments(
    segments: &[String],
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<Vec<String>, PreservationError> {
    let total = segments.iter().map(String::len).sum::<usize>();
    if start > end || end > total {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut base = 0usize;
    let mut insertion_segment = None;
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        if start < limit || (start == total && limit == total) {
            insertion_segment = Some(index);
            break;
        }
        base = limit;
    }
    let insertion_segment = insertion_segment
        .or_else(|| segments.len().checked_sub(1))
        .ok_or(PreservationError::UnsupportedTextMapping)?;

    let mut base = 0usize;
    let mut updated = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        let before_len = start.saturating_sub(base).min(segment.len());
        let after_start = end.saturating_sub(base).min(segment.len());
        let mut value = segment[..before_len].to_owned();
        if index == insertion_segment {
            value.push_str(inserted);
        }
        if end < limit || end <= base {
            value.push_str(&segment[after_start..]);
        }
        updated.push(value);
        base = limit;
    }
    Ok(updated)
}
