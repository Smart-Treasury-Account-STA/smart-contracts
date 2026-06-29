# PoC Review Guide

## Purpose

This guide explains how to review the current Smart Treasury Account PoC. The PoC is a partial implementation intended to demonstrate concrete Soroban implementation patterns while keeping the full architecture documented separately.

The deployed testnet PoC can be reviewed through `TESTNET_DEPLOYMENT.md`, which includes contract IDs, explorer links, deployment transactions, and example interaction transactions.

## What to Inspect

Primary contract packages:

- `contracts/smart_account_poc/src/lib.rs`
- `contracts/policy_registry_poc/src/lib.rs`
- `contracts/intent_registry_poc/src/lib.rs`
- `contracts/recovery_guard_poc/src/lib.rs`

Key technical areas:

- treasury initialization
- signer records and payment roles
- signer weight accounting
- signer revocation
- asset allowlist checks
- recipient allowlist checks
- nonce replay protection
- policy version pinning
- pause/freeze enforcement
- standalone policy validation
- scheduled intent windows
- authorized scheduled execution marking
- child execution replay protection
- authenticated guardian approval and delayed recovery checks
- unit tests for success and rejection paths

## Verification Commands

Run:

```bash
cargo test
```

Expected result:

- all unit tests pass
- tests cover valid flows and rejection paths across account, policy, intent, and recovery modules

For live testnet verification, use the contract IDs and read-only commands in `TESTNET_DEPLOYMENT.md`.

## Implemented Test Cases

The PoC test suite verifies:

- initialization and status reporting
- allowed payment intent validation
- nonce replay rejection
- disallowed asset rejection
- disallowed recipient rejection
- amount cap rejection
- pause rejection
- freeze rejection
- insufficient signer weight rejection
- multiple approvers meeting threshold
- duplicate approver rejection
- revoked signer rejection
- stale policy version rejection
- policy registry fail-closed checks
- policy registry version checks
- scheduled child execution replay rejection
- scheduled execution window rejection
- cancelled scheduled intent rejection
- sanitized scheduled intent creation
- delayed recovery threshold and timelock checks
- duplicate guardian approval rejection
- duplicate guardian count handling
- removed-guardian approval rejection and double-removal rejection

## Security Notes

The PoC is deliberately not a production wallet. It does not implement full custom account authorization or real SAC transfer execution. Those belong to the full SmartAccount and payment execution modules described in the architecture.

The purpose of this PoC is to show the core enforcement direction:

- policy checks happen before execution
- replay state is consumed once
- approver weights are counted explicitly
- duplicate approvers are rejected
- account safety states block normal treasury actions
- scheduled executions are bounded by intent state
- scheduled execution marking requires an authorized executor
- scheduled execution windows use Soroban ledger state instead of caller-provided time
- recovery finalization requires authenticated guardian threshold and ledger-based delay checks
