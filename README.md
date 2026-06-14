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

The smart contract code included in this repository is a focused PoC preview of selected onchain modules. It is intentionally partial, but structured as a real Soroban workspace with concrete implementation patterns and unit tests. See `docs/POC_SCOPE.md` for the exact PoC boundary.

## Contract Workspace

The PoC currently includes four focused Soroban contract packages:

| Package | Purpose |
|---|---|
| `contracts/smart_account_poc` | signer registry, weighted approvals, payment validation, nonce replay protection, pause/freeze state |
| `contracts/policy_registry_poc` | asset rules, recipient rules, amount caps, policy version checks |
| `contracts/intent_registry_poc` | scheduled intent records, executor-gated execution marking, ledger-based execution windows, child execution replay protection |
| `contracts/recovery_guard_poc` | guardian registration, authenticated guardian approvals, delayed recovery requests, threshold and ledger-based finalization checks |

These packages are not the full production contract suite. They are a clean preview of the main security patterns that will be expanded into the complete STA architecture.

## Technical Documents

- `docs/TECHNICAL_ARCHITECTURE.md`: full Smart Treasury Account architecture.
- `docs/SMART_CONTRACT_SPECIFICATION.md`: smart contract design and module responsibilities.
- `docs/POC_SCOPE.md`: scope of the partial smart contract PoC.
- `docs/POC_REVIEW_GUIDE.md`: how to review and verify the PoC.
- `docs/TESTNET_DEPLOYMENT.md`: deployed testnet contract IDs and example transactions.
- `contracts/README.md`: PoC contract workspace map.

## Run Tests

```bash
cargo test
```
