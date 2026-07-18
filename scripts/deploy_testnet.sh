#!/usr/bin/env bash
# Deploys and wires the V1 Smart Treasury Account workspace to Stellar
# testnet using the stellar CLI. Reproduces the deployment recorded in
# docs/TESTNET_DEPLOYMENT.md.
#
# Requirements: `stellar` CLI configured with a funded testnet identity
# (default: `sta-testnet-deployer`; override with DEPLOYER below), and
# `stellar network` already has a `testnet` entry (`stellar network ls`).
#
# What this script does NOT do: it does not exercise any entrypoint gated
# by the treasury's own signer/policy authorization (`execute_transfer_payment`,
# `execute_split_payment`, `create_scheduled_payment`, `ExecutionEntryPoint::execute`).
# Those require an off-chain client that can construct OpenZeppelin's
# `AuthPayload` authorization for the composed `SmartAccount` custom account
# — exactly the wallet/SDK/relayer integration layer that `docs/V1_SCOPE.md`
# names as not yet included in V1. Everything wired here uses the plain
# owner/admin authorization (`Address::require_auth()` on a regular Stellar
# account), which the CLI signs automatically. See the "Known limitation"
# section of `docs/TESTNET_DEPLOYMENT.md` for detail and for how the
# signer-gated flows are proven instead (105 local integration tests using
# `mock_all_auths()`, plus `webauthn_verifier`'s real-cryptography fixtures).

set -euo pipefail
cd "$(dirname "$0")/.."

NETWORK="${NETWORK:-testnet}"
DEPLOYER="${DEPLOYER:-sta-testnet-deployer}"
GUARDIAN_ID="${GUARDIAN_ID:-sta-testnet-guardian}"
RECIPIENT_ID="${RECIPIENT_ID:-sta-testnet-recipient}"
ASSET_CODE="${ASSET_CODE:-STA}"

for id in "$DEPLOYER" "$GUARDIAN_ID" "$RECIPIENT_ID"; do
  stellar keys address "$id" >/dev/null 2>&1 || stellar keys generate "$id" --network "$NETWORK"
done

DEPLOYER_ADDR=$(stellar keys address "$DEPLOYER")
GUARDIAN_ADDR=$(stellar keys address "$GUARDIAN_ID")
RECIPIENT_ADDR=$(stellar keys address "$RECIPIENT_ID")

echo "== Building optimized WASM =="
stellar contract build --optimize --out-dir wasm

deploy() {
  stellar contract deploy --wasm "wasm/$1.wasm" --source "$DEPLOYER" --network "$NETWORK" -- | tail -1
}

echo "== Deploying contracts =="
WEBAUTHN_VERIFIER=$(deploy sta_webauthn_verifier)
POLICY_ENGINE=$(deploy sta_policy_engine)
INTENT_REGISTRY=$(deploy sta_intent_registry)
RECOVERY_MANAGER=$(deploy sta_recovery_manager)
SMART_ACCOUNT=$(deploy sta_smart_account)
TRANSFER_ADAPTER=$(deploy sta_transfer_adapter)
SPLIT_ADAPTER=$(deploy sta_split_adapter)

echo "webauthn_verifier=$WEBAUTHN_VERIFIER"
echo "policy_engine=$POLICY_ENGINE"
echo "intent_registry=$INTENT_REGISTRY"
echo "recovery_manager=$RECOVERY_MANAGER"
echo "smart_account=$SMART_ACCOUNT"
echo "transfer_adapter=$TRANSFER_ADAPTER"
echo "split_adapter=$SPLIT_ADAPTER"

echo "== Initializing policy_engine / recovery_manager =="
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" -- \
  initialize --admin "$DEPLOYER_ADDR"
stellar contract invoke --id "$RECOVERY_MANAGER" --source "$DEPLOYER" --network "$NETWORK" -- \
  initialize --admin "$DEPLOYER_ADDR" --guardian_threshold 1

echo "== Initializing transfer_adapter / split_adapter (smart_account pinned) =="
stellar contract invoke --id "$TRANSFER_ADAPTER" --source "$DEPLOYER" --network "$NETWORK" -- \
  initialize --admin "$DEPLOYER_ADDR" --smart_account "$SMART_ACCOUNT"
stellar contract invoke --id "$SPLIT_ADAPTER" --source "$DEPLOYER" --network "$NETWORK" -- \
  initialize --admin "$DEPLOYER_ADDR" --smart_account "$SMART_ACCOUNT"

echo "== Initializing smart_account (owner + founding Ed25519 wallet signer) =="
stellar contract invoke --id "$SMART_ACCOUNT" --source "$DEPLOYER" --network "$NETWORK" -- initialize \
  --owner "$DEPLOYER_ADDR" \
  --initial_signers "[{\"Delegated\":\"$DEPLOYER_ADDR\"}]" \
  --initial_policies '{}' \
  --policy_engine "$POLICY_ENGINE" \
  --intent_registry "$INTENT_REGISTRY" \
  --recovery_manager "$RECOVERY_MANAGER"

echo "== Wiring adapters into smart_account (owner-gated, not signer-gated) =="
stellar contract invoke --id "$SMART_ACCOUNT" --source "$DEPLOYER" --network "$NETWORK" -- \
  set_adapter --operation transfer --adapter "$TRANSFER_ADAPTER"
stellar contract invoke --id "$SMART_ACCOUNT" --source "$DEPLOYER" --network "$NETWORK" -- \
  set_adapter --operation split --adapter "$SPLIT_ADAPTER"

echo "== Registering a guardian =="
stellar contract invoke --id "$RECOVERY_MANAGER" --source "$DEPLOYER" --network "$NETWORK" -- \
  add_guardian --guardian "$GUARDIAN_ADDR"

echo "== Deploying a test SAC asset and minting to smart_account =="
TOKEN=$(stellar contract asset deploy --asset "$ASSET_CODE:$DEPLOYER" --source "$DEPLOYER" --network "$NETWORK" | tail -1)
echo "token=$TOKEN"
stellar contract invoke --id "$TOKEN" --source "$DEPLOYER" --network "$NETWORK" -- \
  mint --to "$SMART_ACCOUNT" --amount 1000000000

echo "== Configuring policy_engine rules =="
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" -- set_asset_rule \
  --asset "$TOKEN" --rule '{"enabled":true,"max_single_transfer":"10000000"}'
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" -- \
  set_recipient_allowed --recipient "$RECIPIENT_ADDR" --allowed true
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" -- \
  set_operation_allowed --operation transfer --allowed true
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" -- \
  set_operation_allowed --operation split --allowed true

echo "== Proving valid + invalid policy checks live on-chain =="
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" --send=no -- \
  validate_policy --check "{\"operation\":\"transfer\",\"asset\":\"$TOKEN\",\"destination\":\"$RECIPIENT_ADDR\",\"amount\":\"5000000\",\"expected_version\":1}"
echo "-- expect the following two calls to fail with Error(Contract, #2004) and #2005 --"
set +e
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" --send=no -- \
  validate_policy --check "{\"operation\":\"transfer\",\"asset\":\"$TOKEN\",\"destination\":\"$DEPLOYER_ADDR\",\"amount\":\"5000000\",\"expected_version\":1}"
stellar contract invoke --id "$POLICY_ENGINE" --source "$DEPLOYER" --network "$NETWORK" --send=no -- \
  validate_policy --check "{\"operation\":\"transfer\",\"asset\":\"$TOKEN\",\"destination\":\"$RECIPIENT_ADDR\",\"amount\":\"20000000\",\"expected_version\":1}"
set -e

echo "== Done. Record the printed contract IDs in docs/TESTNET_DEPLOYMENT.md. =="
echo "NOTE: intent_registry is deployed but deliberately left uninitialized here"
echo "— see docs/TESTNET_DEPLOYMENT.md for why (requires a signer-authorized"
echo "execute() dispatch through smart_account, i.e. an off-chain SDK client)."
