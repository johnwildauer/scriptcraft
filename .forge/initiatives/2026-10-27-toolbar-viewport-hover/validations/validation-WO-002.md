# Validation: WO-002 — Icon-Only Hover Region

## Validation metadata

- **Validation type:** `fresh-context implementation validation`
- **Validator session/agent ID:** `validation-WO-002-toolbar-viewport-hover-2026-10-27T0554Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-002-icon-only-hover-region.md`](../work-orders/WO-002-icon-only-hover-region.md:1)
- **Initiative:** [`toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:1)
- **Work-order status at validation:** `in-review`
- **Session separation exception:** none

## Comparison basis and scope

This fresh-context validation used the Forge validation process, project manifest,
initiative plan, design review, WO-001 implementation and validation evidence,
WO-002 implementation evidence, execution log, current source, durable-document
diff, and a fresh required-gate run. The comparison was limited to WO-002:
icon-plus-padding hover ownership, label exclusion and styling, click dispatch,
centered layout, preservation of semantics/viewport/color behavior, deterministic
tests, durable-document impact, and required Cargo/diff gates.

WO-003 was explicitly excluded. No final toolbar edge-budget or rightmost-group
correction was validated or inferred.

## Diagnostic sweep

The independent comparison considered these possible sources of an acceptance
failure before narrowing the result:

1. The allocated interaction rectangle might still cover the whole Item group.
2. The painted hover fill might still use the outer group rectangle.
3. The label might accidentally remain interactive or receive altered styling.
4. The icon response might no longer dispatch the existing action adapter.
5. The icon-plus-padding rectangle might be off-center or overlap the label area.
6. WO-002 changes might regress existing semantic, viewport, or toolbar color behavior.
7. The deterministic test, durable-document, or project-policy gate evidence might be incomplete.

The most likely risks were (1)/(2), because the prior implementation used the
full Item rectangle, and (4), because splitting the response could disconnect
the existing click path. Source inspection and the fresh test/gate run were used
to validate those assumptions rather than infer success from the plan alone.

## Findings

### PASS — icon-plus-padding hover region and label exclusion

- **Severity:** pass
- **Evidence:** [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:447)
  allocates the outer group but calls [`toolbar_icon_hover_rect()`](../../../../apps/scriptcraft/src/main.rs:1391)
  to derive a separate icon-control rectangle using the 24-point interaction
  size and the explicit 2-point [`TOOLBAR_ICON_HOVER_PADDING`](../../../../apps/scriptcraft/src/main.rs:28).
  The response and painter both use that rectangle at
  [`main.rs`](../../../../apps/scriptcraft/src/main.rs:452) and
  [`main.rs`](../../../../apps/scriptcraft/src/main.rs:458). The label is rendered
  separately in the group scope at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:465),
  outside the response rectangle.
- **Assessment:** The implementation aligns with the authorized icon-plus-padding
  boundary and does not retain the former full-group hover surface.
- **Required next state:** Retain the implementation; no fix work order is required.

### PASS — centered layout and label styling

- **Severity:** pass
- **Evidence:** [`toolbar_icon_hover_rect()`](../../../../apps/scriptcraft/src/main.rs:1391)
  centers the response on the group x-coordinate and places it at the top icon
  band. The deterministic test
  [`toolbar_icon_hover_region_is_centered_and_excludes_the_label_area()`](../../../../apps/scriptcraft/src/main.rs:1552)
  verifies the expected 32.0–60.0 horizontal bounds, 20.0–48.0 vertical bounds,
  group centering, and exclusion of a lower label point. The rendering path uses
  a normal [`egui::Label`](../../../../apps/scriptcraft/src/main.rs:465) for the
  text, with no hover fill or altered label color applied.
- **Assessment:** The label remains visually separate and the icon control is
  centered within the existing group geometry.
- **Required next state:** No layout or styling remediation is required.

### PASS — click dispatch and preserved tooltip/accessibility label

- **Severity:** pass
- **Evidence:** The icon response is allocated with [`Sense::click()`](../../../../apps/scriptcraft/src/main.rs:453),
  retains [`on_hover_text()`](../../../../apps/scriptcraft/src/main.rs:454) using
  `item.label`, and dispatches the existing action through
  [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:421) when
  [`response.clicked()`](../../../../apps/scriptcraft/src/main.rs:469) is true.
  The full workspace test suite also passed the existing toolbar metadata,
  dispatch, overflow, and role-behavior tests.
- **Assessment:** Splitting the hit region did not remove the existing command
  adapter path or the label used for tooltip/accessibility surfaces.
- **Required next state:** No dispatch remediation is required.

### PASS — preserved semantics, viewport behavior, and color source

- **Severity:** pass
- **Evidence:** The WO-002 diff changes only the action-item response/fill
  boundary and the authorized requirements/blueprint wording. Existing
  [`default_toolbar_items()`](../../../../apps/scriptcraft/src/main.rs:147),
  role/overflow paths, and WO-001's validated
  [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1479) remain in
  place. Fill selection still goes through
  [`toolbar_group_fill()`](../../../../apps/scriptcraft/src/main.rs:1405), whose
  hovered source is `style.visuals.widgets.hovered.bg_fill` and whose idle state
  is transparent. The deterministic test
  [`toolbar_icon_hover_fill_uses_the_existing_hover_color_source_only()`](../../../../apps/scriptcraft/src/main.rs:1565)
  verifies both states and the 2-point padding.
- **Assessment:** No semantic command, viewport, native-color, global-theme, or
  persistence behavior was changed within WO-002. WO-001's native visual smoke
  limitation remains applicable; no native visual success is claimed here.
- **Required next state:** Preserve the documented limitation and retain the implementation.

### PASS — deterministic coverage and durable-document impact

- **Severity:** pass
- **Evidence:** The fresh test run passed the app's 57 unit tests and the
  workspace's 44 core unit tests plus all integration targets, including the two
  new hover-region/fill tests and the existing toolbar geometry/dispatch tests.
  The requirements update in
  [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104)
  records the icon-plus-padding response and label exclusion. The blueprint
  update in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66)
  records the separate response, normal label color, tooltip/accessibility
  availability, and lightness-only hover behavior. Those updates match the
  work order's declared `update` impact classifications.
- **Assessment:** The durable-document impact is accurate and the test evidence
  is deterministic. No unauthorized production or durable-document scope was
  found; WO-003 remains untouched.
- **Required next state:** No documentation correction is required.

### PASS — required Cargo and diff gates

- **Severity:** pass
- **Evidence:** A fresh chained run completed with exit code 0 for
  [`cargo fmt --all -- --check`](../../../../Cargo.toml:1),
  [`cargo check --workspace`](../../../../Cargo.toml:1),
  [`cargo test --workspace`](../../../../Cargo.toml:1),
  [`cargo build --workspace`](../../../../Cargo.toml:1),
  [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1),
  and [`git diff --check`](../../../../.gitignore:1). The current comparable
  diff contains the authorized app implementation and requirements/blueprint
  updates, plus initiative evidence files; no WO-003 implementation file or
  unrelated production change was introduced. No clearly deceptive, backdoor,
  malicious, nefarious, or otherwise rogue behavior was found.
- **Assessment:** All required project-policy gates available in this checkout
  passed. The no-native-window limitation is non-blocking because the manifest
  defines no required manual/external gate.
- **Required next state:** No gate remediation is required.

## Result

- **Outcome:** `pass with warning`
- **Hard-stop findings:** none
- **Warnings/notes:** Native visual smoke was unavailable, so this validation
  confirms source/configuration behavior and deterministic tests only; it does not
  claim native visual hover success or native titlebar success.
- **WO-002 disposition:** Remains `in-review`; this validation does not mark the
  work order `complete`.
- **Scope boundary:** WO-003 was not validated, implemented, or assessed.

## Required next state

The orchestrator should record this fresh validation in the initiative lifecycle
and apply the normal validation transition. WO-002 may advance only through the
normal orchestrator status transition after this `pass with warning`; WO-003
requires its own separately scoped implementation and fresh validation. Preserve
the native visual-smoke limitation and do not convert it into a native visual
success claim.
