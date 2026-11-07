# ScriptCraft macOS rebuild and packaging guide

## Purpose and distribution boundary

This source-owned packaging path builds an Apple Silicon `ScriptCraft.app` from the Rust workspace, ad-hoc signs the assembled bundle, and creates a compressed read-only DMG with an Applications shortcut. It is intended for local rebuilds after development changes.

The app and DMG are **ad-hoc signed and unnotarized**. Ad-hoc signing supplies a local bundle signature/integrity check; it does not authenticate a developer, satisfy Gatekeeper for downloaded software, or imply notarization. This guide makes no clean-launch, Developer ID, or notarization claim. The current artifact target is `aarch64-apple-darwin`; it is an artifact choice for this packaging path, not a permanent product platform-support declaration. No minimum macOS version is declared, and the bundle does not register FDX document associations or claim Finder double-click/Open With support.

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
file target/macos-package/ScriptCraft.app/Contents/MacOS/scriptcraft
lipo -info target/macos-package/ScriptCraft.app/Contents/MacOS/scriptcraft
codesign --verify --strict --verbose=2 target/macos-package/ScriptCraft.app
codesign -dv --verbose=4 target/macos-package/ScriptCraft.app
otool -L target/macos-package/ScriptCraft.app/Contents/MacOS/scriptcraft
hdiutil verify target/macos-package/ScriptCraft-<version>-arm64.dmg
shasum -a 256 -c target/macos-package/ScriptCraft-<version>-arm64.dmg.sha256
```

Expected evidence is an ad-hoc signature and the current `arm64` architecture, not Apple developer authentication or notarization. Investigate every non-system dynamic-library path rather than assuming recipients have Rust, Cargo, or development libraries installed. The script itself does not run Gatekeeper assessment.

## Finder launch and functional acceptance scenarios

Perform these user-owned scenarios on a suitable macOS system; the packaging script does not launch or interactively verify the app:

1. Mount the DMG in Finder. Confirm it contains the displayed ScriptCraft app and an Applications shortcut, then copy the app to a separate local folder and launch it through Finder rather than Cargo or a terminal.
2. Inspect the Finder/bundle icon and macOS Dock icon. Confirm the packaged ScriptCraft icon persists after startup and after opening a project; macOS windows do not have separate window icons. Exercise native menu items and shortcuts, toolbar controls, embedded Courier Prime screenplay faces, and Material Symbols icons.
3. Exercise bundled child-instance New and Open. Confirm accepted actions launch independent child instances using the app-bundle executable, cancellation launches no child, and launch/open failure remains visibly reported without disclosing screenplay content.
4. Create/edit a document; exercise undo/redo, FDX Save and Save As, close/reopen, title changes, and PDF export. Include Unicode content and an existing-destination overwrite decision. Confirm the native FDX remains the source of truth and preserve current product behavior.
5. Exercise appearance, zoom, fullscreen, status guidance, and in-app sprint-timer completion. Treat operating-system notification authorization and delivery as separately unverified; bundling does not guarantee notification delivery.
6. Confirm the copied app works after relocation and does not depend on the repository working directory or source font paths. Verify the copied bundle signature after copying. If performing an explicit integrity-negative experiment, do so only on a disposable copy and confirm a modified bundled file is detected.
7. Inspect the mounted DMG and compare the checksum sidecar with the exact versioned DMG. Record macOS/build details, launch location, outcomes, failures/remediation, and the checksum in a user-owned validation record. Script completion alone is not package acceptance.

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
- **DMG creation or mounting:** Confirm the output volume has enough free space and no application is using a prior mounted image. If a prior verification mount remains, eject that specific image in Finder or with `hdiutil detach` using its actual mount point, then rerun. The script only mounts its newly created image read-only for its own layout/signature inspection and detaches it before publishing the checksum.

## Validation ownership

The user owns all workspace checks, packaging command execution, artifact inspection, Finder launch, functional smoke, recipient behavior, review, and acceptance. This source guide and packaging implementation do not claim compilation, a produced artifact, successful signing, Gatekeeper behavior, application behavior, or recipient acceptance until the user performs and records those steps.
