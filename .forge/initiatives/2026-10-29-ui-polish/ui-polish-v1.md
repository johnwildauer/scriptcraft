# Initiative: UI Polish

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** `2026-10-29`
- **Updated:** `2026-10-30`
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `project policy`
- **Mode confirmation:** Standard is the manifest default; the user did not request rapid mode.
- **Rapid-mode eligibility:** `not eligible` — this standard initiative spans multiple UI behaviors and nontrivial platform and integration risks.
- **Base/default branch:** `master` — confirmed by current Git inspection.
- **Initiative branch:** `dev/2026-10-29-ui-polish` — creation recorded below; current checkout must be reverified before dispatch.
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug` (manifest default)
- **Branch capability/evidence:** Local branch capability confirmed by successful `git switch -c dev/2026-10-29-ui-polish`; `git branch --show-current` confirmed this branch active.
- **Reduced-capability fallback or opt-out evidence:** Not applicable unless required branch creation fails; no opt-out requested.
- **Branch creation failure:** `none` — local branch creation succeeded.
- **Validation cadence:** `per work order` (manifest policy; WO-001–005 each passed separate independent validation; final combined batch verification/closeout remains pending).
- **Phase transition evidence:** Opened in `draft`; transitioned `draft → plan` after investigation and the user's subsequent explicit instruction to spawn architect agents to prepare work orders for all four requested groups. The user later explicitly authorized implementation of all WOs and directed the phase to `execute`; see the execution authorization record below. This direct decision is recorded separately from, and does not rewrite, the contemporaneous review artifact.
- **Direct-translation confirmation:** `confirmed with revised grouping` — the user instructed: “spawn an architect agent to write WO/s for each of the following tasks: scale/page geometry; status integration and appearance; sprint timer; Focus Mode”; allowed multiple WOs per group and batched durable-document updates. The earlier planning note about post-implementation regression testing is not evidence that the user accepted a testing responsibility; no user-owned test obligation is inferred.

## Standard artifact locations

- **Authoritative plan:** `ui-polish-v1.md`
- **Design review:** [reviews/design-review.md](reviews/design-review.md) — recorded outcome `pass`; its contemporaneous approval field is preserved. The user's subsequent direct approval and correction of the review-transition record are captured below; the supplemental assessment is also preserved as authored.
- **Validations:** [validation-WO-001.md](validations/validation-WO-001.md), [validation-WO-002.md](validations/validation-WO-002.md), [validation-WO-003.md](validations/validation-WO-003.md), [validation-WO-004.md](validations/validation-WO-004.md), and [validation-WO-005.md](validations/validation-WO-005.md) — all `pass`.
- **Execution log:** [execution-log.md](execution-log.md)
- **Work orders:** [work-orders/](work-orders/) — WO-001–005 are `complete` after separate passing validations; final combined batch verification/closeout remains pending.
- **Current authorization reconciliation:** The user states the group assessment was a pass, the reviewer erred about the review transition, the initiative was manually moved to `execute`, and expressly authorizes every WO. This is direct human design approval/authorization, not inferred approval. The existing formal review and supplemental assessment files remain unmodified; their contemporaneous content is not rewritten as a new review result.

## Outcome

Establish a coherent desktop writing viewport and status bar with system-aware scaling, stable page scrolling, appearance modes, a sprint timer and native Focus Mode, while preserving fixed screenplay data and print geometry. The planning record below is historical; the later user-directed execution authorization is recorded at the end of this file.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** ScriptCraft's dependable desktop screenplay-writing experience; the manifest roadmap is open-ended and does not prescribe this specific initiative.
- **Initiative type:** `product MVP stage` — desktop viewport and status controls.
- **Single observable outcome:** A coherent desktop viewport/status experience meeting all five requested changes without changing screenplay data or fixed print geometry.
- **Validation boundary:** Sequential fresh per-WO validation of scoped automated acceptance and mandatory project commands, with native runtime limitations recorded when unavailable and documentary reconciliation in WO-005. No manual-testing responsibility is assumed for the user.
- **Split assessment:** Retain one initiative because the features share page, status bar, viewport, and one integrated user-observable boundary; use separate sequential WOs for distinct ownership and manageable review.
- **If split, traceability and dependencies:** No initiative split proposed; work-order dependencies and shared surfaces are recorded below.

See [the initiative process](../../process/02-initiative.md) for the draft-to-plan gate. A filled template is not evidence of convergence.

## Scope

### In scope

- Scale and fixed-page scroll geometry; status integration and appearance; Sprint Timer; Focus Mode; tests and authorized durable-document reconciliation as detailed in the draft WOs.

### Out of scope

- Product/code changes or implementation.
- Design review, execution, validation, commit, or merge during this planning task.
- Changes to semantic screenplay data, print pagination, project format, exports or unrelated interface features.

## Context and motivation

The initial setup saw only an initiative name; the subsequent full user request specified the five changes recorded in the investigation below. Relevant durable context includes [REQ-WRITING-LOOP-005 and REQ-WRITING-LOOP-006](../../requirements/features/screenplay-writing-loop.md) and the [ScriptCraft Desktop Application blueprint](../../blueprints/containers/scriptcraft-desktop.md). The earlier setup scratchpad is retained as historical context only.

## Exploration scratchpad — historical setup only

Use this section during the open-ended planning loop. Keep observations,
research, alternatives, and user decisions distinct from settled scope.

### Initial findings

- The current durable UI contracts include desktop toolbar behavior and status-bar/visible-page behavior; the desktop-container blueprint also records title-bar, editor, and viewport presentation contracts.
- No specific UI surface or problem has been selected for this initiative.
- Git inspection found `master` checked out and one pre-existing untracked file, `.reference/egui-helpers.md`; that file is unrelated and must remain untouched.

### Questions and hypotheses

- Which visible UI surface(s) or interaction(s) should be polished?
- What user-observable problem or quality gap should the work address?
- What evidence will show that the result is successful, and what should remain explicitly out of scope?

### Options considered

No options have been evaluated; defer until the target outcome is understood.

### Decisions and discussion notes

- The initiative is opened in standard mode and remains in `draft`.
- The phrase “UI polish” is recorded as the request label only; it does not authorize a specific design or implementation.

### Next investigation or conversation focus

- Continue exploration by identifying the target UI area(s), the observable improvement sought, and the intended boundaries before convergence or work-order preparation.

## Requirements and blueprint references

- **Requirements:** [Screenplay writing loop](../../requirements/features/screenplay-writing-loop.md) — fixed-layout invariants and `REQ-WRITING-LOOP-005`/`006` status and desktop contracts; proposed changes and conflicts are traced in each WO and WO-005.
- **Blueprints:** [ScriptCraft Desktop Application](../../blueprints/containers/scriptcraft-desktop.md); [Screenplay Writing Loop](../../blueprints/features/screenplay-writing-loop.md); [technical context](../../blueprints/technical-context.md) and [screenplay core](../../blueprints/components/screenplay-core.md) — current technical boundaries and evidence-based documentation impact are described in the WOs.

## Proposed design

The evidence-based proposed design and agent-default decisions are recorded in the subsequent full-scope investigation and detailed in the WOs. They originally awaited review and approval; the user's later direct authorization, confirmed group-pass assessment, and direction to execute supersede that pending-dispatch state without fabricating a new review result.

## Work-order plan

The planning handoff initially left WO-002–005 draft and undispatched. The later direct user authorization explicitly covers every WO and moved the initiative to `execute`. At the WO-002 handoff, WO-003–005 remained undispatched; the subsequent WO-003 implementation and `execute → validate` transition are recorded below.

1. [WO-001: scale and page geometry](work-orders/WO-001-scale-page-geometry.md) — UI scale, fixed page/zoom geometry and scrolling; implementation and independent validation are complete.
2. [WO-002: status integration and appearance](work-orders/WO-002-status-integration-and-appearance.md) — shared status seam, overflow, appearance and page palette; depends on WO-001.
3. [WO-003: Sprint Timer](work-orders/WO-003-sprint-timer.md) — timer state, popup and best-effort notifications; depends on WO-001–002; implementation and independent validation are complete.
4. [WO-004: Focus Mode](work-orders/WO-004-focus-mode-fullscreen.md) — native fullscreen and state reconciliation; depends on WO-001–003.
5. [WO-005: durable-document reconciliation](work-orders/WO-005-durable-document-reconciliation.md) — reconciles requirements and blueprints against validated feature work; depends on WO-001–004.

Shared desktop/status files are owned sequentially; later WOs consume earlier seams. Temporary documentation discrepancies are explicitly classified as requiring updates, not as unchanged, and are deferred to WO-005 as expressly authorized by the user. No requirements or blueprint update is claimed in WO-002; the interim discrepancy remains tracked for WO-005.

## Risks and decisions

- **Scaling and overflow:** Native scale and independent page zoom can be multiplied twice; outer width and row gaps can create spurious scroll. **Mitigation:** explicit logical/physical arithmetic, measured inner geometry, threshold and resize regressions in WO-001.
- **Shared surface and focus:** Compact status, popup/overflow, editor IME and viewport freshness can conflict. **Mitigation:** serial ownership, shared seam, keyboard access and rendered tests in WO-002–004.
- **Platform capability:** OS notification dependency, permissions and bundle identity remain unverified; macOS fullscreen command/restoration and monitor behavior require actual desktop evidence. **Owner:** implementing agents and orchestrator; record unsupported behavior honestly and route genuine blockers through Forge.
- **Existing durable mismatch:** The title-strip requirement and current native-toolbar composition differ. **Owner:** orchestrator/design review; authorize a disposition without silently changing product behavior. WO-005 records the discrepancy.
- **Documentation batching:** A validator may find deferred durable updates incompatible with per-WO policy. **Owner:** design review and orchestrator; resolve the process issue before dispatch, not by claiming accurate docs or waived gates.
- **Branch/prior work:** Setup recorded successful branch creation and preserved a pre-existing untracked reference; this planning session did not re-run Git. Reconcile current branch, worktree, ancestry and the still-active overlapping status-bar initiative before dispatch.

## Design readiness checklist

- [x] Scope, exclusions, outcome, parent goal and validation boundary are explicit; split assessment is recorded.
- [x] Requirements and blueprints are linked; five draft WOs are ordered and carry durable-document impact classifications.
- [x] Initiative mode, user direct-translation confirmation, draft-to-plan transition and per-WO validation cadence are recorded.
- [x] Native/user regression, project-policy checks and pending human approval are identified without claiming execution.
- [x] The subsequent user-confirmed group assessment, correction of the review-transition record, manual move to `execute`, and direct authorization of every WO reconcile the earlier readiness gate for dispatch. The contemporaneous review artifact remains as authored; no new review result is invented.

## Subsequent full-scope investigation — 2026-10-29

At the time of the initial planning handoff on 2026-10-29, before the later direct execution authorization, this section superseded the setup-only statements above that scope was unidentified, that work orders were not requested, and that a target surface was not selected. Those statements reflected the setup agent's narrow instructions, not the full request then supplied. The user explicitly requested planning all five changes and draft work orders, with no design review or implementation. The prescribed plan artifact is this file; do not create a duplicate initiative plan. The historical branch-creation evidence above is retained; this investigation had no execution-capable Git tool and did not independently recheck the current branch or worktree. The later dispatch-time inspection and current authorization are recorded in the execution log. The [prior status-bar initiative](../2026-10-28-status-bar/status-bar-v1.md) records itself as active and has overlapping surfaces; reconcile actual ancestry, uncommitted work and its outstanding work before implementation, not by switching branches or rewriting that history.

### Convergence proposal and boundary

**Outcome:** a coherent desktop writing viewport and status bar that preserve fixed screenplay geometry while adapting safely to display scaling, page overflow, appearance, timing and native fullscreen. One integration boundary covers all four ordered right-hand controls and the shared editor/status/viewport surface. Although the five requested behaviors are distinct and merit separate work orders, they share that integration boundary; do not split the initiative without a specific new user direction.

**In scope:** display scaling and page geometry; page and chrome appearance; transient sprint countdown and stopwatch with best-effort OS notification; native fullscreen focus button; accessibility and narrow-width fallback; integrated regression and associated durable requirement/blueprint updates. **Out of scope:** implementation during this planning task, unrelated toolbar/editor redesign, persistent user preferences or project schema, changes to print pagination/PDF or screenplay content, commits/branch changes, design review, approval, execution and validation. All engineering defaults below are proposals for design review, not approved implementation.

### Evidence: scale and geometry

- [Current app constants](../../../apps/scriptcraft/src/main.rs#L22) set a nominal page width of 360 points multiplied by 1.6, producing 576 egui points. This is a page-presentation multiplier, not the native UI scale. [Page metrics](../../../apps/scriptcraft/src/main.rs#L959) fit that baseline to available width, and [zoom](../../../apps/scriptcraft/src/main.rs#L992) multiplies the fit result; resizing therefore currently shrinks a 100% page. [The input hook](../../../apps/scriptcraft/src/main.rs#L1192) separately manages egui UI scale; [status-bar scale helpers](../../../apps/scriptcraft/src/status_bar.rs#L8) currently select 2.0 when native scale is unavailable. Existing tests deliberately assert this behavior. Avoid conflating either constant with the other.
- [Manifest](../../../apps/scriptcraft/Cargo.toml#L8) declares eframe 0.29; [lockfile](../../../Cargo.lock#L875) resolves eframe, egui and egui-winit 0.29.1 and winit 0.30.13. Local egui 0.29.1 source `egui/src/data/input.rs:19-20` and `context.rs:476-482` define effective pixels per egui point as UI zoom times native pixels per point, with 1.0 native assumption when missing. egui-winit `lib.rs:250-251,273-280,980-981` reads window scale and updates monitor-change events. A framework-reported 1.0 is known even if the integration internally substituted it: do not reinterpret it as unknown without evidence. Physical pixels equal egui logical points times effective pixels per point. Native scale is not monitor physical DPI in inches.
- **Agent-default proposal:** keep egui UI zoom 1.0 with a finite positive native factor, and 2.3 only when genuinely unavailable/invalid; resolve before input every frame as the existing hook does, replacing fallback immediately when native scale arrives. Page width at 100% should be a stable, explicitly chosen 360 egui points times the *resolved scale value* divided by effective pixels per point, so the intended nominal physical page width is 360 times the resolved scale, not 360 times its square. On the standard policy effective equals resolved, hence 360 logical points at 100%, and the OS scales it once. This deliberately replaces the historic 576 logical-point baseline and merits explicit design review; status zoom is an independent 0.75–3.0 page-only multiplier. If the intent was instead to retain the former apparent page size at a specific monitor, that is a visual-calibration decision; do not silently apply 1.6 or native factor twice. In a 2.0-native window a 100% page is 360 logical/720 physical pixels; 150% is 540/1080; at native 1.5 they are 360/540 and 540/810; missing native gives 360/828 and 540/1242. These are planned test expectations, not observed screenshots.
- [Page surface](../../../apps/scriptcraft/src/main.rs#L507) computes width and centering against outer available width, calls a two-axis scroll area, sets its content minimum to that outer width, then adds leading/trailing space inside a horizontal layout. The row's automatic item gaps and potential scrollbar inner-width reduction can push measured content beyond the actual clip; a causal hypothesis, not a reproduced measured defect. Locally cached egui 0.29.1 `containers/scroll_area.rs:90-96,538-570,818,915-920,1233-1250` compares measured content against the inner rect, uses prior-frame bar state and clamps offsets. Instrument actual row bounds, item gaps, content size, inner clip and bar use before asserting a root cause; rule out animated/overlay bar interactions.
- **Agent-default geometry:** exterior buffer B = 16 egui points per side, P = 360 × page zoom in logical points, V = actual available scroll **inner** width after any reserved vertical scrollbar. Measured content width C = max(V, P + 2B); sheet left = (C − P)/2, with no hidden row-spacing additions. Horizontal overflow exists iff P + 2B > V, subject only to a documented sub-physical-pixel rounding tolerance; at equality it does not. Maximum horizontal offset = max(0, C − V); reset/clamp to zero on fit, clamp on resize/zoom and preserve a valid existing offset otherwise. This does not resize the page in a narrow viewport. Vertical flow remains natural and independent. The implementation must solve outer-to-inner feedback without oscillation and validate the actual egui output, not only this arithmetic. Preserve page hit testing, caret/IME, selection, conditional reveal and visible-page freshness.
- Durable conflict: [AC-WRITING-006.7 and 006.10](../../requirements/features/screenplay-writing-loop.md#L183) prescribe responsive-fit 100% and fallback exactly 2.0; [desktop blueprint](../../blueprints/containers/scriptcraft-desktop.md#L153) repeats those. Proposed WOs must explicitly revise both, keep 75–300%, presets, pinch, no project/PDF/core side effects and no global UI scaling from page zoom. The fixed [612 × 792 profile](../../blueprints/components/screenplay-core.md#L159) remains unchanged.

### Evidence and choices: appearance and shared footer

- [Page painting](../../../apps/scriptcraft/src/main.rs#L393) hard-codes near-white sheet, dark text and fixed selection/caret/IME colors. Theme-aware page, text, selection, caret, preedit, border and contrast are all necessary; changing egui visuals alone is insufficient. [Status composition](../../../apps/scriptcraft/src/status_bar.rs#L602) has different wide and compact paths and a 720-point breakpoint. Both require coordinated control placement and measured width budgeting; the existing clipped single-line/error/Diagnostics/page/zoom, accessible activation and 60-frame stable-height contracts remain.
- egui 0.29.1 `data/input.rs:82-83`, `context.rs:1752-1772` and `memory/mod.rs:312-342` expose system preference and resolved theme; egui-winit `lib.rs:412-414` handles OS theme changes. System with unknown OS theme follows egui's resolved fallback, not an assumed OS state. Agent-default: transient per-window System at startup; Day = egui default light, Night = default dark, Midnight = an explicitly darker derived dark palette; isolate/reset visuals when changing modes to prevent midnight contamination of Night/System. Page is light only for resolved light, dark otherwise. Validate readable foreground/background and selection/caret/preedit contrasts and system changes while System is active; never change PDF or project.
- The right-hand order is Sprint Timer, Focus Mode, Appearance, zoom. At narrow widths use a keyboard-accessible overflow/disclosure with the same logical order and access rather than clipping controls; test wide, threshold-adjacent, compact and extremely narrow layouts, including open overlays, without hiding existing required status/error/zoom/diagnostics or growing the footer.

### Sprint timer: proposed state contract

- Own one transient desktop model independent of the popup; inject monotonic time and notification sink in headless tests. Agent-default top selector: two mutually exclusive modes, Countdown and Stopwatch, presented with a toggle or segmented control; the custom numeric minutes field is the countdown configuration, not a third timer mode. No additional preset mode is implied. Mode switching is disabled whenever a countdown or stopwatch is running or paused; during idle it is allowed and resets the selected mode's idle defaults.
- Countdown idle has numeric-only minutes `25`, enabled Start Sprint and disabled End Sprint. Agent-default bounded range is 1–1440 whole minutes; filter non-digits, retain empty input while editing, reject empty/zero/overflow/nonrepresentable values on Start or Enter with inline accessible feedback and no mutation. Starting captures the entered value, sets a monotonic deadline, and shows remaining time as ceiling seconds in `mm:ss` (minutes may exceed two digits). Start becomes Pause; Pause freezes remaining, Resume creates a new deadline, End restores the captured input value and idle controls. Repeated start, end, enter and resume are idempotent/no-op when inappropriate. Closing while running or paused preserves timing; elapsed time is reconciled from the monotonic clock while closed and on the next execution after wake, not accumulated by frame count. Whether that clock includes sleep is platform-dependent and must not be assumed.
- On reaching zero, transition once to completed, send at most one best-effort OS notification, and show `Great Work! You completed a 1 Minute Sprint` or plural `Minutes` using the captured input. Replace the disabled Start/Pause position with an enabled Reset action; End remains visible and disabled. Reset returns to countdown default 25 immediately; closing completed also resets to 25. Do not treat completion as active or emit another notification on repaint, reopen or Reset. OS notification authorization, crate availability, sandbox/bundle identity, and actual delivery are unverified: choose a safe supported dependency/adapter after inspecting available offline sources, report failure/unavailability through bounded non-sensitive in-app feedback, and do not claim delivery when only a request was made. If no timer is active or paused, closing the popup resets defaults; an early End leaves the captured value visible until close or a subsequent start.
- Stopwatch idle shows `00:00:00`, Start and disabled End. Start records monotonic origin; Pause accumulates elapsed duration, Resume records a fresh origin, End clears to zeros and idle. Display hours beyond 99 without wrapping; use checked/saturating arithmetic. Closing active or paused retains state. Schedule bounded repaint while running even when popup is closed, and at the countdown deadline; avoid an idle or paused repaint loop. A UI process that is suspended cannot guarantee notification at the exact wall-clock deadline; notify on next execution unless a verified OS scheduling service is available. App exit/relaunch persistence is not requested.

### Focus Mode: evidence and proposal

- egui 0.29.1 `viewport.rs:1008-1009` names the command borderless fullscreen, and egui-winit `lib.rs:1439-1440` maps it to winit Borderless; `lib.rs:995` publishes native window fullscreen state. Local winit 0.30.13 macOS `window_delegate.rs:1224-1226,1301-1318,1414-1418` documents green-button entry/restore and calls AppKit `toggleFullScreen:`; its delegate also serializes fullscreen transitions (`:117-120,290-309`). This is strong source evidence for native macOS behavior but not proof that this exact app, platform configuration or every OS version matches green-button behavior. Native smoke must check Space/menu-bar animation, OS green-button/Escape changes and restored geometry/maximize; do not claim exact restoration in advance.
- Agent-default: use the framework viewport command rather than unsafe custom AppKit, reflect observed viewport fullscreen rather than optimistic button state, suppress duplicate pending requests, and reconcile OS-driven transitions. When state is unknown, choose disabled/no-op with explanation rather than guessing. Let the native backend restore the previous window settings; do not write fabricated position/size on exit. If native behavior fails on the target, stop and record a design/implementation blocker instead of silently substituting maximization. Preserve existing title-strip maximization semantics and window decoration.

### Validation, dependencies and provisional decomposition

Mandatory future checks are the manifest's format, strict Clippy, workspace test, check and build commands; each work order gets its own fresh implementer and distinct fresh validator, sequentially. Add deterministic headless tests plus actual rendered egui frame/input tests; desktop smoke is important for monitor migration, actual native scrollbar, OS theme and fullscreen transitions and notification delivery, but the manifest does not currently configure a desktop manual hard-stop. Record unavailable smoke as unavailable; never claim a pass. If a requested platform behavior cannot be meaningfully validated or its required implementation dependency is unavailable, record the specific blocker and route it through review/policy rather than inventing capabilities.

Proposed serial ownership, each with requirements and desktop-blueprint updates scoped to its feature:

1. **Scale and fixed page/scroll geometry:** own page metrics, input scaling and geometry tests; update conflicting 006.7/006.10 and desktop blueprint.
2. **Status integration and appearance:** own shared wide/compact right-side layout, accessible overflow, Appearance UI and page/theme palettes; establish stable insertion seam and preserve zoom/status contracts.
3. **Sprint timer:** own timer model, popup, notification adapter, tests and timer status item; consume shared status seam. Explicitly verify OS delivery limitations and dependency availability.
4. **Focus Mode:** own fullscreen reconciliation and status item; depend on status seam and prior timer insertion, verify native semantics/restore.
5. **Integration and documentation reconciliation:** exercise all four right-side items, cross-feature interactions, narrow widths, DPI/zoom/resize/theme/fullscreen, existing editing/status/export invariants and verify durable documents are consistent; no unrelated refactor.

All stages touch the same desktop shell and may touch the same requirements/blueprint: sequential execution and explicit ownership avoid conflicting edits. At drafting, every work order quoted applicable current acceptance criteria verbatim, identified files, tests and dependencies, and was left draft pending the user's later decision. The current authorization record below supersedes that historical pending state; the direct user decision, rather than this planning note, authorizes execution. Durable requirement/blueprint updates remain separately deferred to WO-005.

**Historical process checkpoint:** The user explicitly requested draft work orders and forbade review/execution. The current [initiative process](../../process/02-initiative.md#L159) requires confirmation after summarizing direct translation of a broad request before advancing `draft → plan` and creating work orders. The following handoff records that subsequent confirmation; it is not design approval and no `plan → review` transition is claimed.

### User-confirmed planning handoff — 2026-10-29

The subsequent user instruction quoted in metadata confirms direct translation with revised grouping, and authorizes `draft → plan`. Four feature groups are to be assigned to architect agents: scale/page geometry; status integration and appearance; Sprint Timer; Focus Mode. Multiple work orders per group are permitted. Durable requirements/blueprint edits may be batched into a separate dependent work order, with each feature work order explicitly identifying the temporary documentation lag and the reconciliation owner. Use unique sequential whole-number work-order IDs for independent work; letter suffixes are reserved by [the Forge overview](../../process/00-overview.md#L112) for fixes to a failed validation, so they should not be used for unrelated sibling scopes merely as grouping labels.

The initial planning handoff included language assigning post-implementation regression to the user. This is retained only as historical planning text and is not evidence of the user's acceptance of a testing duty. The user's later explicit instruction directs that no manual-testing responsibility be assumed; the WOs must record native evidence limits honestly without assigning the user a test. Automated project commands, implementation acceptance evidence and fresh per-WO validation remain mandatory. No user test or acceptance is inferred.

### Additional delegated investigation and planning handoff

- Five separately drafted WOs now exist as listed in the work-order plan; each specifies a fresh implementer and a separate fresh validator. The user-requested feature groups were delegated to four distinct architect sessions, followed by a separate documentation planning session. No implementation or review was delegated.
- Sprint Timer research identified that the notification-related package in the lockfile is indirect, not evidence of a directly usable offline adapter; locally inspected macOS notification headers document authorization and callbacks but do not establish safe Rust integration, app identity, foreground display or delivery. Native capability remains unverified. Local egui popup behavior may permit only one popup at a time, so overflow-to-timer routing must be tested, not assumed. Monotonic-clock behavior across system sleep is platform-dependent; do not promise sleep-inclusive timing without evidence.
- Fullscreen research additionally found that the backend can expose a target fullscreen state before native transition completion; pending-state handling must not assume that an observed target proves the animation is finished. Native green-button equivalence and restore require real user/platform evidence. The existing title-strip requirement conflicts with observed current panel composition and must be dispositioned without silently restoring or deleting product behavior.
- At this historical planning checkpoint, the proposed WO-005 batch deliberately left known temporary requirements/blueprint lag during feature execution. The user later directly authorized this documented temporary state and all WOs; the current interim discrepancy and WO-005 ownership are recorded above and in WO-002. This annotation does not claim that the requirements or blueprints are already reconciled.

### Design review record — 2026-10-29

- The fresh review attempt is recorded as [blocked](reviews/design-review.md), not passed, because the formal-review precondition remains unsatisfied: this plan still records phase `plan`, and design readiness has not been transitioned/confirmed by the orchestrator.
- The user-directed partial group assessment of all five draft WOs is [recorded separately](reviews/design-review-assessment.md). It identifies execution blockers, warnings, and the per-WO dispositions without changing any proposal or product code.
- No work order is approved or dispatched; no design approval is recorded. The initiative phase/status remain `plan` / `active` pending orchestrator disposition of the review blockers and a valid formal review transition.

## User-directed execution authorization — 2026-10-29

- The user explicitly authorized implementation of all WOs and directed the initiative to `execute`. WO-001 alone was dispatched for this task; its implementation evidence is recorded and its status is `in-review`. The phase is `execute` / status `active` pending immediate independent validation.
- This is a direct user authorization to proceed, not a formal design-review pass. The formal [design-review artifact](reviews/design-review.md) remains `blocked`, and the supplemental [group assessment](reviews/design-review-assessment.md) remains a partial assessment. Neither is rewritten or represented as passed.
- The authorization to execute WO-001 as written includes its proposed 360-logical-point page at 100% and its explicitly temporary requirements/blueprint deferral to WO-005. The requirements and desktop blueprint remain unchanged during WO-001 and must be described as temporarily inconsistent, not verified. This records the interim state and approval basis without claiming a general policy bypass for later WOs.
- Read-only pre-dispatch evidence confirmed branch `dev/2026-10-29-ui-polish` at `0df130f`, with the prior status-bar initiative branch as an ancestor. The pre-existing modified `.forge/process/04-work-order.md`, untracked `.reference/egui-helpers.md`, and other existing initiative artifacts are preserved; no branch switch, commit, or merge is authorized.
- Validation cadence remains **per work order**. A separate fresh validator is required immediately after WO-001 is recorded `in-review`; this implementation session will not create a validation verdict or claim independent validation. Native OS-specific scaling remains unverified absent an actual OS/runtime check.

## WO-003 implementation handoff — 2026-10-30

- **Authorization and prerequisites:** The user explicitly confirmed that the group assessment passed, directed execution, authorized every work order, and gave human design approval. WO-001 and WO-002 are each `complete` with separate passing validation artifacts. This direct user decision is the approval basis; it does not rewrite the contemporaneous review artifacts or create a validation result.
- **Dispatch/state:** WO-003 was dispatched to a fresh coding implementation session. The interface does not supply an implementer session identifier; none is inferred. WO-003 is now `in-review`; the initiative phase transitioned `execute → validate` while status remains `active`. WO-004 and WO-005 were not dispatched or implemented.
- **Branch/worktree:** Read-only inspection confirmed `dev/2026-10-29-ui-polish` at `0df130f`; `dev/2026-10-28-status-bar` remains an ancestor. Pre-existing `.forge/process/04-work-order.md` modifications and `.reference/egui-helpers.md` are preserved, as are the accepted WO-001/WO-002 source changes. No branch switch, commit, or merge occurred.
- **Implementation evidence:** Added the production timer model, stopwatch status action and foreground popup, completion notification attempt/fallback, lifecycle repaint scheduling, and controlled/rendered tests. Final focused `cargo test -p scriptcraft wo003_` passed 15/15. Final `cargo test --workspace` passed 223 tests (141 app, 45 core unit, 37 integration; 0 failures; 0 doc-tests); formatting, strict Clippy, workspace check/build, and `git diff --check` passed. Detailed attempts and OS limitations are recorded in [WO-003](work-orders/WO-003-sprint-timer.md) and the [execution log](execution-log.md).
- **Notification evidence/limit:** `/usr/bin/osascript` is present on this macOS host and the implementation asynchronously submits a fixed-text notification request without adding dependencies or unsafe code. Tests did not send a system notification. OS authorization, identity, presentation, and delivery are unverified; a successful process exit is not represented as delivery. Non-macOS/command failures use bounded in-app fallback.
- **Documentation and next state:** Requirements and blueprints remain explicitly deferred to WO-005; their current mismatch is tracked as temporary, not called unchanged. A separate fresh validator must now compare WO-003 against the accepted WO-002 baseline and create [validation-WO-003.md](validations/validation-WO-003.md). This implementation session did not independently validate or assign a manual-testing duty to the user.

## WO-003 independent validation — 2026-10-30

- **Validation:** [validation-WO-003.md](validations/validation-WO-003.md) — pass. The validator inspected the timer model, app lifecycle, status integration, rendered and deterministic tests, and compared against the accepted WO-001/WO-002 baseline. Fresh `cargo test -p scriptcraft wo003_` passed 15/15; `git diff --check` passed. The implementation's recorded format, strict Clippy, workspace test, check and build results were fresh and successful, so they were not redundantly rerun.
- **Notification boundary and limitations:** The macOS adapter asynchronously invokes `/usr/bin/osascript` with fixed completion text and reports command acceptance separately from delivery; failures and unsupported platforms retain in-app completion feedback. No OS notification or desktop/native smoke was performed. Authorization, identity, presentation and actual delivery remain unverified; this is an evidence limitation, not a fabricated success or user testing assignment.
- **Verdict and state transition:** No grossly unrelated or rogue work was found. WO-003 is now complete; no fix work order is required. Initiative phase returns from `validate` to `execute` / status remains `active`. WO-004's WO-001–003 dependency chain is satisfied; WO-004 itself remains undispatched and must follow its own authorized fresh implementation and validation cycle. No product code, branch, commit or merge action occurred in this validation.

## WO-004 implementation handoff — 2026-10-30

- **State transition:** WO-004 implementation evidence is recorded in its [work order](work-orders/WO-004-focus-mode-fullscreen.md) and the [execution log](execution-log.md). Initiative phase transitioned `execute → validate` / status remains `active`; WO-004 is `in-review`, not complete, pending a distinct fresh validator. WO-005 was not dispatched or implemented.
- **Scope/evidence:** Added a per-window native fullscreen action and reported-state reconciliation after Sprint Timer and before Appearance, including inline/overflow interaction, timeout/recovery, OS-originated state settling, keyboard/accessibility, rendered egui command tests, and bounded footer tests. Required focused/workspace tests and manifest format, Clippy, check, and build commands passed; counts and retries are in WO-004 and the execution log.
- **Platform limit:** No actual macOS app/window fullscreen smoke was performed. Framework-rendered tests and source inspection do not establish green-button/Space/menu-bar equivalence, native transition completion, Escape behavior, or window restoration; these remain unverified, with no manual-testing duty assigned to the user.
- **Preservation:** Requirements and blueprints remain unchanged under WO-005's explicit deferral. WO-005, core, Cargo manifests/lockfile, branch, commit, and merge are unchanged. Existing unrelated worktree changes remain preserved.

## WO-004 independent validation — 2026-10-30

- **Validation:** [validation-WO-004.md](validations/validation-WO-004.md) — `pass`. A fresh independent validator compared the current Focus Mode source and recorded implementation evidence against WO-004 and the accepted WO-001–003 validation baseline. No grossly unrelated or rogue work was found.
- **Focused verification:** Fresh `cargo test -p scriptcraft wo004_` passed 8/8. The final implementation evidence records successful `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (231 passed), `cargo check --workspace`, and `cargo build --workspace`; the validator did not redundantly rerun those successful required commands.
- **Implementation alignment:** The controller reconciles optional reported state, pending generations, timeouts and OS-originated changes; the app dispatches only the per-viewport fullscreen command. Rendered tests cover accessible inline/overflow placement, entry/exit commands, timer/appearance/zoom/session-revision preservation, and bounded footer height.
- **Framework/native evidence limit:** Inspected local egui/egui-winit/winit sources establish a borderless fullscreen command route and a backend-owned macOS transition/restoration path, not native behavior of this app. No macOS app/window fullscreen smoke occurred. Green-button equivalence, Space/menu-bar and Escape behavior, transition completion, and actual geometry/maximized/multi-monitor restoration remain unverified; no manual-testing duty or native success is claimed.
- **Inspection-command retry:** The initial read-only `rg` source search exited 127 because `rg` is unavailable in this environment. The equivalent search was rerun successfully with system `grep`; no project files were affected.
- **Verdict and state:** WO-004 is now `complete`; the initiative phase transitions `validate → execute` and remains `active`. No fix work order is required. No product code, branch, commit, or merge action occurred during validation; existing unrelated and accepted worktree changes were preserved.
- **Next state at this historical checkpoint:** WO-005's four-predecessor dependency was satisfied. It was then awaiting dispatch; later user authorization and WO-005 execution are recorded below and in the execution log.

## WO-005 documentation implementation handoff — 2026-10-30

- **Authorization/prerequisites:** The user directly confirmed the group assessment passed, manually moved the initiative to `execute`, and authorized every WO. WO-001–004 each have a separate passing validation. No review or validation artifact was rewritten or fabricated.
- **State at implementation handoff:** The initiative remained `execute` / `active` and WO-005 was `in-review`; see the independent validation entry below for the verdict and current transition.
- **Scope/results:** Reconciled the writing-loop requirements and desktop, feature-composition, and technical-context blueprints against accepted WO-001–004 behavior. Fixed page/scale, status/appearance, timer, and Focus Mode contracts are traceable in WO-005. The core blueprint, product code, tests, manifests, and lockfile were not edited.
- **Unresolved/evidence limits:** AC-WRITING-005.6's app-owned title-strip requirement remains unresolved and distinct from status-bar WO-008's removed project-metadata heading. No notification delivery, native fullscreen completion/restoration, OS-theme delivery, or native visual smoke is claimed. The unrelated desktop persistence wording is deferred.
- **Checks and preservation:** Required format, strict Clippy, workspace tests (231 passed), workspace check and build passed. Markdown target/link verification and `git diff --check` passed. Branch remains `dev/2026-10-29-ui-polish` at `0df130f`; the pre-existing process-file modification, unrelated untracked reference, accepted predecessor source, and overlapping status-bar initiative were preserved. No branch, index, commit, merge, or cleanup operation occurred.
- **Next gate at implementation handoff:** WO-005 recorded the changed-file inventory, traceability matrix, command results, evidence limitations, and unavailable implementer session identifier. A separate fresh validator was required; see the validation record below.

## WO-005 independent validation — 2026-10-30

- **Validation:** [validation-WO-005.md](validations/validation-WO-005.md) — `pass`. The fresh validator compared the documentary changes and traceability matrix with the authorized work order, relevant app/core evidence, and WO-001–004 plus each distinct passing validation artifact. No grossly unrelated or rogue documentation change was found.
- **Alignment:** Requirements and blueprints accurately describe the fixed 360-logical-point page presentation, 16-point buffered inner-clip geometry/overflow, valid-native-scale-once and 2.3 fallback policy, resolved appearance modes/page palette, ordered status actions, transient Sprint Timer behavior, and reported-state Focus Mode behavior within their validated evidence. Core pagination, the 612 × 792 print profile, semantic/source mapping, and fixed-layout export boundary remain unchanged.
- **Unresolved discrepancy/evidence limits:** AC-WRITING-005.6 still requires the separate app-owned title strip, while current source composes the toolbar below the native titlebar. It remains explicitly unresolved and distinct from the project-metadata heading removed by status-bar WO-008. Native notification delivery, OS theme/visual behavior, fullscreen completion/restoration and monitor behavior remain unverified; no user testing duty or native success is inferred.
- **Status/next phase:** WO-005 is `complete`; all five work orders have separate passing validations. The initiative transitions `validate → closeout` and remains `active`. Final combined batch verification and closeout actions have not been performed.

## Closeout checkpoint — 2026-10-30

- **Checkpoint/authorization:** The user approved the consolidated closeout checkpoint and explicitly authorized committing all current changes and merging this initiative to `master`. This includes pre-existing untracked/reference and workflow changes; do not discard or omit them.
- **Recorded preconditions:** WO-001–005 are each `complete` with a distinct passing validation. The final combined verification recorded `cargo fmt --all -- --check`, strict workspace Clippy, all 231 workspace tests, workspace check/build, and `git diff --check` as passed. User approval and execution authorization are recorded above; the formal review and supplemental assessment remain preserved as authored rather than rewritten. No unresolved Git conflict or rollback/migration work is recorded.
- **Live Git inventory:** The active branch is `dev/2026-10-29-ui-polish`; local `master` is at the same `0df130f` parent, and `dev/2026-10-28-status-bar` is an ancestor. No remote is configured. The worktree has seven modified tracked paths and twenty untracked paths: the UI Polish initiative artifacts, `.reference/egui-helpers.md`, `apps/scriptcraft/src/focus_mode.rs`, and `apps/scriptcraft/src/sprint_timer.rs`, plus the recorded product, durable-document, and `.forge/process/04-work-order.md` modifications. No staged changes existed. A credential-pattern scan found no potential credential indicators. Ignored `.DS_Store` files and `target/` are excluded by Git and are not part of the authorized commit inventory.
- **Closeout follow-ups:** Preserve the title-strip discrepancy and evidence limitations without treating them as verified or silently resolving them:

  | Follow-up | Rationale and next action | Discoverable owner |
  |---|---|---|
  | AC-WRITING-005.6 app-owned title strip versus current toolbar/native-titlebar composition | Obtain explicit product disposition and update the requirement/blueprint only through an authorized future change. Keep distinct from status-bar WO-008's removed project-metadata heading. | Product owner and future-initiative orchestrator |
  | Native OS evidence | OS theme/visual behavior, monitor-scale migration, fullscreen completion/restoration and monitor behavior, and notification authorization/presentation/delivery remain unverified. Capture platform evidence in a future native-capable validation when available; this does not assign a manual-testing duty to the user or add a project-policy hard stop. | Future desktop implementer/validator, coordinated by the orchestrator |
  | Historical desktop persistence wording in the desktop blueprint | Correct separately if still applicable; it was explicitly outside WO-005's authorized scope. | Future technical-documentation owner/orchestrator |

- **Closeout transaction state:** Preconditions and follow-ups are recorded. Initiative remains `closeout` / `active` until the required commit and merge complete; the final status flip and Git-result evidence are to be recorded only after those operations succeed.
