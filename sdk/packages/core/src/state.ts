/**
 * First-class typed reads for policy-version, replay-state (nonce), and
 * recovery-state -- not just wrapped wallet-signing calls. Every function
 * here is a read-only simulation (no signature, no submission) built
 * directly on the generated per-contract clients.
 */
import { Client as SmartAccountClient, type AccountStatus, type ContextRule } from "@sta/smart_account-bindings";
import { Client as PolicyEngineClient } from "@sta/policy_engine-bindings";
import { Client as IntentRegistryClient, type ScheduledIntent } from "@sta/intent_registry-bindings";
import { Client as RecoveryManagerClient, type RecoveryRequest } from "@sta/recovery_manager-bindings";
import { Client as AccountFactoryClient, type WasmHashes } from "@sta/account_factory-bindings";
import type { NetworkConfig } from "./config.js";

export type { AccountStatus, ContextRule, ScheduledIntent, RecoveryRequest, WasmHashes };

interface ClientCtorOptions {
  contractId: string;
  networkPassphrase: string;
  rpcUrl: string;
}

/** Every generated `Client` constructor takes the same
 * `{ contractId, networkPassphrase, rpcUrl }` shape -- this is the one
 * place that assembles it, so the five `*Client` factories below are each
 * a one-liner naming which contract address and class to use. */
function makeClient<T>(Ctor: new (opts: ClientCtorOptions) => T, contractId: string, net: NetworkConfig): T {
  return new Ctor({ contractId, networkPassphrase: net.networkPassphrase, rpcUrl: net.rpcUrl });
}

export function smartAccountClient(net: NetworkConfig): SmartAccountClient {
  return makeClient(SmartAccountClient, net.contracts.smartAccount, net);
}

export function policyEngineClient(net: NetworkConfig): PolicyEngineClient {
  return makeClient(PolicyEngineClient, net.contracts.policyEngine, net);
}

export function intentRegistryClient(net: NetworkConfig): IntentRegistryClient {
  return makeClient(IntentRegistryClient, net.contracts.intentRegistry, net);
}

export function recoveryManagerClient(net: NetworkConfig): RecoveryManagerClient {
  return makeClient(RecoveryManagerClient, net.contracts.recoveryManager, net);
}

export function accountFactoryClient(net: NetworkConfig): AccountFactoryClient {
  return makeClient(AccountFactoryClient, net.contracts.accountFactory, net);
}

/** `smart_account.status()` -- check before offering any payment action; a
 * paused or frozen treasury should disable the payment UI, not let the
 * user hit a rejected simulation (docs/DAPP_INTEGRATION_SPEC.md §4). */
export async function readAccountStatus(net: NetworkConfig): Promise<AccountStatus> {
  const tx = await smartAccountClient(net).status();
  return tx.result.unwrap();
}

export async function readOwner(net: NetworkConfig): Promise<string | undefined> {
  const tx = await smartAccountClient(net).get_owner();
  return tx.result;
}

/** Rule IDs are `0..count`, not necessarily contiguous after removals --
 * check existence with `readContextRule`, don't assume. */
export async function readContextRulesCount(net: NetworkConfig): Promise<number> {
  const tx = await smartAccountClient(net).get_context_rules_count();
  return tx.result;
}

export async function readContextRule(net: NetworkConfig, contextRuleId: number): Promise<ContextRule> {
  const tx = await smartAccountClient(net).get_context_rule({ context_rule_id: contextRuleId });
  return tx.result;
}

export async function isNonceUsed(net: NetworkConfig, nonce: bigint): Promise<boolean> {
  const tx = await smartAccountClient(net).is_nonce_used({ nonce });
  return tx.result;
}

/** The authoritative current policy version -- read fresh immediately
 * before building a payment, never cached across a user session (see
 * docs/DAPP_INTEGRATION_SPEC.md §6 step 1: a stale value here causes a
 * clean `VersionMismatch` rejection rather than executing under outdated
 * rules). */
export async function readPolicyVersion(net: NetworkConfig): Promise<number> {
  const tx = await policyEngineClient(net).version();
  return tx.result.unwrap();
}

export async function readScheduledIntent(net: NetworkConfig, intentId: Buffer): Promise<ScheduledIntent> {
  const tx = await intentRegistryClient(net).get_intent({ intent_id: intentId });
  return tx.result.unwrap();
}

export async function isChildExecuted(net: NetworkConfig, intentId: Buffer, childSequence: number): Promise<boolean> {
  const tx = await intentRegistryClient(net).is_child_executed({ intent_id: intentId, child_sequence: childSequence });
  return tx.result.unwrap();
}

/** Recovery-request state -- `RecoveryRequest { replacement_owner,
 * replacement_signers, replacement_policies, earliest_ledger, approvers,
 * cancelled, finalized }`. First-class typed state, not a raw XDR blob. */
export async function readRecoveryRequest(net: NetworkConfig, requestId: Buffer): Promise<RecoveryRequest> {
  const tx = await recoveryManagerClient(net).request_status({ request_id: requestId });
  return tx.result.unwrap();
}

export async function readLiveApprovalCount(net: NetworkConfig, requestId: Buffer): Promise<number> {
  const tx = await recoveryManagerClient(net).live_approval_count({ request_id: requestId });
  return tx.result.unwrap();
}

export async function isGuardian(net: NetworkConfig, guardian: string): Promise<boolean> {
  const tx = await recoveryManagerClient(net).is_guardian({ guardian });
  return tx.result;
}

/** Monotonically increasing, never-cleared -- compare against a locally
 * stored "last applied" value to detect a pending guardian freeze rather
 * than treating this as a boolean (see
 * docs/SECURITY_REVIEW_STRICT.md's guardian-freeze-epoch redesign). */
export async function readGuardianFreezeEpoch(net: NetworkConfig): Promise<number> {
  const tx = await recoveryManagerClient(net).guardian_freeze_epoch();
  return tx.result;
}

export async function readFactoryWasmHashes(net: NetworkConfig): Promise<WasmHashes> {
  const tx = await accountFactoryClient(net).get_wasm_hashes();
  return tx.result.unwrap();
}
