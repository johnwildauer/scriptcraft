use super::*;
use crate::formatting::FixedPoint;
use crate::{BodyOwner, ScreenplayElement, SourceProvenance, SourceSection};
use crate::{SourceFormat, StableId};
use std::collections::BTreeSet;

use super::navigation::element_at_path_mut;
use super::reconciliation::{
    clear_stale_anchor_bindings, managed_child_index, paragraph_foreign_id,
    paragraph_id_attribute_count, paragraph_template, plain_source_name,
    ManagedParagraphTemplate,
};
use super::text::{semantic_text_runs, source_name_from_qname};

pub(super) fn is_exact_dual_dialogue_member_location(
    sections: &[PreservedSection],
    location: &ManagedParagraphLocation,
    group_id: &StableId,
) -> bool {
    let [root_index, content_index, wrapper_index, dual_index, paragraph_index] =
        location.node_path.as_slice()
    else {
        return false;
    };
    let Some(section) = sections.get(location.section_index) else {
        return false;
    };
    let Some(PreservedNode::Element(root)) = section.children.get(*root_index) else {
        return false;
    };
    let Some(PreservedNode::Element(content)) = root.children.get(*content_index) else {
        return false;
    };
    let Some(PreservedNode::Element(wrapper)) = content.children.get(*wrapper_index) else {
        return false;
    };
    let Some(PreservedNode::Element(dual)) = wrapper.children.get(*dual_index) else {
        return false;
    };
    let Some(PreservedNode::Element(paragraph)) = dual.children.get(*paragraph_index) else {
        return false;
    };
    let dual_count = wrapper
        .children
        .iter()
        .filter(|node| {
            matches!(node,
                PreservedNode::Element(element) if element.name.local_name == "DualDialogue"
                    && element.name.namespace_uri == wrapper.name.namespace_uri
            )
        })
        .count();
    root.name.local_name == "FinalDraft"
        && content.name.local_name == "Content"
        && root.name.namespace_uri == content.name.namespace_uri
        && wrapper.name.local_name == "Paragraph"
        && wrapper.name.namespace_uri == content.name.namespace_uri
        && matches!(&wrapper.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
        && dual.name.local_name == "DualDialogue"
        && dual.name.namespace_uri == wrapper.name.namespace_uri
        && dual_count == 1
        && paragraph.name.local_name == "Paragraph"
        && paragraph.name.namespace_uri == dual.name.namespace_uri
        && matches!(&paragraph.ownership, SourceOwnership::Managed(_))
}

impl PreservationState {
    /// Moves the exact existing managed body Paragraph nodes under a generated
    /// General/DualDialogue wrapper. This runs only on a detached transaction
    /// candidate; all indexes are rebuilt before the candidate can be published.
    pub fn wrap_dual_dialogue_members(
        &mut self,
        group_id: &StableId,
        member_ids: &[StableId],
        source_ordinal: u32,
        overrides: Option<crate::formatting::SourceParagraphOverrides>,
    ) -> Result<(String, SourceAnchorId, u32), PreservationError> {
        if self.source.format != SourceFormat::Fdx || member_ids.len() < 2 {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        self.ensure_managed_paragraph_index();
        self.ensure_source_identity_use_index();
        let mut locations = Vec::with_capacity(member_ids.len());
        for id in member_ids {
            let location = self
                .indexed_location(id)?
                .cloned()
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if location.node_path.len() != 3 {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            locations.push(location);
        }
        let first = locations
            .first()
            .ok_or(PreservationError::UnsupportedTextMapping)?;
        if locations.iter().any(|location| {
            location.section_index != first.section_index
                || location.node_path[..2] != first.node_path[..2]
        }) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let root_index = first.node_path[0];
        let content_index = first.node_path[1];
        let child_indexes = locations
            .iter()
            .map(|location| location.node_path[2])
            .collect::<Vec<_>>();
        if child_indexes.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let section = self
            .sections
            .get(first.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let root = match section.children.get(root_index) {
            Some(PreservedNode::Element(root)) if root.name.local_name == "FinalDraft" => root,
            _ => return Err(PreservationError::UnsupportedTextMapping),
        };
        let content = match root.children.get(content_index) {
            Some(PreservedNode::Element(content))
                if content.name.local_name == "Content"
                    && content.name.namespace_uri == root.name.namespace_uri =>
            {
                content
            }
            _ => return Err(PreservationError::UnsupportedTextMapping),
        };
        for (id, child_index) in member_ids.iter().zip(&child_indexes) {
            if !matches!(content.children.get(*child_index), Some(PreservedNode::Element(paragraph))
                if paragraph.name.local_name == "Paragraph"
                    && matches!(&paragraph.ownership, SourceOwnership::Managed(owner) if owner == id))
            {
                return Err(PreservationError::ManagedAnchorMissing);
            }
        }
        for pair in child_indexes.windows(2) {
            if content.children[pair[0] + 1..pair[1]].iter().any(|node| {
                !matches!(node, PreservedNode::Text(text) if text.chars().all(char::is_whitespace))
            }) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
        }
        let root_namespace = content.name.namespace_uri.clone();
        let prefix = content
            .name
            .qualified_name
            .as_deref()
            .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix.to_owned()));
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let mut outer = source_element("Paragraph", &root_namespace, prefix.as_deref());
        outer.source_type = Some("General".to_owned());
        outer.ownership = SourceOwnership::DualDialogueWrapper(group_id.clone());
        outer.anchor = Some(anchor_id.clone());
        push_source_attribute(&mut outer, "Type", "General");
        push_source_attribute(&mut outer, "id", &foreign_id);
        if let Some(overrides) = overrides {
            push_wrapper_overrides(&mut outer, overrides);
        }
        let mut dual = source_element("DualDialogue", &root_namespace, prefix.as_deref());
        dual.ownership = SourceOwnership::Opaque;
        let mut spell = source_element("SpellCheckIgnoreLists", &root_namespace, prefix.as_deref());
        spell.ownership = SourceOwnership::Opaque;
        for name in ["IgnoredRanges", "IgnoredWords"] {
            let mut list = source_element(name, &root_namespace, prefix.as_deref());
            list.ownership = SourceOwnership::Opaque;
            spell.children.push(PreservedNode::Element(list));
        }
        dual.children.push(PreservedNode::Element(spell));

        let section = self
            .sections
            .get_mut(first.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let root = match section.children.get_mut(root_index) {
            Some(PreservedNode::Element(root)) => root,
            _ => return Err(PreservationError::ManagedAnchorMissing),
        };
        let content = match root.children.get_mut(content_index) {
            Some(PreservedNode::Element(content)) => content,
            _ => return Err(PreservationError::ManagedAnchorMissing),
        };
        let mut moved = Vec::with_capacity(child_indexes.len());
        for child_index in child_indexes.iter().rev() {
            moved.push(content.children.remove(*child_index));
        }
        moved.reverse();
        dual.children.extend(moved);
        outer.children.push(PreservedNode::Element(dual));
        content
            .children
            .insert(child_indexes[0], PreservedNode::Element(outer));
        self.identities.push(ForeignIdentity::new(
            ForeignIdDomain::Paragraph,
            foreign_id.clone(),
        )?);
        self.anchors.push(SourceAnchor {
            id: anchor_id.clone(),
            target: AnchorTarget::DualDialogueWrapper(group_id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
        Ok((foreign_id, anchor_id, source_ordinal))
    }

    /// Removes only the resolved wrapper and its nested spell-check subtree,
    /// restoring the exact managed member Paragraph nodes at the wrapper slot.
    pub fn unwrap_dual_dialogue_members(
        &mut self,
        group_id: &StableId,
        member_ids: &[StableId],
        provenance: &crate::dual_dialogue::DualDialogueWrapperProvenance,
    ) -> Result<(), PreservationError> {
        if self.source.format != SourceFormat::Fdx || member_ids.len() < 2 {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let wrapper_anchor = provenance
            .preservation_anchor
            .as_ref()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let foreign_id = provenance.outer_foreign_id.as_deref();
        if !provenance.source.as_ref().is_some_and(|source| {
            source.format == SourceFormat::Fdx
                && source.section == SourceSection::Content
                && source.foreign_id.as_deref() == foreign_id
                && source.foreign_role.as_deref() == Some("General")
        }) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        self.ensure_managed_paragraph_index();
        let mut wrapper_locations = Vec::new();
        for (section_index, section) in self.sections.iter().enumerate() {
            for (root_index, root_node) in section.children.iter().enumerate() {
                let PreservedNode::Element(root) = root_node else {
                    continue;
                };
                if root.name.local_name != "FinalDraft" {
                    continue;
                }
                for (content_index, content_node) in root.children.iter().enumerate() {
                    let PreservedNode::Element(content) = content_node else {
                        continue;
                    };
                    if content.name.local_name != "Content"
                        || content.name.namespace_uri != root.name.namespace_uri
                    {
                        continue;
                    }
                    for (outer_index, outer_node) in content.children.iter().enumerate() {
                        if matches!(outer_node, PreservedNode::Element(outer)
                            if matches!(&outer.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
                                && outer.name.local_name == "Paragraph"
                                && outer.name.namespace_uri == content.name.namespace_uri)
                        {
                            wrapper_locations.push((
                                section_index,
                                root_index,
                                content_index,
                                outer_index,
                            ));
                        }
                    }
                }
            }
        }
        let [(section_index, root_index, content_index, outer_index)] =
            wrapper_locations.as_slice()
        else {
            return Err(PreservationError::ManagedAnchorMissing);
        };
        let section = self
            .sections
            .get(*section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let PreservedNode::Element(root) = section
            .children
            .get(*root_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?
        else {
            return Err(PreservationError::ManagedAnchorMissing);
        };
        let PreservedNode::Element(content) = root
            .children
            .get(*content_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?
        else {
            return Err(PreservationError::ManagedAnchorMissing);
        };
        let PreservedNode::Element(outer) = content
            .children
            .get(*outer_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?
        else {
            return Err(PreservationError::ManagedAnchorMissing);
        };
        let source_foreign_id = paragraph_foreign_id(outer);
        let source_type_count = outer
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
            })
            .count();
        if source_foreign_id != foreign_id
            || paragraph_id_attribute_count(outer) != usize::from(foreign_id.is_some())
            || outer.anchor.as_ref() != Some(wrapper_anchor)
            || outer.source_type.as_deref() != Some("General")
            || source_type_count != 1
            || !outer.attributes.iter().any(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name == "Type"
                    && attribute.value == "General"
            })
        {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let wrapper_anchor_records = self
            .anchors
            .iter()
            .filter(|anchor| anchor.id == *wrapper_anchor)
            .collect::<Vec<_>>();
        if !matches!(wrapper_anchor_records.as_slice(), [anchor]
            if anchor.target == AnchorTarget::DualDialogueWrapper(group_id.clone())
                && anchor.relation == AnchorRelation::Represents)
        {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let wrapper_identity = if let Some(foreign_id) = foreign_id {
            let identity = ForeignIdentity::new(ForeignIdDomain::Paragraph, foreign_id)?;
            let registrations = self
                .identities
                .iter()
                .filter(|registered| **registered == identity)
                .count();
            if registrations != 1 {
                return Err(PreservationError::ManagedAnchorMissing);
            }
            Some(identity)
        } else {
            None
        };
        let duals = outer
            .children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element) if element.name.local_name == "DualDialogue" => {
                    Some(element)
                }
                PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => None,
                _ => None,
            })
            .collect::<Vec<_>>();
        if duals.len() != 1
            || duals[0].name.namespace_uri != outer.name.namespace_uri
            || outer.children.iter().any(|node| match node {
                PreservedNode::Element(element) => {
                    element.name.local_name == "DualDialogue"
                        && element.name.namespace_uri != outer.name.namespace_uri
                }
                PreservedNode::Text(text) | PreservedNode::CData(text) => {
                    !text.chars().all(char::is_whitespace)
                }
                _ => false,
            })
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let dual = duals[0];
        let mut spell_count = 0usize;
        let mut found_members = Vec::with_capacity(member_ids.len());
        for node in dual.children.iter() {
            match node {
                PreservedNode::Element(element)
                    if element.name.local_name == "SpellCheckIgnoreLists" =>
                {
                    spell_count += 1
                }
                PreservedNode::Element(element) if element.name.local_name == "Paragraph" => {
                    if let SourceOwnership::Managed(id) = &element.ownership {
                        found_members.push(id.clone());
                    } else {
                        return Err(PreservationError::UnsupportedTextMapping);
                    }
                }
                PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if spell_count != 1 || found_members != member_ids {
            return Err(PreservationError::ManagedAnchorMissing);
        }

        let member_nodes = dual
            .children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element)
                    if matches!(&element.ownership, SourceOwnership::Managed(_)) =>
                {
                    Some(node.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut member_identities = Vec::new();
        for node in &member_nodes {
            let PreservedNode::Element(member) = node else {
                continue;
            };
            if paragraph_id_attribute_count(member) > 1 {
                return Err(PreservationError::AmbiguousDirectAttribute);
            }
            let Some(foreign_id) = paragraph_foreign_id(member) else {
                continue;
            };
            let identity = ForeignIdentity::new(ForeignIdDomain::Paragraph, foreign_id)?;
            member_identities.push(identity);
        }
        let mut new_member_identity_values = BTreeSet::new();
        let new_member_identities = member_identities
            .into_iter()
            .filter(|identity| {
                !self.identities.contains(identity)
                    && new_member_identity_values.insert(identity.clone())
            })
            .collect::<Vec<_>>();

        let wrapper_identity_set = wrapper_identity.iter().cloned().collect::<BTreeSet<_>>();
        if self.board_graph.references.iter().any(|reference| {
            reference.policy == ReferencePolicy::RequiredInternal
                && wrapper_identity_set
                    .iter()
                    .any(|identity| reference.source == *identity || reference.target == *identity)
        }) {
            return Err(PreservationError::PotentialReferenceConflict);
        }

        let mut stale_anchors = self
            .anchors
            .iter()
            .filter(|anchor| {
                anchor.id == *wrapper_anchor
                    || matches!(&anchor.target,
                        AnchorTarget::DualDialogueWrapper(target) if target == group_id)
            })
            .map(|anchor| anchor.id.clone())
            .collect::<BTreeSet<_>>();
        collect_subtree_anchor_ids(outer, &mut stale_anchors);
        let section = self
            .sections
            .get_mut(*section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let PreservedNode::Element(root) = section
            .children
            .get_mut(*root_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?
        else {
            return Err(PreservationError::ManagedAnchorMissing);
        };
        let PreservedNode::Element(content) = root
            .children
            .get_mut(*content_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?
        else {
            return Err(PreservationError::ManagedAnchorMissing);
        };
        content.children.remove(*outer_index);
        for (offset, member) in member_nodes.into_iter().enumerate() {
            content.children.insert(*outer_index + offset, member);
        }
        self.identities.extend(new_member_identities);
        for section in &mut self.sections {
            clear_stale_anchor_bindings(
                &mut section.children,
                &stale_anchors,
                &BTreeSet::new(),
            );
        }
        self.anchors
            .retain(|anchor| !stale_anchors.contains(&anchor.id));
        self.invalidate_source_identity_use_index();
        if let Some(identity) = wrapper_identity {
            self.prune_orphaned_paragraph_identities(&BTreeSet::from([identity.value]));
        }
        self.source_layout_settings
            .direct_paragraph_overrides
            .remove(group_id);
        self.rebuild_managed_paragraph_index();
        Ok(())
    }
}

fn collect_subtree_anchor_ids(
    element: &PreservedElement,
    anchors: &mut BTreeSet<SourceAnchorId>,
) {
    if matches!(&element.ownership, SourceOwnership::Managed(_)) {
        return;
    }
    if let Some(anchor) = &element.anchor {
        anchors.insert(anchor.clone());
    }
    for child in &element.children {
        if let PreservedNode::Element(child) = child {
            collect_subtree_anchor_ids(child, anchors);
        }
    }
}

fn source_element(
    local_name: &str,
    namespace_uri: &Option<String>,
    prefix: Option<&str>,
) -> PreservedElement {
    let qualified_name = prefix.map_or_else(
        || local_name.to_owned(),
        |prefix| format!("{prefix}:{local_name}"),
    );
    PreservedElement {
        name: SourceName {
            namespace_uri: namespace_uri.clone(),
            local_name: local_name.to_owned(),
            qualified_name: Some(qualified_name),
        },
        attributes: Vec::new(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: SharedVec::default(),
    }
}

fn push_source_attribute(element: &mut PreservedElement, local_name: &str, value: &str) {
    element.attributes.push(PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: local_name.to_owned(),
            qualified_name: Some(local_name.to_owned()),
        },
        value: value.to_owned(),
    });
}

fn push_wrapper_overrides(
    element: &mut PreservedElement,
    overrides: crate::formatting::SourceParagraphOverrides,
) {
    if let Some(value) = overrides.paragraph.left_indent {
        push_source_attribute(element, "LeftIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.right_indent {
        push_source_attribute(element, "RightIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.space_before {
        push_source_attribute(element, "SpaceBefore", &value.decimal_string());
    }
    if let Some(value) = overrides.starts_new_page {
        push_source_attribute(element, "StartsNewPage", if value { "Yes" } else { "No" });
    }
}

fn fixed_inches_string(value: FixedPoint) -> String {
    let raw = value.raw();
    const PER_INCH: u64 = 7_200;
    let magnitude = raw.unsigned_abs();
    let whole = magnitude / PER_INCH;
    let remainder = magnitude % PER_INCH;
    if remainder == 0 {
        return format!("{}{whole}", if raw < 0 { "-" } else { "" });
    }
    let mut fraction = format!("{remainder:06}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{}{whole}.{fraction}", if raw < 0 { "-" } else { "" })
}


impl PreservationState {
    pub(super) fn insert_nested_managed_paragraph(
        &mut self,
        semantic: &ScreenplayElement,
        index: usize,
        group: &crate::dual_dialogue::DualDialogueGroup,
    ) -> Result<SourceProvenance, PreservationError> {
        self.ensure_managed_paragraph_index();
        let member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        let template_location = member_ids
            .iter()
            .filter(|id| *id != &semantic.id)
            .find_map(|id| self.indexed_location(id).ok().flatten().cloned())
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !is_exact_dual_dialogue_member_location(&self.sections, &template_location, &group.id) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        for member_id in member_ids.iter().filter(|id| *id != &semantic.id) {
            let location = self
                .indexed_location(member_id)?
                .cloned()
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if !is_exact_dual_dialogue_member_location(&self.sections, &location, &group.id)
                || location.section_index != template_location.section_index
                || location.node_path[..4] != template_location.node_path[..4]
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
        }
        let template = self
            .managed_element_at(&template_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let ManagedParagraphTemplate {
            paragraph_name,
            paragraph_bindings,
            text_name,
            text_bindings,
            id_spelling,
        } = paragraph_template(&template)?;
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        // Managed Type/provenance projections use the exact key facade; the
        // surrounding group-shape checks remain transitional static semantics.
        let type_label = semantic.element_type();
        let mut paragraph = PreservedElement {
            name: paragraph_name,
            attributes: vec![
                PreservedAttribute {
                    name: plain_source_name("Type"),
                    value: type_label.as_str().to_owned(),
                },
                PreservedAttribute {
                    name: plain_source_name(&id_spelling),
                    value: foreign_id.clone(),
                },
            ],
            namespace_bindings: paragraph_bindings,
            source_type: Some(type_label.as_str().to_owned()),
            ownership: SourceOwnership::Managed(semantic.id.clone()),
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: Some(anchor_id.clone()),
            children: SharedVec::default(),
        };
        for run in semantic_text_runs(semantic) {
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

        let member_position = member_ids
            .iter()
            .position(|id| id == &semantic.id)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let parent_path = &template_location.node_path[..4];
        let nodes = {
            let section = self
                .sections
                .get_mut(template_location.section_index)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let parent = element_at_path_mut(&mut section.children, parent_path)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            &mut parent.children
        };
        let mut insert_at = None;
        for following in member_ids.iter().skip(member_position + 1) {
            if let Some(position) = managed_child_index(nodes, following) {
                insert_at = Some(position);
                break;
            }
        }
        if insert_at.is_none() {
            for preceding in member_ids[..member_position].iter().rev() {
                if let Some(position) = managed_child_index(nodes, preceding) {
                    insert_at = Some(position + 1);
                    break;
                }
            }
        }
        let position = insert_at.ok_or(PreservationError::ManagedAnchorMissing)?;
        nodes.insert(position, PreservedNode::Element(paragraph));
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(semantic.id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(
            ForeignIdDomain::Paragraph,
            foreign_id.clone(),
        )?);
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
        Ok(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::Content,
            ordinal: u32::try_from(index).map_err(|_| PreservationError::TooManyEvents)?,
            foreign_id: Some(foreign_id),
            foreign_role: Some(type_label.as_str().to_owned()),
        })
    }
}
