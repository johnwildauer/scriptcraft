use super::*;
use crate::StableId;
use super::text::{is_managed_paragraph_attribute, is_paragraph_id_attribute, is_text_element};

pub(super) fn find_managed_element_counted<'a>(
    sections: &'a [PreservedSection],
    id: &StableId,
) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &StableId) -> Option<&'a PreservedElement> {
        for node in nodes {
            if let PreservedNode::Element(element) = node {
                if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                    return Some(element);
                }
                if let Some(found) = find(&element.children, id) {
                    return Some(found);
                }
            }
        }
        None
    }
    sections
        .iter()
        .find_map(|section| find(&section.children, id))
}

pub(super) fn element_at_path<'a>(nodes: &'a [PreservedNode], path: &[usize]) -> Option<&'a PreservedElement> {
    let (index, rest) = path.split_first()?;
    let PreservedNode::Element(element) = nodes.get(*index)? else {
        return None;
    };
    if rest.is_empty() {
        Some(element)
    } else {
        element_at_path(&element.children, rest)
    }
}

fn indexed_element_context(mut nodes: &[PreservedNode], path: &[usize]) -> Option<(usize, bool)> {
    let mut inherited_space_preserve = false;
    for index in path {
        let PreservedNode::Element(element) = nodes.get(*index)? else {
            return None;
        };
        inherited_space_preserve = effective_xml_space_preserve(element, inherited_space_preserve);
        nodes = &element.children;
    }
    (!path.is_empty()).then_some((path.len().saturating_sub(1), inherited_space_preserve))
}

fn effective_xml_space_preserve(element: &PreservedElement, inherited: bool) -> bool {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.local_name == "space"
                && (attribute.name.namespace_uri.as_deref()
                    == Some("http://www.w3.org/XML/1998/namespace")
                    || attribute.name.qualified_name.as_deref() == Some("xml:space"))
        })
        .map_or(inherited, |attribute| match attribute.value.as_str() {
            "default" => false,
            "preserve" => true,
            _ => true,
        })
}

fn find_managed_context_counted(
    sections: &[PreservedSection],
    id: &StableId,
) -> Option<(usize, bool)> {
    fn visit(
        nodes: &[PreservedNode],
        id: &StableId,
        depth: usize,
        inherited_space_preserve: bool,
    ) -> Option<(usize, bool)> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            let space_preserve = effective_xml_space_preserve(element, inherited_space_preserve);
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some((depth, space_preserve));
            }
            if let Some(found) = visit(
                &element.children,
                id,
                depth.saturating_add(1),
                space_preserve,
            ) {
                return Some(found);
            }
        }
        None
    }

    sections
        .iter()
        .find_map(|section| visit(&section.children, id, 0, false))
}

pub(super) fn element_at_path_mut<'a>(
    nodes: &'a mut [PreservedNode],
    path: &[usize],
) -> Option<&'a mut PreservedElement> {
    let (index, rest) = path.split_first()?;
    let PreservedNode::Element(element) = nodes.get_mut(*index)? else {
        return None;
    };
    if rest.is_empty() {
        Some(element)
    } else {
        element_at_path_mut(&mut element.children, rest)
    }
}

pub(super) fn same_sections_except_managed_location(
    before: &[PreservedSection],
    after: &[PreservedSection],
    location: &ManagedParagraphLocation,
) -> bool {
    before.len() == after.len()
        && before
            .iter()
            .zip(after)
            .enumerate()
            .all(|(index, (left, right))| {
                left.name == right.name
                    && left.kind == right.kind
                    && left.ordinal == right.ordinal
                    && if index == location.section_index {
                        same_nodes_except_path(&left.children, &right.children, &location.node_path)
                    } else {
                        left.children.shares_storage_with(&right.children)
                    }
            })
}

fn same_nodes_except_path(
    before: &[PreservedNode],
    after: &[PreservedNode],
    path: &[usize],
) -> bool {
    if before.len() != after.len() {
        return false;
    }
    let Some((target_index, remaining_path)) = path.split_first() else {
        return before == after;
    };
    if *target_index >= before.len() {
        return false;
    }
    before
        .iter()
        .zip(after)
        .enumerate()
        .all(|(index, (left, right))| {
            if index != *target_index {
                return same_preserved_node_shallow(left, right);
            }
            if remaining_path.is_empty() {
                return matches!(
                    (left, right),
                    (PreservedNode::Element(_), PreservedNode::Element(_))
                );
            }
            match (left, right) {
                (PreservedNode::Element(left), PreservedNode::Element(right)) => {
                    left.name == right.name
                        && left.attributes == right.attributes
                        && left.namespace_bindings == right.namespace_bindings
                        && left.source_type == right.source_type
                        && left.ownership == right.ownership
                        && left.dependency_confidence == right.dependency_confidence
                        && left.anchor == right.anchor
                        && same_nodes_except_path(&left.children, &right.children, remaining_path)
                }
                _ => false,
            }
        })
}

fn same_preserved_node_shallow(before: &PreservedNode, after: &PreservedNode) -> bool {
    match (before, after) {
        (PreservedNode::Text(left), PreservedNode::Text(right))
        | (PreservedNode::CData(left), PreservedNode::CData(right))
        | (PreservedNode::Comment(left), PreservedNode::Comment(right)) => {
            left.shares_storage_with(right)
        }
        (
            PreservedNode::ProcessingInstruction {
                target: left_target,
                data: left_data,
            },
            PreservedNode::ProcessingInstruction {
                target: right_target,
                data: right_data,
            },
        ) => {
            left_target.shares_storage_with(right_target)
                && left_data.shares_storage_with(right_data)
        }
        (PreservedNode::Element(left), PreservedNode::Element(right)) => {
            left.name == right.name
                && left.attributes == right.attributes
                && left.namespace_bindings == right.namespace_bindings
                && left.source_type == right.source_type
                && left.ownership == right.ownership
                && left.dependency_confidence == right.dependency_confidence
                && left.anchor == right.anchor
                && left.children.shares_storage_with(&right.children)
        }
        _ => false,
    }
}

pub(super) fn same_managed_paragraph_shell(
    before: &PreservedElement,
    after: &PreservedElement,
) -> bool {
    let opaque_attributes = |element: &PreservedElement| element.attributes.iter().filter(|attribute| {
        !is_paragraph_id_attribute(attribute) && !is_managed_paragraph_attribute(attribute)
    }).cloned().collect::<Vec<_>>();
    let id_attributes = |element: &PreservedElement| element.attributes.iter()
        .filter(|attribute| is_paragraph_id_attribute(attribute)).cloned().collect::<Vec<_>>();
    before.name == after.name
        && before.namespace_bindings == after.namespace_bindings
        && before.ownership == after.ownership
        && before.dependency_confidence == after.dependency_confidence
        && before.anchor == after.anchor
        && (before.source_type == after.source_type || before.name.local_name == "Paragraph")
        && opaque_attributes(before) == opaque_attributes(after)
        && id_attributes(before) == id_attributes(after)
}

pub(super) fn source_text_bytes_in_element(element: &PreservedElement) -> usize {
    let mut total = 0usize;
    let mut stack = vec![element];
    while let Some(current) = stack.pop() {
        for child in &current.children {
            match child {
                PreservedNode::Element(nested) => stack.push(nested),
                PreservedNode::Text(text)
                | PreservedNode::CData(text)
                | PreservedNode::Comment(text) => total = total.saturating_add(text.len()),
                PreservedNode::ProcessingInstruction { data, .. } => {
                    total = total.saturating_add(data.len())
                }
            }
        }
    }
    total
}

pub(super) fn contains_managed_element_counted(nodes: &[PreservedNode], id: &StableId) -> bool {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
            || contains_managed_element_counted(&element.children, id)
        {
            return true;
        }
    }
    false
}

impl PreservationState {
/// Looks up a managed element through the preservation-owned location index.
    /// A missing index retains the counted complete-search fallback for callers
    /// operating on externally constructed preservation states.
    pub fn managed_element(
        &self,
        id: &StableId,
    ) -> Result<Option<&PreservedElement>, PreservationError> {
        match self.indexed_location(id) {
            Ok(Some(location)) => {
                let element = self.managed_element_at(location);
                if element.is_some_and(|element| {
                    !matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
                }) {
                    return Err(PreservationError::ManagedAnchorMissing);
                }
                Ok(element)
            }
            Ok(None) if self.managed_paragraph_index.is_none() => {
                Ok(find_managed_element_counted(&self.sections, id))
            }
            Ok(None) => Ok(None),
            Err(error) => Err(error),
        }
    }

/// Gets the indexed source location's document depth and inherited
    /// `xml:space` state without walking unrelated source branches.
    pub fn managed_element_context(
        &self,
        id: &StableId,
    ) -> Result<Option<(usize, bool)>, PreservationError> {
        let location = match self.indexed_location(id) {
            Ok(Some(location)) => location,
            Ok(None) if self.managed_paragraph_index.is_none() => {
                return Ok(find_managed_context_counted(&self.sections, id));
            }
            Ok(None) => return Ok(None),
            Err(error) => return Err(error),
        };
        let section = self
            .sections
            .get(location.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !self.managed_element_at(location).is_some_and(|element| {
            matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
        }) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        indexed_element_context(&section.children, &location.node_path)
            .map(Some)
            .ok_or(PreservationError::ManagedAnchorMissing)
    }

/// Mutable indexed access for a detached candidate. The source-use index is
    /// invalidated before mutation; identity-sensitive consumers refresh it on
    /// demand while paragraph locations remain available for scoped validation.
    pub fn managed_element_mut_indexed(
        &mut self,
        id: &StableId,
    ) -> Result<&mut PreservedElement, PreservationError> {
        self.ensure_managed_paragraph_index();
        self.invalidate_source_identity_use_index();
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let section = self
            .sections
            .get_mut(location.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let element = element_at_path_mut(&mut section.children, &location.node_path)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        Ok(element)
    }

pub(super) fn indexed_location(
        &self,
        id: &StableId,
    ) -> Result<Option<&ManagedParagraphLocation>, PreservationError> {
        let Some(index) = &self.managed_paragraph_index else {
            return Ok(None);
        };
        let locations = index.get(id).map(Vec::as_slice).unwrap_or_default();
        match locations {
            [] => Ok(None),
            [location] => Ok(Some(location)),
            _ => Err(PreservationError::ManagedAnchorAmbiguous),
        }
    }

pub(super) fn managed_element_at(&self, location: &ManagedParagraphLocation) -> Option<&PreservedElement> {
        element_at_path(
            self.sections.get(location.section_index)?.children.as_ref(),
            &location.node_path,
        )
    }

pub(super) fn is_supported_managed_body_location(
        &self,
        location: &ManagedParagraphLocation,
        id: &StableId,
    ) -> bool {
        let Some(section) = self.sections.get(location.section_index) else {
            return false;
        };
        let mut nodes = section.children.as_ref();
        let mut chain = Vec::with_capacity(location.node_path.len());
        for index in &location.node_path {
            let Some(PreservedNode::Element(element)) = nodes.get(*index) else {
                return false;
            };
            chain.push(element);
            nodes = &element.children;
        }
        let Some(root) = chain.first() else {
            return false;
        };
        if root.name.local_name != "FinalDraft" {
            return false;
        }
        let namespace = root.name.namespace_uri.as_deref();
        if chain
            .iter()
            .any(|element| element.name.namespace_uri.as_deref() != namespace)
        {
            return false;
        }
        match chain.as_slice() {
            [_, content, paragraph]
                if content.name.local_name == "Content"
                    && paragraph.name.local_name == "Paragraph"
                    && matches!(&paragraph.ownership, SourceOwnership::Managed(target) if target == id) =>
            {
                true
            }
            [_, content, wrapper, dual, paragraph]
                if content.name.local_name == "Content"
                    && wrapper.name.local_name == "Paragraph"
                    && wrapper.source_type.as_deref() == Some("General")
                    && matches!(&wrapper.ownership, SourceOwnership::DualDialogueWrapper(_))
                    && dual.name.local_name == "DualDialogue"
                    && paragraph.name.local_name == "Paragraph"
                    && matches!(&paragraph.ownership, SourceOwnership::Managed(target) if target == id) =>
            {
                let Some(dual_index) = location.node_path.get(3) else {
                    return false;
                };
                let Some(member_index) = location.node_path.get(4) else {
                    return false;
                };
                let Some(PreservedNode::Element(wrapper_dual)) = wrapper.children.get(*dual_index)
                else {
                    return false;
                };
                if wrapper_dual.name.local_name != "DualDialogue"
                    || wrapper_dual.children.get(*member_index).is_none_or(|node| {
                        !matches!(node, PreservedNode::Element(member) if matches!(&member.ownership, SourceOwnership::Managed(target) if target == id))
                    })
                {
                    return false;
                }
                let spell_blocks = wrapper_dual
                    .children
                    .iter()
                    .enumerate()
                    .filter_map(|(index, node)| match node {
                        PreservedNode::Element(element)
                            if element.name.local_name == "SpellCheckIgnoreLists" => Some(index),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                spell_blocks.len() == 1 && spell_blocks[0] < *member_index
            }
            _ => false,
        }
    }

pub fn has_managed_paragraph_index(&self) -> bool {
        self.managed_paragraph_index.is_some()
    }

pub fn dual_dialogue_wrapper_anchor(&self, id: &StableId) -> Option<SourceAnchorId> {
        let mut stack = self.sections.iter().flat_map(|section| section.children.iter()).collect::<Vec<_>>();
        while let Some(node) = stack.pop() {
            if let PreservedNode::Element(element) = node {
                if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == id) {
                    return element.anchor.clone();
                }
                stack.extend(element.children.iter());
            }
        }
        None
    }

pub fn has_managed_element(&self, id: &StableId) -> bool {
        if let Some(index) = &self.managed_paragraph_index {
            let locations = index.get(id).map(Vec::as_slice).unwrap_or_default();
            return !locations.is_empty();
        }
        self.sections
            .iter()
            .any(|section| contains_managed_element_counted(&section.children, id))
    }

/// Returns whether the exact managed Paragraph owns a direct inline Alts
    /// child. The codec has already validated its role, namespace, and
    /// multiplicity; callers use this only to guard role transitions.
    pub fn has_inline_alts(&self, id: &StableId) -> bool {
        let paragraph = match self.indexed_location(id) {
            Ok(Some(location)) => self.managed_element_at(location),
            Ok(None) if self.managed_paragraph_index.is_none() => {
                find_managed_element_counted(&self.sections, id)
            }
            _ => None,
        };
        paragraph.is_some_and(|paragraph| {
            paragraph.children.iter().any(|node| matches!(node,
                PreservedNode::Element(element)
                    if element.name.local_name == "Alts"
                        && element.name.namespace_uri == paragraph.name.namespace_uri
            ))
        })
    }

pub(super) fn has_owner_opaque_data(&self, id: &StableId) -> bool {
        let paragraph = match self.indexed_location(id) {
            Ok(Some(location)) => self.managed_element_at(location),
            Ok(None) if self.managed_paragraph_index.is_none() => {
                find_managed_element_counted(&self.sections, id)
            }
            _ => None,
        };
        paragraph.is_some_and(|paragraph| {
            paragraph.attributes.iter().any(|attribute| {
                !is_paragraph_id_attribute(attribute) && !is_managed_paragraph_attribute(attribute)
            }) || paragraph.children.iter().any(|node| match node {
                PreservedNode::Element(element) => !is_text_element(element, paragraph)
                    || element.children.iter().any(|child| !matches!(child,
                        PreservedNode::Text(_) | PreservedNode::CData(_))),
                PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => true,
            })
        })
    }

pub(super) fn managed_element_mut(
        &mut self,
        id: &StableId,
    ) -> Result<&mut PreservedElement, PreservationError> {
        self.ensure_managed_paragraph_index();
        self.invalidate_source_identity_use_index();
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let indexed_target_matches = self
            .sections
            .get(location.section_index)
            .and_then(|section| element_at_path(&section.children, &location.node_path))
            .is_some_and(|element| {
                matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
            });
        let location = if indexed_target_matches {
            location
        } else {
            // Structural source reconciliation can invalidate a cached path.
            // Never let a stale location apply a text edit to a different
            // managed paragraph; rebuild from the authoritative tree and
            // resolve the requested identity again before returning it.
            self.rebuild_managed_paragraph_index();
            self.indexed_location(id)?
                .cloned()
                .ok_or(PreservationError::ManagedAnchorMissing)?
        };
        let section = self
            .sections
            .get_mut(location.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let element = element_at_path_mut(&mut section.children, &location.node_path)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        Ok(element)
    }
}
