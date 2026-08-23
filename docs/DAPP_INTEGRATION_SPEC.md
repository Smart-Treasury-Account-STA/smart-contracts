# dApp Integration Specification — Smart Treasury Account

## Purpose and audience

This document specifies how a client — the Tranche 2 testnet dApp, and by
extension the Tranche 2 scheduled-payment relayer — connects to the deployed
V1 Soroban contracts described in `contracts/README.md` and
`docs/TESTNET_DEPLOYMENT.md`. It assumes the reader is comfortable with
Stellar/Soroban transaction building, RPC simulation, and standard wallet
integration, and focuses on the one piece of this system that does not
follow standard patterns: `smart_account`'s authorization model.

It does not re-derive contract behavior already specified elsewhere —
`docs/SMART_CONTRACT_SPECIFICATION.md` and `docs/TECHNICAL_ARCHITECTURE.md`
own that. This document is the bridge between "the contracts exist and are
deployed" and "here is exactly what a client sends to them, and why."

Scope note: this spec covers `Signer::Delegated` (plain Ed25519 wallet
signers) end to end, since that is all Tranche 2's dApp deliverable calls
for ("no custom wallet or signature-verification protocol is built").
`Signer::External` (passkey/WebAuthn, via `webauthn_verifier`) uses the same
`AuthPayload`/context-rule mechanics described in §5 with a different
signature scheme; it is out of scope here and left for a later tranche.

## 1. System overview

Seven contracts, composed rather than each reimplementing the same
primitives:

| Contract | Role | Auth model exposed to a client |
|---|---|---|
| `smart_account` | Treasury root. Owns funds indirectly via adapters, decides what payments/schedules are approved. | **Custom account** — see §5. Every fund-moving or schedule-creating call requires this. |
| `policy_engine` | Asset/recipient/operation allowlists, amount caps, versioned policy state. | None for reads (`validate_policy`, `version` are permissionless). Admin-gated for writes — out of dApp scope for Tranche 2 (configured by the treasury operator via CLI/ops tooling, not the payment UI). |
| `intent_registry` | Canonical scheduled-payment state, execution windows, exactly-once child execution. | Admin-gated writes (admin = `smart_account`, satisfied by sub-invocation — see §7). Executor-gated `mark_child_executed` (a plain relayer key — see §8). |
| `recovery_manager` | Guardian quorum, timelocked recovery/guardian administration. | Out of Tranche 2 scope; noted for completeness in §9. |
| `transfer_adapter` / `split_adapter` | Narrow, preauthorized SAC transfer execution. Never called directly by a client — only reachable through `smart_account`. | N/A — not a client integration point. |
| `webauthn_verifier` | Passkey/secp256r1 signature verification for `Signer::External`. | Out of scope here (see above). |

A client only ever calls into `smart_account` (for anything that moves
funds or schedules a payment) and `policy_engine` (for read-only
pre-checks). It never calls the adapters or `intent_registry` directly.

## 2. Environment setup

- **RPC**: `https://soroban-testnet.stellar.org` (matches
  `docs/TESTNET_DEPLOYMENT.md` and `scripts/deploy_testnet.sh`).
- **Network passphrase**: `Test SDF Network ; September 2015`.
- **Transaction/XDR building**: `@stellar/stellar-sdk`. Pin an exact version
  at project start and verify import paths against that version — recent
  major versions moved Soroban RPC helpers into a `rpc` sub-module and
  changed some class names; do not assume an import path from an older
  tutorial still matches.
- **Wallet connectivity**: Stellar Wallets Kit
  (`@creit.tech/stellar-wallets-kit`), configured for Freighter and xBull on
  `TESTNET`. It provides a uniform `getAddress()` / `signTransaction()` /
  `signAuthEntry()` surface across both wallets — build against that
  interface, not each wallet's raw API, so adding a third wallet later is
  configuration, not new integration code.
- **Contract addresses**: read from `docs/TESTNET_DEPLOYMENT.md` §2 at
  build time (or from a config file generated from it) — do not hardcode
  addresses in application code, since a redeployment changes them. As of
  the 2026-07-23 deployment:

  | Contract | Address |
  |---|---|
  | `smart_account` | `CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS` |
  | `policy_engine` | `CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC` |
  | `intent_registry` | `CDHTNPBXUMPCKUJ76HQ767MDRD4IVRRH4H5DOF4JUOO36QKSV4GXFRMR` |
  | `recovery_manager` | `CALI5XJASA66LKZPF3ZF7HOLGOFUWZYIHB6SENXCZ5Y7QVT7UQKKR6UM` |
  | `transfer_adapter` | `CAX766XYR56WO7Y4HFOHYQUO5AIN5QLHAHKJ3DINXM2WUQY5UE7KGE26` |
  | `split_adapter` | `CAFTFU2E4MGZT6BLCVN2FAQB6GIBJRR5ICI7C7LZEMACBDUUTQKVHHV3` |

  Confirm against `docs/TESTNET_DEPLOYMENT.md` before use — this table is a
  snapshot, that document is the live record.

## 3. Wallet connection

Standard Stellar Wallets Kit flow: initialize the kit for `WalletNetwork.TESTNET`
with the Freighter and xBull modules enabled, let the user pick a wallet,
call `getAddress()` to obtain their public key (`G...`). That address is
what the dApp checks against the treasury's registered signers (§4) — it is
**not** automatically a treasury signer just because it connected. A wallet
connecting with an address that isn't registered under any context rule
should be treated as "connected, but not authorized to act on this
treasury" in the UI, not as an error.

## 4. Reading treasury state

All of the following are read-only (`--send=no` equivalent: simulate
without submitting) and require no authorization:

| What | Call | Notes |
|---|---|---|
| Overall status | `smart_account.status()` → `AccountStatus { initialized, paused, frozen, policy_version_hint }` | Check before offering any payment action — a paused or frozen treasury should disable the payment UI with an explanation, not let the user hit a rejected simulation. |
| Owner | `smart_account.get_owner()` → `Option<Address>` | |
| Context rule count | `smart_account.get_context_rules_count()` → `u32` | Rule IDs are `0..count`, not necessarily contiguous after removals — check existence, don't assume. |
| A specific context rule | `smart_account.get_context_rule(id)` → `ContextRule { id, context_type, name, signers, signer_ids, policies, policy_ids, valid_until }` | This is how the dApp discovers **which signers are authorized for which kind of call**, and is the basis for §5's `context_rule_ids` selection. `context_type` is `Default` (any call), `CallContract(Address)`, or `CreateContract(hash)`. |
| Current policy version | `policy_engine.version()` → `u32` | Compare against `status().policy_version_hint` for a fast staleness check; call this directly for the authoritative value before building a payment (see §6). |
| Nonce already used | `smart_account.is_nonce_used(nonce)` → `bool` | Useful defensively; nonce generation strategy is the client's own responsibility (§6). |

For the founding testnet deployment, context rule `0` is a `Default` rule
containing one signer, `Signer::Delegated(deployer)`, with no policies
attached at the rule level (policy enforcement for payments happens inside
`smart_account`'s own entrypoint logic via `policy_engine.validate_policy`,
not via a rule-level policy). A production treasury may configure
differently — read `get_context_rule` at runtime, never hardcode rule `0`
or a specific signer set.

## 5. The core integration problem: `smart_account`'s custom authorization

This is the part of the system that does not behave like a normal Stellar
account, and is the reason this document exists.

### 5.1 Why

`smart_account` implements Soroban's `CustomAccountInterface`
(`__check_auth`), composed from OpenZeppelin's `stellar-accounts` crate
(`do_check_auth`/`authenticate` in
`stellar-accounts-0.7.2/src/smart_account/storage.rs`). Any call that does
`env.current_contract_address().require_auth()` — which is every
fund-moving entrypoint (`execute_transfer_payment`, `execute_split_payment`)
and every schedule-creating entrypoint (`create_scheduled_payment`,
`cancel_scheduled_payment`) — needs the transaction to carry a
`SorobanAuthorizationEntry` whose `credentials.signature` is not a
signature at all, but a contract-defined `AuthPayload` struct:

```rust
pub struct AuthPayload {
    pub signers: Map<Signer, Bytes>,   // Signer::Delegated(addr) -> arbitrary bytes
    pub context_rule_ids: Vec<u32>,    // one entry per auth context, aligned by index
}

pub enum Signer {
    Delegated(Address),        // a plain wallet key — what Tranche 2 uses
    External(Address, Bytes),  // a passkey verifier + key — out of scope here
}
```

No wallet, and no version of `@stellar/stellar-sdk`, has a built-in helper
for this shape — it's specific to this contract's own composition. The
wallet's role is narrower than "sign the transaction": it signs one
specific derived value (§5.3), and the dApp assembles the rest by hand.

### 5.2 What actually gets checked

Reading `do_check_auth` directly (this is the ground truth, not a
paraphrase — verify against the pinned `stellar-accounts` version if it
ever bumps):

1. `signatures.context_rule_ids` must have one entry per auth context in
   the transaction (in practice: one, for the single top-level
   `smart_account` call being authorized).
2. For each context, the named context rule (`get_context_rule(id)`) is
   loaded and matched: its `context_type` must be `Default` or match the
   actual call, and (for rules with no attached policies, which is the
   founding testnet configuration) every one of the rule's `signers` must
   be present as a key in `AuthPayload.signers`.
3. An `auth_digest` is computed:
   `sha256(signature_payload.to_bytes() || context_rule_ids.to_xdr())`,
   where `signature_payload` is the standard Soroban-computed hash for
   this authorization entry (the same value any normal `Address`
   credential would sign). Binding `context_rule_ids` into the digest is
   deliberate — it stops a client from signing once and then swapping in a
   different, less-restrictive rule ID afterward.
4. For each `Signer::Delegated(addr)` present in `AuthPayload.signers`,
   `addr.require_auth_for_args((auth_digest,))` is called. **This is the
   only cryptographic check that happens.** The raw bytes stored against
   that signer in `AuthPayload.signers` are never inspected — put an empty
   `Bytes` there. All the real authorization work happens through this
   nested call.

So a client needs to produce **two** things per required `Signer::Delegated`
signer, not one signature:

- **Entry A** (once, for `smart_account` itself): the `AuthPayload`
  structure — no wallet interaction, the dApp constructs this from data
  already read in §4.
- **Entry B** (one per required signer): a standard classic-account
  authorization entry, for the nested
  `addr.require_auth_for_args((auth_digest,))` call — this is what the
  wallet actually signs.

### 5.3 Building Entry B — what the wallet signs

The nested call's authorized invocation, per Soroban's authorization-frame
rules (confirmed against `soroban-env-host`'s `auth.rs`: a
`require_auth_for_args` call's recorded `contract_address`/`function_name`
come from the *currently executing frame*, which at the point `authenticate()`
runs is `smart_account` itself, executing `__check_auth`):

```
function:  ContractFn(InvokeContractArgs {
             contract_address: <smart_account address>,
             function_name:    "__check_auth",
             args:             [ auth_digest as Bytes ],
           })
sub_invocations: []
```

`auth_digest` cannot be known before `signature_payload` is known, and
`signature_payload` is derived from Entry A's own `nonce` and
`signature_expiration_ledger` — so the build order is:

1. Choose `nonce` (random 62-bit int) and `signature_expiration_ledger`
   (current ledger + a short window, e.g. +100) for Entry A.
2. Compute Entry A's `signature_payload` the normal way (hash of the
   `HashIDPreimage::SorobanAuthorization` preimage over `smart_account`'s
   own root invocation — the actual `execute_transfer_payment`/etc. call).
3. Compute `context_rule_ids` (e.g. `[0]`) and its XDR bytes, then
   `auth_digest = sha256(signature_payload || context_rule_ids_xdr)`.
4. Build Entry B's invocation (above, using `auth_digest`), with its own
   `nonce`/`signature_expiration_ledger`.
5. Get the wallet to sign Entry B. **`@stellar/stellar-sdk` exposes
   `authorizeEntry(entry, signer, validUntilLedgerSeq, networkPassphrase)`
   for exactly this** — it computes Entry B's `signature_payload` and
   invokes `signer` with the resulting preimage. Pass a callback that
   forwards to the connected wallet's `signAuthEntry` (Stellar Wallets Kit
   exposes this uniformly across Freighter/xBull) rather than a raw
   `Keypair`, so the private key never touches the dApp:

   ```ts
   const signedEntryB = await authorizeEntry(
     entryB,
     async (preimage) => {
       const { signedAuthEntry } = await kit.signAuthEntry(
         preimage.toXDR('base64'),
         { address: connectedAddress, networkPassphrase: NETWORK_PASSPHRASE },
       );
       return Buffer.from(signedAuthEntry, 'base64');
     },
     expirationLedger,
     NETWORK_PASSPHRASE,
   );
   ```

   Verify this exact call shape against the pinned SDK version's
   documentation before shipping — `authorizeEntry`'s signature has been
   stable across recent majors, but confirm rather than assume.
6. Build Entry A directly (no signing call — assemble the XDR by hand,
   `AuthPayload` as above with the signer's `Bytes` value empty).
7. Attach `[entryA, entryB]` as the `auth` list on the operation, then run
   the normal `simulateTransaction`/`prepareTransaction` flow with these
   entries **already present** — `@stellar/stellar-sdk`'s assembly step
   preserves supplied auth entries rather than trying to auto-fill them,
   which matters here because auto-fill cannot discover Entry A's shape
   (it's contract-specific — see §5.1).

### 5.4 Multiple required signers

If a context rule lists more than one `Signer::Delegated` (a genuine
multisig/weighted-threshold configuration via OZ's `weighted_threshold`
policy), Entry A's `signers` map gets one key per required signer, and
Entry B becomes **one authorization entry per signer**, each independently
signed by that signer's own wallet, all authorizing the identical
`auth_digest` (since it only depends on Entry A's `nonce`/expiration/
`context_rule_ids`, not on which signer produced it). This is the natural
place a "pending approvals" UI belongs: collect entries from each connected
signer over time, attach all of them once the rule's threshold is met, then
proceed to §6.

## 6. Payment flow: prepare → simulate → approve → submit → track

This maps directly to Tranche 2 Deliverable 2's listed dApp screens.

1. **Prepare.** Build the unsigned operation:
   `smart_account.execute_transfer_payment(asset, destination, amount, nonce, expected_policy_version)`.
   - `nonce`: any not-yet-used `u64` for this account (an incrementing
     local counter reconciled against `is_nonce_used` is sufficient; nonces
     are shared across `execute_transfer_payment` and
     `execute_split_payment` — see `contracts/smart_account/src/lib.rs`'s
     `nonce_replay_protection_is_shared_across_transfer_and_split_operations`
     test).
   - `expected_policy_version`: read fresh via `policy_engine.version()`
     immediately before building — **do not cache this across a user
     session**. It is pinned into the transaction; if the operator changes
     policy between when the screen loaded and when the user approves, the
     stale value causes a clean rejection (`PolicyEngineError::VersionMismatch`,
     `#2006`) rather than executing under outdated rules. Treat that
     rejection as "policy changed, refresh and re-confirm with the user,"
     not as a generic error.

2. **Simulate (pre-check).** Before asking the user to approve anything,
   call `policy_engine.validate_policy(check)` read-only with the same
   `operation`/`asset`/`destination`/`amount`/`expected_version` the
   payment will use. This is permissionless and cheap, and lets the UI
   surface a rejection (`RecipientNotAllowed` #2004, `AmountAboveLimit`
   #2005, `AssetNotAllowed` #2003, `OperationNotAllowed` #2008) **before**
   spending a wallet interaction on it — this is the live-on-testnet
   behavior demonstrated in `docs/TESTNET_DEPLOYMENT.md` §6.1.

3. **Approve.** Build Entries A and B per §5, get Entry B signed by the
   connected wallet(s).

4. **Submit.** Run `prepareTransaction`/`simulateTransaction` with the auth
   entries attached (fee/resource estimation only at this point — the auth
   entries are not re-derived), sign the transaction envelope itself with
   the submitting account's key (this can be any funded account — it pays
   the network fee, and is independent of who authorized the `smart_account`
   call), then `sendTransaction` and poll `getTransaction` until a terminal
   status.

5. **Track.** On success, the transaction's contract events carry a typed
   `#[contractevent]` payload — for a transfer, `TransferPaid { asset,
   destination, amount, nonce }`. Match on this (or the corresponding
   `SplitPaid`/`ScheduledPaymentExecuted`) to drive the UI's "payment
   confirmed" state rather than re-deriving it from the raw result XDR. See
   §10 for the full event/error reference.

## 7. Scheduled payments (dApp side)

`create_scheduled_payment(intent)` and `cancel_scheduled_payment(intent_id)`
use the **identical** custom-authorization flow as §5/§6 — they call
`env.current_contract_address().require_auth()` the same way. Build Entries
A and B against the actual `create_scheduled_payment`/`cancel_scheduled_payment`
root invocation instead of `execute_transfer_payment`; everything else in
§5 is unchanged.

`ScheduledIntentArgs` fields the dApp sets directly: `intent_id` (client-
generated `BytesN<32>`, e.g. a random value — collisions are rejected by
`intent_registry` with `IntentAlreadyExists`, #3002), `asset`, `destination`,
`amount`, `start_ledger`, `end_ledger`, `max_executions`. Two fields the
dApp may set but which the contract **silently overwrites** on write —
don't rely on echoing them back for display before submission:
`policy_version` (set to `policy_engine.version()` at creation time) and
`adapter` (set to whatever `transfer_adapter` is currently configured).
Both are pinned at approval time specifically so a later policy or adapter
change can't silently alter an already-approved schedule — surface the
*resolved* values back to the user from the emitted `IntentCreated` event
or a follow-up `get_intent` read, not from what was submitted.

`start_ledger`/`end_ledger` define the execution window in raw ledger
sequence numbers, not timestamps — convert from a user-facing date/time
using the network's ~5s average ledger close time for display purposes
only; do not treat that conversion as exact when constructing the actual
window (build in slack).

## 8. Relayer integration notes (bridging to Deliverable 3)

Included here because the dApp and relayer share the same `intent_registry`
state and the mental model matters for both teams, even though the relayer
itself is a separate deliverable and a separate service.

The critical simplification: `execute_scheduled_payment(intent_id,
child_sequence)` on `smart_account` has **no** `require_auth()` of its own
— it is deliberately permissionless (the authorization decision already
happened at `create_scheduled_payment` time). The only authorization check
in the whole call path is inside `intent_registry.mark_child_executed`,
which requires the registry's configured `Executor` address
(`intent_registry.set_executor`, admin-gated, set once during ops setup —
not by the dApp) to satisfy `require_auth()`.

Because `Executor` is a **plain Stellar account**, not `smart_account`,
this does not need §5's custom `AuthPayload` machinery at all. The relayer:

1. Holds one ordinary keypair, configured as `Executor`.
2. Sets that keypair as the transaction's **source account**.
3. Builds and simulates
   `smart_account.execute_transfer_payment`... no —
   `smart_account.execute_scheduled_payment(intent_id, child_sequence)` as
   the operation.
4. `prepareTransaction` auto-fills the nested `Executor` requirement as a
   `SOROBAN_CREDENTIALS_SOURCE_ACCOUNT` entry (satisfied implicitly by the
   transaction envelope's own signature — no separate `AuthPayload` or
   nonce/digest construction needed, unlike §5).
5. Signs the transaction envelope with the relayer's own key and submits
   through the standard `@stellar/stellar-sdk` RPC flow.

`child_sequence` is the relayer's own choice per execution attempt, but not
an arbitrary one: sequences are 1-based (`0` is rejected outright with
`InvalidChildSequence`, #3013), and any not-yet-used value from `1` up is
otherwise valid *unless* the intent has a cadence configured
(`interval_ledgers > 0`), in which case `child_sequence` must also be `<=
max_executions` and its own computed due ledger
(`start_ledger + interval_ledgers * (child_sequence - 1)`) must have
arrived — `intent_registry.is_child_executed(intent_id,
child_sequence)` is a permissionless read to check before submitting. This
is what gives the "exactly once" guarantee Deliverable 3 describes: a retry
or duplicate submission with the **same** `child_sequence` is rejected with
`ChildAlreadyExecuted` (#3009) rather than re-paying, so idempotent retry
logic is "pick a new `child_sequence` only after confirming the previous
attempt genuinely failed on-chain," not "retry the same submission
blindly." `ExecutionTooEarly`/`ExecutionExpired` (#3007/#3008) and
`ExecutionLimitReached` (#3012) are the other rejection modes the relayer's
status-tracking logic should distinguish from a genuine failure — none of
them indicate a bug, they indicate the window or budget the treasury signer
already approved has been exhausted or not yet reached.

## 9. Explicitly out of scope for this document

- `recovery_manager` guardian/recovery flows (`open_recovery`,
  `approve_recovery`, `finalize_recovery`, guardian administration) —
  not part of Tranche 2's dApp deliverable.
- `Signer::External` (passkey) construction — same `AuthPayload` mechanics
  as §5, different Entry B (a WebAuthn assertion verified through
  `webauthn_verifier` rather than `require_auth_for_args`).
- Treasury bootstrapping (`smart_account.initialize`) and policy
  configuration writes (`policy_engine.set_asset_rule` etc.) — operator/ops
  tooling, not the payment dApp.
- Governance actions (`propose_adapter_change`, `add_guardian`, and the
  rest of the timelocked owner/admin surface) — see
  `docs/GOVERNANCE_MULTISIG_DESIGN.md`.

## 10. Reference: errors and events

### 10.1 `smart_account` errors (`SmartAccountTreasuryError`, 8000–8015)

| Code | Name | When |
|---|---|---|
| 8000 | AlreadyInitialized | — |
| 8001 | NotInitialized | — |
| 8002 | Paused | Treasury paused; disable payment actions in the UI ahead of this |
| 8003 | Frozen | Emergency freeze active; same treatment as Paused |
| 8004 | AdapterNotConfigured | No adapter wired for the operation — ops issue, not user-fixable |
| 8005 | NonceAlreadyUsed | Reuse a fresh nonce and retry |
| 8006 | InvalidAmount | Amount ≤ 0 |
| 8007 | RecipientAmountLengthMismatch | Split: mismatched array lengths — client bug |
| 8008 | EmptySplit | Split with zero recipients |
| 8013 | DuplicateRecipient | Same address twice in a split |
| 8009/8010 | Recovery-related | Out of scope (§9) |
| 8011 | Unauthorized | Guardian-only entrypoint called by a non-guardian |
| 8012 | GuardianFreezeNotRequested | Out of scope (§9) |
| 8014 | NoPendingAdapterChange | Ops/governance surface, not dApp |
| 8015 | AdapterChangeDelayNotElapsed | Ops/governance surface, not dApp |

### 10.2 `policy_engine` errors (`PolicyEngineError`, 2000–2008)

| Code | Name | When | UX |
|---|---|---|---|
| 2003 | AssetNotAllowed | Asset not on the allowlist | Reject before wallet approval (§6 step 2) |
| 2004 | RecipientNotAllowed | Destination not allowlisted | Same |
| 2005 | AmountAboveLimit | Above the configured per-transfer cap | Same |
| 2006 | VersionMismatch | `expected_policy_version` stale | Refresh version, re-confirm with user, don't silently retry |
| 2008 | OperationNotAllowed | `transfer`/`split` disabled entirely | Same as Paused treatment |
| 2002/2007 | InvalidAmount/InvalidVersion | Malformed input — client bug | |

### 10.3 `intent_registry` errors relevant to the relayer (`IntentRegistryError`, 3000–3012)

| Code | Name | Relayer treatment |
|---|---|---|
| 3006 | IntentCancelled | Stop retrying this intent entirely |
| 3007 | ExecutionTooEarly | Window not open yet — reschedule, not a failure |
| 3008 | ExecutionExpired | Window closed — stop, log, do not retry |
| 3009 | ChildAlreadyExecuted | This exact `child_sequence` was already consumed — pick a new one only if the prior attempt is confirmed failed, otherwise this is the idempotency guarantee working correctly |
| 3012 | ExecutionLimitReached | `max_executions` budget exhausted — stop |
| 3010 | UnauthorizedExecutor | Ops misconfiguration (wrong relayer key) — page, don't retry |

### 10.4 Events for status tracking

| Contract | Event | Topic | Fields |
|---|---|---|---|
| `smart_account` | `TransferPaid` | `pay_ok` | `asset, destination, amount, nonce` |
| `smart_account` | `SplitPaid` | `splt_ok` | `asset, recipient_count, nonce` |
| `smart_account` | `ScheduledPaymentExecuted` | `auto_ok` | `intent_id, child_sequence, asset, destination, amount` |
| `intent_registry` | `IntentCreated` | `intent` | `intent_id` |
| `intent_registry` | `IntentCancelled` | `cancel` | `intent_id` |
| `intent_registry` | `ChildExecuted` | `exec` | `intent_id, child_sequence` |
| `policy_engine` | `PolicyValidated` | `pol_ok` | `operation, asset, destination, amount, expected_version` |

Full struct definitions live in each contract's `lib.rs` (`#[contractevent]`
blocks) — treat this table as an index, not the source of truth.

## 11. Reference: read-only entrypoints by contract

Full, current API surface (verified live against the testnet deployment
via `stellar contract invoke --id <address> -- --help`):

- **`smart_account`**: `status`, `paused`, `get_owner`, `get_context_rule`,
  `get_context_rules_count`, `get_signer_id`, `get_policy_id`,
  `is_nonce_used`, `contract_name`.
- **`policy_engine`**: `version`, `validate_policy`, `contract_name`.
- **`intent_registry`**: `get_intent`, `is_child_executed`, `contract_name`.
- **`recovery_manager`**: `is_guardian`, `request_status`,
  `live_approval_count`, `guardian_freeze_requested`, `contract_name` (§9 —
  reference only).

Any entrypoint not listed here that appears in a given contract's
`-- --help` output is a write path and requires the authorization model
described in §5, §7, or §8 depending on which contract it's on.
