# Validation — WO-007b/WO-007c Parenthetical Fix Chain (v2)

## Validation metadata

- **Validation status:** `PASS WITH WARNINGS`
- **Validated chain:** [`WO-007-parenthetical-transition-and-boundary-policy.md`](../work-orders/WO-007-parenthetical-transition-and-boundary-policy.md) → [`WO-007a-parenthetical-advance-boundary-fix.md`](../work-orders/WO-007a-parenthetical-advance-boundary-fix.md) → [`WO-007b-rendered-parenthetical-key-route.md`](../work-orders/WO-007b-rendered-parenthetical-key-route.md) → [`WO-007c-delimiter-only-parenthetical-delete.md`](../work-orders/WO-007c-delimiter-only-parenthetical-delete.md)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-WO-007bc-validation-v2-2026-10-27`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Product-code changes:** none
- **Existing-artifact changes:** none; this validation artifact is the only artifact created by this validation

## Validation boundary and comparison basis

This validation assesses only: (1) WO-007b rendered pointer mapping and both
focused Tab/Enter production routes without direct source-offset or private-key
helper bypass; (2) WO-007c exact delimiter-only `(`, `()`, and `)` Delete/Tab
behavior, first/only/predecessor fallback, undo/redo, and preservation; and (3)
preservation of WO-007, WO-007a, WO-005, and WO-005a contracts, scope, durable
impact, and required Cargo gates. It does not reopen completed geometry,
responsiveness, layout, export, blink, or native-only behavior.

The comparison basis was the validation process in
[`05-validation.md`](../../../process/05-validation.md:1), the final WO-007
design review in [`design-review-wo-007.md`](../reviews/design-review-wo-007.md:162),
the failing prior validation in
[`validation-WO-007a-v2.md`](validation-WO-007a-v2.md:88), the four work-order
artifacts above, the WO-007c handoff in
[`validation-WO-007c-v2.md`](validation-WO-007c-v2.md:11), the execution evidence
in [`execution-log.md`](../execution-log.md:121), current production source in
[`main.rs`](../../../../apps/scriptcraft/src/main.rs:502) and
[`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:163), focused
tests in [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:134),
the predecessor evidence in
[`validation-WO-005a-v3.md`](validation-WO-005a-v3.md:81), and the project gates
in [`project-manifest.md`](../../../project-manifest.md:75).

The implementation evidence, current source, and focused tests are sufficiently
consistent for a meaningful comparison. Command execution was not available in
this fresh repository-reading session; Cargo results below are therefore
recorded evidence, not independently rerun results.

## Findings

### PASS-001 — Rendered pointer mapping reaches the approved inner-text boundary

- **Severity:** pass
- **Acceptance coverage:** WO007B-AC-001 and the missing evidence identified by `EGC-WO007A-F-001`.
- **Evidence:** [`rendered_parenthetical_inner_end_routes_pointer_and_both_keys()`](../../../../apps/scriptcraft/src/main.rs:1483) renders `(words)`, derives the visual point after the final inner character and before `)`, sends a pointer move/click through the rendered surface, and asserts the resulting selection is the UTF-8-safe offset `"(words".len()` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1547).
- **Assessment:** The test does not assign the source selection offset before the pointer event. The selection assertion occurs after the production rendered surface and pointer handling have supplied the position, so the former direct-offset bypass is removed.
- **Required next state:** None.

### PASS-002 — Focused Tab and Enter use the production surface route

- **Severity:** pass
- **Acceptance coverage:** WO007B-AC-002, WO007B-AC-003, and the hard-stop requirement from `EGC-WO007A-F-001`.
- **Evidence:** The same focused regression independently runs `Key::Tab` and `Key::Enter` through [`handle_surface_input()`](../../../../apps/scriptcraft/src/main.rs:739), which calls [`dispatch_surface_events()`](../../../../apps/scriptcraft/src/main.rs:635) and then the production [`key_event()`](../../../../apps/scriptcraft/src/main.rs:708) dispatch. The key loop is visible at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1484), and each case asserts preserved `(words)`, a new Dialogue element, and selection at element `1`, offset `0` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1576).
- **Assessment:** Both commands are exercised from a focused rendered surface. The regression neither calls the private key helper directly nor substitutes a core-only call for the production event path. The prior WO-007a hard stop is satisfied.
- **Required next state:** None.

### PASS-003 — Exact delimiter-only Delete behavior and fallback branches

- **Severity:** pass
- **Acceptance coverage:** WO007-AC-002 as amended by WO-007c.
- **Evidence:** [`is_delimiter_only_parenthetical()`](../../../../crates/scriptcraft-core/src/editing.rs:635) is bounded exactly to `(`, `()`, and `)`. [`EditingSession::delete()`](../../../../crates/scriptcraft-core/src/editing.rs:163) handles every valid caret offset in those shapes, replaces a first or only delimiter-only element with empty Dialogue at element `0`, offset `0`, and removes a later element to the predecessor's text end at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:174). The focused matrix iterates all three exact shapes and all valid offsets, then separately asserts first-with-successor, only-element, and predecessor outcomes at [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:161).
- **Assessment:** The WO-007c first-element correction is present and bounded. The successor remains when the first element is replaced, the one-element invariant is preserved for the only-element case, and later elements use the predecessor fallback. No new delimiter shape or arbitrary bracket behavior was introduced.
- **Required next state:** None.

### PASS-004 — Exact delimiter-only Tab conversion and setting precedence

- **Severity:** pass
- **Acceptance coverage:** WO007-AC-001 and WO007-AC-002, within WO-007c's exact-shape scope.
- **Evidence:** [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:330) converts only the exact delimiter-only Parenthetical shapes through the bounded predicate, when `auto_parenthetical` is enabled, to the same element as empty Dialogue at offset `0` at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:352). The focused Tab matrix covers every valid offset for `(`, `()`, and `)` and asserts role, text, stable element identity, and resulting selection at [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:238). The setting-off precedence remains covered by [`dialogue_insertion_and_setting_off_precedence_are_exact()`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:68).
- **Assessment:** Tab cleanup is exact-shape and setting-gated. The fix does not broaden cleanup to user-authored non-empty text or unsupported malformed arrangements.
- **Required next state:** None.

### PASS-005 — Undo/redo and preservation are transactional

- **Severity:** pass
- **Acceptance coverage:** WO007-AC-002, WO007-AC-003, and WO-007c transactional/history requirements.
- **Evidence:** The Delete matrix snapshots each first, only, and predecessor case before mutation and checks undo restores both project and selection, then checks redo restores the converted/removal result at [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:171). The Tab matrix checks one-step undo and redo for every exact shape and offset at [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:256). The shared [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:478) boundary creates one history entry only when project or selection changes and updates revision/dirty state at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:500).
- **Assessment:** The new branches use the existing transactional snapshot mechanism; no direct project mutation escapes history or rollback handling. Non-collapsed deletion remains delegated to ordinary range deletion at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:165), and the focused suite retains that coverage.
- **Required next state:** None.

### PASS-006 — WO-007 and WO-007a semantic contracts remain preserved

- **Severity:** pass
- **Evidence:** The focused suite retains supported `(text`, `text)`, and `text` repair behavior, UTF-8-safe inner-end handling, strict-boundary no-ops, before-opening Enter, setting compatibility, persistence, cache identity, and rollback coverage in [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:268). The non-empty closed-pair route remains implemented by [`parenthetical_repair_boundary()`](../../../../crates/scriptcraft-core/src/editing.rs:639), and WO-007b changes only the rendered app regression according to its implementation record at [`WO-007b-rendered-parenthetical-key-route.md`](../work-orders/WO-007b-rendered-parenthetical-key-route.md:55).
- **Assessment:** WO-007c changes the authorized delimiter-only first-element Delete fallback only. It does not alter the WO-007a boundary predicate, non-empty repair/advance semantics, settings, persistence, provenance, or rendered route. The exact delimiter-only behavior is treated as the WO-007c amendment to the earlier empty-delete branch, not as a new general transition policy.
- **Required next state:** None.

### PASS-007 — WO-005 and WO-005a predecessor contracts remain preserved

- **Severity:** pass
- **Evidence:** The chain work orders explicitly preserve the completed predecessor history and do not change completed predecessor artifacts. The prior scope-corrected validation records the authorized WO-005a provenance behavior, undo/redo, rollback, persistence, and preservation contracts as passing at [`validation-WO-005a-v3.md`](validation-WO-005a-v3.md:81). Current WO-007 source retains transient generated-parenthetical invalidation in [`invalidate_changed_generated_parentheticals()`](../../../../crates/scriptcraft-core/src/editing.rs:528), while the focused WO-007 tests retain setting, provenance, user-authored, and persistence coverage.
- **Assessment:** No evidence shows reopening, weakening, or silently importing rejected predecessor findings. WO-005 and WO-005a remain historical completed prerequisites and WO-007 remains the sole authority for the new caret-sensitive behavior.
- **Required next state:** None.

### PASS-008 — Scope and durable-document classification are aligned

- **Severity:** pass
- **Evidence:** WO-007b limits its product change to the rendered app test and classifies requirements/blueprints as `verify unchanged` at [`WO-007b-rendered-parenthetical-key-route.md`](../work-orders/WO-007b-rendered-parenthetical-key-route.md:48). WO-007c limits its product delta to the delimiter-only first-element fallback and focused core tests at [`WO-007c-delimiter-only-parenthetical-delete.md`](../work-orders/WO-007c-delimiter-only-parenthetical-delete.md:40). The current evidence shows no layout, pointer-policy, persistence-schema, settings, native-only, or WO-006 change.
- **Assessment:** The changed behavior is subordinate to the approved WO-007 fix chain. The durable impact is correctly `verify unchanged`; this validation artifact is the only artifact created by this validation.
- **Required next state:** None.

### PASS-009 — Required Cargo gates are recorded as passing

- **Severity:** pass with warning
- **Evidence:** The work-order and execution records report passing [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1) at [`execution-log.md`](../execution-log.md:122) and [`WO-007c-delimiter-only-parenthetical-delete.md`](../work-orders/WO-007c-delimiter-only-parenthetical-delete.md:45). The focused WO-007 suite is recorded as 10 passing tests at [`validation-WO-007c-v2.md`](validation-WO-007c-v2.md:19).
- **Assessment:** These results are sufficient recorded gate evidence under the available repository-inspection capability, but were not independently rerun in this session. Project policy defines no required native/manual hard-stop gate at [`project-manifest.md`](../../../project-manifest.md:88); native typing smoke was not performed and is not inferred.
- **Required next state:** Note only; no fix work order is required.

## Fail findings

None. The former direct-offset/Enter-only hard stop is closed by the rendered
pointer and focused dual-key regression. No plainly unmet required criterion,
clearly unrelated scope, rogue behavior, or failed required policy gate was
found within this validation boundary.

## Final verdict and exact next state

- **Implementation alignment:** **PASS.** WO-007b satisfies the missing rendered pointer and both focused production-route requirements without direct offset or private-helper bypass. WO-007c satisfies exact `(`, `()`, and `)` delimiter-only Delete/Tab behavior, first/only/predecessor fallback, transactional undo/redo, and preservation. WO-007, WO-007a, WO-005, and WO-005a contracts, scope, durable classification, and recorded Cargo gates remain aligned.
- **Warnings:** Cargo gates were recorded rather than independently rerun; native typing smoke was not performed and is not inferred. These are notes under current project policy, not hard stops.
- **Fix work order required:** `no`.
- **Exact next state:** The orchestrator may close the complete chain by marking [`WO-007c-delimiter-only-parenthetical-delete.md`](../work-orders/WO-007c-delimiter-only-parenthetical-delete.md), [`WO-007b-rendered-parenthetical-key-route.md`](../work-orders/WO-007b-rendered-parenthetical-key-route.md), [`WO-007a-parenthetical-advance-boundary-fix.md`](../work-orders/WO-007a-parenthetical-advance-boundary-fix.md), and [`WO-007-parenthetical-transition-and-boundary-policy.md`](../work-orders/WO-007-parenthetical-transition-and-boundary-policy.md) `complete`, linking this validation from the chain records and execution log, and proceeding to the initiative's normal closeout. Do not infer native smoke, reopen WO-005/005a, or create another fix work order from the recorded gate warning.
