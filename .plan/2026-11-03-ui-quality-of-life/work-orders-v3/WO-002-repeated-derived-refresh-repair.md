# V3-WO-002 — Repeated imported-scene derived-refresh repair

- **Status:** Production semantic-run repair and regression pass the explicitly authorized focused command (1 passed, 0 failed); user review, manual Cast List observation, and acceptance pending
- **Initiative:** [`ui-quality-of-life-v3-derived-refresh-and-scene-heading.md`](../ui-quality-of-life-v3-derived-refresh-and-scene-heading.md)
- **Depends on:** [`V3-WO-001`](WO-001-repeated-derived-refresh-oracle.md); unblocked by the user's supplied failing lifecycle and static source trace
- **Validation owner:** User, except for the exact focused command explicitly authorized for this continuation

## Objective

Repair the single established state-management or correction defect that prevents later relevant Character edits from refreshing an imported scene's derived data.

## Scope

1. Start from the WO-001 operation sequence, before/after summaries, and first failing stage. Identify whether the defect is in relevance classification, pending/failed/deferred state transitions, scoped correction, managed Cast List replacement, or scene-arc reconciliation.
2. Change only the owning path required for that branch. Likely inspection boundaries are [`classify_derived_invalidation()`](../../../crates/scriptcraft-core/src/editing.rs:1203), [`complete_derived_invalidation()`](../../../crates/scriptcraft-core/src/editing.rs:441), and [`correct_cast_lists_candidate()`](../../../crates/scriptcraft-core/src/cast_list.rs:105); this list is not authorization to alter all three.
3. Preserve the existing one-second desktop cadence, affected-scene scoping, irrelevant-edit suppression, and suspension of unchanged deferred/failed work.
4. Extend the WO-001 regression through the previously failing second and third updates, including semantic/source run equality and arc membership. Add undo/redo or save/reopen only where the established branch affects that boundary.
5. Retain complete Save/Save As preparation and all native writer, validation, identity, and source-preservation gates.
6. Record the exact root cause and production edit in this order. Update durable requirements or blueprints only if the reviewed behavior contract actually changes.

## Boundaries

- Do not clear every deferred/failed flag on every edit or retry unchanged work indefinitely.
- Do not replace per-scene deferral with fatal failure, or fatal mismatch with silent deferral.
- Do not rewrite normalized Character projection, Cast List ordering, nested arc ownership, or unrelated source.
- If WO-001 does not establish a defect, stop and request the missing fixture/evidence rather than guessing.

## Structural acceptance

- The established failing sequence now updates on every relevant second and later refresh.
- Irrelevant edits still perform no derived attempt; unrelated deferred/failed scenes remain suspended.
- Canonical Character text, supported source runs, independent scenes, history, dirty state, and native save behavior remain intact.

## Handoff

V3-WO-003 investigates the independent Scene Heading role-cycle divergence. No scheduler or Cast List behavior belongs to that order.

## Historical WO-008 closeout disposition — superseded by the user-evidence continuation below

- **Disposition at WO-008 authoring time:** Blocked; WO-001 had established no failing branch then. This snapshot is superseded by the subsequent user evidence and repair record below.
- **Evidence requirement:** [`WO-008 user-run evidence matrix`](WO-008-integration-and-closeout.md#user-run-evidence-matrix) records the repeated-refresh sequence with outcomes unpopulated. Do not start a repair until the user-run sequence or equivalent user evidence establishes a failing branch.

## Execution record — 2026-11-04, user-evidence continuation

- **User evidence:** In an imported document containing scene Character cues but no Cast List, inserting a new Cast List produced a successful first refresh. Later Character edits did not update the list; Save left it stale in the file, while Save/Save As rebase restored later refreshes. A new unsaved document worked. This narrows the branch to the first replacement of a newly inserted managed Cast List; it does not authorize changes to the already passing existing-list path.
- **Confirmed root cause:** [`replace_cast_list_text()`](../../../crates/scriptcraft-core/src/cast_list.rs:339) replaced the managed source Text projection while leaving semantic `runs` empty when the new Cast List had no prior run. The resulting semantic `text` was nonempty, but semantic runs remained empty while retained source contained the generated nonempty `<Text>` run. On the next relevant edit, `cast_list_source_is_replaceable()` passed empty expected runs into [`validate_managed_text_elements_replacement()`](../../../crates/scriptcraft-core/src/preservation.rs:1801); its exact empty-run mapping check rejected the nonempty source Text run as `UnsupportedTextMapping`. The scene was safely deferred, not fatally failed; deferred-only work then remained suspended until later relevant invalidation. Save serialization/reread normalized the run projection during rebase, explaining why later session updates resumed without proving Save had corrected the stale cast value.
- **Narrow production change:** [`replace_cast_list_text()`](../../../crates/scriptcraft-core/src/cast_list.rs:339) now creates one plain semantic [`TextRun`](../../../crates/scriptcraft-core/src/lib.rs:1) containing the canonical replacement when the element previously had no runs and replacement is nonempty. Existing runs still retain the first-run/full-text and later-empty-runs shape; empty replacement still clears runs. No preflight, deferral, failed-state, fatal-error, canonical Character, save preparation/rebase, ordering, scene-arc, or unrelated-source policy was changed. The existing-list path is not altered.
- **Exact coverage:** New [`v3_wo002_imported_inserted_cast_list_refresh.rs`](../../../crates/scriptcraft-core/tests/v3_wo002_imported_inserted_cast_list_refresh.rs) constructs an imported scene with Character data and no Cast List, completes its initial Complete refresh, then uses Enter and `change_role(Role::CastList)` to create the managed list. It covers the first refresh, three subsequent Character edits with affected-scene refreshes, Save and Save As through `save_session()` with rebased session assertions, and further post-rebase refreshes. It checks semantic and retained source Text runs, default clean pending state after each completed refresh/rebase, saved round trips, canonical Character text, and unrelated vendor/source content. This is authored but unrun.
- **Preservation of existing contracts:** Safe deferral for genuinely unsupported mappings, fatal errors, transactionality, native writer/preflight gates, canonical Character text, unrelated retained source, and Save/Save As publication/rebase are unchanged. No zoom or other work order was changed; no public API impact.
- **Validation stop:** No build, test, lint, formatting, implementation check, review, or other command was run. The regression and fix await user-authorized validation. WO-001's previous diagnostic test and desktop scheduler cases also remain authored but unrun.

## Authorized validation continuation — 2026-11-04

- **Initial failure diagnosis:** The user reported that the focused regression reached later Save/reopen verification after the repeated in-session refreshes had succeeded, then failed to find the Cast List by its original `StableId`. This was test identity drift, not evidence of a role/source save defect: semantic `StableId` values are session-local and can be regenerated by Save/Save As rebase or native reopen, while the FDX paragraph ID is the durable identity.
- **Narrow test correction:** [`v3_wo002_imported_inserted_cast_list_refresh.rs`](../../../crates/scriptcraft-core/tests/v3_wo002_imported_inserted_cast_list_refresh.rs) now captures the inserted list's `source_provenance.foreign_id` and resolves semantic elements and retained managed source by that foreign paragraph identity across Save, Save As, and reopen. Assertions still require `Role::CastList`, canonical semantic text, exact semantic/source Text-run equality, canonical Character values, and retained unrelated vendor/source content. Save preparation, publication, clean-state/rebase checks, and all native FDX round-trip checks remain intact. Failure messages now identify the lifecycle step, durable foreign ID, and semantic candidates.
- **Production disposition:** No WO-002 production code changed during this continuation. The supplied failing lookup was caused by carrying a transient semantic ID across an identity-reconstruction boundary; the durable foreign paragraph identity resolves the same Cast List and all role/source assertions pass.
- **Exact authorized agent-run command:** `cargo test -p scriptcraft-core --test v3_wo002_imported_inserted_cast_list_refresh -- --nocapture` — **exit status `0`**, `1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. Compilation completed with existing warnings. This is the only WO-002 command run in this continuation; no other core tests or broader checks were run. The earlier test-sequencing failure was in Save/reopen lookup using a transient semantic `StableId`; changing the test to use the durable foreign FDX paragraph ID resolved that lookup without changing production code in this continuation.
- **Disposition:** The production semantic-run repair and supplied regression pass with Save/Save As/reopen checks preserved. The identity-drift correction adds no production behavior. Manual Cast List desktop observation, broader validation, user review, and acceptance remain outstanding.
