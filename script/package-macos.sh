#!/bin/bash
# Build an Ingut.app bundle from a compiled ingut binary.
# Usage: script/package-macos.sh <ingut-binary> <version> <output.app>

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="${1:?path to the ingut binary}"
VERSION="${2:?version, without a leading v}"
OUTPUT="${3:?path to the .app to write}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ICONSET="$WORK/AppIcon.iconset"
mkdir -p "$ICONSET"
render_icon() {
  local pixels="$1"
  local name="$2"
  sips -z "$pixels" "$pixels" "$ROOT/assets/icon.png" --out "$ICONSET/$name" >/dev/null
}
render_icon 16 icon_16x16.png
render_icon 32 icon_16x16@2x.png
render_icon 32 icon_32x32.png
render_icon 64 icon_32x32@2x.png
render_icon 128 icon_128x128.png
render_icon 256 icon_128x128@2x.png
render_icon 256 icon_256x256.png
render_icon 512 icon_256x256@2x.png
render_icon 512 icon_512x512.png
render_icon 1024 icon_512x512@2x.png
iconutil -c icns "$ICONSET" -o "$WORK/AppIcon.icns"

APP="$WORK/Ingut.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINARY" "$APP/Contents/MacOS/ingut"
chmod +x "$APP/Contents/MacOS/ingut"
cp "$WORK/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
sed "s/VERSION/${VERSION}/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"

rm -rf "$OUTPUT"
mkdir -p "$(dirname "$OUTPUT")"
ditto "$APP" "$OUTPUT"
