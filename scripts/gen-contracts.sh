#!/usr/bin/env bash
# Generate Swift types from the Rust contracts crate (single source of truth).
# Requires typeshare-cli: cargo install typeshare-cli
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/clients/macos/Sources/Token9/Generated/Contracts.swift"

if ! command -v typeshare >/dev/null 2>&1; then
  echo "typeshare not found. install it with:" >&2
  echo "  cargo install typeshare-cli" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT")"
typeshare "$ROOT/crates/token9-contracts" --lang swift --output-file "$OUT"
echo "generated: $OUT"
