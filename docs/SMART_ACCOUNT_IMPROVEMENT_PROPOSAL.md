# Smart Account Improvement Proposal

This is a design review, not an implementation — nothing below has been built. It answers three things asked together: how to permanently close the class of bug that hit `smart_account` this week (a context rule accumulating signers with no policy, silently becoming unanimous), how to make deploying and configuring a new treasury genuinely self-service, and what would actually make this the most capable programmable account system on Stellar rather than a solid but conventional one.

## Where this starts from

Worth stating plainly, because it shapes every recommendation below: this is already unusually far along for a Soroban account system. Real passkey support (`webauthn_verifier`), real timelocked adapter/guardian changes, real guardian-quorum recovery separated from spend authority, real versioned policy pinning, real nonce replay protection, real exactly-once scheduled execution — all live, tested (120 tests, ~98% coverage), and now proven against actual testnet transactions, not just local mocks. The gaps identified here are specific and closeable, not signs of a shaky foundation.

## Priority 1 — Attach real threshold policies (closes the revoke bug permanently)

**The problem, precisely:** a context rule with no policy attached requires every listed signer to co-sign, unconditionally. That's not a bug — it's `stellar-accounts`' documented default — but nothing today stops a rule from silently accumulating signers past what that default can sanely support. That's exactly what happened this week: `add_signer` was used exactly as designed, and a 1-of-1 rule became an unnoticed 3-of-3.

**The fix already exists, unbuilt.** `stellar-accounts` 0.7.2 ships two ready policy modules we aren't using yet: `simple_threshold` (M-of-N, equal weight) and `weighted_threshold` (per-signer weight, for cases where one signer should outweigh another). Both expose `install`/`enforce`/`uninstall`/`get_threshold`/`set_threshold` as plain functions — composing them is a thin wrapper contract, the same pattern already used for every other piece of this system (this is not new engineering risk, it's the same shape as `transfer_adapter`, under 100 lines).

**Proposal:** two new small packages, `contracts/threshold_policy` (wraps `simple_threshold`) and, as a fast-follow if genuinely unequal voting power is ever wanted, `contracts/weighted_policy` (wraps `weighted_threshold`). Attach `threshold_policy` to every context rule that has, or might ever have, more than one signer — including rule 0 today. Once attached, a rule with N signers and threshold T requires any T of them, and — critically — `set_threshold` becomes the deliberate, explicit action an admin takes when changing the signer set, instead of the signer count silently redefining what "approved" means.

**The residual risk, inherited from OZ, not introduced by us:** the module's own documentation names this exact class of issue — a threshold set at install time doesn't auto-adjust when signers are later added or removed. Their own guidance: update the threshold explicitly, ideally in the same transaction, whenever the signer set changes. That discipline needs to live somewhere enforceable, not just in a doc comment — see the operational safeguard below.

**Operational safeguard, cheap, do this regardless of timeline:** `smart_account::add_signer`/`remove_signer` currently have no opinion about policy state at all. Worth adding a guard — e.g., reject `add_signer` on a rule that already has ≥1 signer and no policy attached, forcing a policy decision at the moment a rule stops being trivially single-signer, rather than after. This is a real Rust code change to `smart_account`, small and testable, and it's the single highest-leverage line item on this whole list relative to effort.

## Priority 2 — Self-service deployment: a factory, and the architectural fork it forces

**The ask:** let a user deploy and configure their own smart account easily, not by hand-running seven `stellar contract deploy` commands and wiring five pinned addresses correctly in order.

**The easy part:** an `account_factory` contract exposing `deploy_account(profile, initial_signers, ...) -> Address`, using Soroban's in-contract deployment API (`env.deployer()...deploy_v2(...)`) to instantiate and initialize a fresh `smart_account` in one transaction. Pair it with a small number of named profiles instead of asking a new user to hand-construct context rules and policies from scratch:

- **Solo** — one signer, no policy (today's default shape).
- **Multisig(N, M)** — M signers, `threshold_policy` attached with threshold N.
- **Passkey-first** — an `External` (WebAuthn) signer as primary, an `External` or `Delegated` recovery signer as fallback.

**The hard part, and the actual decision this proposal surfaces:** `policy_engine`, `intent_registry`, and `recovery_manager` are each currently a *1:1 pinned instance per treasury* — one `policy_engine` deployment per `smart_account`. A factory that deploys a full fresh set of all four contracts per user is correct but expensive (four deployments per account, four sets of WASM storage rent, harder to upgrade or audit at scale — every treasury runs its own private copy of the logic). The alternative is making those three contracts **multi-tenant**: one shared `policy_engine` instance serving every treasury, with `smart_account`'s address folded into its storage keys (`DataKey::Asset(smart_account, asset)` instead of `DataKey::Asset(asset)`, and so on for `intent_registry`/`recovery_manager`). That's the shape real account infrastructure elsewhere (Safe's factory + singleton pattern, ERC-4337's shared EntryPoint) actually uses, and it's what "the best programmable account on Stellar" implies — a platform other teams deploy onto, not a template every team forks.

This is a real migration, not a config change — every one of those three contracts' storage keys changes shape. I'd recommend committing to the multi-tenant model now, before more testnet treasuries exist on the 1:1 shape, rather than migrating later. Worth a decision from you specifically before any code gets written here, since it's the one item on this list that isn't purely additive to what exists.

## Priority 3 — Spending limits (a real differentiator, not just parity)

`stellar-accounts` also ships `spending_limit` — a rolling-window cap (e.g., "no more than X over any trailing 24h period"), separate from `policy_engine`'s flat per-transfer cap. The two are complementary: `policy_engine` stops a single oversized payment, `spending_limit` stops many small ones from adding up to a drain. This is a thin wrapper, same effort class as Priority 1's `threshold_policy`, and it's a genuinely useful capability most account systems (on any chain) don't offer as a first-class, composable policy.

## Priority 4 — Finish the governance decentralization already designed

`docs/GOVERNANCE_MULTISIG_DESIGN.md` already specifies how to remove the single-key admin/owner concentration (the same class of risk that made the deployer key so sensitive throughout this project's testing). It's designed, not implemented. Given how much operational weight that one key has carried this week alone, I'd move this up rather than treat it as a someday item — especially once Priority 2's factory means many independent treasuries exist, each still defaulting to single-key ownership unless this ships.

## Priority 5 — Session keys / scoped temporary signers (larger, real capability gap)

Not designed anywhere yet. The gap: every registered signer today has standing authority for whatever its context rule allows, indefinitely, until explicitly removed. A genuinely differentiated programmable account lets an owner grant a *time-boxed* or *spend-capped* signer — e.g., "this key can approve transfers up to 100 XLM/day, expiring in 7 days" — without touching the account's permanent signer set. Mechanically this is a new context rule with a `valid_until` (already supported) plus a dedicated spending-limit policy (Priority 3) plus a naming/discovery convention so a dApp can tell a session-scoped rule apart from a permanent one. This is the most speculative item here — worth scoping properly, not estimating from this document alone.

## Priority 6 — A real TypeScript SDK package

Right now the off-chain signing knowledge (the `AuthPayload` construction, the nested-signature mechanics) lives as prose in `docs/DAPP_INTEGRATION_SPEC.md` and as Python one-off scripts in this repo. Every integrator — Cedric's dApp included — currently has to re-derive or hand-roll this. Packaging it as a real `@sta/sdk` (or similar) npm package — `buildAuthPayload`, `signWithWallet`, `prepareTransferPayment`, etc. — turns "read the spec and get it right" into "call a function." This is the SDK layer `docs/TECHNICAL_ARCHITECTURE.md` already scopes as part of the target system; it just hasn't been extracted from the dApp's own `stellarClient.ts` into something reusable yet.

## Suggested sequencing

1. The `add_signer` operational safeguard (Priority 1's guard) — smallest change, closes the actual bug class immediately, do this regardless of what else gets prioritized.
2. `threshold_policy` contract + attach to rule 0 and rule 1 on the live testnet deployment — the real fix.
3. Decide the multi-tenant-vs-1:1 question for Priority 2 — this gates everything about the factory, so it needs a decision before implementation starts, not during.
4. `spending_limit` policy — cheap, same shape as step 2, ships alongside it easily.
5. Governance decentralization (Priority 4) and the factory (Priority 2, once step 3 is decided) — comparable effort, sequence based on which you need sooner.
6. TypeScript SDK (Priority 6) and session keys (Priority 5) — larger, worth their own scoping pass when you're ready for them specifically.

Nothing here has been implemented. Tell me which of these you want scoped into an actual implementation plan first.
