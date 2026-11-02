#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::EditingSession;
use scriptcraft_core::fdx::{self, FdxError};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
    TransactionalSaveError,
};
use scriptcraft_core::preservation::PreservedNode;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const REFERENCE: &[u8] = include_bytes!("../../../.reference/reference2.fdx");
const PRIVATE_MARKER: &str = "diagnostic-private-source-type-2389";
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v5-wo001-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path)
            .unwrap_or_else(|_| unreachable!("temporary diagnostic directory can be created"));
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

fn inject_internal_source_type_mismatch(project: &mut scriptcraft_core::Project) {
    fn visit(nodes: &mut [PreservedNode]) -> bool {
        for node in nodes {
            if let PreservedNode::Element(element) = node {
                if element.source_type.is_some() {
                    element.source_type = Some(PRIVATE_MARKER.to_owned());
                    return true;
                }
                if visit(&mut element.children) {
                    return true;
                }
            }
        }
        false
    }

    let state = project
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("opened reference has preservation state"));
    let changed = state
        .sections
        .iter_mut()
        .any(|section| visit(&mut section.children));
    assert!(changed);
}

fn swap_id_values(project: &mut scriptcraft_core::Project) -> bool {
    fn visit(nodes: &mut [PreservedNode]) -> bool {
        let locations = nodes
            .iter()
            .enumerate()
            .filter_map(|(node_index, node)| {
                let PreservedNode::Element(element) = node else { return None };
                element
                    .attributes
                    .iter()
                    .position(|attribute| attribute.name.local_name == "id")
                    .map(|attribute_index| (node_index, attribute_index))
            })
            .collect::<Vec<_>>();
        if locations.len() >= 2 {
            let ((first_node, first_attribute), (second_node, second_attribute)) =
                (locations[0], locations[1]);
            if first_node < second_node {
                let (left, right) = nodes.split_at_mut(second_node);
                let PreservedNode::Element(first) = &mut left[first_node] else { unreachable!() };
                let PreservedNode::Element(second) = &mut right[0] else { unreachable!() };
                std::mem::swap(
                    &mut first.attributes[first_attribute].value,
                    &mut second.attributes[second_attribute].value,
                );
            } else {
                let (left, right) = nodes.split_at_mut(first_node);
                let PreservedNode::Element(second) = &mut left[second_node] else { unreachable!() };
                let PreservedNode::Element(first) = &mut right[0] else { unreachable!() };
                std::mem::swap(
                    &mut first.attributes[first_attribute].value,
                    &mut second.attributes[second_attribute].value,
                );
            }
            return true;
        }
        nodes.iter_mut().any(|node| match node {
            PreservedNode::Element(element) => visit(&mut element.children),
            _ => false,
        })
    }

    let Some(state) = project.preservation.as_mut() else { return false };
    state.sections.iter_mut().any(|section| visit(&mut section.children))
}

fn assert_public_code30(error: &TransactionalSaveError) {
    let TransactionalSaveError::Codec(FdxError::SaveDiagnosticSourceTreeMismatch(detail)) = error else {
        unreachable!("normalized tree mismatch retains its dedicated public code 30 detail")
    };
    assert!(detail.starts_with("path=section["));
    assert!(detail.contains("field=source-type"));
    assert!(detail.len() <= 240);
    assert!(!detail.contains(PRIVATE_MARKER));
    let complete = error.to_string();
    assert!(complete.contains("30: FDX contains a construct"));
    assert!(complete.contains("field=source-type"));
    assert!(!complete.contains(PRIVATE_MARKER));
}

#[test]
fn code30_detail_survives_public_codec_save_and_save_as_without_publication() {
    let directory = TestDirectory::new();
    let source_path = directory.path("source.fdx");
    let destination = directory.path("copy.fdx");
    fs::write(&source_path, REFERENCE)
        .unwrap_or_else(|_| unreachable!("checked-in reference is writable as a disposable fixture"));
    let backend = StdFdxStorageFileSystem;
    let opened = open(&backend, &source_path)
        .unwrap_or_else(|_| unreachable!("checked-in Final Draft reference opens"));
    let original_bytes = fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("disposable source remains readable"));
    let baseline = opened.baseline.clone();
    let mut project = opened.document.project;
    inject_internal_source_type_mismatch(&mut project);

    let direct = fdx::write_v6(&project)
        .err()
        .unwrap_or_else(|| unreachable!("source Type metadata mismatch fails strict code 30 comparison"));
    assert!(matches!(&direct, FdxError::SaveDiagnosticSourceTreeMismatch(_)));
    assert!(direct.to_string().starts_with("30: FDX contains a construct"));
    assert!(!direct.to_string().contains(PRIVATE_MARKER));

    let mut save_session_state = EditingSession::new(project.clone())
        .unwrap_or_else(|_| unreachable!("diagnostic candidate initializes"));
    let session_before = save_session_state.clone();
    let mut save_target = FdxSaveTarget::from_baseline(baseline.clone());
    let target_before = save_target.clone();
    let save_error = save_session(
        &backend,
        &mut save_session_state,
        &mut save_target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("code 30 candidate is rejected transactionally"));
    assert_public_code30(&save_error);
    assert_eq!(save_session_state, session_before);
    assert_eq!(save_target, target_before);
    assert_eq!(fs::read(&source_path).unwrap_or_else(|_| unreachable!()), original_bytes);

    let mut save_as_session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("Save As diagnostic candidate initializes"));
    let save_as_session_before = save_as_session.clone();
    let mut save_as_target = FdxSaveTarget::new_unsaved();
    let save_as_target_before = save_as_target.clone();
    let save_as_error = save_session(
        &backend,
        &mut save_as_session,
        &mut save_as_target,
        FdxSaveMode::SaveAs(destination.clone()),
    )
    .err()
    .unwrap_or_else(|| unreachable!("code 30 candidate is rejected before Save As publication"));
    assert_public_code30(&save_as_error);
    assert_eq!(save_as_session, save_as_session_before);
    assert_eq!(save_as_target, save_as_target_before);
    assert!(!destination.exists());
    assert_eq!(fs::read(&source_path).unwrap_or_else(|_| unreachable!()), original_bytes);
}

#[test]
fn reference_backed_id_placement_difference_reports_attribute_field_without_values() {
    let document = fdx::read_v6(REFERENCE)
        .unwrap_or_else(|_| unreachable!("checked-in reference is readable"));
    let mut project = document.project;
    assert!(swap_id_values(&mut project));

    let error = fdx::write_v6(&project)
        .err()
        .unwrap_or_else(|| unreachable!("swapped ID placement is caught by strict tree comparison"));
    let FdxError::SaveDiagnosticSourceTreeMismatch(detail) = error else {
        unreachable!("equal ID sets with changed placement reach source-tree code 30")
    };
    assert!(detail.starts_with("path=section["));
    assert!(detail.contains("field=attribute.value"));
    assert!(detail.len() <= 240);
    assert!(!detail.contains("uuid"));
    assert!(!detail.contains("id="));
}
