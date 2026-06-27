#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 <input.ll> [output.bc]"
  exit 2
fi

INPUT_LL="$1"
OUTPUT_BC=${2:-${INPUT_LL%.*}.bc}


# prefer versioned llvm-as (llvm-as-16) if available
if [ -n "${LLVM_AS-}" ]; then
  AS_CMD="$LLVM_AS"
elif command -v llvm-as-16 >/dev/null 2>&1; then
  AS_CMD=llvm-as-16
elif command -v llvm-as >/dev/null 2>&1; then
  AS_CMD=llvm-as
else
  echo "llvm-as not found; install LLVM 16 (provides llvm-as-16) or set LLVM_AS env var"
  exit 3
fi

echo "[emit] $AS_CMD $INPUT_LL -o $OUTPUT_BC"
$AS_CMD "$INPUT_LL" -o "$OUTPUT_BC"

echo "[emit] wrote $OUTPUT_BC"
