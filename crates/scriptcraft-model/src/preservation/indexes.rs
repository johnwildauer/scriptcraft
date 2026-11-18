use super::*;
use crate::StableId;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Default)]
pub(super) struct SourceIdentityUseIndex {
    paragraph_identity_counts: BTreeMap<String, usize>,
    paragraph_source_identity_counts: BTreeMap<String, usize>,
    referenced_paragraph_ids: BTreeSet<String>,
    anchored_paragraph_ids: BTreeSet<String>,
    externally_used_paragraph_ids: BTreeSet<String>,
    source_id_values: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct SubstringAutomatonNode {
    transitions: BTreeMap<u8, usize>,
    failure: usize,
    output_link: Option<usize>,
    outputs: Vec<usize>,
}

#[derive(Debug, Default)]
struct SubstringAutomaton {
    patterns: Vec<String>,
    nodes: Vec<SubstringAutomatonNode>,
}

impl SubstringAutomaton {
    fn new(patterns: Vec<String>) -> Self {
        let mut automaton = Self {
            patterns,
            nodes: vec![SubstringAutomatonNode::default()],
        };
        for (pattern_index, pattern) in automaton.patterns.iter().enumerate() {
            let mut node_index = 0;
            for byte in pattern.bytes() {
                let next = if let Some(next) = automaton.nodes[node_index].transitions.get(&byte) {
                    *next
                } else {
                    let next = automaton.nodes.len();
                    automaton.nodes.push(SubstringAutomatonNode::default());
                    automaton.nodes[node_index].transitions.insert(byte, next);
                    next
                };
                node_index = next;
            }
            automaton.nodes[node_index].outputs.push(pattern_index);
        }

        let mut pending = std::collections::VecDeque::new();
        let root_children = automaton.nodes[0]
            .transitions
            .values()
            .copied()
            .collect::<Vec<_>>();
        pending.extend(root_children);
        while let Some(node_index) = pending.pop_front() {
            let edges = automaton.nodes[node_index]
                .transitions
                .iter()
                .map(|(byte, child)| (*byte, *child))
                .collect::<Vec<_>>();
            for (byte, child) in edges {
                let mut failure = automaton.nodes[node_index].failure;
                while failure != 0 && !automaton.nodes[failure].transitions.contains_key(&byte) {
                    failure = automaton.nodes[failure].failure;
                }
                let fallback = automaton.nodes[failure]
                    .transitions
                    .get(&byte)
                    .copied()
                    .unwrap_or(0);
                automaton.nodes[child].failure = fallback;
                automaton.nodes[child].output_link = if automaton.nodes[fallback].outputs.is_empty()
                {
                    automaton.nodes[fallback].output_link
                } else {
                    Some(fallback)
                };
                pending.push_back(child);
            }
        }
        automaton
    }

    fn scan(&self, text: &str, skip_exact: Option<&str>, hits: &mut BTreeSet<String>) {
        if self.patterns.is_empty() {
            return;
        }
        let mut state = 0;
        for byte in text.bytes() {
            while state != 0 && !self.nodes[state].transitions.contains_key(&byte) {
                state = self.nodes[state].failure;
            }
            state = self.nodes[state]
                .transitions
                .get(&byte)
                .copied()
                .unwrap_or(0);
            let mut output_state = Some(state);
            while let Some(output) = output_state {
                for pattern_index in &self.nodes[output].outputs {
                    let pattern = &self.patterns[*pattern_index];
                    if skip_exact != Some(pattern.as_str()) {
                        hits.insert(pattern.clone());
                    }
                }
                output_state = self.nodes[output].output_link;
            }
        }
    }
}

impl SourceIdentityUseIndex {
    fn build(
        sections: &[PreservedSection],
        identities: &[ForeignIdentity],
        references: &[ForeignReference],
        anchors: &[SourceAnchor],
        board_references: &[ForeignReference],
        managed_index: &mut BTreeMap<StableId, Vec<ManagedParagraphLocation>>,
        text_bytes: &mut usize,
    ) -> (Self, u64) {
        let mut index = Self::default();
        for identity in identities {
            if identity.domain == ForeignIdDomain::Paragraph {
                *index
                    .paragraph_identity_counts
                    .entry(identity.value.clone())
                    .or_default() += 1;
            }
        }
        for reference in references.iter().chain(board_references) {
            if reference.source.domain == ForeignIdDomain::Paragraph {
                index
                    .referenced_paragraph_ids
                    .insert(reference.source.value.clone());
            }
            if reference.target.domain == ForeignIdDomain::Paragraph {
                index
                    .referenced_paragraph_ids
                    .insert(reference.target.value.clone());
            }
        }
        for anchor in anchors {
            if let AnchorTarget::Foreign(identity) = &anchor.target {
                if identity.domain == ForeignIdDomain::Paragraph {
                    index.anchored_paragraph_ids.insert(identity.value.clone());
                }
            }
        }

        let patterns = index
            .paragraph_identity_counts
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let automaton = SubstringAutomaton::new(patterns);
        let mut visited = 0u64;
        fn reserve_payload_ids(element: &PreservedElement, index: &mut SourceIdentityUseIndex) {
            for attribute in &element.attributes {
                if attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
                {
                    index.source_id_values.insert(attribute.value.clone());
                }
            }
            for child in &element.children {
                if let PreservedNode::Element(child) = child {
                    reserve_payload_ids(child, index);
                }
            }
        }
        // The recursive collector threads bounded counters and path state down
        // the source tree; a context object would add indirection to this walk.
        #[allow(clippy::too_many_arguments)]
        fn collect(
            nodes: &[PreservedNode],
            section_index: usize,
            path: &mut Vec<usize>,
            parent: Option<&PreservedElement>,
            grandparent: Option<&PreservedElement>,
            automaton: &SubstringAutomaton,
            index: &mut SourceIdentityUseIndex,
            managed_index: &mut BTreeMap<StableId, Vec<ManagedParagraphLocation>>,
            visited: &mut u64,
            text_bytes: &mut usize,
        ) {
            for (node_index, node) in nodes.iter().enumerate() {
                *visited = visited.saturating_add(1);
                match node {
                    PreservedNode::Element(element) => {
                        path.push(node_index);
                        if parent.is_some_and(|owner| {
                            owner.name.local_name == "Paragraph"
                                && is_user_paragraph_data_name(&element.name, &owner.name)
                        }) {
                            // Payload IDs still reserve lexical source values for
                            // collision-safe allocation, but have no typed identity
                            // or reference meaning in ScriptCraft.
                            reserve_payload_ids(element, index);
                            path.pop();
                            continue;
                        }
                        let exact_alternate_copy = element.name.local_name == "Paragraph"
                            && parent.is_some_and(|parent| {
                                parent.name.local_name == "Alt"
                                    && parent.name.namespace_uri == element.name.namespace_uri
                            })
                            && grandparent.is_some_and(|grandparent| {
                                grandparent.name.local_name == "AltCollection"
                                    && grandparent.name.namespace_uri == element.name.namespace_uri
                            });
                        if let SourceOwnership::Managed(id) = &element.ownership {
                            managed_index.entry(id.clone()).or_default().push(
                                ManagedParagraphLocation {
                                    section_index,
                                    node_path: path.clone(),
                                },
                            );
                        }
                        let mut unqualified_ids = element.attributes.iter().filter(|attribute| {
                            attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                        });
                        let unique_id = unqualified_ids
                            .next()
                            .map(|attribute| attribute.value.as_str())
                            .filter(|_| unqualified_ids.next().is_none());
                        if element.name.local_name == "Paragraph" && !exact_alternate_copy {
                            if let Some(value) = unique_id {
                                *index
                                    .paragraph_source_identity_counts
                                    .entry(value.to_owned())
                                    .or_default() += 1;
                            }
                        }
                        for attribute in &element.attributes {
                            if attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                            {
                                index.source_id_values.insert(attribute.value.clone());
                            }
                            let exempt = ((exact_alternate_copy
                                || (element.name.local_name == "Paragraph"
                                    && matches!(&element.ownership, SourceOwnership::Managed(_))))
                                && attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id"))
                            .then_some(unique_id)
                            .flatten()
                            .filter(|value| *value == attribute.value.as_str());
                            automaton.scan(
                                &attribute.value,
                                exempt,
                                &mut index.externally_used_paragraph_ids,
                            );
                        }
                        collect(
                            &element.children,
                            section_index,
                            path,
                            Some(element),
                            parent,
                            automaton,
                            index,
                            managed_index,
                            visited,
                            text_bytes,
                        );
                        path.pop();
                    }
                    PreservedNode::Text(text)
                    | PreservedNode::CData(text)
                    | PreservedNode::Comment(text) => {
                        *text_bytes = text_bytes.saturating_add(text.len());
                        automaton.scan(text, None, &mut index.externally_used_paragraph_ids);
                    }
                    PreservedNode::ProcessingInstruction { target, data } => {
                        *text_bytes = text_bytes.saturating_add(data.len());
                        automaton.scan(target, None, &mut index.externally_used_paragraph_ids);
                        automaton.scan(data, None, &mut index.externally_used_paragraph_ids);
                    }
                }
            }
        }
        for (section_index, section) in sections.iter().enumerate() {
            collect(
                &section.children,
                section_index,
                &mut Vec::new(),
                None,
                None,
                &automaton,
                &mut index,
                managed_index,
                &mut visited,
                text_bytes,
            );
        }
        (index, visited)
    }

    fn paragraph_is_safely_removable(&self, id: &str) -> bool {
        self.paragraph_identity_counts.get(id) == Some(&1)
            && !self.referenced_paragraph_ids.contains(id)
            && !self.anchored_paragraph_ids.contains(id)
            && !self.externally_used_paragraph_ids.contains(id)
    }
}

/// Collects only managed source locations. Recognized user-payload subtrees are
/// skipped exactly as in the complete identity-use walk, so opaque descendants
/// cannot acquire managed meaning merely from their stored ownership value.
fn collect_managed_paragraph_locations(
    nodes: &[PreservedNode],
    section_index: usize,
    parent: Option<&PreservedElement>,
    path: &mut Vec<usize>,
    managed_index: &mut BTreeMap<StableId, Vec<ManagedParagraphLocation>>,
    visited: &mut u64,
) {
    for (node_index, node) in nodes.iter().enumerate() {
        *visited = visited.saturating_add(1);
        let PreservedNode::Element(element) = node else {
            continue;
        };
        path.push(node_index);
        if parent.is_some_and(|owner| {
            owner.name.local_name == "Paragraph"
                && is_user_paragraph_data_name(&element.name, &owner.name)
        }) {
            path.pop();
            continue;
        }
        if let SourceOwnership::Managed(id) = &element.ownership {
            managed_index
                .entry(id.clone())
                .or_default()
                .push(ManagedParagraphLocation {
                    section_index,
                    node_path: path.clone(),
                });
        }
        collect_managed_paragraph_locations(
            &element.children,
            section_index,
            Some(element),
            path,
            managed_index,
            visited,
        );
        path.pop();
    }
}

impl PreservationState {
/// Reconstructs all transient preservation indexes after direct edits to
    /// the publicly exposed source tree or identity/reference collections.
    /// Managed editing APIs maintain paragraph locations transactionally and
    /// rebuild identity uses only when an identity-sensitive consumer needs them.
    pub fn rebuild_indexes(&mut self) {
        self.rebuild_source_identity_use_index();
    }

/// Rebuilds only the transient source-tree location map from the
    /// authoritative ordered preservation tree. Source text and attributes are
    /// not scanned for identity uses, and the prior byte total remains the
    /// baseline for scoped single-paragraph delta validation. Any such explicit
    /// rebuild invalidates identity-use freshness because direct tree edits may
    /// have changed source text, IDs, or references.
    pub fn rebuild_managed_paragraph_index(&mut self) {
        let mut managed_index = BTreeMap::new();
        let mut visited = 0u64;
        for (section_index, section) in self.sections.iter().enumerate() {
            collect_managed_paragraph_locations(
                &section.children,
                section_index,
                None,
                &mut Vec::new(),
                &mut managed_index,
                &mut visited,
            );
        }
        self.managed_paragraph_index = Some(managed_index);
        self.invalidate_source_identity_use_index();
    }

/// Rebuilds both transient indexes for an identity-sensitive consumer. The
    /// complete source walk retains the established managed-location behavior,
    /// raw-ID reservation, lexical-use detection, and source byte accounting.
    fn rebuild_source_identity_use_index(&mut self) {
        let mut managed_index = BTreeMap::new();
        let mut text_bytes = 0usize;
        let (source_index, visited) = SourceIdentityUseIndex::build(
            &self.sections,
            &self.identities,
            &self.references,
            &self.anchors,
            &self.board_graph.references,
            &mut managed_index,
            &mut text_bytes,
        );
        self.managed_paragraph_index = Some(managed_index);
        self.source_text_bytes = text_bytes;
        self.source_identity_use_index = Some(source_index);
    }

pub(super) fn invalidate_source_identity_use_index(&mut self) {
        self.source_identity_use_index = None;
    }

    pub fn ensure_source_identity_use_index(&mut self) {
        if self.source_identity_use_index.is_none() {
            self.rebuild_source_identity_use_index();
        }
    }

/// Answers a removal-safety query only from a current complete source-use
    /// index. Mutable managed-source access invalidates that index, so this
    /// method never interprets absence as permission to remove an identity.
    pub fn paragraph_identity_safely_removable(&mut self, id: &str) -> bool {
        self.ensure_source_identity_use_index();
        self.source_identity_use_index
            .as_ref()
            .expect("identity-use ensure installs a current index")
            .paragraph_is_safely_removable(id)
    }

/// Returns raw source IDs only after refreshing collision-reservation state.
    /// An absent index is never equivalent to an empty reservation set.
    pub fn indexed_source_id_values(&mut self) -> BTreeSet<String> {
        self.ensure_source_identity_use_index();
        self.source_identity_use_index
            .as_ref()
            .expect("identity-use ensure installs a current index")
            .source_id_values
            .clone()
    }

    pub fn ensure_managed_paragraph_index(&mut self) {
        if self.managed_paragraph_index.is_none() {
            self.rebuild_managed_paragraph_index();
        }
    }
}
