#!/usr/bin/env bash
set -euo pipefail

CRATE_DIR="runtime-rs"
PROFILE=${PROFILE:-release}

echo "[runtime] Building Rust runtime crate in $CRATE_DIR (profile=$PROFILE)" >&2
cd "$CRATE_DIR"
cargo build --${PROFILE}

# locate staticlib
OUT_DIR="target/${PROFILE}"
LIB=$(ls "$OUT_DIR"/libruntime_rs.* 2>/dev/null | head -n 1 || true)
if [ -z "$LIB" ]; then
  echo "Could not find libruntime_rs in $OUT_DIR" >&2
  exit 1
fi

REALPATH=$(realpath "$LIB")
echo "[runtime] built: $REALPATH" >&2

echo "$REALPATH"
