use super::identity::{validate_field, validate_identity, validate_ownership};
use super::navigation::{
    same_managed_paragraph_shell, same_sections_except_managed_location,
    source_text_bytes_in_element,
};
use super::text::{is_plain_managed_paragraph, is_text_element, same_direct_owner_opaque_children};
use super::*;
use crate::{ScreenplayElement, StableId};
use std::collections::{BTreeMap, BTreeSet};

fn validate_name(name: &SourceName) -> Result<(), PreservationError> {
    if name.local_name.is_empty() {
        return Err(PreservationError::InvalidSourceName);
    }
    validate_field(&name.local_name)?;
    if let Some(uri) = &name.namespace_uri {
        validate_field(uri)?;
    }
    if let Some(qualified) = &name.qualified_name {
        validate_field(qualified)?;
    }
    Ok(())
}

fn is_board_domain(domain: &ForeignIdDomain) -> bool {
    matches!(
        domain,
        ForeignIdDomain::ListItem | ForeignIdDomain::Board | ForeignIdDomain::Image
    )
}

impl PreservationState {
    pub fn scoped_global_invariants_match(&self, baseline: &PreservationState) -> bool {
        self.source == baseline.source
            && self.source_layout_settings == baseline.source_layout_settings
            && self.anchors == baseline.anchors
            && self.identities == baseline.identities
            && self.references == baseline.references
            && self.board_graph == baseline.board_graph
    }

    pub fn set_validated_source_text_bytes(&mut self, byte_count: usize) {
        self.source_text_bytes = byte_count;
    }

    /// Validates only a single managed paragraph against a complete valid
    /// baseline. It refuses any topology or identity change, leaving those
    /// cases for complete source-tree validation.
    pub fn validate_scoped_managed_edit(
        &self,
        baseline: &PreservationState,
        semantic: &ScreenplayElement,
    ) -> Result<usize, PreservationError> {
        let baseline_location = baseline
            .indexed_location(&semantic.id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let candidate_location = self
            .indexed_location(&semantic.id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if baseline_location != candidate_location
            || !baseline.is_supported_managed_body_location(&baseline_location, &semantic.id)
            || !self.is_supported_managed_body_location(&candidate_location, &semantic.id)
        {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        if !same_sections_except_managed_location(
            &baseline.sections,
            &self.sections,
            &candidate_location,
        ) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let before = baseline
            .managed_element_at(&baseline_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let after = self
            .managed_element_at(&candidate_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !matches!(&before.ownership, SourceOwnership::Managed(id) if id == &semantic.id)
            || !matches!(&after.ownership, SourceOwnership::Managed(id) if id == &semantic.id)
            || !is_plain_managed_paragraph(after)
            || !same_managed_paragraph_shell(before, after)
            || !same_direct_owner_opaque_children(before, after)
            || after.name.local_name != "Paragraph"
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let type_label = semantic.element_type();
        let namespace_qualified_type_count = after
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name.namespace_uri.is_some() && attribute.name.local_name == "Type"
            })
            .count();
        let role_attribute_count = after
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
            })
            .count();
        if after.source_type.as_deref() != Some(type_label.as_str())
            || role_attribute_count != 1
            || namespace_qualified_type_count != 0
            || !after.attributes.iter().any(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name == "Type"
                    && attribute.value == type_label.as_str()
            })
        {
            return Err(PreservationError::UnsupportedRoleMapping);
        }
        let mut source_projection = String::new();
        for child in &after.children {
            match child {
                PreservedNode::Text(text) | PreservedNode::CData(text)
                    if text.trim().is_empty() => {}
                PreservedNode::Text(text) | PreservedNode::CData(text) => {
                    source_projection.push_str(text)
                }
                PreservedNode::Element(text_element) if is_text_element(text_element, after) => {
                    for text_child in &text_element.children {
                        match text_child {
                            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                                source_projection.push_str(text)
                            }
                            // Text descendants outside direct character nodes are
                            // opaque topology, not screenplay text.
                            _ => {}
                        }
                    }
                }
                PreservedNode::Element(element) if is_text_element(element, after) => {
                    for text_child in &element.children {
                        match text_child {
                            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                                source_projection.push_str(text)
                            }
                            _ => return Err(PreservationError::UnsupportedTextMapping),
                        }
                    }
                }
                PreservedNode::Element(_)
                | PreservedNode::Comment(_)
                | PreservedNode::ProcessingInstruction { .. } => {}
                PreservedNode::Text(_) | PreservedNode::CData(_) => {
                    return Err(PreservationError::UnsupportedTextMapping)
                }
            }
        }
        if source_projection != semantic.text {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ScopedEditValidation,
            ));
        }
        let old_bytes = source_text_bytes_in_element(before);
        let new_bytes = source_text_bytes_in_element(after);
        let new_total = self
            .source_text_bytes
            .saturating_sub(old_bytes)
            .saturating_add(new_bytes);
        if new_total > MAX_SOURCE_TEXT_BYTES {
            return Err(PreservationError::TextTooLarge);
        }
        Ok(new_total)
    }

    pub fn validate(&self) -> Result<(), PreservationError> {
        if self.source.original_bytes.len() > MAX_SOURCE_BYTES {
            return Err(PreservationError::SourceTooLarge);
        }
        if std::str::from_utf8(&self.source.original_bytes).is_err() {
            return Err(PreservationError::InvalidEncoding);
        }
        if self.sections.len() > MAX_SOURCE_EVENTS {
            return Err(PreservationError::TooManyEvents);
        }

        let mut event_count = self.sections.len();
        let mut text_bytes = 0usize;
        let mut anchor_uses = BTreeMap::<SourceAnchorId, usize>::new();
        let mut ownership_bindings = Vec::new();
        let mut previous_section_order = None;
        for section in &self.sections {
            if previous_section_order.is_some_and(|order| section.ordinal <= order) {
                return Err(PreservationError::DuplicateSourceOrder);
            }
            previous_section_order = Some(section.ordinal);
            validate_name(&section.name)?;
            let mut stack = section
                .children
                .iter()
                .rev()
                .map(|node| (node, 1usize))
                .collect::<Vec<_>>();
            while let Some((node, depth)) = stack.pop() {
                if depth > MAX_SOURCE_DEPTH {
                    return Err(PreservationError::ExcessiveDepth);
                }
                event_count = event_count.saturating_add(1);
                if event_count > MAX_SOURCE_EVENTS {
                    return Err(PreservationError::TooManyEvents);
                }
                match node {
                    PreservedNode::Element(element) => {
                        validate_name(&element.name)?;
                        if let Some(source_type) = &element.source_type {
                            validate_field(source_type)?;
                        }
                        if element.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT {
                            return Err(PreservationError::TooManyAttributes);
                        }
                        if element.namespace_bindings.len() > MAX_ATTRIBUTES_PER_ELEMENT {
                            return Err(PreservationError::TooManyAttributes);
                        }
                        event_count = event_count
                            .saturating_add(element.attributes.len())
                            .saturating_add(element.namespace_bindings.len());
                        if event_count > MAX_SOURCE_EVENTS {
                            return Err(PreservationError::TooManyEvents);
                        }
                        for attribute in &element.attributes {
                            validate_name(&attribute.name)?;
                            validate_field(&attribute.value)?;
                        }
                        for binding in &element.namespace_bindings {
                            validate_field(&binding.prefix)?;
                            validate_field(&binding.namespace_uri)?;
                        }
                        if let Some(anchor) = &element.anchor {
                            *anchor_uses.entry(anchor.clone()).or_default() += 1;
                        }
                        ownership_bindings
                            .push((element.anchor.clone(), element.ownership.clone()));
                        validate_ownership(&element.ownership)?;
                        stack.extend(
                            element
                                .children
                                .iter()
                                .rev()
                                .map(|child| (child, depth.saturating_add(1))),
                        );
                    }
                    PreservedNode::Text(text)
                    | PreservedNode::CData(text)
                    | PreservedNode::Comment(text) => {
                        text_bytes = text_bytes.saturating_add(text.len());
                        if text_bytes > MAX_SOURCE_TEXT_BYTES {
                            return Err(PreservationError::TextTooLarge);
                        }
                    }
                    PreservedNode::ProcessingInstruction { target, data } => {
                        if target.is_empty() {
                            return Err(PreservationError::InvalidSourceName);
                        }
                        validate_field(target)?;
                        text_bytes = text_bytes.saturating_add(data.len());
                        if text_bytes > MAX_SOURCE_TEXT_BYTES {
                            return Err(PreservationError::TextTooLarge);
                        }
                    }
                }
            }
        }

        let mut defined = BTreeSet::new();
        event_count = event_count
            .saturating_add(self.anchors.len())
            .saturating_add(self.identities.len())
            .saturating_add(self.references.len())
            .saturating_add(self.board_graph.entities.len())
            .saturating_add(self.board_graph.references.len());
        if event_count > MAX_SOURCE_EVENTS {
            return Err(PreservationError::TooManyEvents);
        }
        for identity in self.identities.iter().chain(
            self.board_graph
                .entities
                .iter()
                .map(|entity| &entity.identity),
        ) {
            validate_identity(identity)?;
            if !defined.insert(identity.clone()) {
                return Err(PreservationError::DuplicateIdentity);
            }
        }

        let mut anchor_ids = BTreeSet::new();
        for anchor in &self.anchors {
            if !anchor_ids.insert(anchor.id.clone()) {
                return Err(PreservationError::DuplicateAnchor);
            }
            if anchor_uses.get(&anchor.id) != Some(&1) {
                return Err(PreservationError::UnboundAnchor);
            }
            if let AnchorTarget::Foreign(identity) = &anchor.target {
                if !defined.contains(identity) {
                    return Err(PreservationError::DanglingReference);
                }
            }
        }
        let anchor_targets = self
            .anchors
            .iter()
            .map(|anchor| (anchor.id.clone(), (&anchor.target, anchor.relation)))
            .collect::<BTreeMap<_, _>>();
        for (anchor, ownership) in &ownership_bindings {
            match ownership {
                SourceOwnership::Opaque => {}
                SourceOwnership::Managed(id) => {
                    if !anchor.as_ref().is_some_and(|anchor| {
                        matches!(
                            anchor_targets.get(anchor),
                            Some((AnchorTarget::ProjectElement(target), AnchorRelation::Represents))
                                if target == id
                        )
                    }) {
                        return Err(PreservationError::InvalidAnchor);
                    }
                }
                SourceOwnership::DualDialogueWrapper(id) => {
                    if !anchor.as_ref().is_some_and(|anchor| {
                        matches!(
                            anchor_targets.get(anchor),
                            Some((AnchorTarget::DualDialogueWrapper(target), AnchorRelation::Represents))
                                if target == id
                        )
                    }) {
                        return Err(PreservationError::InvalidAnchor);
                    }
                }
                SourceOwnership::Attached(identity) => {
                    if !defined.contains(identity)
                        || !anchor.as_ref().is_some_and(|anchor| {
                            matches!(
                                anchor_targets.get(anchor),
                                Some((AnchorTarget::Foreign(target), AnchorRelation::Attached))
                                    if target == identity
                            )
                        })
                    {
                        return Err(PreservationError::DanglingReference);
                    }
                }
            }
        }
        if anchor_uses
            .keys()
            .any(|anchor| !anchor_ids.contains(anchor))
        {
            return Err(PreservationError::UnboundAnchor);
        }

        for reference in self.references.iter().chain(&self.board_graph.references) {
            validate_identity(&reference.source)?;
            validate_identity(&reference.target)?;
            if !defined.contains(&reference.source) {
                return Err(PreservationError::MissingReferenceSource);
            }
            match reference.policy {
                ReferencePolicy::RequiredInternal => {
                    if !defined.contains(&reference.target) {
                        return Err(PreservationError::DanglingReference);
                    }
                }
                ReferencePolicy::CrossDomainMayOrphan => {
                    if is_board_domain(&reference.source.domain)
                        == is_board_domain(&reference.target.domain)
                    {
                        return Err(PreservationError::DanglingReference);
                    }
                }
            }
        }

        let mut image_bytes = 0usize;
        let mut previous_order = None;
        for entity in &self.board_graph.entities {
            if previous_order.is_some_and(|order| entity.source_order <= order) {
                return Err(PreservationError::DuplicateSourceOrder);
            }
            previous_order = Some(entity.source_order);
            if entity.kind == BoardEntityKind::Image {
                if let Some(payload) = &entity.image_payload {
                    if payload.len() > MAX_IMAGE_BYTES {
                        return Err(PreservationError::ImageTooLarge);
                    }
                    image_bytes = image_bytes.saturating_add(payload.len());
                    if image_bytes > MAX_TOTAL_IMAGE_BYTES {
                        return Err(PreservationError::TotalImageBytesTooLarge);
                    }
                    if entity
                        .declared_image_length
                        .is_some_and(|length| length != payload.len() as u64)
                    {
                        return Err(PreservationError::ImageLengthMismatch);
                    }
                }
            } else if entity.image_payload.is_some() || entity.declared_image_length.is_some() {
                return Err(PreservationError::ImageLengthMismatch);
            }
            if let Some(path) = &entity.source_path {
                validate_field(path)?;
            }
            if let Some(anchor) = &entity.anchor {
                if !anchor_ids.contains(anchor) {
                    return Err(PreservationError::UnboundAnchor);
                }
            }
        }
        Ok(())
    }

    pub fn validate_project_elements(
        &self,
        project_element_ids: &BTreeSet<StableId>,
        dual_dialogue_group_ids: &BTreeSet<StableId>,
    ) -> Result<(), PreservationError> {
        self.validate()?;
        for anchor in &self.anchors {
            match &anchor.target {
                AnchorTarget::ProjectElement(id) if !project_element_ids.contains(id) => {
                    return Err(PreservationError::AnchorTargetMissing);
                }
                AnchorTarget::DualDialogueWrapper(id) if !dual_dialogue_group_ids.contains(id) => {
                    return Err(PreservationError::AnchorTargetMissing);
                }
                _ => {}
            }
        }
        let mut stack = self
            .sections
            .iter()
            .flat_map(|section| section.children.iter())
            .collect::<Vec<_>>();
        while let Some(node) = stack.pop() {
            if let PreservedNode::Element(element) = node {
                if let SourceOwnership::Managed(id) = &element.ownership {
                    if !project_element_ids.contains(id) {
                        return Err(PreservationError::AnchorTargetMissing);
                    }
                }
                if let SourceOwnership::DualDialogueWrapper(id) = &element.ownership {
                    if !dual_dialogue_group_ids.contains(id) {
                        return Err(PreservationError::AnchorTargetMissing);
                    }
                }
                stack.extend(element.children.iter());
            }
        }
        Ok(())
    }

    /// Validates the binding and uniqueness rules for retained direct-root
    /// ElementSettings winners. Definitions without a source node may exist in
    /// non-FDX projects; every present definition anchor must resolve exactly.
    pub fn validate_element_definition_anchors(
        &self,
        definition_keys: &BTreeSet<crate::element_type::ElementTypeKey>,
    ) -> Result<(), PreservationError> {
        let mut winners = BTreeMap::<crate::element_type::ElementTypeKey, SourceAnchorId>::new();
        for anchor in &self.anchors {
            let AnchorTarget::ElementDefinition(key) = &anchor.target else {
                continue;
            };
            if anchor.relation != AnchorRelation::Represents {
                return Err(PreservationError::InvalidAnchor);
            }
            if !definition_keys.contains(key) {
                return Err(PreservationError::AnchorTargetMissing);
            }
            if winners.insert(key.clone(), anchor.id.clone()).is_some() {
                return Err(PreservationError::DuplicateAnchor);
            }
        }

        let mut found = BTreeSet::new();
        for section in &self.sections {
            for node in &section.children {
                let PreservedNode::Element(root) = node else {
                    continue;
                };
                for child in &root.children {
                    let PreservedNode::Element(setting) = child else {
                        continue;
                    };
                    if setting.name.local_name != "ElementSettings"
                        || setting.name.namespace_uri != root.name.namespace_uri
                    {
                        continue;
                    }
                    let type_attributes = setting
                        .attributes
                        .iter()
                        .filter(|attribute| {
                            attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name == "Type"
                        })
                        .collect::<Vec<_>>();
                    if type_attributes.len() != 1 {
                        if setting.anchor.as_ref().is_some_and(|anchor| {
                            self.anchors.iter().any(|candidate| {
                                candidate.id == *anchor
                                    && matches!(
                                        &candidate.target,
                                        AnchorTarget::ElementDefinition(_)
                                    )
                            })
                        }) {
                            return Err(PreservationError::InvalidAnchor);
                        }
                        continue;
                    }
                    let Some((key, winner_anchor)) = winners
                        .iter()
                        .find(|(key, _)| key.as_str() == type_attributes[0].value)
                    else {
                        continue;
                    };
                    let is_winner = setting.anchor.as_ref() == Some(winner_anchor);
                    if !is_winner && setting.anchor.is_some() {
                        return Err(PreservationError::InvalidAnchor);
                    }
                    if is_winner {
                        if setting.source_type.as_deref() != Some(key.as_str())
                            || !found.insert(key.clone())
                        {
                            return Err(PreservationError::InvalidAnchor);
                        }
                    }
                }
            }
        }
        if found.len() != winners.len() {
            return Err(PreservationError::UnboundAnchor);
        }
        Ok(())
    }
}
