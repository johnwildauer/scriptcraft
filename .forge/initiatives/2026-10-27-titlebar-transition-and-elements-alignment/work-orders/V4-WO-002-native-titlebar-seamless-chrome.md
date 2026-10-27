# Proposed Work Order: V4-WO-002 — Seamless Native Titlebar Chrome

## Metadata

- **Status:** `draft` — proposal only; not approved for implementation or admitted to the current v4 execution sequence.
- **Proposed owner:** Fresh desktop implementer, after successor-initiative design review and explicit human approval.
- **Proposed order:** One bounded follow-up work order dependent on the completed native baseline evidence in [`V4-WO-001`](V4-WO-001-vanilla-native-titlebar-comparison.md:1).
- **Traceability:** This candidate is linked from the v4 plan but represents a distinct follow-up outcome. The current v4 phase is `execute`; do not dispatch it from v4 or infer a phase rewind. See the process boundary in [`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:121).
- **Validation:** A separate fresh validator must assess this order only after a future approved implementation and recorded visual-gate evidence.

## Summary and single outcome

On the ordinary native macOS window path, remove visible native title text and make the titlebar/content background appear as one seamless app surface, while retaining visible and usable native traffic-light controls, a safe usable app toolbar, native maximize/restore behavior, and the V4-WO-001 observation that neither maximize nor restore swells, shrinks, or snaps. This is a visual integration proposal, not a guarantee that installed APIs or macOS produce the requested appearance.

## In scope

- Freshly inspect the actually resolved eframe, egui-winit, and winit versions and their source before implementation. Record exact supported APIs, macOS mapping, decoration-order semantics, and any platform/runtime constraints before editing.
- If the resolved API audit confirms the present 0.29.1 stack as recorded, evaluate only this candidate viewport configuration: retain native decorations with [`ViewportBuilder::with_decorations(true)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:330) and an opaque overall window with [`ViewportBuilder::with_transparent(false)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:384); request hidden title text with [`ViewportBuilder::with_title_shown(false)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:437); request a transparent native titlebar with [`ViewportBuilder::with_titlebar_shown(false)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:451); request content behind it with [`ViewportBuilder::with_fullsize_content_view(true)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:430); and explicitly keep native buttons shown with [`ViewportBuilder::with_titlebar_buttons_shown(true)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:444). Do not substitute whole-window transparency for the macOS titlebar-transparency hint.
- Interpret the installed API semantics precisely: egui-winit 0.29.1 maps [`title_shown`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1592) to winit's [`with_title_hidden`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/macos.rs:286), maps [`titlebar_buttons_shown`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1593) to [`with_titlebar_buttons_hidden`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/macos.rs:290), maps [`titlebar_shown(false)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1737) to [`with_titlebar_transparent(true)`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/macos.rs:284), and forwards [`fullsize_content_view`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1591) to winit ([resolved mapping](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1731); [winit macOS extension semantics](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/macos.rs:271)). Winit documents that changing decorations overwrites titlebar-related properties; audit and apply options consistently rather than assuming call order or field names imply actual appearance ([`WindowAttributesExtMacOS`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/macos.rs:273)). These mappings establish request support only, not actual macOS rendering.
- Keep the selected native mode free of app-owned maximize commands and app-owned transition behavior. Do not add a custom titlebar double-click response, send a maximize viewport command from native chrome, or route native interactions through a custom transition policy. The existing [`CustomHidden` rollback implementation](../../../../apps/scriptcraft/src/main.rs:53) may remain untouched but must not be selected for this outcome.
- Use the existing app-owned chrome fill and clear-color contract as the content surface behind the transparent native titlebar. Make only the minimum native-mode chrome/layout adjustment needed to keep toolbar controls below and clear of traffic lights and native titlebar hit regions; do not add a fake titlebar, overlay controls, or a new geometry/transition policy.
- Preserve the validated black-bar/clear-color/restored-padding result, toolbar allocation/overflow and `128.0` Elements width, and existing Elements, popup, cursor, input/IME, accessibility/semantic, and page-content behavior.
- Obtain direct user visual evidence on a clean rebuilt macOS app; automated configuration tests cannot substitute for this gate.

## Out of scope

- Any app-owned maximize/restore handler, custom transition animation/policy, transition metric hold/release behavior, polling, global zoom, or custom overlay used to imitate native chrome.
- Hiding or replacing native traffic-light controls; whole-window borderless mode; whole-window transparency as a workaround; speculative native APIs not present in the resolved dependency source; or asserting an API request guarantees seamless appearance.
- Changes to the native maximize/restore mechanics, unrelated toolbar/content geometry, toolbar behavior, Elements width/behavior, black-bar/padding ownership, editing semantics, or dependency versions.
- Permanent product/design assumptions beyond the approved follow-up outcome; unrelated cleanup; changes to v1/v2/v3/history or the V4-WO-001 implementation/validation/evidence.
- Any implementation, validation, review artifact, execution-log entry, requirement/blueprint edit, or Git/branch operation during this proposal step.

## Requirements

Applicable durable text from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

> **REQ-WRITING-LOOP-005: Desktop command toolbar**
>
> **User Story:** As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.

- [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115) remains applicable: Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.
- [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) is the durable user-facing titlebar contract implicated by this proposal: Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support. The proposed seamless native outcome changes this visible title-strip contract and must not be treated as implementation-authorized until the successor design review and human approval resolve it.
- [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137) remains applicable: Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.

## Blueprint

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) is the affected technical contract. It currently describes native presentation requests and a unified app-owned title-strip/toolbar surface. Reconcile that description only after the proposed native ownership, hidden title, transparent titlebar, full-size content, app-surface fill, and toolbar-safe boundary are implemented and visually verified.

## Implementation guidance and acceptance boundary

1. Before edits, inspect the exact locked versions and installed APIs again. If any candidate option is absent, ignored, has incompatible decoration semantics, or cannot preserve the traffic lights/native window behavior, stop and record `blocked` with source evidence. Do not replace the unsupported behavior with custom maximize handling.
2. Implement only the smallest startup/chrome change for the selected native path. Keep the product runner identity, native decorations, native buttons, native double-click/green-button behavior, opaque overall window, existing app-surface clear color, and selected native mode. Do not add later option overrides or another diagnostics mode.
3. Add focused deterministic tests for selected native options and their explicit values, native-mode absence of app-owned maximize/transition handling, no later startup override, toolbar-safe layout invariants available to the app, and preservation of named chrome/content contracts. State clearly which properties require live macOS verification.
4. After a clean rebuild, require the user to inspect the actual window in both restored and maximized states, including the full maximize and restore animations. The user gate passes only if all criteria below pass:
   - native close/minimize/green traffic-light controls remain visible and usable;
   - no `ScriptCraft` title text is visible beside the native controls;
   - native titlebar and app content background read visually as one seamless surface, without a distinct native color band or seam;
   - toolbar is usable, legible, and safely clear of the traffic lights/titlebar control region in both restored and maximized states;
   - native maximize and restore both complete without UI swelling, shrinking, or a final snap, preserving the V4-WO-001 user-observed result.
5. Any failed criterion is a failed visual acceptance gate. If the dependency/OS behavior is unavailable or the desired outcome cannot be achieved without a prohibited custom maximize path, mark the work blocked/failed with exact evidence and stop; do not claim partial success or silently relax the gate. A fresh validator follows only after the approved implementation, required checks, and complete user-gate evidence exist.

## Files and systems likely to change after approval

- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and focused tests colocated with native startup/chrome behavior; this work order and the future successor initiative's execution log only during authorized execution.
- **Potential authorized durable-document updates after review/approval and successful visual verification:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105), limited to the titlebar ownership/presentation contract and toolbar-safe native content boundary.
- **Inspect read-only:** [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:1), [`Cargo.lock`](../../../../Cargo.lock:1), and the exact installed eframe/egui-winit/winit source/API documentation.
- **Avoid changing:** all other application/core source and tests, dependency manifests/lockfile, V4-WO-001 and its validation, execution-log history, v1/v2/v3/history, unrelated requirements/blueprints, toolbar semantics/width, black-bar ownership/padding/clear color, Elements behavior, and unrelated transition code.

## Required checks after future approved implementation

- Focused native-options/ownership/layout-preservation tests.
- [`cargo clean`](../../../project-manifest.md:80).
- [`cargo fmt --all -- --check`](../../../project-manifest.md:81).
- [`cargo check --workspace`](../../../project-manifest.md:84).
- [`cargo test --workspace`](../../../project-manifest.md:83).
- [`cargo build --workspace`](../../../project-manifest.md:85).
- [`cargo clippy --workspace --all-targets -- -D warnings`](../../../project-manifest.md:82).
- [`git diff --check`](../execution-log.md:439).
- Record exact outputs and failures; none of these automated checks establishes title text absence, actual titlebar/content color blending, toolbar safety against native controls, or animation stability. The complete user visual gate above is mandatory.

## Durable-document impact

- **Requirements impact:** `update` — narrowly revise [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) if and only if the successor design is approved and the tested result is adopted, because the current requirement explicitly promises an app-owned title strip and app-owned double-click maximize response while this outcome proposes hidden native title text and native-owned maximize/restore. No requirements edit is authorized now.
- **Blueprint impact:** `update` — reconcile [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) if and only if approved and verified, because native titlebar transparency/full-size-content and chrome ownership/safe toolbar placement alter its technical description. No blueprint edit is authorized now.
- **Evidence/rationale:** V4-WO-001's user result establishes native chrome usability and stable native maximize/restore but also records visible title text and a distinct native bar color. The current blueprint and requirement were inspected; neither has been changed.

## Proposal state and gates

- This file is a draft proposal only. It does not amend V4-WO-001, alter v4 approval, create an implementation authorization, or satisfy any review gate.
- The current v4 phase sequence does not permit silently adding this distinct outcome to the completed work-order lane. Before formal design review, carry this candidate into one successor initiative after v4 closeout; then obtain a fresh-context design review and a separate explicit human approval. Only that approval can authorize implementation.
- No review, validation, implementation, source edit, durable-document update, execution-log entry, or Git/branch action has occurred for this proposal.
