# Initiative: Cursor Reveal and Paste Safety

## Initiative metadata

- **Phase:** `plan`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-25
- **Updated:** 2026-10-25
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode is the project default; the user reported concrete interaction and paste-validity regressions after the previous geometry initiative.
- **Rapid-mode eligibility:** `not eligible` — the work crosses transient viewport policy, semantic paste behavior, model validation, and long-document safety.
- **Base/default branch:** `master`
- **Initiative branch:** `dev/2026-10-25-cursor-reveal-and-paste-safety`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug`
- **Branch capability/evidence:** `git switch -c dev/2026-10-25-cursor-reveal-and-paste-safety` succeeded on 2026-10-25.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan` on 2026-10-25: the user reported unwanted cursor-centering during navigation/scrolling and recurring `Model(Invalid)` errors after paste; this plan and two ordered work orders were created.
- **Direct-translation confirmation:** `not needed` — work orders are a focused translation of the reported behavior, with reproduction and implementation decisions intentionally gated.

## Standard artifact locations

- **Authoritative plan:** `cursor-reveal-and-paste-safety-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

Make cursor visibility feel natural—scrolling and selection do not move or center the caret, while edits and navigation reveal it only when it would otherwise leave the viewport—and make paste handling safe and diagnosable for large, multi-line, whitespace-heavy content without invalidating the working model.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** Dependable screenplay writing loop in [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md).
- **Initiative type:** `product MVP stage`
- **Single observable outcome:** A writer can scroll freely without the view recentering around the caret, then type/navigate/paste through multi-line content with conditional caret reveal and a valid or clearly bounded recoverable model state.
- **Validation boundary:** Fresh design review, separate fresh validation per WO, headless event/selection tests, paste fixtures covering newline/space/size dimensions, and a focused native smoke when interaction is available.
- **Split assessment:** `keep as one initiative`
- **If split, traceability and dependencies:** Not applicable. WO-001 is a transient viewport policy correction; WO-002 is a model/input safety investigation. They share the editor event boundary but remain independently testable and sequential.

## Scope

### In scope

- Conditional caret reveal based on whether the post-action caret is outside the currently visible editor viewport, without centering it by default.
- Distinguishing user scrolling and pointer selection from commands that mutate/move the caret and should request conditional reveal.
- Reproducing `Model(Invalid)` after paste across size, newline convention, repeated blank lines, whitespace, Unicode, and element/profile dimensions.
- Determining whether paste should remain one semantic element with embedded line breaks or become multiple screenplay elements, and making the chosen behavior explicit and safe.
- Preserving last-valid session/model state when paste, layout, serialization, or validation fails.

### Out of scope

- General clipboard format support, rich-text/HTML/RTF import, arbitrary paste formatting, collaboration, or new screenplay roles.
- Making scrolling itself move the caret, persisting viewport/caret coordinates, or adding an override editor.
- Raising safety limits without reproducing the actual failure and recording bounded allocation reasoning.

## Context and motivation

The current app stores a transient `reveal_caret` flag and sets it in [`mutate()`](../../apps/scriptcraft/src/main.rs:182) after every successful editing-session operation. The render loop then calls [`Ui::scroll_to_rect()`](../../apps/scriptcraft/src/main.rs:432) with [`egui::Align::Center`](../../apps/scriptcraft/src/main.rs:432). As a result, arrow navigation and other successful commands can center the caret even when it is already visible; a pending reveal can also be consumed on a later frame in a way that conflicts with user scrolling. The current behavior does not distinguish “caret changed” from “caret needs revealing.”

Paste events are sent directly to [`EditingSession::insert()`](../../apps/scriptcraft/src/main.rs:525), which inserts the entire string into the current semantic element. Newlines are accepted as text and later interpreted by layout's [`wrap_element()`](../../crates/scriptcraft-core/src/layout.rs:365); they are not currently split into separate screenplay elements. The edit path validates the project before committing, and JSON serialization separately enforces [`MAX_JSON_BYTES`](../../crates/scriptcraft-core/src/lib.rs:15). Therefore spaces and multiple newlines are not inherently invalid JSON/model input; the reported `Model(Invalid)` must be reproduced and traced rather than assumed to be a newline parser defect.

## Exploration scratchpad

### Initial findings

- **Confirmed reveal-policy cause:** every successful `mutate()` call sets `reveal_caret = true`, including arrow movement, role changes, delete/backspace, Enter/Tab, undo/redo, and text insertion. The next layout pass centers the caret rectangle instead of minimally revealing it.
- **Pointer/scroll boundary:** pointer selection updates the session selection directly through [`apply_pointer_selection()`](../../apps/scriptcraft/src/main.rs:217) and does not itself request reveal, but a previously queued reveal flag can still be consumed after the pointer/scroll state changes.
- **Paste semantic behavior:** paste is a single `insert(text)` operation. Embedded `\n`, `\r\n`, spaces, and tabs are stored in one element's canonical text unless a later explicit operation splits it. Layout handles newline delimiters for visual lines, not semantic element creation.
- **Model validity boundary:** `EditingSession::mutate()` clones, applies, validates, and only then commits. A failed insert should leave the prior session intact. The exact user-facing `Model(Invalid)` source may be a layout/model error rendered after the edit, a stale error from a prior operation, a serialized profile/schema mismatch, or a size/boundary failure; the next WO must capture the exact error variant and triggering input.
- **Existing bounds:** per-element text and total model/JSON sizes are bounded. A large paste can legitimately fail with a typed size error, but it must not leave a corrupt or unusable model and the UI should distinguish this from invalid JSON/model data.

### Initial thoughts

1. **Cursor reveal:** replace “always center when caret changed” with “request reveal after a caret-affecting command, then scroll only if the caret rectangle is outside the visible clip rectangle.” Use minimal/nearest scrolling rather than center alignment. Pointer selection and passive scrolling must clear or supersede pending reveal intent. Large paste should request reveal once after the paste operation, not once per line.
2. **Paste safety:** first create a matrix of input fixtures and record whether failure occurs in edit validation, layout construction, serialization, or UI error presentation. Do not preemptively split newlines into elements: that changes screenplay semantics and role assignment. If multi-line paste needs semantic splitting, make it an explicit policy with role inheritance/transition rules and one transactional undo step.
3. **Error recovery:** preserve the last valid session on every failed paste/layout/serialization path, display a specific bounded error, and ensure the next valid edit remains possible. The phrase `Model(Invalid)` is not sufficient evidence by itself; the underlying error and input dimensions must be logged in tests.

### Risks and decisions

- **Risk:** Minimal reveal logic can oscillate near viewport edges or fail when page geometry changes after paste. **Mitigation:** compute post-action caret geometry after layout, compare against the actual clip rect, and test top/bottom/partially visible cases.
- **Risk:** Splitting pasted newlines into elements could unexpectedly assign screenplay roles or alter one-paste undo semantics. **Mitigation:** default to preserving canonical text until reproduction or an explicit product decision requires semantic splitting.
- **Risk:** Treating every paste failure as a model corruption bug could weaken bounds. **Mitigation:** classify exact error variants and preserve typed limits.
- **Decision needed:** Whether pasted line breaks should remain embedded in the current element or become multiple elements after the reproduction matrix. Owner: user/product owner; trigger: WO-002 design review.

## Requirements and blueprint references

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md) — `AC-WRITING-001.3`, `AC-WRITING-001.5`, `AC-WRITING-002.3`, `AC-WRITING-002.4`, `NFR-WRITING-001`, and `NFR-WRITING-002`.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md), [`screenplay-core.md`](../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md), and [`technical-context.md`](../../blueprints/technical-context.md).
- **Research:** [`04-command-session-ui-boundary.md`](../../.analysis/stage2/04-command-session-ui-boundary.md), [`05-testing-safety-portability.md`](../../.analysis/stage2/05-testing-safety-portability.md), and [`07-wordcraft-gui-system.md`](../../.analysis/07-wordcraft-gui-system.md).

## Proposed work-order plan

1. [`WO-001-conditional-caret-reveal.md`](work-orders/WO-001-conditional-caret-reveal.md) — separate caret-affecting command intent from passive scrolling/pointer selection and reveal only when the caret is outside the viewport.
2. [`WO-002-paste-model-validity.md`](work-orders/WO-002-paste-model-validity.md) — reproduce the paste/`Model(Invalid)` failure matrix and implement the smallest bounded safety/semantic correction.

WO-001 is UI policy and should execute first. WO-002 depends on the current event boundary but can be headless-tested independently; sequential execution keeps the paste diagnosis separate from viewport behavior.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The advisory split assessment is recorded.
- [x] Requirements and blueprints are linked.
- [x] Work orders are independently executable and dependency-ordered.
- [x] Every work order carries a durable-document impact classification.
- [x] Initiative mode metadata and branch decision are recorded.
- [x] Validation cadence and policy gates are identified.
- [ ] Fresh design review has passed.
- [ ] Required human design approval has been obtained.

