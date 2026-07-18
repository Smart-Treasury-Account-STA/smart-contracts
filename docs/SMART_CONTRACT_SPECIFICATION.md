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

### 1.1 Repository V1 Mapping

The repository includes a working V1 implementation of the contract responsibilities below, superseding the earlier partial PoC. `docs/V1_SCOPE.md` is the authoritative statement of what's implemented, what's integrated from OpenZeppelin's Stellar contracts rather than built, and what remains deferred.

| Full module | V1 package | Scope represented in V1 |
|---|---|---|
| SmartAccount | `contracts/smart_account` | Composes OZ `SmartAccount`/`CustomAccountInterface`/`Ownable`/`Pausable` (context rules, signer registry, `__check_auth`, owner gating, pause state — not custom-built); adds nonce-replay-protected interactive payments, scheduled payment creation/execution, one-way emergency freeze, recovery pull |
| PolicyEngine | `contracts/policy_engine` | asset rules, recipient rules, operation allow/block, amount caps, policy version checks |
| IntentRegistry | `contracts/intent_registry` | scheduled intent records, executor-gated execution marking, ledger-based execution windows, per-child execution replay protection, cumulative execution-count bounding |
| RecoveryManager | `contracts/recovery_manager` | guardian records, guardian removal, authenticated guardian approvals live-recomputed against current guardian registration at finalize time, delayed recovery requests, ledger-based finalization checks |
| Passkey/WebAuthn verifier | `contracts/webauthn_verifier` | stateless wrapper dispatching to OZ's `verifiers::webauthn`/`verifiers::ed25519` — zero custom cryptography |
| Payment execution module | `contracts/transfer_adapter`, `contracts/split_adapter` | real SAC transfer/split execution, narrowly preauthorized via `smart_account.require_auth()` on the exact call |

The remaining production modules — full `ConditionVerifier`, TypeScript SDK, relayer, dApp, monitoring, and delayed-governance replacement of pinned module addresses — are specified as part of the complete STA architecture and remain out of scope for this repository; see `docs/V1_SCOPE.md`'s "Not Yet Included in V1" section.

The current V1 packages are deployed live on Stellar testnet — see `docs/TESTNET_DEPLOYMENT.md`. The prior PoC's separate deployment record is archived in `docs/archive/POC_TESTNET_DEPLOYMENT.md`.

## 2. SmartAccount Contract

The SmartAccount contract is the root treasury authority. It is responsible for:

- treasury initialization
- signer records
- signer roles
- signer weights
- threshold validation
- custom account authorization through `__check_auth`, delegating entirely to `stellar_accounts::smart_account::do_check_auth` (context rules, signer registry, policy attachment) rather than a custom-built verifier or threshold engine — implemented in `contracts/smart_account`, verified in `docs/V1_SCOPE.md` §1
- policy version binding
- replay protection
- pause and freeze controls: pause via OpenZeppelin's audited `stellar_contract_utils::pausable` module, freeze as a custom one-way flag lifted only through `RecoveryManager`
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
