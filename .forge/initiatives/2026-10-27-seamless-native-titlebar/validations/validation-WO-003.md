# Independent Validation — WO-003 Titlebar Experiment Residue Cleanup

## Disposition

**PASS — independent validation only.** The approved cleanup's focused and workspace verification passed; retired-symbol and direct-dependency searches were clean; Cargo graph and reverse-tree checks confirm the removed direct dependencies remain available transitively; and the reviewed worktree had no drift before testing. No unexpected visible effect was found in the cleanup boundary. This report does not mark WO-003 complete. WO-003 remains `in-progress` pending its recorded process/state updates; no product files, work orders, historical reviews, or prior validation records were edited by this validator.

The separate implementation review is [`implementation-review-WO-003-final.md`](../reviews/implementation-review-WO-003-final.md:1). Earlier review reports remain historical and untouched. The approved scope and implementation evidence are in [`WO-003-titlebar-experiment-residue-cleanup.md`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:1).

## Fresh Git snapshot, drift, and attribution

The independent snapshot was taken before verification and repeated after all checks, immediately before writing this report:

- **Branch:** `dev/2026-10-27-seamless-native-titlebar`.
- **HEAD:** `da651983a9bdc87439237f032700bbcf4fa1981a`.
- **Staged:** empty at both snapshots.
- **Unstaged tracked paths:**
  - Modified: `.forge/initiatives/2026-10-27-seamless-native-titlebar/execution-log.md`, `.forge/initiatives/2026-10-27-seamless-native-titlebar/seamless-native-titlebar-v1.md`, `.forge/initiatives/2026-10-27-seamless-native-titlebar/work-orders/WO-002a-native-transition-scaling-mitigation.md`, `Cargo.lock`, `apps/scriptcraft/Cargo.toml`, and `apps/scriptcraft/src/main.rs`.
  - Deleted: `apps/scriptcraft/src/platform/macos_titlebar.rs`.
- **Untracked before this report:** the four historical WO-003 implementation review reports, [`WO-002b-restore-native-macos-titlebar.md`](../work-orders/WO-002b-restore-native-macos-titlebar.md:1), and the approved WO-003 work order. No other untracked paths were present. This validation is the only new path created by this dispatch.
- **Tracked diff:** seven paths, 201 insertions and 1,472 deletions; same path set and totals as the fresh independent implementation review. The tracked state, untracked inventory, branch, HEAD, and staged state matched that review snapshot; there was no pre-test drift and no test-generated tracked/untracked change.

Attribution was checked against the approved order's preflight and implementation evidence and the separate review's fresh path inventory. `apps/scriptcraft/Cargo.toml`, `Cargo.lock`, and deletion of `apps/scriptcraft/src/platform/macos_titlebar.rs` are the cleanup's dependency/bridge changes. The large `main.rs` diff is cumulative: the ordinary native-header restoration and its already-attributed restoration code remain distinct from the WO-003 residue removal and narrow retained-test adjustments; the selected startup remains the ordinary visible, decorated, opaque native window with `fullsize_content_view(false)`. The initiative plan, execution log, and WO-002a record are Forge/history/status records with the attribution recorded by the implementation evidence and independent review; they do not represent additional product behavior changes. No unrelated or unexplained change was found.

Untracked historical review reports and the WO-002b order were preserved. The WO-002 validation file was not in the changed-path list and still records outcome `fail` and required state `blocked` ([`validation-WO-002.md`](../validations/validation-WO-002.md:49)); WO-002a still records its unsuccessful attempt and `blocked` disposition ([`WO-002a-native-transition-scaling-mitigation.md`](../work-orders/WO-002a-native-transition-scaling-mitigation.md:120)). Neither earlier disposition was amended.

## Verification results

All commands below were run from the repository root. Exit status `0` denotes success.

### Focused checks

| Exact command | Exit | Result |
|---|---:|---|
| `cargo fmt --all -- --check` | 0 | Formatting check passed. |
| `cargo test -p scriptcraft native_startup_selects_the_single_locked_viewport_tuple` | 0 | 1 passed, 0 failed. |
| `cargo test -p scriptcraft toolbar_starts_at_content_boundary_and_preserves_full_width_and_height` | 0 | 1 passed, 0 failed. |
| `cargo test -p scriptcraft toolbar_chrome_uses_open_widget_fill_without_mutating_style` | 0 | 1 passed, 0 failed. |
| `cargo test -p scriptcraft restored_toolbar_padding_stays_inside_the_exact_panel_boundary` | 0 | 1 passed, 0 failed. |
| `cargo test -p scriptcraft viewport_clear_color_matches_the_chrome_surface_owner` | 0 | 1 passed, 0 failed. |
| `cargo test -p scriptcraft presentation_metrics` | 0 | 2 passed, 0 failed. |

Focused test total: **7 passed, 0 failed** across the six filtered test invocations. The native-startup and toolbar-boundary regression tests, toolbar fill/padding/clear-color checks, and page-presentation metric checks all passed.

### Workspace checks

| Exact command | Exit | Result |
|---|---:|---|
| `cargo check --workspace` | 0 | Passed. |
| `cargo test --workspace` | 0 | **159 passed, 0 failed**; 77 app unit tests, 45 core unit tests, 37 integration tests, and 0 doc tests. |
| `cargo build --workspace` | 0 | Passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Passed with warnings denied. |
| `git diff --check` | 0 | Passed; repeated after verification with the same result. |

The recorded test suites were: app unit 77; core unit 45; integrations 2, 5, 4, 6, 2, 10, 5, and 3 (37 total); doc tests 0. All reported zero failures.

### Retired-symbol and direct-dependency searches

The shell environment does not provide `rg` (`rg` invocations exited 127). The same audits were therefore performed with `git grep`; both completed with the expected no-match status (grep exit 1, explicitly treated as no matches):

- `git grep -n -E 'perform_zoom|performZoom:|native_double_click_excludes_native_control_bounds_and_hit_slop|TitlebarPresentationMode|CustomHidden|PresentationTransitionPolicy|title_strip\(|title_strip_viewport_command|toolbar_top_inset_for_mode|ViewportDiagnostics|ViewportMetricSample|ViewportCommandObservation|ChromeSurfaceLedger|PhysicalPixelSpan|logical_y_to_physical_rows|chrome_surface_ledger|bounded_push|record_geometry|record_toolbar_bounds|record_chrome_ledger|record_presentation|record_command|finish_frame|titlebar_mode_selects_custom_surface_and_policy_explicitly|title_strip_uses_exact_height_and_centered_placeholder_text|title_strip_viewport_toggle_is_safe_for_known_and_unknown_state|transition_policy_holds_combined_metrics_and_releases_without_a_snap|transition_policy_unknown_state_recovers_boundedly_and_repeated_requests_coalesce|transition_policy_opposite_request_rebases_once_without_stale_capture|shared_chrome_geometry_has_no_gap_and_full_width_title_hit_region|toolbar_bottom_ownership_ledger_is_contiguous_and_pixel_bounded|chrome_fill_owns_former_strip_rows_at_each_representative_scale|physical_pixel_row_conversion_expands_fractional_edges_deterministically|viewport_diagnostics_are_bounded_and_preserve_latest_samples' -- apps/scriptcraft/src crates`
- `git grep -n -E 'raw[-_]window[-_]handle|objc2[-_]app[-_]kit|\bobjc2\b' -- apps/scriptcraft/src apps/scriptcraft/Cargo.toml crates Cargo.toml`

Both searches returned **no matches** in app/core source or applicable manifests, including no direct use or declaration of the retired bridge-only macOS dependencies.

### Cargo dependency graph and reverse-tree checks

`rustup target list --installed` exited 0 and listed `aarch64-apple-darwin`; the macOS-target graph checks were therefore available and run. Every command below exited 0:

- Graphs: `cargo tree -p scriptcraft -e normal`; `cargo tree -p scriptcraft --target aarch64-apple-darwin -e normal`.
- Host reverse trees: `cargo tree -p scriptcraft -i raw-window-handle@0.6.2`; `cargo tree -p scriptcraft -i objc2@0.6.5`; `cargo tree -p scriptcraft -i objc2-app-kit@0.3.2`; `cargo tree -p scriptcraft -i objc2@0.5.3`; `cargo tree -p scriptcraft -i objc2-app-kit@0.2.2`.
- macOS-target reverse trees: `cargo tree -p scriptcraft --target aarch64-apple-darwin -i raw-window-handle@0.6.2`; `cargo tree -p scriptcraft --target aarch64-apple-darwin -i objc2@0.6.5`; `cargo tree -p scriptcraft --target aarch64-apple-darwin -i objc2-app-kit@0.3.2`.

The reverse trees show the removed `raw-window-handle 0.6.2`, `objc2 0.6.5`, and `objc2-app-kit 0.3.2` remain reachable through GUI/windowing and dialog dependencies, including `eframe`/`winit` and `rfd`. The host graph also retains transitive `objc2 0.5.3` and `objc2-app-kit 0.2.2` through GUI dependencies. Inspection of the manifest and lockfile diff confirms the only dependency changes are removal of the three direct `scriptcraft` edges; their package records/versions and unrelated lock entries are retained.

## Visible-behavior assessment and final state

The current native startup and toolbar-focused assertions passed; source/reference searches found no surviving custom title-strip, app-owned maximize/transition, or experiment-ledger paths. Review of the cleanup attribution shows only dead/inactive experiment code, experiment-only tests/diagnostics, and validated direct dependency edges were removed. The native startup/toolbar restoration changes in the cumulative diff are attributed to the earlier WO-002b baseline, not newly introduced by this cleanup. No unexpected visible effect was found. Under the approved cleanup's conditional visual-gate rule, no application launch or new user visual gate was required; none was performed, and no visual observation is claimed.

**Validation outcome: PASS for this independent validation only.** WO-003 remains `in-progress`; this report does not declare it complete. No staging, commit, reset, or product edit was performed. WO-002 remains `blocked` with its original fail validation, and WO-002a remains `blocked`.
