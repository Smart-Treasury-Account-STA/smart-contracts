# Smart Treasury Account

Smart Treasury Account (STA) is a Soroban-native programmable treasury account for Stellar.

STA enables organizations to manage treasury operations through Soroban smart contracts, Stellar Asset Contracts, wallet-based approvals, policy-controlled execution, scheduled payments, and transparent operational monitoring.

## Product Architecture

The complete STA architecture includes:

- Soroban SmartAccount contracts
- treasury policy logic for assets, recipients, limits, thresholds, and execution rules
- SAC-based payment execution
- scheduled payment intent handling with replay protection
- submit-only relayer flow for approved scheduled execution
- production dApp
- initial public TypeScript SDK
- monitoring, documentation, and testing support

## Core Services

STA is designed to provide:

- programmable treasury accounts on Stellar
- signer roles, signer weights, and threshold approval rules
- approved asset and recipient policies
- vendor, payroll, and operational payment flows
- revenue split support
- scheduled treasury payments
- wallet-first access through Stellar wallet tooling
- relayer-safe execution without relayer custody
- audit-friendly events and status tracking
- pause, freeze, and recovery-oriented controls

## Repository Scope

This repository is technical only.

The documentation describes the full Smart Treasury Account system: contracts, dApp, SDK, relayer, deployment model, testing strategy, and operational controls.

The smart contract code included in this repository is a working V1 implementation of the core onchain modules — not yet the full production suite (no dApp, SDK, relayer, or deployment automation), but a real, cross-contract-integrated, tested Soroban workspace. Passkey and wallet signer authentication, and signer weight/threshold math, are integrated from audited OpenZeppelin Stellar contracts (`stellar-accounts`, `stellar-access`, `stellar-contract-utils`) rather than built from scratch. See `docs/V1_SCOPE.md` for the exact V1 boundary, what's genuinely new versus integrated, and how the signer/approval logic has been stress-tested.

## Contract Workspace

Seven Soroban contract packages:

| Package | Purpose |
|---|---|
| `contracts/webauthn_verifier` | stateless WebAuthn/secp256r1 + Ed25519 signature verification, wrapping `stellar-accounts` with zero custom cryptography |
| `contracts/smart_account` | treasury root: composes OpenZeppelin's `SmartAccount`/`Ownable`/`Pausable`, interactive and scheduled payment execution, one-way emergency freeze, recovery pull |
| `contracts/policy_engine` | asset/recipient/operation allowlists, amount caps, policy version pinning |
| `contracts/intent_registry` | scheduled intent lifecycle, ledger-based execution windows, per-child and cumulative replay protection |
| `contracts/recovery_manager` | guardian registration, authenticated approval (live-recomputed against current guardian set), ledger-based timelock, permissionless finalization |
| `contracts/transfer_adapter` | single-recipient SAC transfer, narrowly preauthorized |
| `contracts/split_adapter` | bounded one-to-many SAC split, narrowly preauthorized, each recipient independently policy-checked |

These packages are not the full production contract suite (no `ConditionVerifier`, no delayed governance replacement of pinned module addresses — see `docs/V1_SCOPE.md`), but the core treasury-control logic — policy, scheduling, recovery, and execution — is real and tested against real deployed instances of every module, not mocks.

## Technical Documents

Start at **`docs/README.md`** — it's the reviewer-facing index with a recommended reading order. Individually:

- `docs/V1_SCOPE.md`: scope of the current V1 implementation, passkey integration, budget/novelty justification, and stress-tested edge cases.
- `docs/V1_REVIEW_GUIDE.md`: how to review and verify V1.
- `docs/SMART_CONTRACT_SPECIFICATION.md`: smart contract design and module responsibilities.
- `docs/TECHNICAL_ARCHITECTURE.md`: full Smart Treasury Account architecture.
- `docs/TESTNET_DEPLOYMENT.md`: live V1 testnet deployment record — real contract addresses, transactions, and a named limitation around signer-gated calls (reproducible via `scripts/deploy_testnet.sh`).
- `contracts/README.md`: V1 contract workspace map.

`docs/archive/` holds the earlier partial PoC's testnet deployment record only, kept for historical traceability — it is intentionally separated from the documents above and is not part of the current implementation to review.

## Run Tests

```bash
cargo test --workspace
```

105 tests across 7 packages (98.6% line / 97.8% region / 90.9% function coverage workspace-wide, `cargo llvm-cov --workspace`), including real cryptographic fixtures and real Stellar Asset Contract transfers. Has been through two rounds of dedicated security review — see `docs/V1_SCOPE.md` §4 and §5 for the findings and fixes.

## Build Deployable WASM

```bash
stellar contract build --optimize --out-dir wasm
```

## Deploy to Testnet

```bash
./scripts/deploy_testnet.sh
```

Deploys and wires all 7 contracts to Stellar testnet using the `stellar` CLI. See `docs/TESTNET_DEPLOYMENT.md` for the live contract addresses, transaction record, and a named limitation on which entrypoints a bare CLI deployment can and can't exercise (signer-gated payment calls need an off-chain wallet/SDK client to construct the treasury's own authorization — see that doc's §7).
