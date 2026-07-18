# V1 Review Guide

## Purpose

This guide explains how to review the current Smart Treasury Account V1 implementation: seven Soroban packages composing real OpenZeppelin Stellar contracts for signer authentication/passkeys/threshold policy, plus four STA-authored contracts for treasury-specific risk, scheduling, recovery, and execution. `docs/V1_SCOPE.md` covers the full rationale (why this split, what's genuinely new, what was stress-tested); this guide is about how to walk the code and verify it yourself.

`docs/TESTNET_DEPLOYMENT.md` records a live V1 deployment of all seven packages to Stellar testnet, reproducible via `scripts/deploy_testnet.sh` — real contract addresses, real transactions, and one explicitly named limitation (signer-gated payment entrypoints need an off-chain wallet/SDK client to construct the treasury's own authorization; the bare CLI can't drive those, only the plain owner/admin-gated ones). The earlier PoC deployment that used to occupy this document (different contract names, `soroban-sdk 22.0.1`) is preserved for history in that document's §9.

## What to Inspect

Integrated (not authored here — read `docs/V1_SCOPE.md` §1–2 first to understand why):

- `~/.cargo/registry/.../stellar-accounts-0.7.2/src/smart_account/` — context rules, signer registry, `do_check_auth`
- `~/.cargo/registry/.../stellar-accounts-0.7.2/src/verifiers/webauthn.rs` and `ed25519.rs` — signature verification
- `~/.cargo/registry/.../stellar-accounts-0.7.2/src/policies/{weighted_threshold,spending_limit,simple_threshold}.rs` — threshold math
- `~/.cargo/registry/.../stellar-access-0.7.2/src/ownable/` and `~/.cargo/registry/.../stellar-contract-utils-0.7.2/src/pausable/`

Authored in this repository:

- `contracts/webauthn_verifier/src/lib.rs` — thin dispatch wrapper (should stay small; if this file starts containing signature-parsing logic, that's a regression back toward "building the auth layer from scratch")
- `contracts/smart_account/src/lib.rs` — treasury root; check that every entrypoint requiring signer approval calls `env.current_contract_address().require_auth()` (not a bespoke check), and that `__check_auth` is a one-line delegation to `do_check_auth`
- `contracts/policy_engine/src/lib.rs`
- `contracts/intent_registry/src/lib.rs`
- `contracts/recovery_manager/src/lib.rs` — in particular `live_approval_count` and its use inside `finalize_recovery`; this is the fix for the mid-flight signer-set-change class of bug (see below)
- `contracts/transfer_adapter/src/lib.rs`, `contracts/split_adapter/src/lib.rs` — check that `execute_transfer`/`execute_split` call `smart_account.require_auth()` before any token movement

Key technical areas, and where each is enforced:

| Area | Where |
|---|---|
| Passkey / Ed25519 signer authentication | `webauthn_verifier` (verification), `smart_account::__check_auth` (dispatch) |
| Signer weight / threshold accounting | OZ `weighted_threshold`/`simple_threshold` policies, attached at `smart_account::initialize` |
| Asset / recipient / operation allowlists, amount caps | `policy_engine::validate_policy` |
| Policy version pinning | `policy_engine` version counter + `expected_version` check on every `validate_policy` call |
| Interactive-payment nonce replay | `smart_account`'s own `UsedNonce` map |
| Scheduled execution windows, per-child + cumulative replay | `intent_registry` |
| Pause / one-way freeze | `stellar_contract_utils::pausable` (pause) + `smart_account`'s own `Frozen` flag (freeze; no unfreeze entrypoint exists) |
| Recovery: guardian- or admin-initiated, live-recomputed quorum, timelock | `recovery_manager::open_recovery` (admin or active guardian), `finalize_recovery` + `live_approval_count` |
| Guardian-initiated emergency freeze | `recovery_manager::request_guardian_freeze` (any one active guardian) + `smart_account::apply_guardian_freeze` (permissionless pull) |
| Recovery application (owner replacement, freeze lift) | `smart_account::apply_recovery` — pulls from `recovery_manager`, never pushed |
| Scheduled payment cancellation | `smart_account::cancel_scheduled_payment` → `intent_registry::cancel_intent` |
| Narrow, exactly-preauthorized execution | `transfer_adapter`, `split_adapter` |

## Verification Commands

```bash
cargo test --workspace
```

Expected: 105 tests pass, 0 failures, across `sta-webauthn-verifier` (9), `sta-policy-engine` (12), `sta-intent-registry` (14), `sta-recovery-manager` (27), `sta-transfer-adapter` (5), `sta-split-adapter` (8), `sta-smart-account` (30).

```bash
cargo llvm-cov --workspace --summary-only
```

Expected: 98.6% line / 97.8% region / 90.9% function coverage workspace-wide — every metric above 90% at the workspace level, every package at or above 85% on lines and regions. See `docs/V1_SCOPE.md`'s Verification section for the per-package breakdown and why function-count percentages read lower for four of the packages (verified via raw symbol inspection to be macro-generated dispatch shims unreachable from native tests, not untested logic).

```bash
stellar contract build --optimize --out-dir wasm
```

Expected: all seven packages produce WASM. Spot-check the deployed interface actually contains the OZ composition, not just the source:

```bash
stellar contract info interface --wasm wasm/sta_smart_account.wasm
```

Expected: alongside `execute_transfer_payment`/`execute_split_payment`/`apply_recovery`/etc., you should see `add_context_rule`, `add_signer`, `add_policy`, `get_owner`, `transfer_ownership`, `pause`, `unpause`, and `__check_auth` — these come from the OZ trait defaults, not hand-written code in `contracts/smart_account`.

```bash
./scripts/deploy_testnet.sh
```

Expected: deploys and wires all 7 packages to Stellar testnet under fresh contract IDs (a new deployment, distinct from the one recorded in `docs/TESTNET_DEPLOYMENT.md`), then proves a `policy_engine.validate_policy` call passes for an allowed payment and fails closed with `#2004`/`#2005` for an unapproved recipient and an over-cap amount, respectively — live on-chain, not simulated. See that script's header comment and `docs/TESTNET_DEPLOYMENT.md` §7 for exactly which entrypoints it can and can't drive (signer-gated payment calls need an off-chain SDK client, not the bare CLI).

## Implemented Test Cases

See `docs/V1_SCOPE.md` §3 for the specific signer/approval edge cases (mid-flight guardian removal, threshold changes after approval, replay across nonce/child-execution/cumulative-usage dimensions, recovery pull ordering, unmocked-auth negative tests, policy-version-bump/freeze/cancel/duplicate-recipient/shared-nonce cross-contract edge cases), §4 for the first round of findings, and §5 for a second round focused on feature completeness against the architecture doc. Full per-file breakdown:

- `webauthn_verifier`: real secp256r1 WebAuthn assertion verified end to end, tampered-signature and malformed-signature-encoding rejection (both schemes), real Ed25519 wallet-signature verification, credential-ID-suffix canonicalization, unsupported-key-length rejection (both `verify` and `canonicalize_key`), batch canonicalization ordering.
- `policy_engine`: allowed-payment happy path, fail-closed on unknown operation/asset/recipient, **explicitly-disabled-asset rejection (distinct from no-rule-at-all)**, stale-version and amount-cap rejection, non-positive-amount rejection, operation disabled after being enabled still blocks, `bump_version` rejects non-increasing versions, calling before `initialize` rejects, TTL extension on write/read and via the permissionless maintenance entrypoint.
- `intent_registry`: create + single execution, execution-window rejection (too early/expired), cancelled-intent rejection, caller-supplied `cancelled`/`execution_count` sanitized away, zero-`max_executions` rejected at creation, cumulative execution limit enforced across distinct (non-replayed) children, non-positive-amount/inverted-window/duplicate-intent-id rejection, double-initialize and before-initialize rejection, **policy version pinned at creation and returned verbatim**, TTL extension on write/read and via maintenance entrypoint.
- `recovery_manager`: threshold+timelock finalization, duplicate-approval and below-threshold rejection, removed-guardian cannot approve / double-removal rejected, cancelled request cannot finalize, finalization-before-timelock rejected, **guardian removed after approving no longer counts**, **threshold raised after approvals invalidates a previously-sufficient request**, finalized request rejects further approval/re-finalization, **minimum recovery delay enforced (absolute and relative-to-current-ledger)**, **guardian activation delay enforced**, duplicate-guardian/duplicate-request rejection, max-approvers bound, TTL extension on write and via maintenance entrypoint, **an active guardian can open a recovery request without the admin, an inactive guardian or unrelated third party cannot**, **an active guardian can raise and clear-view the emergency-freeze flag, an inactive guardian or non-guardian cannot**.
- `transfer_adapter` / `split_adapter`: real SAC transfer/split against a registered Stellar Asset Contract, non-positive-amount rejection, double-initialize rejection, length-mismatch/empty-split/over-bound-recipient-count rejection (split only), **no-authorization-at-all rejection** via `set_auths(&[])`.
- `smart_account`: init/status, full policy-engine-and-adapter-backed transfer payment, nonce replay rejection (and shared across transfer/split operations), asset-not-allowed propagation from `policy_engine`, paused/frozen rejection (and freeze has no direct unfreeze, **and pause/unpause reject a caller that isn't the owner**), split payment fan-out with independent per-recipient policy checks (fails closed if any one recipient isn't allowed, **if any one recipient's amount exceeds the cap even when others are fine, or if a recipient is duplicated**), scheduled-payment creation + relayer-triggered execution + **cancellation (end to end through intent_registry)**, **`execute_scheduled_payment` proven to use only canonical intent data**, **a policy-version bump between scheduling and execution blocks it closed**, **freezing blocks scheduled execution too**, **reconfiguring the adapter after intent approval silently redirects execution (proving the documented residual risk is real)**, recovery pull happy path (owner replaced, freeze lifted, replay-guarded), recovery pull rejected pre-finalization, **a guardian can independently freeze the treasury with no owner involvement**, no-authorization-at-all rejection, instance/nonce TTL extension, and the composed OZ governance surface (`get_owner`, two-step `transfer_ownership`/`accept_ownership`, `add_context_rule`/`add_signer`/`remove_signer`, `ExecutionEntryPoint::execute` dispatch to a target contract).

## Security Notes

This is a working V1, not yet a mainnet-ready system — see `docs/V1_SCOPE.md`'s "Not Yet Included in V1" and "Known, named residual risk" sections for the specific, named gaps still open (scoped session keys entirely absent; OZ's own documented weighted-threshold signer-divergence caveat; adapter resolution at execution time rather than pinned at intent-creation time), §4 for the first round of findings a dedicated security review found and fixed (a critical caller-supplied-data bug in scheduled execution, a missing minimum recovery delay, a missing guardian-activation delay, and the TTL/storage-archival production requirement that had no coverage at all), and §5 for a second round: a scheduled payment that could never actually be cancelled, recovery that was unusable in exactly the scenario it exists for (admin-only initiation), no guardian-facing emergency freeze at all, and unrejected duplicate recipients in a split. All are stated explicitly rather than left for a reviewer to discover.

What V1 does enforce, concretely and testably:

- passkey and wallet signers are authenticated through real, audited, external verification code — not a stub
- signer weight/threshold math is OZ's, not reimplemented
- treasury policy checks (asset/recipient/operation/amount/version) happen before every execution, independent of who signed
- replay state is consumed exactly once, across three distinct replay dimensions (interactive nonce, scheduled per-child, scheduled cumulative count)
- adapters cryptographically require the specific treasury's authorization for the exact action, not just "some caller invoked me"
- recovery is a distinct authorization path from day-to-day spend, pulled and applied exactly once, with approval counted live against current guardian registration rather than a cached total
- pause is reversible; freeze is one-way and can only be lifted by a completed recovery
