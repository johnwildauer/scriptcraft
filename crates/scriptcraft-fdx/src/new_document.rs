//! Preservation-free serialization of current Project content to an FDX source tree.

use crate::constants::FDX_VERSION;
use crate::content::{add_wrapper_direct_attributes, new_paragraph, set_unqualified_attribute};
use crate::context::{SaveContext, SaveOperation};
use crate::diagnostics::{map_paragraph_id_error, FdxError, SaveDiagnosticCode};
use crate::text_state::text_state_element;
use crate::title_page::{new_title_page_header_and_footer, new_title_page_paragraph};
use crate::tree::preserved_element;
use crate::window_state::window_state_element;
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::preservation::{ForeignIdDomain, PreservedNode};
use scriptcraft_model::{BodyOwner, Project, SemanticCapabilities};
use std::collections::{BTreeMap, BTreeSet};

/// Serialize a complete Project without source-preservation input.
///
/// This generates output from supplied current content; it is not a semantic
/// document constructor or a source of template defaults. The declaration and
/// newline convention are handled by the top-level writer.
pub(crate) fn make_new_document_nodes(
    project: &Project,
    context: &mut SaveContext,
) -> Result<Vec<PreservedNode>, FdxError> {
    let mut root = preserved_element(
        "FinalDraft",
        vec![("DocumentType", "Script"), ("Version", FDX_VERSION)],
    );
    let mut lineage_ids = project_beat_ids(project);
    lineage_ids.extend(project_source_paragraph_ids(project));
    let document_ref_id = context.allocate_lineage_id(&mut lineage_ids)?;
    let xref_id = if matches!(
        context.operation(),
        SaveOperation::Save | SaveOperation::SaveAs
    ) {
        Some(context.allocate_lineage_id(&mut lineage_ids)?)
    } else {
        None
    };
    let mut document_ref = preserved_element(
        "DocumentRef",
        vec![("DateTime", context.timestamp()), ("id", &document_ref_id)],
    );
    if let Some(xref_id) = xref_id {
        let xref_datetime = match context.operation() {
            SaveOperation::DirectSerialization => context.timestamp(),
            SaveOperation::Save | SaveOperation::SaveAs => context
                .session_open_timestamp()
                .ok_or(FdxError::InvalidValue)?,
        };
        document_ref
            .children
            .push(PreservedNode::Element(preserved_element(
                "XRef",
                vec![("DateTime", xref_datetime), ("id", &xref_id)],
            )));
    }
    root.children.push(PreservedNode::Element(document_ref));

    let mut content = preserved_element("Content", Vec::new());
    let mut reserved_paragraph_ids = lineage_ids;
    reserved_paragraph_ids.extend(project_source_paragraph_ids(project));
    let mut paragraph_ids = ParagraphIdAllocator::new(reserved_paragraph_ids);
    let mut alt_owner_paragraph_ids = BTreeMap::new();
    let mut index = 0usize;
    while let Some(element) = project.elements.get(index) {
        if let Some(group) = project
            .dual_dialogue_groups
            .iter()
            .find(|group| group.first_member() == &element.id)
        {
            let mut outer = preserved_element("Paragraph", vec![("Type", "General"), ("id", "")]);
            let outer_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
            set_unqualified_attribute(&mut outer, "id", &outer_id)?;
            if let Some(overrides) = project.dual_dialogue_paragraph_overrides.get(&group.id) {
                add_wrapper_direct_attributes(&mut outer, *overrides)?;
            }
            let mut dual = preserved_element("DualDialogue", Vec::new());
            let mut spell = preserved_element("SpellCheckIgnoreLists", Vec::new());
            spell
                .children
                .push(PreservedNode::Element(preserved_element(
                    "IgnoredRanges",
                    Vec::new(),
                )));
            spell
                .children
                .push(PreservedNode::Element(preserved_element(
                    "IgnoredWords",
                    Vec::new(),
                )));
            dual.children.push(PreservedNode::Element(spell));
            let members = group.member_ids().cloned().collect::<Vec<_>>();
            if members.is_empty() {
                return Err(SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error());
            }
            for (offset, member_id) in members.iter().enumerate() {
                let member = project
                    .elements
                    .get(index.saturating_add(offset))
                    .filter(|candidate| &candidate.id == member_id)
                    .ok_or_else(|| SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error())?;
                let overrides = project.direct_paragraph_overrides.get(member_id).copied();
                let definition = project
                    .element_definitions()
                    .get(&member.element_type())
                    .ok_or(FdxError::InvalidValue)?;
                let mut generated = new_paragraph(
                    member,
                    definition,
                    overrides,
                    &mut paragraph_ids,
                )?;
                if let PreservedNode::Element(paragraph) = &mut generated {
                    crate::dialogue_alts_output::attach_generated_inline(paragraph, member)?;
                    alt_owner_paragraph_ids.insert(
                        member.id.clone(),
                        crate::dialogue_alts_output::generated_paragraph_id(paragraph)?,
                    );
                }
                dual.children.push(generated);
            }
            outer.children.push(PreservedNode::Element(dual));
            content.children.push(PreservedNode::Element(outer));
            index = index
                .checked_add(members.len())
                .ok_or(FdxError::ResourceLimit)?;
            continue;
        }

        let direct_overrides = project.direct_paragraph_overrides.get(&element.id).copied();
        let definition = project
            .element_definitions()
            .get(&element.element_type())
            .ok_or(FdxError::InvalidValue)?;
        let mut paragraph = new_paragraph(
            element,
            definition,
            direct_overrides,
            &mut paragraph_ids,
        )?;
        if let PreservedNode::Element(generated) = &mut paragraph {
            crate::dialogue_alts_output::attach_generated_inline(generated, element)?;
            alt_owner_paragraph_ids.insert(
                element.id.clone(),
                crate::dialogue_alts_output::generated_paragraph_id(generated)?,
            );
        }
        if has_capability(project, element, |capabilities| capabilities.scene_boundary) {
            let scene_end = project.elements[index + 1..]
                .iter()
                .position(|candidate| {
                    has_capability(project, candidate, |capabilities| {
                        capabilities.scene_boundary
                    })
                })
                .map_or(project.elements.len(), |offset| index + 1 + offset);
            let names = scriptcraft_model::scene_metadata::derived_character_projection(
                project,
                &project.elements[index + 1..scene_end],
            )
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
            let PreservedNode::Element(heading) = &mut paragraph else {
                return Err(SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error());
            };
            let properties = scriptcraft_model::scene_metadata::new_scene_properties(
                &names,
                &mut paragraph_ids,
            )
            .map_err(|error| match error {
                scriptcraft_model::scene_metadata::SceneMetadataError::ParagraphIdentityEntropyUnavailable => {
                    FdxError::ParagraphIdentityEntropyUnavailable
                }
                scriptcraft_model::scene_metadata::SceneMetadataError::ParagraphIdentityCollisionLimit => {
                    FdxError::ParagraphIdentityCollisionLimit
                }
                _ => SaveDiagnosticCode::NewDocumentScenePropertiesUnsupported.error(),
            })?;
            let text_index = heading
                .children
                .iter()
                .position(|node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text"))
                .unwrap_or(heading.children.len());
            heading
                .children
                .insert(text_index, PreservedNode::Element(properties));
        }
        content.children.push(paragraph);
        index = index.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    }
    root.children.push(PreservedNode::Element(content));
    let page_layout = crate::settings::page_layout_element(&root.name, project.page_layout())?;
    root.children
        .push(PreservedNode::Element(page_layout));
    root.children
        .push(PreservedNode::Element(window_state_element(
            &root.name,
            context.window_state(),
        )));
    root.children
        .push(PreservedNode::Element(text_state_element(
            &root.name,
            project,
            context.text_state(),
            context.invisibles_state().show_body,
        )?));
    root.children
        .push(PreservedNode::Element(crate::split_state::script_panel_element(
            &root.name,
            context.display_mode(),
        )));
    for definition in project.element_definitions().iter() {
        let settings = crate::element_settings_serialization::serialize_element_settings(definition, &root)?;
        root.children.push(PreservedNode::Element(settings));
    }

    let mut title_page = preserved_element("TitlePage", Vec::new());
    title_page
        .children
        .push(PreservedNode::Element(new_title_page_header_and_footer(
            &mut paragraph_ids,
        )?));
    let mut title_content = preserved_element("Content", Vec::new());
    for element in &project.title_page {
        let definition = match element.element_type() {
            Some(key) => Some(
                project
                    .element_definitions()
                    .get(&key)
                    .ok_or(FdxError::InvalidValue)?,
            ),
            None => None,
        };
        title_content
            .children
            .push(new_title_page_paragraph(
                element,
                definition,
                &mut paragraph_ids,
            )?);
    }
    title_page
        .children
        .push(PreservedNode::Element(title_content));
    title_page
        .children
        .push(PreservedNode::Element(preserved_element(
            "TextState",
            vec![
                ("Scaling", "100"),
                ("Selection", "0,0"),
                (
                    "ShowInvisibles",
                    if context.invisibles_state().show_title_page {
                        "Yes"
                    } else {
                        "No"
                    },
                ),
            ],
        )));
    root.children.push(PreservedNode::Element(title_page));
    crate::dialogue_alts_output::append_generated_collection(
        &mut root,
        project,
        &alt_owner_paragraph_ids,
    )?;
    crate::beat_board_output::append_generated_board(&mut root, project, context)?;
    crate::script_notes_output::append_generated_script_notes(&mut root, project)?;
    Ok(vec![PreservedNode::Element(root)])
}

fn project_beat_ids(project: &Project) -> BTreeSet<String> {
    let mut reserved = BTreeSet::new();
    if let Some(id) = &project.beat_board().foreign_id { reserved.insert(id.clone()); }
    for card in &project.beat_board().cards {
        reserved.insert(card.foreign_id.clone());
        reserved.extend(card.paragraphs.iter().filter_map(|paragraph| paragraph.foreign_id.clone()));
    }
    reserved
}

fn has_capability(
    project: &Project,
    element: &BodyOwner,
    capability: impl FnOnce(SemanticCapabilities) -> bool,
) -> bool {
    project
        .element_semantics(&element.element_type())
        .is_some_and(|semantics| capability(semantics.capabilities()))
}

fn project_source_paragraph_ids(project: &Project) -> BTreeSet<String> {
    fn collect_source_attributes(nodes: &[PreservedNode], reserved: &mut BTreeSet<String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph" {
                if let Some(id) = element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
                {
                    reserved.insert(id.value.clone());
                }
            }
            collect_source_attributes(&element.children, reserved);
        }
    }

    let mut reserved = BTreeSet::new();
    for provenance in project
        .elements
        .iter()
        .filter_map(|element| element.source_provenance.as_ref())
        .chain(
            project
                .title_page
                .iter()
                .filter_map(|element| element.source_provenance.as_ref()),
        )
    {
        if let Some(id) = &provenance.foreign_id {
            reserved.insert(id.clone());
        }
    }
    reserved.extend(
        project
            .dual_dialogue_groups
            .iter()
            .filter_map(|group| group.wrapper.outer_foreign_id.clone()),
    );
    if let Some(preservation) = &project.preservation {
        reserved.extend(
            preservation
                .identities
                .iter()
                .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
                .map(|identity| identity.value.clone()),
        );
        for section in preservation.sections.iter() {
            collect_source_attributes(&section.children, &mut reserved);
        }
    }
    reserved
}
