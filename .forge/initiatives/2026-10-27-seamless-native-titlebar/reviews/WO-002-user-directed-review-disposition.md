# User-Directed Review Disposition — WO-002

## Record metadata

- **Record type:** One-time user-directed review-gate disposition; this is not a design-review report.
- **Work order:** [`WO-002-native-titlebar-double-click.md`](../work-orders/WO-002-native-titlebar-double-click.md)
- **Initiative:** [`../seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md)
- **Independent reviewer:** None. No reviewer session or independent design assessment is claimed.
- **Date:** `2026-10-28`; no more specific receipt timestamp is recorded.
- **Current disposition:** User-directed auto-pass/decision renewed for the performZoom-first scope below; supersedes the earlier Fill-only action decision. No fresh independent reviewer ran.

## Historical user direction and disposition — earlier Fill-only scope

The user expressly directed: “auto-pass the review of this work order (user direction)”. Disposition for WO-002 only: **user-directed auto-pass / one-time bypass of the independent design-review gate**.

This is an auditable record of the user's requested exception, not an independent `PASS`, not technical verification, and not validation evidence. It does not alter the global Forge process or project-manifest review policy, does not apply to any other work order, and does not waive implementation checks, the WO-001 pending fresh validation boundary, WO-002's clean Cargo gates, its manual user gate, or separate fresh validation.

## Historical separate implementation authorization

The user separately authorized a fresh implementer after readiness. WO-002 is not dispatched by this record. Dispatch remains conditional on the work-order preflight and on WO-001's pending fresh validation having a recorded disposition so the same-source work remains independently comparable. No implementation, validation, or user-setting change is claimed.

## Superseding user-directed decision — performZoom-first (2026-10-28)

The newer explicit user direction is: “further investigating, it seems like the `performZoom` call is the correct one, at least to start. lets edit the WO and review to affirm this as the desired change, and then we'll review during implementation.” The user additionally directed this artifact to record the newer user-directed auto-pass/decision, not an independent review.

- **Disposition for revised WO-002 only:** **user-directed auto-pass / one-time independent design-review bypass for the performZoom-first scope**. This is the user's decision and approval of the initial candidate, not an independent technical `PASS`, implementation verification, or validation result. No fresh independent reviewer was dispatched or ran. The earlier auto-pass and Fill-only decision remain above as superseded history.
- **Selected initial candidate:** native blank-top-band double-click routed through the current NSWindow's public [`performZoom:` action](../work-orders/WO-002-native-titlebar-double-click.md#implementation-plan-and-safety-boundary). It is not claimed equivalent to Fill, fullscreen, or every System Settings action. Fill discovery/public Fill support, preference discovery, and other preference support are not prerequisites.
- **Preserved boundaries:** live per-event AppKit handle, no cached raw pointer, synchronous main-thread/lifetime safety, native traffic-light and popup/menu/toolbar hit exclusions, no app-owned maximized state or egui maximize command, no fullscreen, no private selectors/synthetic events, and preserved appearance and stable native animation. All other WO safety, evidence, clean-check, and separate fresh-validation obligations remain intact.
- **Implementation-time user gate:** the separate implementer must launch the clean-built app and expose actual behavior for the user's review during implementation. API names/tests cannot establish the desired result. If behavior differs from desired behavior, stop and report before proposing alternatives; do not silently switch actions or change the contract. User acceptance and preservation evidence are required before claiming success.
- **Readiness/sequence:** WO-001's [`recorded failed validation`](../validations/validation-WO-001.md:64) satisfies predecessor sequencing; WO-001 remains blocked/not complete and its validation is unchanged. The earlier Fill-capability block no longer applies, so revised WO-002 is approved/ready for a new separate fresh implementer dispatch. This documentation record does not dispatch one or set the WO in progress.
- **Policy boundary:** this is a scope-specific user exception, not a change to project/global review policy. It waives neither implementation checks nor the actual-window user gate nor separate fresh validation. No code, build, test, launch, or validation is claimed by this disposition.
