# Initiative: Toolbar Exact Width and Hover Geometry

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Rapid-mode eligibility:** `not eligible` — the change spans runtime font measurement/allocation and layered hover geometry.
- **Parent initiative:** [`../2026-10-27-toolbar-viewport-hover/toolbar-viewport-hover-v1.md`](../2026-10-27-toolbar-viewport-hover/toolbar-viewport-hover-v1.md:1) — closed predecessor; this linked follow-up preserves Forge's terminal closed status.
- **Base/default branch:** `dev/2026-10-26-editor-geometry-corrections-v2`
- **Initiative branch:** `dev/2026-10-27-toolbar-exact-width-hover`
- **Branch policy:** `required`
- **Branch capability/evidence:** `git switch -c dev/2026-10-27-toolbar-exact-width-hover` succeeded on 2026-10-27.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan → review` on 2026-10-27 after the user's explicit request to create and implement two WOs. `review → execute` on 2026-10-27 after fresh design review PASS and the user's explicit request served as human approval. `execute/validate → closeout` on 2026-10-27 after WO-001 passed through WO-001a fix validation and WO-002 passed fresh validation.
- **Direct-translation confirmation:** `requested and confirmed`.

## Outcome

Replace guessed label-width allocation with exact runtime text measurement and make the icon hover region uniform around the icon, persistent while the pointer moves over the painted icon, with no redundant label tooltip.

## Investigation findings

- [`toolbar_item_width()`](../../apps/scriptcraft/src/main.rs:171) estimates labels using a fixed character width; exact allocation must use egui's actual text galley measurement for the active style/font.
- [`toolbar_action_button()`](../../apps/scriptcraft/src/main.rs:446) registers an explicit icon response, then renders labels that can register competing hover responses. The fix should make decorative child widgets non-interactive and retain one explicit icon-region owner.
- [`toolbar_icon_hover_rect()`](../../apps/scriptcraft/src/main.rs:1412) currently produces a top-anchored `interact_size` square; the requested uniform area must be centered around the icon control with equal expansion on all sides.
- The label tooltip is currently attached via [`on_hover_text()`](../../apps/scriptcraft/src/main.rs:468); the user requested removing it because the label is already visible.

## Work-order plan

1. [`WO-001-exact-runtime-label-width.md`](work-orders/WO-001-exact-runtime-label-width.md) — measure actual egui label text and allocate exact Item widths.
2. [`WO-002-uniform-persistent-icon-hover.md`](work-orders/WO-002-uniform-persistent-icon-hover.md) — center/uniformly expand the icon hover region, prevent child hover competition, and remove redundant tooltip.

Fix chain: [`WO-001a-reconcile-local-toolbar-geometry.md`](work-orders/WO-001a-reconcile-local-toolbar-geometry.md) reconciles focused tests with pre-existing user-local geometry values without changing production behavior.

## Requirements and blueprints

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md:90), REQ-WRITING-LOOP-005 / AC-WRITING-005.3 and AC-WRITING-005.5.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:58), transient Item measurement and interaction ownership.

## Scope

### In scope

- Runtime exact text measurement using the active egui style/font, with deterministic test seams.
- Exact width propagation through allocation/rendering and right-edge tests.
- Uniform icon-region hover bounds, one hover owner, label excluded from hover, pointer movement across the icon preserving hover, and no label tooltip.

### Out of scope

- New commands, viewport flags, native titlebar behavior, core semantics, persistence, exports, page layout, global theme redesign, or unrelated toolbar ordering.

## Design readiness checklist

- [x] Investigation, outcome, scope, and two-WO dependency order recorded.
- [x] Parent initiative, requirements, and blueprint links recorded.
- [x] Validation cadence and branch evidence recorded.
- [ ] Fresh design review and human execution approval pending.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** Both consecutive work orders, executed sequentially with fresh validation after each.
- **Approval evidence:** User explicitly requested creation and implementation of the two fixes; fresh design review returned PASS with no blockers.

## Closeout follow-ups

- Native visual exact-width/hover inspection remains unavailable; repeat on a window-capable environment. Owner: Forge orchestrator; trigger: next native desktop validation session.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** WO-001 passed through the WO-001a test reconciliation fix chain; WO-002 passed fresh validation; all required gates passed. Implementation/closeout commit `05e6b51`; merge evidence is recorded in the final execution log.
