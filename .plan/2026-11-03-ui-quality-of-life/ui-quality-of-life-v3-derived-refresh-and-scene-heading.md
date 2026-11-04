# UI Quality of Life v3 — Repeated derived updates and Scene Heading synchronization

**Status:** Current-state and feedback analysis only; no v3 work orders drafted or implementation authorized  
**Supersedes as active working context:** [`ui-quality-of-life-v2-zoom-and-typing.md`](ui-quality-of-life-v2-zoom-and-typing.md), while preserving that record's history  
**Validation owner:** User for all checks, reproduction, review, and acceptance  

> **Policy boundary:** v3 analysis used static source inspection and the user's reported desktop observations. No builds, tests, formatting, lint, profiling, desktop automation, implementation checks, reviews, or independent validators were run. Previously authored tests remain unrun by agents unless the user separately records otherwise. [`project-manifest.md`](../../.forge/project-manifest.md:73) remains authoritative.

## 1. Current working state

| Area | Current recorded state |
|---|---|
| Render-speed v1 | RS-WO-001 through RS-WO-012 were implemented/authored. Viewport-bounded painting, fixed-cell galleys, wrap reuse, interaction indexes, copy-on-write preservation, and scoped validation exist. Full user validation remains outstanding. |
| Zoom v2 | V2-WO-001.1 and V2-WO-001.2 were recorded as implementing viewport-center anchor capture and one-shot two-axis scroll compensation. In the user's desktop test, those changes did not take effect: zoom still feels anchored to the existing scroll spot rather than preserving the document point at the viewport center. No diagnosis or zoom acceptance is inferred. |
| Derived scheduling v2 | Irrelevant ordinary edits and provably irrelevant structural Action edits no longer schedule derived work. Character/scene changes use affected-scene scopes where provable, with complete fallback for ambiguity. |
| Failed/deferred refresh v2 | Fatal failed scopes remain pending but suspend until relevant invalidation; safe-stop deferral is tracked separately. The user's latest report confirms no continuing Cast List freeze or error, but does not validate the complete state machine. |
| Cast List correction v2 | Unsupported stale-list source structures are preflighted and deferred before arc reconciliation; supported independent scenes may update. The original large user fixture remains unavailable as a focused regression artifact. |
| Source indexing v2 | Derived removal safety, reserved-ID queries, and managed-heading lookup now use transient indexes instead of per-identity or per-scene complete-source scans. Publicly mutable preservation collections require the documented maintenance boundary after out-of-band edits. |
| Scene Heading v2 investigation | V2-WO-002.6 was blocked because the failure was not established. The latest user report supplies a diagnostic site and saved XML shape, so that blocked investigation is superseded by the v3 findings below; no production fix has yet been authorized. |
| Deferred optimization | V2-WO-004 through V2-WO-006 were not started. They are not part of the current v3 analysis unless explicitly reconsidered later. |

## 2. New user feedback

1. Cast List refresh no longer freezes or throws an error, but it updates only once and does not reflect later document edits.
2. Repeated Tab/Enter role cycling can eventually produce ManagedTextMismatch at the text-replacement site when entering text in a Scene Heading.
3. Attempting to delete that Scene Heading produces StructuralAttachmentConflict.
4. Saving in this state succeeds and emits a Scene Heading containing generated SceneProperties and an empty SceneArcBeats container, but no Text element.
5. In the user's desktop test, V2-WO-001.1 and V2-WO-001.2 did not take effect: zoom still feels anchored to the existing scroll spot rather than preserving the document point at the viewport center. This records the observed behavior without identifying a cause.

These are correctness/state-management issues, not merely remaining performance costs.

## 3. Finding — repeated Cast List updates require a state-sequence reproduction

A successful first update is expected to clear the completed pending scope in [`complete_derived_invalidation()`](../../crates/scriptcraft-core/src/editing.rs:441). A later Character text or role change should produce a new affected-scene scope through [`classify_derived_invalidation()`](../../crates/scriptcraft-core/src/editing.rs:1203), which compares ordered Character identity/text projections by owning scene. A due scheduler tick should then call [`refresh_cast_lists_with_outcome()`](../../crates/scriptcraft-core/src/editing.rs:505) again.

The source therefore does **not** support a simple conclusion that completed scenes are permanently marked clean. At least one of the following must be happening in the user's sequence, but static inspection cannot select among them:

- the later edit is not producing the expected affected-scene invalidation;
- the scene remains in safe-deferred or failed state and the later edit is not clearing/reactivating that state as intended;
- a refresh runs but its correction considers the scene unchanged or defers it;
- the visible Cast List and the scene being edited are not the same owning scene after structural role/topology changes.

The preflight added in [`cast_list_source_is_replaceable()`](../../crates/scriptcraft-core/src/cast_list.rs:252) can return a safe-deferred outcome without an error, while [`scene_needs_cast_list_update()`](../../crates/scriptcraft-core/src/cast_list.rs:236) decides whether list replacement is currently needed. After the first update, multi-run lists contain a full first run and empty later runs in both semantic and preserved projections, so a second update must be exercised explicitly rather than assuming the first-update tests cover it.

No dedicated repeated-update sequence was found in the current authored Cast List tests: single update/save/reopen and deferral cases do not establish second/third Character edits across actual pending-state completion and scheduler reactivation. The highest-value missing evidence is a deterministic sequence of at least two successive Character changes in one imported scene, recording the value-free pending/retryable/deferred/failed summary and requested/attempted/changed scene counts after each edit and refresh.

This is an unresolved diagnosis, not evidence for a particular fix. Clearing every deferred/failed flag on every edit would be unsafe and could recreate the former retry loop.

## 4. Finding — the saved empty Scene Heading shape is legal

The absence of a Text element is not by itself malformed or evidence of save corruption.

The native writer explicitly removes managed Text elements when semantic text is empty in [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:2173). The reader also projects wholly empty Text runs to no semantic runs in [`make_semantic_element()`](../../crates/scriptcraft-core/src/fdx.rs:3720). This matches the durable empty-managed-paragraph contract: empty canonical text has no semantic runs and no direct managed Text children, while role, identity, attachments, and other source-owned children remain.

The text-replacement path also has explicit support for a childless managed paragraph. In [`replace_text_element_children()`](../../crates/scriptcraft-core/src/preservation.rs:2821), an empty old text plus zero-offset insertion creates a namespaced Text element and then inserts content.

Therefore, typing into a genuinely empty semantic/source Scene Heading should not produce the reported mismatch. The saved XML is a useful fixture shape, but the missing Text element alone does not explain the error.

## 5. Finding — the text-replacement error proves an earlier semantic/source divergence

The new diagnostic localizes the failure to [`replace_managed_text()`](../../crates/scriptcraft-core/src/preservation.rs:1606) and its helper, rather than scoped validation, whole-list replacement, or paragraph removal.

The immediate failing comparison is in [`replace_text_element_children()`](../../crates/scriptcraft-core/src/preservation.rs:2821): the concatenated source Text-run content must equal the semantic element's prior text before applying the edit. The direct-text-child branch has the same requirement in [`replace_managed_text()`](../../crates/scriptcraft-core/src/preservation.rs:1606).

Consequently, the user's sequence establishes this stronger finding:

- at the moment text insertion was attempted, the semantic Scene Heading's prior text and its retained in-memory source Text projection disagreed;
- the eventual saved empty paragraph shows the source shape after later deletion/save, but does not reveal the semantic text or run state at the earlier failure;
- role cycling, split/join, derived refresh, or an earlier failed/rolled-back candidate must be examined to find where that divergence was introduced;
- forcing insertion into a childless paragraph or deleting this check would conceal rather than repair the synchronization defect.

The reproduction must capture the exact role sequence and the semantic text/runs immediately before the failing insertion. The current XML fixture should be paired with a role-cycle sequence that reaches a nonempty semantic Scene Heading while its source has no Text, or otherwise produces unequal old text/source segments. That precise before-state is still not established.

## 6. Finding — Scene Heading deletion has a direct attachment guard

Structural deletion is a separate, deterministic issue. [`remove_managed_paragraph()`](../../crates/scriptcraft-core/src/preservation.rs:1997) requires [`is_plain_managed_paragraph()`](../../crates/scriptcraft-core/src/preservation.rs:2708). A generated SceneProperties child is not a Text element or whitespace node, so the paragraph is not plain and removal returns StructuralAttachmentConflict before identity/provenance checks or source removal.

Thus the reported deletion error is fully explained by the current implementation. The guard does not distinguish app-owned, successfully generated SceneProperties/SceneArcBeats from unknown authored nested data.

A related guard exists in [`change_managed_role()`](../../crates/scriptcraft-core/src/preservation.rs:1802): entering or leaving Scene Heading while attached or unknown nested data is present also returns StructuralAttachmentConflict. This makes role-cycle behavior and deletion behavior part of the same ownership question, even though the text-replacement mismatch is a distinct failure.

The saved shape has an empty generated arc, but the current removal guard does not use that fact. Any remediation would need an explicit, bounded distinction between safely generated scene containers and opaque/authored attachments; simply deleting every attached Scene Heading would violate the existing preservation contract. No remediation is drafted in this document.

## 7. Investigation evidence needed before v3 work orders

### Repeated Cast List update

- One imported scene with a supported Cast List and at least two Character cues.
- Two or more successive Character insert/rename/remove operations, including the exact role-creation method used in the desktop.
- Value-free pending/retryable/deferred/failed state and scene counters after each operation.
- Whether the first update was a complete activation refresh or an affected-scene refresh.

### Scene Heading text failure

- The exact Tab/Enter sequence and starting role/text, reduced to the shortest repeating cycle.
- The semantic text/runs and retained Text-run projection immediately before the failing insertion.
- The newly reported text-replacement category retained in the error record.
- The supplied empty-heading XML shape used as one branch of the fixture, not treated as the complete explanation.

### Scene Heading deletion/role attachment

- A generated empty SceneProperties/SceneArcBeats case.
- A supported generated arc containing nested Character content.
- An unknown/authored attachment control that must continue to safe-stop.

## 8. Gate

This v3 record intentionally contains no work orders. The next planning step requires user review of these findings and explicit authorization of a remediation scope. No v3 implementation should begin from this analysis alone.
