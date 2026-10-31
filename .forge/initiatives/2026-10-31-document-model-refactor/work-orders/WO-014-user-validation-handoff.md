# WO-014 user-owned validation handoff

This document is an execution aid for the user, not a test result, review, or acceptance record. Under the [repository-wide policy](../../../project-manifest.md#authoritative-internal-review-and-validation-policy), the user owns all review and validation. Agents must not run these commands or scenarios, inspect results as reviewers, perform manual checks, or conduct external-app experiments.

## Current evidence boundary

- WO-012 implementation is recorded at `521774d2719cf68c9309411ecb1d40248d822474`; WO-013 implementation is recorded at `2c9bc8b72b6f7a386d1c190d3527e81bfb3280c5`. Both remain **unvalidated**. Commits, code, test additions, and earlier recorded check output are implementation/history only; none is user validation evidence for these revisions.
- WO-014 validation is **not performed**. The scenarios and commands below are proposals for user execution. No pass, compatibility result, review, or acceptance is claimed here.
- The initiative records material boundaries for FDX state, persistence and recovery in its [WO-012/WO-013 status](../document-model-refactor-v1.md#native-fdx-cutover-status-and-user-handoff-2026-10-31) and [WO-013 status](../document-model-refactor-v1.md#wo-013-json-sunset-and-recovery-status-2026-10-31). Treat those as implementation descriptions, not proof that the behavior works.

## Safe fixture and provenance rules

1. Prefer fixtures newly generated from scratch with synthetic screenplay text, invented character/location names, and no personal, client, unpublished, or otherwise sensitive material. Do not copy populated user documents into the repository, test artifacts, logs, screenshots, or issue reports.
2. For advanced-structure cases, build a minimal, isolated fixture from a public specification or a user-created synthetic document. If an image is needed, generate a tiny synthetic image and embed only that image. Never resolve or publish a source document's recorded local paths; never copy embedded payloads, local paths, IDs, or metadata from private files. Do not place raw FDX payloads in validation reports.
3. Record fixture provenance in non-sensitive terms: `synthetic`, `public-spec-derived`, or `Final-Draft-generated synthetic`; include the generating application/version only when actually known, a short fixture label, and a cryptographic digest if useful. Do not record a user's home directory or full source/destination path. Keep test files in a disposable, user-controlled temporary directory outside the repository unless the user deliberately approves a sanitized fixture for version control.
4. Use copies for destructive or migration cases. Preserve a baseline copy; compare only the synthetic input/output or a redacted structural summary. Never experiment on originals. A separate output/backup protects the source but does not prove the resulting file is valid.

## Suggested automated command sequence

Run from the repository root, as the user, after deciding the checkout and toolchain are the intended validation target. Capture command, exit status, date, OS/toolchain identity and exact tested commit. These commands are not authorized for agent execution.

1. Focused integrated core tests, including the existing WO-008 end-to-end target:

   [`cargo test -p scriptcraft-core --test wo008_e2e`](../../../../Cargo.toml:1)

2. Core and desktop package suites, then the complete workspace suite:

   [`cargo test -p scriptcraft-core`](../../../../Cargo.toml:1)

   [`cargo test -p scriptcraft`](../../../../Cargo.toml:1)

   [`cargo test --workspace`](../../../../Cargo.toml:1)

3. Applicable repository checks, preserving exact results rather than summarizing them as interoperability evidence:

   [`cargo fmt --all -- --check`](../../../../Cargo.toml:1)

   [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1)

   [`cargo check --workspace`](../../../../Cargo.toml:1)

   [`cargo build --workspace`](../../../../Cargo.toml:1)

The commands establish only what their executed tests/checks cover. They do not replace desktop interaction, real Final Draft scenarios, user review, or evidence for untested formats/versions. If a command is unavailable or fails, record its exact command, exit status, concise sanitized diagnostic, affected scenario and disposition; do not modify or delete user data to force a pass.

## End-to-end scenario matrix

Execute only scenarios suitable for the user's environment and fixtures. Record `not run`, `pass`, `fail`, or `blocked` per row; do not infer a pass from another row. For failures, preserve a minimal synthetic reproducer and sanitized diagnostics.

| ID | Scenario and user observations to record |
|---|---|
| E2E-01 | **New native document:** create a new document in the desktop app, save as FDX in the disposable area, close/reopen it, and record extension, declared Version 6, visible content, clean/dirty state, and any error. Verify that the chosen file is the one reopened. |
| E2E-02 | **Version 6 no-op and Save As isolation:** open a synthetic Version 6 document; make an untouched Save As to a new destination; compare source and destination structurally and note any lexical normalization. Confirm the source remains unchanged. Reopen the output in ScriptCraft and, for the real-app leg, the identified external editor. Do not equate parseability with preservation. |
| E2E-03 | **Observed roles/order and visibility:** cover all 17 types observed in the initiative's supplied sample (six conventional types plus Note, New Act, End of Act, Cast List, General, Shot, Sequence, Outline 1–3, Summary) using synthetic text. Record role, order, text, hidden/visible and page behavior separately. Presence of a role label does not establish identical external semantics. |
| E2E-04 | **Title/source and unknown data:** include ordered title-page paragraphs, blank paragraphs, alignment/style attributes, headers/footers, unknown section/inline content and mixed runs in a synthetic fixture. Perform a no-op and an edit outside an unknown range, then inspect whether structure remains and whether any operation is refused. Do not infer title metadata from paragraph position. |
| E2E-05 | **Scene and board data:** use synthetic nested scene attachments and independently authored board/item/image records with an embedded synthetic image and internal links. Exercise a no-op and ordinary script edits including scene-boundary changes. Record source-owned data and internal-link changes separately from the explicitly permitted silent script/board cross-domain orphaning. Never use a real image payload or recorded local path. |
| E2E-06 | **Editing, cast refresh, undo and save:** use multiple synthetic scenes with repeated character cues. Exercise insertion, replacement, role change, split/join, delete and reorder; test scene-boundary updates, final scene, delayed cast refresh (about one second in the current implementation description), direct list edit, undo/redo, dirty state, pending refresh during save and Save As. Record whether the live and serialized list agree only after a successful save, without assuming Final Draft's implementation/timing. |
| E2E-07 | **Save failure and conflict safety:** on disposable synthetic files, exercise destination exists/no-overwrite Save As, external modification conflict, invalid extension, serialization/validation failure, and any available injected storage failure. Compare pre/post destination, live edits, path, clean/dirty state and preservation baseline. Record the post-publication temporary-cleanup warning separately if encountered; do not claim stronger atomicity than the storage layer provides. |
| E2E-08 | **Version paths:** use separate synthetic files declaring Version 6, an explicit older version and an explicit newer version. Confirm warning/consent and cancellation behavior, importer attempt for non-6 input, no open-time source write, older title/content conversion and General fallback where applicable, and Version 6 output only after explicit successful save. Separately try missing, malformed/duplicate version, missing content, unconvertible type and malformed XML cases; record incompatible-document handling and source/session preservation. Do not treat changing the version attribute as migration evidence. |
| E2E-09 | **JSON sunset and recovery limits:** try a synthetic legacy JSON document and a harmless synthetic legacy recovery artifact. Record bounded refusal, no accidental FDX interpretation, and no reads/writes/rotation/deletion of the fixture artifacts. Do not treat retained internal JSON codec tests as a user migration or recovery path. Do not expect replacement snapshot recovery: none is promised. |
| E2E-10 | **Untrusted input and privacy:** use isolated synthetic malformed, oversized, deeply nested, entity/DTD/XXE, invalid-encoding and embedded-image-limit inputs as safe to prepare. Observe typed failure, limits, absence of external file/network resolution, no destination/session replacement, and diagnostics that contain no screenplay text, image payload or local path. Do not use a real external path as a probe. |
| E2E-11 | **Desktop workflow:** interact with New, Open, Save, Save As, migration dialog, cancellation, reopen, close/dirty behavior and error/status presentation using the synthetic cases above. Record OS, app build/commit, steps and visible outcomes; this is distinct from headless tests. |

The matrix covers nominated scenarios, not every possible FDX producer, encoding, namespace, XML construct, application release, OS, or foreign reference. Unknown links may be name-based, positional, implicit or undocumented. A result on a synthetic fixture does not establish behavior on arbitrary existing files.

## Real Final Draft compatibility procedure (user-owned, if available)

1. Identify the exact Final Draft release/version and OS. Record them and how the synthetic fixture was generated, without including a private installation path or account/license data. If no identifiable installation is available, mark all real-app rows `blocked` or `not run`; do not substitute XML inspection or another editor.
2. In an isolated temporary workspace, create or open only permission-safe synthetic FDX. Include selected cases: each targeted role, formatted multi-run text, title-page layout, notes versus hidden Note paragraphs, scene numbering/dual-dialogue/manual-break cases only where safely constructible, nested scene data, and a tiny synthetic board image. Keep a pristine baseline and a separate working copy.
3. In Final Draft, open the baseline; inspect visible roles, title page, notes, scene properties, cast display, and board/image links that the fixture actually contains. Save a separate copy and reopen it in Final Draft. Record what the UI showed and which structural fields changed, without assuming XML retention proves semantic interpretation.
4. Open the disposable copy in ScriptCraft; record any warnings or refusals. Perform one bounded edit at a time (body text, role/scene boundary, title-independent edit, and cast cue) and save to a new file. Reopen that output in ScriptCraft and Final Draft. Compare synthetic content, ordering, formatting, hidden/visible behavior, internal links and any lost/changed fields. Use separate copies for migration, destructive edits and app resaves.
5. Where practical, repeat with a Final Draft-generated output and a ScriptCraft-generated output, each saved/reopened by both applications. Report every tested version pair separately. Stop if the fixture contains unexpected private data or if a destructive operation could affect authored content.

External-app observations support only the exact fixture, operation, versions and platform recorded. They do not establish general Final Draft interoperability, schema conformance, compatibility with other Final Draft releases, or that Final Draft will regenerate omitted/stale content. WO-014 must make **no general Final Draft interoperability claim**.

## Limitations to carry into results and closeout

- **Storage:** describe the existing same-directory replacement/Save As behavior and documented platform-specific atomicity, final external-writer race, durability, metadata/ACL limitations. A successful check on one filesystem/platform cannot strengthen those guarantees.
- **Recovery:** WO-013 retired legacy JSON recovery entry points and discovery; existing JSON/recovery artifacts remain on disk but are not a supported desktop restore path. No replacement FDX snapshot recovery capability was added. Do not promise restoration of edits or old recovery snapshots.
- **Legacy JSON:** desktop JSON open/save is sunset; residual core codec helpers/tests are internal compatibility residue, not supported import, migration, or persistence. Existing files are not deleted by this change, but are not openable/recoverable in the desktop.
- **FDX compatibility:** real application version/OS, version migration, unknown source content, internal references, title/layout semantics, boards/images, hidden content, and external regeneration are bounded by actual evidence. No test subset, synthetic fixture, or XML validity implies blanket interoperability.
- **Historical checks:** keep prior command outputs and implementation records dated and labeled historical. They are not re-runs for WO-012/013's cited commits and do not satisfy WO-014. All user scenarios and outcomes must be newly recorded with their own tested commit and environment.

## User result record template

Copy this template into a user-owned validation record when executing the handoff. Do not fill it with inferred or agent-generated outcomes.

```text
WO-014 user validation record
Tested commit:
Date/time and timezone:
OS / architecture:
Rust toolchain (for commands):
Final Draft version (or unavailable):
Fixture labels and provenance (no sensitive paths/data):
Commands, exact exit statuses, and sanitized output references:
Scenario IDs and status (not run / pass / fail / blocked):
Observed failures, limitations, and issue references:
Synthetic evidence vs. actual external-app evidence:
User review disposition:
User validation disposition:
WO-014 acceptance: pending / accepted by user (evidence reference):
```

Until the user supplies and records results, WO-014 evidence and final acceptance remain pending. This handoff does not mark the original evidence requirement complete.
