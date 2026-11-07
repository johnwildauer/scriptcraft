# ScriptCraft Windows x64 portable packaging

## Purpose and distribution boundary

This source-owned packaging path builds the `scriptcraft` desktop package for `x86_64-pc-windows-msvc` and creates a relocatable portable ZIP for 64-bit Windows. It does not create an installer, modify the registry, install file associations, install fonts, or require a recipient to have Rust or Cargo installed.

The first artifact is **unsigned**. No Authenticode signing, timestamping, SmartScreen reputation, or other Microsoft trust approval is claimed. Windows may show a SmartScreen or unknown-publisher warning for a downloaded executable; only a recipient who trusts the source should choose whether to run it. Do not describe this package as signed or security-approved.

The application embeds its Courier Prime and Material Symbols font bytes at compile time. The package therefore carries the corresponding license and attribution records, not a requirement to install the fonts system-wide. Rust dependency license/notice review remains a separate distribution-preparation task, as it is for the macOS flow.

The release executable uses the Windows GUI subsystem and does not open a companion console window. Debug builds retain the console subsystem for diagnostics. This describes the executable subsystem only; it does not mean the application is headless or claim that GUI launch or functional testing has been performed.

## Source files

- [`package.ps1`](package.ps1) is the single build and packaging entry point.
- [`../resources/AppIcon.png`](../resources/AppIcon.png) is the canonical shared artwork source. The package stages this PNG as `AppIcon.png` in the portable folder, and the runtime window icon uses this shared PNG.
- [`../resources/AppIcon.ico`](../resources/AppIcon.ico) is a separate, manually maintained Windows derivative. Windows Cargo builds embed this ICO into `ScriptCraft.exe` for the Explorer/EXE icon; it is not staged as a separate portable-package file.
- [`../resources/README.md`](../resources/README.md) records shared icon ownership and the manual ICO update procedure.
- [`../../apps/scriptcraft/Cargo.toml`](../../apps/scriptcraft/Cargo.toml) supplies the package name and version.
- The font license and attribution inputs are the existing records under [`../../fonts/`](../../fonts/).

Keep the shared icon resources and this packaging directory in version control. Generated executables, staging directories, ZIPs, and checksums belong under the ignored `target/windows-x64-package/` tree and must not be committed.

## Prerequisites

Prepare these prerequisites before running the script. The script checks for tools and inputs but does not install software, access the network, or use signing credentials.

1. A 64-bit Windows 10 or Windows 11 machine.
2. PowerShell 5.1 or later, or PowerShell 7+.
3. Rustup, Cargo, and the `x86_64-pc-windows-msvc` Rust standard-library target. If the target is missing, install it separately with `rustup target add x86_64-pc-windows-msvc`.
4. Visual Studio Build Tools or Visual Studio with the **Desktop development with C++** workload, including an MSVC x64 linker and a Windows SDK. A Rust installation without a working MSVC linker is not sufficient.
5. Enough free disk space for the release build and the staged ZIP.

The script uses the MSVC target explicitly even when the host default target differs. It does not select the GNU toolchain or cross-compile from macOS.

## Optional workspace checks

From the repository root, run whichever checks are appropriate for the current change before packaging:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

A previous successful check is not evidence for the current checkout. Record and resolve current failures before treating an artifact as accepted.

## Build and package

From the repository root, run:

```powershell
& .\packaging\windows-x64\package.ps1
```

If the local PowerShell execution policy blocks the script, use a one-process policy override rather than changing the machine policy:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\windows-x64\package.ps1
```

The script resolves the repository root from its own location, so it may also be invoked by its full path from another working directory. It performs the following required stages and fails closed with an actionable error:

1. Checks the manifest, icon, font records, Cargo/Rustup, MSVC target, and linker prerequisites.
2. Reads the version from the desktop package manifest rather than maintaining a second packaging version.
3. Builds only the `scriptcraft` package in release mode for `x86_64-pc-windows-msvc`.
4. Verifies that the produced executable is a PE32+ AMD64 image with the `IMAGE_SUBSYSTEM_WINDOWS_GUI` (2) optional-header subsystem, not merely a file with an `.exe` suffix.
5. Embeds the manually maintained ICO into the release executable at build time, then recreates only the fixed staging directory for the current package version and copies the executable, `AppIcon.png`, a short recipient README, and the font license/attribution records into it. The runtime window icon continues to use the shared PNG.
6. Creates a ZIP with one top-level portable folder, verifies the expected entries by reading the archive, and writes a SHA-256 sidecar for the exact ZIP filename.

The script does not run the GUI, claim functional acceptance, sign the executable, or silently add unrelated files from `target/`.

## Output paths and layout

For manifest version `<version>`, the generated outputs are:

```text
target/windows-x64-package/
  ScriptCraft-<version>-windows-x64/
    ScriptCraft.exe
    AppIcon.png
    README.txt
    Licenses/
      CourierPrime/
        OFL.txt
        OFL-FAQ.txt
        Attribution.txt
      MaterialSymbols/
        LICENSE
        Asset-Attribution.md
  ScriptCraft-<version>-windows-x64.zip
  ScriptCraft-<version>-windows-x64.zip.sha256
```

The ZIP contains the `ScriptCraft-<version>-windows-x64/` folder at its root. Extract the complete folder anywhere writable, then launch `ScriptCraft.exe` from that folder. Do not copy only the executable out of the folder if the recipient README or license records are needed for distribution compliance. The application itself does not require a repository-relative working directory.

The version is currently `0.1.0` in [`../../apps/scriptcraft/Cargo.toml`](../../apps/scriptcraft/Cargo.toml), but the filename is always derived from the current manifest. Existing ZIPs for other versions are preserved; do not substitute an old artifact or checksum.

## Rebuilding after source changes

Preserve [`package.ps1`](package.ps1), the shared [`AppIcon.png`](../resources/AppIcon.png), and the source font records. After source changes, rerun the optional checks selected above and then rerun [`package.ps1`](package.ps1). Use only newly produced outputs for the current manifest version. The script may replace its fixed current-version staging directory and current-version ZIP/checksum, but it must not delete the broad `target/` tree or unrelated package versions.

If the shared artwork changes, update [`../resources/AppIcon.png`](../resources/AppIcon.png) deliberately and rebuild; the Windows package stages this PNG and the runtime window icon uses it. Update the manually maintained [`../resources/AppIcon.ico`](../resources/AppIcon.ico) when needed for Windows Explorer/EXE artwork, then rebuild so the new ICO is embedded in `ScriptCraft.exe`. Cargo consumes the committed ICO and does not run ImageMagick or generate it. The ICO remains inside the executable and is not an extra file in the portable ZIP. Explorer may cache icons, so a refresh or copying the package to a new folder may be needed to see an updated icon.

## Artifact inspection

After packaging, inspect the exact versioned paths. Replace `<version>` with the manifest version:

```powershell
$packageRoot = Join-Path (Get-Location) 'target\windows-x64-package'
$zip = Join-Path $packageRoot 'ScriptCraft-<version>-windows-x64.zip'
Get-FileHash -Algorithm SHA256 $zip
Get-Content "$zip.sha256"
Get-AuthenticodeSignature (Join-Path $packageRoot 'ScriptCraft-<version>-windows-x64\ScriptCraft.exe')
```

The checksum sidecar uses the format `<lowercase-sha256>  <zip-filename>`. Recalculate the hash and compare it with the sidecar; do not treat the mere existence of a sidecar as verification. An unsigned executable is expected in this pass. If `dumpbin` is available from a Visual Studio developer environment, `dumpbin /dependents ScriptCraft.exe` can additionally be used to review runtime DLL dependencies. Investigate any non-system dependency before distribution; portable means no installer, not a guarantee that every Windows runtime component is bundled.

## Recipient launch and functional acceptance

Perform these user-owned checks on a suitable Windows x64 machine after extracting the ZIP to a fresh local folder outside the repository:

1. Launch `ScriptCraft.exe` by double-clicking it, not through Cargo. Confirm the window opens and the ScriptCraft artwork appears where the current runtime supports it.
2. Exercise the in-window Windows menu and Ctrl-based shortcuts, toolbar controls, embedded Courier Prime screenplay faces, and Material Symbols icons.
3. Create and edit a document; exercise undo/redo, FDX Save and Save As, close/reopen, Unicode text, title changes, and PDF export. Confirm the native FDX remains the source of truth.
4. Exercise New and Open child-instance behavior, cancellation, and visible error handling. Confirm the child executable is resolved relative to the running portable executable rather than the repository.
5. Exercise appearance, zoom, fullscreen, status guidance, and sprint-timer completion. Treat operating-system notification permission and delivery as separately unverified.
6. Move the entire extracted folder to another local path and repeat a launch and save/open smoke test. Confirm no source font path, Cargo installation, or repository working directory is needed.
7. Record Windows/build details, extraction and launch locations, outcomes, failures/remediation, and the SHA-256 value in a user-owned validation record. Script completion and structural checks alone are not package acceptance.

## Troubleshooting

- **Target not installed:** Run `rustup target list --installed` and install `x86_64-pc-windows-msvc` separately if absent. Then rerun the script.
- **Linker or Windows SDK missing:** Open a Visual Studio x64 Native Tools developer prompt, verify that `link.exe` is available, and install/repair the Desktop development with C++ workload and Windows SDK if needed. The packaging script does not download these components.
- **PowerShell blocks the script:** Use the one-process `ExecutionPolicy Bypass` command shown above only for a trusted local source tree; do not change the machine-wide policy as a packaging workaround.
- **Build or dependency failure:** Read the first Cargo/compiler error, resolve it in the source or toolchain, and rerun. Do not publish a partial staging directory or an old ZIP as a replacement.
- **Missing runtime DLL on a recipient machine:** Inspect dependencies with `dumpbin /dependents` where available and determine whether a Microsoft runtime prerequisite or a separate packaging change is required. Do not copy arbitrary DLLs from a development machine into the ZIP.
- **SmartScreen or unknown-publisher warning:** This is expected to be possible for an unsigned artifact. Establish source trust and distribution signing policy separately; do not represent a bypass as signing or approval.
- **Stale output:** Remove only a known package-generated current-version output if necessary. Preserve unrelated `target/` contents and other package versions.

## Validation ownership

The user owns workspace checks, packaging command execution, artifact inspection, GUI launch, functional smoke testing, recipient behavior, review, and acceptance. This guide and the packaging implementation do not claim compilation, a produced artifact, successful launch, runtime dependency completeness, SmartScreen behavior, or recipient acceptance until those checks are performed and recorded.
