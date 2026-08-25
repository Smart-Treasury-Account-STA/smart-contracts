import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}








/**
 * Security review finding: recovery originally only ever replaced
 * `Ownable`'s `owner` — but day-to-day spend authority on `smart_account`
 * is the OZ context-rule/signer registry, a completely separate
 * authorization system `owner` does not gate at all (see
 * `docs/GOVERNANCE_MULTISIG_DESIGN.md` §9). A compromised *payment
 * signer* — the more likely operational compromise, not the owner key —
 * survived recovery entirely: the account unfroze, ownership changed, and
 * the same compromised signer could still call `execute_transfer_payment`
 * immediately afterward. `docs/TECHNICAL_ARCHITECTURE.md` §12.7 already
 * documented the intended behavior ("Old signers and sessions are
 * cleared. New signers are installed.") — this had never actually been
 * implemented. `replacement_signers`/`replacement_policies` carry what
 * `smart_account::apply_recovery` installs as the account's *only*
 * context rule once finalized, replacing every existing one — see that
 * function's doc comment for why replacing rather than merely adding is
 * nece
 */
export interface RecoveryRequest {
  approvers: Array<string>;
  cancelled: boolean;
  earliest_ledger: u32;
  finalized: boolean;
  replacement_owner: string;
  replacement_policies: Map<string, any>;
  replacement_signers: Array<Signer>;
  request_id: Buffer;
}






export const RecoveryManagerError = {
  4000: {message:"AlreadyInitialized"},
  4001: {message:"NotInitialized"},
  4002: {message:"InvalidThreshold"},
  4003: {message:"GuardianAlreadyExists"},
  4004: {message:"GuardianNotFound"},
  4005: {message:"RequestAlreadyExists"},
  4006: {message:"RequestNotFound"},
  4007: {message:"DuplicateApproval"},
  4008: {message:"BelowThreshold"},
  4009: {message:"TimelockActive"},
  4010: {message:"RequestCancelled"},
  4011: {message:"RequestFinalized"},
  4012: {message:"TooManyApprovers"},
  4013: {message:"DelayTooShort"},
  4014: {message:"GuardianNotYetActive"},
  4015: {message:"Unauthorized"},
  4016: {message:"NoPendingGuardianRemoval"},
  4017: {message:"NoPendingThresholdChange"},
  4018: {message:"GuardianChangeDelayNotElapsed"},
  4019: {message:"GuardianFreezeEpochOverflow"},
  4020: {message:"NoReplacementSigners"},
  4021: {message:"ThresholdWouldBecomeUnsatisfiable"}
}






/**
 * Represents different types of signers in the smart account system.
 */
export type Signer = {tag: "Delegated", values: readonly [string]} | {tag: "External", values: readonly [string, Buffer]};

export interface Client {
  /**
   * Construct and simulate a extend_ttl transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless TTL maintenance for named, already-existing guardians
   * and requests — extending TTL creates no authority (§16.1). Also
   * covers any pending threshold change and any pending removal for the
   * given guardians: those are only bumped otherwise at `propose_*` time,
   * so a proposal left unapplied long enough after its delay elapses
   * could archive before `apply_*` is ever called, silently losing an
   * already-authorized governance action rather than just delaying it
   * (independent security review finding). `bump_ttl` no-ops on keys
   * that don't exist, so calling this with no pending changes is safe.
   * `Initialized`/`Admin`/`GuardianThreshold`/`PendingGuardianThreshold`
   * live in instance storage (one shared TTL, refreshed automatically by
   * `ensure_initialized` on every call that reaches it), so a single
   * `extend_ttl` call below covers all of them together.
   */
  extend_ttl: ({guardians, request_ids}: {guardians: Array<string>, request_ids: Array<Buffer>}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({admin, guardian_threshold}: {admin: string, guardian_threshold: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a is_guardian transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_guardian: ({guardian}: {guardian: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a add_guardian transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Registers a guardian. The guardian is recorded immediately
   * (`is_guardian` reflects it right away), but its approval does not
   * count toward any threshold until `GUARDIAN_ACTIVATION_DELAY_LEDGERS`
   * have passed — see the constant's doc comment for why. Removing a
   * guardian, unlike adding one, is itself timelocked — see
   * `propose_remove_guardian`.
   */
  add_guardian: ({guardian}: {guardian: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a contract_name transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  contract_name: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a open_recovery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Opens a recovery request. Callable by `admin`/owner **or by any
   * currently-active guardian** — this is a deliberate fix, not the
   * original design: requiring the admin's own authorization to open a
   * recovery request means recovery is unusable in exactly the scenario
   * it exists for (the owner's key is lost or destroyed, not merely
   * "compromised but still controllable by its holder"). A guardian who
   * suspects the owner is compromised, or who is trying to help an owner
   * who has genuinely lost access, can start the process unassisted;
   * `approve_recovery`'s threshold and `finalize_recovery`'s timelock
   * are what actually gate the outcome, not who was allowed to open the
   * request.
   * 
   * `replacement_signers`/`replacement_policies` must describe a
   * satisfiable context rule (at least one signer or one policy) — see
   * `RecoveryRequest`'s doc comment for why recovery needs to carry
   * this at all, not just `replacement_owner`. Rejected fast, here, if
   * `replacement_signers` is empty, rather than letting a genuinely
   * unsatisfiable recovery r
   */
  open_recovery: ({caller, request_id, replacement_owner, replacement_signers, replacement_policies, earliest_ledger}: {caller: string, request_id: Buffer, replacement_owner: string, replacement_signers: Array<Signer>, replacement_policies: Map<string, any>, earliest_ledger: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a request_status transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  request_status: ({request_id}: {request_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<RecoveryRequest>>>

  /**
   * Construct and simulate a cancel_recovery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  cancel_recovery: ({request_id}: {request_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a approve_recovery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  approve_recovery: ({request_id, guardian}: {request_id: Buffer, guardian: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a finalize_recovery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  finalize_recovery: ({request_id}: {request_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<string>>>

  /**
   * Construct and simulate a live_approval_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Recomputes, from current guardian registrations, how many of this
   * request's historical approvers are still valid right now. This is
   * the number `finalize_recovery` actually checks against the
   * threshold — never the raw `approvers.len()`.
   */
  live_approval_count: ({request_id}: {request_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a guardian_freeze_epoch transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless view of the current freeze epoch — see
   * `request_guardian_freeze`'s doc comment for why this is a monotonic
   * counter rather than a consumable flag. `smart_account::
   * apply_guardian_freeze` compares this against the last epoch it
   * applied to decide whether there is a genuinely new, not-yet-applied
   * freeze request.
   */
  guardian_freeze_epoch: (options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a apply_guardian_removal transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Applies a pending guardian removal once its delay has elapsed.
   * Permissionless: the authorization decision (the admin proposing this
   * specific removal) already happened, so *when* an already-authorized
   * removal lands needs no further gate. Does not touch any open
   * request's stored `approvers` list — that history is preserved as an
   * audit trail — but the removed guardian's past approval stops
   * counting toward `live_approval_count` / `finalize_recovery` from
   * this point on, because both recompute against current guardian
   * registration rather than trusting the historical approval.
   */
  apply_guardian_removal: ({guardian}: {guardian: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a apply_threshold_change transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Applies a pending guardian threshold change once its delay has
   * elapsed. Permissionless: the authorization decision (the admin
   * proposing this specific change) already happened, so *when* an
   * already-authorized change lands needs no further gate. Doesn't call
   * `ensure_initialized` (there is nothing to permission-check), so it
   * explicitly extends the instance TTL itself on this write, the same
   * way `smart_account::apply_adapter_change` does for its own
   * instance-storage write.
   */
  apply_threshold_change: (options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a cancel_guardian_removal transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Cancels a pending guardian removal before it takes effect.
   * Admin-gated, letting a legitimate admin walk back an erroneous or
   * no-longer-wanted proposal before its delay elapses.
   */
  cancel_guardian_removal: ({guardian}: {guardian: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a cancel_threshold_change transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Cancels a pending guardian threshold change before it takes effect.
   * Admin-gated, letting a legitimate admin walk back an erroneous or
   * no-longer-wanted proposal before its delay elapses.
   */
  cancel_threshold_change: (options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a propose_remove_guardian transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Proposes removing a guardian. Does not take effect immediately —
   * see `apply_guardian_removal`. The guardian remains fully active
   * (still counted by `is_guardian`, `live_approval_count`, and able to
   * approve requests or request a freeze) until the proposal is applied.
   * Independent security review finding: immediate removal under
   * single-admin control meant a compromised admin could dismantle the
   * guardian set with no delay for anyone to notice.
   */
  propose_remove_guardian: ({guardian}: {guardian: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a request_guardian_freeze transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Real gap this session found: `docs/TECHNICAL_ARCHITECTURE.md` §12.7
   * documents the canonical recovery workflow as starting with "Guardian
   * ... triggers freeze" — but `smart_account::freeze()` was owner-only,
   * with no guardian-facing path at all. A single active guardian
   * requesting freeze is enough to raise the flag (unlike recovery
   * itself, which needs quorum + timelock to *replace* authority,
   * stopping active bleeding while recovery is still pending doesn't
   * need the same bar — the worst case is an unnecessary pause, not a
   * loss of funds or authority). `smart_account::apply_guardian_freeze`
   * pulls this epoch the same way `apply_recovery` pulls a finalized
   * request: permissionlessly, and this contract has no knowledge of, or
   * dependency on, the treasury that pulls it.
   * 
   * Stores a monotonically increasing epoch rather than a boolean flag.
   * Security review finding on an earlier revision of this function: a
   * plain `bool`, consumed (cleared) by a permissionless
   * `consume_guardian_freeze_request` call, let *any* third party
   */
  request_guardian_freeze: ({guardian}: {guardian: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a propose_threshold_change transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Proposes a new guardian threshold. Does not take effect immediately
   * — see `apply_threshold_change`. Once applied, it takes
   * effect for any request not yet finalized: `finalize_recovery` always
   * reads the threshold fresh rather than the value in effect when the
   * request was opened or approved.
   */
  propose_threshold_change: ({new_threshold}: {new_threshold: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions &
      Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy(null, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAABQAAAAAAAAAAAAAAC0luaXRpYWxpemVkAAAAAAEAAAAEaW5pdAAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAAAAAAAEmd1YXJkaWFuX3RocmVzaG9sZAAAAAAABAAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAADUd1YXJkaWFuQWRkZWQAAAAAAAABAAAABWd1YXJkAAAAAAAAAgAAAAAAAAAIZ3VhcmRpYW4AAAATAAAAAQAAAAAAAAAMYWN0aXZhdGVzX2F0AAAABAAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAADlJlY292ZXJ5T3BlbmVkAAAAAAABAAAABG9wZW4AAAABAAAAAAAAAApyZXF1ZXN0X2lkAAAAAAPuAAAAIAAAAAEAAAAC",
        "AAAAAQAABABTZWN1cml0eSByZXZpZXcgZmluZGluZzogcmVjb3Zlcnkgb3JpZ2luYWxseSBvbmx5IGV2ZXIgcmVwbGFjZWQKYE93bmFibGVgJ3MgYG93bmVyYCDigJQgYnV0IGRheS10by1kYXkgc3BlbmQgYXV0aG9yaXR5IG9uIGBzbWFydF9hY2NvdW50YAppcyB0aGUgT1ogY29udGV4dC1ydWxlL3NpZ25lciByZWdpc3RyeSwgYSBjb21wbGV0ZWx5IHNlcGFyYXRlCmF1dGhvcml6YXRpb24gc3lzdGVtIGBvd25lcmAgZG9lcyBub3QgZ2F0ZSBhdCBhbGwgKHNlZQpgZG9jcy9HT1ZFUk5BTkNFX01VTFRJU0lHX0RFU0lHTi5tZGAgwqc5KS4gQSBjb21wcm9taXNlZCAqcGF5bWVudApzaWduZXIqIOKAlCB0aGUgbW9yZSBsaWtlbHkgb3BlcmF0aW9uYWwgY29tcHJvbWlzZSwgbm90IHRoZSBvd25lciBrZXkg4oCUCnN1cnZpdmVkIHJlY292ZXJ5IGVudGlyZWx5OiB0aGUgYWNjb3VudCB1bmZyb3plLCBvd25lcnNoaXAgY2hhbmdlZCwgYW5kCnRoZSBzYW1lIGNvbXByb21pc2VkIHNpZ25lciBjb3VsZCBzdGlsbCBjYWxsIGBleGVjdXRlX3RyYW5zZmVyX3BheW1lbnRgCmltbWVkaWF0ZWx5IGFmdGVyd2FyZC4gYGRvY3MvVEVDSE5JQ0FMX0FSQ0hJVEVDVFVSRS5tZGAgwqcxMi43IGFscmVhZHkKZG9jdW1lbnRlZCB0aGUgaW50ZW5kZWQgYmVoYXZpb3IgKCJPbGQgc2lnbmVycyBhbmQgc2Vzc2lvbnMgYXJlCmNsZWFyZWQuIE5ldyBzaWduZXJzIGFyZSBpbnN0YWxsZWQuIikg4oCUIHRoaXMgaGFkIG5ldmVyIGFjdHVhbGx5IGJlZW4KaW1wbGVtZW50ZWQuIGByZXBsYWNlbWVudF9zaWduZXJzYC9gcmVwbGFjZW1lbnRfcG9saWNpZXNgIGNhcnJ5IHdoYXQKYHNtYXJ0X2FjY291bnQ6OmFwcGx5X3JlY292ZXJ5YCBpbnN0YWxscyBhcyB0aGUgYWNjb3VudCdzICpvbmx5Kgpjb250ZXh0IHJ1bGUgb25jZSBmaW5hbGl6ZWQsIHJlcGxhY2luZyBldmVyeSBleGlzdGluZyBvbmUg4oCUIHNlZSB0aGF0CmZ1bmN0aW9uJ3MgZG9jIGNvbW1lbnQgZm9yIHdoeSByZXBsYWNpbmcgcmF0aGVyIHRoYW4gbWVyZWx5IGFkZGluZyBpcwpuZWNlAAAAAAAAAA9SZWNvdmVyeVJlcXVlc3QAAAAACAAAAAAAAAAJYXBwcm92ZXJzAAAAAAAD6gAAABMAAAAAAAAACWNhbmNlbGxlZAAAAAAAAAEAAAAAAAAAD2VhcmxpZXN0X2xlZGdlcgAAAAAEAAAAAAAAAAlmaW5hbGl6ZWQAAAAAAAABAAAAAAAAABFyZXBsYWNlbWVudF9vd25lcgAAAAAAABMAAAAAAAAAFHJlcGxhY2VtZW50X3BvbGljaWVzAAAD7AAAABMAAAAAAAAAAAAAABNyZXBsYWNlbWVudF9zaWduZXJzAAAAA+oAAAfQAAAABlNpZ25lcgAAAAAAAAAAAApyZXF1ZXN0X2lkAAAAAAPuAAAAIA==",
        "AAAABQAAAAAAAAAAAAAAD0d1YXJkaWFuUmVtb3ZlZAAAAAABAAAAB3VuZ3VhcmQAAAAAAQAAAAAAAAAIZ3VhcmRpYW4AAAATAAAAAQAAAAI=",
        "AAAABQAAAAAAAAAAAAAAEFJlY292ZXJ5QXBwcm92ZWQAAAABAAAABGFwcHIAAAACAAAAAAAAAApyZXF1ZXN0X2lkAAAAAAPuAAAAIAAAAAEAAAAAAAAACGd1YXJkaWFuAAAAEwAAAAEAAAAC",
        "AAAABQAAAAAAAAAAAAAAEFRocmVzaG9sZENoYW5nZWQAAAABAAAABnRocmVzaAAAAAAAAQAAAAAAAAANbmV3X3RocmVzaG9sZAAAAAAAAAQAAAAAAAAAAg==",
        "AAAABQAAAAAAAAAAAAAAEVJlY292ZXJ5Q2FuY2VsbGVkAAAAAAAAAQAAAAZjYW5jZWwAAAAAAAEAAAAAAAAACnJlcXVlc3RfaWQAAAAAA+4AAAAgAAAAAQAAAAI=",
        "AAAABQAAAAAAAAAAAAAAEVJlY292ZXJ5RmluYWxpemVkAAAAAAAAAQAAAAVmaW5hbAAAAAAAAAIAAAAAAAAACnJlcXVlc3RfaWQAAAAAA+4AAAAgAAAAAQAAAAAAAAARcmVwbGFjZW1lbnRfb3duZXIAAAAAAAATAAAAAAAAAAI=",
        "AAAAAAAAA1pQZXJtaXNzaW9ubGVzcyBUVEwgbWFpbnRlbmFuY2UgZm9yIG5hbWVkLCBhbHJlYWR5LWV4aXN0aW5nIGd1YXJkaWFucwphbmQgcmVxdWVzdHMg4oCUIGV4dGVuZGluZyBUVEwgY3JlYXRlcyBubyBhdXRob3JpdHkgKMKnMTYuMSkuIEFsc28KY292ZXJzIGFueSBwZW5kaW5nIHRocmVzaG9sZCBjaGFuZ2UgYW5kIGFueSBwZW5kaW5nIHJlbW92YWwgZm9yIHRoZQpnaXZlbiBndWFyZGlhbnM6IHRob3NlIGFyZSBvbmx5IGJ1bXBlZCBvdGhlcndpc2UgYXQgYHByb3Bvc2VfKmAgdGltZSwKc28gYSBwcm9wb3NhbCBsZWZ0IHVuYXBwbGllZCBsb25nIGVub3VnaCBhZnRlciBpdHMgZGVsYXkgZWxhcHNlcwpjb3VsZCBhcmNoaXZlIGJlZm9yZSBgYXBwbHlfKmAgaXMgZXZlciBjYWxsZWQsIHNpbGVudGx5IGxvc2luZyBhbgphbHJlYWR5LWF1dGhvcml6ZWQgZ292ZXJuYW5jZSBhY3Rpb24gcmF0aGVyIHRoYW4ganVzdCBkZWxheWluZyBpdAooaW5kZXBlbmRlbnQgc2VjdXJpdHkgcmV2aWV3IGZpbmRpbmcpLiBgYnVtcF90dGxgIG5vLW9wcyBvbiBrZXlzCnRoYXQgZG9uJ3QgZXhpc3QsIHNvIGNhbGxpbmcgdGhpcyB3aXRoIG5vIHBlbmRpbmcgY2hhbmdlcyBpcyBzYWZlLgpgSW5pdGlhbGl6ZWRgL2BBZG1pbmAvYEd1YXJkaWFuVGhyZXNob2xkYC9gUGVuZGluZ0d1YXJkaWFuVGhyZXNob2xkYApsaXZlIGluIGluc3RhbmNlIHN0b3JhZ2UgKG9uZSBzaGFyZWQgVFRMLCByZWZyZXNoZWQgYXV0b21hdGljYWxseSBieQpgZW5zdXJlX2luaXRpYWxpemVkYCBvbiBldmVyeSBjYWxsIHRoYXQgcmVhY2hlcyBpdCksIHNvIGEgc2luZ2xlCmBleHRlbmRfdHRsYCBjYWxsIGJlbG93IGNvdmVycyBhbGwgb2YgdGhlbSB0b2dldGhlci4AAAAAAApleHRlbmRfdHRsAAAAAAACAAAAAAAAAAlndWFyZGlhbnMAAAAAAAPqAAAAEwAAAAAAAAALcmVxdWVzdF9pZHMAAAAD6gAAA+4AAAAgAAAAAA==",
        "AAAAAAAAAAAAAAAKaW5pdGlhbGl6ZQAAAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAABJndWFyZGlhbl90aHJlc2hvbGQAAAAAAAQAAAABAAAD6QAAAAIAAAfQAAAAFFJlY292ZXJ5TWFuYWdlckVycm9y",
        "AAAAAAAAAAAAAAALaXNfZ3VhcmRpYW4AAAAAAQAAAAAAAAAIZ3VhcmRpYW4AAAATAAAAAQAAAAE=",
        "AAAABAAAAAAAAAAAAAAAFFJlY292ZXJ5TWFuYWdlckVycm9yAAAAFgAAAAAAAAASQWxyZWFkeUluaXRpYWxpemVkAAAAAA+gAAAAAAAAAA5Ob3RJbml0aWFsaXplZAAAAAAPoQAAAAAAAAAQSW52YWxpZFRocmVzaG9sZAAAD6IAAAAAAAAAFUd1YXJkaWFuQWxyZWFkeUV4aXN0cwAAAAAAD6MAAAAAAAAAEEd1YXJkaWFuTm90Rm91bmQAAA+kAAAAAAAAABRSZXF1ZXN0QWxyZWFkeUV4aXN0cwAAD6UAAAAAAAAAD1JlcXVlc3ROb3RGb3VuZAAAAA+mAAAAAAAAABFEdXBsaWNhdGVBcHByb3ZhbAAAAAAAD6cAAAAAAAAADkJlbG93VGhyZXNob2xkAAAAAA+oAAAAAAAAAA5UaW1lbG9ja0FjdGl2ZQAAAAAPqQAAAAAAAAAQUmVxdWVzdENhbmNlbGxlZAAAD6oAAAAAAAAAEFJlcXVlc3RGaW5hbGl6ZWQAAA+rAAAAAAAAABBUb29NYW55QXBwcm92ZXJzAAAPrAAAAAAAAAANRGVsYXlUb29TaG9ydAAAAAAAD60AAAAAAAAAFEd1YXJkaWFuTm90WWV0QWN0aXZlAAAPrgAAAAAAAAAMVW5hdXRob3JpemVkAAAPrwAAAAAAAAAYTm9QZW5kaW5nR3VhcmRpYW5SZW1vdmFsAAAPsAAAAAAAAAAYTm9QZW5kaW5nVGhyZXNob2xkQ2hhbmdlAAAPsQAAAAAAAAAdR3VhcmRpYW5DaGFuZ2VEZWxheU5vdEVsYXBzZWQAAAAAAA+yAAAAAAAAABtHdWFyZGlhbkZyZWV6ZUVwb2NoT3ZlcmZsb3cAAAAPswAAAAAAAAAUTm9SZXBsYWNlbWVudFNpZ25lcnMAAA+0AAAAAAAAACFUaHJlc2hvbGRXb3VsZEJlY29tZVVuc2F0aXNmaWFibGUAAAAAAA+1",
        "AAAAAAAAAVlSZWdpc3RlcnMgYSBndWFyZGlhbi4gVGhlIGd1YXJkaWFuIGlzIHJlY29yZGVkIGltbWVkaWF0ZWx5CihgaXNfZ3VhcmRpYW5gIHJlZmxlY3RzIGl0IHJpZ2h0IGF3YXkpLCBidXQgaXRzIGFwcHJvdmFsIGRvZXMgbm90CmNvdW50IHRvd2FyZCBhbnkgdGhyZXNob2xkIHVudGlsIGBHVUFSRElBTl9BQ1RJVkFUSU9OX0RFTEFZX0xFREdFUlNgCmhhdmUgcGFzc2VkIOKAlCBzZWUgdGhlIGNvbnN0YW50J3MgZG9jIGNvbW1lbnQgZm9yIHdoeS4gUmVtb3ZpbmcgYQpndWFyZGlhbiwgdW5saWtlIGFkZGluZyBvbmUsIGlzIGl0c2VsZiB0aW1lbG9ja2VkIOKAlCBzZWUKYHByb3Bvc2VfcmVtb3ZlX2d1YXJkaWFuYC4AAAAAAAAMYWRkX2d1YXJkaWFuAAAAAQAAAAAAAAAIZ3VhcmRpYW4AAAATAAAAAQAAA+kAAAACAAAH0AAAABRSZWNvdmVyeU1hbmFnZXJFcnJvcg==",
        "AAAAAAAAAAAAAAANY29udHJhY3RfbmFtZQAAAAAAAAAAAAABAAAAEQ==",
        "AAAAAAAABABPcGVucyBhIHJlY292ZXJ5IHJlcXVlc3QuIENhbGxhYmxlIGJ5IGBhZG1pbmAvb3duZXIgKipvciBieSBhbnkKY3VycmVudGx5LWFjdGl2ZSBndWFyZGlhbioqIOKAlCB0aGlzIGlzIGEgZGVsaWJlcmF0ZSBmaXgsIG5vdCB0aGUKb3JpZ2luYWwgZGVzaWduOiByZXF1aXJpbmcgdGhlIGFkbWluJ3Mgb3duIGF1dGhvcml6YXRpb24gdG8gb3BlbiBhCnJlY292ZXJ5IHJlcXVlc3QgbWVhbnMgcmVjb3ZlcnkgaXMgdW51c2FibGUgaW4gZXhhY3RseSB0aGUgc2NlbmFyaW8KaXQgZXhpc3RzIGZvciAodGhlIG93bmVyJ3Mga2V5IGlzIGxvc3Qgb3IgZGVzdHJveWVkLCBub3QgbWVyZWx5CiJjb21wcm9taXNlZCBidXQgc3RpbGwgY29udHJvbGxhYmxlIGJ5IGl0cyBob2xkZXIiKS4gQSBndWFyZGlhbiB3aG8Kc3VzcGVjdHMgdGhlIG93bmVyIGlzIGNvbXByb21pc2VkLCBvciB3aG8gaXMgdHJ5aW5nIHRvIGhlbHAgYW4gb3duZXIKd2hvIGhhcyBnZW51aW5lbHkgbG9zdCBhY2Nlc3MsIGNhbiBzdGFydCB0aGUgcHJvY2VzcyB1bmFzc2lzdGVkOwpgYXBwcm92ZV9yZWNvdmVyeWAncyB0aHJlc2hvbGQgYW5kIGBmaW5hbGl6ZV9yZWNvdmVyeWAncyB0aW1lbG9jawphcmUgd2hhdCBhY3R1YWxseSBnYXRlIHRoZSBvdXRjb21lLCBub3Qgd2hvIHdhcyBhbGxvd2VkIHRvIG9wZW4gdGhlCnJlcXVlc3QuCgpgcmVwbGFjZW1lbnRfc2lnbmVyc2AvYHJlcGxhY2VtZW50X3BvbGljaWVzYCBtdXN0IGRlc2NyaWJlIGEKc2F0aXNmaWFibGUgY29udGV4dCBydWxlIChhdCBsZWFzdCBvbmUgc2lnbmVyIG9yIG9uZSBwb2xpY3kpIOKAlCBzZWUKYFJlY292ZXJ5UmVxdWVzdGAncyBkb2MgY29tbWVudCBmb3Igd2h5IHJlY292ZXJ5IG5lZWRzIHRvIGNhcnJ5CnRoaXMgYXQgYWxsLCBub3QganVzdCBgcmVwbGFjZW1lbnRfb3duZXJgLiBSZWplY3RlZCBmYXN0LCBoZXJlLCBpZgpgcmVwbGFjZW1lbnRfc2lnbmVyc2AgaXMgZW1wdHksIHJhdGhlciB0aGFuIGxldHRpbmcgYSBnZW51aW5lbHkKdW5zYXRpc2ZpYWJsZSByZWNvdmVyeSByAAAADW9wZW5fcmVjb3ZlcnkAAAAAAAAGAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAACnJlcXVlc3RfaWQAAAAAA+4AAAAgAAAAAAAAABFyZXBsYWNlbWVudF9vd25lcgAAAAAAABMAAAAAAAAAE3JlcGxhY2VtZW50X3NpZ25lcnMAAAAD6gAAB9AAAAAGU2lnbmVyAAAAAAAAAAAAFHJlcGxhY2VtZW50X3BvbGljaWVzAAAD7AAAABMAAAAAAAAAAAAAAA9lYXJsaWVzdF9sZWRnZXIAAAAABAAAAAEAAAPpAAAAAgAAB9AAAAAUUmVjb3ZlcnlNYW5hZ2VyRXJyb3I=",
        "AAAAAAAAAAAAAAAOcmVxdWVzdF9zdGF0dXMAAAAAAAEAAAAAAAAACnJlcXVlc3RfaWQAAAAAA+4AAAAgAAAAAQAAA+kAAAfQAAAAD1JlY292ZXJ5UmVxdWVzdAAAAAfQAAAAFFJlY292ZXJ5TWFuYWdlckVycm9y",
        "AAAAAAAAAAAAAAAPY2FuY2VsX3JlY292ZXJ5AAAAAAEAAAAAAAAACnJlcXVlc3RfaWQAAAAAA+4AAAAgAAAAAQAAA+kAAAACAAAH0AAAABRSZWNvdmVyeU1hbmFnZXJFcnJvcg==",
        "AAAABQAAAAAAAAAAAAAAF0d1YXJkaWFuRnJlZXplUmVxdWVzdGVkAAAAAAEAAAAHZ2ZyZWV6ZQAAAAABAAAAAAAAAAhndWFyZGlhbgAAABMAAAABAAAAAg==",
        "AAAABQAAAAAAAAAAAAAAF0d1YXJkaWFuUmVtb3ZhbFByb3Bvc2VkAAAAAAEAAAAIdW5ndWFyZHAAAAACAAAAAAAAAAhndWFyZGlhbgAAABMAAAABAAAAAAAAABBlZmZlY3RpdmVfbGVkZ2VyAAAABAAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAAF1RocmVzaG9sZENoYW5nZVByb3Bvc2VkAAAAAAEAAAAHdGhyZXNocAAAAAACAAAAAAAAAA1uZXdfdGhyZXNob2xkAAAAAAAABAAAAAAAAAAAAAAAEGVmZmVjdGl2ZV9sZWRnZXIAAAAEAAAAAAAAAAI=",
        "AAAAAAAAAAAAAAAQYXBwcm92ZV9yZWNvdmVyeQAAAAIAAAAAAAAACnJlcXVlc3RfaWQAAAAAA+4AAAAgAAAAAAAAAAhndWFyZGlhbgAAABMAAAABAAAD6QAAAAIAAAfQAAAAFFJlY292ZXJ5TWFuYWdlckVycm9y",
        "AAAABQAAAAAAAAAAAAAAGEd1YXJkaWFuUmVtb3ZhbENhbmNlbGxlZAAAAAEAAAAIdW5ndWFyZGMAAAABAAAAAAAAAAhndWFyZGlhbgAAABMAAAABAAAAAg==",
        "AAAABQAAAAAAAAAAAAAAGFRocmVzaG9sZENoYW5nZUNhbmNlbGxlZAAAAAEAAAAHdGhyZXNoYwAAAAAAAAAAAg==",
        "AAAAAAAAAAAAAAARZmluYWxpemVfcmVjb3ZlcnkAAAAAAAABAAAAAAAAAApyZXF1ZXN0X2lkAAAAAAPuAAAAIAAAAAEAAAPpAAAAEwAAB9AAAAAUUmVjb3ZlcnlNYW5hZ2VyRXJyb3I=",
        "AAAAAAAAAO1SZWNvbXB1dGVzLCBmcm9tIGN1cnJlbnQgZ3VhcmRpYW4gcmVnaXN0cmF0aW9ucywgaG93IG1hbnkgb2YgdGhpcwpyZXF1ZXN0J3MgaGlzdG9yaWNhbCBhcHByb3ZlcnMgYXJlIHN0aWxsIHZhbGlkIHJpZ2h0IG5vdy4gVGhpcyBpcwp0aGUgbnVtYmVyIGBmaW5hbGl6ZV9yZWNvdmVyeWAgYWN0dWFsbHkgY2hlY2tzIGFnYWluc3QgdGhlCnRocmVzaG9sZCDigJQgbmV2ZXIgdGhlIHJhdyBgYXBwcm92ZXJzLmxlbigpYC4AAAAAAAATbGl2ZV9hcHByb3ZhbF9jb3VudAAAAAABAAAAAAAAAApyZXF1ZXN0X2lkAAAAAAPuAAAAIAAAAAEAAAPpAAAABAAAB9AAAAAUUmVjb3ZlcnlNYW5hZ2VyRXJyb3I=",
        "AAAAAAAAAUZQZXJtaXNzaW9ubGVzcyB2aWV3IG9mIHRoZSBjdXJyZW50IGZyZWV6ZSBlcG9jaCDigJQgc2VlCmByZXF1ZXN0X2d1YXJkaWFuX2ZyZWV6ZWAncyBkb2MgY29tbWVudCBmb3Igd2h5IHRoaXMgaXMgYSBtb25vdG9uaWMKY291bnRlciByYXRoZXIgdGhhbiBhIGNvbnN1bWFibGUgZmxhZy4gYHNtYXJ0X2FjY291bnQ6OgphcHBseV9ndWFyZGlhbl9mcmVlemVgIGNvbXBhcmVzIHRoaXMgYWdhaW5zdCB0aGUgbGFzdCBlcG9jaCBpdAphcHBsaWVkIHRvIGRlY2lkZSB3aGV0aGVyIHRoZXJlIGlzIGEgZ2VudWluZWx5IG5ldywgbm90LXlldC1hcHBsaWVkCmZyZWV6ZSByZXF1ZXN0LgAAAAAAFWd1YXJkaWFuX2ZyZWV6ZV9lcG9jaAAAAAAAAAAAAAABAAAABA==",
        "AAAAAAAAAkRBcHBsaWVzIGEgcGVuZGluZyBndWFyZGlhbiByZW1vdmFsIG9uY2UgaXRzIGRlbGF5IGhhcyBlbGFwc2VkLgpQZXJtaXNzaW9ubGVzczogdGhlIGF1dGhvcml6YXRpb24gZGVjaXNpb24gKHRoZSBhZG1pbiBwcm9wb3NpbmcgdGhpcwpzcGVjaWZpYyByZW1vdmFsKSBhbHJlYWR5IGhhcHBlbmVkLCBzbyAqd2hlbiogYW4gYWxyZWFkeS1hdXRob3JpemVkCnJlbW92YWwgbGFuZHMgbmVlZHMgbm8gZnVydGhlciBnYXRlLiBEb2VzIG5vdCB0b3VjaCBhbnkgb3BlbgpyZXF1ZXN0J3Mgc3RvcmVkIGBhcHByb3ZlcnNgIGxpc3Qg4oCUIHRoYXQgaGlzdG9yeSBpcyBwcmVzZXJ2ZWQgYXMgYW4KYXVkaXQgdHJhaWwg4oCUIGJ1dCB0aGUgcmVtb3ZlZCBndWFyZGlhbidzIHBhc3QgYXBwcm92YWwgc3RvcHMKY291bnRpbmcgdG93YXJkIGBsaXZlX2FwcHJvdmFsX2NvdW50YCAvIGBmaW5hbGl6ZV9yZWNvdmVyeWAgZnJvbQp0aGlzIHBvaW50IG9uLCBiZWNhdXNlIGJvdGggcmVjb21wdXRlIGFnYWluc3QgY3VycmVudCBndWFyZGlhbgpyZWdpc3RyYXRpb24gcmF0aGVyIHRoYW4gdHJ1c3RpbmcgdGhlIGhpc3RvcmljYWwgYXBwcm92YWwuAAAAFmFwcGx5X2d1YXJkaWFuX3JlbW92YWwAAAAAAAEAAAAAAAAACGd1YXJkaWFuAAAAEwAAAAEAAAPpAAAAAgAAB9AAAAAUUmVjb3ZlcnlNYW5hZ2VyRXJyb3I=",
        "AAAAAAAAAdlBcHBsaWVzIGEgcGVuZGluZyBndWFyZGlhbiB0aHJlc2hvbGQgY2hhbmdlIG9uY2UgaXRzIGRlbGF5IGhhcwplbGFwc2VkLiBQZXJtaXNzaW9ubGVzczogdGhlIGF1dGhvcml6YXRpb24gZGVjaXNpb24gKHRoZSBhZG1pbgpwcm9wb3NpbmcgdGhpcyBzcGVjaWZpYyBjaGFuZ2UpIGFscmVhZHkgaGFwcGVuZWQsIHNvICp3aGVuKiBhbgphbHJlYWR5LWF1dGhvcml6ZWQgY2hhbmdlIGxhbmRzIG5lZWRzIG5vIGZ1cnRoZXIgZ2F0ZS4gRG9lc24ndCBjYWxsCmBlbnN1cmVfaW5pdGlhbGl6ZWRgICh0aGVyZSBpcyBub3RoaW5nIHRvIHBlcm1pc3Npb24tY2hlY2spLCBzbyBpdApleHBsaWNpdGx5IGV4dGVuZHMgdGhlIGluc3RhbmNlIFRUTCBpdHNlbGYgb24gdGhpcyB3cml0ZSwgdGhlIHNhbWUKd2F5IGBzbWFydF9hY2NvdW50OjphcHBseV9hZGFwdGVyX2NoYW5nZWAgZG9lcyBmb3IgaXRzIG93bgppbnN0YW5jZS1zdG9yYWdlIHdyaXRlLgAAAAAAABZhcHBseV90aHJlc2hvbGRfY2hhbmdlAAAAAAAAAAAAAQAAA+kAAAACAAAH0AAAABRSZWNvdmVyeU1hbmFnZXJFcnJvcg==",
        "AAAAAAAAALBDYW5jZWxzIGEgcGVuZGluZyBndWFyZGlhbiByZW1vdmFsIGJlZm9yZSBpdCB0YWtlcyBlZmZlY3QuCkFkbWluLWdhdGVkLCBsZXR0aW5nIGEgbGVnaXRpbWF0ZSBhZG1pbiB3YWxrIGJhY2sgYW4gZXJyb25lb3VzIG9yCm5vLWxvbmdlci13YW50ZWQgcHJvcG9zYWwgYmVmb3JlIGl0cyBkZWxheSBlbGFwc2VzLgAAABdjYW5jZWxfZ3VhcmRpYW5fcmVtb3ZhbAAAAAABAAAAAAAAAAhndWFyZGlhbgAAABMAAAABAAAD6QAAAAIAAAfQAAAAFFJlY292ZXJ5TWFuYWdlckVycm9y",
        "AAAAAAAAALlDYW5jZWxzIGEgcGVuZGluZyBndWFyZGlhbiB0aHJlc2hvbGQgY2hhbmdlIGJlZm9yZSBpdCB0YWtlcyBlZmZlY3QuCkFkbWluLWdhdGVkLCBsZXR0aW5nIGEgbGVnaXRpbWF0ZSBhZG1pbiB3YWxrIGJhY2sgYW4gZXJyb25lb3VzIG9yCm5vLWxvbmdlci13YW50ZWQgcHJvcG9zYWwgYmVmb3JlIGl0cyBkZWxheSBlbGFwc2VzLgAAAAAAABdjYW5jZWxfdGhyZXNob2xkX2NoYW5nZQAAAAAAAAAAAQAAA+kAAAACAAAH0AAAABRSZWNvdmVyeU1hbmFnZXJFcnJvcg==",
        "AAAAAAAAAbxQcm9wb3NlcyByZW1vdmluZyBhIGd1YXJkaWFuLiBEb2VzIG5vdCB0YWtlIGVmZmVjdCBpbW1lZGlhdGVseSDigJQKc2VlIGBhcHBseV9ndWFyZGlhbl9yZW1vdmFsYC4gVGhlIGd1YXJkaWFuIHJlbWFpbnMgZnVsbHkgYWN0aXZlCihzdGlsbCBjb3VudGVkIGJ5IGBpc19ndWFyZGlhbmAsIGBsaXZlX2FwcHJvdmFsX2NvdW50YCwgYW5kIGFibGUgdG8KYXBwcm92ZSByZXF1ZXN0cyBvciByZXF1ZXN0IGEgZnJlZXplKSB1bnRpbCB0aGUgcHJvcG9zYWwgaXMgYXBwbGllZC4KSW5kZXBlbmRlbnQgc2VjdXJpdHkgcmV2aWV3IGZpbmRpbmc6IGltbWVkaWF0ZSByZW1vdmFsIHVuZGVyCnNpbmdsZS1hZG1pbiBjb250cm9sIG1lYW50IGEgY29tcHJvbWlzZWQgYWRtaW4gY291bGQgZGlzbWFudGxlIHRoZQpndWFyZGlhbiBzZXQgd2l0aCBubyBkZWxheSBmb3IgYW55b25lIHRvIG5vdGljZS4AAAAXcHJvcG9zZV9yZW1vdmVfZ3VhcmRpYW4AAAAAAQAAAAAAAAAIZ3VhcmRpYW4AAAATAAAAAQAAA+kAAAACAAAH0AAAABRSZWNvdmVyeU1hbmFnZXJFcnJvcg==",
        "AAAAAAAABABSZWFsIGdhcCB0aGlzIHNlc3Npb24gZm91bmQ6IGBkb2NzL1RFQ0hOSUNBTF9BUkNISVRFQ1RVUkUubWRgIMKnMTIuNwpkb2N1bWVudHMgdGhlIGNhbm9uaWNhbCByZWNvdmVyeSB3b3JrZmxvdyBhcyBzdGFydGluZyB3aXRoICJHdWFyZGlhbgouLi4gdHJpZ2dlcnMgZnJlZXplIiDigJQgYnV0IGBzbWFydF9hY2NvdW50OjpmcmVlemUoKWAgd2FzIG93bmVyLW9ubHksCndpdGggbm8gZ3VhcmRpYW4tZmFjaW5nIHBhdGggYXQgYWxsLiBBIHNpbmdsZSBhY3RpdmUgZ3VhcmRpYW4KcmVxdWVzdGluZyBmcmVlemUgaXMgZW5vdWdoIHRvIHJhaXNlIHRoZSBmbGFnICh1bmxpa2UgcmVjb3ZlcnkKaXRzZWxmLCB3aGljaCBuZWVkcyBxdW9ydW0gKyB0aW1lbG9jayB0byAqcmVwbGFjZSogYXV0aG9yaXR5LApzdG9wcGluZyBhY3RpdmUgYmxlZWRpbmcgd2hpbGUgcmVjb3ZlcnkgaXMgc3RpbGwgcGVuZGluZyBkb2Vzbid0Cm5lZWQgdGhlIHNhbWUgYmFyIOKAlCB0aGUgd29yc3QgY2FzZSBpcyBhbiB1bm5lY2Vzc2FyeSBwYXVzZSwgbm90IGEKbG9zcyBvZiBmdW5kcyBvciBhdXRob3JpdHkpLiBgc21hcnRfYWNjb3VudDo6YXBwbHlfZ3VhcmRpYW5fZnJlZXplYApwdWxscyB0aGlzIGVwb2NoIHRoZSBzYW1lIHdheSBgYXBwbHlfcmVjb3ZlcnlgIHB1bGxzIGEgZmluYWxpemVkCnJlcXVlc3Q6IHBlcm1pc3Npb25sZXNzbHksIGFuZCB0aGlzIGNvbnRyYWN0IGhhcyBubyBrbm93bGVkZ2Ugb2YsIG9yCmRlcGVuZGVuY3kgb24sIHRoZSB0cmVhc3VyeSB0aGF0IHB1bGxzIGl0LgoKU3RvcmVzIGEgbW9ub3RvbmljYWxseSBpbmNyZWFzaW5nIGVwb2NoIHJhdGhlciB0aGFuIGEgYm9vbGVhbiBmbGFnLgpTZWN1cml0eSByZXZpZXcgZmluZGluZyBvbiBhbiBlYXJsaWVyIHJldmlzaW9uIG9mIHRoaXMgZnVuY3Rpb246IGEKcGxhaW4gYGJvb2xgLCBjb25zdW1lZCAoY2xlYXJlZCkgYnkgYSBwZXJtaXNzaW9ubGVzcwpgY29uc3VtZV9ndWFyZGlhbl9mcmVlemVfcmVxdWVzdGAgY2FsbCwgbGV0ICphbnkqIHRoaXJkIHBhcnR5AAAAF3JlcXVlc3RfZ3VhcmRpYW5fZnJlZXplAAAAAAEAAAAAAAAACGd1YXJkaWFuAAAAEwAAAAEAAAPpAAAAAgAAB9AAAAAUUmVjb3ZlcnlNYW5hZ2VyRXJyb3I=",
        "AAAAAAAAASRQcm9wb3NlcyBhIG5ldyBndWFyZGlhbiB0aHJlc2hvbGQuIERvZXMgbm90IHRha2UgZWZmZWN0IGltbWVkaWF0ZWx5CuKAlCBzZWUgYGFwcGx5X3RocmVzaG9sZF9jaGFuZ2VgLiBPbmNlIGFwcGxpZWQsIGl0IHRha2VzCmVmZmVjdCBmb3IgYW55IHJlcXVlc3Qgbm90IHlldCBmaW5hbGl6ZWQ6IGBmaW5hbGl6ZV9yZWNvdmVyeWAgYWx3YXlzCnJlYWRzIHRoZSB0aHJlc2hvbGQgZnJlc2ggcmF0aGVyIHRoYW4gdGhlIHZhbHVlIGluIGVmZmVjdCB3aGVuIHRoZQpyZXF1ZXN0IHdhcyBvcGVuZWQgb3IgYXBwcm92ZWQuAAAAGHByb3Bvc2VfdGhyZXNob2xkX2NoYW5nZQAAAAEAAAAAAAAADW5ld190aHJlc2hvbGQAAAAAAAAEAAAAAQAAA+kAAAACAAAH0AAAABRSZWNvdmVyeU1hbmFnZXJFcnJvcg==",
        "AAAAAgAAAEJSZXByZXNlbnRzIGRpZmZlcmVudCB0eXBlcyBvZiBzaWduZXJzIGluIHRoZSBzbWFydCBhY2NvdW50IHN5c3RlbS4AAAAAAAAAAAAGU2lnbmVyAAAAAAACAAAAAQAAAD1BIGRlbGVnYXRlZCBzaWduZXIgdGhhdCB1c2VzIGJ1aWx0LWluIHNpZ25hdHVyZSB2ZXJpZmljYXRpb24uAAAAAAAACURlbGVnYXRlZAAAAAAAAAEAAAATAAAAAQAAAHJBbiBleHRlcm5hbCBzaWduZXIgd2l0aCBjdXN0b20gdmVyaWZpY2F0aW9uIGxvZ2ljLgpDb250YWlucyB0aGUgdmVyaWZpZXIgY29udHJhY3QgYWRkcmVzcyBhbmQgdGhlIHB1YmxpYyBrZXkgZGF0YS4AAAAAAAhFeHRlcm5hbAAAAAIAAAATAAAADg==" ]),
      options
    )
  }
  public readonly fromJSON = {
    extend_ttl: this.txFromJSON<null>,
        initialize: this.txFromJSON<Result<void>>,
        is_guardian: this.txFromJSON<boolean>,
        add_guardian: this.txFromJSON<Result<void>>,
        contract_name: this.txFromJSON<string>,
        open_recovery: this.txFromJSON<Result<void>>,
        request_status: this.txFromJSON<Result<RecoveryRequest>>,
        cancel_recovery: this.txFromJSON<Result<void>>,
        approve_recovery: this.txFromJSON<Result<void>>,
        finalize_recovery: this.txFromJSON<Result<string>>,
        live_approval_count: this.txFromJSON<Result<u32>>,
        guardian_freeze_epoch: this.txFromJSON<u32>,
        apply_guardian_removal: this.txFromJSON<Result<void>>,
        apply_threshold_change: this.txFromJSON<Result<void>>,
        cancel_guardian_removal: this.txFromJSON<Result<void>>,
        cancel_threshold_change: this.txFromJSON<Result<void>>,
        propose_remove_guardian: this.txFromJSON<Result<void>>,
        request_guardian_freeze: this.txFromJSON<Result<void>>,
        propose_threshold_change: this.txFromJSON<Result<void>>
  }
}