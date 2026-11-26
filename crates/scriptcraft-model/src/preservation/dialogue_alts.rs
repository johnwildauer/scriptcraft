use super::error::PreservationError;
use super::state::PreservationState;
use super::tree::{
    DependencyConfidence, PreservedAttribute, PreservedElement, PreservedNode, SourceName,
    SourceOwnership,
};
use crate::dialogue_alts::{DialogueAltId, DialogueAlternatives};
use crate::StableId;

impl PreservationState {
    /// Reconciles the supported inline Alt projection for one managed owner.
    ///
    /// The baseline state authorizes edits to an existing inline projection;
    /// without a baseline, a same-namespace inline Alts child remains opaque.
    pub fn reconcile_managed_dialogue_alts(
        &mut self,
        owner_id: &StableId,
        baseline: Option<&DialogueAlternatives>,
        candidate: Option<&DialogueAlternatives>,
    ) -> Result<(), PreservationError> {
        if baseline.is_none() && candidate.is_none() {
            return Ok(());
        }

        let paragraph = self.managed_element_mut(owner_id)?;
        if paragraph.name.local_name != "Paragraph" {
            return Err(PreservationError::UnsupportedRoleMapping);
        }

        let paragraph_name = paragraph.name.clone();
        let mut children = paragraph.children.iter().cloned().collect::<Vec<_>>();
        let alts_indices = managed_alts_indices(&children, &paragraph_name);

        match (baseline, candidate) {
            (None, Some(alternatives)) => {
                if !alts_indices.is_empty() {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
                children.push(PreservedNode::Element(new_inline_alts(
                    paragraph,
                    alternatives,
                )));
            }
            (Some(_), Some(alternatives)) => {
                if alts_indices.len() > 1 {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
                if let Some(index) = alts_indices.first().copied() {
                    let PreservedNode::Element(existing) = &children[index] else {
                        return Err(PreservationError::UnsupportedRoleMapping);
                    };
                    let current_index = preflight_managed_inline_alts(existing, &paragraph_name)?;

                    let mut replacement_ids = alternatives
                        .inactive
                        .iter()
                        .map(|alternative| {
                            PreservedNode::Element(new_inline_alt_id(
                                paragraph,
                                alternative.id.to_string(),
                            ))
                        })
                        .collect::<Vec<_>>();
                    let mut replacement_children = Vec::new();
                    let mut inserted = false;
                    for child in existing.children.iter() {
                        if is_managed_alt_id(child, &paragraph_name) {
                            if !inserted {
                                replacement_children.append(&mut replacement_ids);
                                inserted = true;
                            }
                        } else {
                            replacement_children.push(child.clone());
                        }
                    }
                    if !inserted {
                        replacement_children.append(&mut replacement_ids);
                    }
                    let mut updated = existing.clone();
                    updated.attributes[current_index].value = alternatives.current.to_string();
                    updated.children = replacement_children.into();
                    children[index] = PreservedNode::Element(updated);
                } else {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
            }
            (Some(_), None) => {
                if alts_indices.len() > 1 {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
                if let Some(index) = alts_indices.first().copied() {
                    let PreservedNode::Element(existing) = &children[index] else {
                        return Err(PreservationError::UnsupportedRoleMapping);
                    };
                    preflight_managed_inline_alts(existing, &paragraph_name)?;
                    children.remove(index);
                } else {
                    return Ok(());
                }
            }
            (None, None) => return Ok(()),
        }

        paragraph.children = children.into();
        Ok(())
    }
}

fn managed_alts_indices(children: &[PreservedNode], paragraph: &SourceName) -> Vec<usize> {
    children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "Alts"
                    && element.name.namespace_uri == paragraph.namespace_uri =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect()
}

fn is_managed_alt_id(node: &PreservedNode, paragraph: &SourceName) -> bool {
    matches!(node,
        PreservedNode::Element(element)
            if element.name.local_name == "AltId"
                && element.name.namespace_uri == paragraph.namespace_uri)
}

fn preflight_managed_inline_alts(
    alts: &PreservedElement,
    paragraph: &SourceName,
) -> Result<usize, PreservationError> {
    let current_indices = alts
        .attributes
        .iter()
        .enumerate()
        .filter_map(|(index, attribute)| {
            (attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Current")
                .then_some(index)
        })
        .collect::<Vec<_>>();
    let [current_index] = current_indices.as_slice() else {
        return Err(PreservationError::AmbiguousDirectAttribute);
    };

    // Namespace declarations are stored separately in namespace_bindings. Any
    // other preserved attribute (including a namespaced Current) is semantic
    // source that this projection cannot safely overwrite or delete.
    if alts.attributes.len() != 1 || alts.source_type.is_some() {
        return Err(PreservationError::UnsupportedRoleMapping);
    }

    for child in alts.children.iter() {
        match child {
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            PreservedNode::Element(alt_id)
                if alt_id.name.local_name == "AltId"
                    && alt_id.name.namespace_uri == paragraph.namespace_uri =>
            {
                if !alt_id.attributes.is_empty() || alt_id.source_type.is_some() {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
                let mut value = String::new();
                for content in alt_id.children.iter() {
                    match content {
                        PreservedNode::Text(text) | PreservedNode::CData(text) => {
                            value.push_str(text);
                        }
                        PreservedNode::Element(_)
                        | PreservedNode::Comment(_)
                        | PreservedNode::ProcessingInstruction { .. } => {
                            return Err(PreservationError::UnsupportedRoleMapping);
                        }
                    }
                }
                if DialogueAltId::parse_decimal(&value).is_none() {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
            }
            PreservedNode::Text(_)
            | PreservedNode::CData(_)
            | PreservedNode::Element(_)
            | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => {
                return Err(PreservationError::UnsupportedRoleMapping);
            }
        }
    }

    Ok(*current_index)
}

fn new_inline_alts(
    paragraph: &PreservedElement,
    alternatives: &DialogueAlternatives,
) -> PreservedElement {
    let mut element = PreservedElement {
        name: lineage_name(&paragraph.name, "Alts"),
        attributes: vec![PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "Current".to_owned(),
                qualified_name: Some("Current".to_owned()),
            },
            value: alternatives.current.to_string(),
        }],
        namespace_bindings: paragraph.namespace_bindings.clone(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: Vec::new().into(),
    };
    for alternative in &alternatives.inactive {
        element
            .children
            .push(PreservedNode::Element(new_inline_alt_id(
                paragraph,
                alternative.id.to_string(),
            )));
    }
    element
}

fn new_inline_alt_id(paragraph: &PreservedElement, id: String) -> PreservedElement {
    PreservedElement {
        name: lineage_name(&paragraph.name, "AltId"),
        attributes: Vec::new(),
        namespace_bindings: paragraph.namespace_bindings.clone(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: vec![PreservedNode::Text(id.into())].into(),
    }
}

fn lineage_name(parent: &SourceName, local_name: &str) -> SourceName {
    let prefix = parent
        .qualified_name
        .as_deref()
        .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
    let qualified_name = prefix.map_or_else(
        || local_name.to_owned(),
        |prefix| format!("{prefix}:{local_name}"),
    );
    SourceName {
        namespace_uri: parent.namespace_uri.clone(),
        local_name: local_name.to_owned(),
        qualified_name: Some(qualified_name),
    }
}
