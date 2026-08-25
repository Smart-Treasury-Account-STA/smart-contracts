/**
 * Recovery-state as a first-class typed read, not a raw XDR blob --
 * `RecoveryRequest { replacement_owner, replacement_signers,
 * replacement_policies, earliest_ledger, approvers, cancelled, finalized }`.
 *
 * Defaults to the completed guardian-freeze-and-recovery walkthrough
 * already recorded in docs/TESTNET_FACTORY_DEPLOYMENT.md §14 (a
 * throwaway, reduced-timelock treasury built specifically to demonstrate
 * this flow quickly -- see that section for why). Override
 * RECOVERY_MANAGER_ID/GUARDIAN/REQUEST_ID to point at a different one.
 *
 *   npx tsx examples/06-read-recovery-state.ts
 */
import { TESTNET, readGuardianFreezeEpoch, isGuardian, readRecoveryRequest, recoveryManagerClient } from "sta-sdk";

async function main() {
  const net = {
    ...TESTNET,
    contracts: {
      ...TESTNET.contracts,
      recoveryManager:
        process.env.RECOVERY_MANAGER_ID ?? "CDUTVUO3SJDRMKVGJ67TUX33PWSIRYJZXYDTQPJSTBRI6XGKNE3ATJ2V",
    },
  };
  const guardian = process.env.GUARDIAN ?? "GDXVIRLSBDKT7EZM2RM3FH26W3TPF77IJ7GZBA5IOA6ZJBTW26NNO3AV";
  const requestIdHex =
    process.env.REQUEST_ID ?? "70c20d062f77445181d0c09c070da0f448ea097ae513d13861fcb144ed1b20b5";

  console.log("is guardian:", await isGuardian(net, guardian));
  console.log("guardian freeze epoch:", await readGuardianFreezeEpoch(net));

  const request = await readRecoveryRequest(net, Buffer.from(requestIdHex, "hex"));
  console.log("recovery request:", request);

  const live = await recoveryManagerClient(net).live_approval_count({
    request_id: Buffer.from(requestIdHex, "hex"),
  });
  console.log("live approval count:", live.result.unwrap());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
