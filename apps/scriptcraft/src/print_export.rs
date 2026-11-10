//! Immutable, platform-neutral input to the native print operation.
//!
//! This module builds only PDF-backed Print composition. Native panels and
//! printer APIs are intentionally owned by later macOS/Windows adapters.

use scriptcraft_core::exports::{self, ExportError};
use scriptcraft_core::{LayoutInputs, Project};
use scriptcraft_commands::PdfExportOptions;
use std::ffi::OsStr;
use std::path::Path;

const MAX_PRINT_JOB_TITLE_BYTES: usize = 128;
const UNTITLED_PRINT_JOB_TITLE: &str = "Untitled Screenplay";

/// Owned job data: the native operation can retain these bytes independently
/// of later project edits or layout-cache changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrintRequest {
    pdf_data: Vec<u8>,
    job_title: String,
}

/// Owned semantic/fixed-layout snapshot captured before the native panel opens.
/// The native settings result can retain it through acceptance without building
/// PDF bytes until the later delivery step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrintCompositionSnapshot {
    project: Project,
    layout_inputs: LayoutInputs,
    native_path: Option<std::path::PathBuf>,
}

impl PrintCompositionSnapshot {
    pub(super) fn capture(
        project: &Project,
        layout_inputs: &LayoutInputs,
        native_path: Option<&Path>,
    ) -> Self {
        Self {
            project: project.clone(),
            layout_inputs: layout_inputs.clone(),
            native_path: native_path.map(Path::to_path_buf),
        }
    }

    pub(super) fn build_request(
        &self,
        options: PdfExportOptions,
    ) -> Result<PrintRequest, ExportError> {
        build_print_request(
            &self.project,
            &self.layout_inputs,
            options,
            self.native_path.as_deref(),
        )
    }
}

impl PrintRequest {
    pub(super) fn pdf_data(&self) -> &[u8] {
        &self.pdf_data
    }

    pub(super) fn job_title(&self) -> &str {
        &self.job_title
    }
}

/// Shared PDF builder used by both Save As PDF and native printing. The core
/// exporter owns `LayoutComposition::Print` and consumes fixed document inputs.
pub(super) fn build_pdf_bytes(
    project: &Project,
    layout_inputs: &LayoutInputs,
    options: PdfExportOptions,
) -> Result<Vec<u8>, ExportError> {
    exports::export_pdf_with_layout_inputs_and_options(
        project,
        layout_inputs,
        exports::PdfExportOptions {
            include_title_page: options.include_title_page,
        },
    )
}

/// Builds one complete request from borrowed semantic/fixed-layout inputs and
/// returns only owned bytes and a bounded basename-derived job title.
pub(super) fn build_print_request(
    project: &Project,
    layout_inputs: &LayoutInputs,
    options: PdfExportOptions,
    native_path: Option<&Path>,
) -> Result<PrintRequest, ExportError> {
    Ok(PrintRequest {
        pdf_data: build_pdf_bytes(project, layout_inputs, options)?,
        job_title: print_job_title(native_path),
    })
}

pub(super) fn print_job_title(native_path: Option<&Path>) -> String {
    let Some(stem) = native_path
        .and_then(Path::file_stem)
        .filter(|stem| !stem.is_empty())
        .and_then(OsStr::to_str)
    else {
        return UNTITLED_PRINT_JOB_TITLE.to_owned();
    };

    let mut bounded = String::new();
    for character in stem.chars() {
        let character = if character.is_control() { ' ' } else { character };
        if bounded.len() + character.len_utf8() > MAX_PRINT_JOB_TITLE_BYTES {
            break;
        }
        bounded.push(character);
    }
    let bounded = bounded.trim();
    if bounded.is_empty() {
        UNTITLED_PRINT_JOB_TITLE.to_owned()
    } else {
        bounded.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_print_request, print_job_title, PrintCompositionSnapshot,
        MAX_PRINT_JOB_TITLE_BYTES,
    };
    use scriptcraft_commands::PdfExportOptions;
    use scriptcraft_core::dual_dialogue::{
        DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance,
    };
    use scriptcraft_core::layout::{BodyEditorPresentation, LayoutCache};
    use scriptcraft_core::{LayoutInputs, Project, Role, ScreenplayElement, StableId};
    use std::path::Path;

    fn valid<T, E>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|_| unreachable!("print-request fixtures use valid bounded input"))
    }

    fn extracted_text(bytes: &[u8]) -> String {
        let pdf = String::from_utf8_lossy(bytes);
        let mut output = String::new();
        let mut remaining = pdf.as_ref();
        while let Some(end) = remaining.find(") Tj") {
            let (before_operator, after_operator) = remaining.split_at(end);
            if let Some(start) = before_operator.rfind('(') {
                output.push_str(&before_operator[start + 1..]);
            }
            remaining = &after_operator[4..];
        }
        output
    }

    fn fixture() -> Project {
        let mut project = valid(Project::new());
        project.elements.clear();
        for (index, (role, text)) in [
            (Role::Summary, "SUMMARY_PRINT_SENTINEL"),
            (Role::Outline1, "OUTLINE1_PRINT_SENTINEL"),
            (Role::Outline2, "OUTLINE2_PRINT_SENTINEL"),
            (Role::Outline3, "OUTLINE3_PRINT_SENTINEL"),
            (Role::Note, "NOTE_PRINT_SENTINEL"),
            (Role::Action, "VISIBLE_PRINT_CONTROL"),
        ]
        .into_iter()
        .enumerate()
        {
            project.elements.push(valid(ScreenplayElement::new(
                format!("print-request-{index}"),
                role,
                text,
            )));
        }
        project
    }

    fn dual_dialogue_fixture() -> Project {
        let mut project = valid(Project::new());
        project.elements = [
            (Role::Action, "Before"),
            (Role::Character, "LEFT CUE"),
            (Role::Dialogue, "Left dialogue"),
            (Role::Character, "RIGHT CUE"),
            (Role::Dialogue, "Right dialogue"),
            (Role::Action, "After"),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (role, text))| {
            valid(ScreenplayElement::new(format!("group-print-{index}"), role, text))
        })
        .collect();
        project.dual_dialogue_groups.push(DualDialogueGroup {
            id: valid(StableId::new("print-request-group")),
            left: DualDialogueSide {
                character: project.elements[1].id.clone(),
                paragraphs: vec![project.elements[2].id.clone()],
            },
            right: DualDialogueSide {
                character: project.elements[3].id.clone(),
                paragraphs: vec![project.elements[4].id.clone()],
            },
            wrapper: DualDialogueWrapperProvenance::default(),
        });
        project
    }

    #[test]
    fn include_title_page_option_selects_title_first_or_body_only_print_output() {
        let project = fixture();
        let inputs = LayoutInputs::default();
        let included = valid(build_print_request(
            &project,
            &inputs,
            PdfExportOptions::default(),
            None,
        ));
        let excluded = valid(build_print_request(
            &project,
            &inputs,
            PdfExportOptions {
                include_title_page: false,
            },
            None,
        ));
        let included_text = extracted_text(included.pdf_data());
        let excluded_text = extracted_text(excluded.pdf_data());
        let title_position = included_text
            .find("SCRIPT TITLE")
            .unwrap_or_else(|| unreachable!("the default title page contains its title placeholder"));
        let body_position = included_text
            .find("VISIBLE_PRINT_CONTROL")
            .unwrap_or_else(|| unreachable!("Print composition contains the visible body"));
        assert!(title_position < body_position);
        assert!(!excluded_text.contains("SCRIPT TITLE"));
        assert!(excluded_text.contains("VISIBLE_PRINT_CONTROL"));
    }

    #[test]
    fn hidden_roles_are_omitted_from_print_pdf_but_visible_action_remains() {
        let project = fixture();
        let request = valid(build_print_request(
            &project,
            &LayoutInputs::default(),
            PdfExportOptions {
                include_title_page: false,
            },
            None,
        ));
        let text = extracted_text(request.pdf_data());
        assert!(text.contains("VISIBLE_PRINT_CONTROL"));
        for sentinel in [
            "SUMMARY_PRINT_SENTINEL",
            "OUTLINE1_PRINT_SENTINEL",
            "OUTLINE2_PRINT_SENTINEL",
            "OUTLINE3_PRINT_SENTINEL",
            "NOTE_PRINT_SENTINEL",
        ] {
            assert!(!text.contains(sentinel), "Print omits {sentinel}");
        }
    }

    #[test]
    fn print_request_is_independent_of_active_body_dual_dialogue_presentation() {
        let project = dual_dialogue_fixture();
        let group_id = project.dual_dialogue_groups[0].id.clone();
        let inactive = BodyEditorPresentation::for_project(&project, None);
        let active = BodyEditorPresentation::for_project(&project, Some(group_id));
        let inputs = LayoutInputs::default();
        let mut cache = LayoutCache::default();
        let (inactive_body_layout, _) = valid(cache.get_or_build_body_editor(
            1,
            &project,
            &inputs,
            &inactive,
        ));
        assert!(inactive_body_layout.active_group_id.is_none());
        let inactive_request = valid(build_print_request(
            &project,
            &inputs,
            PdfExportOptions {
                include_title_page: false,
            },
            None,
        ));
        let (active_body_layout, _) = valid(cache.get_or_build_body_editor(
            1,
            &project,
            &inputs,
            &active,
        ));
        assert!(active_body_layout.active_group_id.is_some());
        let active_request = valid(build_print_request(
            &project,
            &inputs,
            PdfExportOptions {
                include_title_page: false,
            },
            None,
        ));
        assert_eq!(active_request.pdf_data(), inactive_request.pdf_data());
        let text = extracted_text(active_request.pdf_data());
        assert!(text.find("LEFT CUE").is_some());
        assert!(text.find("RIGHT CUE").is_some());
        assert!(text.find("Left dialogue").is_some());
        assert!(text.find("Right dialogue").is_some());
    }

    #[test]
    fn pre_panel_snapshot_keeps_captured_project_and_filename_for_later_build() {
        let mut project = fixture();
        let inputs = LayoutInputs::default();
        let path = Path::new("/private/scripts/captured-name.fdx");
        let snapshot = PrintCompositionSnapshot::capture(&project, &inputs, Some(path));

        project.elements[5].text = "edited while print panel is open".to_owned();
        let request = valid(snapshot.build_request(PdfExportOptions {
            include_title_page: false,
        }));
        let text = extracted_text(request.pdf_data());
        assert!(text.contains("VISIBLE_PRINT_CONTROL"));
        assert!(!text.contains("edited while print panel is open"));
        assert_eq!(request.job_title(), "captured-name");
    }

    #[test]
    fn repeated_requests_are_identical_and_return_bytes_detached_from_project() {
        let mut project = fixture();
        let before = project.clone();
        let inputs = LayoutInputs::default();
        let options = PdfExportOptions {
            include_title_page: false,
        };
        let first = valid(build_print_request(&project, &inputs, options, None));
        let repeated = valid(build_print_request(&project, &inputs, options, None));
        assert_eq!(first, repeated);
        assert_eq!(project, before);

        project.elements[5].text = "edited after request".to_owned();
        assert!(extracted_text(first.pdf_data()).contains("VISIBLE_PRINT_CONTROL"));
        assert!(!extracted_text(first.pdf_data()).contains("edited after request"));
    }

    #[test]
    fn print_job_title_uses_only_a_bounded_sanitized_filename_stem() {
        assert_eq!(
            print_job_title(Some(Path::new("/private/documents/episode.final.fdx"))),
            "episode.final",
        );
        assert_eq!(print_job_title(None), "Untitled Screenplay");
        assert_eq!(
            print_job_title(Some(Path::new("/private/documents/\t\n.fdx"))),
            "Untitled Screenplay",
        );
        let long_title = format!("{}.fdx", "épisode".repeat(40));
        let title = print_job_title(Some(Path::new(&long_title)));
        assert!(title.len() <= MAX_PRINT_JOB_TITLE_BYTES);
        assert!(!title.contains('\n'));
        assert!(!title.contains("/private/documents"));
    }
}
