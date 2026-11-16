#![forbid(unsafe_code)]

use scriptcraft_core::cast_list::{correct_cast_lists, correct_cast_lists_with_outcome};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::fdx::{import_fdx, read_v6, write_v6, FdxImportOutcome, FdxMigrationConsent};
use scriptcraft_core::fdx_storage::{
    save_session, FdxSaveMode, FdxSaveTarget, FdxStorageError, FdxStorageErrorKind,
    FdxStorageFileSystem, StdFdxStorageFileSystem, StorageOperation, TransactionalSaveError,
};
use scriptcraft_core::preservation::{
    DependencyConfidence, PreservedAttribute, PreservedElement, PreservedNode, SectionKind,
    SourceName, SourceOwnership,
};
use scriptcraft_core::{
    project_validation_diagnostics, reset_project_validation_diagnostics, Project, Role,
};
use std::fs;
use std::path::{Path, PathBuf};

const SYNTHETIC: &[u8] = include_bytes!("fixtures/user_paragraph_data.fdx");
const REFERENCE_V2: &[u8] = include_bytes!("../../../.reference/The Once and Future King.fdx");
const SCENE_SENTINEL: &str = "SCENE_PAYLOAD_SENTINEL";
const CAST_SENTINEL: &str = "CAST_PAYLOAD_SENTINEL";
const ARC_SENTINEL: &str = "ARC_PAYLOAD_SENTINEL";

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("integration scenario uses bounded supported input"))
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-user-paragraph-data-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path)
            .unwrap_or_else(|_| unreachable!("unique integration directory can be created"));
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct FailReplacement;

impl FdxStorageFileSystem for FailReplacement {
    type TempFile = fs::File;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        StdFdxStorageFileSystem.read_bounded(path, limit)
    }

    fn inspect_destination(
        &self,
        path: &Path,
    ) -> Result<Option<Self::Permissions>, FdxStorageError> {
        StdFdxStorageFileSystem.inspect_destination(path)
    }

    fn create_temp(
        &self,
        destination: &Path,
    ) -> Result<(PathBuf, Self::TempFile), FdxStorageError> {
        StdFdxStorageFileSystem.create_temp(destination)
    }

    fn set_temp_permissions(
        &self,
        temp: &mut Self::TempFile,
        permissions: &Self::Permissions,
    ) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.set_temp_permissions(temp, permissions)
    }

    fn sync_temp(&self, temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.sync_temp(temp)
    }

    fn replace(&self, _temp: &Path, _destination: &Path) -> Result<(), FdxStorageError> {
        Err(FdxStorageError {
            kind: FdxStorageErrorKind::Io(StorageOperation::Replace),
            temporary_cleanup_failed: false,
        })
    }

    fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.install_new(temp, destination)
    }

    fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.remove_temp(temp)
    }
}

fn collect_paragraph_payloads(nodes: &[PreservedNode], output: &mut Vec<(String, PreservedElement)>) {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if element.name.local_name == "Paragraph" {
            let id = element
                .attributes
                .iter()
                .find(|attribute| attribute.name.namespace_uri.is_none() && attribute.name.local_name == "id")
                .map(|attribute| attribute.value.clone())
                .unwrap_or_else(|| "missing-owner-id".to_owned());
            for child in &element.children {
                if let PreservedNode::Element(payload) = child {
                    if payload.name.local_name == "UserParagraphData"
                        && payload.name.namespace_uri == element.name.namespace_uri
                    {
                        output.push((id.clone(), payload.clone()));
                    }
                }
            }
        }
        if element.name.local_name != "UserParagraphData" {
            collect_paragraph_payloads(&element.children, output);
        }
    }
}

fn paragraph_payloads(project: &Project) -> Vec<(String, PreservedElement)> {
    let mut payloads = Vec::new();
    if let Some(preservation) = &project.preservation {
        for section in &preservation.sections {
            collect_paragraph_payloads(&section.children, &mut payloads);
        }
    }
    payloads
}

fn section_payload_count(project: &Project, kind: SectionKind) -> usize {
    let Some(preservation) = &project.preservation else {
        return 0;
    };
    let mut payloads = Vec::new();
    for section in &preservation.sections {
        if section.kind == kind {
            collect_paragraph_payloads(&section.children, &mut payloads);
        }
    }
    payloads.len()
}

fn integration_scene_fixture() -> Vec<u8> {
    let source = std::str::from_utf8(SYNTHETIC)
        .unwrap_or_else(|_| unreachable!("checked-in synthetic fixture is UTF-8"));
    let source = source
        .replacen("Type=\"Action\" id=\"body-001\"", "Type=\"Scene Heading\" id=\"body-001\"", 1)
        .replacen("Opening action.", "INT. LIVING ROOM - DAY", 1)
        .replacen("opaque body text", SCENE_SENTINEL, 1);
    let with_cast_list = source.replacen(
        "</DualDialogue>\n    </Paragraph>\n  </Content>",
        "</DualDialogue>\n    </Paragraph>\n    <Paragraph Type=\"Cast List\" id=\"cast-001\"><Text>(old)</Text><UserParagraphData owner=\"cast-list\"><CastPayload>CAST_PAYLOAD_SENTINEL</CastPayload></UserParagraphData></Paragraph>\n  </Content>",
        1,
    );
    with_cast_list.replacen(
        "</UserParagraphData>",
        "</UserParagraphData><VendorSceneSibling xmlns=\"urn:vendor:scene\" flag=\"preserve\"/>",
        1,
    ).into_bytes()
}

fn multiple_scene_fixture(scene_count: usize, unsupported_scene: Option<usize>) -> Vec<u8> {
    let mut source = String::from(
        r#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Cast List" id="unheaded-cast"><Text>(unheaded)</Text></Paragraph>"#,
    );
    for index in 0..scene_count {
        source.push_str(&format!(
            "<Paragraph Type=\"Scene Heading\" id=\"scene-{index}\"><Text>INT. ROOM {index} - DAY</Text>"
        ));
        if unsupported_scene == Some(index) {
            source.push_str(
                "<SceneProperties><VendorSceneData marker=\"preserve\"/></SceneProperties>",
            );
        }
        source.push_str(&format!(
            "</Paragraph><Paragraph Type=\"Character\" id=\"cue-{index}\"><Text>CHARACTER-{index}</Text></Paragraph><Paragraph Type=\"Cast List\" id=\"cast-{index}\"><Text>(stale-{index})</Text></Paragraph>"
        ));
    }
    source.push_str("</Content></FinalDraft>");
    source.into_bytes()
}

fn generated_arc_payload(namespace: &Option<String>, owner_id: &str) -> PreservedElement {
    let vendor_namespace = "urn:scriptcraft:integration-payload".to_owned();
    let payload_name = SourceName {
        namespace_uri: namespace.clone(),
        local_name: "UserParagraphData".to_owned(),
        qualified_name: Some("UserParagraphData".to_owned()),
    };
    let nested = PreservedElement {
        name: SourceName {
            namespace_uri: Some(vendor_namespace.clone()),
            local_name: "ArcPayload".to_owned(),
            qualified_name: Some("v:ArcPayload".to_owned()),
        },
        attributes: vec![
            PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: "kind".to_owned(),
                    qualified_name: Some("kind".to_owned()),
                },
                value: "opaque".to_owned(),
            },
            PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: "id".to_owned(),
                    qualified_name: Some("id".to_owned()),
                },
                value: owner_id.to_owned(),
            },
        ],
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: vec![
            PreservedNode::Text(ARC_SENTINEL.into()),
            PreservedNode::CData(" + arc cdata".into()),
            PreservedNode::Comment("opaque arc comment".into()),
        ]
        .into(),
    };
    PreservedElement {
        name: payload_name,
        attributes: vec![PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "owner".to_owned(),
                qualified_name: Some("owner".to_owned()),
            },
            value: "generated-arc".to_owned(),
        }],
        namespace_bindings: vec![scriptcraft_core::preservation::NamespaceBinding {
            prefix: "v".to_owned(),
            namespace_uri: vendor_namespace,
        }]
        .into(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: vec![PreservedNode::Element(nested)].into(),
    }
}

fn attach_payload_to_arc_paragraphs(project: &mut Project) {
    fn visit(nodes: &mut [PreservedNode], inside_arc_beat: bool) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            let is_arc_beat = inside_arc_beat || element.name.local_name == "CharacterArcBeat";
            if inside_arc_beat && element.name.local_name == "Paragraph" {
                let owner_id = element
                    .attributes
                    .iter()
                    .find(|attribute| {
                        attribute.name.namespace_uri.is_none()
                            && attribute.name.local_name.eq_ignore_ascii_case("id")
                    })
                    .map(|attribute| attribute.value.clone())
                    .unwrap_or_else(|| unreachable!("generated arc paragraphs have IDs"));
                let payload = generated_arc_payload(&element.name.namespace_uri, &owner_id);
                element.children.push(PreservedNode::Element(payload));
            }
            visit(&mut element.children, is_arc_beat);
        }
    }

    if let Some(preservation) = &mut project.preservation {
        for section in &mut preservation.sections {
            visit(&mut section.children, false);
        }
        preservation.rebuild_indexes();
    }
}

fn has_root_user_document_data(project: &Project) -> bool {
    project.preservation.as_ref().is_some_and(|preservation| {
        preservation
            .sections
            .iter()
            .any(|section| section.name.local_name == "UserDocumentData")
    })
}

fn has_vendor_scene_sibling(project: &Project) -> bool {
    fn contains(nodes: &[PreservedNode]) -> bool {
        nodes.iter().any(|node| match node {
            PreservedNode::Element(element) => element.name.local_name == "VendorSceneSibling"
                || contains(&element.children),
            _ => false,
        })
    }
    project.preservation.as_ref().is_some_and(|preservation| {
        preservation.sections.iter().any(|section| contains(&section.children))
    })
}

fn generated_arc_paragraph_ids(project: &Project) -> Vec<String> {
    fn visit(nodes: &[PreservedNode], in_arc: bool, ids: &mut Vec<String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            let now_in_arc = in_arc || element.name.local_name == "CharacterArcBeat";
            if now_in_arc && element.name.local_name == "Paragraph" {
                if let Some(id) = element.attributes.iter().find(|attribute| {
                    attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id")
                }) {
                    ids.push(id.value.clone());
                }
            }
            visit(&element.children, now_in_arc, ids);
        }
    }

    let mut ids = Vec::new();
    if let Some(preservation) = &project.preservation {
        for section in &preservation.sections {
            visit(&section.children, false, &mut ids);
        }
    }
    ids
}

fn paragraph_by_foreign_id(project: &Project, id: &str) -> Option<PreservedElement> {
    fn visit(nodes: &[PreservedNode], id: &str) -> Option<PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            let foreign_id = element.attributes.iter().find(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
            }).map(|attribute| attribute.value.as_str());
            if element.name.local_name == "Paragraph" && foreign_id == Some(id) {
                return Some(element.clone());
            }
            if let Some(found) = visit(&element.children, id) {
                return Some(found);
            }
        }
        None
    }

    project.preservation.as_ref()?.sections.iter()
        .find_map(|section| visit(&section.children, id))
}

fn make_paragraph_owner_opaque(nodes: &mut [PreservedNode], id: &str) -> bool {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        let foreign_id = element.attributes.iter().find(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        }).map(|attribute| attribute.value.as_str());
        if element.name.local_name == "Paragraph" && foreign_id == Some(id) {
            element.ownership = SourceOwnership::Opaque;
            return true;
        }
        if make_paragraph_owner_opaque(&mut element.children, id) {
            return true;
        }
    }
    false
}

fn replace_character_cue(session: &mut EditingSession, old: &str, new: &str) {
    let index = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == old)
        .unwrap_or_else(|| unreachable!("fixture has the requested Character cue"));
    valid(session.set_selection(Selection {
        anchor: Position { element: index, offset: 0 },
        focus: Position { element: index, offset: old.len() },
    }));
    valid(session.insert(new));
}

#[test]
fn clean_session_stays_clean_after_changed_imported_derived_refresh() {
    let imported = valid(read_v6(&integration_scene_fixture()));
    let mut session = valid(EditingSession::new(imported.project));

    assert!(!session.dirty());
    assert_eq!(
        session.project().elements.last().map(|element| element.text.as_str()),
        Some("(old)")
    );
    let outcome = valid(session.refresh_cast_lists_with_outcome());

    assert!(outcome.changed);
    assert!(!outcome.deferred);
    assert!(has_vendor_scene_sibling(session.project()));
    assert_eq!(
        session.project().elements.last().map(|element| element.text.as_str()),
        Some("(LINA, OMAR)")
    );
    assert!(!session.dirty());
}

#[test]
fn dirty_session_stays_dirty_after_changed_imported_derived_refresh() {
    let imported = valid(read_v6(&integration_scene_fixture()));
    let mut session = valid(EditingSession::new(imported.project));
    let character_index = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Character)
        .unwrap_or_else(|| unreachable!("fixture includes a Character cue"));
    let character_length = session.project().elements[character_index].text.len();
    valid(session.set_selection(Selection {
        anchor: Position {
            element: character_index,
            offset: 0,
        },
        focus: Position {
            element: character_index,
            offset: character_length,
        },
    }));
    valid(session.insert("NORA"));
    assert!(session.dirty());

    let outcome = valid(session.refresh_cast_lists_with_outcome());

    assert!(outcome.changed);
    assert!(!outcome.deferred);
    assert_eq!(
        session.project().elements.last().map(|element| element.text.as_str()),
        Some("(NORA, OMAR)")
    );
    assert!(session.dirty());
}

#[test]
fn current_scenes_skip_per_scene_candidates_and_complete_validation() {
    for scene_count in [1, 8] {
        let mut project = valid(read_v6(&multiple_scene_fixture(scene_count, None))).project;
        let initial = valid(correct_cast_lists_with_outcome(&mut project));
        assert!(initial.changed);

        reset_project_validation_diagnostics();
        let current = valid(correct_cast_lists_with_outcome(&mut project));

        assert!(!current.changed);
        assert!(!current.deferred);
        assert_eq!(current.requested_scenes, scene_count as u64);
        assert_eq!(current.attempted_scenes, scene_count as u64);
        assert_eq!(current.unchanged_scenes, scene_count as u64);
        assert_eq!(current.project_candidate_clones, 1);
        assert_eq!(current.scene_candidate_clones, 0);
        assert_eq!(
            project_validation_diagnostics().complete_project_validations,
            0
        );
    }

    let mut source_less = valid(read_v6(&multiple_scene_fixture(1, None))).project;
    source_less.preservation = None;
    for element in &mut source_less.elements {
        element.source_provenance = None;
    }
    assert!(valid(correct_cast_lists_with_outcome(&mut source_less)).changed);
    reset_project_validation_diagnostics();
    let current = valid(correct_cast_lists_with_outcome(&mut source_less));
    assert_eq!(current.project_candidate_clones, 1);
    assert_eq!(current.scene_candidate_clones, 0);
    assert_eq!(
        project_validation_diagnostics().complete_project_validations,
        0
    );
}

#[test]
fn one_pass_scene_ranges_keep_first_middle_last_and_unheaded_lists_separate() {
    let mut project = valid(read_v6(&multiple_scene_fixture(5, None))).project;
    let outcome = valid(correct_cast_lists_with_outcome(&mut project));

    assert!(outcome.changed);
    let cast_lists = project
        .elements
        .iter()
        .filter(|element| element.role == Role::CastList)
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(cast_lists[0], "(unheaded)");
    assert_eq!(
        &cast_lists[1..],
        [
            "(CHARACTER-0)",
            "(CHARACTER-1)",
            "(CHARACTER-2)",
            "(CHARACTER-3)",
            "(CHARACTER-4)",
        ]
    );
}

#[test]
fn current_changed_and_deferred_scenes_progress_independently_with_exact_counters() {
    let mut project = valid(read_v6(&multiple_scene_fixture(3, Some(1)))).project;
    let initial = valid(correct_cast_lists_with_outcome(&mut project));
    assert!(initial.changed);
    assert!(initial.deferred);
    assert_eq!(initial.deferred_scenes, 1);

    let changed_character = project
        .elements
        .iter_mut()
        .find(|element| element.role == Role::Character && element.text == "CHARACTER-0")
        .unwrap_or_else(|| unreachable!("first scene contains its Character cue"));
    changed_character.text = "NORA".to_owned();

    reset_project_validation_diagnostics();
    let outcome = valid(correct_cast_lists_with_outcome(&mut project));

    assert!(outcome.changed);
    assert!(outcome.deferred);
    assert_eq!(outcome.requested_scenes, 3);
    assert_eq!(outcome.attempted_scenes, 3);
    assert_eq!(outcome.changed_scenes, 1);
    assert_eq!(outcome.unchanged_scenes, 1);
    assert_eq!(outcome.deferred_scenes, 1);
    assert_eq!(outcome.project_candidate_clones, 2);
    assert_eq!(
        project_validation_diagnostics().complete_project_validations,
        1
    );
    let cast_lists = project
        .elements
        .iter()
        .filter(|element| element.role == Role::CastList)
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(cast_lists, ["(NORA)", "(stale-1)", "(CHARACTER-2)"]);
}

#[test]
fn referenced_arc_replacement_is_owner_local_and_keeps_raw_external_source() {
    let mut project = valid(read_v6(&integration_scene_fixture())).project;
    assert!(valid(correct_cast_lists(&mut project)));
    let old_arc_ids = generated_arc_paragraph_ids(&project);
    assert_eq!(old_arc_ids.len(), 2);
    let old_arc_id = old_arc_ids[0].clone();
    let preservation = project.preservation.as_mut().expect("import retains source");
    let content = preservation
        .sections
        .iter_mut()
        .find(|section| section.name.local_name == "Content")
        .expect("fixture has body Content");
    content.children.push(PreservedNode::Element(PreservedElement {
        name: SourceName {
            namespace_uri: None,
            local_name: "VendorExternalUse".to_owned(),
            qualified_name: Some("VendorExternalUse".to_owned()),
        },
        attributes: vec![PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "ref".to_owned(),
                qualified_name: Some("ref".to_owned()),
            },
            value: old_arc_id.clone(),
        }],
        namespace_bindings: Vec::new().into(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: Vec::new().into(),
    }));
    content.children.push(PreservedNode::Element(PreservedElement {
        name: SourceName {
            namespace_uri: None,
            local_name: "VendorArcIdCollision".to_owned(),
            qualified_name: Some("VendorArcIdCollision".to_owned()),
        },
        attributes: vec![PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "id".to_owned(),
                qualified_name: Some("id".to_owned()),
            },
            value: old_arc_id.clone(),
        }],
        namespace_bindings: Vec::new().into(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: Vec::new().into(),
    }));
    preservation.identities.retain(|identity| identity.value != old_arc_id);
    preservation.rebuild_indexes();

    let mut session = valid(EditingSession::new(project));
    replace_character_cue(&mut session, "LINA", "NORA");
    let correction = valid(session.refresh_cast_lists_with_outcome());

    assert!(correction.changed);
    assert!(!correction.deferred);
    assert_eq!(session.project().elements.last().map(|element| element.text.as_str()), Some("(NORA, OMAR)"));
    let current_arc_ids = generated_arc_paragraph_ids(session.project());
    assert_eq!(current_arc_ids.len(), 2);
    assert!(!current_arc_ids.contains(&old_arc_id));
    assert!(!session.project().preservation.as_ref().unwrap().identities.iter().any(|identity| {
        identity.value == old_arc_id
    }));
    let serialized = valid(write_v6(session.project()));
    let serialized_text = std::str::from_utf8(&serialized)
        .unwrap_or_else(|_| unreachable!("native output is UTF-8"));
    assert!(serialized_text.contains(&format!("ref=\"{old_arc_id}\"")));
    assert!(serialized_text.contains(&format!("id=\"{old_arc_id}\"")));
    let reopened = valid(read_v6(&serialized));
    assert_eq!(reopened.project.elements, session.project().elements);
    assert!(!generated_arc_paragraph_ids(&reopened.project).contains(&old_arc_id));
}

#[test]
fn missing_managed_scene_anchor_is_deferred_without_refresh_failure() {
    let mut project = valid(read_v6(&integration_scene_fixture())).project;
    assert!(valid(correct_cast_lists(&mut project)));
    let previous_cast = project.elements.last().map(|element| element.text.clone());
    let preservation = project.preservation.as_mut().expect("source-backed scene");
    assert!(preservation.sections.iter_mut().any(|section| {
        make_paragraph_owner_opaque(&mut section.children, "body-001")
    }));
    preservation.rebuild_indexes();

    let outcome = valid(correct_cast_lists_with_outcome(&mut project));
    assert!(!outcome.changed);
    assert!(outcome.deferred);
    assert_eq!(outcome.deferred_scenes, 1);
    assert_eq!(project.elements.last().map(|element| element.text.clone()), previous_cast);
}

#[test]
fn deleted_heading_drops_its_nested_derived_owner_and_recreation_is_fresh() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Scene Heading" id="scene-old"><Text>INT. OLD ROOM - DAY</Text></Paragraph>
      <Paragraph Type="Character" id="cue-old"><Text>LINA</Text></Paragraph>
      <Paragraph Type="Action" id="action-old"><Text>Carry forward.</Text></Paragraph>
      <Paragraph Type="Scene Heading" id="scene-next"><Text>EXT. STREET - NIGHT</Text></Paragraph>
      <Paragraph Type="Character" id="cue-next"><Text>OMAR</Text></Paragraph>
      <Paragraph Type="Action" id="action-next"><Text>Continue.</Text></Paragraph>
    </Content></FinalDraft>"#;
    let mut project = valid(read_v6(source)).project;
    assert!(valid(correct_cast_lists(&mut project)));
    let old_arc_ids = generated_arc_paragraph_ids(&project);
    assert_eq!(old_arc_ids.len(), 1);
    let old_heading_id = project.elements[0].id.clone();
    let old_arc_id = old_arc_ids[0].clone();
    let preservation = project.preservation.as_mut().expect("source-backed heading");
    let content = preservation
        .sections
        .iter_mut()
        .find(|section| section.name.local_name == "Content")
        .expect("source has Content");
    content.children.push(PreservedNode::Element(PreservedElement {
        name: SourceName {
            namespace_uri: None,
            local_name: "VendorArcReference".to_owned(),
            qualified_name: Some("VendorArcReference".to_owned()),
        },
        attributes: vec![PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "ref".to_owned(),
                qualified_name: Some("ref".to_owned()),
            },
            value: old_arc_id.clone(),
        }],
        namespace_bindings: Vec::new().into(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: Vec::new().into(),
    }));
    preservation.rebuild_indexes();

    let mut session = valid(EditingSession::new(project));
    let heading_len = session.project().elements[0].text.len();
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: heading_len },
    }));
    valid(session.insert(""));
    valid(session.delete());
    assert!(session.project().elements.iter().all(|element| element.id != old_heading_id));
    assert!(!generated_arc_paragraph_ids(session.project()).contains(&old_arc_id));

    valid(session.set_selection(Selection::caret(Position { element: 1, offset: 2 })));
    valid(session.split());
    let recreated_index = 2;
    valid(session.change_role(Role::SceneHeading));
    let recreated = &session.project().elements[recreated_index];
    assert_eq!(recreated.role, Role::SceneHeading);
    assert_ne!(recreated.id, old_heading_id);
    let recreated_foreign_id = recreated
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("recreated heading gets fresh current source identity");
    let recreated_source = paragraph_by_foreign_id(session.project(), recreated_foreign_id)
        .expect("recreated heading has a source owner");
    assert!(!recreated_source.children.iter().any(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "UserParagraphData"
    )));
    assert!(!generated_arc_paragraph_ids(session.project()).contains(&old_arc_id));
    let bytes = valid(write_v6(session.project()));
    let text = std::str::from_utf8(&bytes).unwrap_or_else(|_| unreachable!("native output is UTF-8"));
    assert!(text.contains(&format!("ref=\"{old_arc_id}\"")));
    let reopened = valid(read_v6(&bytes));
    assert_eq!(reopened.project.elements, session.project().elements);
}

#[test]
fn unsupported_scene_metadata_defers_locally_and_saved_rebase_preserves_the_deferral() {
    let source = integration_scene_fixture();
    let source_text = std::str::from_utf8(&source)
        .unwrap_or_else(|_| unreachable!("synthetic source is UTF-8"));
    let source_text = source_text.replace(
        "</Content>\n  <TitlePage>",
        "<Paragraph Type=\"Scene Heading\" id=\"scene-two\"><Text>INT. PARK - NIGHT</Text><SceneProperties><VendorSceneData marker=\"opaque\"/></SceneProperties></Paragraph><Paragraph Type=\"Character\" id=\"cue-two\"><Text>IVY</Text></Paragraph><Paragraph Type=\"Cast List\" id=\"cast-two\"><Text>(old-two)</Text></Paragraph></Content>\n  <TitlePage>",
    );
    let imported = valid(read_v6(source_text.as_bytes()));
    let mut session = valid(EditingSession::new(imported.project));
    replace_character_cue(&mut session, "LINA", "NORA");
    replace_character_cue(&mut session, "IVY", "ZOE");

    let correction = valid(session.refresh_cast_lists_with_outcome());
    assert!(correction.changed);
    assert!(correction.deferred);
    assert_eq!(correction.deferred_scenes, 1);
    assert_eq!(session.project().elements.iter().find(|element| element.role == Role::CastList).map(|element| element.text.as_str()), Some("(NORA, OMAR)"));
    assert_eq!(session.project().elements.last().map(|element| element.text.as_str()), Some("(old-two)"));
    assert!(valid(write_v6(session.project())).windows(b"VendorSceneData".len()).any(|window| window == b"VendorSceneData"));

    let deferred_until_relevant_edit = valid(session.refresh_cast_lists_with_outcome());
    assert_eq!(deferred_until_relevant_edit.attempted_scenes, 0);
    replace_character_cue(&mut session, "ZOE", "IVY");
    let retried = valid(session.refresh_cast_lists_with_outcome());
    assert!(!retried.changed);
    assert!(retried.deferred);
    assert_eq!(retried.requested_scenes, 1);
    assert_eq!(retried.attempted_scenes, 1);
    assert_eq!(retried.deferred_scenes, 1);
    assert_eq!(retried.project_candidate_clones, 1);

    let directory = TestDirectory::new();
    let path = directory.path("deferred-scenes.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(path.clone()),
    ));
    assert_eq!(session.pending_derived_work_summary().deferred_scene_count, 1);
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    let saved = valid(fs::read(&path));
    let reopened = valid(read_v6(&saved));
    assert_eq!(reopened.project.elements, session.project().elements);
    assert!(std::str::from_utf8(&saved).unwrap_or_else(|_| unreachable!("native output is UTF-8"))
        .contains("VendorSceneData"));
}

#[test]
fn representative_edit_families_survive_repeated_save_save_as_reopen_and_reedit() {
    let directory = TestDirectory::new();
    let first_path = directory.path("first-cycle.fdx");
    let second_path = directory.path("second-cycle.fdx");
    let mut session = valid(EditingSession::new(
        valid(read_v6(SYNTHETIC)).project,
    ));
    let mut target = FdxSaveTarget::new_unsaved();

    let body_length = session.project().elements[0].text.len();
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: body_length })));
    valid(session.insert(" edited"));
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(first_path.clone()),
    ));
    let first_reopen = valid(read_v6(&valid(fs::read(&first_path))));
    assert_eq!(first_reopen.project.elements, session.project().elements);
    assert_eq!(first_reopen.project.title_page, session.project().title_page);

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 2 })));
    valid(session.split());
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    let second_reopen = valid(read_v6(&valid(fs::read(&first_path))));
    assert_eq!(second_reopen.project.elements, session.project().elements);
    assert_eq!(second_reopen.project.title_page, session.project().title_page);

    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert_title("Revised "));
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(second_path.clone()),
    ));
    let third_reopen = valid(read_v6(&valid(fs::read(&second_path))));
    assert_eq!(third_reopen.project.elements, session.project().elements);
    assert_eq!(third_reopen.project.title_page, session.project().title_page);

    valid(session.set_selection(Selection::caret(Position { element: 2, offset: 0 })));
    valid(session.toggle_dual_dialogue());
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    let unwrapped_reopen = valid(read_v6(&valid(fs::read(&second_path))));
    assert_eq!(unwrapped_reopen.project.elements, session.project().elements);
    assert_eq!(unwrapped_reopen.project.dual_dialogue_groups, session.project().dual_dialogue_groups);

    valid(session.set_selection(Selection::caret(Position { element: 2, offset: 0 })));
    valid(session.toggle_dual_dialogue());
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    let wrapped_reopen = valid(read_v6(&valid(fs::read(&second_path))));
    assert_eq!(wrapped_reopen.project.elements, session.project().elements);
    assert_eq!(wrapped_reopen.project.dual_dialogue_groups, session.project().dual_dialogue_groups);

    // Saved-session rebase must retain canonical intent and allow a second edit
    // cycle rather than reviving the Open-time preservation graph as authority.
    let character_index = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Character)
        .unwrap_or_else(|| unreachable!("fixture has a Character cue"));
    let character_len = session.project().elements[character_index].text.len();
    valid(session.set_selection(Selection {
        anchor: Position { element: character_index, offset: 0 },
        focus: Position { element: character_index, offset: character_len },
    }));
    valid(session.insert("MARA"));
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    let final_reopen = valid(read_v6(&valid(fs::read(&second_path))));
    assert_eq!(final_reopen.project.elements, session.project().elements);
    assert_eq!(final_reopen.project.title_page, session.project().title_page);
}

#[test]
fn derived_scene_and_cast_refresh_keep_payload_owned_and_noncanonical_exports_ignore_it() {
    let imported = valid(read_v6(&integration_scene_fixture()));
    let mut project = imported.project;
    assert!(valid(correct_cast_lists(&mut project)));
    assert_eq!(project.elements.last().map(|element| element.text.as_str()), Some("(LINA, OMAR)"));
    attach_payload_to_arc_paragraphs(&mut project);
    let before = paragraph_payloads(&project);
    assert!(!project
        .preservation
        .as_ref()
        .is_some_and(|preservation| preservation.identities.iter().any(|identity| {
            matches!(identity.value.as_str(), "payload-body-id" | "not-a-group")
        })));
    assert!(before.iter().any(|(_, payload)| {
        payload.children.iter().any(|node| matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "CastPayload"
                    && element.children.iter().any(|child| matches!(child, PreservedNode::Text(text) if text.as_str() == CAST_SENTINEL))))
    }));
    let arc_payload_owners = before
        .iter()
        .filter(|(_, payload)| {
            payload.children.iter().any(|node| matches!(node,
                PreservedNode::Element(element)
                    if element.name.local_name == "ArcPayload"))
        })
        .map(|(owner, _)| owner.clone())
        .collect::<Vec<_>>();
    assert_eq!(arc_payload_owners.len(), 2);

    let text = valid(export_text(&project));
    let pdf = valid(export_pdf(&project));
    assert!(!text.text.contains(SCENE_SENTINEL));
    assert!(!text.text.contains(CAST_SENTINEL));
    assert!(!text.text.contains(ARC_SENTINEL));
    let pdf_text = std::str::from_utf8(&pdf)
        .unwrap_or_else(|_| unreachable!("generated PDF content is UTF-8"));
    assert!(!pdf_text.contains(SCENE_SENTINEL));
    assert!(!pdf_text.contains(CAST_SENTINEL));
    assert!(!pdf_text.contains(ARC_SENTINEL));

    let mut session = valid(EditingSession::new(project));
    let character_index = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Character)
        .unwrap_or_else(|| unreachable!("fixture includes a Character cue"));
    let character_length = session.project().elements[character_index].text.len();
    valid(session.set_selection(Selection {
        anchor: Position {
            element: character_index,
            offset: 0,
        },
        focus: Position {
            element: character_index,
            offset: character_length,
        },
    }));
    valid(session.insert("NORA"));
    assert!(valid(session.refresh_cast_lists()));
    assert_eq!(
        session.project().elements.last().map(|element| element.text.as_str()),
        Some("(NORA, OMAR)")
    );
    let after = paragraph_payloads(session.project());
    let remaining_arc_owners = after
        .iter()
        .filter(|(_, payload)| {
            payload.children.iter().any(|node| matches!(node,
                PreservedNode::Element(element)
                    if element.name.local_name == "ArcPayload"))
        })
        .map(|(owner, _)| owner.clone())
        .collect::<Vec<_>>();
    assert_eq!(remaining_arc_owners.len(), 1);
    assert!(arc_payload_owners
        .iter()
        .any(|owner| remaining_arc_owners.contains(owner)));
    assert!(after.iter().all(|(owner, payload)| {
        !arc_payload_owners.contains(owner) || before.iter().any(|(old_owner, old_payload)| {
            old_owner == owner && old_payload == payload
        })
    }));
    let native = valid(write_v6(session.project()));
    let reopened = valid(read_v6(&native));
    assert_eq!(paragraph_payloads(&reopened.project), after);
    assert_eq!(reopened.project.elements, session.project().elements);
}

#[test]
fn transactional_save_save_as_reopen_and_conflict_keep_the_same_opaque_owners() {
    let directory = TestDirectory::new();
    let first_path = directory.path("first.fdx");
    let second_path = directory.path("second.fdx");
    let conflict_path = directory.path("existing.fdx");
    let imported = valid(read_v6(&integration_scene_fixture()));
    let mut project = imported.project;
    assert!(valid(correct_cast_lists(&mut project)));
    attach_payload_to_arc_paragraphs(&mut project);
    let expected_payloads = paragraph_payloads(&project);
    let mut session = valid(EditingSession::new(project));
    let heading_length = session.project().elements[0].text.len();
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: heading_length,
    })));
    valid(session.insert(" revised"));
    assert!(session.dirty());
    let mut target = FdxSaveTarget::new_unsaved();
    fs::write(&conflict_path, b"prior destination bytes")
        .unwrap_or_else(|_| unreachable!("conflict destination can be written"));
    let before_failure = session.clone();
    let failed = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(conflict_path.clone()),
    );
    assert!(matches!(
        failed,
        Err(TransactionalSaveError::Storage(ref storage))
            if storage.kind == FdxStorageErrorKind::DestinationExists
    ));
    assert_eq!(fs::read(&conflict_path).ok().as_deref(), Some(&b"prior destination bytes"[..]));
    assert_eq!(session, before_failure);
    assert_eq!(paragraph_payloads(session.project()), expected_payloads);
    assert_eq!(target.path(), None);
    assert_eq!(target.baseline(), None);

    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(first_path.clone()),
    ));
    assert_eq!(paragraph_payloads(session.project()), expected_payloads);
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    assert_eq!(paragraph_payloads(session.project()), expected_payloads);
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(second_path.clone()),
    ));
    assert_eq!(target.path(), Some(second_path.as_path()));
    assert!(!session.dirty());

    for path in [&first_path, &second_path] {
        let bytes = valid(fs::read(path));
        let reopened = valid(read_v6(&bytes));
        assert_eq!(paragraph_payloads(&reopened.project), expected_payloads);
        assert_eq!(reopened.project.elements, session.project().elements);
    }
}

#[test]
fn injected_native_replacement_failure_keeps_destination_session_and_payload_unchanged() {
    let directory = TestDirectory::new();
    let destination = directory.path("replacement-failure.fdx");
    let prior_bytes = b"previously published destination";
    fs::write(&destination, prior_bytes)
        .unwrap_or_else(|_| unreachable!("prior destination can be written"));
    let imported = valid(read_v6(&integration_scene_fixture()));
    let mut project = imported.project;
    assert!(valid(correct_cast_lists(&mut project)));
    attach_payload_to_arc_paragraphs(&mut project);
    let expected_payloads = paragraph_payloads(&project);
    let mut session = valid(EditingSession::new(project));
    let heading_length = session.project().elements[0].text.len();
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: heading_length,
    })));
    valid(session.insert(" revised"));
    let before_session = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();

    let error = save_session(
        &FailReplacement,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(destination.clone()),
    )
    .err()
    .unwrap_or_else(|| unreachable!("injected native replacement must fail"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Io(StorageOperation::Replace)));
    assert_eq!(fs::read(&destination).ok().as_deref(), Some(&prior_bytes[..]));
    assert_eq!(session, before_session);
    assert_eq!(paragraph_payloads(session.project()), expected_payloads);
    assert!(session.dirty());
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
    assert_eq!(
        fs::read_dir(&directory.0)
            .unwrap_or_else(|_| unreachable!("integration directory remains available"))
            .count(),
        1
    );
}

#[test]
fn untouched_version_two_reference_save_and_save_as_keep_seventy_attachments_without_root_reconstruction() {
    let reference_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.reference/The Once and Future King.fdx");
    let source_before = valid(fs::read(&reference_path));
    assert_eq!(source_before, REFERENCE_V2);
    let outcome = valid(import_fdx(&source_before, FdxMigrationConsent::Continue));
    let FdxImportOutcome::Candidate(candidate) = outcome else {
        unreachable!("Continue creates the Version 2 migration candidate")
    };
    assert_eq!(candidate.document.project.elements.len(), 22);
    assert_eq!(candidate.document.project.title_page.len(), 48);
    let expected_payloads = paragraph_payloads(&candidate.document.project);
    assert_eq!(expected_payloads.len(), 70);
    assert_eq!(
        section_payload_count(&candidate.document.project, SectionKind::Content),
        22
    );
    assert_eq!(
        section_payload_count(&candidate.document.project, SectionKind::TitlePage),
        48
    );
    let directory = TestDirectory::new();
    let first_path = directory.path("reference-save.fdx");
    let second_path = directory.path("reference-save-as.fdx");
    let mut session = valid(EditingSession::new(candidate.document.project));
    let mut target = FdxSaveTarget::new_unsaved();

    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(first_path.clone()),
    ));
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(second_path.clone()),
    ));

    for path in [&first_path, &second_path] {
        let bytes = valid(fs::read(path));
        let reopened = valid(read_v6(&bytes));
        assert_eq!(reopened.project.elements.len(), 22);
        assert_eq!(reopened.project.title_page.len(), 48);
        assert_eq!(paragraph_payloads(&reopened.project), expected_payloads);
        assert!(!has_root_user_document_data(&reopened.project));
    }
    assert_eq!(valid(fs::read(&reference_path)), source_before);
}
