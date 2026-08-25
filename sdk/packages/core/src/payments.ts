/**
 * Transaction-preparation helpers: prepare -> simulate -> approve, per
 * docs/DAPP_INTEGRATION_SPEC.md §6/§7. Each function returns a prepared,
 * auth-entry-attached `Transaction` -- simulated (fee/resource estimation)
 * with the custom `smart_account` `AuthPayload` entries already present,
 * ready for the caller to sign the envelope with a fee-paying account and
 * submit (see `signAndSubmit` below). None of these functions touch a
 * wallet directly -- signing is always an injected `Keypair | SigningCallback`
 * (see `auth.ts`), so a browser dApp can swap in a connected wallet's
 * `signAuthEntry` without changing anything here.
 */
import {
  Account,
  Keypair,
  Operation,
  TransactionBuilder,
  BASE_FEE,
  rpc,
  xdr,
  type SigningCallback,
  type Transaction,
} from "@stellar/stellar-sdk";
import type { Client as SmartAccountClient, ScheduledIntentArgs } from "@sta/smart_account-bindings";
import { buildExecutorAuthEntry, buildInvocation, buildSmartAccountAuthEntries } from "./auth.js";
import type { NetworkConfig } from "./config.js";

const DEFAULT_EXPIRATION_WINDOW_LEDGERS = 100;
const POLL_INTERVAL_MS = 3000;
const MAX_POLL_ATTEMPTS = 40; // ~2 minutes at POLL_INTERVAL_MS

/**
 * Builds, simulates, and returns a prepared `Transaction` invoking one
 * contract function with a given set of auth entries already attached.
 * Shared scaffolding for every `prepare*` helper below (signer-authored
 * `smart_account` calls and the relayer's `execute_scheduled_payment`
 * alike) -- only the auth entries and operation args actually differ
 * between them. Takes an already-fetched `sourceAccount` rather than
 * fetching it itself, so callers can build auth entries (a signing round
 * trip, possibly a wallet prompt) and fetch the source account
 * concurrently instead of serially.
 */
function buildAndPrepareTransaction(
  server: rpc.Server,
  net: NetworkConfig,
  sourceAccount: Account,
  contract: string,
  functionName: string,
  args: xdr.ScVal[],
  auth: xdr.SorobanAuthorizationEntry[],
): Promise<Transaction> {
  const builder = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase: net.networkPassphrase,
  })
    .addOperation(Operation.invokeContractFunction({ contract, function: functionName, args, auth }))
    .setTimeout(120)
    .build();

  return server.prepareTransaction(builder);
}

export interface PrepareOptions {
  net: NetworkConfig;
  smartAccountClient: SmartAccountClient;
  /** Any funded account that pays the network fee -- independent of who
   * authorizes the `smart_account` call (docs/DAPP_INTEGRATION_SPEC.md §6
   * step 4). Only its public key is needed to build the transaction. */
  feeSourceAddress: string;
  signerAddress: string;
  sign: Keypair | SigningCallback;
  contextRuleIds?: number[];
}

async function prepareSmartAccountCall(
  opts: PrepareOptions,
  functionName: string,
  args: xdr.ScVal[],
): Promise<Transaction> {
  const server = new rpc.Server(opts.net.rpcUrl);
  const latestLedger = await server.getLatestLedger();
  const signatureExpirationLedger = latestLedger.sequence + DEFAULT_EXPIRATION_WINDOW_LEDGERS;

  const rootInvocation = buildInvocation({
    contractId: opts.net.contracts.smartAccount,
    functionName,
    args,
  });

  // Neither depends on the other's result -- run the signing round trip
  // (possibly a wallet prompt) and the account-fetch RPC call concurrently
  // rather than paying both latencies serially.
  const [[entryA, entryB], sourceAccount] = await Promise.all([
    buildSmartAccountAuthEntries({
      spec: opts.smartAccountClient.spec,
      smartAccountId: opts.net.contracts.smartAccount,
      rootInvocation,
      signerAddress: opts.signerAddress,
      sign: opts.sign,
      networkPassphrase: opts.net.networkPassphrase,
      contextRuleIds: opts.contextRuleIds,
      signatureExpirationLedger,
    }),
    server.getAccount(opts.feeSourceAddress),
  ]);

  return buildAndPrepareTransaction(
    server,
    opts.net,
    sourceAccount,
    opts.net.contracts.smartAccount,
    functionName,
    args,
    [entryA, entryB],
  );
}

export interface TransferPaymentArgs {
  asset: string;
  destination: string;
  amount: bigint;
  nonce: bigint;
  expectedPolicyVersion: number;
}

export async function prepareTransferPayment(
  opts: PrepareOptions,
  payment: TransferPaymentArgs,
): Promise<Transaction> {
  const args = opts.smartAccountClient.spec.funcArgsToScVals("execute_transfer_payment", {
    asset: payment.asset,
    destination: payment.destination,
    amount: payment.amount,
    nonce: payment.nonce,
    expected_policy_version: payment.expectedPolicyVersion,
  });
  return prepareSmartAccountCall(opts, "execute_transfer_payment", args);
}

export interface SplitPaymentArgs {
  asset: string;
  recipients: string[];
  amounts: bigint[];
  nonce: bigint;
  expectedPolicyVersion: number;
}

export async function prepareSplitPayment(
  opts: PrepareOptions,
  payment: SplitPaymentArgs,
): Promise<Transaction> {
  const args = opts.smartAccountClient.spec.funcArgsToScVals("execute_split_payment", {
    asset: payment.asset,
    recipients: payment.recipients,
    amounts: payment.amounts,
    nonce: payment.nonce,
    expected_policy_version: payment.expectedPolicyVersion,
  });
  return prepareSmartAccountCall(opts, "execute_split_payment", args);
}

/** `intent`'s caller-supplied `policy_version`/`adapter` are ignored and
 * overwritten by the contract (pinned to current values at approval time)
 * -- pass any placeholder value; read the resolved fields back from the
 * `IntentCreated` event or a follow-up `get_intent` read
 * (docs/DAPP_INTEGRATION_SPEC.md §7). */
export async function prepareScheduledPayment(
  opts: PrepareOptions,
  intent: ScheduledIntentArgs,
): Promise<Transaction> {
  const args = opts.smartAccountClient.spec.funcArgsToScVals("create_scheduled_payment", { intent });
  return prepareSmartAccountCall(opts, "create_scheduled_payment", args);
}

export async function prepareCancelScheduledPayment(
  opts: PrepareOptions,
  intentId: Buffer,
): Promise<Transaction> {
  const args = opts.smartAccountClient.spec.funcArgsToScVals("cancel_scheduled_payment", {
    intent_id: intentId,
  });
  return prepareSmartAccountCall(opts, "cancel_scheduled_payment", args);
}

/** Signs the transaction envelope with the fee-paying account's key
 * (independent of the `smart_account` auth entries, already attached by
 * `prepare*`) and submits it, polling until a terminal status
 * (docs/DAPP_INTEGRATION_SPEC.md §6 step 4). */
export async function signAndSubmit(
  net: NetworkConfig,
  tx: Transaction,
  feeSourceKeypair: Keypair,
): Promise<rpc.Api.GetSuccessfulTransactionResponse> {
  const server = new rpc.Server(net.rpcUrl);
  tx.sign(feeSourceKeypair);
  const sendResponse = await server.sendTransaction(tx);
  if (sendResponse.status === "ERROR") {
    throw new Error(`sendTransaction failed: ${JSON.stringify(sendResponse.errorResult)}`);
  }

  console.log("submitted:", sendResponse.hash);

  let response = await server.getTransaction(sendResponse.hash);
  for (
    let attempt = 0;
    response.status === rpc.Api.GetTransactionStatus.NOT_FOUND && attempt < MAX_POLL_ATTEMPTS;
    attempt++
  ) {
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    response = await server.getTransaction(sendResponse.hash);
  }
  if (response.status === rpc.Api.GetTransactionStatus.NOT_FOUND) {
    throw new Error(
      `transaction ${sendResponse.hash} not found after ${MAX_POLL_ATTEMPTS} polling attempts`,
    );
  }
  if (response.status !== rpc.Api.GetTransactionStatus.SUCCESS) {
    throw new Error(`transaction ${sendResponse.hash} failed: ${JSON.stringify(response)}`);
  }
  return response;
}

export interface RelayerExecuteOptions {
  net: NetworkConfig;
  intentId: Buffer;
  childSequence: number;
  executorAddress: string;
  sign: Keypair | SigningCallback;
}

/**
 * Prepares a relayer's `execute_scheduled_payment` call. Unlike the
 * signer-authored payment helpers above, this needs no `smart_account`
 * `AuthPayload` at all -- only an explicit authorization entry for
 * `intent_registry.mark_child_executed`'s `Executor` requirement (see
 * `buildExecutorAuthEntry`'s doc comment in `auth.ts` for why the bare
 * `SourceAccount` auto-fill a relayer might expect does not cover this).
 */
export async function prepareRelayerExecution(opts: RelayerExecuteOptions): Promise<Transaction> {
  const server = new rpc.Server(opts.net.rpcUrl);
  const latestLedger = await server.getLatestLedger();
  const signatureExpirationLedger = latestLedger.sequence + DEFAULT_EXPIRATION_WINDOW_LEDGERS;

  const [executorEntry, sourceAccount] = await Promise.all([
    buildExecutorAuthEntry({
      intentRegistryId: opts.net.contracts.intentRegistry,
      intentId: opts.intentId,
      childSequence: opts.childSequence,
      executorAddress: opts.executorAddress,
      sign: opts.sign,
      networkPassphrase: opts.net.networkPassphrase,
      signatureExpirationLedger,
    }),
    server.getAccount(opts.executorAddress),
  ]);

  const args = [xdr.ScVal.scvBytes(opts.intentId), xdr.ScVal.scvU32(opts.childSequence)];
  return buildAndPrepareTransaction(
    server,
    opts.net,
    sourceAccount,
    opts.net.contracts.smartAccount,
    "execute_scheduled_payment",
    args,
    [executorEntry],
  );
}

export { Account };
