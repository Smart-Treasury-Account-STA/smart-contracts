/**
 * Read-only treasury state -- no signing, no submission. Run with:
 *   npx tsx examples/01-read-treasury-state.ts
 */
import {
  TESTNET,
  readAccountStatus,
  readOwner,
  readContextRulesCount,
  readContextRule,
  readPolicyVersion,
} from "sta-sdk";

async function main() {
  const net = TESTNET;

  // Independent reads -- run concurrently rather than paying each
  // simulation's RPC latency serially.
  const [status, owner, ruleCount, policyVersion] = await Promise.all([
    readAccountStatus(net),
    readOwner(net),
    readContextRulesCount(net),
    readPolicyVersion(net),
  ]);
  console.log("status:", status);
  console.log("owner:", owner);
  console.log("context rule count:", ruleCount);
  console.log("policy version:", policyVersion);

  for (let id = 0; id < ruleCount; id++) {
    try {
      const rule = await readContextRule(net, id);
      console.log(`context rule ${id}:`, JSON.stringify(rule, null, 2));
    } catch {
      console.log(`context rule ${id}: not found (removed)`);
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
