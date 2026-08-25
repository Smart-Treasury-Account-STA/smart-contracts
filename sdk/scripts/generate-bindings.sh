#!/usr/bin/env bash
# Reproducible SDK bindings generation: rebuilds WASM from current source,
# then regenerates each contract's TypeScript client from that fresh WASM.
# Run from the repo root.
#
# Only the contracts a client actually calls get bindings --
# transfer_adapter/split_adapter are deliberately excluded (see
# docs/DAPP_INTEGRATION_SPEC.md §1: "never called directly by a client").
set -euo pipefail
cd "$(dirname "$0")/../.."

echo "== Building optimized WASM =="
stellar contract build --optimize --out-dir wasm

CONTRACTS=(
  "sta_smart_account:smart_account"
  "sta_policy_engine:policy_engine"
  "sta_intent_registry:intent_registry"
  "sta_recovery_manager:recovery_manager"
  "sta_account_factory:account_factory"
)

for entry in "${CONTRACTS[@]}"; do
  wasm_name="${entry%%:*}"
  out_name="${entry##*:}"
  echo "== Generating TypeScript bindings for $out_name =="
  stellar contract bindings typescript \
    --wasm "wasm/${wasm_name}.wasm" \
    --output-dir "sdk/generated/${out_name}" \
    --overwrite
done

echo "== Done. Run 'npm install' in sdk/ to build the generated packages. =="
