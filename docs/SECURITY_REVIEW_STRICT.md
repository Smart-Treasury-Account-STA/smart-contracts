# Strict Security Review: Design and Code

**Scope:** every contract in this workspace (`smart_account`, `recovery_manager`, `policy_engine`, `intent_registry`, `transfer_adapter`, `split_adapter`, `webauthn_verifier`, plus the two contracts added this cycle, `governance_account` and `threshold_policy`) and the design documents that describe them, principally `docs/GOVERNANCE_MULTISIG_DESIGN.md`. This is a follow-up to the review rounds already recorded in `docs/V1_SCOPE.md` §4–§6 — it does not repeat their findings, only what surfaced beyond them.

**Method:** every entrypoint's authorization gate was re-derived from source (this contract's own code, and the OZ crate source it composes — never from memory or from what a doc comment claimed), cross-checked against `docs/TECHNICAL_ARCHITECTURE.md`'s stated invariants, and checked for: replay/state-consumption bugs, TTL/storage-archival coverage, arithmetic overflow, error-code collisions, and — new for this pass — whether a design document's description of *what a change protects* matches what the code actually gates.

## Summary

| # | Severity | Area | Status |
|---|---|---|---|
| 1 | Medium-High | `recovery_manager` / `smart_account` — stale guardian-freeze flag, no replay guard | **Fixed** |
| 2 | Medium | `transfer_adapter` / `split_adapter` — missing permissionless TTL maintenance | **Fixed** |
| 3 | Informational | Cross-contract numeric error-code overlap with OZ's own error ranges | Documented, no fix needed |
| 4 | High (design-level) | `docs/GOVERNANCE_MULTISIG_DESIGN.md` — signer/context-rule/policy administration is not owner-gated, contrary to the document's original claim | **Documented as a named residual risk**, code intentionally unchanged (see rationale below) |

---

## 1. Stale guardian-freeze flag allowed unauthorized re-freeze (Fixed)

**Finding.** `recovery_manager::request_guardian_freeze` set a persistent boolean flag (`GuardianFreezeRequested`). `smart_account::apply_guardian_freeze` read it via `guardian_freeze_requested()` — a plain view, never cleared. Once any guardian had ever requested a freeze, the flag stayed `true` forever. Contrast with the recovery path's own `AppliedRecovery` flag, which *is* consumed. Practical effect: `apply_guardian_freeze` could be called an unbounded number of times after a single freeze request, including long after the incident that prompted it was resolved and the treasury unfrozen again by the owner — re-freezing the treasury required no fresh guardian action at all.

**Fix.** Added `recovery_manager::consume_guardian_freeze_request`, which reads the flag and clears it in the same call:

```rust
pub fn consume_guardian_freeze_request(env: Env) -> bool {
    let key = DataKey::GuardianFreezeRequested;
    let requested = env.storage().persistent().get(&key).unwrap_or(false);
    if requested {
        env.storage().persistent().remove(&key);
    }
    requested
}
```

`smart_account::apply_guardian_freeze` now calls this instead of the old read-only view. See `contracts/recovery_manager/src/lib.rs` (`consume_guardian_freeze_request_clears_it_exactly_once`, `consume_guardian_freeze_request_with_nothing_pending_returns_false`) and `contracts/smart_account/src/test.rs` (`apply_guardian_freeze_cannot_replay_an_already_consumed_request`, which proves a second `apply_guardian_freeze()` call with no intervening `request_guardian_freeze` now fails with `Error(Contract, #8012)`).

## 2. `transfer_adapter` / `split_adapter` had no permissionless TTL maintenance (Fixed)

**Finding.** `docs/TECHNICAL_ARCHITECTURE.md` §16 requires every contract to expose a permissionless way to extend its own instance TTL, independent of its main business logic succeeding — because Soroban instance-storage archival is not a soft failure: an archived contract cannot be invoked at all without a separate restore operation. Every other contract in the workspace already had this (`smart_account::extend_instance_ttl`, `recovery_manager::extend_ttl`, `policy_engine`, `intent_registry::extend_intent_ttl`, `governance_account::extend_instance_ttl`). `transfer_adapter` and `split_adapter` did not: their only TTL-touching code path was inside `execute_transfer`/`execute_split`, both of which require the configured treasury's own authorization to even reach. A treasury that goes fully dormant — plausible for e.g. a quarterly-disbursement treasury that only ever calls `execute_transfer` and never `execute_split`, or vice versa — had no way for *anyone* to keep the untouched adapter's instance storage alive.

**Fix.** Added `extend_instance_ttl(env: Env)` to both contracts, matching the exact shape already used elsewhere in the workspace. See `contracts/transfer_adapter/src/lib.rs` / `src/test.rs` (`extend_instance_ttl_is_permissionless`) and `contracts/split_adapter/src/lib.rs` / `src/test.rs` (same test name).

## 3. Cross-contract numeric error-code overlap with OZ's own ranges (Informational, no fix)

**Finding.** This workspace's own contracts are each assigned a disjoint 1000-wide error-code block (`policy_engine` 2000s, `intent_registry` 3000s, `recovery_manager` 4000s, `webauthn_verifier` 5000s, `transfer_adapter` 6000s, `split_adapter` 7000s, `smart_account` 8000s, `governance_account` 9000s — verified by reading every `#[contracterror]` enum directly, no collisions among them). However, `intent_registry`'s 3000s block collides numerically with OZ's own `stellar_accounts::smart_account::SmartAccountError`, which also starts at 3000 (`ContextRuleNotFound = 3000`, `UnvalidatedContext = 3002`, …) — confirmed directly against `stellar-accounts-0.7.2/src/smart_account/mod.rs`. For example, a bare `Error(Contract, #3002)` diagnostic means `IntentAlreadyExists` if it came from `intent_registry`, but `UnvalidatedContext` if it came from `smart_account`'s own `__check_auth`.

**Why this is not a bug:** Soroban scopes errors by which contract's invocation actually failed — a `simulateTransaction`/`getTransaction` result always identifies the failing contract address alongside the raw code, so no client correctly reading the full diagnostic can actually confuse the two. It is only a risk for a human skimming a bare error number out of context (e.g. copy-pasted into a support message without the contract address). No code change is warranted; noted here so it isn't mistaken for an unreviewed gap.

## 4. Governance design does not protect signer/context-rule/policy administration (documented, not code-fixed)

**Finding.** `docs/GOVERNANCE_MULTISIG_DESIGN.md`'s original §1 stated that `add_context_rule`/`add_signer` were among `smart_account`'s "owner-gated calls." This was checked directly against the OZ crate source rather than taken on faith, and it is incorrect: `add_context_rule`, `update_context_rule_name`, `update_context_rule_valid_until`, `remove_context_rule`, `add_signer`, `remove_signer`, `add_policy`, and `remove_policy` are all OZ trait defaults gated by `e.current_contract_address().require_auth()` (`smart_account`'s own composed signer/context-rule system), not by the separate `owner` field:

```rust
// stellar-accounts-0.7.2/src/smart_account/mod.rs
fn add_signer(e: &Env, context_rule_id: u32, signer: Signer) -> u32 {
    e.current_contract_address().require_auth();
    storage::add_signer(e, context_rule_id, &signer)
}
```

**Why this matters.** The entire point of `docs/GOVERNANCE_MULTISIG_DESIGN.md` is to repoint `owner` at a multisig (`governance_account`) so a single compromised key can no longer unilaterally control the treasury. That protects `propose_adapter_change`, `freeze`, `pause`/`unpause`, and `transfer_ownership` — but it does **not** protect the eight calls above. Concretely: today, whoever satisfies any one context rule (e.g. rule 0's sole signer) can add new signers to that rule, remove existing ones, attach or detach policies, or create an entirely new context rule granting themselves broader authority — with zero owner or governance involvement. Deploying `governance_account` and repointing `owner` at it, exactly as this design document recommends, would do nothing to close this. A single compromised payment-signer key is effectively a compromise of the treasury's entire authorization topology, not just that one rule's spending power.

**Considered fix (not applied).** The same override pattern `smart_account` already uses for `pause`/`unpause` (add an `ownable::enforce_owner_auth(e)` check on top of the OZ default) would close this cleanly for all eight calls — verified the underlying `storage::add_signer`/`storage::add_context_rule`/etc. free functions are all `pub`, so the override is mechanically straightforward. **This was deliberately not implemented in this pass.** It is a breaking change to every already-integrated caller of these eight entrypoints — `scripts/add_dev_test_signer.py`, `scripts/swap_context_rule_signer.py`, `scripts/dev_fix_rule_signers.py`, `scripts/set_intent_executor.py`, and any equivalent dApp code — each of which would need to add a second, owner-scoped authorization entry to its transaction construction. That directly conflicts with this project's standing priority that dApp integration stay easy, not accumulate complexity. Silently shipping a breaking auth change to a live, externally-integrated testnet deployment without coordinating it first is a worse outcome than leaving a named, documented gap.

**Disposition.** Recorded as new §9 in `docs/GOVERNANCE_MULTISIG_DESIGN.md`, "Known limitation: this design does not protect signer/context-rule/policy administration," with the same operational-mitigation framing already used for `governance_account`'s own no-recovery-path gap (§7): treat any context rule's signer set as equivalent in sensitivity to full topology control, because in practice it is. The fix remains fully scoped for whenever it's coordinated with the dApp: add the same `ownable::enforce_owner_auth` override used by `pause`/`unpause` to all eight calls, updating every listed caller in the same change.

---

## Verification

All fixes above were run through the project's standard verification trio after landing:

- `cargo build --workspace` — clean.
- `cargo test --workspace` — 139 unit tests passing (up from 137 before this pass), 0 failed.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo fmt --all -- --check` — clean.

## Re-review of the fixes themselves

Each fix above was re-read after landing, specifically looking for problems the fix itself could have introduced:

- **Finding 1's fix** (`consume_guardian_freeze_request`): confirmed it uses `persistent().remove`, not just overwriting with `false` — so no stale-key TTL bookkeeping is left behind. Confirmed the *only* caller of the old `guardian_freeze_requested` view was `smart_account::apply_guardian_freeze`, so removing the view and replacing it with the consuming version has no other call site to miss. Confirmed the new test asserts the *second* call's `Error(Contract, #8012)` specifically — that's `smart_account`'s existing "nothing to apply" error, i.e. the fix causes a *second* `apply_guardian_freeze` to fail for the same reason a *first* one would fail with no request ever made, not a new or different error path.
- **Finding 2's fix** (`extend_instance_ttl` on both adapters): confirmed the function is a plain, unauthenticated read-free write to instance TTL only — it cannot be used to move funds, change the configured treasury address, or read/write any other state, matching every other contract's identically-shaped maintenance entrypoint. Confirmed via `extend_instance_ttl_is_permissionless` (`e.set_auths(&[])` before calling) that no authorization is accidentally required.
- **Finding 4's documentation fix**: re-read `docs/GOVERNANCE_MULTISIG_DESIGN.md` §1 and the new §9 together to confirm they no longer contradict each other, and confirmed the corrected §1 wording still accurately lists the calls that genuinely *are* owner-gated (`propose_adapter_change`, `freeze`, `pause`/`unpause`, `transfer_ownership`) without dropping any of them.

No new issues were introduced by any of the fixes in this pass.
