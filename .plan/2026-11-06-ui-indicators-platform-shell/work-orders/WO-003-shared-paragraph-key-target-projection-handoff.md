# WO-003 — Shared paragraph-key target projection handoff

- **Submission date:** 2026-11-07
- **Branch:** `dev/2026-11-06-ui-indicators-platform-shell` (existing checked-out branch; no branch switch)
- **State:** Implementation submitted; not validated or accepted
- **Scope:** WO-003 only. WO-001 and WO-002 were unvalidated starting submissions and remain preserved.
- **Validation owner:** User, under `.forge/project-manifest.md` repository policy.

## Public core API for WO-004

Use `scriptcraft_core::editing::EditingSession::paragraph_key_targets()` on the current editor
session. It returns:

```rust
pub struct ParagraphKeyTargets {
    pub enter: ParagraphKeyTarget,
    pub tab: ParagraphKeyTarget,
}

pub enum ParagraphKeyTarget {
    Target(Role),
    NoOp,
    Unavailable,
}
```

- `Target(role)` is the displayable resolved role, including same-role Tab targets such as
  `Target(Role::General)`. It is not a guarantee that the edit will publish.
- `NoOp` is a resolved/consumed path with no displayable role, including strict-boundary rejects and
  Parenthetical policy cases that the editing command consumes without a target.
- `Unavailable` means there is no current hint decision, including noncollapsed selection; an
  Enter resolver error is also projected as unavailable. Actual `enter()` keeps its established
  `InvalidSelection`/`UnsupportedTransition` error behavior, while `tab()` keeps its consumed no-op
  behavior for noncollapsed selections.

The method is read-only: it does not simulate edits or change project, selection, Parenthetical
provenance, history, dirty state, revision, or preserved source. Enter/Tab editing and the projection
share private resolver functions; edit methods execute the resolver's action in the existing
transactional `mutate` path. The resolver incorporates the session's runtime `EditPolicy` and
generated-Parenthetical provenance, and uses canonical UTF-8 byte offsets. WO-004 should format the
returned `Role` values in the desktop layer and should not reproduce Enter/Tab transition rules.

The projection resolves the editing decision only. Preservation conflicts, element/text capacity,
or other transaction-time safety checks may still reject a later real command. Hints must not imply
that those checks were performed or guaranteed to pass.

## Changed files

- `crates/scriptcraft-core/src/editing.rs` — public typed API, shared Enter/Tab decisions, and narrow
  refactor so actual editing executes those shared decisions.
- `crates/scriptcraft-core/tests/paragraph_key_projection.rs` — focused projection and command
  comparison coverage.
- `.forge/requirements/features/screenplay-writing-loop.md` — read-only core projection contract.
- `.forge/blueprints/technical-context.md` — shared resolver/API architecture and limitation.
- `.forge/project-manifest.md` — WO-003 implementation submission evidence.
- `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md` — WO-003 status
  and implementation submission record.
- This handoff.

No dependency, lockfile, command catalog, desktop UI, modifier routing, or rendered status behavior
changed. WO-004–WO-007 were not implemented.

## Authored coverage

The new integration suite covers all 17 `Role` values at empty and nonempty paragraph ends, compares
projected target roles with the resulting caret paragraph after real Enter/Tab where applicable, and
asserts session equality for no-op/projection paths. Focused scenarios cover:

- strict Scene Heading, Character, and Transition interiors versus actual ends, including a
  multibyte UTF-8 string and byte offsets;
- strict-boundary policy disabled, with nonempty Character still retaining its Tab exception;
- blank versus nonblank Character Tab;
- collapsed versus noncollapsed and reversed selections (including Enter's real error and Tab's
  consumed no-op);
- Dialogue's interior split-to-Dialogue and end transition-to-Action;
- Parenthetical delimiter-only Tab, supported repair boundaries, generated-pair provenance, and
  automatic-Parenthetical-disabled behavior;
- projection repeatability and no changes to session equality.

The coverage is authored only. No test, build, check, lint, formatter, launch, manual verification,
review, independent validation, or other check was run. Compilation and all command/projection
behavior remain unverified; user review, validation, and acceptance are pending.

## WO-004 integration and acceptance notes

WO-004 can call `paragraph_key_targets()` from its current `EditingSession` and independently decide
whether to render Enter, Tab, both, or neither based on the desktop hint policy. `NoOp` and
`Unavailable` both contain no role, but remain distinct for UI policy/diagnostics. The projected
decision is not a speculative transaction and does not supersede the actual command result.

User-owned validation should verify every authored scenario, especially exact Parenthetical
delimiter/repair/generated outcomes, strict UTF-8 boundaries, noncollapsed selection suppression,
same-role General Tab target versus no mutation, and unchanged history/dirty/revision/source state
after projection. No validation or acceptance is claimed by this handoff.
