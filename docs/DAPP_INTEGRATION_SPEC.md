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

Eight contracts, composed rather than each reimplementing the same
primitives:

| Contract | Role | Auth model exposed to a client |
|---|---|---|
| `smart_account` | Treasury root. Owns funds indirectly via adapters, decides what payments/schedules are approved. | **Custom account** — see §5. Every fund-moving or schedule-creating call requires this. |
| `account_factory` | Deploys and wires a complete treasury (all six other contracts below except `webauthn_verifier`) in one call. | Plain `caller.require_auth()` — a root-level address requirement, not `smart_account`'s custom `AuthPayload`. See §12. |
| `policy_engine` | Asset/recipient/operation allowlists, amount caps, versioned policy state. | None for reads (`validate_policy`, `version` are permissionless). Admin-gated for writes — out of dApp scope for Tranche 2 (configured by the treasury operator via CLI/ops tooling, not the payment UI), except the one-time `admin` assignment at deployment — see §12. |
| `intent_registry` | Canonical scheduled-payment state, execution windows, exactly-once child execution. | Admin-gated writes (admin = `smart_account`, satisfied by sub-invocation — see §7). Executor-gated `mark_child_executed` (a plain relayer key — see §8). |
| `recovery_manager` | Guardian quorum, timelocked recovery/guardian administration. | Guardian *administration* (`add_guardian` during initial setup) is in scope — see §12. The operational recovery flows (`open_recovery`, `approve_recovery`, `finalize_recovery`) remain out of Tranche 2 scope; noted for completeness in §9. |
| `transfer_adapter` / `split_adapter` | Narrow, preauthorized SAC transfer execution. Never called directly by a client — only reachable through `smart_account`. | N/A — not a client integration point. |
| `webauthn_verifier` | Passkey/secp256r1 signature verification for `Signer::External`. | Out of scope here (see above). |

A client calls into `smart_account` (for anything that moves funds or
schedules a payment), `policy_engine` (for read-only pre-checks), and now
`account_factory` (for deploying a new treasury — §12). It never calls the
adapters or `intent_registry` directly, and does not call `policy_engine`
or `recovery_manager` for anything beyond §12's one-time deployment setup.

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
- **Contract addresses**: `sdk/packages/core/src/config.ts`'s `TESTNET`
  export is the live source of truth — do not hardcode addresses in
  application code, since a redeployment changes them (a client that
  deploys its *own* treasury per §12 gets its own address set back from
  `deploy_account` instead of using this shared one at all). Snapshot,
  the account_factory-deployed treasury from
  `docs/TESTNET_FACTORY_DEPLOYMENT.md` §13.2 (post finding-29 fix — see
  `docs/SECURITY_REVIEW_STRICT.md`):

  | Contract | Address |
  |---|---|
  | `smart_account` | `CD6GY4UUTNPW4TUV7LDL5SELN4BBHJG4KDDT3W6G23DY6XCGM75MULMQ` |
  | `account_factory` | `CAQQTRRYNXIQGFVNCTMTBJDXW3PN7O44KPT7GWCCE4FRKTOHDBCWGUZO` |
  | `policy_engine` | `CCOP7NRMST5K6TL7FBDMX25LDEPW3DSFBOGBIKVFIDNAAZY7GBMVP3M4` |
  | `intent_registry` | `CAFIATSIZQSBILZJWVT4PVDXPVITJHLP6LPAVKDRHCA7I7XPZSLTRPUS` |
  | `recovery_manager` | `CCHC4YKVYS3CAZUOUYWYTEMQ6TZDW75WB2BGENUC2CDWDX5RH7NMKZWU` |
  | `transfer_adapter` | `CBRYGIR3ORDW5LE6J7AVPSKRNTMRUYHD6FVPHQMJGPQLQ5FQUZ2U6GFH` |
  | `split_adapter` | `CBQA7UI7QN6RN4IZT7WPDHWTK2OO7J4FH2KMCVJGKMKVFGDURD63UQ7U` |

  The original 2026-07-23 hand-deployed treasury
  (`docs/TESTNET_DEPLOYMENT.md`) still exists and still runs the older,
  pre-fix code — do not point new integration work at it.

  Confirm against `sdk/packages/core/src/config.ts` before use — this
  table is a snapshot, that file is the live record.

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
this does not need §5's custom `AuthPayload` machinery at all — but it
does still need one explicit, signed authorization entry. **Correction to
this section, found by actually running this against live testnet (both
via the bare `stellar` CLI and via `@stellar/stellar-sdk ^14.5.0`) while
building the SDK**: an earlier revision of this section claimed
`prepareTransaction` auto-fills the nested `Executor` requirement as a
`SOROBAN_CREDENTIALS_SOURCE_ACCOUNT` entry. It does not. `SourceAccount`
auto-fill only covers a `require_auth()` at the ROOT of the invocation
tree; `mark_child_executed`'s `executor.require_auth()` is two levels
deep (`execute_scheduled_payment -> intent_registry.mark_child_executed
-> ensure_executor`), so both the CLI and the JS SDK reject it with
`Error(Auth, InvalidAction)` / "encountered authorization not tied to the
root contract invocation for an address" unless an explicit entry is
supplied. The relayer:

1. Holds one ordinary keypair, configured as `Executor`.
2. Sets that keypair as the transaction's **source account** (this part
   was always correct — it pays the fee and is independent of the
   authorization entry below).
3. Builds and simulates
   `smart_account.execute_scheduled_payment(intent_id, child_sequence)` as
   the operation.
4. Builds one explicit `SorobanAuthorizationEntry`, rooted directly at
   `intent_registry.mark_child_executed(intent_id, child_sequence)` (not
   at the outer `execute_scheduled_payment` call, since that's where the
   real `require_auth()` fires), with the relayer's own address as a
   classic (non-custom-account) credential. `@stellar/stellar-sdk`'s
   `authorizeEntry(entry, signer, validUntilLedgerSeq, networkPassphrase)`
   signs it the same way it signs §5's Entry B — no `AuthPayload`,
   `context_rule_ids`, or nested-digest construction involved, since the
   executor isn't a custom account. Attach this single entry to the
   operation before calling `prepareTransaction` (which preserves it,
   same as §5.3 step 7).
5. Signs the transaction envelope with the relayer's own key and submits
   through the standard `@stellar/stellar-sdk` RPC flow.

Reference implementation: `buildExecutorAuthEntry` /
`prepareRelayerExecution` in `sdk/packages/core/src/auth.ts` and
`payments.ts`, exercised end to end in
`sdk/examples/04-scheduled-payment-create-and-relayer-execute.ts`.

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

- `recovery_manager`'s **operational** recovery flows (`open_recovery`,
  `approve_recovery`, `finalize_recovery`) — not part of Tranche 2's dApp
  deliverable. `add_guardian` **during initial treasury setup** is now in
  scope — see §12.6 — this bullet covers the recovery/incident-response
  flows specifically, not guardian registration.
- `Signer::External` (passkey) construction — same `AuthPayload` mechanics
  as §5, different Entry B (a WebAuthn assertion verified through
  `webauthn_verifier` rather than `require_auth_for_args`).
- Policy *content* configuration writes (`policy_engine.set_asset_rule`,
  `set_recipient_allowed`, `set_operation_allowed`) remain operator/ops
  tooling, not the payment dApp — treasury *bootstrapping* itself
  (deploying the six contracts and assigning owner/admin/signer/guardian/
  executor roles) moved into scope as of §12 and is no longer covered by
  this bullet.
- Post-deployment governance actions on an *already-running* treasury
  (`propose_adapter_change`, adding/removing guardians after the initial
  setup wizard, the rest of the timelocked owner/admin surface) — see
  `docs/GOVERNANCE_MULTISIG_DESIGN.md`. §12 covers only the one-time setup
  path immediately after deployment.

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
  `live_approval_count`, `guardian_freeze_epoch`, `contract_name` (§9/§12 —
  reference only; corrected from an earlier revision of this document,
  which named a `guardian_freeze_requested` read that no longer exists —
  see `docs/SECURITY_REVIEW_STRICT.md`'s guardian-freeze-epoch redesign).
- **`account_factory`**: `get_wasm_hashes`, `contract_name` — see §12.

Any entrypoint not listed here that appears in a given contract's
`-- --help` output is a write path and requires the authorization model
described in §5, §7, §8, or §12 depending on which contract it's on.

## 12. Deploying and setting up a new treasury from the dApp

This is the treasury-creation feature: a dApp user deploys their own
`smart_account` (plus its five supporting contracts) and assigns its
roles, rather than connecting to one that already exists. Everything
below is additive to §1–§11 — the payment/scheduling/relayer flows for an
already-deployed treasury are unchanged.

### 12.1 The role model — what's fixed at deploy time vs. adjustable later

This is the load-bearing constraint the whole design has to respect, and
it comes directly from the contracts, not a policy choice this document
is making: **`policy_engine.admin` and `recovery_manager.admin` are set
once, at `initialize`, and there is no entrypoint on either contract that
ever changes them again.** Verified directly against source — neither
contract stores or exposes anything resembling `transfer_admin`.
`transfer_adapter`/`split_adapter`'s `admin` parameter is narrower still:
it gates their one `initialize` call and is never even persisted to
storage afterward, so it isn't a role at all past deployment. Only
`smart_account.owner` is genuinely mutable post-deploy, via OZ's
`Ownable` two-step `transfer_ownership`/`accept_ownership`.

| Role | Lives on | Set via | Changeable after deploy? |
|---|---|---|---|
| Owner | `smart_account.owner` | `caller` (Tier 1, §12.3) or `smart_account.initialize`'s `owner` (Tier 2, §12.4) | **Yes** — two-step `transfer_ownership`/`accept_ownership` |
| Policy admin | `policy_engine.admin` | Same `caller`/`admin` as above | **No** — permanent from `initialize` |
| Recovery admin | `recovery_manager.admin` | Same | **No** — permanent from `initialize` |
| Adapter bootstrap admin | `transfer_adapter`/`split_adapter`'s `admin` param | Same | N/A — not stored past `initialize`, irrelevant afterward |
| Payment signers | `smart_account` context rule `0` | `initial_signers`/`initial_policies` | **Yes** — owner-gated `add_signer`/`remove_signer`/`add_context_rule` etc. |
| Guardians | `recovery_manager`'s guardian set | Not set by `deploy_account` at all — added in a follow-up step, §12.6 | **Yes** — recovery-admin-gated `add_guardian`/`propose_remove_guardian`, ~1 day activation/removal delay |
| Executor (relayer) | `intent_registry.Executor` | `deploy_account`'s `executor` param (Tier 1) or `smart_account.initialize`'s `config.initial_executor` (Tier 2) | **Yes, but not trivially** — no `smart_account` passthrough exists; rotating it means a direct, fully custom-authorized call to `intent_registry.set_executor` — §12.7 |

The practical consequence for the UI: **the Policy Admin and Recovery
Admin fields need their own explicit, un-skippable confirmation step**,
worded to make clear this choice is permanent — not a "connected wallet"
default a user could click through without noticing. Everything else in
this table has a real recovery path if the initial choice turns out
wrong; these two do not.

### 12.2 Two setup tiers

`account_factory.deploy_account` forces a single `caller` address into
*all three* of Owner, Policy Admin, and Recovery Admin at once — it has
no parameters to set them independently (verified against
`contracts/account_factory/src/lib.rs`: every sub-contract `initialize`
call in `deploy_account` passes the same `caller`). That is a deliberate
simplification in the contract itself (see that file's own module doc
comment), not a bug, but it means true role separation at deploy time
needs a different, longer path. Offer both:

- **Tier 1 — Guided** (§12.3): one `account_factory.deploy_account` call,
  one signature. `caller` becomes Owner + Policy Admin + Recovery Admin
  together. Right default for an individual or a team comfortable with
  one key holding all three admin roles (a very reasonable choice —
  it's exactly what `docs/GOVERNANCE_MULTISIG_DESIGN.md` recommends
  pointing at a `governance_account` multisig address rather than a
  personal key, and `smart_account.owner` specifically can still move
  later via §12.6).
- **Tier 2 — Custom** (§12.4): the same six deploy-and-`initialize`
  sequence `scripts/deploy_testnet.sh` already performs by hand, run
  from the dApp instead — but with Owner, Policy Admin, and Recovery
  Admin collected as three independently editable addresses in the UI.
  Trades one signature for five or six (one per contract's own
  `initialize`, since without `account_factory`'s single entry point
  each call is its own transaction) in exchange for real separation of
  duties from day one — e.g. a compliance address holding Policy Admin,
  a distinct operations address holding Recovery Admin, and the actual
  treasury owner holding Owner alone.

Present this as a single up-front choice ("Quick setup" vs. "Custom role
assignment"), not a per-field toggle — mixing tiers mid-flow only adds
confusion, since Tier 1's whole value is collapsing six transactions into
one.

### 12.3 Tier 1 walkthrough

Using the SDK's `account_factory` client (`sdk/packages/core/src/state.ts`'s
`accountFactoryClient`) directly — `deploy_account`'s `caller.require_auth()`
is a plain, root-level address requirement, not `smart_account`'s custom
`AuthPayload`, so no `auth.ts` machinery is needed here, only an ordinary
wallet signature. `sdk/examples/05-deploy-treasury-via-factory.ts`
implements exactly this (with a raw `Keypair` in place of a connected
wallet's signer):

```ts
import { accountFactoryClient, TESTNET } from "sta-sdk";

const client = accountFactoryClient(TESTNET);
client.options.publicKey = caller;              // from the connected wallet
client.options.signTransaction = kit.signTransaction; // Stellar Wallets Kit

const tx = await client.deploy_account({
  caller,
  salt: randomSalt32Bytes(),
  initial_signers: [{ tag: "Delegated", values: [caller] }], // see §12.5 for multi-signer
  initial_policies: new Map(),
  guardian_threshold: 1,        // guardians themselves added in §12.6 -- this is only the number
  executor,                     // a dedicated relayer key, NOT a human wallet -- see §12.1's row
});
const sent = await tx.signAndSend();
const { smart_account, policy_engine, intent_registry, recovery_manager,
        transfer_adapter, split_adapter } = sent.result.unwrap();
```

Persist the returned `DeployedAccount` addresses client-side (this is the
treasury's own config now, distinct from the shared `TESTNET` snapshot in
§2) and immediately continue into §12.6 — a freshly deployed treasury has
policy rules that reject everything by default and zero guardians.

### 12.4 Tier 2 walkthrough

Same six calls `scripts/deploy_testnet.sh` makes, driven from the dApp
with per-role addresses collected up front (`policyAdmin`,
`recoveryAdmin`, `owner`, plus `initialSigners`/`guardianThreshold`/
`executor` as in Tier 1). No `account_factory` involved — each
`initialize` is deployed and called independently, so this needs the
generated bindings for each contract individually
(`sdk/generated/policy_engine`, `.../intent_registry`, etc. — already
part of this SDK's package set). Order matters (`intent_registry` is
deployed but left uninitialized until `smart_account.initialize`
bootstraps it via the invoker-shortcut, same as Tier 1 — see that
function's doc comment in `contracts/smart_account/src/lib.rs`):

1. Deploy + `policy_engine.initialize(admin: policyAdmin)`.
2. Deploy + `recovery_manager.initialize(admin: recoveryAdmin, guardian_threshold)`.
3. Deploy `intent_registry` (uninitialized).
4. Deploy `transfer_adapter`/`split_adapter` (their `admin` param can be
   anyone, including the deployer itself — it's write-once and unused
   afterward, per §12.1).
5. Deploy + `smart_account.initialize(owner, initial_signers, initial_policies, config: { policy_engine, intent_registry, recovery_manager, initial_adapters, initial_executor })`.

Each step is its own wallet-signed transaction; surface this transaction
count in the UI up front ("Custom setup: 5–6 approvals") so it isn't a
surprise partway through. A user abandoning the flow partway leaves
orphaned, harmlessly-inert deployed-but-unwired contracts — no funds are
ever at risk before step 5 completes, since nothing is funded yet.

### 12.5 Multi-signer / weighted-threshold signers at deploy time

Both tiers' `initial_signers`/`initial_policies` already support more
than "one wallet, full control" — this is existing contract flexibility
the deploy UI should expose directly, not a gap to design around:

- **Single signer** (default, shown above): one `Signer::Delegated`,
  `initial_policies` empty — any one registered signer can approve any
  payment.
- **N-of-M multisig**: multiple `Signer::Delegated` entries in
  `initial_signers`, plus an OZ `weighted_threshold` (or
  `simple_threshold`) policy in `initial_policies` keyed to the policy
  contract's address — the treasury requires that many independent
  wallet approvals (§5.4) before a payment's `AuthPayload` is considered
  satisfied. Model this in the UI as "add a signer" (repeatable) + "how
  many approvals are required" — the natural real-world mental model for
  a team treasury — rather than exposing raw context-rule/policy
  concepts to the end user.

Additional signers/context rules can also be added after deployment
(owner-gated `add_signer`/`add_context_rule`), so the deploy-time choice
here is a starting point, not a permanent ceiling.

### 12.6 Post-deploy setup wizard (both tiers converge here)

A freshly deployed treasury is unusable until this runs — `policy_engine`
starts with no asset/recipient/operation rules configured (every payment
attempt fails closed), and no guardians are registered regardless of
what `guardian_threshold` was set to.

1. **Configure policy rules.** `policy_engine.set_asset_rule`,
   `set_recipient_allowed`, `set_operation_allowed` — admin-gated, signed
   by whoever holds Policy Admin (§12.1). This is the one piece of §12
   that overlaps with policy *content* configuration, which §9 otherwise
   still treats as ops tooling — narrowly in scope here only because a
   treasury with zero rules configured cannot pay anyone at all.
2. **Add guardians.** `recovery_manager.add_guardian(address)` per
   guardian, admin-gated, signed by whoever holds Recovery Admin. Each
   added guardian has a ~1 day activation delay
   (`GUARDIAN_ACTIVATION_DELAY_LEDGERS`) before its approval counts
   toward `guardian_threshold` — present this as "add guardians now, they
   become active in about a day," not as a blocking step in the wizard.
3. **Transfer ownership, if the bootstrap `caller`/`owner` was meant to
   be temporary** (common in Tier 1: deploy with a throwaway/ops key,
   then hand the treasury to its real owner). `smart_account
   .transfer_ownership(new_owner, live_until_ledger)`, followed by the
   new owner's own `accept_ownership()` call — a genuine two-step,
   two-signature handoff (matches the OZ `Ownable` pattern used
   throughout this workspace; neither the Policy Admin nor Recovery Admin
   roles have an equivalent, per §12.1).
4. **Rotate the executor, if needed** — §12.7.

None of these four are required before the treasury can *receive* funds,
only before it can *pay out* (step 1) or be protected by guardians
(step 2) — a UI can let a user fund the treasury immediately after
deployment while nudging them to finish setup, rather than gating the
whole flow on all four completing.

### 12.7 Rotating the executor after deploy

Real friction worth designing around deliberately rather than
discovering late: `smart_account` has **no** entrypoint that forwards to
`intent_registry.set_executor` — every other cross-contract write this
document covers goes *through* `smart_account` (§7's
`create_scheduled_payment`, etc.), but executor rotation does not. A
transaction calling `intent_registry.set_executor(new_executor)` directly
still needs `smart_account`'s authorization (`intent_registry`'s admin is
`smart_account` itself — set at bootstrap, §1), and since `smart_account`
is not the *direct* caller of that transaction, the invoker-shortcut
that would otherwise satisfy `intent_registry`'s admin check for free
does not apply — this needs the full Entry A/Entry B `AuthPayload`
construction from §5,
just rooted at `intent_registry.set_executor` instead of a
`smart_account` payment entrypoint. This SDK's `auth.ts` already supports
an arbitrary root invocation for exactly this reason — no new tooling is
needed, only a different `rootInvocation`:

```ts
import { buildInvocation, buildSmartAccountAuthEntries } from "sta-sdk";

const rootInvocation = buildInvocation({
  contractId: net.contracts.intentRegistry,
  functionName: "set_executor",
  args: [Address.fromString(newExecutor).toScVal()],
});
const [entryA, entryB] = await buildSmartAccountAuthEntries({
  spec: smartAccountClient(net).spec,
  smartAccountId: net.contracts.smartAccount,
  rootInvocation,
  signerAddress,           // current owner/signer authorizing this
  sign,                    // wallet's signAuthEntry, or a Keypair
  networkPassphrase: net.networkPassphrase,
  signatureExpirationLedger,
});
// attach [entryA, entryB] to an intent_registry.set_executor(newExecutor)
// operation built against the generated intent_registry client, then
// simulate/sign the envelope/submit as usual (§6 steps 4–5).
```

Treat executor rotation as an owner-signed action in the UI (it needs a
registered `smart_account` signer's approval, same authorization weight
as a payment), not an admin action — the three admin roles in §12.1 have
no say over it at all.
