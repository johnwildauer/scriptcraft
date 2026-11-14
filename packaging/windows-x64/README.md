# ScriptCraft Windows x64 portable packaging

## Purpose and distribution boundary

This source-owned packaging path builds the `scriptcraft` desktop package for `x86_64-pc-windows-msvc` and creates a relocatable portable ZIP for 64-bit Windows. It does not create an installer, modify the registry, install file associations, install fonts, or require a recipient to have Rust or Cargo installed. A deliberate association choice belongs to the user and Windows may record it; that OS-owned choice is not package registration.

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
5. Embeds the manually maintained ICO and the application resource script, including the Common Controls v6 activation manifest, into the release executable at build time; then recreates only the fixed staging directory for the current package version and copies the executable, `AppIcon.png`, a short recipient README, and the font license/attribution records into it. The embedded manifest enables the native Task Dialog used for the unsaved-changes choice; it is not an installer or registry change. The runtime window icon continues to use the shared PNG.
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
4. With a clean Body session, close the root window and confirm no unsaved-changes prompt appears. With dirty Body content, exercise File > Close and the root window Close control; inspect the native Task Dialog and try Save, Don’t Save, and Cancel. Cancel must leave the root open; Save must close only after a successful save leaves the session clean; cancellation/failure during Save must leave the document open with its unsaved changes preserved.
5. With a previously saved FDX, make an edit and choose Don’t Save. Reopen that exact file through the app and confirm the on-disk file still contains the last saved version. With Title Page focused, File > Close should close only that child window and leave the root document open.
6. Exercise New and in-app Open separately, confirming release New uses the independent blank-instance path and in-app Open uses the portable executable resolved relative to the running package folder, not the repository. Confirm a release launch has no companion console window. For in-app Open, test picker cancellation, clean selection, and dirty Save / Don’t Save / Cancel; Save proceeds only after a successful Save or Save As leaves the session clean; exercise Save As picker cancellation, save failure, and Saved-but-still-dirty defense and confirm each launches no child. Don’t Save does not write/revert the source. Compare a user-owned before/after hash or byte copy of a previously saved FDX around Don’t Save, including delayed acknowledgement and failed replacement. Confirm the parent remains open and locked until the selected document is active in the child; process start is not success.
7. Extend the in-app Open matrix with non-Version-6 migration pending, Continue plus activation, Cancel, and conversion failure; launch/read/version/import/activation failure; early child exit with zero and nonzero status; malformed/closed acknowledgement; timeout and late Opened. Confirm each non-Opened result resumes the original parent session/title with edits still available and timeout does not kill the child. Repeat relevant cases with Title Page focused and paths containing spaces and Unicode; confirm New remains independent. This is not a positional-argument or Windows-shell-open scenario.
8. Windows shell opens are independent new-process opens, not part of this in-app replacement contract. Follow the separate Windows shell opening and association guidance below; the package retains its no-installer, no-package-registration boundary.
9. Exercise appearance, zoom, fullscreen, status guidance, and sprint-timer completion. Treat operating-system notification permission and delivery as separately unverified.
10. Move the entire extracted folder to another local path and repeat a launch and save/open smoke test. Confirm no source font path, Cargo installation, or repository working directory is needed.
11. Record Windows/build details, extraction and launch locations, outcomes, failures/remediation, and the SHA-256 value in a user-owned validation record. Script completion and structural checks alone are not package acceptance.

## Windows shell opening and user-owned associations

Launching `ScriptCraft.exe` without a document path remains an ordinary blank launch. A shell document launch supplies the executable and exactly one quoted native path as a separate argument. For example, from PowerShell:

```powershell
& 'C:\ScriptCraft\ScriptCraft.exe' 'C:\Users\Avery\Screenplays\Café Scene\夜の脚本.fdx'
```

Windows preserves the path as one native argument, including its spaces and Unicode characters. Do not split, re-quote, or combine path text. Two positional paths, or a positional path combined with a custom document input, are ambiguous and rejected in full; no valid prefix is opened. Unknown options and invalid startup presentation are rejected visibly before a document window is created. The shell-open instructions here use only the one-path form and do not require any internal application options.

For a valid shell open, the newly started initial process opens the selected document directly. It does not forward the request to an existing instance or replace that instance's document. Every already-running instance remains open and unchanged, whether clean or dirty. Read, content validation, version, import, or activation failures show bounded native feedback in the requesting new instance and leave that instance untitled; the feedback does not expose the full path or screenplay content. For non-Version-6 files, Continue follows the existing migration flow, and any Continue failure is reported as a failure. Cancel is a distinct user decision: it leaves the requesting new instance untitled without presenting Cancel itself as an open failure.

The ZIP is portable: it does not install an FDX association, and Windows may not offer ScriptCraft in Open With until the user browses to the extracted executable. To make a user-owned choice on Windows 10 or 11:

1. Right-click one `.fdx` file and choose **Open With**. If ScriptCraft is not listed, choose **Choose another app** (or the equivalent version-specific option), then use the browse route to select `ScriptCraft.exe` in the extracted package folder.
2. Choose the one-time open option to use ScriptCraft only for that selection. Choose **Always** or the offered default-app choice only if you deliberately want Windows to use ScriptCraft as the default for that file type. Labels and dialog layout vary by Windows version.
3. Where available, use Windows **Settings > Apps > Default apps** to review or deliberately change the default. Windows controls and may record a user-selected association; ScriptCraft's ZIP does not create it. If the package folder or executable is moved later, the user may need to update that Windows-owned choice to the new location.

Opening from Explorer, Open With, or a user-selected default always requests a new independent process. Opening two files sequentially therefore creates two independent opens; it does not discover, activate, prompt, save, lock, acknowledge, or close another instance. A deliberate Windows default and a one-time Open With selection have different association effects, so record any observed change as the user's choice, not as evidence that packaging registered an association.

Drag behavior is limited to dragging one file onto `ScriptCraft.exe` or onto a suitable shortcut created by the user; Windows supplies the same one-path argument convention. A multi-file drag combined into one invocation is rejected in full, with no partial opens. If Windows instead starts separate one-path processes, each is an independent open. Taskbar-hover delivery and dragging onto the taskbar are not supported or implied, and dropping files inside an application window is outside this scope.

### User-owned Windows shell acceptance matrix

Run these scenarios against the extracted portable executable on Windows 10 or 11, not through Cargo. These are validation instructions, not results or acceptance claims. For each run, record the Windows version/edition, package version and ZIP hash, extraction path, gesture or invocation, process/window outcome, any failure and remediation, and source-byte comparisons. Record association/default state before and after each deliberate Windows choice and distinguish that choice from package behavior.

| Scenario | User-owned check |
| --- | --- |
| Ordinary and cold launch | With ScriptCraft fully closed, launch `ScriptCraft.exe` without a path and confirm the ordinary blank initial window. Then use Open With to open one valid Version 6 FDX and confirm it opens directly in the new initial window, without an extra blank child or console. |
| Existing clean and dirty instances | Repeat shell opening while clean and dirty instances already exist. Confirm each request creates a new independent process and every existing instance remains open, editable, and unchanged in document, title, selection, dirty state, and on-disk bytes, with no prompt, save, lock, acknowledgement, replacement, or close. |
| Spaces and Unicode | Open a valid FDX whose directory and filename contain spaces and non-ASCII characters. Confirm the exact selected file opens as one path and no path text is split, lost, or substituted. |
| Sequential opens | Open two different FDX files one after the other from Explorer/Open With. Confirm two independent new instances each open their requested document and neither request is delivered to the other process. |
| Multi-selection | Select multiple FDX files and invoke the available Explorer/Open With action. If Windows supplies more than one positional path to one process, confirm the invocation is visibly rejected in full and no selected file opens as a valid prefix. If Windows starts separate one-path processes, confirm each open is independent. |
| Open With versus deliberate default | First use the one-time Open With choice and observe whether Windows changes its default. Separately make a deliberate **Always**/default choice or use **Settings > Apps > Default apps**, then open an FDX and confirm Windows launches the extracted executable. Record any OS-owned association change alongside the deliberate choice; do not attribute it to ZIP extraction or packaging. |
| Invalid and unreadable documents | Request missing, unreadable, malformed, and non-FDX content (including non-FDX content renamed with an `.fdx` extension). Confirm the new requesting process reports a bounded native failure, remains untitled, and does not rewrite the source or alter another instance. |
| Non-Version-6 outcomes | Open non-Version-6 files and exercise Continue, Cancel, and a Continue path that fails. Confirm successful Continue alone activates the document; Cancel leaves only the new instance untitled without being reported as failure; an actual Continue/import/activation failure is visibly reported and leaves it untitled. |
| Executable and shortcut drags | Drag one valid FDX onto the extracted `ScriptCraft.exe`, then separately onto a suitable user-created shortcut. Confirm each supported gesture opens directly in its independent new process. |
| Multi-file drags | Drag multiple FDX files together onto the executable or shortcut. If Windows sends multiple paths to one process, confirm all are rejected with no partial open; if Windows starts separate single-path processes, confirm their independent-open behavior. |
| Relocation | After making a deliberate default association, move the complete extracted folder and test the old choice. Confirm and record whether Windows requires the user to update the association to the new executable location; do not treat the portable ZIP as an installer. |
| Native invalid-invocation errors and console | Exercise two positional paths, positional plus custom input, an unknown option, and invalid startup presentation where available. Confirm each invalid invocation produces bounded native feedback before any document window, opens no valid prefix, and creates no console. Confirm valid blank and one-path launch do not show an invalid-invocation error and release launches have no companion console. |
| Source-byte preservation and excluded drops | Compare user-owned before/after byte copies or hashes for the requested source around successful open, failed open, migration Cancel, and interaction with existing dirty instances; confirm shell opening does not rewrite the source. Confirm no taskbar-hover/taskbar drop or in-window file drop is required or implied by these checks. |

Do not use a registry editor, association commands, SmartScreen-disabling steps, administrative or machine-wide changes, or any package-supplied registration option to perform these checks. Package/build completion, association discoverability, and this matrix's documentation do not establish Windows behavior; the user owns the observations and acceptance decision.

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
