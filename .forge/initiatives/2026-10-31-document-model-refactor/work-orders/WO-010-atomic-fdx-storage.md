# Work Order: WO-010 — Atomic FDX storage and conflict safety

## Outcome
Provide filesystem-safe storage for complete validated FDX candidates independently of desktop commands.

## Scope
Implement bounded reads and same-directory uniquely created temporary writes, conflict detection, replacement and cleanup; define actual platform support and sync/durability limitations. Retain transitional JSON paths until native cutover.

## Relevant references
- [Persistence safety](../../../blueprints/technical-context.md#fdx-model-and-ownership-contracts)
- [Current persistence](../../../../crates/scriptcraft-core/src/persistence.rs)

## Implementation
Detect external modification against the open/save baseline; specify Save As collisions, symlink and permission behavior and fail safely where an atomic guarantee cannot be met. Separate preparation from filesystem effects. Never describe atomic rename alone as durable across power failure. No automatic backup is required.

## Acceptance criteria
- Preparation, conflict, temp creation, partial write, flush/sync and replacement failures preserve the prior destination and caller's session; temporary cleanup is attempted and reported safely.
- Save As never mutates the source; successful replacement returns enough evidence for updating the baseline only after success.

## Useful checks
- Inject each failure and concurrent/external modification; test temp collisions, symlinks and permissions on applicable platforms; document untested platform guarantees and run workspace checks.

## Dependency and handoff
Depends on WO-007. One dedicated commit and fresh validation; supply storage API and failure contract to WO-012.
