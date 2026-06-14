# Contract Workspace

This directory contains the focused Soroban PoC implementation for Smart Treasury Account.

The full technical design is documented in `../docs/TECHNICAL_ARCHITECTURE.md` and `../docs/SMART_CONTRACT_SPECIFICATION.md`. The code here is intentionally smaller than the final system, but it is organized around the same core contract responsibilities.

## Included PoC Contracts

| Contract package | What it demonstrates |
|---|---|
| `smart_account_poc` | treasury account initialization, signer records, signer weights, approval thresholds, policy version binding, nonce replay protection, pause/freeze controls |
| `policy_registry_poc` | asset allowlist rules, recipient allowlist rules, transfer amount caps, policy version validation |
| `intent_registry_poc` | scheduled intent storage, executor-gated execution marking, ledger-based execution windows, cancellation, child execution replay protection |
| `recovery_guard_poc` | guardian records, delayed recovery requests, authenticated guardian approvals, threshold and ledger-based timelock checks |

## PoC Boundary

The PoC does not execute real SAC transfers, implement production `__check_auth`, or include the dApp, SDK, relayer, deployment scripts, or monitoring stack. Those are specified in the architecture documents.

The purpose of this workspace is to show concrete Soroban implementation direction for the highest-risk onchain patterns: authorization state, policy checks, replay protection, scheduled execution state, real ledger-bound timing checks, and recovery controls.

## Verification

Run from the repository root:

```bash
cargo test
```

## Testnet Deployment

The current PoC contracts have been deployed to Stellar testnet. Contract IDs, WASM hashes, deployment transactions, and demonstration transactions are recorded in `../docs/TESTNET_DEPLOYMENT.md`.
