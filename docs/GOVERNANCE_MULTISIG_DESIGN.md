# Governance Multisig: Design Notes

**Status: design only, not implemented.** This document scopes the second follow-up item from `docs/V1_SCOPE.md` §6's final review sign-off — distributing the admin/owner role itself across multiple keys — as a written plan, per that section's own statement that this is "a separate, larger feature decision" not to be folded into a review-fix pass. Nothing here has been built. It exists so the decision can be made deliberately, with the actual trade-offs in view, rather than either building it hastily or leaving it unscoped indefinitely.

## 1. Problem, precisely stated

Every owner-gated call on `smart_account` (`propose_adapter_change`, `freeze`, `pause`/`unpause`, `transfer_ownership`, `add_context_rule`/`add_signer`, …) and every admin-gated call on `recovery_manager` (`propose_remove_guardian`, `propose_threshold_change`, `add_guardian`, `cancel_recovery`, …) is authorized by a single `Address` — checked via `ownable::enforce_owner_auth`/`ensure_admin`, which is just `owner.require_auth()` / `admin.require_auth()`. In V1, that `Address` is a single Stellar keypair (a classic account or an `Ed25519`/passkey signer behind it). One compromised key can call any of these.

`docs/V1_SCOPE.md` §6 already added a real mitigation for the highest-impact subset (`propose_adapter_change`, `propose_remove_guardian`, `propose_threshold_change`): a ~1 day ledger-measured delay between proposing and the change taking effect, with a cancel path. That means a single compromised key can no longer make those *specific* changes land immediately — but it can still *propose* them, and it can still call the calls that were never timelocked (`add_guardian`, `freeze`, `pause`, ownership transfer) with immediate effect. Distributing the key itself is what closes that remaining gap.

## 2. The key technical fact this design leans on

`owner`/`admin` in this codebase are typed as plain `soroban_sdk::Address`, and every gate is a plain `some_address.require_auth()` call. Soroban's authorization model treats that call identically regardless of whether `some_address` is a classic Ed25519 account or a **contract address implementing `CustomAccountInterface`** (exactly what `contracts/smart_account` itself already is, via `stellar_accounts::smart_account::do_check_auth`). Verified directly against the crate source (`stellar-access-0.7.2/src/ownable/storage.rs`):

```rust
pub fn enforce_owner_auth(e: &Env) -> Address {
    let Some(owner) = get_owner(e) else { panic_with_error!(e, OwnableError::OwnerNotSet) };
    owner.require_auth();
    owner
}
```

Nothing here inspects *what kind* of address `owner` is. **This means distributing the owner/admin role does not require touching `contracts/smart_account` or `contracts/recovery_manager` at all.** It requires deploying a separate small contract that itself enforces N-of-M authorization, and pointing `owner`/`admin` at *that contract's address* instead of a bare keypair. When `enforce_owner_auth`/`ensure_admin` calls `require_auth()` on it, the Soroban host dispatches to that contract's own `__check_auth`, which decides whether enough of the M signers actually authorized this specific call.

This is the same reasoning already used elsewhere in this workspace (`docs/V1_SCOPE.md` §1): compose an existing, audited authorization primitive rather than build one inline.

## 3. Two ways to build the N-of-M contract

### Option A (recommended): a minimal governance account, reusing the same OZ composition `contracts/smart_account` already uses

A new, small contract — call it `contracts/governance_account` — that composes exactly the same OpenZeppelin pieces `contracts/smart_account` does for its own signer authorization, and nothing else:

```rust
#[contractimpl(contracttrait)]
impl OzSmartAccountTrait for GovernanceAccount {}   // context rules, signer registry, do_check_auth

#[contractimpl(contracttrait)]
impl ExecutionEntryPoint for GovernanceAccount {}    // optional: lets the multisig itself call arbitrary functions

#[contractimpl]
impl CustomAccountInterface for GovernanceAccount {
    // one-line delegation to do_check_auth, identical to contracts/smart_account
}
```

No `Ownable`, no `Pausable`, no treasury logic — this contract's only job is to decide "did enough of my registered signers authorize this specific call," using `stellar_accounts::policies::simple_threshold` (a plain N-of-M count, not `weighted_threshold`) attached to its context rule. `simple_threshold` is the better conceptual fit here than `weighted_threshold` — governance approval is naturally "any N of these M people," not a weighted vote — **but it does not avoid the "Signer Set Divergence" risk**: checked directly against the crate source (`stellar-accounts-0.7.2/src/policies/simple_threshold.rs`), `simple_threshold` carries the *identical* documented caveat as `weighted_threshold` (threshold is not automatically revalidated when signers are added/removed from the context rule — can silently under- or over-weaken the effective quorum). This is not a reason to avoid it; every practical multisig primitive has some version of this property. It just means whoever operates the governance account must follow the same "review/update the threshold before or after any signer-set change" runbook discipline `docs/V1_SCOPE.md` already calls out for `weighted_threshold`.

**Why this is the recommended option:** it is almost entirely OZ trait composition, the same code already integrated, tested, and reasoned about in `contracts/smart_account`. The genuinely new surface is close to zero — no new authorization logic to design or review, just a second, narrower deployment of code already in this workspace.

**Trade-offs:**
- One more contract to deploy, initialize, and operationally manage (a signer set to maintain, guardians/governance participants to onboard and offboard).
- Every governance call now costs one extra cross-contract hop (`smart_account`/`recovery_manager` → `governance_account.require_auth()` → `do_check_auth`) — negligible for infrequent admin operations, irrelevant for resource budgets at this call frequency.
- Carries the signer-set-divergence operational caveat described above regardless of `simple_threshold` vs. `weighted_threshold` — an operational runbook item, not a reason to prefer Option B.

### Option B: a minimal, purpose-built N-of-M multisig contract

A small (~60-100 line), STA-authored contract with its own signer list and threshold, implementing `CustomAccountInterface` directly rather than composing OZ's `SmartAccount`. Simpler in the sense of "less machinery" (no context rules, no policy attachment, no passkey/WebAuthn path) — just: store a `Vec<Address>` of governance signers and a threshold `u32`; `__check_auth` requires `require_auth_for_args` from at least `threshold` of them for the signature payload.

**Trade-offs:**
- This *is* new custom-authorization code — the exact category this project's stated philosophy (`docs/V1_SCOPE.md` §1, §2) has consistently avoided in favor of integrating OZ's already-audited primitives. It would need the same level of scrutiny `contracts/smart_account`'s own auth path got, from scratch.
- Smaller attack surface in lines of code, but a *new* one, not a reused one.
- Not recommended unless Option A's dependency on `simple_threshold`/`SmartAccount` is judged unacceptable for some reason (e.g. wanting to avoid any dependency on `stellar-accounts` for this specific contract).

## 4. What "wiring it in" looks like (still no changes to existing contracts)

1. Deploy the governance account (Option A) once, with its own signer set and a `simple_threshold` policy at whatever N-of-M the operator wants.
2. At `smart_account::initialize`, pass the governance account's address as `owner` instead of a single keypair's address.
3. At `recovery_manager::initialize`, pass the governance account's address (the same one, or optionally a *different* governance account with a different signer set — see open question below) as `admin`.
4. Nothing else changes. `propose_adapter_change`, `add_guardian`, `freeze`, etc. all continue to call `owner.require_auth()`/`admin.require_auth()` exactly as they do today — they simply now resolve to a multisig's `__check_auth` instead of a single key's signature.

## 5. Open questions to decide before implementing (not this document's job to answer)

- **Same governance account for both roles, or two?** Using one signer set for both `smart_account`'s owner and `recovery_manager`'s admin is simpler operationally, but means a single compromised quorum threatens both day-to-day configuration *and* the recovery path meant to route around a compromised owner — arguably undermining the separation `docs/TECHNICAL_ARCHITECTURE.md` (Architecture Decision 4) already establishes between spend/config authority and recovery authority. Using two distinct governance accounts (possibly with overlapping but not identical signer sets) preserves that separation at the cost of two sets of keys to manage.
- **Threshold and signer count.** Not this document's call — an operational/business decision per deployment, not a protocol one.
- **Whether `add_guardian`/`freeze`/`pause` should also gain the propose/apply timelock from §6**, now that the *caller* itself is harder to compromise. Worth revisiting once a decision on this document is made — the two mitigations (timelock, distributed authority) are complementary, not substitutes for each other.

## 6. Recommendation

Option A, with `simple_threshold`, deployed once as `contracts/governance_account` and used as both `smart_account`'s `owner` and `recovery_manager`'s `admin` unless the separation argument in §5 is judged to matter enough to justify two. This keeps the "integrate, don't build" posture consistent with the rest of the workspace, requires zero changes to already-reviewed contracts, and directly closes the "single key proposes every governance action" gap named in `docs/V1_SCOPE.md`'s "Not Yet Included in V1."

Next step, if this direction is approved: implement `contracts/governance_account` (small, mostly-composition, as sketched in §3), add it to the workspace, write its own test suite (mirroring `contracts/smart_account`'s signer/context-rule tests, scoped down to just the auth surface), and update `scripts/deploy_testnet.sh` to deploy it and wire `smart_account`/`recovery_manager` against it instead of a bare keypair.
