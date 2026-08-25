/**
 * Full scheduled-payment lifecycle: the treasury signer creates the
 * schedule (custom `smart_account` auth, same as 02/03), then the relayer
 * executes it (docs/DAPP_INTEGRATION_SPEC.md §8: `execute_scheduled_payment`
 * has no `smart_account` auth gate of its own; the only check is
 * `intent_registry`'s `Executor` requirement, a *plain* account).
 *
 *   OWNER_SECRET_KEY=... FEE_SOURCE_SECRET_KEY=... RELAYER_SECRET_KEY=... \
 *   ASSET=... DESTINATION=... AMOUNT=... \
 *   npx tsx examples/04-scheduled-payment-create-and-relayer-execute.ts
 */
import { randomBytes } from "node:crypto";
import { Keypair, rpc } from "@stellar/stellar-sdk";
import {
  TESTNET,
  smartAccountClient,
  prepareScheduledPayment,
  prepareRelayerExecution,
  signAndSubmit,
  parseContractEvents,
  findEvent,
} from "sta-sdk";

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`missing required env var ${name}`);
  return value;
}

async function main() {
  const net = TESTNET;
  const server = new rpc.Server(net.rpcUrl);
  const ownerKeypair = Keypair.fromSecret(requireEnv("OWNER_SECRET_KEY"));
  const feeSourceKeypair = Keypair.fromSecret(requireEnv("FEE_SOURCE_SECRET_KEY"));
  const relayerKeypair = Keypair.fromSecret(requireEnv("RELAYER_SECRET_KEY"));

  const asset = requireEnv("ASSET");
  const destination = requireEnv("DESTINATION");
  const amount = BigInt(requireEnv("AMOUNT"));

  const latestLedger = await server.getLatestLedger();
  const intentId = randomBytes(32);

  console.log("== creating scheduled payment ==");
  const client = smartAccountClient(net);
  const createTx = await prepareScheduledPayment(
    {
      net,
      smartAccountClient: client,
      feeSourceAddress: feeSourceKeypair.publicKey(),
      signerAddress: ownerKeypair.publicKey(),
      sign: ownerKeypair,
    },
    {
      intent_id: intentId,
      asset,
      destination,
      amount,
      start_ledger: latestLedger.sequence + 1,
      end_ledger: latestLedger.sequence + 5000,
      interval_ledgers: 0,
      max_executions: 1,
      // Ignored/overwritten server-side (docs/DAPP_INTEGRATION_SPEC.md §7):
      execution_count: 0,
      policy_version: 0,
      adapter: net.contracts.transferAdapter,
      cancelled: false,
    },
  );
  const createResult = await signAndSubmit(net, createTx, feeSourceKeypair);
  console.log("created, ledger:", createResult.ledger, "intent_id:", intentId.toString("hex"));

  console.log("== relayer executing scheduled payment ==");
  // execute_scheduled_payment itself needs no smart_account auth at all --
  // the only requirement in the whole call graph is intent_registry's
  // Executor (a plain account). See auth.ts's buildExecutorAuthEntry doc
  // comment: this still needs one explicit, signed authorization entry
  // (verified empirically while building this SDK -- the bare
  // SourceAccount auto-fill a relayer might expect does not cover it).
  const execTx = await prepareRelayerExecution({
    net,
    intentId,
    childSequence: 1,
    executorAddress: relayerKeypair.publicKey(),
    sign: relayerKeypair,
  });
  const execResult = await signAndSubmit(net, execTx, relayerKeypair);
  console.log("executed, ledger:", execResult.ledger);

  const events = parseContractEvents(execResult.events.contractEventsXdr[0] ?? []);
  console.log("ScheduledPaymentExecuted event:", findEvent(events, "auto_ok"));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
