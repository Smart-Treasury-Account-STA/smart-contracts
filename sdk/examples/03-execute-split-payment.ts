/**
 * A real, signer-authorized `execute_split_payment` call. Run with:
 *   OWNER_SECRET_KEY=... FEE_SOURCE_SECRET_KEY=... ASSET=... \
 *   RECIPIENT_1=... RECIPIENT_2=... AMOUNT_1=... AMOUNT_2=... NONCE=... \
 *   npx tsx examples/03-execute-split-payment.ts
 */
import { Keypair } from "@stellar/stellar-sdk";
import {
  TESTNET,
  smartAccountClient,
  readPolicyVersion,
  prepareSplitPayment,
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
  const ownerKeypair = Keypair.fromSecret(requireEnv("OWNER_SECRET_KEY"));
  const feeSourceKeypair = Keypair.fromSecret(requireEnv("FEE_SOURCE_SECRET_KEY"));

  const asset = requireEnv("ASSET");
  const recipients = [requireEnv("RECIPIENT_1"), requireEnv("RECIPIENT_2")];
  const amounts = [BigInt(requireEnv("AMOUNT_1")), BigInt(requireEnv("AMOUNT_2"))];
  const nonce = BigInt(requireEnv("NONCE"));

  const expectedPolicyVersion = await readPolicyVersion(net);
  console.log("expected policy version:", expectedPolicyVersion);

  const client = smartAccountClient(net);
  const tx = await prepareSplitPayment(
    {
      net,
      smartAccountClient: client,
      feeSourceAddress: feeSourceKeypair.publicKey(),
      signerAddress: ownerKeypair.publicKey(),
      sign: ownerKeypair,
    },
    { asset, recipients, amounts, nonce, expectedPolicyVersion },
  );

  const result = await signAndSubmit(net, tx, feeSourceKeypair);
  console.log("submitted, ledger:", result.ledger);

  const events = parseContractEvents(result.events.contractEventsXdr[0] ?? []);
  console.log("SplitPaid event:", findEvent(events, "splt_ok"));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
