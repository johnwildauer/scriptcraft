#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{
    export_pdf_with_layout_inputs_and_options, export_text_with_layout_inputs, PdfExportOptions,
};
use scriptcraft_core::fdx_storage::{open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem};
use scriptcraft_core::formatting::SourceLayoutSettings;
use scriptcraft_core::layout::{LayoutComposition, ScreenplayLayout};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{LayoutInputs, Project, Role};

const BARBIE: &[u8] = include_bytes!("../../../.reference/Barbie (2023).fdx");
const DUAL_DIALOGUE_EXAMPLE: &[u8] =
    include_bytes!("../../../.reference/Dual Dialogue Example.fdx");

fn topology(project: &Project) -> Vec<(String, Vec<(Role, Option<String>)>)> {
    project.dual_dialogue_groups.iter().map(|group| {
        let members = group.member_ids().filter_map(|id| {
            project.elements.iter().find(|element| &element.id == id).map(|element| {
                (element.role, element.source_provenance.as_ref().and_then(|source| source.foreign_id.clone()))
            })
        }).collect();
        (group.wrapper.outer_foreign_id.clone().unwrap_or_default(), members)
    }).collect()
}

fn collect_elements<'a>(nodes: &'a [PreservedNode], output: &mut Vec<&'a PreservedElement>) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            output.push(element);
            collect_elements(&element.children, output);
        }
    }
}

fn attr<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.local_name == name)
        .map(|attribute| attribute.value.as_str())
}

fn direct_alts(element: &PreservedElement) -> Vec<&PreservedElement> {
    element.children.iter().filter_map(|node| match node {
        PreservedNode::Element(child) if child.name.local_name == "Alts" => Some(child),
        _ => None,
    }).collect()
}

fn alt_collection(project: &Project) -> Option<PreservedElement> {
    let source = project.preservation.as_ref()?;
    let mut elements = Vec::new();
    collect_elements(&source.sections[0].children, &mut elements);
    elements.into_iter().find(|element| element.name.local_name == "AltCollection").cloned()
}

fn source_owner<'a>(project: &'a Project, id: &str) -> Option<&'a PreservedElement> {
    let source = project.preservation.as_ref()?;
    let mut elements = Vec::new();
    collect_elements(&source.sections[0].children, &mut elements);
    elements.into_iter().find(|element| {
        matches!(element.ownership, SourceOwnership::Managed(_)) && attr(element, "id") == Some(id)
    })
}

fn reference_save_round_trip(bytes: &[u8], name: &str, expected_counts: (usize, usize, usize)) {
    let imported = read_v6(bytes);
    assert!(imported.is_ok(), "the unmodified reference imports before temporary saves");
    let Some(imported) = imported.ok() else { return };
    assert_eq!(
        (imported.project.elements.len(), imported.project.title_page.len(), imported.project.dual_dialogue_groups.len()),
        expected_counts,
        "the unmodified reference has its documented semantic counts",
    );
    let expected_elements = imported.project.elements.len();
    let expected_title_page = imported.project.title_page.len();
    let expected_topology = topology(&imported.project);
    let expected_collection = alt_collection(&imported.project);
    let session_result = EditingSession::new(imported.project);
    assert!(session_result.is_ok());
    let Ok(mut session) = session_result else { return };
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos()).unwrap_or_default();
    let directory = std::env::temp_dir().join(format!("scriptcraft-fdx-wo003-{name}-{nonce}"));
    let created = std::fs::create_dir(&directory);
    assert!(created.is_ok());
    if created.is_err() { return; }
    let first_path = directory.join("save.fdx");
    let second_path = directory.join("save-as.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    let save_as = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(first_path.clone()),
    );
    assert!(save_as.is_ok(), "unmodified reference Save As passes candidate and re-read gates");
    if save_as.is_err() {
        let _ = std::fs::remove_dir_all(&directory);
        return;
    }
    assert!(save_session(&StdFdxStorageFileSystem, &mut session, &mut target, FdxSaveMode::Save).is_ok());
    let first_reopen = open(&StdFdxStorageFileSystem, &first_path);
    assert!(first_reopen.is_ok());
    if let Ok(first_reopen) = first_reopen {
        assert_eq!(first_reopen.document.project.elements.len(), expected_elements);
        assert_eq!(first_reopen.document.project.title_page.len(), expected_title_page);
        assert_eq!(first_reopen.document.project.dual_dialogue_groups.len(), expected_counts.2);
        assert_eq!(topology(&first_reopen.document.project), expected_topology);
        assert_eq!(alt_collection(&first_reopen.document.project), expected_collection);
    }
    let second_save = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(second_path.clone()),
    );
    assert!(second_save.is_ok(), "unmodified reference Save As preserves the source tree without text assertions");
    let second_reopen = open(&StdFdxStorageFileSystem, &second_path);
    assert!(second_reopen.is_ok());
    if let Ok(second_reopen) = second_reopen {
        assert_eq!(second_reopen.document.project.elements.len(), expected_elements);
        assert_eq!(second_reopen.document.project.title_page.len(), expected_title_page);
        assert_eq!(second_reopen.document.project.dual_dialogue_groups.len(), expected_counts.2);
        assert_eq!(topology(&second_reopen.document.project), expected_topology);
        assert_eq!(alt_collection(&second_reopen.document.project), expected_collection);
    }
    let _ = std::fs::remove_dir_all(&directory);
}

fn known_empty_topologies() -> Vec<(String, Vec<(Role, String)>)> {
    vec![
        ("920111c4-0429-44ae-9337-348f8e14620f".to_owned(), vec![
            (Role::Character, "ecdd5d1c-0246-42f3-9731-39aa834b3945".to_owned()),
            (Role::Character, "cbe3c8aa-eeea-4b56-82fb-332bc5ba93f9".to_owned()),
        ]),
        ("8bdc2c71-5bad-40e2-b0fd-4200c0b210ec".to_owned(), vec![
            (Role::Character, "9268757d-5602-454d-9356-a878b10d5da8".to_owned()),
            (Role::Character, "68cb0c54-3a40-482f-9c92-3d4d66273ebf".to_owned()),
        ]),
        ("70b76e63-de7a-4516-aa77-8c2c5f7e37d5".to_owned(), vec![
            (Role::Character, "be45d5f2-a015-40e7-9017-df87043df395".to_owned()),
            (Role::Character, "c77461da-e39e-4ecf-ad79-8037efd3984a".to_owned()),
        ]),
        ("42b82e0a-80e7-4878-ab14-0db76c99929f".to_owned(), vec![
            (Role::Character, "912d04ad-9a93-4519-9f65-25f0b436b15a".to_owned()),
            (Role::Character, "1853656d-070b-40c1-a7d7-bb7a7c4d263a".to_owned()),
            (Role::Dialogue, "d50cccf7-2b5b-4b36-8fa7-5ad28c96ce84".to_owned()),
        ]),
        ("e6656a0d-7d0e-445e-b516-b34902e05423".to_owned(), vec![
            (Role::Character, "08205a70-29b7-4ffc-b5d4-040fc5363f52".to_owned()),
            (Role::Character, "bc7da39e-6b42-4829-8927-40748055806b".to_owned()),
        ]),
        ("3eeacbdd-0545-4a74-a747-392895ea88a4".to_owned(), vec![
            (Role::Character, "4077ee2c-e21e-4ecf-935c-0e6eb20a4091".to_owned()),
            (Role::Character, "9df17231-604a-4866-8c70-9ee22736790d".to_owned()),
        ]),
        ("6bdf6134-af9c-4728-b290-d60fcc17edda".to_owned(), vec![
            (Role::Character, "3529358a-3dd2-4072-b4e8-39d119cab2ab".to_owned()),
            (Role::Character, "2c6d4ecf-0919-4e75-b091-10f5946488f6".to_owned()),
        ]),
    ]
}

#[test]
fn unmodified_barbie_reference_imports_all_empty_and_prior_valid_groups_and_round_trips_identity() {
    let imported = read_v6(BARBIE);
    assert!(imported.is_ok(), "the unmodified Barbie reference must import");
    let Some(imported) = imported.ok() else { return };
    let project = &imported.project;
    assert_eq!(project.elements.len(), 2_459);
    assert_eq!(project.title_page.len(), 24);
    assert_eq!(project.dual_dialogue_groups.len(), 20);
    assert!(!project.elements.iter().any(|element| element.role == Role::General));

    let empty_left = project.dual_dialogue_groups.iter().filter(|group| group.left.paragraphs.is_empty()).count();
    let empty_right = project.dual_dialogue_groups.iter().filter(|group| group.right.paragraphs.is_empty()).count();
    let cc = project.dual_dialogue_groups.iter().filter(|group| {
        group.left.paragraphs.is_empty()
            && group.right.paragraphs.is_empty()
            && group.member_ids().all(|id| project.elements.iter().find(|item| &item.id == id).is_some_and(|item| item.role == Role::Character))
    }).count();
    let ccd = project.dual_dialogue_groups.iter().filter(|group| {
        group.left.paragraphs.is_empty()
            && group.right.paragraphs.len() == 1
            && group.member_ids().map(|id| project.elements.iter().find(|item| &item.id == id).map(|item| item.role)).collect::<Vec<_>>()
                == vec![Some(Role::Character), Some(Role::Character), Some(Role::Dialogue)]
    }).count();
    let prior_valid = project.dual_dialogue_groups.iter().filter(|group| {
        !group.left.paragraphs.is_empty() && !group.right.paragraphs.is_empty()
    }).count();
    assert_eq!((cc, ccd, prior_valid), (6, 1, 13));
    assert_eq!((empty_left, empty_right), (7, 6));

    let actual_known = topology(project).into_iter().filter_map(|(outer, members)| {
        let is_empty_form = members.len() == 2 && members.iter().all(|(role, _)| *role == Role::Character)
            || members.len() == 3 && members.iter().map(|(role, _)| *role).collect::<Vec<_>>()
                == vec![Role::Character, Role::Character, Role::Dialogue];
        is_empty_form.then(|| (outer, members.into_iter().map(|(role, id)| (role, id.unwrap_or_default())).collect()))
    }).collect::<Vec<_>>();
    assert_eq!(actual_known, known_empty_topologies());

    let source_topology = topology(project);
    let Some(source) = project.preservation.as_ref() else { return };
    let mut elements = Vec::new();
    collect_elements(&source.sections[0].children, &mut elements);
    assert_eq!(elements.iter().filter(|element| matches!(element.ownership, SourceOwnership::DualDialogueWrapper(_))).count(), 20);
    for group in &project.dual_dialogue_groups {
        assert!(elements.iter().any(|element| {
            element.ownership == SourceOwnership::DualDialogueWrapper(group.id.clone())
                && element.attributes.iter().any(|attribute| attribute.name.local_name == "id"
                    && Some(attribute.value.as_str()) == group.wrapper.outer_foreign_id.as_deref())
        }));
    }

    let serialized = write_v6(project);
    assert!(serialized.is_ok(), "the source-backed writer must retain strict identity and source-tree checks");
    let Some(serialized) = serialized.ok() else { return };
    let reopened = read_v6(&serialized);
    assert!(reopened.is_ok(), "the serialized reference must re-read");
    let Some(reopened) = reopened.ok() else { return };
    assert_eq!(reopened.project.elements.len(), 2_459);
    assert_eq!(reopened.project.title_page.len(), 24);
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 20);
    assert_eq!(topology(&reopened.project), source_topology);
    for before in &project.dual_dialogue_groups {
        let mapped = scriptcraft_core::dual_dialogue::map_group_identity(
            project,
            &reopened.project,
            &before.id,
        );
        let expected = reopened.project.dual_dialogue_groups.iter().find(|after| {
            after.wrapper.outer_foreign_id == before.wrapper.outer_foreign_id
        }).map(|after| after.id.clone());
        assert_eq!(mapped, expected);
    }
    let Some(reopened_source) = reopened.project.preservation.as_ref() else { return };
    let mut reopened_elements = Vec::new();
    collect_elements(&reopened_source.sections[0].children, &mut reopened_elements);
    assert_eq!(reopened_elements.iter().filter(|element| matches!(element.ownership, SourceOwnership::DualDialogueWrapper(_))).count(), 20);
}

#[test]
fn unmodified_dual_dialogue_example_preserves_inline_alts_and_original_collection_identities() {
    let imported = read_v6(DUAL_DIALOGUE_EXAMPLE);
    assert!(imported.is_ok(), "the unmodified Dual Dialogue Example must import");
    let Some(imported) = imported.ok() else { return };
    let project = &imported.project;
    assert_eq!(project.elements.len(), 12);
    assert_eq!(project.title_page.len(), 54);
    assert_eq!(project.dual_dialogue_groups.len(), 1);

    let Some(source) = project.preservation.as_ref() else { return };
    let mut elements = Vec::new();
    collect_elements(&source.sections[0].children, &mut elements);
    let ordinary = elements.iter().copied().find(|element| {
        matches!(element.ownership, SourceOwnership::Managed(_))
            && attr(element, "id") == Some("4268d64c-39c4-4c54-a30e-bd183965a9aa")
    });
    assert!(ordinary.is_some());
    let Some(ordinary) = ordinary else { return };
    assert_eq!(direct_alts(ordinary).len(), 1);
    let ordinary_element = project.elements.iter().find(|element| {
        element.source_provenance.as_ref().and_then(|source| source.foreign_id.as_deref())
            == attr(ordinary, "id")
    });
    assert!(ordinary_element.is_some_and(|element| {
        let direct_text = ordinary.children.iter().filter_map(|node| match node {
            PreservedNode::Element(text) if text.name.local_name == "Text" => {
                Some(text.children.iter().filter_map(|child| match child {
                    PreservedNode::Text(value) | PreservedNode::CData(value) => Some(value.as_str()),
                    _ => None,
                }).collect::<String>())
            }
            _ => None,
        }).collect::<String>();
        element.text == direct_text
    }));

    let member = elements.iter().copied().find(|element| {
        matches!(element.ownership, SourceOwnership::Managed(_))
            && attr(element, "id") == Some("a52ddde4-d3f8-4353-b400-c3117422bc59")
    });
    assert!(member.is_some());
    let Some(member) = member else { return };
    assert!(direct_alts(member).is_empty(), "the collection copy is not a member attachment");

    let collection = elements.iter().copied().find(|element| {
        element.name.local_name == "AltCollection"
    });
    assert!(collection.is_some());
    let Some(collection) = collection else { return };
    let alt_entries = collection.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Alt" => Some(element),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(alt_entries.len(), 2);
    let copies = alt_entries.iter().filter_map(|alt| {
        alt.children.iter().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Paragraph" => {
                attr(element, "id")
            }
            _ => None,
        })
    }).collect::<Vec<_>>();
    assert_eq!(copies, vec![
        "a52ddde4-d3f8-4353-b400-c3117422bc59",
        "4268d64c-39c4-4c54-a30e-bd183965a9aa",
    ]);
    let registered = source.identities.iter().filter(|identity| {
        identity.domain == scriptcraft_core::preservation::ForeignIdDomain::Paragraph
    }).map(|identity| identity.value.as_str()).collect::<Vec<_>>();
    assert_eq!(registered.iter().filter(|id| **id == "4268d64c-39c4-4c54-a30e-bd183965a9aa").count(), 1);
    assert_eq!(registered.iter().filter(|id| **id == "a52ddde4-d3f8-4353-b400-c3117422bc59").count(), 1);

    let before_topology = topology(project);
    let output = write_v6(project);
    assert!(output.is_ok(), "the reference must pass direct-write strict gates");
    let Some(output) = output.ok() else { return };
    let reopened = read_v6(&output);
    assert!(reopened.is_ok(), "the direct-written reference must re-read");
    let Some(reopened) = reopened.ok() else { return };
    assert_eq!(reopened.project.elements.len(), 12);
    assert_eq!(reopened.project.title_page.len(), 54);
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 1);
    assert_eq!(topology(&reopened.project), before_topology);
    let Some(reopened_source) = reopened.project.preservation.as_ref() else { return };
    assert_eq!(reopened_source.sections, source.sections);
    let mut output_elements = Vec::new();
    collect_elements(&reopened_source.sections[0].children, &mut output_elements);
    assert_eq!(output_elements.iter().filter(|element| element.name.local_name == "AltCollection").count(), 1);
    let output_member = output_elements.iter().copied().find(|element| {
        matches!(element.ownership, SourceOwnership::Managed(_))
            && attr(element, "id") == Some("a52ddde4-d3f8-4353-b400-c3117422bc59")
    });
    assert!(output_member.is_some_and(|element| direct_alts(element).is_empty()));
}

#[test]
fn ccd_alts_owner_deletion_retains_cc_and_orphaned_collection_through_history_storage_and_reopen() {
    let source = r#"<FinalDraft DocumentType="Script" Version="6">
<Content><Paragraph Type="General" id="ccd-wrapper"><DualDialogue>
<SpellCheckIgnoreLists><IgnoredRanges/><IgnoredWords/></SpellCheckIgnoreLists>
<Paragraph Type="Character" id="ccd-left-cue"><Text>LEFT</Text></Paragraph>
<Paragraph Type="Character" id="ccd-right-cue"><Text>RIGHT</Text></Paragraph>
<Paragraph Type="Dialogue" id="ccd-right-dialogue"><Text>canonical source run</Text><Alts Current="alt-1" vendor-inline="keep"><AltId Id="alt-1"/><Choice><Text>INLINE_ONLY_SENTINEL</Text></Choice></Alts></Paragraph>
</DualDialogue></Paragraph></Content>
<AltCollection vendor-collection="preserve-complete"><Alt Id="alt-1"><Paragraph Type="Dialogue" id="ccd-right-dialogue"><Text>COPY_SENTINEL</Text></Paragraph></Alt><Alt Id="orphan"><Paragraph Type="Dialogue" id="ccd-existing-orphan"><Text>ORPHAN_SENTINEL</Text></Paragraph></Alt></AltCollection>
</FinalDraft>"#;
    let imported = read_v6(source.as_bytes());
    assert!(imported.is_ok(), "the combined C,C,D with inline owner and alternate copy is accepted");
    let Some(imported) = imported.ok() else { return };
    let project = imported.project;
    assert_eq!(project.dual_dialogue_groups.len(), 1);
    assert_eq!(project.elements.len(), 3);
    let group_before = project.dual_dialogue_groups[0].clone();
    assert!(group_before.left.paragraphs.is_empty());
    assert_eq!(group_before.right.paragraphs.len(), 1);
    let left_id = group_before.left.character.clone();
    let right_id = group_before.right.character.clone();
    let owner_id = group_before.right.paragraphs[0].clone();
    let alt_collection_before = alt_collection(&project);
    assert!(alt_collection_before.is_some());
    let Some(alt_collection_before) = alt_collection_before else { return };
    let owner = source_owner(&project, "ccd-right-dialogue");
    assert!(owner.is_some());
    let Some(owner) = owner else { return };
    let original_alts = direct_alts(owner).first().copied().cloned();
    assert!(original_alts.is_some());

    let session_result = EditingSession::new(project.clone());
    assert!(session_result.is_ok());
    let Ok(mut session) = session_result else { return };
    let right_cue_index = project.elements.iter().position(|element| element.id == right_id);
    let dialogue_index = project.elements.iter().position(|element| element.id == owner_id);
    assert!(right_cue_index.is_some() && dialogue_index.is_some());
    let (Some(right_cue_index), Some(dialogue_index)) = (right_cue_index, dialogue_index) else { return };
    assert!(session.set_selection(Selection {
        anchor: Position { element: right_cue_index, offset: session.project().elements[right_cue_index].text.len() },
        focus: Position { element: dialogue_index, offset: session.project().elements[dialogue_index].text.len() },
    }).is_ok());
    assert!(session.delete().is_ok(), "deleting the owner leaves a valid empty right side");
    assert_eq!(session.project().elements.len(), 2);
    assert_eq!(session.project().elements.iter().map(|element| element.id.clone()).collect::<Vec<_>>(), vec![left_id.clone(), right_id.clone()]);
    let retained_group = session.project().dual_dialogue_groups.iter().find(|group| {
        group.left.character == left_id && group.right.character == right_id
    });
    assert!(retained_group.is_some());
    assert!(retained_group.is_some_and(|group| group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()));
    assert!(source_owner(session.project(), "ccd-right-dialogue").is_none(), "the deleted managed owner and inline attachment disappear");
    assert_eq!(alt_collection(session.project()), Some(alt_collection_before.clone()), "AltCollection, copies, and unrelated orphans are untouched");

    let output = write_v6(session.project());
    assert!(output.is_ok(), "the C,C candidate passes full source, identity, and storage serialization gates");
    if let Some(output) = output.ok() {
        let reopened = read_v6(&output);
        assert!(reopened.is_ok());
        if let Some(reopened) = reopened.ok() {
            assert_eq!(reopened.project.elements.iter().map(|element| element.id.clone()).collect::<Vec<_>>(), vec![left_id.clone(), right_id.clone()]);
            assert!(reopened.project.dual_dialogue_groups.iter().any(|group| {
                group.left.character == left_id && group.right.character == right_id
                    && group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()
            }));
            assert!(source_owner(&reopened.project, "ccd-right-dialogue").is_none());
            assert_eq!(alt_collection(&reopened.project), Some(alt_collection_before.clone()));
        }
    }

    let after_delete = session.project().clone();
    let inputs = LayoutInputs::default();
    let text_export = export_text_with_layout_inputs(&after_delete, &inputs);
    assert!(text_export.is_ok());
    let pdf_export = export_pdf_with_layout_inputs_and_options(
        &after_delete,
        &inputs,
        PdfExportOptions { include_title_page: false },
    );
    assert!(pdf_export.is_ok());
    if let (Ok(text_export), Ok(pdf_export)) = (text_export, pdf_export) {
        let pdf_text = String::from_utf8_lossy(&pdf_export);
        for alternate_only in ["INLINE_ONLY_SENTINEL", "COPY_SENTINEL", "ORPHAN_SENTINEL"] {
            assert!(!text_export.text.contains(alternate_only), "plain-text export excludes {alternate_only}");
            assert!(!pdf_text.contains(alternate_only), "PDF export excludes {alternate_only}");
        }
    }
    let print_layout = ScreenplayLayout::build_with_composition(
        &after_delete,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::Print { include_title_page: false },
    );
    assert!(print_layout.is_ok());
    if let Ok(print_layout) = print_layout {
        assert_eq!(
            print_layout.pages.iter().flat_map(|page| &page.lines).map(|line| line.element).collect::<Vec<_>>(),
            [0, 1],
            "post-deletion C,C layout contains only the two real Character cues",
        );
    }
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &project, "Undo restores full original source and semantic group snapshot");
    assert_eq!(source_owner(session.project(), "ccd-right-dialogue")
        .and_then(|element| direct_alts(element).first().copied().cloned()), original_alts);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &after_delete, "Redo restores C,C plus the full orphan-preserving source tree");

    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos()).unwrap_or_default();
    let directory = std::env::temp_dir().join(format!("scriptcraft-ccd-alts-lifecycle-{nonce}"));
    let created = std::fs::create_dir(&directory);
    assert!(created.is_ok());
    if created.is_err() { return; }
    let first_path = directory.join("deleted-owner-save.fdx");
    let second_path = directory.join("deleted-owner-save-as.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    let save_as = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(first_path.clone()),
    );
    assert!(save_as.is_ok(), "Save As accepts the actual C,C owner-deletion candidate");
    if save_as.is_ok() {
        let mapped = scriptcraft_core::dual_dialogue::map_group_identity(
            &after_delete,
            session.project(),
            &after_delete.dual_dialogue_groups[0].id,
        );
        assert_eq!(mapped, session.project().dual_dialogue_groups.first().map(|group| group.id.clone()));
        assert!(save_session(&StdFdxStorageFileSystem, &mut session, &mut target, FdxSaveMode::Save).is_ok());
        let reopened = open(&StdFdxStorageFileSystem, &first_path);
        assert!(reopened.is_ok());
        if let Ok(reopened) = reopened {
            assert_eq!(reopened.document.project.elements.len(), 2);
            assert!(reopened.document.project.dual_dialogue_groups.iter().any(|group| {
                group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()
            }));
            assert!(source_owner(&reopened.document.project, "ccd-right-dialogue").is_none());
            assert_eq!(alt_collection(&reopened.document.project), Some(alt_collection_before.clone()));
        }
        assert!(save_session(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(second_path.clone()),
        ).is_ok());
        let reopened_as = open(&StdFdxStorageFileSystem, &second_path);
        assert!(reopened_as.is_ok());
        if let Ok(reopened_as) = reopened_as {
            assert_eq!(reopened_as.document.project.elements.len(), 2);
            assert!(reopened_as.document.project.dual_dialogue_groups.iter().any(|group| {
                group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()
            }));
            assert_eq!(alt_collection(&reopened_as.document.project), Some(alt_collection_before));
        }
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn unmodified_references_save_save_as_and_reopen_without_screenplay_text_assertions() {
    reference_save_round_trip(BARBIE, "barbie-reference", (2_459, 24, 20));
    reference_save_round_trip(DUAL_DIALOGUE_EXAMPLE, "dual-dialogue-example-reference", (12, 54, 1));
}
