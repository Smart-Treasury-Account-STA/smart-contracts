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
# signer-gated flows are proven instead (local integration tests using
# `mock_all_auths()`, plus `webauthn_verifier`'s real-cryptography fixtures).
#
# Adapters are bound directly at `initialize` via its `initial_adapters`
# argument — no timelock, since this is the account's first-ever
# configuration (see that function's doc comment in
# contracts/smart_account/src/lib.rs). Only *later* rebindings on an
# already-funded, already-operating treasury go through
# `propose_adapter_change`/`apply_adapter_change`'s ~1 day timelock
# (independent security review finding — see `docs/V1_SCOPE.md` §6).

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
# smart_account::initialize bootstraps intent_registry itself, passing its
# own address as admin via Soroban's invoker-shortcut -- intent_registry
# must be deployed but left UNINITIALIZED going into this call (that is
# already the case above: only webauthn_verifier/policy_engine/
# recovery_manager were initialized separately). See that function's doc
# comment in contracts/smart_account/src/lib.rs for why this needs no
# separate authorization step, unlike a prior version of this script.
stellar contract invoke --id "$SMART_ACCOUNT" --source "$DEPLOYER" --network "$NETWORK" -- initialize \
  --owner "$DEPLOYER_ADDR" \
  --initial_signers "[{\"Delegated\":\"$DEPLOYER_ADDR\"}]" \
  --initial_policies '{}' \
  --config "{\"policy_engine\":\"$POLICY_ENGINE\",\"intent_registry\":\"$INTENT_REGISTRY\",\"recovery_manager\":\"$RECOVERY_MANAGER\",\"initial_adapters\":{\"transfer\":\"$TRANSFER_ADAPTER\",\"split\":\"$SPLIT_ADAPTER\"},\"initial_executor\":\"$DEPLOYER_ADDR\"}"
# DEPLOYER is a placeholder executor here, not a real relayer identity --
# rotate it with scripts/set_intent_executor.py once a real relayer key
# exists (that script still needs smart_account's own custom-account
# authorization, same as before; only the *initial* executor is set for
# free by initialize now).

echo "== Adapters bound at initialize (no timelock for this first-ever binding; see initialize's doc comment) =="

echo "== Registering a guardian =="
stellar contract invoke --id "$RECOVERY_MANAGER" --source "$DEPLOYER" --network "$NETWORK" -- \
  add_guardian --guardian "$GUARDIAN_ADDR"

echo "== Deploying a test SAC asset and minting to smart_account =="
# SAC addresses are deterministic (derived from issuer + asset code), so
# re-running this script with the same DEPLOYER/ASSET_CODE hits "contract
# already exists" on a second run rather than actually failing — look the
# existing contract ID up instead of tolerating a hard error either way.
if TOKEN=$(stellar contract asset deploy --asset "$ASSET_CODE:$DEPLOYER" --source "$DEPLOYER" --network "$NETWORK" 2>/tmp/asset_deploy_err.log | tail -1) && [ -n "$TOKEN" ]; then
  :
elif grep -q "already exists" /tmp/asset_deploy_err.log; then
  TOKEN=$(stellar contract id asset --asset "$ASSET_CODE:$DEPLOYER" --network "$NETWORK")
else
  cat /tmp/asset_deploy_err.log >&2
  exit 1
fi
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
echo "NOTE: intent_registry no longer needs a separate bootstrap step --"
echo "smart_account::initialize (above) already initialized it directly,"
echo "with smart_account itself as admin. scripts/bootstrap_intent_registry.py"
echo "remains only for the existing testnet deployment, which was already"
echo "bootstrapped the old way before this script was updated; do not run it"
echo "against a smart_account deployed with this version of the script --"
echo "intent_registry.initialize would simply reject the second call."
