use super::*;
use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
use crate::{BodyOwner, Role, ScreenplayElement, SourceFormat, SourceProvenance, SourceSection, StableId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use super::navigation::element_at_path_mut;
use super::text::is_text_element;
use super::text::{fdx_role_name, semantic_text_runs, source_name_from_qname};
use super::dual_dialogue::is_exact_dual_dialogue_member_location;

pub(super) struct ManagedParagraphTemplate {
    pub(super) paragraph_name: SourceName,
    pub(super) paragraph_bindings: Vec<NamespaceBinding>,
    pub(super) text_name: SourceName,
    pub(super) text_bindings: Vec<NamespaceBinding>,
    pub(super) id_spelling: String,
}

pub(super) fn paragraph_template(
    paragraph: &PreservedElement,
) -> Result<ManagedParagraphTemplate, PreservationError> {
    if paragraph.name.local_name != "Paragraph" {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let text = paragraph.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if is_text_element(element, paragraph) => Some(element),
        _ => None,
    });
    let (text_name, text_bindings) = if let Some(text) = text {
        (text.name.clone(), text.namespace_bindings.clone())
    } else {
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
            Vec::new(),
        )
    };
    let id_spelling = paragraph
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        })
        .and_then(|attribute| attribute.name.qualified_name.clone())
        .unwrap_or_else(|| "ID".to_owned());
    Ok(ManagedParagraphTemplate {
        paragraph_name: paragraph.name.clone(),
        paragraph_bindings: paragraph.namespace_bindings.clone(),
        text_name,
        text_bindings,
        id_spelling,
    })
}

impl PreservationState {
/// Reconciles a body-structure edit in the same candidate as its semantic
    /// model. Existing foreign paragraph IDs stay with surviving elements;
    /// newly inserted paragraphs receive fresh foreign and source-anchor IDs.
    /// Surviving owners keep strict text-mapping safeguards. Deleted owners'
    /// subtrees are discarded, and unrelated raw references may become orphans.
    pub fn reconcile_body_structure(
        &mut self,
        before: &[Arc<BodyOwner>],
        after: &mut [Arc<BodyOwner>],
        _generated_scene_heading_deletions: &BTreeSet<StableId>,
        groups: &[crate::dual_dialogue::DualDialogueGroup],
    ) -> Result<(), PreservationError> {
        self.ensure_managed_paragraph_index();
        self.ensure_source_identity_use_index();
        let before_ids = before
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let after_ids = after
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let structural_change = before_ids != after_ids
            || before.iter().map(|item| &item.id).collect::<Vec<_>>()
                != after.iter().map(|item| &item.id).collect::<Vec<_>>();
        if !structural_change {
            return Ok(());
        }

        let mut structurally_reconciled_text = BTreeSet::new();
        // Preserve retained owner attachments across joins and ranged deletion.
        // Only the survivor's left prefix and the final removed Paragraph's
        // right suffix may contribute canonical text; all removed Alts remain
        // attached to their deleted owners and are discarded with those nodes.
        for (index, previous) in before.iter().enumerate() {
            let end = (index + 1..before.len())
                .find(|next| after_ids.contains(&before[*next].id))
                .unwrap_or(before.len());
            if end == index + 1 {
                continue;
            }
            let removed = &before[index + 1..end];
            let Some(updated) = after.iter().find(|item| item.id == previous.id) else {
                continue;
            };
            let final_removed = &removed[removed.len() - 1];
            let has_inline_attachment = self.has_owner_opaque_data(&previous.id)
                || removed.iter().any(|item| self.has_owner_opaque_data(&item.id));
            if !has_inline_attachment
            {
                continue;
            }
            if (self.has_inline_alts(&previous.id) && previous.role != Role::Dialogue)
                || removed.iter().any(|item| {
                    self.has_inline_alts(&item.id) && item.role != Role::Dialogue
                })
            {
                return Err(PreservationError::StructuralAttachmentConflict);
            }
            let mut partitions = Vec::new();
            for (prefix_len, _) in previous.text.char_indices().chain(
                std::iter::once((previous.text.len(), '\0')),
            ) {
                if !updated.text.starts_with(&previous.text[..prefix_len]) {
                    continue;
                }
                let suffix_len = updated.text.len().saturating_sub(prefix_len);
                let Some(suffix_start) = final_removed.text.len().checked_sub(suffix_len) else {
                    continue;
                };
                if final_removed.text.is_char_boundary(suffix_start)
                    && updated.text[prefix_len..] == final_removed.text[suffix_start..]
                {
                    partitions.push((prefix_len, suffix_start));
                }
            }
            let [(prefix_len, suffix_start)] = partitions.as_slice() else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            let expected_runs = previous.runs.iter().map(|run| run.text.clone()).collect::<Vec<_>>();
            self.reconcile_joined_text_nodes(
                &previous.id,
                &previous.text,
                *prefix_len,
                &final_removed.id,
                &final_removed.text,
                *suffix_start,
                &updated.text,
                (!previous.runs.is_empty()).then_some(expected_runs.as_slice()),
            )?;
            structurally_reconciled_text.insert(previous.id.clone());
        }

        for element in before {
            if !after_ids.contains(&element.id) && self.has_managed_element(&element.id) {
                self.remove_managed_paragraph(
                    &element.id,
                    element.source_provenance.as_ref(),
                )?;
            }
        }

        let ordered_ids = after
            .iter()
            .map(|element| element.id.clone())
            .collect::<Vec<_>>();
        for (index, element) in after.iter_mut().enumerate() {
            if before_ids.contains(&element.id) || self.has_managed_element(&element.id) {
                continue;
            }
            let provenance = self.insert_managed_paragraph(element, index, &ordered_ids, groups)?;
            BodyOwner::update(element, |element| {
                element.source_provenance = Some(provenance);
            });
            self.rebuild_managed_paragraph_index();
        }

        // A newly inserted right Paragraph after an existing owner is a split
        // only when its two canonical texts exactly partition the old text.
        // Trim the original owner's mapped Text nodes; its opaque Alts remains
        // attached there and the fresh Paragraph has no copied attachment.
        for index in 1..after.len() {
            let right = &after[index];
            let left = &after[index - 1];
            let Some(original) = before.iter().find(|item| item.id == left.id) else {
                continue;
            };
            if before_ids.contains(&right.id)
                || original.text != format!("{}{}", left.text, right.text)
                || !self.has_owner_opaque_data(&original.id)
            {
                continue;
            }
            let expected = original.runs.iter().map(|run| run.text.clone()).collect::<Vec<_>>();
            self.trim_split_text(
                &original.id,
                &original.text,
                left.text.len(),
                (!original.runs.is_empty()).then_some(expected.as_slice()),
            )?;
            structurally_reconciled_text.insert(original.id.clone());
        }

        // Other structural text projections continue through the existing
        // strict path. In particular, arbitrary attachments and unsupported
        // run mappings are not rewritten to make a candidate pass.
        for element in after.iter() {
            let Some(previous) = before.iter().find(|item| item.id == element.id) else {
                continue;
            };
            if previous.text != element.text
                && self.has_managed_element(&element.id)
                && !structurally_reconciled_text.contains(&element.id)
            {
                self.replace_managed_projection(element)?;
            }
        }
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

fn insert_managed_paragraph(
        &mut self,
        semantic: &ScreenplayElement,
        index: usize,
        ordered: &[StableId],
        groups: &[crate::dual_dialogue::DualDialogueGroup],
    ) -> Result<SourceProvenance, PreservationError> {
        if let Some(group) = groups.iter().find(|group| {
            group.member_ids().any(|member| member == &semantic.id)
        }) {
            return self.insert_nested_managed_paragraph(semantic, index, group);
        }
        let ManagedParagraphTemplate {
            paragraph_name,
            paragraph_bindings,
            text_name,
            text_bindings,
            id_spelling,
        } = {
            let children = content_children_mut(&mut self.sections)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let template = find_any_managed_paragraph(children)
                .ok_or(PreservationError::UnsupportedTextMapping)?;
            paragraph_template(template)?
        };
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let text_runs = semantic_text_runs(semantic);
        let mut paragraph = PreservedElement {
            name: paragraph_name,
            attributes: vec![
                PreservedAttribute {
                    name: plain_source_name("Type"),
                    value: fdx_role_name(semantic.role).to_owned(),
                },
                PreservedAttribute {
                    name: plain_source_name(&id_spelling),
                    value: foreign_id.clone(),
                },
            ],
            namespace_bindings: paragraph_bindings,
            source_type: Some(fdx_role_name(semantic.role).to_owned()),
            ownership: SourceOwnership::Managed(semantic.id.clone()),
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: Some(anchor_id.clone()),
            children: SharedVec::default(),
        };
        for run in text_runs {
            paragraph
                .children
                .push(PreservedNode::Element(PreservedElement {
                    name: text_name.clone(),
                    attributes: run
                        .attributes
                        .iter()
                        .map(|attribute| PreservedAttribute {
                            name: source_name_from_qname(&attribute.name),
                            value: attribute.value.clone(),
                        })
                        .collect(),
                    namespace_bindings: text_bindings.clone(),
                    source_type: None,
                    ownership: SourceOwnership::Opaque,
                    dependency_confidence: DependencyConfidence::Unknown,
                    anchor: None,
                    children: vec![PreservedNode::Text(run.text.into())].into(),
                }));
        }

        self.invalidate_source_identity_use_index();
        let mut wrapped_member_slots = BTreeMap::new();
        for group in groups {
            for member in group.member_ids() {
                let Some(location) = self.indexed_location(member)?.cloned() else {
                    continue;
                };
                if is_exact_dual_dialogue_member_location(&self.sections, &location, &group.id) {
                    wrapped_member_slots.insert(member.clone(), location.node_path[2]);
                }
            }
        }
        let children = content_children_mut(&mut self.sections)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut insert_at = None;
        for following in ordered.iter().skip(index + 1) {
            if let Some(position) = managed_child_index(children, following) {
                insert_at = Some(position);
                break;
            }
            if let Some(position) = wrapped_member_slots.get(following) {
                insert_at = Some(*position);
                break;
            }
        }
        if insert_at.is_none() {
            for preceding in ordered[..index].iter().rev() {
                if let Some(position) = managed_child_index(children, preceding) {
                    insert_at = Some(position + 1);
                    break;
                }
                if let Some(position) = wrapped_member_slots.get(preceding) {
                    insert_at = Some(*position + 1);
                    break;
                }
            }
        }
        let position = insert_at.unwrap_or(children.len());
        children.insert(position, PreservedNode::Element(paragraph));
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(semantic.id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(
            ForeignIdDomain::Paragraph,
            foreign_id.clone(),
        )?);
        Ok(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::Content,
            ordinal: u32::try_from(index).map_err(|_| PreservationError::TooManyEvents)?,
            foreign_id: Some(foreign_id),
            foreign_role: Some(fdx_role_name(semantic.role).to_owned()),
        })
    }


pub(super) fn remove_managed_paragraph(
        &mut self,
        id: &StableId,
        provenance: Option<&SourceProvenance>,
    ) -> Result<(), PreservationError> {
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut removed_foreign_ids = BTreeSet::new();
        let mut removed_project_ids = BTreeSet::from([id.clone()]);
        let mut removed_anchor_ids = BTreeSet::new();
        {
            let target = self
                .managed_element_at(&location)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let foreign_id = paragraph_foreign_id(target).map(str::to_owned);
            if provenance.and_then(|item| item.foreign_id.as_deref()) != foreign_id.as_deref() {
                return Err(PreservationError::ManagedTextMismatchAt(
                    ManagedTextMismatchSite::ParagraphRemoval,
                ));
            }
            collect_owner_registrations(
                target,
                &mut removed_foreign_ids,
                &mut removed_project_ids,
                &mut removed_anchor_ids,
            );
        }

        // Semantic structure has authorized this deletion. Discard only the
        // removed owner's subtree and typed registrations; unrelated raw XML
        // references survive as orphan data and do not veto the edit.
        if !remove_managed_at_location(&mut self.sections, &location) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let stale_anchors = self
            .anchors
            .iter()
            .filter(|anchor| {
                removed_anchor_ids.contains(&anchor.id)
                    || match &anchor.target {
                        AnchorTarget::ProjectElement(target) => {
                            removed_project_ids.contains(target)
                        }
                        AnchorTarget::Foreign(identity) => {
                            identity.domain == ForeignIdDomain::Paragraph
                                && removed_foreign_ids.contains(&identity.value)
                        }
                        AnchorTarget::DualDialogueWrapper(target) => {
                            removed_project_ids.contains(target)
                        }
                    }
            })
            .map(|anchor| anchor.id.clone())
            .collect::<BTreeSet<_>>();
        if !stale_anchors.is_empty() {
            for section in &mut self.sections {
                clear_stale_anchor_bindings(
                    &mut section.children,
                    &stale_anchors,
                    &removed_foreign_ids,
                );
            }
        }
        self.identities.retain(|identity| {
            !(identity.domain == ForeignIdDomain::Paragraph
                && removed_foreign_ids.contains(&identity.value))
        });
        self.references.retain(|reference| {
            !paragraph_reference_touches(reference, &removed_foreign_ids)
        });
        self.board_graph.references.retain(|reference| {
            !paragraph_reference_touches(reference, &removed_foreign_ids)
        });
        for removed_id in removed_project_ids {
            self.source_layout_settings
                .direct_paragraph_overrides
                .remove(&removed_id);
        }
        self.anchors
            .retain(|anchor| !stale_anchors.contains(&anchor.id));
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

pub(super) fn fresh_foreign_paragraph_id(&mut self) -> Result<String, PreservationError> {
        self.ensure_source_identity_use_index();
        let indexed_source_ids = self.indexed_source_id_values();
        let reserved = self
            .identities
            .iter()
            .map(|identity| identity.value.clone())
            .chain(
                self.board_graph
                    .entities
                    .iter()
                    .map(|entity| entity.identity.value.clone()),
            )
            .chain(self.references.iter().flat_map(|reference| {
                [
                    reference.source.value.clone(),
                    reference.target.value.clone(),
                ]
            }))
            .chain(self.board_graph.references.iter().flat_map(|reference| {
                [
                    reference.source.value.clone(),
                    reference.target.value.clone(),
                ]
            }))
            .chain(indexed_source_ids);
        let mut allocator = ParagraphIdAllocator::new(reserved);
        allocator.allocate().map_err(|error| match error {
            ParagraphIdAllocationError::EntropyUnavailable => {
                PreservationError::ParagraphIdentityEntropyUnavailable
            }
            ParagraphIdAllocationError::CollisionLimit => {
                PreservationError::ParagraphIdentityCollisionLimit
            }
        })
    }

pub(super) fn fresh_anchor_id(&self) -> Result<SourceAnchorId, PreservationError> {
        for serial in 1..=u64::try_from(MAX_SOURCE_EVENTS).unwrap_or(u64::MAX) {
            let candidate = SourceAnchorId::new(format!("scriptcraft-anchor-{serial}"))?;
            if !self.anchors.iter().any(|anchor| anchor.id == candidate) {
                return Ok(candidate);
            }
        }
        Err(PreservationError::TooManyEvents)
    }
}

pub(super) fn plain_source_name(name: &str) -> SourceName {
    SourceName {
        namespace_uri: None,
        local_name: name.to_owned(),
        qualified_name: Some(name.to_owned()),
    }
}

fn content_children_mut(sections: &mut [PreservedSection]) -> Option<&mut Vec<PreservedNode>> {
    for section in sections {
        for node in &mut section.children {
            let PreservedNode::Element(root) = node else {
                continue;
            };
            if root.name.local_name != "FinalDraft" {
                continue;
            }
            let namespace = root.name.namespace_uri.clone();
            for child in &mut root.children {
                if let PreservedNode::Element(content) = child {
                    if content.name.local_name == "Content"
                        && content.name.namespace_uri == namespace
                    {
                        return Some(&mut content.children);
                    }
                }
            }
        }
    }
    None
}

fn find_any_managed_paragraph(nodes: &[PreservedNode]) -> Option<&PreservedElement> {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            if matches!(element.ownership, SourceOwnership::Managed(_)) {
                return Some(element);
            }
            if let Some(found) = find_any_managed_paragraph(&element.children) {
                return Some(found);
            }
        }
    }
    None
}

pub(super) fn managed_child_index(nodes: &[PreservedNode], id: &StableId) -> Option<usize> {
    nodes.iter().position(|node| {
        matches!(node, PreservedNode::Element(element)
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id))
    })
}


fn collect_owner_registrations(
    root: &PreservedElement,
    foreign_ids: &mut BTreeSet<String>,
    project_ids: &mut BTreeSet<StableId>,
    anchor_ids: &mut BTreeSet<SourceAnchorId>,
) {
    fn collect(
        element: &PreservedElement,
        parent: Option<&PreservedElement>,
        grandparent: Option<&PreservedElement>,
        in_user_data: bool,
        foreign_ids: &mut BTreeSet<String>,
        project_ids: &mut BTreeSet<StableId>,
        anchor_ids: &mut BTreeSet<SourceAnchorId>,
    ) {
        if let SourceOwnership::Managed(id) = &element.ownership {
            project_ids.insert(id.clone());
        }
        if let Some(anchor) = &element.anchor {
            anchor_ids.insert(anchor.clone());
        }
        let is_user_data = parent.is_some_and(|owner| {
            owner.name.local_name == "Paragraph"
                && is_user_paragraph_data_name(&element.name, &owner.name)
        });
        let is_alternate_copy = element.name.local_name == "Paragraph"
            && parent.is_some_and(|owner| {
                owner.name.local_name == "Alt"
                    && owner.name.namespace_uri == element.name.namespace_uri
            })
            && grandparent.is_some_and(|owner| {
                owner.name.local_name == "AltCollection"
                    && owner.name.namespace_uri == element.name.namespace_uri
            });
        if !in_user_data && !is_alternate_copy && element.name.local_name == "Paragraph" {
            if let Some(id) = paragraph_foreign_id(element) {
                foreign_ids.insert(id.to_owned());
            }
        }
        for child in &element.children {
            if let PreservedNode::Element(child) = child {
                collect(
                    child,
                    Some(element),
                    parent,
                    in_user_data || is_user_data,
                    foreign_ids,
                    project_ids,
                    anchor_ids,
                );
            }
        }
    }
    collect(root, None, None, false, foreign_ids, project_ids, anchor_ids);
}

fn clear_stale_anchor_bindings(
    nodes: &mut [PreservedNode],
    stale_anchors: &BTreeSet<SourceAnchorId>,
    removed_foreign_ids: &BTreeSet<String>,
) {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if element
            .anchor
            .as_ref()
            .is_some_and(|anchor| stale_anchors.contains(anchor))
        {
            element.anchor = None;
        }
        if matches!(&element.ownership, SourceOwnership::Attached(identity)
            if identity.domain == ForeignIdDomain::Paragraph
                && removed_foreign_ids.contains(&identity.value))
        {
            element.ownership = SourceOwnership::Opaque;
            element.anchor = None;
        }
        clear_stale_anchor_bindings(
            &mut element.children,
            stale_anchors,
            removed_foreign_ids,
        );
    }
}

fn paragraph_reference_touches(
    reference: &ForeignReference,
    removed_foreign_ids: &BTreeSet<String>,
) -> bool {
    [&reference.source, &reference.target]
        .into_iter()
        .any(|identity| {
            identity.domain == ForeignIdDomain::Paragraph
                && removed_foreign_ids.contains(&identity.value)
        })
}

fn remove_managed_at_location(
    sections: &mut [PreservedSection],
    location: &ManagedParagraphLocation,
) -> bool {
    let Some((&target_index, parent_path)) = location.node_path.split_last() else {
        return false;
    };
    let Some(section) = sections.get_mut(location.section_index) else {
        return false;
    };
    let parent_nodes = if parent_path.is_empty() {
        &mut section.children
    } else {
        let Some(parent) = element_at_path_mut(&mut section.children, parent_path) else {
            return false;
        };
        &mut parent.children
    };
    if target_index >= parent_nodes.len() {
        return false;
    }
    parent_nodes.remove(target_index);
    true
}

pub(super) fn paragraph_id_attribute_count(paragraph: &PreservedElement) -> usize {
    paragraph.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    }).count()
}

pub(super) fn set_paragraph_foreign_id(
    paragraph: &mut PreservedElement,
    value: &str,
) -> Result<(), PreservationError> {
    let indexes = paragraph.attributes.iter().enumerate().filter(|(_, attribute)| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    }).map(|(index, _)| index).collect::<Vec<_>>();
    match indexes.as_slice() {
        [index] => paragraph.attributes[*index].value = value.to_owned(),
        [] => paragraph.attributes.push(PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "ID".to_owned(),
                qualified_name: Some("ID".to_owned()),
            },
            value: value.to_owned(),
        }),
        _ => return Err(PreservationError::AmbiguousDirectAttribute),
    }
    Ok(())
}

pub(super) fn paragraph_foreign_id(paragraph: &PreservedElement) -> Option<&str> {
    let mut ids = paragraph.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    });
    let id = ids.next()?.value.as_str();
    if ids.next().is_some() {
        None
    } else {
        Some(id)
    }
}
