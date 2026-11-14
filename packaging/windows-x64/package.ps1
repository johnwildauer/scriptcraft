$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$TargetTriple = 'x86_64-pc-windows-msvc'
$ScriptDirectory = $PSScriptRoot
$RepositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $ScriptDirectory '..\..'))
$PackageRoot = Join-Path $RepositoryRoot 'target\windows-x64-package'
$DesktopManifest = Join-Path $RepositoryRoot 'apps\scriptcraft\Cargo.toml'
$IconSource = Join-Path $RepositoryRoot 'packaging\resources\AppIcon.png'

function Stop-Packaging([string] $Message) {
    throw "package.ps1: $Message"
}

function Require-File([string] $Path, [string] $Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-Packaging "required $Description is missing: $Path"
    }
}

function Get-CommandPath([string] $Name, [string] $Remediation) {
    $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $command) {
        Stop-Packaging "required tool '$Name' is unavailable; $Remediation"
    }
    return $command.Source
}

function Assert-NotReparsePoint([string] $Path, [string] $Description) {
    if (Test-Path -LiteralPath $Path) {
        $item = Get-Item -LiteralPath $Path -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            Stop-Packaging "$Description is a symbolic link or reparse point; refusing to modify it: $Path"
        }
    }
}

function Remove-OwnedPath([string] $Path, [string] $Description) {
    Assert-NotReparsePoint $Path $Description
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Get-WindowsSdkRoot {
    $candidates = @()
    if ($env:WindowsSdkDir) {
        $candidates += $env:WindowsSdkDir
    }
    try {
        $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('SOFTWARE\Microsoft\Windows Kits\Installed Roots')
        if ($null -ne $key) {
            $root = $key.GetValue('KitsRoot10')
            if ($root) { $candidates += [string] $root }
            $key.Dispose()
        }
    }
    catch {
        # Environment discovery is also supported; registry access may be restricted.
    }

    foreach ($candidate in ($candidates | Select-Object -Unique)) {
        if (-not (Test-Path -LiteralPath $candidate -PathType Container)) { continue }
        $versions = Get-ChildItem -LiteralPath (Join-Path $candidate 'Lib') -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending
        foreach ($version in $versions) {
            $sdkLib = Join-Path $version.FullName 'um\x64\kernel32.lib'
            $sdkInclude = Join-Path $candidate "Include\$($version.Name)\um\Windows.h"
            $ucrtLib = Join-Path $version.FullName 'ucrt\x64\ucrt.lib'
            if ((Test-Path -LiteralPath $sdkLib -PathType Leaf) -and
                (Test-Path -LiteralPath $sdkInclude -PathType Leaf) -and
                (Test-Path -LiteralPath $ucrtLib -PathType Leaf)) {
                return [pscustomobject]@{ Root = $candidate; Version = $version.Name }
            }
        }
    }
    return $null
}

function Assert-Pe32PlusAmd64([string] $Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    $reader = New-Object -TypeName System.IO.BinaryReader -ArgumentList $stream
    try {
        if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) {
            Stop-Packaging 'the built executable does not have a valid DOS/PE header'
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadInt32()
        if ($peOffset -lt 64 -or [long] $peOffset -gt ($stream.Length - 24)) {
            Stop-Packaging 'the built executable has an invalid PE header offset'
        }
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            Stop-Packaging 'the built executable is missing the PE signature'
        }
        $machine = $reader.ReadUInt16()
        $stream.Position = $peOffset + 20
        $optionalHeaderSize = $reader.ReadUInt16()
        if ($optionalHeaderSize -lt 70 -or ([long] $peOffset + 24 + $optionalHeaderSize) -gt $stream.Length) {
            Stop-Packaging 'the built executable has a truncated PE32+ optional header; the subsystem field is not safely readable'
        }
        $stream.Position = $peOffset + 24
        $optionalMagic = $reader.ReadUInt16()
        if ($machine -ne 0x8664 -or $optionalMagic -ne 0x020B) {
            Stop-Packaging ('expected PE32+ AMD64 (machine 0x8664, optional magic 0x020B); found machine 0x{0:X4}, optional magic 0x{1:X4}' -f $machine, $optionalMagic)
        }
        $stream.Position = $peOffset + 24 + 68
        $subsystem = $reader.ReadUInt16()
        if ($subsystem -ne 2) {
            Stop-Packaging ('expected IMAGE_SUBSYSTEM_WINDOWS_GUI (2); found optional-header subsystem {0} (0x{0:X4})' -f $subsystem)
        }
    }
    finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Get-Sha256([string] $Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

try {
    if ($env:OS -ne 'Windows_NT') {
        Stop-Packaging 'this packaging flow requires Windows x64 and an MSVC toolchain; run it on Windows, not PowerShell on another operating system'
    }

    Require-File $DesktopManifest 'desktop Cargo manifest'
    Require-File (Join-Path $RepositoryRoot 'Cargo.toml') 'workspace Cargo manifest'
    Require-File $IconSource 'shared source icon'
    $licenseSources = [ordered]@{
        'Licenses\CourierPrime\OFL.txt' = Join-Path $RepositoryRoot 'fonts\CourierPrime\LICENSE\OFL.txt'
        'Licenses\CourierPrime\OFL-FAQ.txt' = Join-Path $RepositoryRoot 'fonts\CourierPrime\LICENSE\OFL-FAQ.txt'
        'Licenses\CourierPrime\Attribution.txt' = Join-Path $RepositoryRoot 'fonts\CourierPrime\Read me.txt'
        'Licenses\MaterialSymbols\LICENSE' = Join-Path $RepositoryRoot 'fonts\MaterialSymbols\LICENSE'
        'Licenses\MaterialSymbols\Asset-Attribution.md' = Join-Path $RepositoryRoot 'fonts\MaterialSymbols\README.md'
    }
    foreach ($source in $licenseSources.Values) {
        Require-File $source 'font license or attribution record'
    }

    $manifestText = [System.IO.File]::ReadAllText($DesktopManifest)
    $packageMatch = [regex]::Match($manifestText, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)')
    if (-not $packageMatch.Success) { Stop-Packaging 'could not find the [package] section in apps/scriptcraft/Cargo.toml' }
    $versionMatch = [regex]::Match($packageMatch.Groups[1].Value, '(?m)^\s*version\s*=\s*"([^"\r\n]+)"\s*(?:#.*)?$')
    if (-not $versionMatch.Success -or $versionMatch.Groups[1].Value -notmatch '^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?(\+[A-Za-z0-9.-]+)?$') {
        Stop-Packaging 'could not read a supported semantic version from the desktop [package] section'
    }
    $version = $versionMatch.Groups[1].Value
    $stageName = "ScriptCraft-$version-windows-x64"
    $stagePath = Join-Path $PackageRoot $stageName
    $zipName = "$stageName.zip"
    $zipPath = Join-Path $PackageRoot $zipName
    $checksumPath = "$zipPath.sha256"
    $tempZipPath = Join-Path $PackageRoot "$stageName.tmp.zip"
    $tempChecksumPath = "$stageName.tmp.sha256"
    $tempChecksumFullPath = Join-Path $PackageRoot $tempChecksumPath

    $cargoPath = Get-CommandPath 'cargo' 'install Rustup/Cargo or add the selected Rust toolchain binaries to PATH'
    $rustupPath = Get-CommandPath 'rustup' 'install Rustup and add its bin directory to PATH'
    $null = Get-CommandPath 'rustc' 'install/select a Rust toolchain with rustc available on PATH'
    $installedTargets = & $rustupPath target list --installed 2>&1
    if ($LASTEXITCODE -ne 0) {
        Stop-Packaging "Rustup could not list installed targets; repair/select the Rustup toolchain. Output: $($installedTargets -join ' ')"
    }
    if ($installedTargets -notcontains $TargetTriple) {
        Stop-Packaging "Rust target $TargetTriple is not installed; install it separately with: rustup target add $TargetTriple"
    }

    $linkPath = Get-CommandPath 'link.exe' 'install/repair Visual Studio Build Tools with Desktop development with C++, then run from an x64 Native Tools developer prompt'
    $linkHelp = (& $linkPath /? 2>&1 | Out-String)
    if ($linkHelp -notmatch 'Microsoft\s+\(R\)\s+Incremental Linker') {
        Stop-Packaging "link.exe on PATH does not identify as the Microsoft linker ($linkPath); use a Visual Studio x64 Native Tools developer prompt"
    }
    $windowsSdk = Get-WindowsSdkRoot
    if ($null -eq $windowsSdk) {
        Stop-Packaging 'a usable Windows 10/11 SDK (x64 kernel32.lib, ucrt.lib, and Windows.h) was not found; install/repair the Windows SDK and rerun from a Visual Studio x64 Native Tools prompt'
    }

    Assert-NotReparsePoint (Join-Path $RepositoryRoot 'target') 'target directory'
    Assert-NotReparsePoint $PackageRoot 'Windows package output directory'
    if (-not (Test-Path -LiteralPath $PackageRoot -PathType Container)) {
        New-Item -ItemType Directory -Path $PackageRoot -Force | Out-Null
    }
    foreach ($ownedPath in @(
        @{ Path = $stagePath; Description = 'current-version staging directory' },
        @{ Path = $zipPath; Description = 'current-version ZIP' },
        @{ Path = $checksumPath; Description = 'current-version checksum' },
        @{ Path = $tempZipPath; Description = 'current-version temporary ZIP' },
        @{ Path = $tempChecksumFullPath; Description = 'current-version temporary checksum' }
    )) {
        Assert-NotReparsePoint $ownedPath.Path $ownedPath.Description
    }

    Write-Host "Building ScriptCraft $version for $TargetTriple..."
    Write-Host "Using MSVC linker: $linkPath"
    Write-Host "Using Windows SDK: $($windowsSdk.Root) ($($windowsSdk.Version))"
    & $cargoPath build --manifest-path (Join-Path $RepositoryRoot 'Cargo.toml') --package scriptcraft --release --target $TargetTriple
    if ($LASTEXITCODE -ne 0) {
        Stop-Packaging "Cargo release build failed with exit code $LASTEXITCODE; resolve the first compiler/linker error and rerun"
    }

    $builtExecutable = Join-Path $RepositoryRoot "target\$TargetTriple\release\scriptcraft.exe"
    Require-File $builtExecutable 'release executable'
    Assert-Pe32PlusAmd64 $builtExecutable

    Remove-OwnedPath $stagePath 'current-version staging directory'
    Remove-OwnedPath $tempZipPath 'current-version temporary ZIP'
    Remove-OwnedPath $tempChecksumFullPath 'current-version temporary checksum'
    $null = New-Item -ItemType Directory -Path (Join-Path $stagePath 'Licenses\CourierPrime') -Force
    $null = New-Item -ItemType Directory -Path (Join-Path $stagePath 'Licenses\MaterialSymbols') -Force
    Copy-Item -LiteralPath $builtExecutable -Destination (Join-Path $stagePath 'ScriptCraft.exe')
    Copy-Item -LiteralPath $IconSource -Destination (Join-Path $stagePath 'AppIcon.png')
    if ((Get-Sha256 $IconSource) -cne (Get-Sha256 (Join-Path $stagePath 'AppIcon.png'))) {
        Stop-Packaging 'staged AppIcon.png is not byte-for-byte identical to packaging/resources/AppIcon.png'
    }
    $recipientReadme = @"
ScriptCraft $version for Windows x64 (portable, unsigned)

Extract the complete ScriptCraft-$version-windows-x64 folder to a writable location and run ScriptCraft.exe from that folder. Keep the Licenses folder with the application.

Run ScriptCraft.exe with no document path for an ordinary blank launch. To open a document directly in a new instance, start ScriptCraft.exe with exactly one quoted Windows path, for example: ScriptCraft.exe "C:\Users\Name\Documents\My Screenplay.fdx". Paths containing spaces or non-ASCII characters are passed as one native Windows path; Windows supplies the selected path to the application.

To choose an association yourself, use the FDX file's Open With or Choose another app option, then browse to the extracted ScriptCraft.exe. Labels vary by Windows version. Choose an Always or default option only if you deliberately want Windows to use ScriptCraft for that file type, or make that choice in Windows Default Apps settings. Windows controls and may record this user-created association; the package does not create it or choose a default.

Each Explorer open starts an independent new instance and opens the selected document directly there; already running instances remain open and are not changed. Two positional paths, or a positional path combined with custom input, are invalid and produce a bounded native startup error. A document-open failure produces bounded native feedback in the new instance and leaves it untitled. Cancel on a version decision leaves the new instance untitled without treating Cancel as an open failure.

Dragging one file onto ScriptCraft.exe or a suitable shortcut uses the same one-path launch convention. Taskbar drops and drops into an open application window are not supported by this guidance.

The release application uses the Windows GUI subsystem and does not open a companion console window. Debug builds retain the console subsystem for diagnostics.

This package is unsigned. Windows may display an unknown-publisher or SmartScreen warning; no signing or Microsoft trust approval is claimed. Run it only if you trust its source.

This is a portable package with no installer. The application embeds Courier Prime and Material Symbols font data. It does not install fonts, modify the registry, or create file associations.
"@
    [System.IO.File]::WriteAllText((Join-Path $stagePath 'README.txt'), $recipientReadme.Trim() + "`r`n", [System.Text.Encoding]::ASCII)
    foreach ($relativePath in $licenseSources.Keys) {
        $destination = Join-Path $stagePath $relativePath
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force
        Copy-Item -LiteralPath $licenseSources[$relativePath] -Destination $destination
    }

    $expectedFiles = @(
        "$stageName/ScriptCraft.exe",
        "$stageName/AppIcon.png",
        "$stageName/README.txt",
        "$stageName/Licenses/CourierPrime/OFL.txt",
        "$stageName/Licenses/CourierPrime/OFL-FAQ.txt",
        "$stageName/Licenses/CourierPrime/Attribution.txt",
        "$stageName/Licenses/MaterialSymbols/LICENSE",
        "$stageName/Licenses/MaterialSymbols/Asset-Attribution.md"
    )
    $stagedFiles = @(Get-ChildItem -LiteralPath $stagePath -File -Recurse)
    $stagedRelativeFiles = @($stagedFiles | ForEach-Object { $_.FullName.Substring($stagePath.Length + 1).Replace('\', '/') } | Sort-Object)
    $expectedRelativeFiles = @($expectedFiles | ForEach-Object { $_.Substring($stageName.Length + 1) } | Sort-Object)
    if (($stagedRelativeFiles -join "`n") -cne ($expectedRelativeFiles -join "`n")) {
        Stop-Packaging 'staging directory contains missing or unexpected files'
    }

    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::Open($tempZipPath, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in $stagedFiles) {
            $relativeFile = $file.FullName.Substring($stagePath.Length + 1).Replace('\', '/')
            $entryName = "$stageName/$relativeFile"
            $entry = $archive.CreateEntry($entryName, [System.IO.Compression.CompressionLevel]::Optimal)
            $inputStream = [System.IO.File]::OpenRead($file.FullName)
            try {
                $outputStream = $entry.Open()
                try { $inputStream.CopyTo($outputStream) }
                finally { $outputStream.Dispose() }
            }
            finally { $inputStream.Dispose() }
        }
    }
    finally { $archive.Dispose() }

    $archive = [System.IO.Compression.ZipFile]::OpenRead($tempZipPath)
    try {
        $archiveEntries = @($archive.Entries | ForEach-Object { $_.FullName } | Sort-Object)
        if (($archiveEntries -join "`n") -cne (@($expectedFiles | Sort-Object) -join "`n")) {
            Stop-Packaging 'ZIP entries do not exactly match the expected single-folder portable package layout'
        }
        $topLevelFolders = @($archiveEntries | ForEach-Object { ($_ -split '/')[0] } | Select-Object -Unique)
        $unexpectedArchiveEntries = @($archiveEntries | Where-Object { -not $_.StartsWith("$stageName/", [System.StringComparison]::Ordinal) })
        if ($topLevelFolders.Count -ne 1 -or $unexpectedArchiveEntries.Count -ne 0) {
            Stop-Packaging 'ZIP does not contain exactly one top-level portable folder'
        }
    }
    finally { $archive.Dispose() }

    $zipHash = Get-Sha256 $tempZipPath
    $utf8WithoutBom = New-Object -TypeName System.Text.UTF8Encoding -ArgumentList $false
    [System.IO.File]::WriteAllText($tempChecksumFullPath, "$zipHash  $zipName`n", $utf8WithoutBom)
    $checksumText = [System.IO.File]::ReadAllText($tempChecksumFullPath).TrimEnd("`r", "`n")
    if ($checksumText -cne "$((Get-Sha256 $tempZipPath))  $zipName") {
        Stop-Packaging 'temporary ZIP checksum did not match a fresh SHA-256 calculation'
    }

    Remove-OwnedPath $zipPath 'current-version ZIP'
    Remove-OwnedPath $checksumPath 'current-version checksum'
    Move-Item -LiteralPath $tempZipPath -Destination $zipPath
    Move-Item -LiteralPath $tempChecksumFullPath -Destination $checksumPath
    $finalHash = Get-Sha256 $zipPath
    $finalChecksum = [System.IO.File]::ReadAllText($checksumPath).TrimEnd("`r", "`n")
    if ($finalChecksum -cne "$finalHash  $zipName") {
        Stop-Packaging 'final ZIP SHA-256 does not match its sidecar'
    }

    Write-Host "Created portable folder: $stagePath"
    Write-Host "Created ZIP: $zipPath"
    Write-Host "Created SHA-256 sidecar: $checksumPath"
    Write-Host "Verified PE32+ AMD64 Windows GUI-subsystem executable, exact ZIP entries, and SHA-256: $finalHash"
    Write-Host 'Distribution is unsigned and portable; no installer, registry changes, signing, SmartScreen approval, or GUI acceptance is claimed.'
}
catch {
    Write-Error $_
    exit 1
}
