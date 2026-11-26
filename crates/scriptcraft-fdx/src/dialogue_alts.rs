//! Bounded semantic import for supported inline Dialogue alternatives.

use crate::identity::raw_identity_attribute;
use crate::reader::{RawElement, RawNode};
use scriptcraft_model::formatted_text::sparse_alt_run_projection;
use scriptcraft_model::{
    DialogueAltId, DialogueAlternative, DialogueAlternatives, Project,
};

#[derive(Default)]
struct ProjectAltBudget {
    variants: usize,
    text_bytes: usize,
    run_values: usize,
    attribute_bytes: usize,
}

impl ProjectAltBudget {
    fn accepts(&mut self, owner: &scriptcraft_model::ScreenplayElement, alternatives: &DialogueAlternatives) -> bool {
        let mut text_bytes = owner.text.len();
        let mut run_values = owner.runs.len();
        let mut attribute_bytes = 0usize;
        for run in &owner.runs {
            run_values = match run_values.checked_add(run.attributes.len()) { Some(value) => value, None => return false };
            for attribute in &run.attributes {
                attribute_bytes = match attribute_bytes.checked_add(attribute.name.len())
                    .and_then(|value| value.checked_add(attribute.value.len()))
                { Some(value) => value, None => return false };
            }
        }
        for alternative in &alternatives.inactive {
            text_bytes = match text_bytes.checked_add(alternative.text.len()) { Some(value) => value, None => return false };
            run_values = match run_values.checked_add(alternative.runs.len()) { Some(value) => value, None => return false };
            for run in &alternative.runs {
                run_values = match run_values.checked_add(run.attributes.len()) { Some(value) => value, None => return false };
                for attribute in &run.attributes {
                    attribute_bytes = match attribute_bytes.checked_add(attribute.name.len())
                        .and_then(|value| value.checked_add(attribute.value.len()))
                    { Some(value) => value, None => return false };
                }
            }
        }
        let Some(variants) = self.variants.checked_add(alternatives.total_count()) else { return false };
        let Some(text_bytes) = self.text_bytes.checked_add(text_bytes) else { return false };
        let Some(run_values) = self.run_values.checked_add(run_values) else { return false };
        let Some(attribute_bytes) = self.attribute_bytes.checked_add(attribute_bytes) else { return false };
        if variants > scriptcraft_model::dialogue_alts::MAX_DIALOGUE_ALT_VARIANTS_PER_PROJECT
            || text_bytes > scriptcraft_model::dialogue_alts::MAX_DIALOGUE_ALT_TEXT_BYTES_PER_PROJECT
            || run_values > scriptcraft_model::dialogue_alts::MAX_DIALOGUE_ALT_RUN_VALUES_PER_PROJECT
            || attribute_bytes > scriptcraft_model::dialogue_alts::MAX_DIALOGUE_ALT_ATTRIBUTE_BYTES_PER_PROJECT
        { return false; }
        self.variants = variants;
        self.text_bytes = text_bytes;
        self.run_values = run_values;
        self.attribute_bytes = attribute_bytes;
        true
    }
}
fn is_fdx(element: &RawElement, name: &str, namespace: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == namespace
}

fn element_children<'a>(element: &'a RawElement, name: &str, namespace: Option<&str>) -> Vec<&'a RawElement> {
    element.children.iter().filter_map(|child| match child {
        RawNode::Element(child) if is_fdx(child, name, namespace) => Some(child),
        _ => None,
    }).collect()
}

fn only_unqualified_attribute<'a>(element: &'a RawElement, name: &str) -> Option<&'a str> {
    let mut values = element.attributes.iter().filter(|candidate| {
        candidate.name.namespace_uri.is_none() && candidate.name.local_name == name
    });
    let value = values.next()?.value.as_str();
    values.next().is_none().then_some(value)
}

fn text_only(element: &RawElement) -> Option<String> {
    let mut value = String::new();
    for child in &element.children {
        match child {
            RawNode::Text(text) | RawNode::CData(text) => value.push_str(text),
            _ => return None,
        }
    }
    Some(value)
}

fn paragraph_id(element: &RawElement) -> Option<&str> {
    raw_identity_attribute(element)
}

fn collect_managed_body_paragraphs<'a>(root: &'a RawElement) -> Vec<&'a RawElement> {
    let namespace = root.name.namespace_uri.as_deref();
    let content = element_children(root, "Content", namespace).into_iter().next();
    let Some(content) = content else { return Vec::new() };
    let mut paragraphs = Vec::new();
    for child in &content.children {
        let RawNode::Element(element) = child else { continue };
        if is_fdx(element, "Paragraph", namespace)
            && crate::content::is_managed_direct_body_paragraph(element, namespace).unwrap_or(false)
        {
            let duals = element_children(element, "DualDialogue", namespace);
            if duals.is_empty() {
                paragraphs.push(element);
            } else if duals.len() == 1 {
                for member in element_children(duals[0], "Paragraph", namespace) {
                    paragraphs.push(member);
                }
            }
        }
    }
    paragraphs
}

fn import_owner_alts(
    owner: &RawElement,
    collection: Option<&RawElement>,
    namespace: Option<&str>,
    definition: &scriptcraft_model::ElementDefinition,
) -> Option<DialogueAlternatives> {
    let alts = element_children(owner, "Alts", namespace);
    let [alts] = alts.as_slice() else { return None };
    let current = DialogueAltId::parse_decimal(only_unqualified_attribute(alts, "Current")?)?;
    let mut ids = Vec::new();
    for child in &alts.children {
        match child {
            RawNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            RawNode::Element(alt_id) if is_fdx(alt_id, "AltId", namespace) => {
                if !alt_id.attributes.is_empty() { return None; }
                if ids.len().checked_add(2)?
                    > scriptcraft_model::dialogue_alts::MAX_DIALOGUE_ALT_VARIANTS_PER_OWNER
                { return None; }
                ids.push(DialogueAltId::parse_decimal(&text_only(alt_id)?)?);
            }
            _ => return None,
        }
    }
    let mut inactive = Vec::with_capacity(ids.len());
    for id in ids {
        let collection = collection?;
        let owner_paragraph_id = paragraph_id(owner)?;
        let mut matches = Vec::new();
        for alt in element_children(collection, "Alt", namespace) {
            let has_numeric_id = alt.attributes.iter().any(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name == "Id"
                    && DialogueAltId::parse_decimal(&attribute.value) == Some(id)
            });
            if !has_numeric_id { continue; }
            let paragraphs = element_children(alt, "Paragraph", namespace);
            if paragraphs.iter().any(|paragraph| paragraph_id(paragraph) == Some(owner_paragraph_id)) {
                matches.push((alt, paragraphs));
            }
        }
        let [(alt, paragraphs)] = matches.as_slice() else { return None };
        if DialogueAltId::parse_decimal(only_unqualified_attribute(alt, "Id")?) != Some(id) {
            return None;
        }
        let [paragraph] = paragraphs.as_slice() else { return None };
        for child in &alt.children {
            match child {
                RawNode::Text(text) if text.chars().all(char::is_whitespace) => {}
                RawNode::Element(candidate) if is_fdx(candidate, "Paragraph", namespace) => {}
                _ => return None,
            }
        }
        for child in &paragraph.children {
            match child {
                RawNode::Text(text) if text.chars().all(char::is_whitespace) => {}
                RawNode::Element(text) if is_fdx(text, "Text", namespace)
                    && text.attributes.iter().all(|attribute| attribute.name.namespace_uri.is_none())
                    && text.children.iter().all(|node| matches!(node, RawNode::Text(_) | RawNode::CData(_))) => {}
                _ => return None,
            }
        }
        let runs = crate::content::semantic_text_runs(paragraph, false, false).ok()?;
        let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
        let runs = sparse_alt_run_projection(definition, &text, &runs).ok()?;
        inactive.push(DialogueAlternative::new(id, text, runs));
    }
    DialogueAlternatives::new(current, inactive).ok()
}

/// Installs semantic alternatives only for a complete, unambiguous supported
/// source projection. Unsupported shapes are retained as opaque source.
pub(crate) fn import(root: &RawElement, project: &mut Project) {
    let namespace = root.name.namespace_uri.as_deref();
    let collection = element_children(root, "AltCollection", namespace).into_iter().next();
    let mut imported = Vec::new();
    let mut budget = ProjectAltBudget::default();
    for (index, raw_owner) in collect_managed_body_paragraphs(root).into_iter().enumerate() {
        let Some(owner) = project.elements.get(index) else { continue };
        let element = owner.as_element();
        let source_id = element.source_provenance.as_ref().and_then(|source| source.foreign_id.as_deref());
        if source_id != paragraph_id(raw_owner) { continue; }
        let Some(definition) = project.element_definitions().get(&element.element_type()) else { continue };
        if !definition.capabilities().dialogue { continue; }
        let Some(alternatives) = import_owner_alts(raw_owner, collection, namespace, definition) else { continue };
        if !budget.accepts(element, &alternatives) { continue; }
        imported.push((index, alternatives));
    }
    for (index, alternatives) in imported {
        let Some(owner) = project.elements.get_mut(index) else { continue };
        scriptcraft_model::BodyOwner::update(owner, |element| {
            element.dialogue_alternatives = Some(alternatives);
        });
    }
}
