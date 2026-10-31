# Work Order: WO-005 — Extended role editing and layout

## Outcome
Expose the directed roles in editing and rendering without misprinting hidden content.

## Scope
Extend role selection, layout measures and explicit Enter/Tab policies for the new types. Honor imported visibility and page-start settings where supported; retain unknown settings rather than assume fixture defaults apply universally.

## Relevant references
- [Role and visibility requirements](../../../requirements/features/screenplay-writing-loop.md#fdx-document-content-and-preservation)
- [Layout](../../../../crates/scriptcraft-core/src/layout.rs) and [toolbar](../../../../apps/scriptcraft/src/toolbar.rs)

## Implementation
Use bounded documented WO-001 defaults where behavior is unspecified, retain existing transition rules and ensure unknown types are not coerced or printed as ordinary Action. Keep title page preserve-only; do not build an outline or beat-board editor.

## Acceptance criteria
- Every directed role can be selected and edited; role distinctions persist in model operations.
- Hidden Note/Outline/Summary data is not included in print output when hidden, and page/source mappings remain valid; visible and hidden settings are not confused with universal defaults.
- Existing layout, toolbar and UTF-8 editing behavior regressions are addressed.

## Useful checks
- Headless transition/layout/PDF tests for all roles and hidden settings; desktop role-menu and selection tests, including narrow toolbar; workspace checks.

## Dependency and handoff
Depends on WO-002 and WO-004. One dedicated commit and fresh validation; compatible with transitional JSON scaffold until cutover.
