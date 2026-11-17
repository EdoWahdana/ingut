#!/bin/bash
# Build the release binary, wrap it in Ingut.app, and install that app.
# Usage: ./build-macos.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ -z "$VERSION" ]]; then
  echo "build-macos: version is missing from Cargo.toml" >&2
  exit 1
fi

cargo build --release

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
"$ROOT/script/package-macos.sh" "$ROOT/target/release/ingut" "$VERSION" "$STAGE/Ingut.app"

if pgrep -x ingut >/dev/null 2>&1; then
  killall ingut
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    pgrep -x ingut >/dev/null 2>&1 || break
    sleep 0.2
  done
fi

rm -rf /Applications/Ingut.app
ditto "$STAGE/Ingut.app" /Applications/Ingut.app
open /Applications/Ingut.app
echo "Installed Ingut $VERSION to /Applications/Ingut.app"
