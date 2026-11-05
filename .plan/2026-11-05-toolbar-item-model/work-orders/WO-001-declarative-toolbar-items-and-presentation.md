# WO-001 — Declarative toolbar items and presentation

- **Status:** Implementation submitted; user review and validation pending; not accepted
- **Initiative:** [`toolbar-item-model-v1.md`](../toolbar-item-model-v1.md)
- **Depends on:** None
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Replace the one-icon-string item representation with a declarative desktop catalog and a generic graphic renderer. Establish shared toolbar presentation defaults and optional per-item/per-icon overrides while retaining the current eight-entry composition, command routing, Elements behavior, allocation rules, and overflow text menu as the intermediate baseline.

Migrate the five currently rendered action items to their assigned outlined Material icons so the new path uses typed icons rather than preserving the legacy Unicode glyphs.

## Scope

1. Create [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs) and register it from [`main.rs`](../../../apps/scriptcraft/src/main.rs:30). Keep screenplay semantics and all application operations outside this module.
2. Define a [`ToolbarItemSpec`](../../../apps/scriptcraft/src/toolbar_items.rs) with:
   - stable string identity;
   - visible label and accessibility name, with explicit caption visibility;
   - graphic specification;
   - behavior specification separate from graphic and enablement;
   - enabled/disabled state policy;
   - measured, fixed-width, fixed-space, or flexible-space layout policy.
3. Define graphic support for:
   - no graphic for spacing;
   - one [`MaterialIcon`](../../../apps/scriptcraft/src/status_bar.rs:643);
   - multiple independently specified icons;
   - one primary icon plus a separately specified disclosure icon;
   - a text dropdown whose selected value is supplied by behavior rather than stored as static graphic text.
4. Give every icon specification optional point-size, enabled-color, and disabled-color overrides. The Show/Hide primary and disclosure icons must therefore be independently adjustable when WO-002 instantiates them.
5. Move the ordered item definitions into the catalog. For this order only, preserve the existing New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, Export PDF composition. Define the overflow entry as a reusable specification but retain the existing literal double-arrow overflow presentation until WO-002.
6. Add one toolbar-local presentation surface in [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs) with named, commented adjustment points for:
   - enabled foreground override, defaulting to the active Appearance text color;
   - disabled foreground override, defaulting to the resolved enabled foreground with reduced alpha;
   - disabled-opacity multiplier used only when no explicit disabled color override applies;
   - default Material icon point size;
   - default spacing between grouped/disclosure graphics.
   Initialize the icon size to a reasonable toolbar-local equivalent of the current effective glyph size, label it as an agent-default, and record the exact value in the handoff. Do not reuse or modify status-bar presentation values.
7. Resolve presentation in this order: item/icon override, toolbar default override, theme-derived default. Apply disabled presentation to every graphic and visible caption of a disabled item. All sizes remain logical points.
8. Generalize [`toolbar_item_width()`](../../../apps/scriptcraft/src/toolbar.rs:152) and its measurement inputs so intrinsic width accounts for the complete graphic arrangement as well as the visible caption. Preserve the current 46-point normal minimum, 128-point Elements override, 72-point Space, four-point row padding, four-point item gap, and flexible-space/overflow budget in this order.
9. Make [`toolbar_action_button()`](../../../apps/scriptcraft/src/toolbar.rs:341) consume resolved graphic data rather than reading one legacy icon string. Keep the existing enabled-action icon-only hover/click rectangle, two-point radius, default cursor, decorative caption, and command dispatch.
10. Keep [`elements_toolbar_item()`](../../../apps/scriptcraft/src/toolbar.rs:406) on the existing selected-role and transactional role-change path. Adapt only the data handoff needed by the catalog; do not change popup height here.
11. Route all visible entries from the catalog through the common allocator and renderer. Do not add a second hard-coded composition in [`toolbar()`](../../../apps/scriptcraft/src/toolbar.rs:521).

## Assigned icons for the current actions

| Item | Outlined Material assignment |
|---|---|
| New | Draft |
| Open | File Open |
| Save | Save |
| Save As | Save As |
| Export PDF | Picture As Pdf |

Use the typed constants supplied by [`egui_material_icons`](../../../apps/scriptcraft/Cargo.toml:10) and the same rich-text/font-family mechanism already used by [`status_icon_control()`](../../../apps/scriptcraft/src/status_bar.rs:643). Do not use ordinary-font lookalikes, images, or hand-drawn glyphs.

## Non-goals

- No leading/trailing Flexible Space additions and no Show/Hide or Export Text rendering yet; WO-002 owns the complete composition.
- No Material overflow replacement yet.
- No Elements popup-height correction; WO-003 owns it.
- No new commands, disabled-item behavior, plain-text export route, or Show/Hide menu.
- No toolbar customization, persistence, status-bar, core, FDX, pagination, editor, or global theme changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. The catalog is the single source of item identity, label, graphic, behavior, enablement, and width policy; the renderer contains no parallel list of default items.
2. Graphics and behavior are independent. A two-icon or icon-dropdown specification does not imply a command or menu.
3. Every individual icon can resolve its own size and state colors, while unspecified values follow the toolbar-wide defaults.
4. Existing enabled actions retain their command behavior, captions, accessible naming, hover geometry, width fitting, and ordering.
5. Elements retains current-role display and transactional role mutation with no popup-height change claimed.
6. No toolbar presentation setting changes status-bar controls or global Appearance styling.

## Durable records

- Update [`AC-WRITING-005.3`](../../../.forge/requirements/features/screenplay-writing-loop.md:530) and the closely related measurement wording in [`AC-WRITING-005.5`](../../../.forge/requirements/features/screenplay-writing-loop.md:539) for declarative graphic/caption presentation and complete graphic-aware measurement. Preserve the old eight-entry order until WO-002.
- Add the catalog/presentation ownership contract to [`technical-context.md`](../../../.forge/blueprints/technical-context.md:20), including logical-point units and desktop-only state.
- Do not mark the complete temporary composition implemented in [`icon_assignement.md`](../../../.reference/icon_assignement.md:12) during this intermediate order.
- Append exact source changes, the agent-default initial icon size, and evidence limitations to this work order and [`toolbar-item-model-v1.md`](../toolbar-item-model-v1.md).

## User-owned review requests

- Inspect the catalog and confirm adding or changing an item requires no parallel renderer metadata edit.
- Review each current action icon, caption, hover rectangle, pointer and keyboard activation, and command result.
- Change the global enabled color, disabled color, disabled opacity, icon size, and graphic gap individually; confirm unspecified item/icon values follow those defaults and explicit overrides win.
- Exercise wide, exact-fit, and narrow widths; verify current overflow behavior and right-edge fitting remain bounded.
- Confirm Elements behavior is unchanged apart from its declarative data handoff.
- Run any desired workspace compilation and behavior checks and record outcomes separately; no agent result is claimed.

## Implementation handoff — 2026-11-05

WO-001 source and its authorized durable-context updates are submitted for user
review; this handoff does not mark the work order complete, accepted, or
validated. WO-002 and WO-003 have not started.

- **Branch evidence:** Before edits, `git status --short --branch` observed
  `master` with pre-existing modifications in
  `.reference/icon_assignement.md` and `apps/scriptcraft/src/toolbar.rs`, plus
  the untracked initiative plan/work-order directory. No initiative branch
  existed. `dev/2026-11-05-toolbar-item-model` was created from that observed
  `master` checkout and switched to; a subsequent status showed the same
  pre-existing paths still present on the initiative branch. No work was
  staged, committed, merged, or discarded. The existing reference-file edits
  were not changed by WO-001. The existing toolbar edits setting top padding to
  4 points and icon hover padding to 1 point were retained.
- **Source changes:** Added `apps/scriptcraft/src/toolbar_items.rs` and registered
  it in `apps/scriptcraft/src/main.rs`. The catalog now owns item identity,
  label/accessibility name, explicit caption visibility, graphic, behavior,
  enablement, and width policy. It defines no/single/multiple/primary-plus-
  disclosure/text-dropdown graphics, per-icon point-size and foreground
  overrides, item color overrides, a reusable 72-point Space spec, the reusable
  32-point overflow spec, and only the approved eight-item intermediate
  composition. New, Open, Save, Save As, and Export PDF use the assigned typed
  outlined Material icons. The generic graphic renderer resolves each icon and
  caption foreground independently; toolbar measurement includes grouped icon
  widths and gaps as well as visible captions. The existing allocator, action
  routing, icon-only hover/click rectangle, two-point radius, default cursor,
  decorative captions, Elements ComboBox/role transaction, and literal `>>`
  overflow presentation remain in the existing toolbar adapter.
- **Initial WO-001 toolbar-local defaults:** `DEFAULT_MATERIAL_ICON_POINT_SIZE`
  was **14.0 logical points**, `DEFAULT_GRAPHIC_GAP` was **4.0 logical points**,
  and `DISABLED_OPACITY_MULTIPLIER` was **0.45**. The original implementation
  report recorded these as agent defaults; the later current icon-size value is
  recorded in the presentation follow-up below. Enabled and disabled explicit
  toolbar color overrides default to `None`; enabled color follows active
  Appearance text, and disabled color falls back to that resolved foreground
  with reduced alpha. Item/icon overrides take precedence. These values are
  toolbar-local and do not change status-bar presentation.
- **Durable-context changes:** Updated only AC-WRITING-005.3 and AC-WRITING-005.5
  in `screenplay-writing-loop.md` for declarative graphic/caption foreground
  presentation and graphic-aware measurement. Added the desktop-only catalog,
  precedence, logical-point, transient-state, and current-composition contract
  to `technical-context.md`. The icon assignment reference was not modified by
  this order.
- **Evidence limits:** No build, check, test, lint, formatting command, manual
  verification, review, or validator was run. No test target or coverage was
  added. Compilation, icon appearance, action/keyboard behavior, layout fitting,
  Appearance/color resolution, and Elements equivalence remain unverified and
  user-owned for review and validation.

## Toolbar presentation follow-up — 2026-11-05

Applied the user's follow-up direction: omitted icon variants resolve through
`DEFAULT_MATERIAL_ICON_VARIANT` to Outlined, while an explicit per-icon variant
override can still select Filled. Added `DEFAULT_ITEM_TEXT_POINT_SIZE` at 14.0
logical points and applied it to toolbar Body/Button text styles, custom captions,
and toolbar text measurement. This makes toolbar item text sizing directly
adjustable without changing the status bar or document typography. Durable
technical context was updated accordingly. The current source retains
`DEFAULT_MATERIAL_ICON_POINT_SIZE` at 18.0 logical points; this follow-up did not
change that value. No validation command, manual
verification, review, or validator was run; rendering and sizing remain
user-unverified.
