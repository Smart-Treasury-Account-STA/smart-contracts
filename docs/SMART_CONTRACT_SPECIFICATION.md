# Smart Treasury Account Smart Contract Specification

## 1. Contract System Overview

The target STA contract system is composed of several Soroban modules:

| Module | Purpose |
|---|---|
| SmartAccount | Root treasury authority, signer model, thresholds, authorization context, pause/freeze controls, and execution coordination |
| PolicyEngine | Asset, recipient, spending, policy version, and risk validation |
| Payment execution module | SAC transfer validation and execution for approved payments |
| IntentRegistry | Scheduled payment intent lifecycle, child execution IDs, replay protection, and execution windows |
| RecoveryManager | Delayed recovery, guardian flows, signer replacement, and frozen-state recovery controls |
| ConditionVerifier | Optional extension for proof-gated execution using signed external attestations |
| TypeScript SDK | Typed helpers for contract calls, transaction preparation, event parsing, and network configuration |

### 1.1 Repository PoC Mapping

The repository includes a focused PoC implementation of selected contract responsibilities. The PoC is intentionally partial and does not replace the full production specification in this document.

| Full module | PoC package | Scope represented in the PoC |
|---|---|---|
| SmartAccount | `contracts/smart_account_poc` | signer records, weights, thresholds, payment validation, nonce replay protection, pause/freeze state |
| PolicyEngine | `contracts/policy_registry_poc` | asset rules, recipient rules, amount caps, policy version checks |
| IntentRegistry | `contracts/intent_registry_poc` | scheduled intent records, executor-gated execution marking, ledger-based execution windows, child execution replay protection |
| RecoveryManager | `contracts/recovery_guard_poc` | guardian records, authenticated guardian approvals, delayed recovery requests, approval counting, ledger-based finalization checks |

The remaining production modules, including real SAC transfer execution, adapter dispatch, full `__check_auth`, SDK, relayer, dApp, deployment scripts, and monitoring, are specified as part of the complete STA architecture.

The current PoC packages are deployed on Stellar testnet. Contract IDs, deployment transactions, and example interactions are recorded in `TESTNET_DEPLOYMENT.md`.

## 2. SmartAccount Contract

The SmartAccount contract is the root treasury authority. It is responsible for:

- treasury initialization
- signer records
- signer roles
- signer weights
- threshold validation
- custom account authorization through `__check_auth`
- policy version binding
- replay protection
- pause and freeze controls
- execution coordination with policy and adapter modules

## 3. PolicyEngine Contract

The PolicyEngine contract validates the rules that determine whether a treasury action is allowed.

Responsibilities:

- asset allowlists
- recipient allowlists
- amount limits
- spending rules
- policy version checks
- operation-specific policy checks
- fail-closed validation for unsupported assets, recipients, or actions

## 4. Payment Execution Module

The payment execution module validates and executes approved Stellar Asset Contract transfers.

Responsibilities:

- verify the asset is an approved SAC asset
- verify the recipient is approved
- verify amount limits
- route approved payment execution
- emit payment execution events
- reject unsupported or malformed payment actions

## 5. IntentRegistry Contract

The IntentRegistry contract manages scheduled treasury payments and replay protection for scheduled execution.

Responsibilities:

- parent scheduled intent records
- child execution IDs
- execution windows
- cancellation and expiry
- replay protection
- execution settlement state

## 6. RecoveryManager Contract

The RecoveryManager contract supports recovery-oriented workflows for compromised or lost signer authority.

Responsibilities:

- delayed recovery initiation
- guardian-driven freeze flows
- signer replacement
- recovery cancellation
- recovery finalization
- recovery event emission

## 7. Optional ConditionVerifier Contract

The ConditionVerifier contract is an extension module for proof-gated execution.

Responsibilities:

- approved attestor sets
- attestor quorum rules
- proof domain binding
- proof freshness
- consumed proof IDs
- replay rejection

## 8. Core Invariants

The complete STA system must enforce the following invariants:

- treasury assets move only through SmartAccount-authorized execution
- unsupported assets fail closed
- unsupported recipients fail closed
- invalid or reused nonces fail closed
- paused accounts reject normal execution
- frozen accounts reject normal execution
- signer thresholds cannot be configured into an unusable state
- scheduled executions cannot replay the same child execution ID
- relayers cannot create authority or bypass policy checks

## 9. Mainnet Contract Modules

The full mainnet implementation will include:

- SmartAccount contract
- PolicyEngine contract
- IntentRegistry contract
- Transfer/Split execution adapters
- optional ConditionVerifier
- deployment configuration
- TypeScript SDK aligned with deployed contract IDs
