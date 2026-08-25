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

  const [entryA, entryB] = await buildSmartAccountAuthEntries({
    spec: opts.smartAccountClient.spec,
    smartAccountId: opts.net.contracts.smartAccount,
    rootInvocation,
    signerAddress: opts.signerAddress,
    sign: opts.sign,
    networkPassphrase: opts.net.networkPassphrase,
    contextRuleIds: opts.contextRuleIds,
    signatureExpirationLedger,
  });

  const sourceAccount = await server.getAccount(opts.feeSourceAddress);
  const builder = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase: opts.net.networkPassphrase,
  })
    .addOperation(
      Operation.invokeContractFunction({
        contract: opts.net.contracts.smartAccount,
        function: functionName,
        args,
        auth: [entryA, entryB],
      }),
    )
    .setTimeout(120)
    .build();

  return server.prepareTransaction(builder);
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
  while (response.status === rpc.Api.GetTransactionStatus.NOT_FOUND) {
    await new Promise((resolve) => setTimeout(resolve, 3000));
    response = await server.getTransaction(sendResponse.hash);
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

  const executorEntry = await buildExecutorAuthEntry({
    intentRegistryId: opts.net.contracts.intentRegistry,
    intentId: opts.intentId,
    childSequence: opts.childSequence,
    executorAddress: opts.executorAddress,
    sign: opts.sign,
    networkPassphrase: opts.net.networkPassphrase,
    signatureExpirationLedger,
  });

  const args = [xdr.ScVal.scvBytes(opts.intentId), xdr.ScVal.scvU32(opts.childSequence)];
  const sourceAccount = await server.getAccount(opts.executorAddress);
  const builder = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase: opts.net.networkPassphrase,
  })
    .addOperation(
      Operation.invokeContractFunction({
        contract: opts.net.contracts.smartAccount,
        function: "execute_scheduled_payment",
        args,
        auth: [executorEntry],
      }),
    )
    .setTimeout(120)
    .build();

  return server.prepareTransaction(builder);
}

export { Account };
