# Governance Multisig: Design Notes

**Status: implemented.** `contracts/governance_account` and `contracts/threshold_policy` exist, are tested (13 tests), and are ready to deploy as the `owner` of `smart_account` and/or the `admin` of `recovery_manager`/`policy_engine` in place of a bare keypair. This document originally scoped the second follow-up item from `docs/V1_SCOPE.md` §6's final review sign-off — distributing the admin/owner role itself across multiple keys — as a written plan; §7 below records the review that happened before implementation, including one option (classic Stellar multisig) that was seriously considered and deliberately rejected, and why.

## 1. Problem, precisely stated

Every owner-gated call on `smart_account` (`propose_adapter_change`, `freeze`, `pause`/`unpause`, `transfer_ownership`), every admin-gated call on `recovery_manager` (`propose_remove_guardian`, `propose_threshold_change`, `add_guardian`, `cancel_recovery`, …), and every admin-gated call on `policy_engine` (`set_asset_rule`, `set_recipient_allowed`, `set_operation_allowed`, `bump_version`) is authorized by a single `Address` — checked via `ownable::enforce_owner_auth`/`ensure_admin`, which is just `owner.require_auth()` / `admin.require_auth()`. In V1, that `Address` is a single Stellar keypair (a classic account or an `Ed25519`/passkey signer behind it). One compromised key can call any of these.

**Note (found reviewing this document's own consistency, `docs/TECHNICAL_ARCHITECTURE.md` §12.8):** `policy_engine`'s admin uses the exact same `ensure_admin`/`admin.require_auth()` shape as `recovery_manager`'s — pointing `governance_account` at it works identically, with zero code changes, same as everything else this document describes. It was missing from this paragraph and from §8's "next step" in an earlier version, despite the opening status line above already claiming it as in scope — corrected here and in §8 rather than left as a silent gap between what the document claims and what it actually walks through.

**Correction (found during the strict security review, see `docs/SECURITY_REVIEW_STRICT.md`):** an earlier version of this paragraph also listed `add_context_rule`/`add_signer` here. That was wrong. Those calls, and the rest of the `SmartAccount` trait's own admin surface (`update_context_rule_name`, `update_context_rule_valid_until`, `remove_context_rule`, `remove_signer`, `add_policy`, `remove_policy`), are gated by `e.current_contract_address().require_auth()` — `smart_account`'s *own* signer/context-rule system, completely independent of the `owner` field this document distributes. See §9.

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

**Implemented as:**
- `contracts/threshold_policy` — thin wrapper around `stellar_accounts::policies::simple_threshold`, deployable, multi-tenant (one instance serves any number of context rules across any number of `SmartAccount`-composed contracts). 7 tests.
- `contracts/governance_account` — composes OZ's `SmartAccount` exactly as sketched in §3's Option A, deliberately with no `Ownable`/`Pausable`/`ExecutionEntryPoint` (nothing here needs to *initiate* calls as itself). `initialize` mirrors `smart_account::initialize`'s own bootstrapping pattern (calls the raw `add_context_rule` storage primitive directly, since no signer exists yet to authorize through the trait entrypoint), and takes an explicit `caller: Address` requiring `caller.require_auth()` — an earlier revision took no address and required no authorization at all, letting anyone bootstrap a freshly deployed instance with signers of their own choosing. That requirement alone doesn't fully close front-running in a deploy-then-separately-initialize flow (an attacker can still race in with their own address as `caller`); deploy and initialize this contract atomically in one transaction, the way `contracts/account_factory` already does for the six contracts it deploys. 8 tests.

Not yet done, deliberately separate from this implementation step: actually deploying an instance and repointing `smart_account`'s `owner` / `recovery_manager`'s `admin` at it on the live testnet deployment. That's a real, consequential state change to the live treasury (once done, the bare CLI can no longer drive any owner/admin-gated call — see §7's dApp-integration discussion), and belongs in its own explicit, confirmed step, not bundled into landing the contract code.

## 7. Review before implementation: a third option was seriously considered, and rejected

A pre-implementation review raised **Option C: classic Stellar multisig** — configuring `owner`/`admin` as an ordinary Stellar account with multiple weighted signers via `set_options`, no Soroban contract at all. This is a real, first-class, battle-tested authorization primitive that avoids the entire `AuthPayload`/nested-auth-entry construction this workspace has repeatedly had to hand-build tooling for (see `docs/TESTNET_DEPLOYMENT.md` §6.4–§6.5 and `scripts/execute_demo_transfer_payment.py`'s docstring for exactly how nontrivial that construction is). Since this design's own scope is one undifferentiated N-of-M gate — no context rules, no policies, no passkeys needed for governance itself — classic multisig would satisfy it natively, for free, with zero new contract code.

**Rejected in favor of Option A, for one decisive reason: this system is meant to be driven by a dApp, and the dApp already speaks `AuthPayload`.** Cedric's dApp (`Smart-Treasury-Account-STA/dApp`) already implements the full `AuthPayload`/nested-signer construction for treasury payments and signer/policy writes (`src/lib/stellarClient.ts`) — connect wallet, build the payload, collect the nested authorization, submit. Pointing `owner`/`admin` at `governance_account` means governance actions reuse that *exact same* client code path, just aimed at a different contract address and a different target call — no new authorization system for the dApp to build. Choosing classic multisig instead would have meant asking the dApp to implement a **second, parallel** authorization flow (transaction-level multi-signature assembly, Stellar's low/medium/high threshold model) solely for governance actions — objectively *more* integration work, not less, despite classic multisig being simpler in isolation. Given the explicit goal that dApp integration stay easy rather than accumulate complexity, reusing the existing pattern outweighs adopting a simpler-in-a-vacuum primitive that would fragment it.

This also means the operational cost flagged in Option A's trade-offs (§3) — every governance action needing N signers to each produce a nested authorization entry — is not *new* tooling burden to build. It is the *same* tooling the dApp already has for payments, reused. That reframes the trade-off meaningfully in Option A's favor once the dApp-integration constraint is in view, versus treating it as pure added cost.

**The "no recovery path" gap, addressed as a deliberate operational decision, not left open:** `governance_account` has no equivalent to `recovery_manager` — if its own signer set or threshold is ever misconfigured into an unsatisfiable state, there is no code-level way to route around it (even `transfer_ownership` on whatever it governs needs *its own* `require_auth()` to succeed first). Building a second recovery subsystem for the governance layer itself was judged not worth the added complexity. The chosen mitigation is operational, matching how real-world multisigs are run on any chain: keep the threshold comfortably below the full signer count (e.g. 3-of-5, not 5-of-5), and keep enough signers registered that simultaneous loss is implausible. This is documented directly in `contracts/governance_account/src/lib.rs`'s module doc comment, not just here.

## 8. Next step

Deploy one `threshold_policy` instance and one `governance_account` instance to testnet, initialize the latter with a real signer set and threshold, and repoint `smart_account`'s `owner`, `recovery_manager`'s `admin`, **and `policy_engine`'s `admin`** at it — as an explicit, separately-confirmed action, since it changes how every owner/admin-gated call on the live treasury must be authorized from that point on. Leaving `policy_engine`'s admin as a bare keypair while distributing the other two would be a silent gap, not a deliberate scope boundary — nothing about it is harder or different in kind from `recovery_manager`'s admin.

## 9. Known limitation: this design does not protect signer/context-rule/policy administration

This is a deliberate scope boundary, recorded here so it is never mistaken for an oversight. `smart_account`'s `add_context_rule`, `add_signer`, `remove_context_rule`, `remove_signer`, `add_policy`, `remove_policy`, `update_context_rule_name`, and `update_context_rule_valid_until` are OZ trait defaults gated by `e.current_contract_address().require_auth()` — i.e. by *whichever signer(s) the affected context rule itself currently requires*, not by `owner`. Repointing `owner` at `governance_account` (§8) does nothing to these eight calls: they are, and remain, authorized entirely by `smart_account`'s own signer registry.

**Concretely:** today, whoever satisfies rule 0 (or any rule they belong to) can unilaterally add/remove signers on that rule, attach/detach policies, or create an entirely new context rule — including one that grants themselves broader authority — without any owner/governance involvement at all. A single compromised payment-signer key is a compromise of the treasury's *entire* authorization topology, not just that one rule's spending power.

**Why this is not being closed by adding an owner check here (rejected option, considered and declined):** the OZ-precedented fix is straightforward — override these eight methods the same way `pause`/`unpause` already override the OZ default (§ `contracts/smart_account/src/lib.rs`'s `OzPausable` impl), adding `ownable::enforce_owner_auth(e)` alongside the existing self-auth. It was deliberately **not** applied, because it is a breaking change to every already-integrated caller: `scripts/add_dev_test_signer.py`, `scripts/swap_context_rule_signer.py`, `scripts/dev_fix_rule_signers.py`, `scripts/set_intent_executor.py`, and any dApp code that calls these entrypoints directly would all need a second, owner-scoped authorization entry added to their transaction construction — directly against this project's standing priority that dApp integration stay easy, not accumulate complexity (§7).

**Operational mitigation until this is revisited:** treat every context rule's own signer set as equivalent in sensitivity to full topology control, not just to that rule's nominal spending power — because in practice it *is* full topology control. Concretely: never register a single-key, unpoliced context rule expecting its blast radius to be limited to "that rule's spending limit"; any signer in any rule can escalate to controlling all other rules. This mirrors the operational mitigation already accepted for `governance_account`'s own no-recovery-path gap in §7 — a named, deliberate residual risk rather than a silently accepted one.

**If this is revisited later:** the fix is fully scoped and precedented (defense-in-depth override adding `ownable::enforce_owner_auth`, mirroring `OzPausable`), it is simply gated on updating every caller listed above in the same change so nothing on testnet breaks silently.
