# ScriptCraft — Project Manifest

This parent document provides durable project orientation. Product behavior and technical contracts belong in the linked requirements and blueprints, not in a feature or implementation inventory here.

## Intent and audience

ScriptCraft is a focused, desktop-first screenplay editor for individual writers. Its purpose is to support a dependable writing loop—drafting and revising screenplay content, reviewing page flow, saving and reopening a native project, and producing clearly bounded review exports—without sacrificing the meaning or safety of the writer’s work.

## Product and engineering constraints

- Keep the product focused on screenplay writing for one writer; prioritize preservation and safe handling of the last valid native project over feature breadth.
- Use Rust. Keep semantic screenplay content and retained native-project data authoritative, separate from transient presentation and editing state.
- Favor repeatable page layout for the same effective inputs, without promising exact rendering compatibility for every imported document.
- Keep formatting, import interpretation, and exports bounded and screenplay-specific. Preserve unsupported source data where required rather than implying support for editing it or blanket format compatibility.
- Native FDX projects are the durable project format. Native saves preserve the valid prior state on failure; PDF and plain-text outputs are noncanonical and have explicit limitations.

## Working preferences and repository policy

Work in a guided, concise, safety-conscious way. Explain consequential defaults, avoid unnecessary scope and checkpoints, and prefer sequential work. Preserve pre-existing user work; do not perform destructive filesystem or Git operations without explicit authorization.

The user owns all review and acceptance. Agents must not conduct or dispatch design, requirements, blueprint, implementation, or other reviews; dispatch validators; or run tests, manual verification, GUI/native/visual validation, launches, or other commands unless separately authorized. The routine agent-run automated Cargo checks allowlisted below are the sole standing command exception; they are not user review, product acceptance, or evidence of rendered/native behavior. Do not claim review or acceptance without user-supplied evidence and an explicit user decision.

The following exact workspace commands are permitted as simple automated checks, including their ordinary compiler/lint output: `cargo build --workspace`, `cargo check --workspace`, `cargo clippy --workspace`, and `cargo fmt --all -- --check`. This allowlist does not permit tests, command variants/extra flags, fixes or writes beyond the named commands' normal behavior, launches, GUI/native/manual/visual checks, or any other command. Work orders may impose a narrower command boundary; their restrictions remain controlling unless the user separately authorizes an exception.

This is workflow policy, not permission to weaken or bypass in-product safety checks or safeguards. Applicable external rules and obligations remain in force. If a genuine external requirement conflicts with this policy, surface it as a blocker; do not treat it as waived or substitute an agent result.



## Unresolved product decision

The permanent supported-platform set and future distribution policy—including whether to provide installer/updater flows or signed/notarized releases—remain undecided. Current packaging choices are scoped to their documented artifact and do not establish permanent platform support or future distribution commitments.

## Project documentation map

- Start with the [product overview](requirements/product-overview.md) for product intent, scope, and user-facing constraints.
- Consult the [technical context](blueprints/technical-context.md) for the technical map and system boundaries.
- Detailed contracts are distributed across [requirements](requirements/) and [blueprints](blueprints/); the [Forge index](_index.md) is the navigation entry point.
- The element/template initiative's [final user validation matrix](../.plan/2026-11-18-element-template/final-user-validation-matrix.md) is a user-owned worksheet target, not a record of checks or manual journeys having run.

## ScriptNotes source-integration map

The source integration recorded in the [ScriptNotes blueprint](blueprints/features/script-notes.md) applies generated ScriptNote run defaults before canonical normalization so candidate finalization is idempotent across Session and writer normalization, while retaining strict semantic, identity, source-tree, destination, and publication verification; recognizes and emits the literal default-color sentinel for new default notes; and implements Color property updates as Preserve/ThemeDefault/Explicit without changing command IDs 76–83 or envelope version 2. The Color menu places None first, then Black and the twelve existing palette colors; ThemeDefault is command-wire-only, not a new palette entry or screenplay formatting-menu choice. Marker groups use paragraph-first baselines and reveal-safe navigation. Card placement prefers the marker-right side when the full card fits monitor/work bounds, caps its left origin at the root right edge without requiring full root containment, and retains left/more-room fallbacks and monitor-based vertical clamping. The fixed-card Body width, persisted paragraph indents, measured placeholders, and display-only title truncation retain their documented behavior. Separate below-card Color/Type popups use frameless rows; the note card and property-menu native viewports request transparent backgrounds and disabled native shadows, with transparent clear color scoped to the open note family and app-painted rounded surfaces. These are source-configuration facts only: they do not establish compilation, persistence/save-reopen behavior, host/compositor transparency, rendered/native behavior, interoperability, or product acceptance.

V3 further records the complete native-surface path: root startup enables transparent-backbuffer capability while root and other surfaces remain ordinarily opaque; note-family clear color is transparent only while open; transparent, undecorated, shadowless child viewports have no opaque full-viewport panel; and app-painted rounded frames define their visible edge. Property-menu sizing is reconciled to the actual popup response outer rectangle at ordinary frame boundaries, without repeated no-op resizes or recentering, while preserving top-left/below-card placement and bounded monitor-height scrolling. Popup shadow is disabled while rounded fill/stroke and frameless rows remain. Windows transition suppression is best-effort per exact ScriptNote viewport title and current-process-verified HWND, bounded once per native-window lifetime and reset on recreation; failures leave ordinary presentation unchanged. The existing pinned Windows dependency adds only Win32_Graphics_Dwm. Root-composited notes remain rejected because off-root rendering is required. These source facts do not guarantee host alpha/compositor support, first-frame suppression, actual native behavior, save/reopen behavior, or acceptance.
