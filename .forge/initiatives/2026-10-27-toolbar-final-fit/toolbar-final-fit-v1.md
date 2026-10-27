# Initiative: Toolbar Final Fit and Chrome

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode; the user requested consecutive work orders in the toolbar initiative family.
- **Rapid-mode eligibility:** `not eligible` — the work covers geometry diagnosis, vertical chrome spacing, and visual chrome color with separate validation boundaries.
- **Parent initiative:** [`../2026-10-27-toolbar-interaction-polish/toolbar-interaction-polish-v1.md`](../2026-10-27-toolbar-interaction-polish/toolbar-interaction-polish-v1.md:1) — closed predecessor; this is a linked follow-up because Forge closed initiatives are terminal.
- **Base/default branch:** `dev/2026-10-26-editor-geometry-corrections-v2`
- **Initiative branch:** `dev/2026-10-27-toolbar-final-fit`
- **Branch policy:** `required`
- **Branch capability/evidence:** `git switch -c dev/2026-10-27-toolbar-final-fit` succeeded on 2026-10-27.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan → review` on 2026-10-27 after investigation and the user's explicit request for consecutive WOs. `review → execute` on 2026-10-27 after fresh design review PASS WITH WARNINGS and the user's explicit request served as human approval. `execute/validate → closeout` on 2026-10-27 after WO-001 passed fresh validation and WO-002 passed through the WO-002a fix chain with fresh validation PASS.
- **Direct-translation confirmation:** `requested and confirmed`.

## Outcome

Make the Export PDF Item fully visible by correcting the toolbar's actual-width calculation, expose independent top/bottom padding constants with 2-point top padding and reduced bottom padding, and make the toolbar fill match the title-bar/app chrome color.

## Scope

### In scope

- Diagnose and correct why the rightmost Export PDF Item is clipped despite apparent available width.
- Add named top and bottom toolbar padding constants; set top padding to 2 points and reduce bottom padding from the current effective value.
- Apply one shared geometry budget to allocation and rendering.
- Match the toolbar background to the existing title-bar/app chrome color source without changing global theme semantics.
- Add wide/maximized, exact-fit, narrow, vertical-padding, and color regression coverage.

### Out of scope

- New commands, Item grouping changes, icon redesign, customization, persistence, core semantics, page layout, exports, or unrelated theme redesign.

## Investigation findings

- [`allocate_toolbar()`](../../apps/scriptcraft/src/main.rs:177) receives `ui.available_width()` and computes Item widths using [`toolbar_content_width()`](../../apps/scriptcraft/src/main.rs:164).
- The default Item width is currently [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../apps/scriptcraft/src/main.rs:21) = `48.0`; row padding and gap are [`TOOLBAR_ROW_PADDING`](../../apps/scriptcraft/src/main.rs:23) = `4.0` and [`TOOLBAR_ITEM_GAP`](../../apps/scriptcraft/src/main.rs:24) = `4.0`.
- [`toolbar()`](../../apps/scriptcraft/src/main.rs:519) adds row padding/gaps manually, but the actual `TopBottomPanel` content width and nested child minimum widths must be compared against the allocator's logical width. The Export PDF clipping indicates an unmodeled framework minimum/overflow reservation or a mismatch between the measured Item and actual child extent.
- There is no explicit toolbar fill in [`update()`](../../apps/scriptcraft/src/main.rs:1347); `TopBottomPanel::top("toolbar")` uses egui's panel visuals. The title bar is native/window chrome rather than a separately defined application widget, so the implementation must identify the available app chrome color source and document any platform limitation.

## Work-order plan

1. [`WO-001-toolbar-fit-and-vertical-padding.md`](work-orders/WO-001-toolbar-fit-and-vertical-padding.md) — correct Export PDF fit and add top/bottom padding constants and tests.
2. [`WO-002-toolbar-chrome-color.md`](work-orders/WO-002-toolbar-chrome-color.md) — match toolbar fill to the identified title-bar/app chrome color and validate it without changing global theme behavior.

Fix chain: [`WO-002a-toolbar-chrome-color-gate-fix.md`](work-orders/WO-002a-toolbar-chrome-color-gate-fix.md) addresses the failed WO-002 validation's undefined helper/application path only.

The WOs are consecutive: color follows the geometry/padding change.

## Requirements and blueprints

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md:90), REQ-WRITING-LOOP-005 / AC-WRITING-005.5.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:68), transient toolbar allocation and desktop presentation ownership.

## Proposed design

Use one measured toolbar geometry contract for allocation and actual rendering, including real panel content width, child minimums, overflow reservation, horizontal gap, and separate top/bottom padding. Name the vertical controls beside the existing toolbar constants. Resolve the app's title-bar color source, then apply the same fill to the toolbar panel/frame while keeping it transient and presentation-only.

## Design readiness checklist

- [x] User-visible outcome, scope, investigation, and WO dependency order are recorded.
- [x] Requirements and blueprint references are linked.
- [x] Parent closed initiative and follow-up relationship are recorded.
- [x] Validation cadence and branch evidence are recorded.
- [ ] Fresh design review and human execution approval are pending.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** Both consecutive work orders, executed sequentially with fresh validation after each.
- **Approval evidence:** User explicitly requested the three toolbar changes as consecutive WOs; fresh design review returned PASS WITH WARNINGS with no blockers.

## Closeout follow-ups

- Native visual Export PDF/right-edge/padding/color inspection remains unavailable; repeat on a window-capable environment. Owner: Forge orchestrator; trigger: next native desktop validation session.
- Exact pixel equality to native OS title-bar chrome is not claimed because the current eframe API exposes no native title-bar color sampler.

## Closeout record

- **Closeout status:** `in-progress`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** WO-001 passed fresh validation; WO-002 initially failed because its helper/application path was incomplete; WO-002a repaired the path and fresh validation passed all required gates. No unresolved implementation blocker remains. Git evidence will be recorded after commit/merge.
