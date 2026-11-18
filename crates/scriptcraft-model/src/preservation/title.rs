use super::*;
use crate::{SourceFormat, SourceProvenance, SourceSection, StableId, TextRun, TitlePageParagraph};
use std::collections::BTreeSet;
use super::navigation::{element_at_path, element_at_path_mut};
use super::text::{is_managed_paragraph_attribute, is_paragraph_id_attribute, is_plain_managed_paragraph, is_text_element};
use super::reconciliation::{paragraph_foreign_id, paragraph_id_attribute_count, set_paragraph_foreign_id};
pub(super) fn title_effective_runs(semantic: &TitlePageParagraph) -> Vec<TextRun> {
    if semantic.runs.is_empty() {
        if semantic.text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun { text: semantic.text.clone(), attributes: Vec::new() }]
        }
    } else {
        semantic.runs.clone()
    }
}

pub(super) fn title_text_templates(
    paragraph: &PreservedElement,
) -> Result<Vec<PreservedElement>, PreservationError> {
    let mut templates = Vec::new();
    for child in &paragraph.children {
        match child {
            PreservedNode::Element(element) if is_text_element(element, paragraph) => {
                templates.push(element.clone());
            }
            PreservedNode::Element(element) if !is_text_element(element, paragraph) => {}
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {}
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    Ok(templates)
}

pub(super) fn validate_title_text_shape(
    source: &PreservedElement,
    semantic: &TitlePageParagraph,
) -> Result<(), PreservationError> {
    if !is_plain_managed_paragraph(source) {
        return Err(PreservationError::StructuralAttachmentConflict);
    }
    let templates = title_text_templates(source)?;
    let source_text = templates.iter().flat_map(|template| template.children.iter()).filter_map(|child| {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        }
    }).collect::<String>();
    if source_text != semantic.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let runs = title_effective_runs(semantic);
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != semantic.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    if semantic.text.is_empty() {
        if !runs.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        if !templates.is_empty() {
            // Empty source Text elements can carry authored run attributes or
            // namespace state with no semantic run slot to transfer safely.
            return Err(PreservationError::UnsupportedRunTopology);
        }
        return Ok(());
    }
    if templates.len() != runs.len() {
        return Err(PreservationError::UnsupportedRunTopology);
    }
    for (template, run) in templates.iter().zip(&runs) {
        let text = template.children.iter().filter_map(|child| match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        }).collect::<String>();
        if text != run.text
            || template.attributes.len() != run.attributes.len()
            || !template.attributes.iter().zip(&run.attributes)
                .all(|(source, semantic)| source_attribute_matches_run(source, semantic))
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
    }
    Ok(())
}

pub(super) fn partition_title_children(
    source: &PreservedElement,
    original: &TitlePageParagraph,
    left: &TitlePageParagraph,
    right: &TitlePageParagraph,
) -> Result<(Vec<PreservedNode>, Vec<PreservedNode>), PreservationError> {
    if format!("{}{}", left.text, right.text) != original.text
        || left.text.len() > original.text.len()
        || !original.text.is_char_boundary(left.text.len())
        || left.runs.iter().map(|run| run.text.as_str()).collect::<String>() != left.text
        || right.runs.iter().map(|run| run.text.as_str()).collect::<String>() != right.text
    {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    validate_title_text_shape(source, original)?;
    let source_runs = title_effective_runs(original);
    let mut left_children = Vec::new();
    let mut right_children = Vec::new();
    let mut cursor = 0usize;
    let mut run_index = 0usize;
    for child in &source.children {
        match child {
            PreservedNode::Element(element) if is_text_element(element, source) => {
                let content = element.children.iter().filter_map(|node| match node {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                    _ => None,
                }).collect::<String>();
                if content.is_empty() {
                    left_children.push(child.clone());
                    continue;
                }
                let run = source_runs.get(run_index).ok_or(PreservationError::UnsupportedTextMapping)?;
                if run.text != content {
                    return Err(PreservationError::UnsupportedTextMapping);
                }
                run_index += 1;
                let end = cursor.saturating_add(content.len());
                if end <= left.text.len() {
                    left_children.push(child.clone());
                } else if cursor >= left.text.len() {
                    right_children.push(child.clone());
                } else {
                    let split = left.text.len().saturating_sub(cursor);
                    let (left_text, right_text) = split_title_text_element(element, split)?;
                    if let Some(element) = left_text {
                        left_children.push(PreservedNode::Element(element));
                    }
                    if let Some(element) = right_text {
                        right_children.push(PreservedNode::Element(element));
                    }
                }
                cursor = end;
            }
            PreservedNode::Element(element) if !is_text_element(element, &source) => {
                // Split ownership is left-biased: opaque source data remains
                // attached only to the original title paragraph.
                left_children.push(child.clone());
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {
                // Formatting whitespace remains at its original source position
                // with the retained left occurrence; no opaque payload is moved.
                left_children.push(child.clone());
            }
            PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {
                left_children.push(child.clone());
            }
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    if cursor != original.text.len() || run_index != source_runs.len() {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let emitted_left = left_children.iter().flat_map(|node| match node {
        PreservedNode::Element(element) if is_text_element(element, source) => element.children.iter().collect::<Vec<_>>(),
        _ => Vec::new(),
    }).filter_map(|node| match node {
        PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
        _ => None,
    }).collect::<String>();
    let emitted_right = right_children.iter().flat_map(|node| match node {
        PreservedNode::Element(element) if is_text_element(element, source) => element.children.iter().collect::<Vec<_>>(),
        _ => Vec::new(),
    }).filter_map(|node| match node {
        PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
        _ => None,
    }).collect::<String>();
    if emitted_left != left.text || emitted_right != right.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok((left_children, right_children))
}

fn split_title_text_element(
    source: &PreservedElement,
    split: usize,
) -> Result<(Option<PreservedElement>, Option<PreservedElement>), PreservationError> {
    let total = source.children.iter().map(|node| match node {
        PreservedNode::Text(text) | PreservedNode::CData(text) => text.len(),
        _ => 0,
    }).sum::<usize>();
    if split == 0 {
        return Ok((None, Some(source.clone())));
    }
    if split == total {
        return Ok((Some(source.clone()), None));
    }
    if split > total || !source.children.iter().all(|node| matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))) {
        return Err(PreservationError::UnsupportedRunTopology);
    }
    let mut left = source.clone();
    let mut right = source.clone();
    let mut left_nodes = Vec::new();
    let mut right_nodes = Vec::new();
    let mut cursor = 0usize;
    for node in &source.children {
        let (text, cdata) = match node {
            PreservedNode::Text(text) => (text.as_str(), false),
            PreservedNode::CData(text) => (text.as_str(), true),
            _ => unreachable!(),
        };
        if text.is_empty() {
            return Err(PreservationError::UnsupportedRunTopology);
        }
        let end = cursor.saturating_add(text.len());
        if end <= split {
            left_nodes.push(node.clone());
        } else if cursor >= split {
            right_nodes.push(node.clone());
        } else {
            let local = split.saturating_sub(cursor);
            let (left_text, right_text) = text.split_at(local);
            if cdata {
                left_nodes.push(PreservedNode::CData(left_text.to_owned().into()));
                right_nodes.push(PreservedNode::CData(right_text.to_owned().into()));
            } else {
                left_nodes.push(PreservedNode::Text(left_text.to_owned().into()));
                right_nodes.push(PreservedNode::Text(right_text.to_owned().into()));
            }
        }
        cursor = end;
    }
    left.children = left_nodes.into();
    right.children = right_nodes.into();
    Ok((Some(left), Some(right)))
}

pub(super) fn title_text_children_from_templates(
    recipient_source: &PreservedElement,
    templates: &[PreservedElement],
    semantic: &TitlePageParagraph,
) -> Result<Vec<PreservedNode>, PreservationError> {
    let runs = title_effective_runs(semantic);
    let mut rebuilt = Vec::with_capacity(recipient_source.children.len().saturating_add(runs.len()));
    let mut inserted = false;
    for child in &recipient_source.children {
        match child {
            PreservedNode::Element(element) if is_text_element(element, recipient_source) => {
                if !inserted {
                    for run in &runs {
                        let template = templates.iter().find(|template| {
                            template.attributes.len() == run.attributes.len()
                                && template.attributes.iter().zip(&run.attributes)
                                    .all(|(source, semantic)| source_attribute_matches_run(source, semantic))
                        }).ok_or(PreservationError::UnsupportedTextMapping)?;
                        let mut text_element = template.clone();
                        let source_text = text_element.children.iter().filter_map(|node| match node {
                            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                            _ => None,
                        }).collect::<String>();
                        if source_text != run.text {
                            if text_element.children.iter().any(|node| {
                                !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
                            }) {
                                return Err(PreservationError::UnsupportedRunTopology);
                            }
                            if let Some(first) = text_element.children.first_mut() {
                                match first {
                                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                                        **text = run.text.clone();
                                    }
                                    _ => unreachable!(),
                                }
                                for child in text_element.children.iter_mut().skip(1) {
                                    match child {
                                        PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                                        _ => unreachable!(),
                                    }
                                }
                            } else {
                                text_element.children.push(PreservedNode::Text(run.text.clone().into()));
                            }
                        }
                        rebuilt.push(PreservedNode::Element(text_element));
                    }
                    inserted = true;
                }
            }
            PreservedNode::Element(element) if !is_text_element(element, recipient_source) => {
                rebuilt.push(child.clone());
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {
                rebuilt.push(child.clone());
            }
            PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {
                rebuilt.push(child.clone());
            }
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    if !inserted {
        let mut generated = Vec::with_capacity(runs.len());
        for run in &runs {
            let template = templates.iter().find(|template| {
                template.attributes.len() == run.attributes.len()
                    && template.attributes.iter().zip(&run.attributes)
                        .all(|(source, semantic)| source_attribute_matches_run(source, semantic))
            }).ok_or(PreservationError::UnsupportedTextMapping)?;
            let mut text_element = template.clone();
            let source_text = text_element.children.iter().filter_map(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>();
            if source_text != run.text {
                if text_element.children.iter().any(|node| {
                    !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
                }) {
                    return Err(PreservationError::UnsupportedRunTopology);
                }
                if let Some(first) = text_element.children.first_mut() {
                    match first {
                        PreservedNode::Text(text) | PreservedNode::CData(text) => {
                            **text = run.text.clone();
                        }
                        _ => unreachable!(),
                    }
                    for child in text_element.children.iter_mut().skip(1) {
                        match child {
                            PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                            _ => unreachable!(),
                        }
                    }
                } else {
                    text_element.children.push(PreservedNode::Text(run.text.clone().into()));
                }
            }
            generated.push(PreservedNode::Element(text_element));
        }
        let insert_at = rebuilt.iter().position(|node| matches!(node,
            PreservedNode::Element(element) if !is_text_element(element, recipient_source)
        )).unwrap_or(rebuilt.len());
        rebuilt.splice(insert_at..insert_at, generated);
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != semantic.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok(rebuilt)
}

pub(super) fn source_attribute_matches_run(attribute: &PreservedAttribute, run: &crate::RunAttribute) -> bool {
    attribute.name.qualified_name.as_deref()
        .unwrap_or(attribute.name.local_name.as_str()) == run.name
        && attribute.value == run.value
}

impl PreservationState {
/// Reconciles one supported title-page structural transaction against the
    /// exact TitlePage -> Content source shape. Deleted owners need no text
    /// recipient; surviving recipients retain strict text-mapping safeguards.
    /// All callers operate on detached Projects, so publication remains atomic.
    pub fn reconcile_title_structure(
        &mut self,
        before: &[TitlePageParagraph],
        after: &mut [TitlePageParagraph],
    ) -> Result<(), PreservationError> {
        self.ensure_managed_paragraph_index();
        self.ensure_source_identity_use_index();

        let before_ids = before.iter().map(|paragraph| paragraph.id.clone()).collect::<Vec<_>>();
        let after_ids = after.iter().map(|paragraph| paragraph.id.clone()).collect::<Vec<_>>();
        if before_ids == after_ids {
            return Ok(());
        }
        let before_set = before_ids.iter().cloned().collect::<BTreeSet<_>>();
        let after_set = after_ids.iter().cloned().collect::<BTreeSet<_>>();
        if before_set.len() != before_ids.len() || after_set.len() != after_ids.len() {
            return Err(PreservationError::ManagedAnchorAmbiguous);
        }
        let added = after_ids.iter().filter(|id| !before_set.contains(*id)).cloned().collect::<Vec<_>>();
        let removed = before_ids.iter().filter(|id| !after_set.contains(*id)).cloned().collect::<Vec<_>>();

        if added.len() == 1 && removed.is_empty() && after.len() == before.len().saturating_add(1) {
            let new_index = after_ids.iter().position(|id| id == &added[0])
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if new_index == 0 || after[new_index - 1].id != before_ids[new_index - 1]
                || after_ids.iter().enumerate().any(|(index, id)| {
                    index != new_index && id != &before_ids[if index < new_index { index } else { index - 1 }]
                })
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let source_index = new_index - 1;
            let source = before.get(source_index).ok_or(PreservationError::ManagedAnchorMissing)?;
            let (left, right) = after.split_at_mut(new_index);
            if left[source_index].id != source.id
                || left[source_index].text.len().saturating_add(right[0].text.len()) != source.text.len()
                || format!("{}{}", left[source_index].text, right[0].text) != source.text {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            self.split_title_source_paragraph(source, &left[source_index], &mut right[0])?;
            return Ok(());
        }

        if added.is_empty() && !removed.is_empty() && after.len().saturating_add(removed.len()) == before.len() {
            let first_removed = before_ids.iter().position(|id| !after_set.contains(id))
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let last_removed = before_ids.iter().rposition(|id| !after_set.contains(id))
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if before_ids[first_removed..=last_removed].iter().any(|id| after_set.contains(id))
                || before_ids.iter().filter(|id| after_set.contains(*id)).collect::<Vec<_>>()
                    != after_ids.iter().collect::<Vec<_>>()
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            if first_removed == 0 {
                for id in &removed {
                    let semantic = before.iter().find(|paragraph| &paragraph.id == id)
                        .ok_or(PreservationError::ManagedAnchorMissing)?;
                    self.title_source_location(semantic)?;
                    self.remove_managed_paragraph(
                        &semantic.id,
                        semantic.source_provenance.as_ref(),
                    )?;
                }
                return Ok(());
            }
            let recipient = &before[first_removed - 1];
            if !after_set.contains(&recipient.id) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let updated_recipient = after.iter().find(|paragraph| paragraph.id == recipient.id)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if updated_recipient.text == recipient.text
                && updated_recipient.runs == recipient.runs
            {
                for id in &removed {
                    let semantic = before.iter().find(|paragraph| &paragraph.id == id)
                        .ok_or(PreservationError::ManagedAnchorMissing)?;
                    self.title_source_location(semantic)?;
                    self.remove_managed_paragraph(
                        &semantic.id,
                        semantic.source_provenance.as_ref(),
                    )?;
                }
                return Ok(());
            }
            let mut removed_semantics = Vec::with_capacity(removed.len());
            let mut source_templates = Vec::new();
            let recipient_source = self.validate_title_source(recipient)?;
            source_templates.extend(title_text_templates(&recipient_source)?);
            for id in &removed {
                let semantic = before.iter().find(|paragraph| &paragraph.id == id)
                    .ok_or(PreservationError::ManagedAnchorMissing)?;
                let location = self.title_source_location(semantic)?;
                let source = self.managed_element_at(&location)
                    .filter(|element| {
                        matches!(&element.ownership, SourceOwnership::Managed(id) if id == &semantic.id)
                    })
                    .ok_or(PreservationError::ManagedAnchorMissing)?
                    .clone();
                source_templates.extend(title_text_templates(&source)?);
                removed_semantics.push((
                    semantic.id.clone(),
                    semantic.source_provenance.clone(),
                ));
            }
            let replacement_children = title_text_children_from_templates(
                &recipient_source,
                &source_templates,
                updated_recipient,
            )?;
            {
                let target = self.managed_element_mut(&recipient.id)?;
                target.children = replacement_children.into();
            }
            self.rebuild_managed_paragraph_index();
            for (id, provenance) in removed_semantics {
                self.remove_managed_paragraph(&id, provenance.as_ref())?;
            }
            return Ok(());
        }

        Err(PreservationError::UnsupportedTextMapping)
    }

fn split_title_source_paragraph(
        &mut self,
        original: &TitlePageParagraph,
        left_semantic: &TitlePageParagraph,
        right_semantic: &mut TitlePageParagraph,
    ) -> Result<(), PreservationError> {
        let location = self.title_source_location(original)?;
        let source = self.validate_title_source_at(original, &location)?;
        let (left_children, right_children) = partition_title_children(
            &source,
            original,
            left_semantic,
            right_semantic,
        )?;
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let mut right_source = source.clone();
        right_source.attributes.retain(|attribute| {
            is_managed_paragraph_attribute(attribute) || is_paragraph_id_attribute(attribute)
        });
        set_paragraph_foreign_id(&mut right_source, &foreign_id)?;
        right_source.ownership = SourceOwnership::Managed(right_semantic.id.clone());
        right_source.anchor = Some(anchor_id.clone());
        right_source.children = right_children.into();
        {
            let left_source = self.managed_element_mut(&original.id)?;
            left_source.children = left_children.into();
        }
        self.rebuild_managed_paragraph_index();
        let parent = title_content_parent_mut(&mut self.sections, &location)?;
        let insert_at = location.node_path.last().copied()
            .and_then(|index| index.checked_add(1))
            .ok_or(PreservationError::TooManyEvents)?;
        parent.children.insert(insert_at, PreservedNode::Element(right_source));
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(right_semantic.id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(ForeignIdDomain::Paragraph, foreign_id.clone())?);
        right_semantic.source_provenance = Some(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::TitlePage,
            ordinal: original.source_provenance.as_ref()
                .map(|provenance| provenance.ordinal.saturating_add(1))
                .ok_or(PreservationError::ManagedAnchorMissing)?,
            foreign_id: Some(foreign_id),
            foreign_role: original.source_provenance.as_ref()
                .and_then(|provenance| provenance.foreign_role.clone()),
        });
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

fn title_source_location(
        &self,
        semantic: &TitlePageParagraph,
    ) -> Result<ManagedParagraphLocation, PreservationError> {
        let location = self.indexed_location(&semantic.id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !is_title_content_location(&self.sections, &location) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        Ok(location)
    }

fn validate_title_source(
        &self,
        semantic: &TitlePageParagraph,
    ) -> Result<PreservedElement, PreservationError> {
        let location = self.title_source_location(semantic)?;
        self.validate_title_source_at(semantic, &location)
    }

fn validate_title_source_at(
        &self,
        semantic: &TitlePageParagraph,
        location: &ManagedParagraphLocation,
    ) -> Result<PreservedElement, PreservationError> {
        let source = self.managed_element_at(location)
            .filter(|element| matches!(&element.ownership, SourceOwnership::Managed(id) if id == &semantic.id))
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let provenance = semantic.source_provenance.as_ref()
            .filter(|provenance| provenance.format == SourceFormat::Fdx && provenance.section == SourceSection::TitlePage)
            .ok_or(PreservationError::ManagedTextMismatchAt(ManagedTextMismatchSite::ParagraphRemoval))?;
        if paragraph_id_attribute_count(&source) > 1
            || paragraph_foreign_id(&source) != provenance.foreign_id.as_deref()
            || source.source_type != provenance.foreign_role
        {
            return Err(PreservationError::ManagedTextMismatchAt(ManagedTextMismatchSite::ParagraphRemoval));
        }
        let matching_anchors = self.anchors.iter().filter(|anchor| {
            matches!(&anchor.target, AnchorTarget::ProjectElement(id) if id == &semantic.id)
                && anchor.relation == AnchorRelation::Represents
        }).collect::<Vec<_>>();
        if matching_anchors.len() != 1 || source.anchor.as_ref() != Some(&matching_anchors[0].id) {
            return Err(PreservationError::ManagedAnchorAmbiguous);
        }
        validate_title_text_shape(&source, semantic)?;
        Ok(source)
    }
}

pub(super) fn is_title_content_location(
    sections: &[PreservedSection],
    location: &ManagedParagraphLocation,
) -> bool {
    if !matches!(location.node_path.len(), 3 | 4) {
        return false;
    }
    let Some(section) = sections.get(location.section_index) else {
        return false;
    };
    let Some(title) = element_at_path(
        &section.children,
        &location.node_path[..location.node_path.len() - 2],
    ) else {
        return false;
    };
    let Some(content) = element_at_path(
        &section.children,
        &location.node_path[..location.node_path.len() - 1],
    ) else {
        return false;
    };
    let Some(paragraph) = element_at_path(&section.children, &location.node_path) else {
        return false;
    };
    let root_is_valid = location.node_path.len() == 3
        || element_at_path(
            &section.children,
            &location.node_path[..location.node_path.len() - 3],
        ).is_some_and(|root| root.name.local_name == "FinalDraft");
    root_is_valid
        && title.name.local_name == "TitlePage"
        && content.name.local_name == "Content"
        && paragraph.name.local_name == "Paragraph"
        && title.name.namespace_uri == content.name.namespace_uri
        && content.name.namespace_uri == paragraph.name.namespace_uri
}

fn title_content_parent_mut<'a>(
    sections: &'a mut [PreservedSection],
    location: &ManagedParagraphLocation,
) -> Result<&'a mut PreservedElement, PreservationError> {
    if !is_title_content_location(sections, location) {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let section = sections.get_mut(location.section_index)
        .ok_or(PreservationError::ManagedAnchorMissing)?;
    let parent_path = &location.node_path[..location.node_path.len() - 1];
    element_at_path_mut(&mut section.children, parent_path)
        .ok_or(PreservationError::ManagedAnchorMissing)
}
