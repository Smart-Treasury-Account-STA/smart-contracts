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





export interface WasmHashes {
  intent_registry: Buffer;
  policy_engine: Buffer;
  recovery_manager: Buffer;
  smart_account: Buffer;
  split_adapter: Buffer;
  transfer_adapter: Buffer;
}



export interface DeployedAccount {
  intent_registry: string;
  policy_engine: string;
  recovery_manager: string;
  smart_account: string;
  split_adapter: string;
  transfer_adapter: string;
}



export const AccountFactoryError = {
  10000: {message:"AlreadyInitialized"},
  10001: {message:"NotInitialized"}
}

/**
 * Represents different types of signers in the smart account system.
 */
export type Signer = {tag: "Delegated", values: readonly [string]} | {tag: "External", values: readonly [string, Buffer]};

export interface Client {
  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({admin, wasm_hashes}: {admin: string, wasm_hashes: WasmHashes}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a contract_name transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  contract_name: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a deploy_account transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Deploys and wires a complete, ready-to-use treasury stack in one
   * call. See the module doc comment for the authorization model and
   * what "ready to use" means (adapters already bound, no timelock;
   * `intent_registry` already initialized).
   * 
   * `salt` only needs to be unique per `caller` — sub-contract addresses
   * are derived from `sha256(caller || salt || per-contract tag)`, so one
   * caller-chosen salt is enough to place all six deployments
   * deterministically and collision-free against each other, and two
   * different callers can safely choose the identical raw salt value
   * with no cross-caller collision or squatting risk (see `sub_salt`'s
   * doc comment for the security review finding this closed).
   * 
   * `executor` is `intent_registry`'s scheduled-payment relayer address
   * (see `smart_account::initialize`'s doc comment on `initial_executor`
   * for why this needs to be explicit rather than defaulted). Pass
   * `caller` again if there is no separate relayer identity yet.
   */
  deploy_account: ({caller, salt, initial_signers, initial_policies, guardian_threshold, executor}: {caller: string, salt: Buffer, initial_signers: Array<Signer>, initial_policies: Map<string, any>, guardian_threshold: u32, executor: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<DeployedAccount>>>

  /**
   * Construct and simulate a get_wasm_hashes transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_wasm_hashes: (options?: MethodOptions) => Promise<AssembledTransaction<Result<WasmHashes>>>

  /**
   * Construct and simulate a set_wasm_hashes transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Updates the WASM hashes used for every *future* `deploy_account`
   * call. Admin-gated and deliberately mutable — unlike `smart_account`'s
   * own pinned subordinate-module addresses (which protect one already
   * -deployed treasury's established trust), this only ever affects
   * treasuries deployed *after* the update; already-deployed treasuries
   * are unaffected, since their addresses were already fixed at their
   * own deploy time. Shared infrastructure code getting patched over
   * time is the ordinary case, not a residual risk to name.
   */
  set_wasm_hashes: ({wasm_hashes}: {wasm_hashes: WasmHashes}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a extend_instance_ttl transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless TTL maintenance — extending TTL creates no authority,
   * matching the reasoning already used by every other contract in this
   * workspace (`docs/TECHNICAL_ARCHITECTURE.md` §16.1).
   */
  extend_instance_ttl: (options?: MethodOptions) => Promise<AssembledTransaction<null>>

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
      new ContractSpec([ "AAAAAQAAAAAAAAAAAAAACldhc21IYXNoZXMAAAAAAAYAAAAAAAAAD2ludGVudF9yZWdpc3RyeQAAAAPuAAAAIAAAAAAAAAANcG9saWN5X2VuZ2luZQAAAAAAA+4AAAAgAAAAAAAAABByZWNvdmVyeV9tYW5hZ2VyAAAD7gAAACAAAAAAAAAADXNtYXJ0X2FjY291bnQAAAAAAAPuAAAAIAAAAAAAAAANc3BsaXRfYWRhcHRlcgAAAAAAA+4AAAAgAAAAAAAAABB0cmFuc2Zlcl9hZGFwdGVyAAAD7gAAACA=",
        "AAAABQAAAAAAAAAAAAAAC0luaXRpYWxpemVkAAAAAAEAAAAEaW5pdAAAAAEAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAAC",
        "AAAAAQAAAAAAAAAAAAAAD0RlcGxveWVkQWNjb3VudAAAAAAGAAAAAAAAAA9pbnRlbnRfcmVnaXN0cnkAAAAAEwAAAAAAAAANcG9saWN5X2VuZ2luZQAAAAAAABMAAAAAAAAAEHJlY292ZXJ5X21hbmFnZXIAAAATAAAAAAAAAA1zbWFydF9hY2NvdW50AAAAAAAAEwAAAAAAAAANc3BsaXRfYWRhcHRlcgAAAAAAABMAAAAAAAAAEHRyYW5zZmVyX2FkYXB0ZXIAAAAT",
        "AAAABQAAAAAAAAAAAAAAD0FjY291bnREZXBsb3llZAAAAAABAAAACGRlcGxveWVkAAAAAgAAAAAAAAAGY2FsbGVyAAAAAAATAAAAAQAAAAAAAAANc21hcnRfYWNjb3VudAAAAAAAABMAAAABAAAAAg==",
        "AAAAAAAAAAAAAAAKaW5pdGlhbGl6ZQAAAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAAt3YXNtX2hhc2hlcwAAAAfQAAAACldhc21IYXNoZXMAAAAAAAEAAAPpAAAAAgAAB9AAAAATQWNjb3VudEZhY3RvcnlFcnJvcgA=",
        "AAAABQAAAAAAAAAAAAAAEVdhc21IYXNoZXNVcGRhdGVkAAAAAAAAAQAAAAh3YXNtX3NldAAAAAAAAAAC",
        "AAAABAAAAAAAAAAAAAAAE0FjY291bnRGYWN0b3J5RXJyb3IAAAAAAgAAAAAAAAASQWxyZWFkeUluaXRpYWxpemVkAAAAACcQAAAAAAAAAA5Ob3RJbml0aWFsaXplZAAAAAAnEQ==",
        "AAAAAAAAAAAAAAANY29udHJhY3RfbmFtZQAAAAAAAAAAAAABAAAAEQ==",
        "AAAAAAAAA7ZEZXBsb3lzIGFuZCB3aXJlcyBhIGNvbXBsZXRlLCByZWFkeS10by11c2UgdHJlYXN1cnkgc3RhY2sgaW4gb25lCmNhbGwuIFNlZSB0aGUgbW9kdWxlIGRvYyBjb21tZW50IGZvciB0aGUgYXV0aG9yaXphdGlvbiBtb2RlbCBhbmQKd2hhdCAicmVhZHkgdG8gdXNlIiBtZWFucyAoYWRhcHRlcnMgYWxyZWFkeSBib3VuZCwgbm8gdGltZWxvY2s7CmBpbnRlbnRfcmVnaXN0cnlgIGFscmVhZHkgaW5pdGlhbGl6ZWQpLgoKYHNhbHRgIG9ubHkgbmVlZHMgdG8gYmUgdW5pcXVlIHBlciBgY2FsbGVyYCDigJQgc3ViLWNvbnRyYWN0IGFkZHJlc3NlcwphcmUgZGVyaXZlZCBmcm9tIGBzaGEyNTYoY2FsbGVyIHx8IHNhbHQgfHwgcGVyLWNvbnRyYWN0IHRhZylgLCBzbyBvbmUKY2FsbGVyLWNob3NlbiBzYWx0IGlzIGVub3VnaCB0byBwbGFjZSBhbGwgc2l4IGRlcGxveW1lbnRzCmRldGVybWluaXN0aWNhbGx5IGFuZCBjb2xsaXNpb24tZnJlZSBhZ2FpbnN0IGVhY2ggb3RoZXIsIGFuZCB0d28KZGlmZmVyZW50IGNhbGxlcnMgY2FuIHNhZmVseSBjaG9vc2UgdGhlIGlkZW50aWNhbCByYXcgc2FsdCB2YWx1ZQp3aXRoIG5vIGNyb3NzLWNhbGxlciBjb2xsaXNpb24gb3Igc3F1YXR0aW5nIHJpc2sgKHNlZSBgc3ViX3NhbHRgJ3MKZG9jIGNvbW1lbnQgZm9yIHRoZSBzZWN1cml0eSByZXZpZXcgZmluZGluZyB0aGlzIGNsb3NlZCkuCgpgZXhlY3V0b3JgIGlzIGBpbnRlbnRfcmVnaXN0cnlgJ3Mgc2NoZWR1bGVkLXBheW1lbnQgcmVsYXllciBhZGRyZXNzCihzZWUgYHNtYXJ0X2FjY291bnQ6OmluaXRpYWxpemVgJ3MgZG9jIGNvbW1lbnQgb24gYGluaXRpYWxfZXhlY3V0b3JgCmZvciB3aHkgdGhpcyBuZWVkcyB0byBiZSBleHBsaWNpdCByYXRoZXIgdGhhbiBkZWZhdWx0ZWQpLiBQYXNzCmBjYWxsZXJgIGFnYWluIGlmIHRoZXJlIGlzIG5vIHNlcGFyYXRlIHJlbGF5ZXIgaWRlbnRpdHkgeWV0LgAAAAAADmRlcGxveV9hY2NvdW50AAAAAAAGAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAABHNhbHQAAAPuAAAAIAAAAAAAAAAPaW5pdGlhbF9zaWduZXJzAAAAA+oAAAfQAAAABlNpZ25lcgAAAAAAAAAAABBpbml0aWFsX3BvbGljaWVzAAAD7AAAABMAAAAAAAAAAAAAABJndWFyZGlhbl90aHJlc2hvbGQAAAAAAAQAAAAAAAAACGV4ZWN1dG9yAAAAEwAAAAEAAAPpAAAH0AAAAA9EZXBsb3llZEFjY291bnQAAAAH0AAAABNBY2NvdW50RmFjdG9yeUVycm9yAA==",
        "AAAAAAAAAAAAAAAPZ2V0X3dhc21faGFzaGVzAAAAAAAAAAABAAAD6QAAB9AAAAAKV2FzbUhhc2hlcwAAAAAH0AAAABNBY2NvdW50RmFjdG9yeUVycm9yAA==",
        "AAAAAAAAAgpVcGRhdGVzIHRoZSBXQVNNIGhhc2hlcyB1c2VkIGZvciBldmVyeSAqZnV0dXJlKiBgZGVwbG95X2FjY291bnRgCmNhbGwuIEFkbWluLWdhdGVkIGFuZCBkZWxpYmVyYXRlbHkgbXV0YWJsZSDigJQgdW5saWtlIGBzbWFydF9hY2NvdW50YCdzCm93biBwaW5uZWQgc3Vib3JkaW5hdGUtbW9kdWxlIGFkZHJlc3NlcyAod2hpY2ggcHJvdGVjdCBvbmUgYWxyZWFkeQotZGVwbG95ZWQgdHJlYXN1cnkncyBlc3RhYmxpc2hlZCB0cnVzdCksIHRoaXMgb25seSBldmVyIGFmZmVjdHMKdHJlYXN1cmllcyBkZXBsb3llZCAqYWZ0ZXIqIHRoZSB1cGRhdGU7IGFscmVhZHktZGVwbG95ZWQgdHJlYXN1cmllcwphcmUgdW5hZmZlY3RlZCwgc2luY2UgdGhlaXIgYWRkcmVzc2VzIHdlcmUgYWxyZWFkeSBmaXhlZCBhdCB0aGVpcgpvd24gZGVwbG95IHRpbWUuIFNoYXJlZCBpbmZyYXN0cnVjdHVyZSBjb2RlIGdldHRpbmcgcGF0Y2hlZCBvdmVyCnRpbWUgaXMgdGhlIG9yZGluYXJ5IGNhc2UsIG5vdCBhIHJlc2lkdWFsIHJpc2sgdG8gbmFtZS4AAAAAAA9zZXRfd2FzbV9oYXNoZXMAAAAAAQAAAAAAAAALd2FzbV9oYXNoZXMAAAAH0AAAAApXYXNtSGFzaGVzAAAAAAABAAAD6QAAAAIAAAfQAAAAE0FjY291bnRGYWN0b3J5RXJyb3IA",
        "AAAAAAAAAL9QZXJtaXNzaW9ubGVzcyBUVEwgbWFpbnRlbmFuY2Ug4oCUIGV4dGVuZGluZyBUVEwgY3JlYXRlcyBubyBhdXRob3JpdHksCm1hdGNoaW5nIHRoZSByZWFzb25pbmcgYWxyZWFkeSB1c2VkIGJ5IGV2ZXJ5IG90aGVyIGNvbnRyYWN0IGluIHRoaXMKd29ya3NwYWNlIChgZG9jcy9URUNITklDQUxfQVJDSElURUNUVVJFLm1kYCDCpzE2LjEpLgAAAAATZXh0ZW5kX2luc3RhbmNlX3R0bAAAAAAAAAAAAA==",
        "AAAAAgAAAEJSZXByZXNlbnRzIGRpZmZlcmVudCB0eXBlcyBvZiBzaWduZXJzIGluIHRoZSBzbWFydCBhY2NvdW50IHN5c3RlbS4AAAAAAAAAAAAGU2lnbmVyAAAAAAACAAAAAQAAAD1BIGRlbGVnYXRlZCBzaWduZXIgdGhhdCB1c2VzIGJ1aWx0LWluIHNpZ25hdHVyZSB2ZXJpZmljYXRpb24uAAAAAAAACURlbGVnYXRlZAAAAAAAAAEAAAATAAAAAQAAAHJBbiBleHRlcm5hbCBzaWduZXIgd2l0aCBjdXN0b20gdmVyaWZpY2F0aW9uIGxvZ2ljLgpDb250YWlucyB0aGUgdmVyaWZpZXIgY29udHJhY3QgYWRkcmVzcyBhbmQgdGhlIHB1YmxpYyBrZXkgZGF0YS4AAAAAAAhFeHRlcm5hbAAAAAIAAAATAAAADg==" ]),
      options
    )
  }
  public readonly fromJSON = {
    initialize: this.txFromJSON<Result<void>>,
        contract_name: this.txFromJSON<string>,
        deploy_account: this.txFromJSON<Result<DeployedAccount>>,
        get_wasm_hashes: this.txFromJSON<Result<WasmHashes>>,
        set_wasm_hashes: this.txFromJSON<Result<void>>,
        extend_instance_ttl: this.txFromJSON<null>
  }
}