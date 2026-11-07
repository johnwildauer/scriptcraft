#!/bin/bash
set -Eeuo pipefail

readonly TARGET_TRIPLE="aarch64-apple-darwin"
readonly BUNDLE_NAME="ScriptCraft.app"
readonly BUNDLE_IDENTIFIER="com.scriptcraft.app"
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd -P)"
readonly PACKAGE_ROOT="$REPO_ROOT/target/macos-package"
readonly DESKTOP_MANIFEST="$REPO_ROOT/apps/scriptcraft/Cargo.toml"
readonly ICON_MASTER="$SCRIPT_DIR/AppIcon.png"
readonly PLIST_TEMPLATE="$SCRIPT_DIR/Info.plist"
readonly ICONSET_DIR="$PACKAGE_ROOT/AppIcon.iconset"
readonly APP_PATH="$PACKAGE_ROOT/$BUNDLE_NAME"
readonly DMG_STAGE="$PACKAGE_ROOT/dmg-stage"
readonly MOUNT_PATH="$PACKAGE_ROOT/dmg-mount"
readonly TEMP_DMG="$PACKAGE_ROOT/ScriptCraft-package.tmp.dmg"
readonly TEMP_CHECKSUM="$PACKAGE_ROOT/ScriptCraft-package.tmp.sha256"
readonly LINKAGE_REPORT="$PACKAGE_ROOT/linkage.txt"
readonly SIGNATURE_REPORT="$PACKAGE_ROOT/signature-details.txt"
readonly EXECUTABLE_RELATIVE="Contents/MacOS/scriptcraft"

cd "$REPO_ROOT"

fail() {
    printf 'package.sh: %s\n' "$*" >&2
    exit 1
}

require_file() {
    [[ -f "$1" ]] || fail "required source file is missing: ${1#"$REPO_ROOT"/}"
}

require_tool() {
    command -v "$1" >/dev/null 2>&1 || fail "required tool '$1' is unavailable; install or select the macOS/Rust prerequisite, then rerun this script"
}

check_runtime_prerequisites() {
    local tool
    for tool in cargo rustup rustc sips iconutil plutil codesign hdiutil lipo otool shasum awk grep sed cp mkdir rm mv ln chmod ditto readlink; do
        require_tool "$tool"
    done
    command -v xcode-select >/dev/null 2>&1 || fail "Xcode Command Line Tools are required; install/select them before packaging"
    xcode-select -p >/dev/null 2>&1 || fail "Xcode Command Line Tools are not selected; run xcode-select --install or select the installed tools"
    [[ -x /usr/libexec/PlistBuddy ]] || fail "required macOS tool /usr/libexec/PlistBuddy is unavailable"
    rustup target list --installed | grep -Fxq "$TARGET_TRIPLE" || fail "Rust target $TARGET_TRIPLE is not installed; add it with: rustup target add $TARGET_TRIPLE"
}

read_package_version() {
    local version
    version="$(awk '
        $0 == "[package]" { in_package = 1; next }
        /^\[/ { in_package = 0 }
        in_package && $1 == "version" && $2 == "=" {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' "$DESKTOP_MANIFEST")"
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?(\+[A-Za-z0-9.-]+)?$ ]] || fail "could not read a supported semantic version from apps/scriptcraft/Cargo.toml"
    printf '%s' "$version"
}

check_icon_dimensions() {
    local icon_info width height
    icon_info="$(sips -g pixelWidth -g pixelHeight "$ICON_MASTER" 2>/dev/null)" || fail "sips could not read the supplied PNG master icon"
    width="$(printf '%s\n' "$icon_info" | awk '/pixelWidth:/ { print $2; exit }')"
    height="$(printf '%s\n' "$icon_info" | awk '/pixelHeight:/ { print $2; exit }')"
    [[ "$width" == 1024 && "$height" == 1024 ]] || fail "master icon must be exactly 1024x1024 pixels; sips reported ${width:-unknown}x${height:-unknown}"
}

check_sources() {
    local source
    for source in \
        "$DESKTOP_MANIFEST" "$ICON_MASTER" "$PLIST_TEMPLATE" \
        "$REPO_ROOT/Cargo.toml" \
        "$REPO_ROOT/fonts/CourierPrime/LICENSE/OFL.txt" \
        "$REPO_ROOT/fonts/CourierPrime/LICENSE/OFL-FAQ.txt" \
        "$REPO_ROOT/fonts/CourierPrime/Read me.txt" \
        "$REPO_ROOT/fonts/MaterialSymbols/LICENSE" \
        "$REPO_ROOT/fonts/MaterialSymbols/README.md"; do
        require_file "$source"
    done
}

check_package_root() {
    [[ ! -L "$REPO_ROOT/target" ]] || fail "target/ is a symbolic link; refusing generated-output cleanup outside the repository"
    [[ ! -L "$PACKAGE_ROOT" ]] || fail "target/macos-package is a symbolic link; refusing generated-output cleanup outside the package root"
    mkdir -p "$PACKAGE_ROOT"
    [[ ! -L "$MOUNT_PATH" ]] || fail "DMG mountpoint is a symbolic link; refusing to mount outside the package root"
    if [[ -e "$MOUNT_PATH" ]]; then
        [[ -d "$MOUNT_PATH" ]] || fail "expected DMG mountpoint path is occupied by a non-directory: target/macos-package/dmg-mount"
        [[ -z "$(/usr/bin/find "$MOUNT_PATH" -mindepth 1 -maxdepth 1 -print -quit)" ]] || fail "DMG mountpoint is not empty; preserve its contents and resolve it before packaging"
    fi
}

generate_icon() {
    mkdir -p "$ICONSET_DIR"
    sips -z 16 16 "$ICON_MASTER" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null
    sips -z 32 32 "$ICON_MASTER" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null
    sips -z 32 32 "$ICON_MASTER" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null
    sips -z 64 64 "$ICON_MASTER" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null
    sips -z 128 128 "$ICON_MASTER" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null
    sips -z 256 256 "$ICON_MASTER" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
    sips -z 256 256 "$ICON_MASTER" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null
    sips -z 512 512 "$ICON_MASTER" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
    sips -z 512 512 "$ICON_MASTER" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null
    sips -z 1024 1024 "$ICON_MASTER" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null
    iconutil -c icns "$ICONSET_DIR" -o "$APP_PATH/Contents/Resources/AppIcon.icns"
}

check_bundle_layout() {
    [[ -x "$APP_PATH/$EXECUTABLE_RELATIVE" ]] || fail "assembled app executable is missing or not executable"
    [[ -s "$APP_PATH/Contents/Resources/AppIcon.icns" ]] || fail "assembled app icon resource is missing"
    [[ -s "$APP_PATH/Contents/Resources/Licenses/CourierPrime/OFL.txt" ]] || fail "Courier Prime license record is missing"
    [[ -s "$APP_PATH/Contents/Resources/Licenses/CourierPrime/Attribution.txt" ]] || fail "Courier Prime attribution is missing"
    [[ -s "$APP_PATH/Contents/Resources/Licenses/MaterialSymbols/LICENSE" ]] || fail "Material Symbols license record is missing"
    [[ -s "$APP_PATH/Contents/Resources/Licenses/MaterialSymbols/Asset-Attribution.md" ]] || fail "Material Symbols attribution is missing"
    plutil -lint "$APP_PATH/Contents/Info.plist" >/dev/null || fail "generated Info.plist is invalid"
    /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP_PATH/Contents/Info.plist" 2>/dev/null | grep -Fxq "$BUNDLE_IDENTIFIER" || fail "Info.plist bundle identifier is incorrect"
    /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_PATH/Contents/Info.plist" 2>/dev/null | grep -Fxq "$APP_VERSION" || fail "Info.plist marketing version is incorrect"
}

check_linkage() {
    local unexpected
    otool -L "$APP_PATH/$EXECUTABLE_RELATIVE" > "$LINKAGE_REPORT" || fail "could not inspect executable dynamic-library linkage"
    unexpected="$(awk 'NR > 1 {
        library = $1
        if (library ~ /^\/System\/Library\// || library ~ /^\/usr\/lib\//) next
        print library
    }' "$LINKAGE_REPORT")"
    if [[ -n "$unexpected" ]]; then
        printf '%s\n' "$unexpected" >&2
        fail "executable links a non-system or unresolved dynamic library; inspect $LINKAGE_REPORT and resolve it before distribution"
    fi
}

verify_signature_and_architecture() {
    codesign --verify --strict --verbose=2 "$APP_PATH" || fail "ad-hoc app signature verification failed"
    codesign -dv --verbose=4 "$APP_PATH" > "$SIGNATURE_REPORT" 2>&1 || fail "could not inspect the app signature"
    grep -Fq 'Signature=adhoc' "$SIGNATURE_REPORT" || fail "assembled app does not report an ad-hoc signature"
    grep -Fq "Identifier=$BUNDLE_IDENTIFIER" "$SIGNATURE_REPORT" || fail "signature identifier does not match the bundle identifier"
    lipo "$APP_PATH/$EXECUTABLE_RELATIVE" -verify_arch arm64 >/dev/null || fail "app executable does not contain the required arm64 architecture"
    check_linkage
}

cleanup_mounted_image() {
    if [[ "${IMAGE_MOUNTED:-0}" == 1 ]]; then
        hdiutil detach "$MOUNT_PATH" >/dev/null 2>&1 || true
    fi
}

APP_VERSION=""
IMAGE_MOUNTED=0
trap cleanup_mounted_image EXIT

check_sources
check_runtime_prerequisites
check_icon_dimensions
APP_VERSION="$(read_package_version)"
check_package_root

printf 'Building ScriptCraft %s for %s...\n' "$APP_VERSION" "$TARGET_TRIPLE"
cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --package scriptcraft --release --target "$TARGET_TRIPLE"

readonly FINAL_DMG="$PACKAGE_ROOT/ScriptCraft-$APP_VERSION-arm64.dmg"
readonly FINAL_CHECKSUM="$FINAL_DMG.sha256"
readonly BUILT_EXECUTABLE="$REPO_ROOT/target/$TARGET_TRIPLE/release/scriptcraft"
[[ -x "$BUILT_EXECUTABLE" ]] || fail "release build did not produce the expected executable: target/$TARGET_TRIPLE/release/scriptcraft"

# Remove only fixed, package-owned intermediate paths. Existing outputs for other
# manifest versions and all unrelated target/ contents are left untouched.
rm -rf "$APP_PATH" "$ICONSET_DIR" "$DMG_STAGE" "$TEMP_DMG" "$TEMP_CHECKSUM" "$LINKAGE_REPORT" "$SIGNATURE_REPORT"
mkdir -p "$APP_PATH/Contents/MacOS" "$APP_PATH/Contents/Resources/Licenses/CourierPrime" "$APP_PATH/Contents/Resources/Licenses/MaterialSymbols"
cp "$BUILT_EXECUTABLE" "$APP_PATH/$EXECUTABLE_RELATIVE"
chmod 755 "$APP_PATH/$EXECUTABLE_RELATIVE"

generate_icon
sed "s/@APP_VERSION@/$APP_VERSION/g" "$PLIST_TEMPLATE" > "$APP_PATH/Contents/Info.plist"
cp "$REPO_ROOT/fonts/CourierPrime/LICENSE/OFL.txt" "$APP_PATH/Contents/Resources/Licenses/CourierPrime/OFL.txt"
cp "$REPO_ROOT/fonts/CourierPrime/LICENSE/OFL-FAQ.txt" "$APP_PATH/Contents/Resources/Licenses/CourierPrime/OFL-FAQ.txt"
cp "$REPO_ROOT/fonts/CourierPrime/Read me.txt" "$APP_PATH/Contents/Resources/Licenses/CourierPrime/Attribution.txt"
cp "$REPO_ROOT/fonts/MaterialSymbols/LICENSE" "$APP_PATH/Contents/Resources/Licenses/MaterialSymbols/LICENSE"
cp "$REPO_ROOT/fonts/MaterialSymbols/README.md" "$APP_PATH/Contents/Resources/Licenses/MaterialSymbols/Asset-Attribution.md"

check_bundle_layout
codesign --force --sign - --timestamp=none "$APP_PATH"
verify_signature_and_architecture

mkdir -p "$DMG_STAGE"
ditto "$APP_PATH" "$DMG_STAGE/$BUNDLE_NAME"
ln -s /Applications "$DMG_STAGE/Applications"
hdiutil create -volname "ScriptCraft $APP_VERSION" -srcfolder "$DMG_STAGE" -format UDZO -imagekey zlib-level=9 -ov "$TEMP_DMG"
hdiutil verify "$TEMP_DMG"

mkdir -p "$MOUNT_PATH"
hdiutil attach -readonly -nobrowse -mountpoint "$MOUNT_PATH" "$TEMP_DMG" >/dev/null
IMAGE_MOUNTED=1
[[ -d "$MOUNT_PATH/$BUNDLE_NAME/Contents" && -x "$MOUNT_PATH/$BUNDLE_NAME/$EXECUTABLE_RELATIVE" ]] || fail "mounted DMG does not contain the expected app bundle"
[[ -s "$MOUNT_PATH/$BUNDLE_NAME/Contents/Info.plist" && -s "$MOUNT_PATH/$BUNDLE_NAME/Contents/Resources/AppIcon.icns" ]] || fail "mounted DMG app is missing its property list or icon resource"
[[ -s "$MOUNT_PATH/$BUNDLE_NAME/Contents/Resources/Licenses/CourierPrime/OFL.txt" ]] || fail "mounted DMG app is missing the Courier Prime license"
[[ -s "$MOUNT_PATH/$BUNDLE_NAME/Contents/Resources/Licenses/MaterialSymbols/LICENSE" ]] || fail "mounted DMG app is missing the Material Symbols license"
[[ -L "$MOUNT_PATH/Applications" ]] || fail "mounted DMG is missing its Applications symbolic link"
[[ "$(readlink "$MOUNT_PATH/Applications")" == /Applications ]] || fail "DMG Applications symbolic link does not target /Applications"
codesign --verify --strict "$MOUNT_PATH/$BUNDLE_NAME" || fail "signature verification failed for the app copied into the DMG"
hdiutil detach "$MOUNT_PATH" >/dev/null
IMAGE_MOUNTED=0
rmdir "$MOUNT_PATH"

mv -f "$TEMP_DMG" "$FINAL_DMG"
(cd "$PACKAGE_ROOT" && shasum -a 256 "$(basename "$FINAL_DMG")" > "$TEMP_CHECKSUM")
mv -f "$TEMP_CHECKSUM" "$FINAL_CHECKSUM"
printf 'Created app: %s\nCreated DMG: %s\nCreated SHA-256: %s\n' \
    "$APP_PATH" "$FINAL_DMG" "$FINAL_CHECKSUM"
printf 'Distribution is ad-hoc signed and unnotarized. No Gatekeeper assessment was performed.\n'
