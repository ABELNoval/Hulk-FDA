#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 <input.bc> [out_exec] [runtime_lib]"
  exit 2
fi

INPUT_BC="$1"
OUT_EXEC=${2:-a.out}
RUNTIME_LIB=${3:-}

# tools: opt, llc, clang (with auto-detection for versioned LLVM 16)
if command -v opt-16 &> /dev/null; then
  OPT=${OPT:-opt-16}
else
  OPT=${OPT:-opt}
fi

if command -v llc-16 &> /dev/null; then
  LLC=${LLC:-llc-16}
else
  LLC=${LLC:-llc}
fi

if command -v clang-16 &> /dev/null; then
  CLANG=${CLANG:-clang-16}
else
  CLANG=${CLANG:-clang}
fi

TMP_DIR=$(mktemp -d)
OBJ="$TMP_DIR/out.o"

echo "[build] optimizing with opt -> ${INPUT_BC} -> ${TMP_DIR}/opt.bc"
${OPT} -O2 "$INPUT_BC" -o "$TMP_DIR/opt.bc"

echo "[build] compiling to object with llc -> ${OBJ}"
${LLC} -relocation-model=pic -filetype=obj "$TMP_DIR/opt.bc" -o "$OBJ"

if [ -z "$RUNTIME_LIB" ]; then
  # try to build runtime-rs if available
  if [ -d "runtime-rs" ]; then
    echo "[build] building Rust runtime (runtime-rs)"
    RUNTIME_LIB_PATH=$(scripts/build_runtime_rust.sh release)
    RUNTIME_LIB=${RUNTIME_LIB_PATH}
  fi
fi

echo "[build] linking with ${CLANG} -> ${OUT_EXEC}"
if [ -n "$RUNTIME_LIB" ]; then
  ${CLANG} -fPIE "$OBJ" "$RUNTIME_LIB" -o "$OUT_EXEC"
else
  ${CLANG} -fPIE "$OBJ" -o "$OUT_EXEC"
fi

echo "[build] done: ${OUT_EXEC}"
