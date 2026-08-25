/**
 * A real, signer-authorized `execute_transfer_payment` call: prepares the
 * transaction (Entry A + Entry B attached), signs the envelope, submits,
 * and parses the resulting `TransferPaid` event. Run with:
 *
 *   OWNER_SECRET_KEY=... FEE_SOURCE_SECRET_KEY=... \
 *   ASSET=... DESTINATION=... AMOUNT=... NONCE=... \
 *   npx tsx examples/02-execute-transfer-payment.ts
 */
import { Keypair } from "@stellar/stellar-sdk";
import {
  TESTNET,
  smartAccountClient,
  readPolicyVersion,
  prepareTransferPayment,
  signAndSubmit,
  parseContractEvents,
  findEvent,
} from "sta-sdk";

async function main() {
  const net = TESTNET;
  const ownerKeypair = Keypair.fromSecret(requireEnv("OWNER_SECRET_KEY"));
  const feeSourceKeypair = Keypair.fromSecret(requireEnv("FEE_SOURCE_SECRET_KEY"));

  const asset = requireEnv("ASSET");
  const destination = requireEnv("DESTINATION");
  const amount = BigInt(requireEnv("AMOUNT"));
  const nonce = BigInt(requireEnv("NONCE"));

  // Read fresh immediately before building -- never cache across a user
  // session (docs/DAPP_INTEGRATION_SPEC.md §6 step 1).
  const expectedPolicyVersion = await readPolicyVersion(net);
  console.log("expected policy version:", expectedPolicyVersion);

  const client = smartAccountClient(net);
  const tx = await prepareTransferPayment(
    {
      net,
      smartAccountClient: client,
      feeSourceAddress: feeSourceKeypair.publicKey(),
      signerAddress: ownerKeypair.publicKey(),
      sign: ownerKeypair,
    },
    { asset, destination, amount, nonce, expectedPolicyVersion },
  );

  const result = await signAndSubmit(net, tx, feeSourceKeypair);
  console.log("submitted, ledger:", result.ledger);

  const events = parseContractEvents(result.events.contractEventsXdr[0] ?? []);
  const transferPaid = findEvent(events, "pay_ok");
  console.log("TransferPaid event:", transferPaid);
}

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`missing required env var ${name}`);
  return value;
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
