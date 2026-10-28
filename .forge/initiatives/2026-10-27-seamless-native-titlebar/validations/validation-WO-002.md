# Validation: WO-002 — Native Titlebar Double-Click Action

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete — disposition recorded; outcome fail`
- **Validator:** `separate fresh-context WO-002 validator`
- **Validator session/agent ID:** `not exposed by the execution interface`
- **Session exception:** `none`
- **Date:** `2026-10-28`
- **Initiative:** [`seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md)
- **Validation file:** `validation-WO-002.md`
- **Validated work order:** [`WO-002-native-titlebar-double-click.md`](../work-orders/WO-002-native-titlebar-double-click.md)
- **Compared commit/diff:** Current worktree on `dev/2026-10-27-seamless-native-titlebar`; inspected the tracked diff across [`main.rs`](../../../../apps/scriptcraft/src/main.rs), [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml), and [`Cargo.lock`](../../../../Cargo.lock), and directly inspected the untracked bridge. WO-002 records pre-edit `HEAD` as `65b1de6bc2ee2b87625a9ce543769e1920ee45fc`. Tracked diff stat is 354 insertions / 19 deletions, including the separately recorded WO-001 baseline in [`main.rs`](../../../../apps/scriptcraft/src/main.rs) (91 insertions / 15 deletions); the new bridge and Forge artifacts are untracked and are not in that stat.
- **Fix chain:** `none`

## Authorized scope

Compared only the current performZoom-first WO-002 boundary: a native/macOS-only primary double-click in the existing reserved 28-point band, routed synchronously through the current window's public [`performZoom:` action](../work-orders/WO-002-native-titlebar-double-click.md#implementation-plan-and-safety-boundary); the specified live-handle, AppKit main-thread/lifetime, native-control, popup/menu, and toolbar exclusions; no app-owned window-state/transition policy or alternate action; preservation of existing appearance, geometry, content, and stable native transitions; required automated checks; and the implementation-time actual-window user gate.

The comparison basis is the WO-002 changed-file record and implementation evidence, the user-directed review disposition (a one-time user bypass, not an independent review), the [`validation-WO-001.md`](validation-WO-001.md) predecessor disposition, the execution log, the live tracked diffs, and direct inspection of the untracked bridge. At the time of this comparison, the user-requested scaling proposal was a separate draft then numbered WO-003; it was outside this validation and was not treated as a WO-002 implementation, remediation, or validation result. That draft was later reconciled, at the user's direction, as the letter-suffixed fix [`WO-002a-native-transition-scaling-mitigation.md`](../work-orders/WO-002a-native-transition-scaling-mitigation.md). This later traceability update does not alter the validation outcome, finding, or required next state. No source, initiative-plan, WO-001, or scaling-proposal content was changed by this validator. No tests, build, or app run were repeated by this validator.

## Validation checklist

- The reported work, changed-file inventory, recorded checks/user gate, and actual source/Cargo evidence support meaningful comparison.
- The implementation and Cargo changes are within WO-002's authorized scope; no clearly unrelated or rogue behavior was found.
- The exact user report fails one explicit native-transition preservation criterion. Other unitemized visual and interaction criteria remain unverified, not inferred as passes or failures.
- Required automated checks are recorded as passing by the implementer; this validator did not rerun them. They do not override the failed actual-window criterion.
- Conditional requirements/blueprint changes remain unchanged pending successful behavior adoption; the current failure does not contradict that condition.

## Findings

### Blockers

- **Stable native transition / no visual popping — Severity: blocker.** **Exact criterion:** preserve stable native maximize/restore animations (WO-002 summary and preservation scope), with the actual-window gate requiring stable native animations and no swelling, shrinking, or final snap. **Evidence:** the user report is verbatim: “performZoom runs, but i'm still getting the visual popping”. It is recorded in [`WO-002`](../work-orders/WO-002-native-titlebar-double-click.md:203) and [`execution-log.md`](../execution-log.md:181). This is direct failed evidence for the explicit stable-transition/popping-preservation criterion. The report does not identify whether maximize, restore, or both were observed, so no direction-specific result is inferred. **Recommendation:** retain this as a failed required gate; do not claim acceptance based on `performZoom` execution or passing automated checks. Keep WO-002 blocked and incomplete until a separately authorized bounded disposition addresses the failed criterion and receives fresh validation. **Required next state:** `blocked`.

### Warnings

- **Other actual-window criteria not itemized — Severity: warning.** Evidence: the user report only identifies action execution and visual popping. Native-control/toolbar/popup behavior, title suppression/surface continuity, content geometry, and separate maximize/restore observations were not individually reported; [`WO-002`](../work-orders/WO-002-native-titlebar-double-click.md:204) records the same evidence limit. Recommendation: retain each as unverified; infer neither pass nor failure. **Required next state:** `blocked` due to the blocker above.
- **Automated results are implementer-attributed — Severity: warning.** Evidence: [`WO-002`](../work-orders/WO-002-native-titlebar-double-click.md:185)–[`194`](../work-orders/WO-002-native-titlebar-double-click.md:194) and [`execution-log.md`](../execution-log.md:176) record the required clean Cargo sequence and focused checks as passing. Recommendation: preserve them as recorded evidence, not validator reruns. **Required next state:** `blocked` due to the blocker above.

### Scope comparison and notes

- **Source/Cargo scope:** The actual [`main.rs`](../../../../apps/scriptcraft/src/main.rs) diff contains the native-band double-click response, native-only routing, popup/layer ownership checks, and dispatch handoff. The directly inspected [`macos_titlebar.rs`](../../../../apps/scriptcraft/src/platform/macos_titlebar.rs:8) uses the live frame handle, checks the AppKit main thread, performs one isolated retain, resolves the current window, checks public native-button geometry, and calls only the public [`performZoom:` action](../work-orders/WO-002-native-titlebar-double-click.md#implementation-plan-and-safety-boundary); it does not cache window state or substitute fullscreen/frame emulation. The same [`main.rs`](../../../../apps/scriptcraft/src/main.rs) diff also contains the native viewport tuple and 28-point inset already present in the WO-001 baseline; WO-002's pre-edit evidence identifies that baseline, so those lines are not misattributed as WO-002 changes.
- [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:13) adds only the WO-authorized macOS-target direct dependencies at the already-resolved versions and narrowly replaces the inherited app lint configuration with package-local `unsafe_code = "deny"` plus the existing Clippy denies. The [`Cargo.lock`](../../../../Cargo.lock) diff adds only the three app dependency edges; no dependency versions changed. The root [`Cargo.toml`](../../../../Cargo.toml:13) and core policy are unchanged.
- **Changed implementation paths recorded for this dispatch:** [`main.rs`](../../../../apps/scriptcraft/src/main.rs), [`macos_titlebar.rs`](../../../../apps/scriptcraft/src/platform/macos_titlebar.rs), [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml), [`Cargo.lock`](../../../../Cargo.lock), [`WO-002`](../work-orders/WO-002-native-titlebar-double-click.md), and [`execution-log.md`](../execution-log.md). The compared source/Cargo changes are authorized; no unrelated or rogue behavior was found.
- No new fix work order was created in this validation. No source, requirements, blueprint, initiative outcome, WO-001 validation, or WO-003 artifact was modified. No staging, commit, merge, or closeout occurred.

## Decision

- **Outcome:** `fail`
- **Required next state:** `blocked` — WO-002 is not complete; the user-reported visual popping fails the explicit stable-transition preservation criterion. The other unitemized visual criteria remain unverified.
- **Fix work order required:** `no new fix work order created by this validation`; any bounded remediation requires its own recorded authorization/disposition and fresh validation before WO-002 can be completed.
