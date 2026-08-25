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

  const status = await readAccountStatus(net);
  console.log("status:", status);

  const owner = await readOwner(net);
  console.log("owner:", owner);

  const ruleCount = await readContextRulesCount(net);
  console.log("context rule count:", ruleCount);

  for (let id = 0; id < ruleCount; id++) {
    try {
      const rule = await readContextRule(net, id);
      console.log(`context rule ${id}:`, JSON.stringify(rule, null, 2));
    } catch {
      console.log(`context rule ${id}: not found (removed)`);
    }
  }

  const policyVersion = await readPolicyVersion(net);
  console.log("policy version:", policyVersion);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
