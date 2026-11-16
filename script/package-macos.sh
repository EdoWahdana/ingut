#!/bin/bash
# Build Ingut.app and a drag-to-Applications disk image.
# Usage: script/package-macos.sh <ingut-binary> <version> <output.dmg>

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="${1:?path to the ingut binary}"
VERSION="${2:?version, without a leading v}"
OUTPUT="${3:?path to the .dmg to write}"

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

if [[ ! -f "$ROOT/packaging/macos/background.png" ]]; then
  LOGO="$ROOT/assets/icon.png" OUT="$ROOT/packaging/macos/background.png" \
    osascript -l JavaScript "$ROOT/packaging/macos/render_background.js"
fi

PYTHON="python3"
if ! python3 -c "import dmgbuild" >/dev/null 2>&1; then
  python3 -m venv "$WORK/venv"
  "$WORK/venv/bin/pip" install dmgbuild
  PYTHON="$WORK/venv/bin/python"
fi

mkdir -p "$(dirname "$OUTPUT")"
"$PYTHON" -m dmgbuild \
  -s "$ROOT/packaging/macos/dmg_settings.py" \
  -D "app=$APP" \
  -D "background=$ROOT/packaging/macos/background.png" \
  -D "icon=$WORK/AppIcon.icns" \
  "Ingut" \
  "$OUTPUT"
