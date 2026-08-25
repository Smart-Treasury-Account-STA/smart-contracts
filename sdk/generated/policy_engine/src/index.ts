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





export interface AssetRule {
  enabled: boolean;
  max_single_transfer: i128;
}


export interface PolicyCheck {
  amount: i128;
  asset: string;
  destination: string;
  expected_version: u32;
  operation: string;
}




export const PolicyEngineError = {
  2000: {message:"AlreadyInitialized"},
  2001: {message:"NotInitialized"},
  2002: {message:"InvalidAmount"},
  2003: {message:"AssetNotAllowed"},
  2004: {message:"RecipientNotAllowed"},
  2005: {message:"AmountAboveLimit"},
  2006: {message:"VersionMismatch"},
  2007: {message:"InvalidVersion"},
  2008: {message:"OperationNotAllowed"}
}




export interface Client {
  /**
   * Construct and simulate a version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  version: (options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a extend_ttl transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless TTL maintenance. `Initialized`/`Admin`/`Version` live
   * in instance storage (one shared TTL, refreshed automatically by
   * `ensure_initialized` on every call that reaches it — see that
   * function) and are covered by the single `extend_ttl` call below;
   * `assets`/`recipients`/`operations` are named, already-existing
   * per-entity persistent entries, extended individually the same way
   * they always were. Extending TTL does not create authority or alter
   * execution semantics (`docs/TECHNICAL_ARCHITECTURE.md` §16.1), so
   * this is intentionally open to any caller — an off-chain monitor can
   * refresh the entries it knows are still active without needing admin
   * keys.
   */
  extend_ttl: ({assets, recipients, operations}: {assets: Array<string>, recipients: Array<string>, operations: Array<string>}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({admin}: {admin: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a bump_version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  bump_version: ({next_version}: {next_version: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a contract_name transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  contract_name: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a set_asset_rule transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_asset_rule: ({asset, rule}: {asset: string, rule: AssetRule}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a validate_policy transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  validate_policy: ({check}: {check: PolicyCheck}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a set_operation_allowed transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Enables or disables an adapter-level operation (e.g. `transfer`,
   * `split`). An operation not explicitly enabled fails closed, so
   * deploying a new adapter never silently grants it spend authority.
   */
  set_operation_allowed: ({operation, allowed}: {operation: string, allowed: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a set_recipient_allowed transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_recipient_allowed: ({recipient, allowed}: {recipient: string, allowed: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

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
      new ContractSpec([ "AAAAAQAAAAAAAAAAAAAACUFzc2V0UnVsZQAAAAAAAAIAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAAAAAATbWF4X3NpbmdsZV90cmFuc2ZlcgAAAAAL",
        "AAAAAQAAAAAAAAAAAAAAC1BvbGljeUNoZWNrAAAAAAUAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAFYXNzZXQAAAAAAAATAAAAAAAAAAtkZXN0aW5hdGlvbgAAAAATAAAAAAAAABBleHBlY3RlZF92ZXJzaW9uAAAABAAAAAAAAAAJb3BlcmF0aW9uAAAAAAAAEQ==",
        "AAAABQAAAAAAAAAAAAAAC0luaXRpYWxpemVkAAAAAAEAAAAEaW5pdAAAAAEAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAAC",
        "AAAAAAAAAAAAAAAHdmVyc2lvbgAAAAAAAAAAAQAAA+kAAAAEAAAH0AAAABFQb2xpY3lFbmdpbmVFcnJvcgAAAA==",
        "AAAAAAAAAptQZXJtaXNzaW9ubGVzcyBUVEwgbWFpbnRlbmFuY2UuIGBJbml0aWFsaXplZGAvYEFkbWluYC9gVmVyc2lvbmAgbGl2ZQppbiBpbnN0YW5jZSBzdG9yYWdlIChvbmUgc2hhcmVkIFRUTCwgcmVmcmVzaGVkIGF1dG9tYXRpY2FsbHkgYnkKYGVuc3VyZV9pbml0aWFsaXplZGAgb24gZXZlcnkgY2FsbCB0aGF0IHJlYWNoZXMgaXQg4oCUIHNlZSB0aGF0CmZ1bmN0aW9uKSBhbmQgYXJlIGNvdmVyZWQgYnkgdGhlIHNpbmdsZSBgZXh0ZW5kX3R0bGAgY2FsbCBiZWxvdzsKYGFzc2V0c2AvYHJlY2lwaWVudHNgL2BvcGVyYXRpb25zYCBhcmUgbmFtZWQsIGFscmVhZHktZXhpc3RpbmcKcGVyLWVudGl0eSBwZXJzaXN0ZW50IGVudHJpZXMsIGV4dGVuZGVkIGluZGl2aWR1YWxseSB0aGUgc2FtZSB3YXkKdGhleSBhbHdheXMgd2VyZS4gRXh0ZW5kaW5nIFRUTCBkb2VzIG5vdCBjcmVhdGUgYXV0aG9yaXR5IG9yIGFsdGVyCmV4ZWN1dGlvbiBzZW1hbnRpY3MgKGBkb2NzL1RFQ0hOSUNBTF9BUkNISVRFQ1RVUkUubWRgIMKnMTYuMSksIHNvCnRoaXMgaXMgaW50ZW50aW9uYWxseSBvcGVuIHRvIGFueSBjYWxsZXIg4oCUIGFuIG9mZi1jaGFpbiBtb25pdG9yIGNhbgpyZWZyZXNoIHRoZSBlbnRyaWVzIGl0IGtub3dzIGFyZSBzdGlsbCBhY3RpdmUgd2l0aG91dCBuZWVkaW5nIGFkbWluCmtleXMuAAAAAApleHRlbmRfdHRsAAAAAAADAAAAAAAAAAZhc3NldHMAAAAAA+oAAAATAAAAAAAAAApyZWNpcGllbnRzAAAAAAPqAAAAEwAAAAAAAAAKb3BlcmF0aW9ucwAAAAAD6gAAABEAAAAA",
        "AAAAAAAAAAAAAAAKaW5pdGlhbGl6ZQAAAAAAAQAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAQAAA+kAAAACAAAH0AAAABFQb2xpY3lFbmdpbmVFcnJvcgAAAA==",
        "AAAABQAAAAAAAAAAAAAAD1BvbGljeVZhbGlkYXRlZAAAAAABAAAABnBvbF9vawAAAAAABQAAAAAAAAAJb3BlcmF0aW9uAAAAAAAAEQAAAAEAAAAAAAAABWFzc2V0AAAAAAAAEwAAAAAAAAAAAAAAC2Rlc3RpbmF0aW9uAAAAABMAAAAAAAAAAAAAAAZhbW91bnQAAAAAAAsAAAAAAAAAAAAAABBleHBlY3RlZF92ZXJzaW9uAAAABAAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAAEEFzc2V0UnVsZVVwZGF0ZWQAAAABAAAABWFzc2V0AAAAAAAAAQAAAAAAAAAFYXNzZXQAAAAAAAATAAAAAQAAAAI=",
        "AAAABAAAAAAAAAAAAAAAEVBvbGljeUVuZ2luZUVycm9yAAAAAAAACQAAAAAAAAASQWxyZWFkeUluaXRpYWxpemVkAAAAAAfQAAAAAAAAAA5Ob3RJbml0aWFsaXplZAAAAAAH0QAAAAAAAAANSW52YWxpZEFtb3VudAAAAAAAB9IAAAAAAAAAD0Fzc2V0Tm90QWxsb3dlZAAAAAfTAAAAAAAAABNSZWNpcGllbnROb3RBbGxvd2VkAAAAB9QAAAAAAAAAEEFtb3VudEFib3ZlTGltaXQAAAfVAAAAAAAAAA9WZXJzaW9uTWlzbWF0Y2gAAAAH1gAAAAAAAAAOSW52YWxpZFZlcnNpb24AAAAAB9cAAAAAAAAAE09wZXJhdGlvbk5vdEFsbG93ZWQAAAAH2A==",
        "AAAAAAAAAAAAAAAMYnVtcF92ZXJzaW9uAAAAAQAAAAAAAAAMbmV4dF92ZXJzaW9uAAAABAAAAAEAAAPpAAAAAgAAB9AAAAARUG9saWN5RW5naW5lRXJyb3IAAAA=",
        "AAAAAAAAAAAAAAANY29udHJhY3RfbmFtZQAAAAAAAAAAAAABAAAAEQ==",
        "AAAAAAAAAAAAAAAOc2V0X2Fzc2V0X3J1bGUAAAAAAAIAAAAAAAAABWFzc2V0AAAAAAAAEwAAAAAAAAAEcnVsZQAAB9AAAAAJQXNzZXRSdWxlAAAAAAAAAQAAA+kAAAACAAAH0AAAABFQb2xpY3lFbmdpbmVFcnJvcgAAAA==",
        "AAAABQAAAAAAAAAAAAAAE1BvbGljeVZlcnNpb25CdW1wZWQAAAAAAQAAAAZwb2xpY3kAAAAAAAEAAAAAAAAADG5leHRfdmVyc2lvbgAAAAQAAAAAAAAAAg==",
        "AAAAAAAAAAAAAAAPdmFsaWRhdGVfcG9saWN5AAAAAAEAAAAAAAAABWNoZWNrAAAAAAAH0AAAAAtQb2xpY3lDaGVjawAAAAABAAAD6QAAAAIAAAfQAAAAEVBvbGljeUVuZ2luZUVycm9yAAAA",
        "AAAABQAAAAAAAAAAAAAAF09wZXJhdGlvbkFsbG93ZWRVcGRhdGVkAAAAAAEAAAACb3AAAAAAAAIAAAAAAAAACW9wZXJhdGlvbgAAAAAAABEAAAABAAAAAAAAAAdhbGxvd2VkAAAAAAEAAAAAAAAAAg==",
        "AAAABQAAAAAAAAAAAAAAF1JlY2lwaWVudEFsbG93ZWRVcGRhdGVkAAAAAAEAAAAEcmNwdAAAAAIAAAAAAAAACXJlY2lwaWVudAAAAAAAABMAAAABAAAAAAAAAAdhbGxvd2VkAAAAAAEAAAAAAAAAAg==",
        "AAAAAAAAAMFFbmFibGVzIG9yIGRpc2FibGVzIGFuIGFkYXB0ZXItbGV2ZWwgb3BlcmF0aW9uIChlLmcuIGB0cmFuc2ZlcmAsCmBzcGxpdGApLiBBbiBvcGVyYXRpb24gbm90IGV4cGxpY2l0bHkgZW5hYmxlZCBmYWlscyBjbG9zZWQsIHNvCmRlcGxveWluZyBhIG5ldyBhZGFwdGVyIG5ldmVyIHNpbGVudGx5IGdyYW50cyBpdCBzcGVuZCBhdXRob3JpdHkuAAAAAAAAFXNldF9vcGVyYXRpb25fYWxsb3dlZAAAAAAAAAIAAAAAAAAACW9wZXJhdGlvbgAAAAAAABEAAAAAAAAAB2FsbG93ZWQAAAAAAQAAAAEAAAPpAAAAAgAAB9AAAAARUG9saWN5RW5naW5lRXJyb3IAAAA=",
        "AAAAAAAAAAAAAAAVc2V0X3JlY2lwaWVudF9hbGxvd2VkAAAAAAAAAgAAAAAAAAAJcmVjaXBpZW50AAAAAAAAEwAAAAAAAAAHYWxsb3dlZAAAAAABAAAAAQAAA+kAAAACAAAH0AAAABFQb2xpY3lFbmdpbmVFcnJvcgAAAA==" ]),
      options
    )
  }
  public readonly fromJSON = {
    version: this.txFromJSON<Result<u32>>,
        extend_ttl: this.txFromJSON<null>,
        initialize: this.txFromJSON<Result<void>>,
        bump_version: this.txFromJSON<Result<void>>,
        contract_name: this.txFromJSON<string>,
        set_asset_rule: this.txFromJSON<Result<void>>,
        validate_policy: this.txFromJSON<Result<void>>,
        set_operation_allowed: this.txFromJSON<Result<void>>,
        set_recipient_allowed: this.txFromJSON<Result<void>>
  }
}