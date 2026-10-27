# Initiative Plan: Native macOS Titlebar Baseline — v4

## Metadata and iteration boundary

- **Phase:** `closeout`
- **Status:** `closed`
- **Authoritative version:** `v4` — converged native-titlebar baseline experiment; it does not replace or close v1, v2, or v3.
- **Created:** `2026-10-27`
- **Updated:** `2026-10-27`
- **Mode:** `standard`; no rapid path was requested.
- **Purpose:** Preserve v3 evidence and authorize one bounded, reversible native macOS titlebar baseline experiment to compare native maximize/restore with the already observed custom hidden-titlebar behavior.
- **Allowed artifacts in this planning step:** this v4 plan and exactly one v4 work order.
- **Excluded from v4 design planning:** application source changes, v3 or historical changes, requirements/blueprint changes, review/validation artifacts, Git/branch actions, implementation, or claims that native titlebar behavior fixes the scaling artifact. The subsequent approved preflight is limited to read-only Git/base/API inspection, this plan's approval/branch/API evidence, and the execution-transition entry in the existing log; no branch switch/create or implementation was authorized or performed.
- **Direct-translation confirmation:** user explicitly directed proceeding with work orders and execution subject to visual-feedback gates; this plan reduces the v4 investigation to one baseline experiment and one work order.
- **Branch policy and preflight evidence:** project policy requires a local initiative branch. At the v4 preflight, `HEAD` and `master` both resolved to `6a6739c5263edee9896b32c00fc6f8470e35f3d6`; current branch was `dev/2026-10-27-titlebar-transition-and-elements-alignment-v3`. The established unsuffixed branch `dev/2026-10-27-titlebar-transition-and-elements-alignment` exists at `28024cdff84c4a60aa6c2fb74123745ec270905c` and is an ancestor of `HEAD`, but is not current. The current v3 continuation branch is the appropriate branch for the cumulative v3 work and is at the recorded `master` base; continue on it without switching or creating a branch. Local branch capability is available (local initiative refs exist, `.git` is writable, and prior initiative branch creation is recorded in this log). No Git branch action was taken.
- **Phase transition evidence:** `draft → plan` on `2026-10-27`; then `review → execute` on `2026-10-27` after the passing v4 design review, explicit human approval, branch/base preflight, and resolved API audit below. The approval authorizes exactly the reviewed [`V4-WO-001`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1); the ordered native-chrome prerequisite and maximize/restore visual gates remain mandatory. No implementation was performed in this transition.

## User-stated goal

Restore the ordinary native macOS titlebar as the baseline so the user can determine whether double-clicking that native bar produces the same whole-UI scaling artifact—UI grows during the maximize/restore animation and snaps back at the end—as the custom hidden-titlebar path, where the app handles the double-click animation itself.

This is an evidence-gathering goal. It is not yet a decision to retain native chrome as the permanent product design, a diagnosis of the animation artifact, or authorization to remove the existing transition machinery.

## Findings: confirmed evidence

### Repeated visual failures in the native diagnostic chain

1. The v3 app-owned transition-metric policy passed deterministic checks but the user reported that toolbar/page presentation still swelled, shrank, and snapped during real maximize and restore. This is a confirmed failure of that attempted solution, not missing display evidence. See the v3 history in [`execution-log.md`](execution-log.md:337) and the original diagnostic boundary in [`V3-WO-002a`](work-orders/V3-WO-002a-native-titlebar-scaling-diagnostic.md:21).
2. Three subsequent native-titlebar diagnostic attempts were each reported by the user as still showing hidden or transparent native titlebar chrome: the attempts preceding [`V3-WO-002b`](work-orders/V3-WO-002b-native-titlebar-mode-wiring.md:23), [`V3-WO-002c`](work-orders/V3-WO-002c-native-titlebar-visibility-handoff.md:18), and [`V3-WO-002d`](work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:18). These are confirmed failed visual gates, not evidence that native animation behaves like the custom animation.
3. The [`V3-WO-002d`](work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:84) implementation added explicit requested native settings and a startup diagnostic, then passed recorded automated checks. Its required user visual gate is still outstanding in the inspected execution record ([`execution-log.md`](execution-log.md:411)). Therefore, the latest build's actual visible titlebar state and any native maximize/restore animation result are unknown; neither success nor another failure is inferred here.

### Current source is not a vanilla native-titlebar baseline

- The selected mode in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:47) is currently `CustomHidden`. The app continues to provide its custom title strip and its app-owned transition-policy path in that mode ([`main.rs`](../../../../apps/scriptcraft/src/main.rs:58), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2467)).
- The alternate native diagnostic is not simply the unmodified platform default: it explicitly sets title, decorations, opacity, title/titlebar/button visibility hints, and full-size-content behavior in [`native_options_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2563). It uses a diagnostic-specific title. The native branch suppresses the app title strip and bypasses the app-owned transition policy, but retains the app toolbar/content.
- Startup mode and options are now connected through a single `StartupPresentation` handoff to the sole `eframe::run_native()` call ([`main.rs`](../../../../apps/scriptcraft/src/main.rs:2590)). This establishes source-level handoff, not proof of the actual native macOS window appearance.
- The earlier v3 execution record claimed that resolved egui-winit 0.29.1 ignored `titlebar_buttons_shown`; the fresh source audit recorded under Gate and next state shows that the field is forwarded to winit. Neither the request nor source forwarding proves that macOS displayed native buttons. The prior v3 record remains historical and unchanged.
- V3 also accumulated a custom transition policy, metric capture/hold/release behavior, diagnostics, mode selection, title-strip routing, and several rounds of native-option wiring. This machinery has not yielded the needed native-vs-custom visual comparison. Its presence is a confounder and a maintenance cost; it is not, by itself, proof that any individual helper is the root cause.

## Evidence versus hypotheses

### Established

- The custom hidden-titlebar path has a user-confirmed maximize/restore swell/shrink/snap artifact.
- The app-owned metric policy did not remove that artifact in the user's visual test.
- Three earlier native-diagnostic visual attempts did not visibly expose native titlebar chrome.
- The latest direct-configuration change has automated evidence but no recorded user visual outcome yet.
- No user-observed comparison establishes whether native macOS titlebar double-click produces the same artifact.

### Open hypotheses—not findings

- The repeated invisibility may involve differences between requested eframe options and actual macOS/winit window behavior, rather than a simple missing startup handoff. This needs direct verification on the ordinary native-window path.
- The custom title/content integration and the native window's content boundary may influence the perceived scaling, but neither is established as causal.
- The native titlebar may avoid the artifact, reproduce it, or produce a different visual effect. No outcome is predicted by this document.

## Converged experiment and boundary

### Single observable outcome

Run the application with an ordinary, visibly native macOS titlebar as the provisional baseline, then record whether double-click maximize and restore reproduce the known toolbar/page swell-shrink-snap artifact. The result is evidence about the native path, not a fix claim or a permanent titlebar product decision.

### Selected approach

- Create exactly one work order: [`V4-WO-001-vanilla-native-titlebar-comparison.md`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1).
- Use the ordinary native window-decoration path as the selected startup baseline for the experiment. Before changing options, the implementer must inspect the resolved eframe/winit defaults and preserve unrelated startup settings; do not simply delete every explicit option or add another diagnostic-specific override layer.
- For this experiment only, native window chrome owns the titlebar and its maximize/restore interaction. Do not paint the custom title strip, a transparent/hidden native-titlebar substitute, a duplicate title surface, or the app-owned title-strip maximize hit region over native chrome. Leave the app toolbar as egui content.
- Keep the prior custom hidden-titlebar path and its app-owned behavior available as the reversible comparison/rollback path. Do not remove accumulated transition helpers or undertake broad cleanup.
- Use the ordinary product window title, not a diagnostic title. No new user-facing mode selector, diagnostic option matrix, native-animation control, or speculative geometry change is part of this experiment.

### Comparison evidence and visual gates

The comparison baseline is already known: the user confirmed swell/shrink/snap during maximize/restore on the custom hidden-titlebar path, and the v3 app-owned metric policy did not remove it. Do not require a repeat custom-path test to establish that evidence. The native result remains unknown until the user inspects the rebuilt application.

There is one consolidated user visual checkpoint, with two ordered observations:

1. **Native-chrome prerequisite:** on the actual rebuilt macOS window, the user confirms the ordinary native titlebar and native window controls are visibly present and non-transparent, the product title is shown without a duplicate custom strip, and the toolbar is app content below/within the native content boundary. If this prerequisite fails, stop: do not interpret any animation as native-titlebar comparison evidence.
2. **Animation comparison:** only after the prerequisite passes, the user double-clicks the native titlebar to maximize and then restore while watching the complete animations. Record separately whether toolbar icon/text and page presentation swell, shrink, or snap during each direction. Compare that report to the confirmed custom-path artifact above and classify the native result as **same**, **absent in both directions**, or **different/partial**. A report of no visible artifact is valid experiment evidence, not proof that the product issue is universally fixed.

The implementer records the clean-built app identity and the user's direct observations. A written user report is sufficient; do not demand a screenshot. Automated tests cannot substitute for either visual observation. Preserve the validated black-bar/clear-color/padding outcome, toolbar allocation and `128.0` Elements width, and Elements/popup/cursor/input/semantic behavior; do not make those unrelated contracts visual-gate work or use them as a reason to expand scope.

### Explicit exclusions

- No further layered native diagnostic iterations, visual startup diagnostics, unsupported native animation controls, global egui zoom, polling, overlays, or toolbar/page geometry adjustments.
- No permanent native-versus-custom product decision, broad removal/refactor of transition code, or assertion that the native path will eliminate the artifact.
- No reopening v1/v2/v3 work, changing validated black-bar or toolbar contracts, or changing requirements/blueprints in this planning step.

## Requirements, blueprints, and durable-document impact

- Relevant product contract: [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), especially [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137). The custom title-strip remains the durable product requirement during this temporary experiment.
- Relevant technical contract: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105), including the current native-presentation hints and unified app-owned title-strip/toolbar surface.
- Work-order classifications: requirements **verify unchanged** and blueprint **verify unchanged**. The experimental startup baseline is temporary and reversible; it does not revise the intended product behavior or technical contract. The work order authorizes no durable-document edits.
- Preserve the validated [`validation-V3-WO-001b.md`](validations/validation-V3-WO-001b.md:1) black-bar/clear-color/restored-padding result.

## Work-order plan

1. [`V4-WO-001-vanilla-native-titlebar-comparison.md`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1) — sole v4 work order; establish an ordinary native macOS titlebar baseline through the smallest reversible startup change, retain the custom hidden-titlebar rollback path and toolbar/content contracts, then obtain the ordered user visual observations above.

No additional diagnostic, cleanup, reconciliation, validation, or documentation work order is authorized by v4. No validation artifact is created during planning.

## Gate and next state

- **Current phase/status:** `execute` / `active`.
- **Design review:** [`design-review-v4.md`](reviews/design-review-v4.md:63) returned `PASS WITH WARNINGS`, with no blocker.
- **Human approval:** received and recorded at `2026-10-27T21:41:42.190Z` (UTC; `2026-10-27 16:41:42.190 CDT`). Exact user statement: “I approve the v4 design and authorize V4-WO-001 execution after branch/base verification and the required API audit.” Scope is only the one reviewed v4 work order; no other v4/v3 work or product decision is authorized. The visual gates in [`V4-WO-001`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:73) remain required.
- **Resolved API/default audit:** [`Cargo.lock`](../../../../Cargo.lock:875) resolves eframe/egui-winit `0.29.1` and winit `0.30.13`. eframe `NativeOptions::default()` uses the default viewport; egui `ViewportBuilder` leaves its optional fields unset. The resolved egui-winit mapping defaults to opaque (`transparent=false`) and decorated (`decorations=true`) windows, takes title from the viewport or runner title, and on macOS defaults title/titlebar/buttons shown and `fullsize_content_view=false` ([egui-winit mapping](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1610), [macOS mapping](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1733), [winit defaults](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/window.rs:132)). The source explicitly forwards `titlebar_buttons_shown` through `with_titlebar_buttons_hidden` at egui-winit line 1736; this contradicts the earlier v3 execution note that the field is ignored. The current app source still selects `CustomHidden` and uses explicit mode-specific native options; this audit makes no claim about actual rendered macOS chrome. No source/configuration change was made.
- **Branch/base:** verified as recorded in Metadata; the current v3 continuation branch is appropriate, so no branch switch/create was required. The worktree contains pre-existing tracked and untracked changes; they were left untouched.
- **Implementation authorization:** the reviewed scope is approved and branch/API preflight is complete. A fresh implementer may now be dispatched for [`V4-WO-001`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1) only. This preflight did not dispatch an implementer or alter the work-order artifact.
- **Visual user gate:** satisfied for [`V4-WO-001`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:73): the user confirmed visible native chrome and a usable toolbar, then reported no swelling, shrinking, or final snap in either maximize or restore direction (**absent in both directions**). The title-by-traffic-lights and visible titlebar-color concerns remain separate follow-up observations, not in-scope acceptance failures.
- **Validation/next state:** [`validation-V4-WO-001.md`](validations/validation-V4-WO-001.md:1) returned `PASS WITH WARNINGS`, recommended `complete`, and found no hard-stop. [`V4-WO-001`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:5) is now `complete`; any new design work remains pending and no follow-up work order is authorized or created by this disposition.
- **Product decision:** pending; the native titlebar is an experiment result, not an established fix or permanent design choice.

## Evidence links

- Prior initiative context: [`titlebar-transition-and-elements-alignment-v3.md`](titlebar-transition-and-elements-alignment-v3.md:1).
- Latest options and startup-handoff result: [`V3-WO-002d-native-titlebar-direct-configuration.md`](work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:84) and [`execution-log.md`](execution-log.md:411).
- Current presentation-mode and startup configuration: [`main.rs`](../../../../apps/scriptcraft/src/main.rs:47), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2563), and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2620).
- Confirmed custom-path artifact and failed metric-policy attempt: [`execution-log.md`](execution-log.md:337) and [`V3-WO-002a-native-titlebar-scaling-diagnostic.md`](work-orders/V3-WO-002a-native-titlebar-scaling-diagnostic.md:21).
- Validated toolbar/black-bar preservation baseline: [`validation-V3-WO-001b.md`](validations/validation-V3-WO-001b.md:1).

## Append-only follow-up iteration — seamless native chrome proposal (2026-10-27)

### Evidence and boundary

- [`V4-WO-001`](work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1) remains `complete`, with [`validation-V4-WO-001.md`](validations/validation-V4-WO-001.md:1) at `PASS WITH WARNINGS`. The user's native maximize and restore observations remain **absent in both directions** for swell/shrink/final snap; the observed native controls and toolbar were usable.
- The same user report identifies two unresolved appearance gaps: native `ScriptCraft` title text is visible beside the traffic lights and the native titlebar color is visibly distinct from the app surface. These observations motivate this separate proposal; they do not reopen or invalidate V4-WO-001.
- Current startup selects `NativeMacOs`, while `native_options_for_mode()` currently returns `eframe::NativeOptions::default()` for that mode; the native-mode toolbar and app-owned surface fill/clear color are already present in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2477) and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2563). No technical cause or guaranteed visual outcome is inferred from the API requests.

### Proposed single outcome

Retain ordinary macOS native window controls and their native maximize/restore behavior, while requesting titleless native chrome and a transparent/full-size titlebar surface so the app background can visually continue behind it. The app toolbar must remain usable and clear of native controls. Preserve the user's observed no-swell/no-snap behavior in both maximize and restore directions and preserve the already validated black-bar/clear-color/restored-padding and toolbar/content contracts.

The candidate resolved 0.29 API configuration and its limits are specified in the sole proposal [`V4-WO-002-native-titlebar-seamless-chrome.md`](work-orders/V4-WO-002-native-titlebar-seamless-chrome.md:1). These are supported configuration requests in the audited dependency source, **not** a claim that the installed stack or macOS guarantees the visual result. The proposed work order requires a fresh resolved-source audit and a strict user visual gate; any unsupported option, inability to keep the toolbar safe, or regression is failure/blocked evidence, not permission to add a custom maximize path.

### Scope and durable-document classification

- One proposed work order only: [`V4-WO-002-native-titlebar-seamless-chrome.md`](work-orders/V4-WO-002-native-titlebar-seamless-chrome.md:1). Its scope is the native macOS startup hints and the minimum chrome placement/fill work needed to test the seamless surface and keep the toolbar safe. It excludes custom maximize handling, a new transition policy, broad geometry changes, and changes to unrelated editing/toolbar behavior.
- Proposed durable-document impact is **requirements: update** and **blueprint: update**, subject to later approval and actual behavior. The outcome changes the documented visible titlebar contract in [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) and the native/app chrome ownership description in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105). The proposed work order authorizes those narrowly scoped reconciliations only during a future approved execution and only if the tested design is retained. Neither durable document is changed in this proposal step.

### Forge transition and approval boundary

- This is an append-only proposal, not a change to the prior v4 approval or the completed V4-WO-001 record. The existing v4 phase/status and all prior history remain unchanged; no implementation, validation, or review artifact is authorized by this append.
- The seamless-chrome outcome is a distinct follow-up outcome. Per the fixed phase sequence in [`00-overview.md`](../../process/00-overview.md:69) and the design-review dispatch conditions in [`03-design-review.md`](../../process/03-design-review.md:3), the current v4 `execute` phase cannot be silently rewound to `plan` or treated as a new `review` phase after V4-WO-001. The minimal process-compliant route is to close out v4 under its existing evidence, then carry this single proposal into one successor initiative and progress it through `draft` → `plan` → `review`. That successor must receive a fresh-context design review and a separate explicit human approval before implementation. No successor initiative plan or review artifact is created here; this single candidate work order is retained and linked for traceability, not as an executable v4 order.
- **Current status:** proposal pending design convergence/review; no implementation approval. No source/configuration, V4-WO-001/validation, execution log, v3/history, requirements, blueprint, or Git state was changed.

## Append-only closeout disposition — native baseline and deferred seamless-chrome objective (2026-10-27)

This is the current v4 closeout state and supersedes earlier current-phase summaries above without rewriting their evidence. V4 is now in `closeout` / `active`; it is not `closed`. The completed baseline experiment and its validation are preserved, while the separate seamless-appearance proposal is explicitly tabled for a successor initiative.

### Final closeout checklist

- [x] **Design/review/approval:** [`design-review-v4.md`](reviews/design-review-v4.md:63) recorded `PASS WITH WARNINGS`; explicit human approval for V4-WO-001 and the preflight evidence are recorded above. Approval did not extend to V4-WO-002.
- [x] **V4-WO-001:** complete. Independent [`validation-V4-WO-001.md`](validations/validation-V4-WO-001.md:76) returned `PASS WITH WARNINGS`, recommended complete, and found no hard-stop. The user confirmed ordinary native chrome and native controls were visible and the toolbar usable; maximize and restore showed no swell, shrink, or snap in either direction. The validation warning is the pre-existing cumulative dirty-tree limit on exact per-WO diff attribution; it is not a failed acceptance criterion.
- [x] **Preserved outcome boundary:** the native animation result is evidence for this comparison only, not a universal fix or permanent product decision. The visible native title text beside the traffic lights and distinct titlebar color remain the unresolved seamless-appearance objective; they do not invalidate V4-WO-001.
- [x] **V4-WO-002:** remains a `draft` proposal only. It is explicitly tabled/deferred to the successor initiative; it was not admitted to v4 execution, authorized, implemented, reviewed, validated, or completed. No source, requirements, or blueprint changes are authorized by this proposal disposition.
- [x] **Successor ownership/gates:** the successor initiative owner/orchestrator must carry the proposal forward after this closeout, obtain a fresh design review and explicit human approval, then enforce a clean-build visual gate for title-text removal, continuous titlebar/app surface color, safe usable native controls/toolbar, and the preserved maximize/restore behavior. The successor initiative is not created here.
- [x] **Durable documents:** requirements and blueprint remain unchanged for the completed V4-WO-001 experiment; V4-WO-002's proposed updates remain conditional on later approval and successful visual verification.
- [x] **Git evidence / terminal status:** completed after fresh inventory. Commit `053a581d4a180cdc2cbe34b03470c639d08c024b` was fast-forwarded into `master`; the initial post-merge tree was clean. Full terminal evidence is recorded below and in the shared execution log.
- [x] **User authorization:** the user authorized committing the current entire cleanly inventoried cumulative tree and merging the continuation branch into `master`. This records permission, not completed Git actions.

## Terminal closeout evidence — v4 (2026-10-27)

- **Phase/status:** `closeout` / `closed`. V4-WO-001 remains `complete` with its independent `PASS WITH WARNINGS` validation and the recorded native-titlebar/animation observations. V4-WO-002 remains a proposal-only deferred successor candidate; it was not approved, implemented, reviewed, validated, or completed.
- **Cumulative initiative commit:** `053a581d4a180cdc2cbe34b03470c639d08c024b` (`Close out terminated v3/v4 titlebar diagnostics`), created on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v3` from base `6a6739c5263edee9896b32c00fc6f8470e35f3d6`.
- **Merge evidence:** fast-forward merge into `master`; no separate merge commit was created. `master` advanced from `6a6739c5263edee9896b32c00fc6f8470e35f3d6` to `053a581d4a180cdc2cbe34b03470c639d08c024b`, which is both the initiative commit and merged tip.
- **Inventory/verification:** exactly 18 initiative paths were committed: cumulative `apps/scriptcraft/src/main.rs`, the shared execution log, the v3/v4 plans, reviews, validations, and work-order artifacts (including the V4-WO-002 proposal). No unrelated or secret untracked files were present. Ignored `.DS_Store`, `.reference/.DS_Store`, and `target/` build output were not staged. `git diff --cached --check` passed.
- **Terminal branch evidence:** after the fast-forward, `git status --short --branch` reported `## master` with no changed paths; `HEAD` and `master` both resolved to `053a581d4a180cdc2cbe34b03470c639d08c024b`. The final closeout-record commit `1b01a1e15f39e3b106350a9ad8903c05b68bb195` was committed on `master`; after that commit, the worktree was clean, current branch was `master`, and `HEAD` and `master` both resolved to `1b01a1e15f39e3b106350a9ad8903c05b68bb195`.
- **Successor boundary:** no successor initiative was created. V4-WO-002 remains deferred pending a new initiative with its required review, explicit approval, and visual gate.
