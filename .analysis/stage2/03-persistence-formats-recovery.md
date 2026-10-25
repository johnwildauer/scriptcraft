# Stage 2 — Persistence, screenplay formats, interoperability, and recovery

## Scope and method

This is a source-based assessment of WordCraft's file persistence and the implications for a
small screenplay-writing application. It separates **observed facts** (the current repository)
from **recommendations** (a proposed screenplay product design). The traced path is:

`file.open` / `file.save` command → engine I/O dispatch → format reader/writer → bounded or
atomic filesystem boundary → session dirty/path state.

The candidate screenplay interchange set is DOCX, Final Draft's FDX, plain text, and PDF. FDX is
not currently implemented or named in the engine's open/save extension tables; conclusions about
FDX below are therefore format/product recommendations, not claims about an existing adapter.

## Executive assessment

WordCraft already has a credible persistence substrate for a single-user editor: a central
extension/content-signature dispatcher, explicit `Result` errors, size and resource bounds,
automation read/write roots, and a sibling-temp-file save path that protects the destination from
serialization/preparation failure. DOCX is the strongest rich interchange target, while PDF is an
output artifact and plain text is a deliberately lossy escape hatch. The current JSON writer is
the closest thing to a native model snapshot, but it is exposed as an agent-oriented format rather
than a versioned screenplay project format.

For a screenplay app, the minimum trustworthy design should use an **owned, versioned native
project format** as the canonical save format, and treat DOCX/FDX/TXT/PDF as explicit import/export
boundaries. The native format should preserve screenplay roles, stable element IDs, project
metadata, and future assets without making every export format the source of truth. A journal or
rotating recovery snapshot should be separate from the normal save file. Existing safe-save is
necessary but is not, by itself, crash recovery, version history, locking, or conflict detection.

## 1. Observed open/save architecture

### 1.1 Dispatch and file boundaries

**Facts.** The engine declares separate extension sets for opening and saving. Opening accepts DOCX
and legacy Word names, text/Markdown/HTML/RTF/ODT/LaTeX, Word XML, and JSON; saving accepts DOCX
flavours, PDF, text/Markdown/HTML/RTF/ODT/LaTeX, PNG, JSON, and XML ([`OPEN_EXTS` and
`SAVE_EXTS`](../../crates/engine/src/io.rs:8)). There is no `fdx` entry in either list.

The public byte boundary is [`open_bytes_with()`](../../crates/engine/src/io.rs:102). It first
handles encrypted Word packages, then uses the extension unless the content signature contradicts
it, and finally calls [`open_by_content()`](../../crates/engine/src/io.rs:195). JSON is directly
deserialized into `Document`; plain text uses the encoding-aware importer; other formats are
delegated through [`io_ext::open()`](../../crates/engine/src/io_ext.rs:5). Every successful open
calls `ensure_nonempty()` ([`open_bytes_with()`](../../crates/engine/src/io.rs:113)).

This content sniffing is useful migration behaviour: ZIP, OLE, RTF, and text signatures can rescue
misnamed documents, while unsupported binary content is refused with an error rather than treated
as garbage ([`signature_contradicts()`](../../crates/engine/src/io.rs:166),
[`open_by_content()`](../../crates/engine/src/io.rs:191)). HTML has an additional path-aware
loader for relative images, with canonical-folder checks, per-file limits, a total budget, and a
small cache ([`open_path_with()`](../../crates/engine/src/io.rs:255),
[`LocalImages::load()`](../../crates/engine/src/io.rs:309)).

**Facts.** Native path open checks automation read authority, rejects files over 2 GiB, reads the
bytes, and dispatches by the path name/content ([`open_path_with()`](../../crates/engine/src/io.rs:253)).
Native save checks write authority, serializes before touching the destination, and passes the
bytes to the safe-save implementation ([`save_path_with()`](../../crates/engine/src/io.rs:423)).
The general `write_file()` helper is still a direct `std::fs::write` boundary
([`write_file()`](../../crates/engine/src/io.rs:406)); it is used for artifacts such as PNG/print
output, not the canonical document save path.

The format adapter split is clean: DOCX/legacy DOC/Flat OPC are handled in
[`io_ext::open()`](../../crates/engine/src/io_ext.rs:5), while DOCX flavours, Flat OPC, PNG, PDF,
and the other format crate exports are handled in [`io_ext::save()`](../../crates/engine/src/io_ext.rs:22).
The conversion path is therefore one model in memory, not format-to-format direct conversion.

### 1.2 Command/session state and save semantics

**Facts.** `file.open` accepts a path or base64 bytes, optional password, and only replaces the
session after parsing succeeds ([`open()`](../../crates/engine/src/cmd/file.rs:137)). It assigns the
opened path, applies the editing language to TXT/Markdown, and remembers a supplied password for
later saves when the source was encrypted ([`open()`](../../crates/engine/src/cmd/file.rs:149)).
`file.new` clears the path, while `set_document()` resets document identity, history-related state,
selection, and dirty state ([`new()`](../../crates/engine/src/cmd/file.rs:116),
[`Session::set_document()`](../../crates/engine/src/session.rs:956)).

`file.save` chooses an explicit path or the session path, otherwise asks the UI for Save As. It
stamps save metadata, serializes through the format boundary, and only marks the session clean and
updates its path for formats considered document saves (`docx`/macro/template variants, ODT, RTF,
JSON, and XML) ([`save()`](../../crates/engine/src/cmd/file.rs:184)). PDF, PNG, Markdown, HTML, TXT,
and LaTeX are consequently exports/copies: writing them does not make the editing session's
canonical path clean. This is a valuable guard against silently treating a lossy export as the
editable source.

Password support is currently Word-package-specific: the password is zeroizing in memory and
encryption is offered only for Word package extensions ([`Password`](../../crates/engine/src/io.rs:56),
[`can_encrypt()`](../../crates/engine/src/io.rs:92)). A screenplay native container should not
inherit this assumption accidentally; it needs an explicit encryption/secret-handling policy.

## 2. Durability, autosave, and recovery

### 2.1 Safe save

**Facts.** [`safe_save::write()`](../../crates/engine/src/io/safe_save.rs:8) creates an exclusively
owned sibling named `.wordcraft-save-<pid>-<counter>.tmp`, writes all bytes, preserves existing
basic permissions, calls `sync_all()`, closes the temporary file, and then renames it over the
destination ([`Temporary::create()`](../../crates/engine/src/io/safe_save.rs:41),
[`Temporary::commit()`](../../crates/engine/src/io/safe_save.rs:75)). Failed preparation removes
only that temporary file and leaves the old destination alone ([`Drop for Temporary`](../../crates/engine/src/io/safe_save.rs:84)). Concurrent prepared saves get distinct names; the last successful rename wins.

The tests inject partial and complete preparation failures, check original-byte preservation and
temporary cleanup, exercise overlapping saves, and verify private temporary permissions and mode
inheritance ([`safe_save` tests](../../crates/engine/src/io/safe_save.rs:95)). The documented contract
explicitly excludes directory synchronization, power-loss guarantees, network-filesystem
semantics, locking, and conflict detection ([`safe_save` module documentation](../../crates/engine/src/io/safe_save.rs:1)).

**Implications.** This is an appropriate MVP save primitive, but a screenplay product should not
describe it as a recovery system. A crash after the file is synced but before rename can leave an
orphan temporary; a power loss can still lose the directory entry; two app instances can overwrite
each other; and a successful save can be followed by an external edit before the next autosave.
Recommendations for a native project format should retain sibling-temp replacement, add a recovery
snapshot/journal, and decide whether a lock/mtime/hash conflict check is required.

### 2.2 Autosave

**Facts.** The UI autosave tick runs after a dirty period of roughly 2.5 seconds and a caret idle
period of 1.5 seconds, invokes ordinary `file.save`, and reports a failure while disabling autosave
for that file until a user save ([`autosave_tick()`](../../crates/ui-egui/src/lib.rs:1468)). Autosave
is allowed only when the document has a path, was explicitly saved in the current session, the
format keeps everything, and the host is not the browser ([`autosave_block()`](../../crates/ui-egui/src/lib.rs:1432)).

The tests pin two important safety policies: an opened DOCX is not rewritten behind the user's back
until the user explicitly saves it, and Markdown remains dirty because its format is lossy even if
it was saved ([`opened_files_are_not_autosaved_until_the_user_saves()`](../../crates/ui-egui/src/lib.rs:3570),
[`autosave_is_blocked_with_a_reason_for_files_it_cannot_keep()`](../../crates/ui-egui/src/lib.rs:3595)).
Macro-enabled DOCX/templates are included in the keep-everything set and browser downloads cannot
serve as autosave ([`autosave_covers_macro_enabled_documents_and_templates()`](../../crates/ui-egui/src/lib.rs:3319),
[`autosave_is_unavailable_in_the_browser()`](../../crates/ui-egui/src/lib.rs:3631)).

**Implications.** Reusing this policy for a screenplay app is safer than autosaving every imported
FDX/DOCX directly, but it is insufficient for a new unsaved project: no path means no autosave.
The native project format should be the first Save As target and should permit recovery snapshots
before the first named save. Autosave should write a recovery copy or journal, not mutate an
interoperability export whose semantics are known to be lossy.

### 2.3 Version history and preferences are not recovery

**Facts.** The `file.versions` command stores up to 50 cloned in-memory `Document` values with a
label and timestamp; `file.recover` merely returns that same in-memory list
([`file.versions` and `file.recover`](../../crates/engine/src/cmd/tools.rs:396)). They disappear when
the process exits and are not files, journals, or crash-restart recovery.

Desktop preferences are separate from documents: `ui.json` stores UI state in the settings folder,
is loaded opportunistically, and is written with a direct best-effort `std::fs::write`; malformed
or missing preferences are simply ignored ([`prefs_path()`](../../apps/wordcraft/src/main.rs:123),
[`load_prefs()`](../../apps/wordcraft/src/main.rs:153), [`save_prefs()`](../../apps/wordcraft/src/main.rs:187)).
Window geometry, UI options, fonts, logs, and the custom dictionary belong there, not in a document
file ([`user_dict_path()`](../../apps/wordcraft/src/main.rs:133)). This separation should be kept for
screenplay projects: editor UI state and proofing preferences should not contaminate portable
project content.

## 3. Interoperability options for a screenplay app

### 3.1 Owned native screenplay project format

**Observed starting point.** JSON is currently a complete `Document` serialization path: `json` is
opened with `serde_json::from_slice` and saved with `serde_json::to_vec_pretty`
([`open_bytes_with()`](../../crates/engine/src/io.rs:116), [`save_bytes_opts()`](../../crates/engine/src/io.rs:217)).
The repository also documents `.json` as “document model” and “for agents” rather than as a stable
user project format ([`file-format-parity.md` format table](../../docs/file-format-parity.md:40)).

**Recommendation.** Define an owned extension such as `.wcscreenplay` (name subject to product
choice) and make it a versioned package, not an unversioned raw struct dump. A minimal trustworthy
v1 can be a single UTF-8 JSON file with:

* a required format identifier and integer schema version;
* screenplay metadata (title, author, contacts, revision information, created/modified times);
* ordered elements with stable IDs, role (`scene_heading`, `action`, `character`, `dialogue`,
  `parenthetical`, `transition`, `note`), text/runs, and explicit scene boundaries;
* project preferences that affect screenplay layout, kept distinct from window/UI state;
* an asset manifest and embedded assets only when the app needs images or reference material; and
* optional extension data that unknown readers preserve or safely ignore.

When assets or recovery history make one file too large, use a ZIP container with a small manifest,
canonical model JSON, and an `assets/` tree. The manifest must be read before allocating, cap entry
counts/sizes and total uncompressed bytes, reject path traversal, and fail closed on unsupported
schema versions. Keep the model canonical and deterministic so snapshots are diffable and backup
tools can identify changes. Do not serialize caret positions, layout caches, or undo stacks as
required content; they are derived/transient and current document positions are structural rather
than durable IDs (see the prior model assessment's [`position findings`](01-document-model-and-editing.md:135)).

An owned format prevents screenplay roles from being inferred from DOCX styles and avoids making
FDX's or Word's feature set the semantic model. It also creates a migration contract: readers can
accept older schema versions, writers emit one current version, and an explicit migration tool can
write a backup before upgrading.

### 3.2 DOCX

**Facts.** DOCX is the repository's most complete external rich format. The parity assessment rates
DOCX read/write at roughly 65%, with synthetic round-trip and malformed-input coverage but no real-
world corpus validation ([`file-format-parity.md` DOCX row](../../docs/file-format-parity.md:20)).
The implementation preserves many Word constructs, including styles, numbering, sections, notes,
comments, revisions, content controls, drawings, opaque embedded parts, and macro projects in
macro-capable flavours ([`file-format-parity.md` DOCX details](../../docs/file-format-parity.md:49),
[`DOCX macro preservation`](../../crates/docx/src/write/mod.rs:392)).

**Recommendation.** Use DOCX for broad office interoperability and delivery, not as the native
screenplay source. It can represent screenplay paragraphs through styles, but it has no standard
screenplay semantic vocabulary; role mapping must be documented and conflicts must be surfaced.
Import should map recognized screenplay styles, retain unknown style information where practical,
and produce warnings for tables/fields/revisions/objects that do not map. Export should be
explicitly labelled “DOCX copy/export” and should not clear native-project dirty state.

DOCX migration is potentially lossy despite strong round-trip support: unsupported or partially
supported features exist, macros require a macro-capable extension, and the repository itself says
real-world corpus coverage remains a gap ([`format readiness criteria`](../../docs/file-format-parity.md:132)).

### 3.3 FDX

**Observed fact.** There is no FDX reader, writer, extension registration, CLI special case, or
FDX test in the inspected persistence paths; the engine's enumerated formats and format crate list
contain no `fdx` ([`OPEN_EXTS`](../../crates/engine/src/io.rs:8), [`formats::EXTENSIONS`](../../crates/formats/src/lib.rs:24)).

**Recommendation.** FDX is the most screenplay-specific interchange target of the candidates, so
it is a good second adapter after the native format. Treat its XML as an interchange schema, not a
native project database. Define a mapping table for scene headings, action, character, dialogue,
parentheticals, transitions, dual dialogue, notes, page breaks, scene numbers, revisions, title
page data, and inline formatting. Preserve unknown XML only if doing so can be bounded and safely
associated with a stable element; otherwise report it as dropped. Test with representative FDX
fixtures and a loss report, not merely byte equality. A missing FDX feature must not silently turn
semantic screenplay roles into generic paragraphs.

### 3.4 Plain text

**Facts.** TXT import detects UTF-8/BOM, UTF-16 (including sniffed unmarked UTF-16), and falls back
to Windows-1252; line endings normalize, and TXT export emits plain text (including list labels)
([`decode_text()`](../../crates/engine/src/io.rs:440), [`file-format-parity.md` TXT row](../../docs/file-format-parity.md:30)).
The format tests cover line endings, encodings, text sniffing, symbol-font conversion, and a
non-rich round trip ([`txt_round_trip()`](../../crates/formats/src/tests.rs:267)).

**Recommendation.** Offer TXT as the safest rescue/export format and a useful interchange for
outline or dialogue text, but never promise preservation of roles, formatting, scene IDs, notes,
revisions, or page layout. A screenplay app may optionally emit a documented plain-text convention
(for example, role labels or Fountain-like syntax), but that is a new text dialect and needs an
explicit parser/version policy rather than being confused with arbitrary TXT.

### 3.5 PDF

**Facts.** PDF is write-only in the current format matrix, produced through the PDF backend; it
supports selectable text, links, outlines, tagging, embedded subsets/outlines, and optional Print
Markup, but the parity document rates it as an output format and lists no PDF import
([`io_ext::save()`](../../crates/engine/src/io_ext.rs:30), [`file-format-parity.md` PDF rows](../../docs/file-format-parity.md:26)).

**Recommendation.** Treat PDF as the final review/printing/delivery artifact. It is intentionally
not a migration source: pagination, glyph placement, and appearance survive, but semantic roles,
editable text structure, stable IDs, notes, and project metadata do not reliably survive a PDF
round trip. Never overwrite the native project when exporting PDF and never mark the project clean
because a PDF was successfully written.

## 4. Minimal trustworthy persistence design

For a small screenplay-writing app, implement the following in order:

1. **Canonical native project.** A versioned `.wcscreenplay` model with stable element IDs and
   screenplay roles; deterministic UTF-8 serialization; explicit project metadata; no required
   UI/layout/undo state.
2. **Transactional save.** Reuse the safe sibling-temp protocol for the native file, but add
   directory synchronization where supported, preserve permissions/metadata as required, and
   surface failures without clearing dirty state. Keep the existing size/path/resource bounds.
3. **Recovery snapshot.** After edits settle, write a bounded recovery copy or append-only journal
   in the app settings/recovery directory, using the same atomic replacement discipline. Include
   project ID, base-file identity/hash, schema version, monotonic sequence, timestamp, and model
   snapshot. Rotate by count and total bytes. On startup, offer recovery only when the snapshot is
   newer than the last committed native save; never silently replace the user's file.
4. **Conflict protection.** Before autosave, compare the destination's recorded identity/mtime/hash
   with the value observed after the last successful save. If it changed externally, stop and offer
   Save Copy / Reload / Merge rather than overwriting.
5. **Explicit exporters/importers.** DOCX and FDX are rich interchange adapters; TXT is a lossless
   text-only fallback; PDF is output. Each lossy operation should return or record a warning summary.
6. **Migration discipline.** Read older native schema versions with bounded migrations, write a
   backup before in-place migration, preserve unknown extension fields where safe, and reject newer
   mandatory schema versions with an actionable error. Never use file extension alone when a robust
   container signature is available.

This design keeps the current excellent invariant that a lossy export does not become the session's
canonical clean file, while filling the current gaps: unsaved-document recovery, durable version
history, explicit screenplay semantics, and external-change safety.

## 5. Test strategy

### Native format

* Golden fixtures for blank, multi-scene, multilingual, long-dialogue, notes, metadata, stable IDs,
  and each screenplay role.
* Serialize → parse → serialize determinism tests and semantic equality tests; do not require
  byte identity after a migration unless canonicalization promises it.
* Unknown-field and older-schema migration tests; newer-required-version refusal tests.
* Asset traversal, duplicate names, size/count limits, malformed JSON/ZIP, truncated files, invalid
  UTF-8, NaN/infinite numeric values, and allocation-budget tests.
* Crash/failure injection at serialization, temporary write, sync, close, and rename boundaries;
  verify old bytes remain readable and recovery candidates are discoverable.

### Interchange

* FDX/DOCX fixture matrix with every screenplay role and a deliberate loss report.
* Native → DOCX/FDX → native semantic comparison: text, role, order, scene boundaries, metadata,
  and supported inline formatting, with expected-loss assertions for unsupported features.
* TXT tests for UTF-8/16/1252, line endings, empty documents, and the documented screenplay text
  convention if one is added.
* PDF smoke tests for page count, selectable text, output errors, and the invariant that PDF export
  leaves the native project dirty/clean state unchanged.
* Existing format hardening should remain part of the gate: the formats crate bounds nesting and
  sizes and explicitly promises never to panic on hostile input ([`formats` module contract](../../crates/formats/src/lib.rs:1)); DOCX malformed tests and round-trip fixtures
  should be extended with screenplay-shaped content.

### Session/recovery behavior

* Open failure leaves the previous document, path, password, and dirty state untouched.
* Save failure leaves dirty state set and destination bytes unchanged; successful native save clears
  dirty state only after replacement succeeds.
* Autosave never writes an imported DOCX/FDX until the user has explicitly adopted a native path or
  explicitly saved the source format; lossy exports never become autosave targets.
* Crash restart offers a newer recovery snapshot, restores it into a new untitled/native project,
  and preserves the original file. Stale, corrupt, mismatched, and deleted recovery records are
  ignored with diagnostics.
* Two-process/external-edit tests verify conflict detection and no silent last-writer-wins for the
  native project.
* CLI conversion tests cover every supported direction, unsupported `.fdx` diagnostics before
  implementation, nonzero failures, automation read/write-root confinement, and broken-pipe
  behaviour. The current CLI already routes `convert` through `file.save` and uses bounded command
  option parsing ([`convert` path](../../apps/wordcraft-cli/src/main.rs:176)).

## Conclusion

The repository's current persistence code is a strong foundation for safe writes and broad document
interchange, but it is a Word document engine rather than a screenplay project store. DOCX provides
the best existing rich interoperability; TXT and PDF have clear, useful but narrow contracts; FDX
would add screenplay-native interchange but is not yet present. The trustworthy product boundary is
therefore an owned, versioned screenplay project plus atomic saves and a separate, bounded recovery
mechanism, with every external format treated as an explicitly mapped and potentially lossy import
or export.
