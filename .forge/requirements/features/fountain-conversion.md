# Requirements Area: Fountain Conversion

> This document specifies desired user-facing behavior for a cohesive area. Describe requirements in established-state voice, even when implementation is future work. Group related capabilities into sections and keep stable IDs so each requirement and acceptance criterion remains independently traceable. Do not add lifecycle status, implementation evidence, progress, or a backlog here.

## Area purpose

Fountain is a secondary interchange format for creating a new native screenplay or producing a separate text-based copy. It does not replace native FDX as the saved source of truth.

## Capability groups

### Import and export

- **REQ-WRITING-LOOP-010: Fountain conversion.** Fountain import shall use a fixed documented conversion protocol to create a new unsaved native document; export shall produce a separate lossy copy without changing the native path or clean state. Import retains as much parseable writer data as the supported protocol permits. Potential conversion loss does not require a warning; fatal failures are reported. Fountain is the desktop's default screenplay-text interchange export; native FDX remains the canonical saved source of truth.

  - **AC-010.1 — Export lifecycle:** File → Export → Fountain Text... uses parameterless, host-only command 75 (`file.export-fountain`) at command-envelope version 2. Cancellation performs no serialization or write. On acceptance, complete serialization precedes one write to the selected destination. Success writes only that destination and does not change the native path, baseline, clean/dirty state, history, selection, or views.
  - **AC-010.2 — Destination:** The save dialog offers Fountain text with a `.fountain` filter. Its suggested name is derived only from the active native filename's final component, replacing that filename's final extension with `.fountain`; absent or unusable names use `Untitled Screenplay.fountain`.
  - **AC-010.3 — Failure safety:** Invalid source, output exceeding 64 MiB, and write failure produce bounded, value-free diagnostics. Serialization and resource-limit failures write nothing. A write-failure diagnostic states that the screenplay is unchanged and output may be incomplete; it does not claim a complete export.
  - **AC-010.4 — Serializer boundary:** Serialization reads the immutable semantic Project and its current definitions directly, without Print layout or resolved fonts. Output is UTF-8 with CRLF and lone CR normalized to LF, and exactly one trailing LF when nonempty; empty content remains empty.
  - **AC-010.5 — Body conversion:** Exact Scene Heading values retain a naturally recognized `INT`, `EXT`, `EST`, `INT./EXT`, `INT/EXT`, or `I/E` prefix and otherwise receive a leading period; Character uses `@` and only an active dual-dialogue right cue receives a final caret; Dialogue and Parenthetical remain contiguous after an eligible cue, with orphaned instances forced to Action; Transition uses `>`; Lyrics prefix each line with `~`; Note uses `[[...]]` only when safe; Summary prefixes each line with `= `; and Outline 1–3 use one to three `#` characters. Unmapped types are forced Action, and PaginateAs does not determine structure. Effective Starts New Page produces a `===` block where applicable; effectively centered Action uses `>...<`; supported manual line breaks and intentional empty dialogue lines are retained. Beat Board content is excluded.
  - **AC-010.6 — Inline conversion and loss:** Definition and direct run Bold, Italic, Underline, and All Caps are represented; All Caps applies Unicode uppercase expansion, literal backslashes/asterisks/underscores are escaped, and emphasis never spans a line break. Strikethrough, font family, size, color, unknown attributes, layout, and native IDs have no Fountain representation and are omitted without dropping visible text.
  - **AC-010.7 — Title conversion:** Typed standard keys are Title, Credit, Author/Authors, Source, Draft date, and Contact; other typed keys are sanitized as bounded custom metadata keys, falling back to Content when unusable. Type-less paragraphs are grouped by contiguous centered-versus-other alignment, with empty paragraphs as separators and Left as the type-less default. The first centered group is Title; a later centered `written by`, `by`, or `by ...` group is Credit; only the immediately following centered group is Author; a centered `based on` or `story by` group is Source; the first noncentered group after centered title content is Contact; remaining groups use Content. This heuristic is lossy export presentation only and does not infer or mutate native title metadata.
  - **AC-010.8 — Default toolbar:** The default main toolbar is `flexible-space`, `views`, `beat-board`, `flexible-space`, `title-page`, `elements`, `flexible-space`, `show-hide`, `export-pdf`, `flexible-space`, `overflow`. The built-in Export Text item and existing custom toolbar arrays are not migrated; settings schema version 1 is unchanged.
  - **AC-010.9 — Evidence boundary:** No permanent tests or test-only fixtures are introduced. Source inspection or a build does not establish native-dialog behavior, rendered output, interoperability, or product acceptance.

## Related documents

- **Product overview:** [`../product-overview.md`](../product-overview.md)
- **Technical blueprint:** [`../../blueprints/technical-context.md`](../../blueprints/technical-context.md)
