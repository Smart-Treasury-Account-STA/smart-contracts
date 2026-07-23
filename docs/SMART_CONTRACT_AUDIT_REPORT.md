# Smart Treasury Account (STA) Smart Contract Audit Report

## Scope
Reviewed the current Soroban smart contract workspace in this repository, including:
- contracts/smart_account/src/lib.rs
- contracts/policy_engine/src/lib.rs
- contracts/intent_registry/src/lib.rs
- contracts/recovery_manager/src/lib.rs
- contracts/transfer_adapter/src/lib.rs
- contracts/split_adapter/src/lib.rs
- contracts/webauthn_verifier/src/lib.rs

## Overall Assessment
The architecture remains strong and the implementation shows thoughtful handling of authorization, policy enforcement, replay protection, recovery, and execution isolation. I did not identify a clear critical or high-severity exploit that would allow direct fund loss through an auth bypass or contract logic flaw.

The most important changes implemented since the earlier review are:
- scheduled payments now pin the adapter address at approval time and use that pinned adapter during execution;
- adapter reconfiguration is now delayed via propose/apply/cancel flows;
- guardian removal and threshold changes are now delayed via propose/apply/cancel flows;
- guardian-triggered freeze handling is now implemented.

## Current Findings

### 1. Medium — Pending governance proposals may become inaccessible over time
- Location: contracts/smart_account/src/lib.rs, contracts/recovery_manager/src/lib.rs
- Description:
  - Delayed governance proposals for adapter changes, guardian removals, and threshold changes are stored in contract state.
  - The current implementation does not explicitly protect those pending proposals against long-lived TTL expiry in the same way the core treasury state is protected.
- Impact:
  - This is primarily a liveness and operational risk.
  - A legitimate delayed governance action could be lost if the proposal sits dormant long enough.
- Recommendation:
  - Explicitly extend TTL on proposal creation and proposal updates.
  - Consider a monitoring or maintenance mechanism for long-lived pending proposals.

### 2. Medium — The trust model is still concentrated in a single owner/admin
- Location: contracts/smart_account/src/lib.rs, contracts/recovery_manager/src/lib.rs
- Description:
  - The current design still relies on a single owner/admin address to propose and cancel sensitive governance actions.
  - Timelocks reduce the risk of immediate abuse, but they do not remove the central trust point.
- Impact:
  - A compromised owner/admin can still schedule a harmful change that becomes effective later.
  - This remains a meaningful concern for production-grade treasury custody.
- Recommendation:
  - Replace the single owner/admin authority with a multisig or governance account contract.
  - The documentation already describes this as the intended follow-up path.

### 3. Low / Medium — Governance actions remain somewhat operationally implicit
- Location: contracts/smart_account/src/lib.rs
- Description:
  - The propose/apply/cancel model is now in place, but it still depends on off-chain observation and monitoring.
- Impact:
  - A user or operator could miss a pending governance change or misinterpret its state.
- Recommendation:
  - Improve event clarity and off-chain tooling so pending governance changes are easy to discover and track.

## Multisig / Governance Note
The documentation now explicitly describes a governance-account/multisig pattern as the mechanism that can replace the single owner/admin authority. This does not require rewriting the treasury business logic itself; the existing contracts can continue to use owner/admin authorization flows, provided the owner/admin address is replaced with a multisig-style account contract.

## Verification
I verified the current implementation by running:
- cargo test --workspace --quiet

Result:
- Exit status: 0

## Conclusion
The current implementation is materially stronger than the earlier version and no critical issue was identified in the latest review. The remaining concerns are governance resilience and liveness of delayed proposals rather than direct exploitability. For production custody use, the next recommended step is to move to a multisig/governance-account authority model and ensure pending proposals are kept alive through explicit TTL management.
