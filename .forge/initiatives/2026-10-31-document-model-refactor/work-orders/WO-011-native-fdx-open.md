# Work Order: WO-011 — Native FDX open and import

## Outcome
Integrate staged FDX opening and non-6 warning into the desktop without unsafe early writes.

## Scope
Add FDX file selection, secure content-based version detection, consent dialog and candidate session construction. Preserve the current active session on cancellation, parse failure or incompatibility; enforce dirty-session protection.

## Relevant references
- [Version decisions](../document-model-refactor-v1.md#final-user-responses-and-remaining-implementation-risks)
- [Desktop open flow](../../../../apps/scriptcraft/src/main.rs)

## Implementation
Warn before non-6 import attempt that updating may contain errors, data may be lost and compatibility is not guaranteed; offer Continue/Cancel. On failure report incompatible and close only the attempted document. Keep the original path eligible for explicit Save and establish a file conflict baseline. Gate FDX open behind an internal staging switch until WO-012 cutover; default public JSON behavior remains buildable and usable.

## Acceptance criteria
- Version 6 and non-6 candidates are activated only after complete success; imported cast text remains unchanged during open.
- Opening or cancelling never writes the input, and failure leaves active state/path/dirty flag unchanged; no mandatory Save As or backup is introduced.

## Useful checks
- Headless adapter/dialog tests for Continue/Cancel, older and newer input, incompatible failures, dirty session and no disk writes; manual native dialog check where available.

## Dependency and handoff
Depends on WO-008 and WO-010. One dedicated commit and fresh validation; staged open is activated atomically with WO-012 native save cutover.
