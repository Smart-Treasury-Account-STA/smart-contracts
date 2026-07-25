# Contract Workspace

This directory contains the V1 Soroban implementation for Smart Treasury Account.

The full technical design is documented in `../docs/TECHNICAL_ARCHITECTURE.md` and `../docs/SMART_CONTRACT_SPECIFICATION.md`. `../docs/V1_SCOPE.md` covers exactly what's implemented here versus integrated from OpenZeppelin's Stellar contracts versus still deferred.

## Contracts

| Contract package | What it does |
|---|---|
| `webauthn_verifier` | Stateless WebAuthn/secp256r1 + Ed25519 verification, wrapping `stellar-accounts` with zero custom cryptography. Deployed once, referenced by any number of `smart_account` signer records. |
| `smart_account` | Treasury root. Composes OpenZeppelin's `SmartAccount` (context rules, signer registry, `__check_auth`), `Ownable`, and `Pausable` — no custom auth or threshold code. Adds treasury-specific logic: nonce-replay-protected interactive payments, scheduled payment creation/execution/cancellation, one-way emergency freeze (owner- or guardian-triggered via `apply_guardian_freeze`), and recovery pull. |
| `policy_engine` | Asset allowlist rules, recipient allowlist rules, per-operation allow/block, transfer amount caps, policy version validation. |
| `intent_registry` | Scheduled intent storage, executor-gated execution marking, ledger-based execution windows, cancellation, per-child replay protection, cumulative execution-count bounding. |
| `recovery_manager` | Guardian records, guardian removal, authenticated guardian approvals (live-recomputed against current guardian registration at finalize time — not a cached counter), threshold and ledger-based timelock checks, permissionless finalization, guardian-initiated `open_recovery` (no admin required) and guardian-initiated emergency freeze request. |
| `transfer_adapter` | Single-recipient SAC transfer. Requires the configured `smart_account`'s authorization for the exact call before moving any balance. |
| `split_adapter` | Bounded one-to-many SAC split. Same preauthorization requirement as `transfer_adapter`. |

## What's Integrated, Not Built

`smart_account` depends directly on three OpenZeppelin Stellar crates (`stellar-accounts`, `stellar-access`, `stellar-contract-utils`, all `0.7.2`): signer registry and context-rule authorization, WebAuthn/Ed25519 signature verification, weighted/simple-threshold signer math, ownership management, and pause state are all theirs, composed via trait defaults (`#[contractimpl(contracttrait)] impl ... for SmartAccountTreasury {}`), not reimplemented. See `../docs/V1_SCOPE.md` §1 for how this is verified to actually land in the deployed contract interface, not just the source.

## V1 Boundary

Not yet included: `ConditionVerifier` (optional proof-gated execution extension), the dApp, SDK, relayer, monitoring stack, delayed-governance replacement of pinned subordinate module addresses, or multisig/distributed authority for the admin/owner role itself (adapter changes and guardian administration *are* timelocked now — see `../docs/V1_SCOPE.md` §6 — but proposing them is still gated by a single key; a design for distributing that role, not yet implemented, is in `../docs/GOVERNANCE_MULTISIG_DESIGN.md`). `../docs/V1_SCOPE.md` names these explicitly, plus an OZ-documented signer-weight divergence caveat. Deployment scripts themselves *are* included — see below.

The purpose of this workspace is concrete, tested Soroban implementation of the highest-risk onchain patterns: passkey/wallet signer authorization (integrated), treasury policy checks, replay protection across three distinct dimensions, real ledger-bound timing checks, spend/recovery separation, and narrow preauthorized execution. Three rounds of security review (`../docs/V1_SCOPE.md` §4, §5, and §6) found and fixed eleven real defects — one critical (scheduled-payment execution trusted caller-supplied data instead of the canonical approved intent) — plus closed a previously-unaddressed TTL/storage-archival gap, a scheduled payment that could never be cancelled, guardian-initiated recovery/freeze that didn't exist despite being documented, unrejected duplicate split recipients, a scheduled payment's adapter being reconfigurable after approval, and immediate (untimelocked) adapter reconfiguration and guardian administration; see §6.

## Verification

Run from the repository root:

```bash
cargo test --workspace
```

120 tests across all 7 packages. Measure coverage with:

```bash
cargo llvm-cov --workspace --summary-only
```

98.7% line / 97.9% region / 91.1% function coverage workspace-wide; every package clears 85% on lines and regions. Build deployable WASM with:

```bash
stellar contract build --optimize --out-dir wasm
```

## Testnet Deployment

All 7 packages are deployed and wired on Stellar testnet as of 2026-07-23 — real contract addresses, transactions, and an initialized `smart_account` with a founding Ed25519 wallet signer, reflecting every fix through this document's §6 (timelocked entrypoints, `#[contractevent]` migration). Reproduce with:

```bash
../scripts/deploy_testnet.sh
```

See `../docs/TESTNET_DEPLOYMENT.md` for the full contract-address/transaction record. That document also names one specific, deliberate limitation: entrypoints gated by the treasury's own signer authorization (`execute_transfer_payment`, `execute_split_payment`, `create_scheduled_payment`, `ExecutionEntryPoint::execute`) can't be driven by the bare `stellar` CLI, since satisfying them requires constructing OpenZeppelin's `AuthPayload` off-chain — exactly the wallet/SDK/relayer layer this repository doesn't include. Everything gated by plain owner/admin authorization (initialization, `propose_adapter_change`, `add_guardian`, all `policy_engine` configuration) is deployed, wired, and demonstrated live, including a real on-chain rejection of an unapproved recipient and an over-cap amount, using the current timelocked entrypoint names throughout. The prior PoC deployment record (different contract names, `soroban-sdk 22.0.1`, no OZ composition) is archived separately in `../docs/archive/POC_TESTNET_DEPLOYMENT.md` — not part of the current implementation.
