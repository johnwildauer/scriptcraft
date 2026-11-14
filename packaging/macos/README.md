# ScriptCraft macOS rebuild and packaging guide

## Purpose and distribution boundary

This source-owned packaging path builds an Apple Silicon `ScriptCraft.app` from the Rust workspace, ad-hoc signs the assembled bundle, and creates a compressed read-only DMG with an Applications shortcut. It is intended for local rebuilds after development changes.

The app and DMG are **ad-hoc signed and unnotarized**. Ad-hoc signing supplies a local bundle signature/integrity check; it does not authenticate a developer, satisfy Gatekeeper for downloaded software, or imply notarization. This guide makes no clean-launch, Developer ID, or notarization claim. The current artifact target is `aarch64-apple-darwin`; it is an artifact choice for this packaging path, not a permanent product platform-support declaration. No minimum macOS version is declared. The bundle declares an FDX document type as an `Editor` with `LSHandlerRank=Alternate`, using the `fdx` filename-extension fallback. This makes the app eligible for Launch Services handling; it neither captures nor installs the default handler, guarantees Finder presentation on every macOS configuration, nor validates document contents.

The icon source is the canonical shared [`AppIcon.png`](../resources/AppIcon.png). macOS package iconset/ICNS generation from that PNG is unchanged.

## Prerequisites

Install or select these prerequisites yourself before packaging; `package.sh` does not install tools, access the network, or use signing credentials:

- Rust and Cargo, with the `aarch64-apple-darwin` standard-library target installed. If needed, add it with `rustup target add aarch64-apple-darwin`.
- Xcode Command Line Tools selected for the active developer directory. `xcode-select -p` should identify the selected tools.
- The macOS command-line tools `sips`, `iconutil`, `plutil`, `codesign`, `hdiutil`, `lipo`, `otool`, and `shasum`, plus the ordinary shell/file utilities used by the script.
- No third-party notice-generation tool is required for this local package. The app bundle includes the repository-provided Courier Prime and Material Symbols license/attribution records.

Rust dependency license/notice review is deferred to a separate distribution-preparation decision and is not a gate for this local app/DMG. This deferral does not establish that later distribution has no dependency-notice obligations. The earlier cargo-about scans and their observed license identifiers are recorded in the initiative history, not treated as a completed legal review or an approved license inventory. Font license and attribution records are copied from the repository without copying font binaries into the bundle. The app's font bytes are already embedded in the executable at build time.

## Optional workspace checks

The repository policy leaves checks and validation to the user. Before packaging, run whichever of these workspace checks you choose, from the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Historical outcomes are not evidence for this checkout. Record any current failures and resolve them as appropriate before treating a package as accepted.

## Rebuild and package

From any current working directory, run the single entry point:

```sh
./packaging/macos/package.sh
```

The script resolves the repository root from its own location, checks the source inputs and required tools, reads the marketing version from `[package].version` in `apps/scriptcraft/Cargo.toml`, and requires the supplied square 1024×1024 PNG master. It then builds only package `scriptcraft` in release mode for `aarch64-apple-darwin`; generates ordinary and Retina iconset images from the master without cropping; assembles the app bundle, processed property list, bundle icon, and supplied font license/attribution records; and checks the property list and bundle layout. At runtime on macOS, the executable locates `Contents/Resources/AppIcon.icns` relative to its own executable inside the running `.app`, extracts the largest decodable PNG representation in that ICNS resource, and supplies those RGBA pixels to eframe. Eframe's macOS startup icon handler applies that same packaged artwork to the native application/Dock icon on the UI thread. The source PNG is embedded only as a fallback when no app-bundle icon resource is present (for example, a development run); a present but unreadable or unsupported bundle icon fails startup rather than silently substituting a different image. No runtime icon lookup uses a repository-relative or current-working-directory path, so relocation of the complete app bundle does not change the lookup.

After assembly, the script ad-hoc signs the complete app with no timestamp and without `--deep`, verifies its signature and arm64 executable architecture, and inspects dynamic-library linkage. It stages only the signed app and an `/Applications` symbolic link, creates a compressed read-only DMG, verifies and mounts the image read-only to inspect its contents/signature/link, then writes a SHA-256 sidecar. Required stages fail closed with an actionable error. Cleanup is limited to fixed generated paths under `target/macos-package/`; unrelated `target/` files and DMGs for other package versions are left alone.

## Output paths and naming

All generated package outputs are under the ignored build-output tree `target/macos-package/`:

```text
target/macos-package/ScriptCraft.app/
  Contents/Info.plist
  Contents/MacOS/scriptcraft
  Contents/Resources/AppIcon.icns
  Contents/Resources/Licenses/
    CourierPrime/OFL.txt
    CourierPrime/OFL-FAQ.txt
    CourierPrime/Attribution.txt
    MaterialSymbols/LICENSE
    MaterialSymbols/Asset-Attribution.md
target/macos-package/ScriptCraft-<version>-arm64.dmg
target/macos-package/ScriptCraft-<version>-arm64.dmg.sha256
```

`<version>` is the desktop package's Cargo version (currently `0.1.0`), not a separately maintained packaging value. The bundle build version is the initial integer `1`; the bundle identifier is `com.scriptcraft.app`. The bundle icon is `Contents/Resources/AppIcon.icns`, declared by `CFBundleIconFile=AppIcon` in `Info.plist`. On macOS, ScriptCraft explicitly gives eframe its documented empty-icon sentinel so eframe does not replace the Launch Services-owned application/Dock icon with eframe's own icon or raw source artwork. The app's visible macOS icon therefore remains the bundle icon; macOS does not use application icons as ordinary window icons. On non-macOS native targets, the source PNG is embedded and supplied as the runtime window icon. Temporary iconset, DMG staging, and verification files also live under the package output root and are generated intermediates, not source assets.

## Rebuilding after source changes

Preserve the source icon, plist template, and packaging script. After ordinary source changes, run the optional workspace checks you choose, rerun `./packaging/macos/package.sh`, and use only artifacts newly produced for the current Cargo version. Do not substitute an old DMG or checksum from another version. When the Cargo package version changes, the resulting DMG/checksum names change; previous-version DMGs are deliberately not removed by the script. Revisit Rust dependency license/notice obligations as part of any separately scoped distribution-preparation review.

## User-owned artifact inspection

After packaging, inspect the exact versioned paths produced by the script. Replace `<version>` below with the version read from `apps/scriptcraft/Cargo.toml`:

```sh
plutil -lint target/macos-package/ScriptCraft.app/Contents/Info.plist
plutil -p target/macos-package/ScriptCraft.app/Contents/Info.plist
file target/macos-package/ScriptCraft.app/Contents/MacOS/scriptcraft
lipo -info target/macos-package/ScriptCraft.app/Contents/MacOS/scriptcraft
codesign --verify --strict --verbose=2 target/macos-package/ScriptCraft.app
codesign -dv --verbose=4 target/macos-package/ScriptCraft.app
otool -L target/macos-package/ScriptCraft.app/Contents/MacOS/scriptcraft
hdiutil verify target/macos-package/ScriptCraft-<version>-arm64.dmg
shasum -a 256 -c target/macos-package/ScriptCraft-<version>-arm64.dmg.sha256
```

Inspect `CFBundleDocumentTypes` in the printed property list: the expected declaration is named `Final Draft document`, has only the `fdx` extension, role `Editor`, and rank `Alternate`. This packaged branch intentionally has neither `LSItemContentTypes` nor `UTImportedTypeDeclarations`: no verified imported UTI is available for this declaration, so the extension fallback is used instead of inventing or exporting a UTI. Confirm that neither the document declaration nor the top-level dictionary contains `NSDocumentClass`; this app routes Open Documents through its native adapter and is not declaring an `NSDocument` document architecture. The packaging script asserts the role, rank, sole fallback extension, mutually exclusive UTI/fallback branches, and absence of `NSDocumentClass` while assembling the bundle. An FDX extension association is routing metadata only: it does not establish that a selected file is valid FDX or promise import success.

Expected other evidence is an ad-hoc signature and the current `arm64` architecture, not Apple developer authentication or notarization. Investigate every non-system dynamic-library path rather than assuming recipients have Rust, Cargo, or development libraries installed. The script itself does not run Gatekeeper assessment. Successful package assembly or these static artifact observations are not Launch Services acceptance evidence.

## Finder/Dock Open Documents behavior and packaged-app acceptance matrix

The `fdx` extension fallback declares a candidate document-handling capability; no imported UTI has been verified or is declared. Keep four observations distinct: bundle metadata makes ScriptCraft eligible for Finder handling but does not guarantee it appears in every Open With list; a one-time Open With choice or a deliberate default-handler change is a user selection and is not caused by packaging; whether a Dock drop is accepted/highlighted is a separate Dock-eligibility behavior that must be observed; and neither association nor selection validates file contents. The ordinary content-read/import path determines whether bytes are valid FDX. Do not infer Finder or Dock behavior from source metadata alone.

For Finder or Dock Open Documents events, the app's retained main-thread target accepts a bare descriptor or a list containing exactly one item, and requests an NSURL through `fileURLValue` on that descriptor. Only a resulting local file URL with a valid native filesystem path that is nonempty UTF-8 and accepted by `DocumentPath` proceeds. The adapter does not require the descriptor type to equal `typeFileURL`, use a custom alias importer, or manually coerce aliases, bookmark data, or FSRefs. A missing URL value, a resulting non-file/non-local URL, an invalid path, or a multi-item list is rejected as a bounded intake error; a multi-document event is rejected in full, with no valid prefix opened. The target is registered immediately before eframe startup; a retained `NSApplicationWillFinishLaunchingNotification` observer re-registers that same target after AppKit setup and before Finder's launch document event. The existing winit application delegate is preserved, and the adapter does not create an `NSDocumentController`. When macOS emits separate single-document events instead, each valid event is handled independently in event order. No in-window file-drop behavior is implied.

Cold and running routing are deliberately different. The initial slot is finalized at the first root update. If no startup document argument owns it, all valid requests queued before that boundary compete under the first-valid rule: the first valid request becomes the initial document and follows the ordinary staged read/import/activation path; errors do not consume the slot. After that boundary, every valid request launches a separate child; on an already running receiver, every request launches a separate child. OS-originated opens bypass in-app Open replacement entirely: they do not prompt the receiver to save, request replacement acknowledgement, mutate or close the receiver, or terminate it. Starting a child is not proof that its document activated.

Failures have bounded ownership. Native intake errors, queue overflow, or child-spawn failure are shown by the receiving instance and leave its current document unchanged. A child that cannot read/import/activate its requested document shows its ordinary startup-open failure and remains untitled; a failed cold initial open leaves that initial instance untitled. No failure is presented as successful activation. The native inbox retains at most 16 events; practical overflow probes should confirm excess events are rejected without evicting earlier queued events and that feedback remains bounded.

Perform these scenarios as user-owned acceptance on a newly packaged, copied `.app`, launched through Finder—not through Cargo—as the packaging script does not launch or interactively verify the app. Record macOS version/build, exact copied bundle location, inspected metadata branch, default-handler state before and after, gesture and cold/running state, results, failures/remediation, and artifact evidence separately. Run only where the platform permits the described gesture or event shape; do not infer unobserved behavior from package creation.

1. **Cold initial open:** With ScriptCraft not running and no startup document argument, use a recorded one-time Finder Open With selection to probe Finder ingress, then separately drop an FDX onto the Dock app icon while it is not running. Record Finder eligibility/selection and Dock drop acceptance independently; neither is presumed from the other. For each delivered request, record whether the callback and bounded intake result occur. The implemented routing assigns the first valid request queued before the first root update to the initial slot; user acceptance of that outcome remains unperformed. Include paths with spaces and Unicode. Also try cold malformed FDX content: the initial instance should remain untitled and report ordinary open failure, not claim successful activation.
2. **Running clean receiver:** Keep a clean ScriptCraft instance open. Double-click one FDX in Finder and drop another onto the running app's Dock icon in separate trials. Each valid request after initial-slot allocation should open in an independent child; the receiver remains open and unchanged. Open two documents in sequential, separate Finder/Dock gestures and confirm independent children in request order.
3. **Running dirty receiver and source preservation:** Repeat the Finder and Dock trials after unsaved edits, including edits to a previously saved source. Compare that source's bytes or a user-owned hash before and after. Confirm the receiver's in-memory edits, title, undo/dirty state, and document remain editable and unchanged, with no OS-originated dirty prompt, replacement acknowledgement, or receiver close. Repeat with Title Page focused.
4. **Receiver lifecycle states:** Repeat OS opens while the receiver has a migration decision pending, then resolve the receiver's own decision using Continue and Cancel in separate runs; OS opening must neither resolve nor replace that state. Also try while a migration warning is displayed, a native output dialog is active where macOS permits event delivery, or an in-app replacement is pending/locked. The OS request remains independent and never participates in or resolves WO-002 replacement.
5. **Content and intake failures:** Open malformed FDX bytes under an `.fdx` name to demonstrate that the association is not content validation; failure belongs to the child, which remains untitled, not to the receiver. Where practical, probe a non-file URL/descriptor, invalid UTF-8 path, empty/malformed event, and one multi-file Open/Dock gesture. Confirm fixed bounded errors and unchanged receiver state. When one Apple event contains multiple URLs, none from that event opens; if macOS instead sends multiple single-file events, each valid event independently follows the child rule.
6. **Burst and queue behavior:** Where practical, deliver enough requests before the app drains the native inbox to exceed its 16-event capacity. Confirm accepted earlier events retain FIFO order, excess requests are rejected without evicting them, and one bounded overflow feedback is surfaced. Treat inability to force a reproducible burst as an untested item, not as a pass.
7. **No default capture:** Record the default FDX handler before trials and after them. Confirm packaging/launch did not silently change or claim the default. Finder's available Open With list may differ by macOS configuration; eligibility metadata does not establish list visibility. A one-time Open With selection is distinct from the user deliberately choosing ScriptCraft as the persistent default. Do not select a persistent default as part of this check.
8. **Evidence record:** Inspect the exact copied app's `Contents/Info.plist`, signature, architecture, and DMG/checksum artifacts using the artifact-inspection guidance above. Keep these observations distinct from Launch Services gesture outcomes and record any user-directed Open With selection or other remediation separately.

## Other packaged-app scenarios: in-app WO-001/WO-002 and general smoke

Keep OS-originated opens distinct from the existing in-app replacement and close contracts. In-app Open continues to use the WO-002 acknowledged replacement behavior; the following scenarios do not redefine it:

1. Mount the DMG in Finder. Confirm it contains the displayed ScriptCraft app and an Applications shortcut, then copy the app to a separate local folder and launch it through Finder rather than Cargo or a terminal.
2. Inspect the Finder/bundle icon and macOS Dock icon. Confirm the packaged ScriptCraft icon persists after startup and after opening a project; macOS windows do not have separate window icons. Exercise native menu items and shortcuts, toolbar controls, embedded Courier Prime screenplay faces, and Material Symbols icons.
3. Exercise in-app New and Open separately. Confirm New launches an independent blank instance using the app-bundle executable. For Open, test picker Cancel (no prompt or child), clean selection (no dirty prompt), and dirty Save / Don’t Save / Cancel: Cancel launches nothing; Save proceeds only after a successful Save or Save As leaves the session clean; exercise Save As picker cancellation, save failure, and Saved-but-still-dirty defense and confirm each launches no child; Don’t Save proceeds without writing or reverting the source. Compare a user-owned before/after hash or byte copy of a previously saved source around Don’t Save, including a delayed replacement and a failed replacement. Confirm the parent remains open and locked while acknowledgement is pending, and closes only after the selected document is active in the child. A child process start alone is not success.
4. In that Open matrix, cover non-Version-6 migration pending, Continue followed by successful activation, Cancel, and conversion failure. Also exercise launch/read/version/import/activation failure, early child exit with zero and nonzero status, malformed/closed acknowledgement, timeout, and a late Opened after timeout. Confirm every non-Opened result resumes the original parent session/title and leaves its in-memory edits available; timeout does not kill the child. Repeat relevant cases with Title Page focused and selected paths containing spaces and Unicode. Confirm New remains independent.
5. Create/edit a document; exercise undo/redo, FDX Save and Save As, close/reopen, title changes, and PDF export. Include Unicode content and an existing-destination overwrite decision. Confirm the native FDX remains the source of truth and preserve current product behavior.
6. With a clean Body session, exercise File > Close and confirm no unsaved-changes prompt appears. With dirty Body content, exercise File > Close and Command-Q separately; choose Cancel and confirm the root remains open, then choose Save and confirm closure follows only after saving leaves the session clean. Also exercise Save picker cancellation or a deliberate save failure and confirm the root remains open with the document and unsaved changes preserved.
7. Repeat dirty-close scenarios with the Title Page window focused. File > Close should close only the Title Page child; Command-Q and Dock Quit should request guarded root closure and present the same dirty-session choice. Choose Don’t Save after editing a previously saved FDX, then reopen that exact file through the app and confirm its on-disk contents remain at the last saved version. This checks that Don’t Save is not a save, revert, or dirty-state reset.
8. Exercise appearance, zoom, fullscreen, status guidance, and in-app sprint-timer completion. Treat operating-system notification authorization and delivery as separately unverified; bundling does not guarantee notification delivery.
9. Confirm the copied app works after relocation and does not depend on the repository working directory or source font paths. Verify the copied bundle signature after copying. If performing an explicit integrity-negative experiment, do so only on a disposable copy and confirm a modified bundled file is detected.
10. Inspect the mounted DMG and compare the checksum sidecar with the exact versioned DMG. Record macOS/build details, launch location, outcomes, failures/remediation, and the checksum in a user-owned validation record. Script completion alone is not package acceptance.

## Recipient Gatekeeper guidance

Because this artifact is unsigned by an Apple Developer ID and unnotarized, a recipient may see a Gatekeeper warning, especially for a quarantined download. Only when the recipient trusts the source and intentionally accepts that risk, use macOS's trust-qualified right-click **Open** flow for the app, or the applicable **Privacy & Security → Open Anyway** choice after the blocked first launch. The exact prompts and availability depend on macOS version and local policy. Do not disable Gatekeeper, remove quarantine attributes, or represent either bypass as notarization or a clean-launch guarantee.

## Troubleshooting

- **Missing Rust target or Xcode tools:** Install/select the user-approved prerequisite separately. Confirm `rustup target list --installed` includes `aarch64-apple-darwin` and `xcode-select -p` succeeds, then rerun the script. The script does not install prerequisites.
- **Icon generation/dimensions:** Confirm the shared [`AppIcon.png`](../resources/AppIcon.png) is the supplied 1024×1024 PNG and that `sips` and `iconutil` are available. Preserve the master; do not crop or replace it with a generated iconset image. A failed icon generation aborts packaging before signing.
- **Runtime macOS Dock icon:** Eframe's macOS startup icon setter can replace the app-bundle icon with the supplied image. The app deliberately supplies eframe's empty-icon sentinel on macOS, leaving the icon declared by `CFBundleIconFile` under Launch Services ownership. Do not restore a parsed-PNG eframe icon on macOS. Non-macOS native builds continue using the embedded PNG. Rebuild with `./packaging/macos/package.sh` after changing the bundle icon or runtime setup.
- **Font license or attribution records:** Confirm the source records remain at their expected `fonts/` paths; the script checks and copies them into the app bundle. Rust dependency notice generation is intentionally outside this local packaging flow and remains a separate distribution-preparation review item.
- **Stale generated output:** The script replaces the fixed `ScriptCraft.app` and current-version package outputs/intermediates only under `target/macos-package/`. Old versioned DMGs are intentionally preserved; identify the desired version explicitly and do not delete the broad `target/` tree. Remove only a known package-generated file if a user decides it is stale.
- **Signature verification:** Ensure all bundle files/resources are present before signing and rerun the script to rebuild the app in the required order. Do not add `--deep` as a bundle-construction workaround. Ad-hoc signatures are not Developer ID signatures and are not notarized.
- **Non-system dynamic-library linkage:** Review `otool -L` output and resolve each unexpected path before distribution. Do not assume a recipient has the local toolchain, build directory, or non-system library installed.
- **Finder or Dock does not open ScriptCraft for FDX:** Follow the evidence stages separately: (1) inspect the copied bundle's `Editor` / `Alternate` extension-fallback declaration for Finder eligibility, and record Finder Open With availability; (2) record whether the user selected ScriptCraft one-time or as the default, and record the existing default handler independently; (3) record whether the Dock icon accepts/highlights a drop; (4) record whether ScriptCraft launches or activates; (5) record whether the native Open Documents callback occurs and whether its file-URL intake accepts or rejects the event; (6) record whether routing assigns the initial slot or launches a child; (7) record whether content read/import/activation succeeds. Activation is not callback evidence, and a callback is not content success. Use a user-directed one-time Open With selection to diagnose Finder ingress; do not use an **Always Open With** choice unless deliberately testing default-handler behavior. Do not prescribe broad Launch Services resets or substitute a Cargo run for copied-bundle behavior. No troubleshooting step disables Gatekeeper.
- **FDX is routed but will not open:** The extension declaration only identifies a candidate handler; it does not validate XML/FDX content. Check which instance displayed the bounded failure: an independent child may remain untitled after an open failure, while receiver-side intake/spawn errors leave its document unchanged. Do not interpret app activation or child process creation as successful document opening.
- **DMG creation or mounting:** Confirm the output volume has enough free space and no application is using a prior mounted image. If a prior verification mount remains, eject that specific image in Finder or with `hdiutil detach` using its actual mount point, then rerun. The script only mounts its newly created image read-only for its own layout/signature inspection and detaches it before publishing the checksum.

## Validation ownership

The user owns all workspace checks, packaging command execution, artifact inspection, Finder launch, functional smoke, recipient behavior, review, and acceptance. This source guide and packaging implementation do not claim compilation, a produced artifact, successful signing, Gatekeeper behavior, application behavior, or recipient acceptance until the user performs and records those steps.
