//! Disposable, current-document controls shared by the toolbar, status, and editor.

use super::element_choices::{ordered_element_choices, ElementChoice};
use super::element_shortcuts::{
    resolve_element_shortcuts, ElementShortcutIssueKind, ResolvedElementShortcuts,
};
use super::menu::MenuAcceleratorOwner;
use super::shortcuts::ShortcutPlatform;
use crate::app::ScriptCraftApp;
use scriptcraft_commands::Command;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct DynamicControlsKey {
    document_identity: Arc<()>,
    session_mutation_token: u64,
    platform: ShortcutPlatform,
    accelerator_owner: MenuAcceleratorOwner,
}

impl PartialEq for DynamicControlsKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document_identity, &other.document_identity)
            && self.session_mutation_token == other.session_mutation_token
            && self.platform == other.platform
            && self.accelerator_owner == other.accelerator_owner
    }
}

impl Eq for DynamicControlsKey {}

/// Count-only summary for diagnostics; it intentionally contains no raw chord,
/// definition label, source value, or screenplay content.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DynamicIssueSummary {
    malformed: usize,
    unsupported: usize,
    reserved: usize,
    conflicts: usize,
    suppressed: usize,
}

impl DynamicIssueSummary {
    fn message(self) -> String {
        format!(
            "ELEMENT_SHORTCUTS: disabled bindings (malformed {}, unsupported {}, reserved {}, conflicts {}, additional issues suppressed {})",
            self.malformed, self.unsupported, self.reserved, self.conflicts, self.suppressed
        )
    }
}

/// One immutable projection from a single current Project and input-ownership key.
#[derive(Debug, Clone)]
pub(crate) struct DynamicElementControls {
    key: DynamicControlsKey,
    pub(crate) choices: Arc<[ElementChoice]>,
    pub(crate) shortcuts: ResolvedElementShortcuts,
    issue_summary: Option<DynamicIssueSummary>,
}

impl DynamicElementControls {
    fn new(
        app: &ScriptCraftApp,
        platform: ShortcutPlatform,
        accelerator_owner: MenuAcceleratorOwner,
    ) -> Self {
        let project = app.session.project();
        let reserved = super::menu::reserved_element_chords(platform);
        let shortcuts =
            resolve_element_shortcuts(project.element_definitions(), platform, &reserved);
        let issue_summary = summarize_issues(&shortcuts);
        Self {
            key: DynamicControlsKey {
                document_identity: Arc::clone(project.layout_identity()),
                session_mutation_token: app.session.mutation_token(),
                platform,
                accelerator_owner,
            },
            choices: ordered_element_choices(project.element_definitions()).into(),
            shortcuts,
            issue_summary,
        }
    }

    fn matches(
        &self,
        app: &ScriptCraftApp,
        platform: ShortcutPlatform,
        accelerator_owner: MenuAcceleratorOwner,
    ) -> bool {
        self.key
            == DynamicControlsKey {
                document_identity: Arc::clone(app.session.project().layout_identity()),
                session_mutation_token: app.session.mutation_token(),
                platform,
                accelerator_owner,
            }
    }
}

fn summarize_issues(shortcuts: &ResolvedElementShortcuts) -> Option<DynamicIssueSummary> {
    if shortcuts.issues.is_empty() && shortcuts.suppressed_issue_count == 0 {
        return None;
    }
    let mut summary = DynamicIssueSummary {
        suppressed: shortcuts.suppressed_issue_count,
        ..DynamicIssueSummary::default()
    };
    for issue in &shortcuts.issues {
        match issue.kind {
            ElementShortcutIssueKind::Malformed => summary.malformed += 1,
            ElementShortcutIssueKind::Unsupported => summary.unsupported += 1,
            ElementShortcutIssueKind::Reserved => summary.reserved += 1,
            ElementShortcutIssueKind::Conflict => summary.conflicts += 1,
        }
    }
    Some(summary)
}

impl ScriptCraftApp {
    /// Refreshes atomically on document identity, mutation, platform, or menu-owner
    /// changes. Call before chrome composition and focused input routing.
    pub(crate) fn refresh_dynamic_controls(&mut self) -> &DynamicElementControls {
        let platform = ShortcutPlatform::current();
        let accelerator_owner = self.menu_policy.accelerator_owner();
        let stale = self
            .dynamic_controls
            .as_ref()
            .is_none_or(|controls| !controls.matches(self, platform, accelerator_owner));
        if stale {
            let controls = DynamicElementControls::new(self, platform, accelerator_owner);
            self.dynamic_controls = Some(controls);
        }
        self.report_dynamic_control_issues();
        self.dynamic_controls
            .as_ref()
            .expect("dynamic controls are installed before use")
    }

    pub(crate) fn invalidate_dynamic_controls(&mut self) {
        self.dynamic_controls = None;
        self.reported_dynamic_issue_summary = None;
    }

    fn report_dynamic_control_issues(&mut self) {
        let Some(summary) = self
            .dynamic_controls
            .as_ref()
            .and_then(|controls| controls.issue_summary)
        else {
            self.reported_dynamic_issue_summary = None;
            self.clear_stale_dynamic_issue_diagnostic();
            return;
        };

        let dynamic_message_visible = self
            .diagnostic_message
            .as_ref()
            .is_some_and(|message| message.as_str().starts_with("ELEMENT_SHORTCUTS:"));
        if dynamic_message_visible && self.reported_dynamic_issue_summary == Some(summary) {
            return;
        }
        if self.reported_dynamic_issue_summary == Some(summary) {
            return;
        }

        // Do not replace an unrelated operation or migration warning. A pending
        // diagnostic is allowed to drain first; the next refresh retries once.
        let unrelated_diagnostic_visible =
            self.diagnostic_message.is_some() && !dynamic_message_visible;
        let pending_diagnostic = self.pending_diagnostic_commands.iter().any(|command| {
            matches!(command, Command::UiStatusSet { message: Some(message) }
                if !message.as_str().starts_with("ELEMENT_SHORTCUTS:"))
        });
        if unrelated_diagnostic_visible || pending_diagnostic {
            return;
        }

        self.pending_diagnostic_commands.retain(|command| {
            !matches!(command, Command::UiStatusSet { message: Some(message) }
                if message.as_str().starts_with("ELEMENT_SHORTCUTS:"))
        });
        self.post_diagnostic(&summary.message());
        self.reported_dynamic_issue_summary = Some(summary);
    }

    fn clear_stale_dynamic_issue_diagnostic(&mut self) {
        let dynamic_message_visible = self
            .diagnostic_message
            .as_ref()
            .is_some_and(|message| message.as_str().starts_with("ELEMENT_SHORTCUTS:"));
        let unrelated_pending = self.pending_diagnostic_commands.iter().any(|command| {
            matches!(command, Command::UiStatusSet { message: Some(message) }
                if !message.as_str().starts_with("ELEMENT_SHORTCUTS:"))
        });
        if unrelated_pending {
            return;
        }
        let already_clearing = self
            .pending_diagnostic_commands
            .iter()
            .any(|command| matches!(command, Command::UiStatusSet { message: None }));
        self.pending_diagnostic_commands.retain(|command| {
            !matches!(command, Command::UiStatusSet { message: Some(message) }
                if message.as_str().starts_with("ELEMENT_SHORTCUTS:"))
        });
        if dynamic_message_visible && !already_clearing {
            self.pending_diagnostic_commands
                .push_back(Command::UiStatusSet { message: None });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_summary_is_count_only_and_coarse() {
        let summary = DynamicIssueSummary {
            malformed: 2,
            unsupported: 1,
            reserved: 3,
            conflicts: 4,
            suppressed: 7,
        }
        .message();
        assert!(summary.starts_with("ELEMENT_SHORTCUTS:"));
        assert!(summary.contains("malformed 2"));
        assert!(!summary.contains("Lyrics"));
        assert!(!summary.contains("Ctrl+"));
    }

    #[test]
    fn document_identity_is_pointer_identity_not_revision_or_token() {
        let first = Arc::new(());
        let clone = Arc::clone(&first);
        let replacement = Arc::new(());
        assert_eq!(Arc::as_ptr(&first), Arc::as_ptr(&clone));
        assert_ne!(Arc::as_ptr(&first), Arc::as_ptr(&replacement));

        let initial = DynamicControlsKey {
            document_identity: Arc::clone(&first),
            session_mutation_token: 4,
            platform: ShortcutPlatform::MacOS,
            accelerator_owner: MenuAcceleratorOwner::NativeMacOS,
        };
        assert_ne!(
            initial,
            DynamicControlsKey {
                document_identity: Arc::clone(&replacement),
                ..initial.clone()
            }
        );
        assert_ne!(
            initial,
            DynamicControlsKey {
                session_mutation_token: 5,
                ..initial.clone()
            }
        );
        assert_ne!(
            initial,
            DynamicControlsKey {
                platform: ShortcutPlatform::Windows,
                ..initial.clone()
            }
        );
        assert_ne!(
            initial,
            DynamicControlsKey {
                accelerator_owner: MenuAcceleratorOwner::InWindow,
                ..initial.clone()
            }
        );
    }
}
