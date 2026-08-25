/**
 * Self-service treasury deployment via `account_factory` -- one call, one
 * signature, a fully wired treasury back (docs/TESTNET_FACTORY_DEPLOYMENT.md
 * §5). `deploy_account`'s `caller.require_auth()` is a plain, root-level
 * address requirement -- not `smart_account`'s custom `AuthPayload` -- so
 * this example uses the generated client directly via
 * `@stellar/stellar-sdk`'s `basicNodeSigner`, rather than this SDK's
 * `auth.ts`/`payments.ts` helpers (those are specifically for
 * `smart_account`'s custom-account calls).
 *
 *   CALLER_SECRET_KEY=... EXECUTOR=... npx tsx examples/05-deploy-treasury-via-factory.ts
 */
import { randomBytes } from "node:crypto";
import { Keypair } from "@stellar/stellar-sdk";
import { basicNodeSigner } from "@stellar/stellar-sdk/contract";
import { accountFactoryClient, TESTNET } from "sta-sdk";

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`missing required env var ${name}`);
  return value;
}

async function main() {
  const net = TESTNET;
  const callerKeypair = Keypair.fromSecret(requireEnv("CALLER_SECRET_KEY"));
  const executor = requireEnv("EXECUTOR");

  const signer = basicNodeSigner(callerKeypair, net.networkPassphrase);
  const client = accountFactoryClient(net);
  client.options.publicKey = callerKeypair.publicKey();
  client.options.signTransaction = signer.signTransaction;

  const salt = randomBytes(32);
  const tx = await client.deploy_account({
    caller: callerKeypair.publicKey(),
    salt,
    initial_signers: [{ tag: "Delegated", values: [callerKeypair.publicKey()] }],
    initial_policies: new Map(),
    guardian_threshold: 1,
    executor,
  });

  const sent = await tx.signAndSend();
  console.log("submitted:", sent.sendTransactionResponse?.hash);
  const deployed = sent.result.unwrap();
  console.log("deployed treasury:", deployed);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
