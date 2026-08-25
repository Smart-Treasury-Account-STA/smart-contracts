/**
 * `smart_account`'s custom-account authorization: "Entry A" / "Entry B"
 * construction, per docs/DAPP_INTEGRATION_SPEC.md §5.
 *
 * `smart_account` implements Soroban's `CustomAccountInterface`
 * (`__check_auth`), composed from OpenZeppelin's `stellar-accounts` crate.
 * Any call that does `env.current_contract_address().require_auth()` --
 * every fund-moving or schedule-creating entrypoint -- needs the
 * transaction to carry a `SorobanAuthorizationEntry` whose
 * `credentials.signature` is not a signature at all, but a
 * contract-defined `AuthPayload` struct (`{ signers, context_rule_ids }`).
 * No wallet, and no version of `@stellar/stellar-sdk`, has a built-in
 * helper for this shape -- it's specific to this contract's own
 * composition. This module builds it directly from the contract's own
 * spec (so the `AuthPayload`/`Signer` struct/enum encoding is always
 * correct against whatever the deployed contract actually expects, not a
 * hand-maintained copy of the shape).
 *
 * Two entries are required per required `Signer::Delegated` signer:
 *
 * - **Entry A** (once, for `smart_account` itself): the `AuthPayload`
 *   structure -- no wallet interaction, assembled directly from the
 *   caller-supplied context rule id(s).
 * - **Entry B** (one per required signer): a standard classic-account
 *   authorization entry for the nested
 *   `addr.require_auth_for_args((auth_digest,))` call -- this is what a
 *   wallet (or a raw `Keypair`, for the examples in this SDK) actually
 *   signs, via `@stellar/stellar-sdk`'s own `authorizeEntry`.
 */
import {
  Address,
  hash,
  Keypair,
  Networks,
  xdr,
  type SigningCallback,
} from "@stellar/stellar-sdk";
import { authorizeEntry } from "@stellar/stellar-sdk";
import type { Spec } from "@stellar/stellar-sdk/contract";

export interface SubInvocationSpec {
  contractId: string;
  functionName: string;
  args: xdr.ScVal[];
  subInvocations?: xdr.SorobanAuthorizedInvocation[];
}

export function buildInvocation(spec: SubInvocationSpec): xdr.SorobanAuthorizedInvocation {
  const fn = new xdr.InvokeContractArgs({
    contractAddress: new Address(spec.contractId).toScAddress(),
    functionName: spec.functionName,
    args: spec.args,
  });
  const func = xdr.SorobanAuthorizedFunction.sorobanAuthorizedFunctionTypeContractFn(fn);
  return new xdr.SorobanAuthorizedInvocation({
    function: func,
    subInvocations: spec.subInvocations ?? [],
  });
}

function networkId(networkPassphrase: string): Buffer {
  return hash(Buffer.from(networkPassphrase));
}

/** The standard Soroban authorization-entry signature payload: the hash of
 * the `HashIdPreimage::SorobanAuthorization` preimage over a given
 * invocation, nonce, and expiration ledger. Every `Address` credential
 * (custom-account or classic) signs a value derived from this. */
export function signaturePayload(
  invocation: xdr.SorobanAuthorizedInvocation,
  nonce: bigint,
  signatureExpirationLedger: number,
  networkPassphrase: string,
): Buffer {
  const preimage = xdr.HashIdPreimage.envelopeTypeSorobanAuthorization(
    new xdr.HashIdPreimageSorobanAuthorization({
      networkId: networkId(networkPassphrase),
      nonce: xdr.Int64.fromString(nonce.toString()),
      signatureExpirationLedger,
      invocation,
    }),
  );
  return hash(preimage.toXDR());
}

function randomNonce(): bigint {
  // 62-bit random nonce, matching scripts/execute_signer_authorized_call.py.
  const high = BigInt(Math.floor(Math.random() * 2 ** 30));
  const low = BigInt(Math.floor(Math.random() * 2 ** 32));
  return (high << 32n) | low;
}

/**
 * Builds and signs one ordinary classic-account `SorobanAuthorizationEntry`
 * -- an `Address` credential whose signature is a real Ed25519 signature
 * (via `authorizeEntry`), not a contract-defined payload like
 * `smart_account`'s `AuthPayload`. Shared by Entry B of
 * `buildSmartAccountAuthEntries` (the nested `__check_auth` call a
 * `Signer::Delegated` wallet signs) and `buildExecutorAuthEntry` (the
 * relayer's `mark_child_executed` authorization) -- both are the same
 * mechanism pointed at a different invocation/address.
 */
async function buildClassicAuthEntry(
  address: string,
  invocation: xdr.SorobanAuthorizedInvocation,
  sign: Keypair | SigningCallback,
  signatureExpirationLedger: number,
  networkPassphrase: string,
): Promise<xdr.SorobanAuthorizationEntry> {
  const nonce = randomNonce();
  const unsignedEntry = new xdr.SorobanAuthorizationEntry({
    credentials: xdr.SorobanCredentials.sorobanCredentialsAddress(
      new xdr.SorobanAddressCredentials({
        address: new Address(address).toScAddress(),
        nonce: xdr.Int64.fromString(nonce.toString()),
        signatureExpirationLedger,
        signature: xdr.ScVal.scvVoid(),
      }),
    ),
    rootInvocation: invocation,
  });
  return authorizeEntry(unsignedEntry, sign, signatureExpirationLedger, networkPassphrase);
}

export interface BuildSmartAccountAuthOptions {
  /** `smart_account`'s own contract Spec (from its generated Client's
   * `.spec`) -- used to encode the `AuthPayload` struct correctly. */
  spec: Spec;
  smartAccountId: string;
  /** The actual `smart_account` call being authorized (its root
   * invocation) -- e.g. `execute_transfer_payment(...)`. Declare any
   * further sub-invocations that must be pre-cleared by this SAME entry
   * (see the module doc comment on this pattern) as its `subInvocations`. */
  rootInvocation: xdr.SorobanAuthorizedInvocation;
  /** G-address of the `Signer::Delegated` wallet authorizing this call. */
  signerAddress: string;
  /** Signs Entry B's classic authorization entry -- a raw `Keypair` for
   * Node/CLI use, or a `SigningCallback` forwarding to a connected
   * wallet's `signAuthEntry` (see docs/DAPP_INTEGRATION_SPEC.md §5.3). */
  sign: Keypair | SigningCallback;
  networkPassphrase: string;
  /** Context rule id(s) the signer is registered under, one per auth
   * context reaching `__check_auth` (root + any declared
   * sub-invocation) -- see `docs/DAPP_INTEGRATION_SPEC.md` §5.2. Default:
   * `[0]` (the founding rule), applied to every context. */
  contextRuleIds?: number[];
  /** How many auth contexts this entry covers (root + sub-invocations).
   * Default: 1 (root only). */
  authContextCount?: number;
  signatureExpirationLedger: number;
}

/** Builds Entry A + Entry B for a single required `Signer::Delegated`.
 * For a multi-signer context rule, call this once per required signer
 * (all against the identical `rootInvocation`) and attach every resulting
 * Entry A + Entry B pair -- see docs/DAPP_INTEGRATION_SPEC.md §5.4. */
export async function buildSmartAccountAuthEntries(
  opts: BuildSmartAccountAuthOptions,
): Promise<[xdr.SorobanAuthorizationEntry, xdr.SorobanAuthorizationEntry]> {
  const contextRuleIds = opts.contextRuleIds ?? [0];
  const nonceA = randomNonce();
  const sigPayloadA = signaturePayload(
    opts.rootInvocation,
    nonceA,
    opts.signatureExpirationLedger,
    opts.networkPassphrase,
  );

  const contextRuleIdsScVal = xdr.ScVal.scvVec(
    contextRuleIds.map((id) => xdr.ScVal.scvU32(id)),
  );
  const authDigest = hash(
    Buffer.concat([Buffer.from(sigPayloadA), Buffer.from(contextRuleIdsScVal.toXDR())]),
  );

  const authPayloadTy = xdr.ScSpecTypeDef.scSpecTypeUdt(
    new xdr.ScSpecTypeUdt({ name: "AuthPayload" }),
  );
  const authPayloadScVal = opts.spec.nativeToScVal(
    {
      signers: new Map([[{ tag: "Delegated", values: [opts.signerAddress] }, Buffer.alloc(0)]]),
      context_rule_ids: contextRuleIds,
    },
    authPayloadTy,
  );

  const entryA = new xdr.SorobanAuthorizationEntry({
    credentials: xdr.SorobanCredentials.sorobanCredentialsAddress(
      new xdr.SorobanAddressCredentials({
        address: new Address(opts.smartAccountId).toScAddress(),
        nonce: xdr.Int64.fromString(nonceA.toString()),
        signatureExpirationLedger: opts.signatureExpirationLedger,
        signature: authPayloadScVal,
      }),
    ),
    rootInvocation: opts.rootInvocation,
  });

  const nestedInvocation = buildInvocation({
    contractId: opts.smartAccountId,
    functionName: "__check_auth",
    args: [xdr.ScVal.scvBytes(authDigest)],
  });
  const entryB = await buildClassicAuthEntry(
    opts.signerAddress,
    nestedInvocation,
    opts.sign,
    opts.signatureExpirationLedger,
    opts.networkPassphrase,
  );

  return [entryA, entryB];
}

export interface BuildExecutorAuthOptions {
  intentRegistryId: string;
  intentId: Buffer;
  childSequence: number;
  executorAddress: string;
  sign: Keypair | SigningCallback;
  networkPassphrase: string;
  signatureExpirationLedger: number;
}

/**
 * Authorizes `intent_registry.mark_child_executed`, the *only* real
 * authorization check in `execute_scheduled_payment`'s whole call graph
 * (docs/DAPP_INTEGRATION_SPEC.md §8: `execute_scheduled_payment` itself
 * has no `smart_account` auth gate).
 *
 * **This corrects a real inaccuracy in `docs/DAPP_INTEGRATION_SPEC.md`
 * §8 step 4**, found by actually running this against live testnet while
 * building this SDK: the spec claims `prepareTransaction` auto-fills the
 * `Executor`'s requirement as a `SOURCE_ACCOUNT` credential. It does not
 * -- verified with `@stellar/stellar-sdk ^14.5.0` against
 * `soroban-testnet.stellar.org`, this fails with `Error(Auth,
 * InvalidAction)` / "encountered authorization not tied to the root
 * contract invocation for an address," the exact same failure mode
 * `scripts/execute_scheduled_payment_as_relayer.py` found against the
 * bare `stellar` CLI this session. `SourceAccount`/auto-fill credentials
 * only cover a `require_auth()` at the ROOT of the invocation tree;
 * `mark_child_executed`'s `executor.require_auth()` is two levels deep
 * (`execute_scheduled_payment -> intent_registry.mark_child_executed ->
 * ensure_executor`), so it needs an explicit entry, built and signed the
 * same way as any other non-root classic-account authorization -- this
 * is *not* custom-account machinery (the executor is a plain account),
 * so no `AuthPayload`/Entry-A-Entry-B pairing is involved, just one
 * ordinary signed entry rooted directly at `mark_child_executed`.
 */
export async function buildExecutorAuthEntry(
  opts: BuildExecutorAuthOptions,
): Promise<xdr.SorobanAuthorizationEntry> {
  const invocation = buildInvocation({
    contractId: opts.intentRegistryId,
    functionName: "mark_child_executed",
    args: [xdr.ScVal.scvBytes(opts.intentId), xdr.ScVal.scvU32(opts.childSequence)],
  });
  return buildClassicAuthEntry(
    opts.executorAddress,
    invocation,
    opts.sign,
    opts.signatureExpirationLedger,
    opts.networkPassphrase,
  );
}

export { Networks };
