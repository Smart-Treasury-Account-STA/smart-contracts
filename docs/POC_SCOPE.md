# Smart Contract PoC Scope

## Purpose

This repository includes a focused smart contract PoC to demonstrate selected implementation patterns for Smart Treasury Account (STA). The PoC is intentionally partial. It is not the full production contract suite.

The full technical architecture is described in `TECHNICAL_ARCHITECTURE.md` and `SMART_CONTRACT_SPECIFICATION.md`.

The current PoC contracts are also deployed on Stellar testnet. Contract IDs and example transactions are recorded in `TESTNET_DEPLOYMENT.md`.

## Included in the PoC

The PoC is organized as a small Soroban workspace with four contract packages.

### `contracts/smart_account_poc`

Demonstrates the root treasury authority pattern:

- treasury initialization
- signer records and signer roles
- signer weights and threshold validation
- signer revocation and weight updates
- duplicate approver rejection
- policy version pinning
- SAC-style payment intent validation
- nonce-based replay protection
- pause and freeze controls

### `contracts/policy_registry_poc`

Demonstrates policy validation as a separate module:

- asset policy records
- recipient allowlist records
- per-transfer amount caps
- policy version updates
- fail-closed validation for missing assets or recipients

### `contracts/intent_registry_poc`

Demonstrates scheduled treasury state:

- scheduled intent records
- executor-gated execution marking
- execution window validation using Soroban ledger state
- cancellation
- child execution replay protection
- sanitized creation state for new intents

### `contracts/recovery_guard_poc`

Demonstrates recovery-oriented controls:

- guardian registration
- delayed recovery request opening
- authenticated guardian approval counting
- duplicate approval rejection
- threshold and timelock finalization checks using Soroban ledger state

## Not Included in the PoC

The PoC does not include:

- full `__check_auth` implementation
- real SAC token transfer execution
- production adapter contracts
- relayer implementation
- TypeScript SDK
- full production recovery manager
- proof-gated attestations
- production dApp
- deployment scripts
- monitoring and indexer services

## Security Interpretation

The PoC is designed to show a clear implementation direction for the main onchain security patterns: account authorization state, policy checks, replay protection, scheduled execution state, and recovery controls. It should not be interpreted as a deployable production wallet.

The production implementation must add custom account authorization, real SAC execution, adapter dispatch, complete signer aggregation, deployment scripts, monitoring, and full integration tests.

## Test Coverage

The PoC unit tests cover:

- initialization and status
- allowed payment intent validation
- nonce replay rejection
- disallowed asset rejection
- disallowed recipient rejection
- amount limit rejection
- pause and freeze rejection
- below-threshold signer rejection
- multi-signer threshold acceptance
- duplicate approver rejection
- revoked signer rejection
- stale policy version rejection
- standalone policy validation
- scheduled intent replay rejection
- scheduled execution window checks
- cancelled scheduled intent rejection
- sanitized scheduled intent creation
- guardian recovery threshold checks
- recovery timelock checks
- duplicate guardian approval rejection
- duplicate guardian count handling in view helpers

## Current Review Value

The PoC is intentionally small, but the implemented slice is security-relevant:

- it fails closed for unsupported assets and recipients
- it rejects replayed payment intents
- it tracks signer weights instead of accepting a single privileged account
- it rejects duplicated approver entries
- it updates threshold weight when a signer is revoked
- it pins payment validation to a policy version
- it blocks normal payment validation while paused or frozen
- it separates policy, intent, and recovery concerns into distinct contracts
- it models scheduled child execution replay protection
- it gates scheduled execution marking behind an authorized executor
- it uses Soroban ledger sequence for execution-window and recovery-delay checks
- it models authenticated guardian approval and delayed recovery controls

The testnet deployment demonstrates these patterns with live contract IDs, events, and transaction links in `TESTNET_DEPLOYMENT.md`.
