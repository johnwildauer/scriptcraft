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

For the approved [inline-style round-trip initiative](../.plan/2026-11-23-inline-style-roundtrip/investigation.md), its work orders narrow the standing command allowlist: before WO-006, no build/check/lint/format command, test, launch, GUI/native/manual validation, screenshot, filesystem experiment, or other runtime command is authorized. WO-006 alone authorizes its specifically stated full workspace build and repair reruns. This initiative-specific restriction does not authorize review, validation, or acceptance activity.

For the approved [menu and context actions initiative](../.plan/2026-11-23-menu-and-context-actions/investigation.md), its work orders narrow this general command allowlist: no build/check/lint/format, test, launch, GUI/native/manual validation, screenshot, or other runtime command is authorized before WO-009. WO-009 authorizes only its specifically stated full workspace build. This initiative-specific restriction does not authorize any agent-run review, validation, or acceptance activity.

The menu/context and formatting work is integrated in source: the typed command catalog contains 63 stable IDs at command-envelope version 2 (two additive formatting commands), with no internal package edge and no permanent test/fixture addition. WO-004 enabled the existing `objc2-app-kit` dependency's `NSBezierPath` and `NSColor` features for optional native swatch drawing. The subsequent swatch repair also enables its `block2` feature and adds locked `block2` 0.6.2 as a direct macOS dependency for the AppKit drawing callback. Detailed behavior and the native-menu initial-check limitation are specified in the linked initiative requirements and blueprints; this summary does not claim runtime validation or acceptance.

For the approved [Very Tight Leading and Run Colors initiative](../.plan/2026-11-23-very-tight-leading-and-run-colors/investigation.md), WO-001 through WO-004 report source integration complete; the later authorized native swatch repair uses AppKit's block-based image factory and direct macOS `block2` 0.6.2 dependency. The first authorized `cargo build --workspace` failed on block callback typing; after repair, the identical command succeeded. The subsequent bounded macOS 27 image-visibility repair preserves the existing native menu item/muda implementation and requests Visible for custom symbol/swatch images only when the setter selector is present; the `cargo build --workspace` run for that repair also succeeded. No tests, launches, screenshots, GUI/native/manual validation, or other runtime commands were run, and no native visual result is claimed. Completion details and Apple/SDK evidence are in [the image-visibility repair record](../.plan/2026-11-23-very-tight-leading-and-run-colors/macos-image-visibility-repair-completion.md). WO-006's build authorization is complete; no agent-run review, validation, or acceptance is implied. The initiative retains 63 stable command IDs at command-envelope version 2, adds no internal package edge, and preserves the zero-permanent-test and zero-test-only-fixture policy. Earlier initiative restrictions above remain historical and unchanged.

## Unresolved product decision

The permanent supported-platform set and future distribution policy—including whether to provide installer/updater flows or signed/notarized releases—remain undecided. Current packaging choices are scoped to their documented artifact and do not establish permanent platform support or future distribution commitments.

## Project documentation map

- Start with the [product overview](requirements/product-overview.md) for product intent, scope, and user-facing constraints.
- Consult the [technical context](blueprints/technical-context.md) for the technical map and system boundaries.
- Detailed contracts are distributed across [requirements](requirements/) and [blueprints](blueprints/); the [Forge index](_index.md) is the navigation entry point.
- The element/template initiative's [final user validation matrix](../.plan/2026-11-18-element-template/final-user-validation-matrix.md) is a user-owned worksheet target, not a record of checks or manual journeys having run.
