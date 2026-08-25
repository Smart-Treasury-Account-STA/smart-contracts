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








export interface ScheduledIntent {
  /**
 * The `transfer_adapter` address configured on `smart_account` at the
 * moment this intent was approved, pinned here for the same reason
 * `policy_version` is: reconfiguring the adapter after approval (via
 * `smart_account::propose_adapter_change`/`apply_adapter_change`) must not silently redirect an
 * already-approved scheduled payment through a different execution
 * path. Execution reads this field back rather than resolving the
 * treasury's *current* adapter configuration.
 */
adapter: string;
  amount: i128;
  asset: string;
  cancelled: boolean;
  destination: string;
  end_ledger: u32;
  /**
 * Cumulative count of successfully marked child executions.
 */
execution_count: u32;
  intent_id: Buffer;
  /**
 * Security review finding: `start_ledger`/`end_ledger`/`max_executions`
 * alone bound a *total* allowance and its outer window, but nothing
 * stopped the executor from consuming every remaining execution the
 * moment the window opened — fine for bounded batch execution (the
 * case this contract was originally built around), but wrong for a
 * genuinely recurring schedule (e.g. "$1,000/month for a year"),
 * where the whole point is spreading executions out over time, not
 * just capping their count. `0` preserves the original behavior
 * exactly (any unused `child_sequence` executable any time in the
 * window) — this is opt-in, not a behavior change for existing
 * callers. When set, `mark_child_executed` additionally requires
 * `child_sequence`'s own due ledger
 * (`start_ledger + interval_ledgers * (child_sequence - 1)`) to have
 * arrived.
 */
interval_ledgers: u32;
  /**
 * Total number of child executions this intent may ever authorize.
 * Bounds recurring automation instead of allowing unlimited replay
 * within an otherwise-valid execution window.
 */
max_executions: u32;
  /**
 * PolicyEngine version in effect when this intent was approved, pinned
 * at creation so a later policy version bump cannot silently change
 * which rules a previously-approved scheduled payment executes under
 * (see `docs/TECHNICAL_ARCHITECTURE.md` §13.2: "Policy version changes
 * cannot silently mutate previously created automation semantics").
 */
policy_version: u32;
  start_ledger: u32;
}



export const IntentRegistryError = {
  3000: {message:"AlreadyInitialized"},
  3001: {message:"NotInitialized"},
  3002: {message:"IntentAlreadyExists"},
  3003: {message:"IntentNotFound"},
  3004: {message:"InvalidAmount"},
  3005: {message:"InvalidWindow"},
  3006: {message:"IntentCancelled"},
  3007: {message:"ExecutionTooEarly"},
  3008: {message:"ExecutionExpired"},
  3009: {message:"ChildAlreadyExecuted"},
  3010: {message:"UnauthorizedExecutor"},
  3011: {message:"InvalidMaxExecutions"},
  3012: {message:"ExecutionLimitReached"},
  3013: {message:"InvalidChildSequence"}
}

export interface Client {
  /**
   * Construct and simulate a get_intent transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_intent: ({intent_id}: {intent_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<ScheduledIntent>>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({admin}: {admin: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a set_executor transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_executor: ({executor}: {executor: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a cancel_intent transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  cancel_intent: ({intent_id}: {intent_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a contract_name transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  contract_name: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a create_intent transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  create_intent: ({intent}: {intent: ScheduledIntent}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a extend_intent_ttl transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless TTL maintenance for named, already-existing intents
   * and their child execution records — extending TTL creates no
   * authority (§16.1), so no admin gate is needed. `Initialized`/`Admin`/
   * `Executor` live in instance storage (one shared TTL, refreshed
   * automatically by `ensure_initialized` on every call that reaches
   * it), so no separate bump is needed for them here.
   */
  extend_intent_ttl: ({intent_id, child_sequences}: {intent_id: Buffer, child_sequences: Array<u32>}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a is_child_executed transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_child_executed: ({intent_id, child_sequence}: {intent_id: Buffer, child_sequence: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<boolean>>>

  /**
   * Construct and simulate a mark_child_executed transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  mark_child_executed: ({intent_id, child_sequence}: {intent_id: Buffer, child_sequence: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

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
      new ContractSpec([ "AAAABQAAAAAAAAAAAAAAC0luaXRpYWxpemVkAAAAAAEAAAAEaW5pdAAAAAEAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAAC",
        "AAAABQAAAAAAAAAAAAAADUNoaWxkRXhlY3V0ZWQAAAAAAAABAAAABGV4ZWMAAAACAAAAAAAAAAlpbnRlbnRfaWQAAAAAAAPuAAAAIAAAAAEAAAAAAAAADmNoaWxkX3NlcXVlbmNlAAAAAAAEAAAAAAAAAAI=",
        "AAAABQAAAAAAAAAAAAAADUludGVudENyZWF0ZWQAAAAAAAABAAAABmludGVudAAAAAAAAQAAAAAAAAAJaW50ZW50X2lkAAAAAAAD7gAAACAAAAABAAAAAg==",
        "AAAAAQAAAAAAAAAAAAAAD1NjaGVkdWxlZEludGVudAAAAAAMAAAB0lRoZSBgdHJhbnNmZXJfYWRhcHRlcmAgYWRkcmVzcyBjb25maWd1cmVkIG9uIGBzbWFydF9hY2NvdW50YCBhdCB0aGUKbW9tZW50IHRoaXMgaW50ZW50IHdhcyBhcHByb3ZlZCwgcGlubmVkIGhlcmUgZm9yIHRoZSBzYW1lIHJlYXNvbgpgcG9saWN5X3ZlcnNpb25gIGlzOiByZWNvbmZpZ3VyaW5nIHRoZSBhZGFwdGVyIGFmdGVyIGFwcHJvdmFsICh2aWEKYHNtYXJ0X2FjY291bnQ6OnByb3Bvc2VfYWRhcHRlcl9jaGFuZ2VgL2BhcHBseV9hZGFwdGVyX2NoYW5nZWApIG11c3Qgbm90IHNpbGVudGx5IHJlZGlyZWN0IGFuCmFscmVhZHktYXBwcm92ZWQgc2NoZWR1bGVkIHBheW1lbnQgdGhyb3VnaCBhIGRpZmZlcmVudCBleGVjdXRpb24KcGF0aC4gRXhlY3V0aW9uIHJlYWRzIHRoaXMgZmllbGQgYmFjayByYXRoZXIgdGhhbiByZXNvbHZpbmcgdGhlCnRyZWFzdXJ5J3MgKmN1cnJlbnQqIGFkYXB0ZXIgY29uZmlndXJhdGlvbi4AAAAAAAdhZGFwdGVyAAAAABMAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAFYXNzZXQAAAAAAAATAAAAAAAAAAljYW5jZWxsZWQAAAAAAAABAAAAAAAAAAtkZXN0aW5hdGlvbgAAAAATAAAAAAAAAAplbmRfbGVkZ2VyAAAAAAAEAAAAOUN1bXVsYXRpdmUgY291bnQgb2Ygc3VjY2Vzc2Z1bGx5IG1hcmtlZCBjaGlsZCBleGVjdXRpb25zLgAAAAAAAA9leGVjdXRpb25fY291bnQAAAAABAAAAAAAAAAJaW50ZW50X2lkAAAAAAAD7gAAACAAAAM3U2VjdXJpdHkgcmV2aWV3IGZpbmRpbmc6IGBzdGFydF9sZWRnZXJgL2BlbmRfbGVkZ2VyYC9gbWF4X2V4ZWN1dGlvbnNgCmFsb25lIGJvdW5kIGEgKnRvdGFsKiBhbGxvd2FuY2UgYW5kIGl0cyBvdXRlciB3aW5kb3csIGJ1dCBub3RoaW5nCnN0b3BwZWQgdGhlIGV4ZWN1dG9yIGZyb20gY29uc3VtaW5nIGV2ZXJ5IHJlbWFpbmluZyBleGVjdXRpb24gdGhlCm1vbWVudCB0aGUgd2luZG93IG9wZW5lZCDigJQgZmluZSBmb3IgYm91bmRlZCBiYXRjaCBleGVjdXRpb24gKHRoZQpjYXNlIHRoaXMgY29udHJhY3Qgd2FzIG9yaWdpbmFsbHkgYnVpbHQgYXJvdW5kKSwgYnV0IHdyb25nIGZvciBhCmdlbnVpbmVseSByZWN1cnJpbmcgc2NoZWR1bGUgKGUuZy4gIiQxLDAwMC9tb250aCBmb3IgYSB5ZWFyIiksCndoZXJlIHRoZSB3aG9sZSBwb2ludCBpcyBzcHJlYWRpbmcgZXhlY3V0aW9ucyBvdXQgb3ZlciB0aW1lLCBub3QKanVzdCBjYXBwaW5nIHRoZWlyIGNvdW50LiBgMGAgcHJlc2VydmVzIHRoZSBvcmlnaW5hbCBiZWhhdmlvcgpleGFjdGx5IChhbnkgdW51c2VkIGBjaGlsZF9zZXF1ZW5jZWAgZXhlY3V0YWJsZSBhbnkgdGltZSBpbiB0aGUKd2luZG93KSDigJQgdGhpcyBpcyBvcHQtaW4sIG5vdCBhIGJlaGF2aW9yIGNoYW5nZSBmb3IgZXhpc3RpbmcKY2FsbGVycy4gV2hlbiBzZXQsIGBtYXJrX2NoaWxkX2V4ZWN1dGVkYCBhZGRpdGlvbmFsbHkgcmVxdWlyZXMKYGNoaWxkX3NlcXVlbmNlYCdzIG93biBkdWUgbGVkZ2VyCihgc3RhcnRfbGVkZ2VyICsgaW50ZXJ2YWxfbGVkZ2VycyAqIChjaGlsZF9zZXF1ZW5jZSAtIDEpYCkgdG8gaGF2ZQphcnJpdmVkLgAAAAAQaW50ZXJ2YWxfbGVkZ2VycwAAAAQAAACtVG90YWwgbnVtYmVyIG9mIGNoaWxkIGV4ZWN1dGlvbnMgdGhpcyBpbnRlbnQgbWF5IGV2ZXIgYXV0aG9yaXplLgpCb3VuZHMgcmVjdXJyaW5nIGF1dG9tYXRpb24gaW5zdGVhZCBvZiBhbGxvd2luZyB1bmxpbWl0ZWQgcmVwbGF5CndpdGhpbiBhbiBvdGhlcndpc2UtdmFsaWQgZXhlY3V0aW9uIHdpbmRvdy4AAAAAAAAObWF4X2V4ZWN1dGlvbnMAAAAAAAQAAAFRUG9saWN5RW5naW5lIHZlcnNpb24gaW4gZWZmZWN0IHdoZW4gdGhpcyBpbnRlbnQgd2FzIGFwcHJvdmVkLCBwaW5uZWQKYXQgY3JlYXRpb24gc28gYSBsYXRlciBwb2xpY3kgdmVyc2lvbiBidW1wIGNhbm5vdCBzaWxlbnRseSBjaGFuZ2UKd2hpY2ggcnVsZXMgYSBwcmV2aW91c2x5LWFwcHJvdmVkIHNjaGVkdWxlZCBwYXltZW50IGV4ZWN1dGVzIHVuZGVyCihzZWUgYGRvY3MvVEVDSE5JQ0FMX0FSQ0hJVEVDVFVSRS5tZGAgwqcxMy4yOiAiUG9saWN5IHZlcnNpb24gY2hhbmdlcwpjYW5ub3Qgc2lsZW50bHkgbXV0YXRlIHByZXZpb3VzbHkgY3JlYXRlZCBhdXRvbWF0aW9uIHNlbWFudGljcyIpLgAAAAAAAA5wb2xpY3lfdmVyc2lvbgAAAAAABAAAAAAAAAAMc3RhcnRfbGVkZ2VyAAAABA==",
        "AAAABQAAAAAAAAAAAAAAD0V4ZWN1dG9yVXBkYXRlZAAAAAABAAAAB2V4ZWNzZXQAAAAAAQAAAAAAAAAIZXhlY3V0b3IAAAATAAAAAQAAAAI=",
        "AAAABQAAAAAAAAAAAAAAD0ludGVudENhbmNlbGxlZAAAAAABAAAABmNhbmNlbAAAAAAAAQAAAAAAAAAJaW50ZW50X2lkAAAAAAAD7gAAACAAAAABAAAAAg==",
        "AAAAAAAAAAAAAAAKZ2V0X2ludGVudAAAAAAAAQAAAAAAAAAJaW50ZW50X2lkAAAAAAAD7gAAACAAAAABAAAD6QAAB9AAAAAPU2NoZWR1bGVkSW50ZW50AAAAB9AAAAATSW50ZW50UmVnaXN0cnlFcnJvcgA=",
        "AAAAAAAAAAAAAAAKaW5pdGlhbGl6ZQAAAAAAAQAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAQAAA+kAAAACAAAH0AAAABNJbnRlbnRSZWdpc3RyeUVycm9yAA==",
        "AAAABAAAAAAAAAAAAAAAE0ludGVudFJlZ2lzdHJ5RXJyb3IAAAAADgAAAAAAAAASQWxyZWFkeUluaXRpYWxpemVkAAAAAAu4AAAAAAAAAA5Ob3RJbml0aWFsaXplZAAAAAALuQAAAAAAAAATSW50ZW50QWxyZWFkeUV4aXN0cwAAAAu6AAAAAAAAAA5JbnRlbnROb3RGb3VuZAAAAAALuwAAAAAAAAANSW52YWxpZEFtb3VudAAAAAAAC7wAAAAAAAAADUludmFsaWRXaW5kb3cAAAAAAAu9AAAAAAAAAA9JbnRlbnRDYW5jZWxsZWQAAAALvgAAAAAAAAARRXhlY3V0aW9uVG9vRWFybHkAAAAAAAu/AAAAAAAAABBFeGVjdXRpb25FeHBpcmVkAAALwAAAAAAAAAAUQ2hpbGRBbHJlYWR5RXhlY3V0ZWQAAAvBAAAAAAAAABRVbmF1dGhvcml6ZWRFeGVjdXRvcgAAC8IAAAAAAAAAFEludmFsaWRNYXhFeGVjdXRpb25zAAALwwAAAAAAAAAVRXhlY3V0aW9uTGltaXRSZWFjaGVkAAAAAAALxAAAAAAAAAAUSW52YWxpZENoaWxkU2VxdWVuY2UAAAvF",
        "AAAAAAAAAAAAAAAMc2V0X2V4ZWN1dG9yAAAAAQAAAAAAAAAIZXhlY3V0b3IAAAATAAAAAQAAA+kAAAACAAAH0AAAABNJbnRlbnRSZWdpc3RyeUVycm9yAA==",
        "AAAAAAAAAAAAAAANY2FuY2VsX2ludGVudAAAAAAAAAEAAAAAAAAACWludGVudF9pZAAAAAAAA+4AAAAgAAAAAQAAA+kAAAACAAAH0AAAABNJbnRlbnRSZWdpc3RyeUVycm9yAA==",
        "AAAAAAAAAAAAAAANY29udHJhY3RfbmFtZQAAAAAAAAAAAAABAAAAEQ==",
        "AAAAAAAAAAAAAAANY3JlYXRlX2ludGVudAAAAAAAAAEAAAAAAAAABmludGVudAAAAAAH0AAAAA9TY2hlZHVsZWRJbnRlbnQAAAAAAQAAA+kAAAACAAAH0AAAABNJbnRlbnRSZWdpc3RyeUVycm9yAA==",
        "AAAAAAAAAXpQZXJtaXNzaW9ubGVzcyBUVEwgbWFpbnRlbmFuY2UgZm9yIG5hbWVkLCBhbHJlYWR5LWV4aXN0aW5nIGludGVudHMKYW5kIHRoZWlyIGNoaWxkIGV4ZWN1dGlvbiByZWNvcmRzIOKAlCBleHRlbmRpbmcgVFRMIGNyZWF0ZXMgbm8KYXV0aG9yaXR5ICjCpzE2LjEpLCBzbyBubyBhZG1pbiBnYXRlIGlzIG5lZWRlZC4gYEluaXRpYWxpemVkYC9gQWRtaW5gLwpgRXhlY3V0b3JgIGxpdmUgaW4gaW5zdGFuY2Ugc3RvcmFnZSAob25lIHNoYXJlZCBUVEwsIHJlZnJlc2hlZAphdXRvbWF0aWNhbGx5IGJ5IGBlbnN1cmVfaW5pdGlhbGl6ZWRgIG9uIGV2ZXJ5IGNhbGwgdGhhdCByZWFjaGVzCml0KSwgc28gbm8gc2VwYXJhdGUgYnVtcCBpcyBuZWVkZWQgZm9yIHRoZW0gaGVyZS4AAAAAABFleHRlbmRfaW50ZW50X3R0bAAAAAAAAAIAAAAAAAAACWludGVudF9pZAAAAAAAA+4AAAAgAAAAAAAAAA9jaGlsZF9zZXF1ZW5jZXMAAAAD6gAAAAQAAAAA",
        "AAAAAAAAAAAAAAARaXNfY2hpbGRfZXhlY3V0ZWQAAAAAAAACAAAAAAAAAAlpbnRlbnRfaWQAAAAAAAPuAAAAIAAAAAAAAAAOY2hpbGRfc2VxdWVuY2UAAAAAAAQAAAABAAAD6QAAAAEAAAfQAAAAE0ludGVudFJlZ2lzdHJ5RXJyb3IA",
        "AAAAAAAAAAAAAAATbWFya19jaGlsZF9leGVjdXRlZAAAAAACAAAAAAAAAAlpbnRlbnRfaWQAAAAAAAPuAAAAIAAAAAAAAAAOY2hpbGRfc2VxdWVuY2UAAAAAAAQAAAABAAAD6QAAAAIAAAfQAAAAE0ludGVudFJlZ2lzdHJ5RXJyb3IA" ]),
      options
    )
  }
  public readonly fromJSON = {
    get_intent: this.txFromJSON<Result<ScheduledIntent>>,
        initialize: this.txFromJSON<Result<void>>,
        set_executor: this.txFromJSON<Result<void>>,
        cancel_intent: this.txFromJSON<Result<void>>,
        contract_name: this.txFromJSON<string>,
        create_intent: this.txFromJSON<Result<void>>,
        extend_intent_ttl: this.txFromJSON<null>,
        is_child_executed: this.txFromJSON<Result<boolean>>,
        mark_child_executed: this.txFromJSON<Result<void>>
  }
}