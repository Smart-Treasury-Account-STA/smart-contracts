import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}







/**
 * Groups `initialize`'s subordinate-module wiring into one argument —
 * clippy's `too_many_arguments` lint (max 7) is why this exists as a
 * struct rather than five more positional parameters; see `initialize`'s
 * doc comment for what each field actually does.
 */
export interface InitConfig {
  initial_adapters: Map<string, string>;
  initial_executor: string;
  intent_registry: string;
  policy_engine: string;
  recovery_manager: string;
}




export interface AccountStatus {
  frozen: boolean;
  initialized: boolean;
  paused: boolean;
  policy_version_hint: u32;
}




/**
 * Mirror of `intent_registry::ScheduledIntent`'s constructor shape (field
 * names/types must match structurally for cross-contract XDR decoding).
 */
export interface ScheduledIntentArgs {
  adapter: string;
  amount: i128;
  asset: string;
  cancelled: boolean;
  destination: string;
  end_ledger: u32;
  execution_count: u32;
  intent_id: Buffer;
  interval_ledgers: u32;
  max_executions: u32;
  policy_version: u32;
  start_ledger: u32;
}




export const SmartAccountTreasuryError = {
  8000: {message:"AlreadyInitialized"},
  8001: {message:"NotInitialized"},
  8002: {message:"Paused"},
  8003: {message:"Frozen"},
  8004: {message:"AdapterNotConfigured"},
  8005: {message:"NonceAlreadyUsed"},
  8006: {message:"InvalidAmount"},
  8007: {message:"RecipientAmountLengthMismatch"},
  8008: {message:"EmptySplit"},
  8009: {message:"RecoveryNotFinalized"},
  8010: {message:"RecoveryAlreadyApplied"},
  8011: {message:"Unauthorized"},
  8012: {message:"GuardianFreezeNotRequested"},
  8013: {message:"DuplicateRecipient"},
  8014: {message:"NoPendingAdapterChange"},
  8015: {message:"AdapterChangeDelayNotElapsed"}
}

/**
 * Context of a single authorized call performed by an address.
 * 
 * Custom account contracts that implement `__check_auth` special function
 * receive a list of `Context` values corresponding to all the calls that
 * need to be authorized.
 */
export type Context = {tag: "Contract", values: readonly [ContractContext]} | {tag: "CreateContractHostFn", values: readonly [CreateContractHostFnContext]} | {tag: "CreateContractWithCtorHostFn", values: readonly [CreateContractWithConstructorHostFnContext]};


/**
 * Authorization context of a single contract call.
 * 
 * This struct corresponds to a `require_auth_for_args` call for an address
 * from `contract` function with `fn_name` name and `args` arguments.
 */
export interface ContractContext {
  args: Array<any>;
  contract: string;
  fn_name: string;
}

/**
 * Contract executable used for creating a new contract and used in
 * `CreateContractHostFnContext`.
 */
export type ContractExecutable = {tag: "Wasm", values: readonly [Buffer]};


/**
 * Authorization context for `create_contract` host function that creates a
 * new contract on behalf of authorizer address.
 */
export interface CreateContractHostFnContext {
  executable: ContractExecutable;
  salt: Buffer;
}


/**
 * Authorization context for `create_contract` host function that creates a
 * new contract on behalf of authorizer address.
 * This is the same as `CreateContractHostFnContext`, but also has
 * contract constructor arguments.
 */
export interface CreateContractWithConstructorHostFnContext {
  constructor_args: Array<any>;
  executable: ContractExecutable;
  salt: Buffer;
}

export const RoleTransferError = {
  2200: {message:"NoPendingTransfer"},
  2201: {message:"InvalidLiveUntilLedger"},
  2202: {message:"InvalidPendingAccount"},
  2203: {message:"TransferExpired"}
}

export const OwnableError = {
  2100: {message:"OwnerNotSet"},
  2101: {message:"TransferInProgress"},
  2102: {message:"OwnerAlreadySet"}
}











/**
 * Error codes for smart account operations.
 */
export const SmartAccountError = {
  /**
   * The specified context rule does not exist.
   */
  3000: {message:"ContextRuleNotFound"},
  /**
   * The provided context cannot be validated against any rule.
   */
  3002: {message:"UnvalidatedContext"},
  /**
   * External signature verification failed.
   */
  3003: {message:"ExternalVerificationFailed"},
  /**
   * Context rule must have at least one signer or policy.
   */
  3004: {message:"NoSignersAndPolicies"},
  /**
   * The valid_until timestamp is in the past.
   */
  3005: {message:"PastValidUntil"},
  /**
   * The specified signer was not found.
   */
  3006: {message:"SignerNotFound"},
  /**
   * The signer already exists in the context rule.
   */
  3007: {message:"DuplicateSigner"},
  /**
   * The specified policy was not found.
   */
  3008: {message:"PolicyNotFound"},
  /**
   * The policy already exists in the context rule.
   */
  3009: {message:"DuplicatePolicy"},
  /**
   * Too many signers in the context rule.
   */
  3010: {message:"TooManySigners"},
  /**
   * Too many policies in the context rule.
   */
  3011: {message:"TooManyPolicies"},
  /**
   * An internal ID counter (context rule, signer, or policy) has reached
   * its maximum value (`u32::MAX`) and cannot be incremented further.
   */
  3012: {message:"MathOverflow"},
  /**
   * External signer key data exceeds the maximum allowed size.
   */
  3013: {message:"KeyDataTooLarge"},
  /**
   * context_rule_ids length does not match auth_contexts length.
   */
  3014: {message:"ContextRuleIdsLengthMismatch"},
  /**
   * Context rule name exceeds the maximum allowed length.
   */
  3015: {message:"NameTooLong"},
  /**
   * A signer in `AuthPayload` is not part of any selected context rule.
   */
  3016: {message:"UnauthorizedSigner"}
}





/**
 * Represents different types of signers in the smart account system.
 */
export type Signer = {tag: "Delegated", values: readonly [string]} | {tag: "External", values: readonly [string, Buffer]};


/**
 * The authorization payload passed to `__check_auth`, bundling cryptographic
 * proofs with context rule selection.
 * 
 * This struct carries two distinct pieces of information that are both
 * required for authorization but cannot be derived from each other:
 * 
 * - `signers` maps each [`Signer`] to its raw signature bytes, providing
 * cryptographic proof that the signer actually signed the transaction
 * payload. A context rule stores which signer *identities* are authorized
 * (via `signer_ids`), but the rule does not contain the signatures
 * themselves — those must be supplied here.
 * 
 * - `context_rule_ids` tells the system which rule to validate for each auth
 * context. Because multiple rules can exist for the same context type, the
 * caller must explicitly select one per context rather than relying on
 * auto-discovery. Each entry is aligned by index with the `auth_contexts`
 * passed to `__check_auth`.
 * 
 * The length of `context_rule_ids` must equal the number of auth contexts;
 * a mismatch is rejected with
 * [`SmartAccountError::ContextRuleIdsLen
 */
export interface AuthPayload {
  /**
 * Per-context rule IDs, aligned by index with `auth_contexts`.
 */
context_rule_ids: Array<u32>;
  /**
 * Signature data mapped to each signer.
 */
signers: Map<Signer, Buffer>;
}


/**
 * A complete context rule defining authorization requirements.
 */
export interface ContextRule {
  /**
 * The type of context this rule applies to.
 */
context_type: ContextRuleType;
  /**
 * Unique identifier for the context rule.
 */
id: u32;
  /**
 * Human-readable name for the context rule.
 */
name: string;
  /**
 * List of policy contracts that must be satisfied.
 */
policies: Array<string>;
  /**
 * Global registry IDs for each policy, positionally aligned with
 * `policies`.
 */
policy_ids: Array<u32>;
  /**
 * Global registry IDs for each signer, positionally aligned with
 * `signers`.
 */
signer_ids: Array<u32>;
  /**
 * List of signers authorized by this rule.
 */
signers: Array<Signer>;
  /**
 * Optional expiration ledger sequence for the rule.
 */
valid_until: Option<u32>;
}

/**
 * Types of contexts that can be authorized by smart account rules.
 */
export type ContextRuleType = {tag: "Default", values: void} | {tag: "CallContract", values: readonly [string]} | {tag: "CreateContract", values: readonly [Buffer]};



export const PausableError = {
  /**
   * The operation failed because the contract is paused.
   */
  1000: {message:"EnforcedPause"},
  /**
   * The operation failed because the contract is not paused.
   */
  1001: {message:"ExpectedPause"}
}

export interface Client {
  /**
   * Construct and simulate a pause transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * The `Pausable` trait signature returns `()`, not `Result`, so a
   * caller/owner mismatch can only be signaled by aborting — done via
   * `panic_with_error!` with this contract's own typed error (matching
   * every other failure path here) rather than a bare string `panic!`,
   * which surfaced as an untyped trap a client couldn't match against
   * the way it can `Error(Contract, #8011)`.
   */
  pause: ({caller}: {caller: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a freeze transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Emergency freeze. One-way by design: there is no `unfreeze()`
   * entrypoint. Normal operation is restored only via `apply_recovery`,
   * so a compromised owner cannot both freeze the account and later
   * unfreeze it unilaterally without going through guardian quorum.
   */
  freeze: (options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a paused transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns true if the contract is paused, and false otherwise.
   * 
   * # Arguments
   * 
   * * `e` - Access to Soroban environment.
   */
  paused: (options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a status transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  status: (options?: MethodOptions) => Promise<AssembledTransaction<Result<AccountStatus>>>

  /**
   * Construct and simulate a unpause transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  unpause: ({caller}: {caller: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a get_owner transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns `Some(Address)` if ownership is set, or `None` if ownership has
   * been renounced.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   */
  get_owner: (options?: MethodOptions) => Promise<AssembledTransaction<Option<string>>>

  /**
   * Construct and simulate a add_policy transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Adds a new policy to an existing context rule, installs it, and returns
   * the assigned policy ID. The policy's `install` method will be called
   * during this operation.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The ID of the context rule to modify.
   * * `policy` - The address of the policy contract to add.
   * * `install_param` - The installation parameter for the policy.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   * * [`SmartAccountError::DuplicatePolicy`] - When the policy already
   * exists in the rule.
   * * [`SmartAccountError::TooManyPolicies`] - When adding would exceed
   * MAX_POLICIES (5).
   * 
   * # Events
   * 
   * * topics - `["policy_added", context_rule_id: u32]`
   * * data - `[policy_id: u32]`
   * 
   * # Notes
   * 
   * Defaults to requiring authorization from the smart account itself
   * (`e.current_contract_address().require_auth()`) and then delegating to
   * [`storage::add_policy`].
   */
  add_policy: ({context_rule_id, policy, install_param}: {context_rule_id: u32, policy: string, install_param: any}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a add_signer transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Adds a new signer to an existing context rule, returning the assigned
   * signer ID.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The ID of the context rule to modify.
   * * `signer` - The signer to add to the context rule.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   * * [`SmartAccountError::DuplicateSigner`] - When the signer already
   * exists in the rule.
   * * [`SmartAccountError::TooManySigners`] - When adding would exceed
   * MAX_SIGNERS (15).
   * 
   * # Events
   * 
   * * topics - `["signer_added", context_rule_id: u32]`
   * * data - `[signer_id: u32]`
   * 
   * # Notes
   * 
   * Defaults to requiring authorization from the smart account itself
   * (`e.current_contract_address().require_auth()`) and then delegating to
   * [`storage::add_signer`].
   */
  add_signer: ({context_rule_id, signer}: {context_rule_id: u32, signer: Signer}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Bootstraps the treasury: sets the owner, seeds a `Default` context
   * rule with the founding signers/policies (e.g. wallet signers plus a
   * `weighted_threshold` policy, or passkey `External` signers), pins the
   * subordinate module addresses, and bootstraps `intent_registry` in the
   * same call (see below).
   * 
   * This calls the OZ storage primitives directly (`ownable::set_owner`,
   * `add_context_rule`) instead of going through their `Ownable`/
   * `SmartAccount` trait entrypoints, both of which require an
   * already-authorized owner/signer to exist — impossible to satisfy
   * before the very first signer is registered.
   * 
   * `intent_registry` must be a **freshly deployed, not-yet-initialized**
   * `IntentRegistry` instance. This function initializes it directly,
   * passing `env.current_contract_address()` (this treasury) as its
   * `admin` — which is the whole reason it needs to happen here rather
   * than as a separate call from outside. `IntentRegistry::initialize`
   * requires `admin.require_auth()`; with `admin` set to this treasury's
   * own address and th
   */
  initialize: ({owner, initial_signers, initial_policies, config}: {owner: string, initial_signers: Array<Signer>, initial_policies: Map<string, any>, config: InitConfig}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a contract_name transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  contract_name: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a get_policy_id transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Retrieves the global registry ID for a policy.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `policy` - The policy address to look up.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::PolicyNotFound`] - When the policy is not
   * registered in the global registry.
   */
  get_policy_id: ({policy}: {policy: string}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a get_signer_id transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Retrieves the global registry ID for a signer.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `signer` - The signer to look up.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::SignerNotFound`] - When the signer is not
   * registered in the global registry.
   */
  get_signer_id: ({signer}: {signer: Signer}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a is_nonce_used transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_nonce_used: ({nonce}: {nonce: u64}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a remove_policy transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Removes a policy from an existing context rule and uninstalls it. The
   * policy's `uninstall` method will be called during this operation.
   * Removing the last policy is allowed only if the rule has at least
   * one signer.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The ID of the context rule to modify.
   * * `policy_id` - The ID of the policy to remove from the context rule.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   * * [`SmartAccountError::PolicyNotFound`] - When the policy doesn't exist
   * in the rule.
   * 
   * # Events
   * 
   * * topics - `["policy_removed", context_rule_id: u32]`
   * * data - `[policy_id: u32]`
   * 
   * # Notes
   * 
   * Defaults to requiring authorization from the smart account itself
   * (`e.current_contract_address().require_auth()`) and then delegating to
   * [`storage::remove_policy`].
   */
  remove_policy: ({context_rule_id, policy_id}: {context_rule_id: u32, policy_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a remove_signer transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Removes a signer from an existing context rule. Removing the last signer
   * is allowed only if the rule has at least one policy.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The ID of the context rule to modify.
   * * `signer_id` - The ID of the signer to remove from the context rule.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   * * [`SmartAccountError::SignerNotFound`] - When the signer doesn't exist
   * in the rule.
   * 
   * # Events
   * 
   * * topics - `["signer_removed", context_rule_id: u32]`
   * * data - `[signer_id: u32]`
   * 
   * # Notes
   * 
   * Defaults to requiring authorization from the smart account itself
   * (`e.current_contract_address().require_auth()`) and then delegating to
   * [`storage::remove_signer`].
   */
  remove_signer: ({context_rule_id, signer_id}: {context_rule_id: u32, signer_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a apply_recovery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Pulls a finalized recovery outcome from the configured
   * `recovery_manager` and applies it: force-overwrites ownership,
   * replaces the *entire* signer/context-rule topology with the
   * guardian-approved replacement, syncs the guardian-freeze epoch, and
   * lifts the freeze. Never pushed by `recovery_manager` — this
   * contract decides, on its own terms, whether to consume a finalized
   * request, exactly once (`AppliedRecovery` replay guard).
   * Permissionless: anyone may call this once `recovery_manager`
   * reports the request finalized, mirroring `finalize_recovery`'s own
   * permissionless design.
   * 
   * Security review finding, two parts. First: this used to overwrite
   * only `Ownable`'s `owner`. Spend authority — `execute_transfer_payment`
   * and everything else gated by `e.current_contract_address()
   * .require_auth()` — is the OZ context-rule/signer registry, a
   * completely separate system `owner` never gates (see
   * `docs/GOVERNANCE_MULTISIG_DESIGN.md` §9). A compromised *payment
   * signer* — the more likely real-world compromise, not the own
   */
  apply_recovery: ({request_id}: {request_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<string>>>

  /**
   * Construct and simulate a accept_ownership transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Accepts a pending ownership transfer.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * 
   * # Errors
   * 
   * * [`crate::role_transfer::RoleTransferError::NoPendingTransfer`] - If
   * there is no pending transfer to accept.
   * 
   * # Events
   * 
   * * topics - `["ownership_transfer_completed"]`
   * * data - `[new_owner: Address]`
   */
  accept_ownership: (options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a add_context_rule transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Creates a new context rule with the specified configuration, returning
   * the newly created `ContextRule` with a unique ID assigned. Installs
   * all specified policies during creation.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_type` - The type of context this rule applies to.
   * * `name` - Human-readable name for the context rule.
   * * `valid_until` - Optional expiration ledger sequence.
   * * `signers` - List of signers authorized by this rule.
   * * `policies` - Map of policy addresses to their installation parameters.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::NoSignersAndPolicies`] - When both signers and
   * policies are empty.
   * * [`SmartAccountError::TooManySigners`] - When signers exceed
   * MAX_SIGNERS (15).
   * * [`SmartAccountError::TooManyPolicies`] - When policies exceed
   * MAX_POLICIES (5).
   * * [`SmartAccountError::DuplicateSigner`] - When the same signer appears
   * multiple times.
   * * [`SmartAccountError::PastValidUntil`] - When valid_until is in the
   * past.
   * * [`SmartAccountError::MathOverflow`] - When the context rule, si
   */
  add_context_rule: ({context_type, name, valid_until, signers, policies}: {context_type: ContextRuleType, name: string, valid_until: Option<u32>, signers: Array<Signer>, policies: Map<string, any>}, options?: MethodOptions) => Promise<AssembledTransaction<ContextRule>>

  /**
   * Construct and simulate a get_context_rule transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Retrieves a context rule by its unique ID, returning the
   * `ContextRule` containing all metadata, signers, and policies.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The unique identifier of the context rule to
   * retrieve.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   */
  get_context_rule: ({context_rule_id}: {context_rule_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<ContextRule>>

  /**
   * Construct and simulate a renounce_ownership transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Renounces ownership of the contract.
   * 
   * Permanently removes the owner, disabling all functions gated by
   * `#[only_owner]`.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * 
   * # Errors
   * 
   * * [`OwnableError::TransferInProgress`] - If there is a pending ownership
   * transfer.
   * * [`OwnableError::OwnerNotSet`] - If the owner is not set.
   * 
   * # Notes
   * 
   * * Authorization for the current owner is required.
   */
  renounce_ownership: (options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a transfer_ownership transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Initiates a 2-step ownership transfer to a new address.
   * 
   * Requires authorization from the current owner. The new owner must later
   * call `accept_ownership()` to complete the transfer.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `new_owner` - The proposed new owner.
   * * `live_until_ledger` - Ledger number until which the new owner can
   * accept. A value of `0` cancels any pending transfer.
   * 
   * # Errors
   * 
   * * [`OwnableError::OwnerNotSet`] - If the owner is not set.
   * * [`crate::role_transfer::RoleTransferError::NoPendingTransfer`] - If
   * trying to cancel a transfer that doesn't exist.
   * * [`crate::role_transfer::RoleTransferError::InvalidLiveUntilLedger`] -
   * If the specified ledger is in the past.
   * * [`crate::role_transfer::RoleTransferError::InvalidPendingAccount`] -
   * If the specified pending account is not the same as the provided `new`
   * address.
   * 
   * # Notes
   * 
   * * Authorization for the current owner is required.
   */
  transfer_ownership: ({new_owner, live_until_ledger}: {new_owner: string, live_until_ledger: u32}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a extend_instance_ttl transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless TTL maintenance for this contract's instance storage
   * (which `PendingAdapter` lives in, among other instance-scoped
   * config) — extending TTL creates no authority, matching
   * `recovery_manager::extend_ttl`'s reasoning. Independent security
   * review finding: `propose_adapter_change` only bumps the instance TTL
   * once, at proposal time; a proposal left unapplied long enough after
   * its delay elapses, on an otherwise fully dormant treasury, could
   * archive before `apply_adapter_change` is ever called. This lets
   * anyone keep it alive without needing the owner to act.
   */
  extend_instance_ttl: (options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a remove_context_rule transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Removes a context rule and cleans up all associated data. This function
   * uninstalls all policies associated with the rule and removes all stored
   * data including signers, policies, and metadata.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The ID of the context rule to remove.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   * 
   * # Events
   * 
   * * topics - `["context_rule_removed", context_rule_id: u32]`
   * * data - `[]`
   * 
   * # Notes
   * 
   * Defaults to requiring authorization from the smart account itself
   * (`e.current_contract_address().require_auth()`) and then delegating to
   * [`storage::remove_context_rule`].
   */
  remove_context_rule: ({context_rule_id}: {context_rule_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a apply_adapter_change transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Applies a pending adapter change once its delay has elapsed.
   * Permissionless, like `apply_recovery`/`apply_guardian_freeze`: the
   * authorization decision (the owner proposing this specific change)
   * already happened at `propose_adapter_change` time, so *when* an
   * already-authorized change actually lands needs no further gate to
   * be safe — it only needs the delay to have passed.
   */
  apply_adapter_change: ({operation}: {operation: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a apply_guardian_freeze transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Real gap this session found: `docs/TECHNICAL_ARCHITECTURE.md` §12.7
   * documents the canonical recovery workflow as starting with "Guardian
   * ... triggers freeze," but `freeze()` above is owner-gated only, with
   * no guardian-facing path at all — if the owner is the one compromised,
   * guardians had no way to stop ongoing spend authority while a full
   * recovery (quorum + timelock) is still pending. This pulls a
   * guardian-raised freeze flag from `recovery_manager` the same way
   * `apply_recovery` pulls a finalized request: permissionlessly, on
   * this contract's own terms, with `recovery_manager` carrying no
   * knowledge of or dependency on this treasury.
   * 
   * Security review finding, twice over. First revision: this called
   * `recovery_manager`'s read-only `guardian_freeze_requested`, a flag
   * set once and never cleared — since this entrypoint is deliberately
   * permissionless (matching `apply_recovery`'s "pull an
   * already-authorized fact" pattern), anyone could re-freeze the
   * treasury at any future point, even long after a full recovery ha
   */
  apply_guardian_freeze: (options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a cancel_adapter_change transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Cancels a pending adapter change before it takes effect. Owner-gated,
   * mirroring `recovery_manager::cancel_recovery`'s pattern of letting
   * the legitimate owner walk back an erroneous or no-longer-wanted
   * proposal before its delay elapses.
   */
  cancel_adapter_change: ({operation}: {operation: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a execute_split_payment transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Executes a one-to-many SAC split from the treasury. Each recipient
   * is independently policy-checked (own recipient-allowlist entry, own
   * per-recipient amount cap) before the adapter call, so a split cannot
   * be used to move funds to a disallowed recipient by hiding it inside
   * an aggregate-approved total.
   */
  execute_split_payment: ({asset, recipients, amounts, nonce, expected_policy_version}: {asset: string, recipients: Array<string>, amounts: Array<i128>, nonce: u64, expected_policy_version: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a propose_adapter_change transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Proposes rewiring which adapter contract handles a given operation.
   * Owner-gated. Unlike `policy_engine` / `intent_registry` /
   * `recovery_manager` (pinned at `initialize`, see Architecture
   * Decision in `docs/TECHNICAL_ARCHITECTURE.md` §6.9), adapters are
   * mutable post-init because the doc explicitly scopes
   * `set_adapter_config` as an owner-configurable entrypoint.
   * 
   * This does not take effect immediately — see `apply_adapter_change`.
   * An independent security review (`docs/SMART_CONTRACT_AUDIT_REPORT.md`
   * finding 3) noted that immediate, undelayed reconfiguration of
   * execution routing is a meaningful risk for a treasury contract
   * specifically (it decides *where funds go*, unlike e.g. `freeze`,
   * which only stops movement and is deliberately kept immediate).
   * Overwrites any existing pending proposal for the same operation —
   * the most recent proposal wins, matching how re-proposing normally
   * works elsewhere in this workspace (e.g. `policy_engine::set_asset_rule`).
   * 
   * Explicitly extends the instance TTL on write. Review findi
   */
  propose_adapter_change: ({operation, adapter}: {operation: string, adapter: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_context_rules_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Retrieves the number of all context rules, including expired rules.
   * Defaults to 0.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   */
  get_context_rules_count: (options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a cancel_scheduled_payment transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Cancels a previously created scheduled payment. Requires this
   * contract's own authorization, same as creation.
   * 
   * This closes a real gap: `intent_registry::cancel_intent` exists and
   * is `ensure_admin`-gated, but `intent_registry`'s configured admin is
   * this contract's own address — before this entrypoint existed, there
   * was no way for anyone, including the treasury's own signers, to ever
   * call it, since only `smart_account` itself can satisfy that admin
   * check via a nested sub-invocation.
   */
  cancel_scheduled_payment: ({intent_id}: {intent_id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a create_scheduled_payment transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Creates a scheduled/recurring payment intent. Requires this
   * contract's own authorization (same signer/context-rule/policy model
   * as interactive payments), then delegates canonical lifecycle and
   * replay state to `intent_registry` — see Architecture Decision 2.
   * `intent_registry` must have been configured with this contract's
   * own address as its admin, so this nested call succeeds via
   * sub-invocation tree membership rather than a second signature.
   * 
   * The caller-supplied `policy_version` and `adapter` on `intent` are
   * both ignored and overwritten: `policy_version` with `policy_engine`'s
   * current version, and `adapter` with the `transfer_adapter` currently
   * configured (via `propose_adapter_change`/`apply_adapter_change`).
   * Both must reflect what was actually in
   * effect when the signer approved this schedule, not a value the
   * caller chose — otherwise a later adapter reconfiguration could
   * silently redirect an already-approved scheduled payment through a
   * different adapter at execution time (see `execute_scheduled_payment`
   * fo
   */
  create_scheduled_payment: ({intent}: {intent: ScheduledIntentArgs}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a execute_transfer_payment transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Executes a single-recipient SAC payment. Requires this contract's
   * own authorization (delegated entirely to the OZ context-rule/signer/
   * policy model via `__check_auth`), then treasury policy, then a
   * narrowly preauthorized adapter call.
   */
  execute_transfer_payment: ({asset, destination, amount, nonce, expected_policy_version}: {asset: string, destination: string, amount: i128, nonce: u64, expected_policy_version: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a update_context_rule_name transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Updates the name of an existing context rule, returning the updated
   * `ContextRule` with the new name.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The ID of the context rule to update.
   * * `name` - The new human-readable name for the context rule.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   * 
   * # Events
   * 
   * * topics - `["context_rule_meta_updated", context_rule_id: u32]`
   * * data - `[name: String, context_type: ContextRuleType, valid_until:
   * Option<u32>]`
   * 
   * # Notes
   * 
   * Defaults to requiring authorization from the smart account itself
   * (`e.current_contract_address().require_auth()`) and then delegating to
   * [`storage::update_context_rule_name`].
   */
  update_context_rule_name: ({context_rule_id, name}: {context_rule_id: u32, name: string}, options?: MethodOptions) => Promise<AssembledTransaction<ContextRule>>

  /**
   * Construct and simulate a execute_scheduled_payment transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Executes an already-approved scheduled payment. Permissionless by
   * design (matches `recovery_manager::finalize_recovery`): the signer
   * approval already happened at `create_scheduled_payment` time. Safety
   * here comes entirely from `intent_registry`'s own execution-window,
   * cancellation, and cumulative-usage checks — not from caller
   * identity — except that `intent_registry.mark_child_executed` itself
   * still requires its configured `Executor` address to authorize the
   * call, so an operational relayer key (not a treasury signer) gates
   * *when* within the valid window execution happens.
   * 
   * Deliberately takes only `intent_id` and `child_sequence` from the
   * caller. An earlier revision also accepted `asset`, `destination`,
   * `amount`, and `expected_policy_version` as caller-supplied
   * parameters and used them directly for the policy check and adapter
   * dispatch — since this entrypoint has no `require_auth()` gate of its
   * own (by design, see above) and the relayer role is explicitly
   * untrusted (`docs/TECHNICAL_ARCHITECTURE.md` §13.
   */
  execute_scheduled_payment: ({intent_id, child_sequence}: {intent_id: Buffer, child_sequence: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a update_context_rule_valid_until transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Updates the expiration time of an existing context rule, returning the
   * updated `ContextRule` with the new expiration time.
   * 
   * # Arguments
   * 
   * * `e` - Access to the Soroban environment.
   * * `context_rule_id` - The ID of the context rule to update.
   * * `valid_until` - New optional expiration ledger sequence. Use `None`
   * for no expiration.
   * 
   * # Errors
   * 
   * * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
   * exists with the given ID.
   * * [`SmartAccountError::PastValidUntil`] - When valid_until is in the
   * past.
   * 
   * # Events
   * 
   * * topics - `["context_rule_meta_updated", context_rule_id: u32]`
   * * data - `[name: String, context_type: ContextRuleType, valid_until:
   * Option<u32>]`
   * 
   * # Notes
   * 
   * Defaults to requiring authorization from the smart account itself
   * (`e.current_contract_address().require_auth()`) and then delegating to
   * [`storage::update_context_rule_valid_until`].
   */
  update_context_rule_valid_until: ({context_rule_id, valid_until}: {context_rule_id: u32, valid_until: Option<u32>}, options?: MethodOptions) => Promise<AssembledTransaction<ContextRule>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions &
      Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy(null, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAABQAAAAAAAAAAAAAABkZyb3plbgAAAAAAAQAAAAZmcm96ZW4AAAAAAAEAAAD1RGlzdGluZ3Vpc2hlcyBhbiBvd25lci10cmlnZ2VyZWQgYGZyZWV6ZSgpYCBmcm9tIGEgZ3VhcmRpYW4tcHVsbGVkCmBhcHBseV9ndWFyZGlhbl9mcmVlemUoKWAg4oCUIGJvdGggdXNlZCB0byBwdWJsaXNoIGFuIGlkZW50aWNhbCwKZGF0YS1mcmVlIGV2ZW50LCB3aGljaCBtYWRlIGl0IGltcG9zc2libGUgdG8gdGVsbCB3aGljaCBwYXRoCmFjdHVhbGx5IGZyb3plIHRoZSBhY2NvdW50IGZyb20gdGhlIGV2ZW50IGxvZyBhbG9uZS4AAAAAAAAVdHJpZ2dlcmVkX2J5X2d1YXJkaWFuAAAAAAAAAQAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAACVNwbGl0UGFpZAAAAAAAAAEAAAAHc3BsdF9vawAAAAADAAAAAAAAAAVhc3NldAAAAAAAABMAAAABAAAAAAAAAA9yZWNpcGllbnRfY291bnQAAAAABAAAAAAAAAAAAAAABW5vbmNlAAAAAAAABgAAAAAAAAAC",
        "AAAAAQAAAP5Hcm91cHMgYGluaXRpYWxpemVgJ3Mgc3Vib3JkaW5hdGUtbW9kdWxlIHdpcmluZyBpbnRvIG9uZSBhcmd1bWVudCDigJQKY2xpcHB5J3MgYHRvb19tYW55X2FyZ3VtZW50c2AgbGludCAobWF4IDcpIGlzIHdoeSB0aGlzIGV4aXN0cyBhcyBhCnN0cnVjdCByYXRoZXIgdGhhbiBmaXZlIG1vcmUgcG9zaXRpb25hbCBwYXJhbWV0ZXJzOyBzZWUgYGluaXRpYWxpemVgJ3MKZG9jIGNvbW1lbnQgZm9yIHdoYXQgZWFjaCBmaWVsZCBhY3R1YWxseSBkb2VzLgAAAAAAAAAAAApJbml0Q29uZmlnAAAAAAAFAAAAAAAAABBpbml0aWFsX2FkYXB0ZXJzAAAD7AAAABEAAAATAAAAAAAAABBpbml0aWFsX2V4ZWN1dG9yAAAAEwAAAAAAAAAPaW50ZW50X3JlZ2lzdHJ5AAAAABMAAAAAAAAADXBvbGljeV9lbmdpbmUAAAAAAAATAAAAAAAAABByZWNvdmVyeV9tYW5hZ2VyAAAAEw==",
        "AAAABQAAAAAAAAAAAAAAC0luaXRpYWxpemVkAAAAAAEAAAAEaW5pdAAAAAEAAAAAAAAABW93bmVyAAAAAAAAEwAAAAEAAAAC",
        "AAAABQAAAAAAAAAAAAAADFRyYW5zZmVyUGFpZAAAAAEAAAAGcGF5X29rAAAAAAAEAAAAAAAAAAVhc3NldAAAAAAAABMAAAABAAAAAAAAAAtkZXN0aW5hdGlvbgAAAAATAAAAAQAAAAAAAAAGYW1vdW50AAAAAAALAAAAAAAAAAAAAAAFbm9uY2UAAAAAAAAGAAAAAAAAAAI=",
        "AAAAAQAAAAAAAAAAAAAADUFjY291bnRTdGF0dXMAAAAAAAAEAAAAAAAAAAZmcm96ZW4AAAAAAAEAAAAAAAAAC2luaXRpYWxpemVkAAAAAAEAAAAAAAAABnBhdXNlZAAAAAAAAQAAAAAAAAATcG9saWN5X3ZlcnNpb25faGludAAAAAAE",
        "AAAABQAAAAAAAAAAAAAADkFkYXB0ZXJDaGFuZ2VkAAAAAAABAAAAB2FkYXB0ZXIAAAAAAgAAAAAAAAAJb3BlcmF0aW9uAAAAAAAAEQAAAAEAAAAAAAAAB2FkYXB0ZXIAAAAAEwAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAAD1JlY292ZXJ5QXBwbGllZAAAAAABAAAAB3JlY292ZXIAAAAAAgAAAAAAAAAKcmVxdWVzdF9pZAAAAAAD7gAAACAAAAABAAAAAAAAABFyZXBsYWNlbWVudF9vd25lcgAAAAAAABMAAAAAAAAAAg==",
        "AAAAAQAAAI1NaXJyb3Igb2YgYGludGVudF9yZWdpc3RyeTo6U2NoZWR1bGVkSW50ZW50YCdzIGNvbnN0cnVjdG9yIHNoYXBlIChmaWVsZApuYW1lcy90eXBlcyBtdXN0IG1hdGNoIHN0cnVjdHVyYWxseSBmb3IgY3Jvc3MtY29udHJhY3QgWERSIGRlY29kaW5nKS4AAAAAAAAAAAAAE1NjaGVkdWxlZEludGVudEFyZ3MAAAAADAAAAAAAAAAHYWRhcHRlcgAAAAATAAAAAAAAAAZhbW91bnQAAAAAAAsAAAAAAAAABWFzc2V0AAAAAAAAEwAAAAAAAAAJY2FuY2VsbGVkAAAAAAAAAQAAAAAAAAALZGVzdGluYXRpb24AAAAAEwAAAAAAAAAKZW5kX2xlZGdlcgAAAAAABAAAAAAAAAAPZXhlY3V0aW9uX2NvdW50AAAAAAQAAAAAAAAACWludGVudF9pZAAAAAAAA+4AAAAgAAAAAAAAABBpbnRlcnZhbF9sZWRnZXJzAAAABAAAAAAAAAAObWF4X2V4ZWN1dGlvbnMAAAAAAAQAAAAAAAAADnBvbGljeV92ZXJzaW9uAAAAAAAEAAAAAAAAAAxzdGFydF9sZWRnZXIAAAAE",
        "AAAAAAAAAXRUaGUgYFBhdXNhYmxlYCB0cmFpdCBzaWduYXR1cmUgcmV0dXJucyBgKClgLCBub3QgYFJlc3VsdGAsIHNvIGEKY2FsbGVyL293bmVyIG1pc21hdGNoIGNhbiBvbmx5IGJlIHNpZ25hbGVkIGJ5IGFib3J0aW5nIOKAlCBkb25lIHZpYQpgcGFuaWNfd2l0aF9lcnJvciFgIHdpdGggdGhpcyBjb250cmFjdCdzIG93biB0eXBlZCBlcnJvciAobWF0Y2hpbmcKZXZlcnkgb3RoZXIgZmFpbHVyZSBwYXRoIGhlcmUpIHJhdGhlciB0aGFuIGEgYmFyZSBzdHJpbmcgYHBhbmljIWAsCndoaWNoIHN1cmZhY2VkIGFzIGFuIHVudHlwZWQgdHJhcCBhIGNsaWVudCBjb3VsZG4ndCBtYXRjaCBhZ2FpbnN0CnRoZSB3YXkgaXQgY2FuIGBFcnJvcihDb250cmFjdCwgIzgwMTEpYC4AAAAFcGF1c2UAAAAAAAABAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAA",
        "AAAAAAAAAQFFbWVyZ2VuY3kgZnJlZXplLiBPbmUtd2F5IGJ5IGRlc2lnbjogdGhlcmUgaXMgbm8gYHVuZnJlZXplKClgCmVudHJ5cG9pbnQuIE5vcm1hbCBvcGVyYXRpb24gaXMgcmVzdG9yZWQgb25seSB2aWEgYGFwcGx5X3JlY292ZXJ5YCwKc28gYSBjb21wcm9taXNlZCBvd25lciBjYW5ub3QgYm90aCBmcmVlemUgdGhlIGFjY291bnQgYW5kIGxhdGVyCnVuZnJlZXplIGl0IHVuaWxhdGVyYWxseSB3aXRob3V0IGdvaW5nIHRocm91Z2ggZ3VhcmRpYW4gcXVvcnVtLgAAAAAAAAZmcmVlemUAAAAAAAAAAAABAAAD6QAAAAIAAAfQAAAAGVNtYXJ0QWNjb3VudFRyZWFzdXJ5RXJyb3IAAAA=",
        "AAAAAAAAAHFSZXR1cm5zIHRydWUgaWYgdGhlIGNvbnRyYWN0IGlzIHBhdXNlZCwgYW5kIGZhbHNlIG90aGVyd2lzZS4KCiMgQXJndW1lbnRzCgoqIGBlYCAtIEFjY2VzcyB0byBTb3JvYmFuIGVudmlyb25tZW50LgAAAAAAAAZwYXVzZWQAAAAAAAAAAAABAAAAAQ==",
        "AAAAAAAAAAAAAAAGc3RhdHVzAAAAAAAAAAAAAQAAA+kAAAfQAAAADUFjY291bnRTdGF0dXMAAAAAAAfQAAAAGVNtYXJ0QWNjb3VudFRyZWFzdXJ5RXJyb3IAAAA=",
        "AAAAAAAAAAAAAAAHdW5wYXVzZQAAAAABAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAA",
        "AAAABQAAAAAAAAAAAAAAFUFkYXB0ZXJDaGFuZ2VQcm9wb3NlZAAAAAAAAAEAAAAIYWRhcHRlcnAAAAADAAAAAAAAAAlvcGVyYXRpb24AAAAAAAARAAAAAQAAAAAAAAAHYWRhcHRlcgAAAAATAAAAAAAAAAAAAAAQZWZmZWN0aXZlX2xlZGdlcgAAAAQAAAAAAAAAAg==",
        "AAAABQAAAAAAAAAAAAAAFkFkYXB0ZXJDaGFuZ2VDYW5jZWxsZWQAAAAAAAEAAAAHYWRhcHRyYwAAAAABAAAAAAAAAAlvcGVyYXRpb24AAAAAAAARAAAAAQAAAAI=",
        "AAAAAAAAAJBSZXR1cm5zIGBTb21lKEFkZHJlc3MpYCBpZiBvd25lcnNoaXAgaXMgc2V0LCBvciBgTm9uZWAgaWYgb3duZXJzaGlwIGhhcwpiZWVuIHJlbm91bmNlZC4KCiMgQXJndW1lbnRzCgoqIGBlYCAtIEFjY2VzcyB0byB0aGUgU29yb2JhbiBlbnZpcm9ubWVudC4AAAAJZ2V0X293bmVyAAAAAAAAAAAAAAEAAAPoAAAAEw==",
        "AAAAAAAAA6xBZGRzIGEgbmV3IHBvbGljeSB0byBhbiBleGlzdGluZyBjb250ZXh0IHJ1bGUsIGluc3RhbGxzIGl0LCBhbmQgcmV0dXJucwp0aGUgYXNzaWduZWQgcG9saWN5IElELiBUaGUgcG9saWN5J3MgYGluc3RhbGxgIG1ldGhvZCB3aWxsIGJlIGNhbGxlZApkdXJpbmcgdGhpcyBvcGVyYXRpb24uCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCiogYGNvbnRleHRfcnVsZV9pZGAgLSBUaGUgSUQgb2YgdGhlIGNvbnRleHQgcnVsZSB0byBtb2RpZnkuCiogYHBvbGljeWAgLSBUaGUgYWRkcmVzcyBvZiB0aGUgcG9saWN5IGNvbnRyYWN0IHRvIGFkZC4KKiBgaW5zdGFsbF9wYXJhbWAgLSBUaGUgaW5zdGFsbGF0aW9uIHBhcmFtZXRlciBmb3IgdGhlIHBvbGljeS4KCiMgRXJyb3JzCgoqIFtgU21hcnRBY2NvdW50RXJyb3I6OkNvbnRleHRSdWxlTm90Rm91bmRgXSAtIFdoZW4gbm8gY29udGV4dCBydWxlCmV4aXN0cyB3aXRoIHRoZSBnaXZlbiBJRC4KKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpEdXBsaWNhdGVQb2xpY3lgXSAtIFdoZW4gdGhlIHBvbGljeSBhbHJlYWR5CmV4aXN0cyBpbiB0aGUgcnVsZS4KKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpUb29NYW55UG9saWNpZXNgXSAtIFdoZW4gYWRkaW5nIHdvdWxkIGV4Y2VlZApNQVhfUE9MSUNJRVMgKDUpLgoKIyBFdmVudHMKCiogdG9waWNzIC0gYFsicG9saWN5X2FkZGVkIiwgY29udGV4dF9ydWxlX2lkOiB1MzJdYAoqIGRhdGEgLSBgW3BvbGljeV9pZDogdTMyXWAKCiMgTm90ZXMKCkRlZmF1bHRzIHRvIHJlcXVpcmluZyBhdXRob3JpemF0aW9uIGZyb20gdGhlIHNtYXJ0IGFjY291bnQgaXRzZWxmCihgZS5jdXJyZW50X2NvbnRyYWN0X2FkZHJlc3MoKS5yZXF1aXJlX2F1dGgoKWApIGFuZCB0aGVuIGRlbGVnYXRpbmcgdG8KW2BzdG9yYWdlOjphZGRfcG9saWN5YF0uAAAACmFkZF9wb2xpY3kAAAAAAAMAAAAAAAAAD2NvbnRleHRfcnVsZV9pZAAAAAAEAAAAAAAAAAZwb2xpY3kAAAAAABMAAAAAAAAADWluc3RhbGxfcGFyYW0AAAAAAAAAAAAAAQAAAAQ=",
        "AAAAAAAAAxVBZGRzIGEgbmV3IHNpZ25lciB0byBhbiBleGlzdGluZyBjb250ZXh0IHJ1bGUsIHJldHVybmluZyB0aGUgYXNzaWduZWQKc2lnbmVyIElELgoKIyBBcmd1bWVudHMKCiogYGVgIC0gQWNjZXNzIHRvIHRoZSBTb3JvYmFuIGVudmlyb25tZW50LgoqIGBjb250ZXh0X3J1bGVfaWRgIC0gVGhlIElEIG9mIHRoZSBjb250ZXh0IHJ1bGUgdG8gbW9kaWZ5LgoqIGBzaWduZXJgIC0gVGhlIHNpZ25lciB0byBhZGQgdG8gdGhlIGNvbnRleHQgcnVsZS4KCiMgRXJyb3JzCgoqIFtgU21hcnRBY2NvdW50RXJyb3I6OkNvbnRleHRSdWxlTm90Rm91bmRgXSAtIFdoZW4gbm8gY29udGV4dCBydWxlCmV4aXN0cyB3aXRoIHRoZSBnaXZlbiBJRC4KKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpEdXBsaWNhdGVTaWduZXJgXSAtIFdoZW4gdGhlIHNpZ25lciBhbHJlYWR5CmV4aXN0cyBpbiB0aGUgcnVsZS4KKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpUb29NYW55U2lnbmVyc2BdIC0gV2hlbiBhZGRpbmcgd291bGQgZXhjZWVkCk1BWF9TSUdORVJTICgxNSkuCgojIEV2ZW50cwoKKiB0b3BpY3MgLSBgWyJzaWduZXJfYWRkZWQiLCBjb250ZXh0X3J1bGVfaWQ6IHUzMl1gCiogZGF0YSAtIGBbc2lnbmVyX2lkOiB1MzJdYAoKIyBOb3RlcwoKRGVmYXVsdHMgdG8gcmVxdWlyaW5nIGF1dGhvcml6YXRpb24gZnJvbSB0aGUgc21hcnQgYWNjb3VudCBpdHNlbGYKKGBlLmN1cnJlbnRfY29udHJhY3RfYWRkcmVzcygpLnJlcXVpcmVfYXV0aCgpYCkgYW5kIHRoZW4gZGVsZWdhdGluZyB0bwpbYHN0b3JhZ2U6OmFkZF9zaWduZXJgXS4AAAAAAAAKYWRkX3NpZ25lcgAAAAAAAgAAAAAAAAAPY29udGV4dF9ydWxlX2lkAAAAAAQAAAAAAAAABnNpZ25lcgAAAAAH0AAAAAZTaWduZXIAAAAAAAEAAAAE",
        "AAAAAAAABABCb290c3RyYXBzIHRoZSB0cmVhc3VyeTogc2V0cyB0aGUgb3duZXIsIHNlZWRzIGEgYERlZmF1bHRgIGNvbnRleHQKcnVsZSB3aXRoIHRoZSBmb3VuZGluZyBzaWduZXJzL3BvbGljaWVzIChlLmcuIHdhbGxldCBzaWduZXJzIHBsdXMgYQpgd2VpZ2h0ZWRfdGhyZXNob2xkYCBwb2xpY3ksIG9yIHBhc3NrZXkgYEV4dGVybmFsYCBzaWduZXJzKSwgcGlucyB0aGUKc3Vib3JkaW5hdGUgbW9kdWxlIGFkZHJlc3NlcywgYW5kIGJvb3RzdHJhcHMgYGludGVudF9yZWdpc3RyeWAgaW4gdGhlCnNhbWUgY2FsbCAoc2VlIGJlbG93KS4KClRoaXMgY2FsbHMgdGhlIE9aIHN0b3JhZ2UgcHJpbWl0aXZlcyBkaXJlY3RseSAoYG93bmFibGU6OnNldF9vd25lcmAsCmBhZGRfY29udGV4dF9ydWxlYCkgaW5zdGVhZCBvZiBnb2luZyB0aHJvdWdoIHRoZWlyIGBPd25hYmxlYC8KYFNtYXJ0QWNjb3VudGAgdHJhaXQgZW50cnlwb2ludHMsIGJvdGggb2Ygd2hpY2ggcmVxdWlyZSBhbgphbHJlYWR5LWF1dGhvcml6ZWQgb3duZXIvc2lnbmVyIHRvIGV4aXN0IOKAlCBpbXBvc3NpYmxlIHRvIHNhdGlzZnkKYmVmb3JlIHRoZSB2ZXJ5IGZpcnN0IHNpZ25lciBpcyByZWdpc3RlcmVkLgoKYGludGVudF9yZWdpc3RyeWAgbXVzdCBiZSBhICoqZnJlc2hseSBkZXBsb3llZCwgbm90LXlldC1pbml0aWFsaXplZCoqCmBJbnRlbnRSZWdpc3RyeWAgaW5zdGFuY2UuIFRoaXMgZnVuY3Rpb24gaW5pdGlhbGl6ZXMgaXQgZGlyZWN0bHksCnBhc3NpbmcgYGVudi5jdXJyZW50X2NvbnRyYWN0X2FkZHJlc3MoKWAgKHRoaXMgdHJlYXN1cnkpIGFzIGl0cwpgYWRtaW5gIOKAlCB3aGljaCBpcyB0aGUgd2hvbGUgcmVhc29uIGl0IG5lZWRzIHRvIGhhcHBlbiBoZXJlIHJhdGhlcgp0aGFuIGFzIGEgc2VwYXJhdGUgY2FsbCBmcm9tIG91dHNpZGUuIGBJbnRlbnRSZWdpc3RyeTo6aW5pdGlhbGl6ZWAKcmVxdWlyZXMgYGFkbWluLnJlcXVpcmVfYXV0aCgpYDsgd2l0aCBgYWRtaW5gIHNldCB0byB0aGlzIHRyZWFzdXJ5J3MKb3duIGFkZHJlc3MgYW5kIHRoAAAACmluaXRpYWxpemUAAAAAAAQAAAAAAAAABW93bmVyAAAAAAAAEwAAAAAAAAAPaW5pdGlhbF9zaWduZXJzAAAAA+oAAAfQAAAABlNpZ25lcgAAAAAAAAAAABBpbml0aWFsX3BvbGljaWVzAAAD7AAAABMAAAAAAAAAAAAAAAZjb25maWcAAAAAB9AAAAAKSW5pdENvbmZpZwAAAAAAAQAAA+kAAAACAAAH0AAAABlTbWFydEFjY291bnRUcmVhc3VyeUVycm9yAAAA",
        "AAAABQAAAAAAAAAAAAAAGFNjaGVkdWxlZFBheW1lbnRFeGVjdXRlZAAAAAEAAAAHYXV0b19vawAAAAAFAAAAAAAAAAlpbnRlbnRfaWQAAAAAAAPuAAAAIAAAAAEAAAAAAAAADmNoaWxkX3NlcXVlbmNlAAAAAAAEAAAAAAAAAAAAAAAFYXNzZXQAAAAAAAATAAAAAAAAAAAAAAALZGVzdGluYXRpb24AAAAAEwAAAAAAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAC",
        "AAAABAAAAAAAAAAAAAAAGVNtYXJ0QWNjb3VudFRyZWFzdXJ5RXJyb3IAAAAAAAAQAAAAAAAAABJBbHJlYWR5SW5pdGlhbGl6ZWQAAAAAH0AAAAAAAAAADk5vdEluaXRpYWxpemVkAAAAAB9BAAAAAAAAAAZQYXVzZWQAAAAAH0IAAAAAAAAABkZyb3plbgAAAAAfQwAAAAAAAAAUQWRhcHRlck5vdENvbmZpZ3VyZWQAAB9EAAAAAAAAABBOb25jZUFscmVhZHlVc2VkAAAfRQAAAAAAAAANSW52YWxpZEFtb3VudAAAAAAAH0YAAAAAAAAAHVJlY2lwaWVudEFtb3VudExlbmd0aE1pc21hdGNoAAAAAAAfRwAAAAAAAAAKRW1wdHlTcGxpdAAAAAAfSAAAAAAAAAAUUmVjb3ZlcnlOb3RGaW5hbGl6ZWQAAB9JAAAAAAAAABZSZWNvdmVyeUFscmVhZHlBcHBsaWVkAAAAAB9KAAAAAAAAAAxVbmF1dGhvcml6ZWQAAB9LAAAAAAAAABpHdWFyZGlhbkZyZWV6ZU5vdFJlcXVlc3RlZAAAAAAfTAAAAAAAAAASRHVwbGljYXRlUmVjaXBpZW50AAAAAB9NAAAAAAAAABZOb1BlbmRpbmdBZGFwdGVyQ2hhbmdlAAAAAB9OAAAAAAAAABxBZGFwdGVyQ2hhbmdlRGVsYXlOb3RFbGFwc2VkAAAfTw==",
        "AAAAAAAAAAAAAAAMX19jaGVja19hdXRoAAAAAwAAAAAAAAARc2lnbmF0dXJlX3BheWxvYWQAAAAAAAPuAAAAIAAAAAAAAAAKc2lnbmF0dXJlcwAAAAAH0AAAAAtBdXRoUGF5bG9hZAAAAAAAAAAADWF1dGhfY29udGV4dHMAAAAAAAPqAAAH0AAAAAdDb250ZXh0AAAAAAEAAAPpAAAAAgAAB9AAAAATT3pTbWFydEFjY291bnRFcnJvcgA=",
        "AAAAAAAAAAAAAAANY29udHJhY3RfbmFtZQAAAAAAAAAAAAABAAAAEQ==",
        "AAAAAAAAAQJSZXRyaWV2ZXMgdGhlIGdsb2JhbCByZWdpc3RyeSBJRCBmb3IgYSBwb2xpY3kuCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCiogYHBvbGljeWAgLSBUaGUgcG9saWN5IGFkZHJlc3MgdG8gbG9vayB1cC4KCiMgRXJyb3JzCgoqIFtgU21hcnRBY2NvdW50RXJyb3I6OlBvbGljeU5vdEZvdW5kYF0gLSBXaGVuIHRoZSBwb2xpY3kgaXMgbm90CnJlZ2lzdGVyZWQgaW4gdGhlIGdsb2JhbCByZWdpc3RyeS4AAAAAAA1nZXRfcG9saWN5X2lkAAAAAAAAAQAAAAAAAAAGcG9saWN5AAAAAAATAAAAAQAAAAQ=",
        "AAAAAAAAAPpSZXRyaWV2ZXMgdGhlIGdsb2JhbCByZWdpc3RyeSBJRCBmb3IgYSBzaWduZXIuCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCiogYHNpZ25lcmAgLSBUaGUgc2lnbmVyIHRvIGxvb2sgdXAuCgojIEVycm9ycwoKKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpTaWduZXJOb3RGb3VuZGBdIC0gV2hlbiB0aGUgc2lnbmVyIGlzIG5vdApyZWdpc3RlcmVkIGluIHRoZSBnbG9iYWwgcmVnaXN0cnkuAAAAAAANZ2V0X3NpZ25lcl9pZAAAAAAAAAEAAAAAAAAABnNpZ25lcgAAAAAH0AAAAAZTaWduZXIAAAAAAAEAAAAE",
        "AAAAAAAAAAAAAAANaXNfbm9uY2VfdXNlZAAAAAAAAAEAAAAAAAAABW5vbmNlAAAAAAAABgAAAAEAAAAB",
        "AAAAAAAAA1pSZW1vdmVzIGEgcG9saWN5IGZyb20gYW4gZXhpc3RpbmcgY29udGV4dCBydWxlIGFuZCB1bmluc3RhbGxzIGl0LiBUaGUKcG9saWN5J3MgYHVuaW5zdGFsbGAgbWV0aG9kIHdpbGwgYmUgY2FsbGVkIGR1cmluZyB0aGlzIG9wZXJhdGlvbi4KUmVtb3ZpbmcgdGhlIGxhc3QgcG9saWN5IGlzIGFsbG93ZWQgb25seSBpZiB0aGUgcnVsZSBoYXMgYXQgbGVhc3QKb25lIHNpZ25lci4KCiMgQXJndW1lbnRzCgoqIGBlYCAtIEFjY2VzcyB0byB0aGUgU29yb2JhbiBlbnZpcm9ubWVudC4KKiBgY29udGV4dF9ydWxlX2lkYCAtIFRoZSBJRCBvZiB0aGUgY29udGV4dCBydWxlIHRvIG1vZGlmeS4KKiBgcG9saWN5X2lkYCAtIFRoZSBJRCBvZiB0aGUgcG9saWN5IHRvIHJlbW92ZSBmcm9tIHRoZSBjb250ZXh0IHJ1bGUuCgojIEVycm9ycwoKKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpDb250ZXh0UnVsZU5vdEZvdW5kYF0gLSBXaGVuIG5vIGNvbnRleHQgcnVsZQpleGlzdHMgd2l0aCB0aGUgZ2l2ZW4gSUQuCiogW2BTbWFydEFjY291bnRFcnJvcjo6UG9saWN5Tm90Rm91bmRgXSAtIFdoZW4gdGhlIHBvbGljeSBkb2Vzbid0IGV4aXN0CmluIHRoZSBydWxlLgoKIyBFdmVudHMKCiogdG9waWNzIC0gYFsicG9saWN5X3JlbW92ZWQiLCBjb250ZXh0X3J1bGVfaWQ6IHUzMl1gCiogZGF0YSAtIGBbcG9saWN5X2lkOiB1MzJdYAoKIyBOb3RlcwoKRGVmYXVsdHMgdG8gcmVxdWlyaW5nIGF1dGhvcml6YXRpb24gZnJvbSB0aGUgc21hcnQgYWNjb3VudCBpdHNlbGYKKGBlLmN1cnJlbnRfY29udHJhY3RfYWRkcmVzcygpLnJlcXVpcmVfYXV0aCgpYCkgYW5kIHRoZW4gZGVsZWdhdGluZyB0bwpbYHN0b3JhZ2U6OnJlbW92ZV9wb2xpY3lgXS4AAAAAAA1yZW1vdmVfcG9saWN5AAAAAAAAAgAAAAAAAAAPY29udGV4dF9ydWxlX2lkAAAAAAQAAAAAAAAACXBvbGljeV9pZAAAAAAAAAQAAAAA",
        "AAAAAAAAAwJSZW1vdmVzIGEgc2lnbmVyIGZyb20gYW4gZXhpc3RpbmcgY29udGV4dCBydWxlLiBSZW1vdmluZyB0aGUgbGFzdCBzaWduZXIKaXMgYWxsb3dlZCBvbmx5IGlmIHRoZSBydWxlIGhhcyBhdCBsZWFzdCBvbmUgcG9saWN5LgoKIyBBcmd1bWVudHMKCiogYGVgIC0gQWNjZXNzIHRvIHRoZSBTb3JvYmFuIGVudmlyb25tZW50LgoqIGBjb250ZXh0X3J1bGVfaWRgIC0gVGhlIElEIG9mIHRoZSBjb250ZXh0IHJ1bGUgdG8gbW9kaWZ5LgoqIGBzaWduZXJfaWRgIC0gVGhlIElEIG9mIHRoZSBzaWduZXIgdG8gcmVtb3ZlIGZyb20gdGhlIGNvbnRleHQgcnVsZS4KCiMgRXJyb3JzCgoqIFtgU21hcnRBY2NvdW50RXJyb3I6OkNvbnRleHRSdWxlTm90Rm91bmRgXSAtIFdoZW4gbm8gY29udGV4dCBydWxlCmV4aXN0cyB3aXRoIHRoZSBnaXZlbiBJRC4KKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpTaWduZXJOb3RGb3VuZGBdIC0gV2hlbiB0aGUgc2lnbmVyIGRvZXNuJ3QgZXhpc3QKaW4gdGhlIHJ1bGUuCgojIEV2ZW50cwoKKiB0b3BpY3MgLSBgWyJzaWduZXJfcmVtb3ZlZCIsIGNvbnRleHRfcnVsZV9pZDogdTMyXWAKKiBkYXRhIC0gYFtzaWduZXJfaWQ6IHUzMl1gCgojIE5vdGVzCgpEZWZhdWx0cyB0byByZXF1aXJpbmcgYXV0aG9yaXphdGlvbiBmcm9tIHRoZSBzbWFydCBhY2NvdW50IGl0c2VsZgooYGUuY3VycmVudF9jb250cmFjdF9hZGRyZXNzKCkucmVxdWlyZV9hdXRoKClgKSBhbmQgdGhlbiBkZWxlZ2F0aW5nIHRvCltgc3RvcmFnZTo6cmVtb3ZlX3NpZ25lcmBdLgAAAAAADXJlbW92ZV9zaWduZXIAAAAAAAACAAAAAAAAAA9jb250ZXh0X3J1bGVfaWQAAAAABAAAAAAAAAAJc2lnbmVyX2lkAAAAAAAABAAAAAA=",
        "AAAAAAAABABQdWxscyBhIGZpbmFsaXplZCByZWNvdmVyeSBvdXRjb21lIGZyb20gdGhlIGNvbmZpZ3VyZWQKYHJlY292ZXJ5X21hbmFnZXJgIGFuZCBhcHBsaWVzIGl0OiBmb3JjZS1vdmVyd3JpdGVzIG93bmVyc2hpcCwKcmVwbGFjZXMgdGhlICplbnRpcmUqIHNpZ25lci9jb250ZXh0LXJ1bGUgdG9wb2xvZ3kgd2l0aCB0aGUKZ3VhcmRpYW4tYXBwcm92ZWQgcmVwbGFjZW1lbnQsIHN5bmNzIHRoZSBndWFyZGlhbi1mcmVlemUgZXBvY2gsIGFuZApsaWZ0cyB0aGUgZnJlZXplLiBOZXZlciBwdXNoZWQgYnkgYHJlY292ZXJ5X21hbmFnZXJgIOKAlCB0aGlzCmNvbnRyYWN0IGRlY2lkZXMsIG9uIGl0cyBvd24gdGVybXMsIHdoZXRoZXIgdG8gY29uc3VtZSBhIGZpbmFsaXplZApyZXF1ZXN0LCBleGFjdGx5IG9uY2UgKGBBcHBsaWVkUmVjb3ZlcnlgIHJlcGxheSBndWFyZCkuClBlcm1pc3Npb25sZXNzOiBhbnlvbmUgbWF5IGNhbGwgdGhpcyBvbmNlIGByZWNvdmVyeV9tYW5hZ2VyYApyZXBvcnRzIHRoZSByZXF1ZXN0IGZpbmFsaXplZCwgbWlycm9yaW5nIGBmaW5hbGl6ZV9yZWNvdmVyeWAncyBvd24KcGVybWlzc2lvbmxlc3MgZGVzaWduLgoKU2VjdXJpdHkgcmV2aWV3IGZpbmRpbmcsIHR3byBwYXJ0cy4gRmlyc3Q6IHRoaXMgdXNlZCB0byBvdmVyd3JpdGUKb25seSBgT3duYWJsZWAncyBgb3duZXJgLiBTcGVuZCBhdXRob3JpdHkg4oCUIGBleGVjdXRlX3RyYW5zZmVyX3BheW1lbnRgCmFuZCBldmVyeXRoaW5nIGVsc2UgZ2F0ZWQgYnkgYGUuY3VycmVudF9jb250cmFjdF9hZGRyZXNzKCkKLnJlcXVpcmVfYXV0aCgpYCDigJQgaXMgdGhlIE9aIGNvbnRleHQtcnVsZS9zaWduZXIgcmVnaXN0cnksIGEKY29tcGxldGVseSBzZXBhcmF0ZSBzeXN0ZW0gYG93bmVyYCBuZXZlciBnYXRlcyAoc2VlCmBkb2NzL0dPVkVSTkFOQ0VfTVVMVElTSUdfREVTSUdOLm1kYCDCpzkpLiBBIGNvbXByb21pc2VkICpwYXltZW50CnNpZ25lciog4oCUIHRoZSBtb3JlIGxpa2VseSByZWFsLXdvcmxkIGNvbXByb21pc2UsIG5vdCB0aGUgb3duAAAADmFwcGx5X3JlY292ZXJ5AAAAAAABAAAAAAAAAApyZXF1ZXN0X2lkAAAAAAPuAAAAIAAAAAEAAAPpAAAAEwAAB9AAAAAZU21hcnRBY2NvdW50VHJlYXN1cnlFcnJvcgAAAA==",
        "AAAAAAAAATBBY2NlcHRzIGEgcGVuZGluZyBvd25lcnNoaXAgdHJhbnNmZXIuCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCgojIEVycm9ycwoKKiBbYGNyYXRlOjpyb2xlX3RyYW5zZmVyOjpSb2xlVHJhbnNmZXJFcnJvcjo6Tm9QZW5kaW5nVHJhbnNmZXJgXSAtIElmCnRoZXJlIGlzIG5vIHBlbmRpbmcgdHJhbnNmZXIgdG8gYWNjZXB0LgoKIyBFdmVudHMKCiogdG9waWNzIC0gYFsib3duZXJzaGlwX3RyYW5zZmVyX2NvbXBsZXRlZCJdYAoqIGRhdGEgLSBgW25ld19vd25lcjogQWRkcmVzc11gAAAAEGFjY2VwdF9vd25lcnNoaXAAAAAAAAAAAA==",
        "AAAAAAAABABDcmVhdGVzIGEgbmV3IGNvbnRleHQgcnVsZSB3aXRoIHRoZSBzcGVjaWZpZWQgY29uZmlndXJhdGlvbiwgcmV0dXJuaW5nCnRoZSBuZXdseSBjcmVhdGVkIGBDb250ZXh0UnVsZWAgd2l0aCBhIHVuaXF1ZSBJRCBhc3NpZ25lZC4gSW5zdGFsbHMKYWxsIHNwZWNpZmllZCBwb2xpY2llcyBkdXJpbmcgY3JlYXRpb24uCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCiogYGNvbnRleHRfdHlwZWAgLSBUaGUgdHlwZSBvZiBjb250ZXh0IHRoaXMgcnVsZSBhcHBsaWVzIHRvLgoqIGBuYW1lYCAtIEh1bWFuLXJlYWRhYmxlIG5hbWUgZm9yIHRoZSBjb250ZXh0IHJ1bGUuCiogYHZhbGlkX3VudGlsYCAtIE9wdGlvbmFsIGV4cGlyYXRpb24gbGVkZ2VyIHNlcXVlbmNlLgoqIGBzaWduZXJzYCAtIExpc3Qgb2Ygc2lnbmVycyBhdXRob3JpemVkIGJ5IHRoaXMgcnVsZS4KKiBgcG9saWNpZXNgIC0gTWFwIG9mIHBvbGljeSBhZGRyZXNzZXMgdG8gdGhlaXIgaW5zdGFsbGF0aW9uIHBhcmFtZXRlcnMuCgojIEVycm9ycwoKKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpOb1NpZ25lcnNBbmRQb2xpY2llc2BdIC0gV2hlbiBib3RoIHNpZ25lcnMgYW5kCnBvbGljaWVzIGFyZSBlbXB0eS4KKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpUb29NYW55U2lnbmVyc2BdIC0gV2hlbiBzaWduZXJzIGV4Y2VlZApNQVhfU0lHTkVSUyAoMTUpLgoqIFtgU21hcnRBY2NvdW50RXJyb3I6OlRvb01hbnlQb2xpY2llc2BdIC0gV2hlbiBwb2xpY2llcyBleGNlZWQKTUFYX1BPTElDSUVTICg1KS4KKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpEdXBsaWNhdGVTaWduZXJgXSAtIFdoZW4gdGhlIHNhbWUgc2lnbmVyIGFwcGVhcnMKbXVsdGlwbGUgdGltZXMuCiogW2BTbWFydEFjY291bnRFcnJvcjo6UGFzdFZhbGlkVW50aWxgXSAtIFdoZW4gdmFsaWRfdW50aWwgaXMgaW4gdGhlCnBhc3QuCiogW2BTbWFydEFjY291bnRFcnJvcjo6TWF0aE92ZXJmbG93YF0gLSBXaGVuIHRoZSBjb250ZXh0IHJ1bGUsIHNpAAAAEGFkZF9jb250ZXh0X3J1bGUAAAAFAAAAAAAAAAxjb250ZXh0X3R5cGUAAAfQAAAAD0NvbnRleHRSdWxlVHlwZQAAAAAAAAAABG5hbWUAAAAQAAAAAAAAAAt2YWxpZF91bnRpbAAAAAPoAAAABAAAAAAAAAAHc2lnbmVycwAAAAPqAAAH0AAAAAZTaWduZXIAAAAAAAAAAAAIcG9saWNpZXMAAAPsAAAAEwAAAAAAAAABAAAH0AAAAAtDb250ZXh0UnVsZQA=",
        "AAAAAAAAAWVSZXRyaWV2ZXMgYSBjb250ZXh0IHJ1bGUgYnkgaXRzIHVuaXF1ZSBJRCwgcmV0dXJuaW5nIHRoZQpgQ29udGV4dFJ1bGVgIGNvbnRhaW5pbmcgYWxsIG1ldGFkYXRhLCBzaWduZXJzLCBhbmQgcG9saWNpZXMuCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCiogYGNvbnRleHRfcnVsZV9pZGAgLSBUaGUgdW5pcXVlIGlkZW50aWZpZXIgb2YgdGhlIGNvbnRleHQgcnVsZSB0bwpyZXRyaWV2ZS4KCiMgRXJyb3JzCgoqIFtgU21hcnRBY2NvdW50RXJyb3I6OkNvbnRleHRSdWxlTm90Rm91bmRgXSAtIFdoZW4gbm8gY29udGV4dCBydWxlCmV4aXN0cyB3aXRoIHRoZSBnaXZlbiBJRC4AAAAAAAAQZ2V0X2NvbnRleHRfcnVsZQAAAAEAAAAAAAAAD2NvbnRleHRfcnVsZV9pZAAAAAAEAAAAAQAAB9AAAAALQ29udGV4dFJ1bGUA",
        "AAAAAAAAAYVSZW5vdW5jZXMgb3duZXJzaGlwIG9mIHRoZSBjb250cmFjdC4KClBlcm1hbmVudGx5IHJlbW92ZXMgdGhlIG93bmVyLCBkaXNhYmxpbmcgYWxsIGZ1bmN0aW9ucyBnYXRlZCBieQpgI1tvbmx5X293bmVyXWAuCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCgojIEVycm9ycwoKKiBbYE93bmFibGVFcnJvcjo6VHJhbnNmZXJJblByb2dyZXNzYF0gLSBJZiB0aGVyZSBpcyBhIHBlbmRpbmcgb3duZXJzaGlwCnRyYW5zZmVyLgoqIFtgT3duYWJsZUVycm9yOjpPd25lck5vdFNldGBdIC0gSWYgdGhlIG93bmVyIGlzIG5vdCBzZXQuCgojIE5vdGVzCgoqIEF1dGhvcml6YXRpb24gZm9yIHRoZSBjdXJyZW50IG93bmVyIGlzIHJlcXVpcmVkLgAAAAAAABJyZW5vdW5jZV9vd25lcnNoaXAAAAAAAAAAAAAA",
        "AAAAAAAAA45Jbml0aWF0ZXMgYSAyLXN0ZXAgb3duZXJzaGlwIHRyYW5zZmVyIHRvIGEgbmV3IGFkZHJlc3MuCgpSZXF1aXJlcyBhdXRob3JpemF0aW9uIGZyb20gdGhlIGN1cnJlbnQgb3duZXIuIFRoZSBuZXcgb3duZXIgbXVzdCBsYXRlcgpjYWxsIGBhY2NlcHRfb3duZXJzaGlwKClgIHRvIGNvbXBsZXRlIHRoZSB0cmFuc2Zlci4KCiMgQXJndW1lbnRzCgoqIGBlYCAtIEFjY2VzcyB0byB0aGUgU29yb2JhbiBlbnZpcm9ubWVudC4KKiBgbmV3X293bmVyYCAtIFRoZSBwcm9wb3NlZCBuZXcgb3duZXIuCiogYGxpdmVfdW50aWxfbGVkZ2VyYCAtIExlZGdlciBudW1iZXIgdW50aWwgd2hpY2ggdGhlIG5ldyBvd25lciBjYW4KYWNjZXB0LiBBIHZhbHVlIG9mIGAwYCBjYW5jZWxzIGFueSBwZW5kaW5nIHRyYW5zZmVyLgoKIyBFcnJvcnMKCiogW2BPd25hYmxlRXJyb3I6Ok93bmVyTm90U2V0YF0gLSBJZiB0aGUgb3duZXIgaXMgbm90IHNldC4KKiBbYGNyYXRlOjpyb2xlX3RyYW5zZmVyOjpSb2xlVHJhbnNmZXJFcnJvcjo6Tm9QZW5kaW5nVHJhbnNmZXJgXSAtIElmCnRyeWluZyB0byBjYW5jZWwgYSB0cmFuc2ZlciB0aGF0IGRvZXNuJ3QgZXhpc3QuCiogW2BjcmF0ZTo6cm9sZV90cmFuc2Zlcjo6Um9sZVRyYW5zZmVyRXJyb3I6OkludmFsaWRMaXZlVW50aWxMZWRnZXJgXSAtCklmIHRoZSBzcGVjaWZpZWQgbGVkZ2VyIGlzIGluIHRoZSBwYXN0LgoqIFtgY3JhdGU6OnJvbGVfdHJhbnNmZXI6OlJvbGVUcmFuc2ZlckVycm9yOjpJbnZhbGlkUGVuZGluZ0FjY291bnRgXSAtCklmIHRoZSBzcGVjaWZpZWQgcGVuZGluZyBhY2NvdW50IGlzIG5vdCB0aGUgc2FtZSBhcyB0aGUgcHJvdmlkZWQgYG5ld2AKYWRkcmVzcy4KCiMgTm90ZXMKCiogQXV0aG9yaXphdGlvbiBmb3IgdGhlIGN1cnJlbnQgb3duZXIgaXMgcmVxdWlyZWQuAAAAAAASdHJhbnNmZXJfb3duZXJzaGlwAAAAAAACAAAAAAAAAAluZXdfb3duZXIAAAAAAAATAAAAAAAAABFsaXZlX3VudGlsX2xlZGdlcgAAAAAAAAQAAAAA",
        "AAAAAAAAAjxQZXJtaXNzaW9ubGVzcyBUVEwgbWFpbnRlbmFuY2UgZm9yIHRoaXMgY29udHJhY3QncyBpbnN0YW5jZSBzdG9yYWdlCih3aGljaCBgUGVuZGluZ0FkYXB0ZXJgIGxpdmVzIGluLCBhbW9uZyBvdGhlciBpbnN0YW5jZS1zY29wZWQKY29uZmlnKSDigJQgZXh0ZW5kaW5nIFRUTCBjcmVhdGVzIG5vIGF1dGhvcml0eSwgbWF0Y2hpbmcKYHJlY292ZXJ5X21hbmFnZXI6OmV4dGVuZF90dGxgJ3MgcmVhc29uaW5nLiBJbmRlcGVuZGVudCBzZWN1cml0eQpyZXZpZXcgZmluZGluZzogYHByb3Bvc2VfYWRhcHRlcl9jaGFuZ2VgIG9ubHkgYnVtcHMgdGhlIGluc3RhbmNlIFRUTApvbmNlLCBhdCBwcm9wb3NhbCB0aW1lOyBhIHByb3Bvc2FsIGxlZnQgdW5hcHBsaWVkIGxvbmcgZW5vdWdoIGFmdGVyCml0cyBkZWxheSBlbGFwc2VzLCBvbiBhbiBvdGhlcndpc2UgZnVsbHkgZG9ybWFudCB0cmVhc3VyeSwgY291bGQKYXJjaGl2ZSBiZWZvcmUgYGFwcGx5X2FkYXB0ZXJfY2hhbmdlYCBpcyBldmVyIGNhbGxlZC4gVGhpcyBsZXRzCmFueW9uZSBrZWVwIGl0IGFsaXZlIHdpdGhvdXQgbmVlZGluZyB0aGUgb3duZXIgdG8gYWN0LgAAABNleHRlbmRfaW5zdGFuY2VfdHRsAAAAAAAAAAAA",
        "AAAAAAAAAqdSZW1vdmVzIGEgY29udGV4dCBydWxlIGFuZCBjbGVhbnMgdXAgYWxsIGFzc29jaWF0ZWQgZGF0YS4gVGhpcyBmdW5jdGlvbgp1bmluc3RhbGxzIGFsbCBwb2xpY2llcyBhc3NvY2lhdGVkIHdpdGggdGhlIHJ1bGUgYW5kIHJlbW92ZXMgYWxsIHN0b3JlZApkYXRhIGluY2x1ZGluZyBzaWduZXJzLCBwb2xpY2llcywgYW5kIG1ldGFkYXRhLgoKIyBBcmd1bWVudHMKCiogYGVgIC0gQWNjZXNzIHRvIHRoZSBTb3JvYmFuIGVudmlyb25tZW50LgoqIGBjb250ZXh0X3J1bGVfaWRgIC0gVGhlIElEIG9mIHRoZSBjb250ZXh0IHJ1bGUgdG8gcmVtb3ZlLgoKIyBFcnJvcnMKCiogW2BTbWFydEFjY291bnRFcnJvcjo6Q29udGV4dFJ1bGVOb3RGb3VuZGBdIC0gV2hlbiBubyBjb250ZXh0IHJ1bGUKZXhpc3RzIHdpdGggdGhlIGdpdmVuIElELgoKIyBFdmVudHMKCiogdG9waWNzIC0gYFsiY29udGV4dF9ydWxlX3JlbW92ZWQiLCBjb250ZXh0X3J1bGVfaWQ6IHUzMl1gCiogZGF0YSAtIGBbXWAKCiMgTm90ZXMKCkRlZmF1bHRzIHRvIHJlcXVpcmluZyBhdXRob3JpemF0aW9uIGZyb20gdGhlIHNtYXJ0IGFjY291bnQgaXRzZWxmCihgZS5jdXJyZW50X2NvbnRyYWN0X2FkZHJlc3MoKS5yZXF1aXJlX2F1dGgoKWApIGFuZCB0aGVuIGRlbGVnYXRpbmcgdG8KW2BzdG9yYWdlOjpyZW1vdmVfY29udGV4dF9ydWxlYF0uAAAAABNyZW1vdmVfY29udGV4dF9ydWxlAAAAAAEAAAAAAAAAD2NvbnRleHRfcnVsZV9pZAAAAAAEAAAAAA==",
        "AAAAAAAAAXdBcHBsaWVzIGEgcGVuZGluZyBhZGFwdGVyIGNoYW5nZSBvbmNlIGl0cyBkZWxheSBoYXMgZWxhcHNlZC4KUGVybWlzc2lvbmxlc3MsIGxpa2UgYGFwcGx5X3JlY292ZXJ5YC9gYXBwbHlfZ3VhcmRpYW5fZnJlZXplYDogdGhlCmF1dGhvcml6YXRpb24gZGVjaXNpb24gKHRoZSBvd25lciBwcm9wb3NpbmcgdGhpcyBzcGVjaWZpYyBjaGFuZ2UpCmFscmVhZHkgaGFwcGVuZWQgYXQgYHByb3Bvc2VfYWRhcHRlcl9jaGFuZ2VgIHRpbWUsIHNvICp3aGVuKiBhbgphbHJlYWR5LWF1dGhvcml6ZWQgY2hhbmdlIGFjdHVhbGx5IGxhbmRzIG5lZWRzIG5vIGZ1cnRoZXIgZ2F0ZSB0bwpiZSBzYWZlIOKAlCBpdCBvbmx5IG5lZWRzIHRoZSBkZWxheSB0byBoYXZlIHBhc3NlZC4AAAAAFGFwcGx5X2FkYXB0ZXJfY2hhbmdlAAAAAQAAAAAAAAAJb3BlcmF0aW9uAAAAAAAAEQAAAAEAAAPpAAAAAgAAB9AAAAAZU21hcnRBY2NvdW50VHJlYXN1cnlFcnJvcgAAAA==",
        "AAAAAAAABABSZWFsIGdhcCB0aGlzIHNlc3Npb24gZm91bmQ6IGBkb2NzL1RFQ0hOSUNBTF9BUkNISVRFQ1RVUkUubWRgIMKnMTIuNwpkb2N1bWVudHMgdGhlIGNhbm9uaWNhbCByZWNvdmVyeSB3b3JrZmxvdyBhcyBzdGFydGluZyB3aXRoICJHdWFyZGlhbgouLi4gdHJpZ2dlcnMgZnJlZXplLCIgYnV0IGBmcmVlemUoKWAgYWJvdmUgaXMgb3duZXItZ2F0ZWQgb25seSwgd2l0aApubyBndWFyZGlhbi1mYWNpbmcgcGF0aCBhdCBhbGwg4oCUIGlmIHRoZSBvd25lciBpcyB0aGUgb25lIGNvbXByb21pc2VkLApndWFyZGlhbnMgaGFkIG5vIHdheSB0byBzdG9wIG9uZ29pbmcgc3BlbmQgYXV0aG9yaXR5IHdoaWxlIGEgZnVsbApyZWNvdmVyeSAocXVvcnVtICsgdGltZWxvY2spIGlzIHN0aWxsIHBlbmRpbmcuIFRoaXMgcHVsbHMgYQpndWFyZGlhbi1yYWlzZWQgZnJlZXplIGZsYWcgZnJvbSBgcmVjb3ZlcnlfbWFuYWdlcmAgdGhlIHNhbWUgd2F5CmBhcHBseV9yZWNvdmVyeWAgcHVsbHMgYSBmaW5hbGl6ZWQgcmVxdWVzdDogcGVybWlzc2lvbmxlc3NseSwgb24KdGhpcyBjb250cmFjdCdzIG93biB0ZXJtcywgd2l0aCBgcmVjb3ZlcnlfbWFuYWdlcmAgY2Fycnlpbmcgbm8Ka25vd2xlZGdlIG9mIG9yIGRlcGVuZGVuY3kgb24gdGhpcyB0cmVhc3VyeS4KClNlY3VyaXR5IHJldmlldyBmaW5kaW5nLCB0d2ljZSBvdmVyLiBGaXJzdCByZXZpc2lvbjogdGhpcyBjYWxsZWQKYHJlY292ZXJ5X21hbmFnZXJgJ3MgcmVhZC1vbmx5IGBndWFyZGlhbl9mcmVlemVfcmVxdWVzdGVkYCwgYSBmbGFnCnNldCBvbmNlIGFuZCBuZXZlciBjbGVhcmVkIOKAlCBzaW5jZSB0aGlzIGVudHJ5cG9pbnQgaXMgZGVsaWJlcmF0ZWx5CnBlcm1pc3Npb25sZXNzIChtYXRjaGluZyBgYXBwbHlfcmVjb3ZlcnlgJ3MgInB1bGwgYW4KYWxyZWFkeS1hdXRob3JpemVkIGZhY3QiIHBhdHRlcm4pLCBhbnlvbmUgY291bGQgcmUtZnJlZXplIHRoZQp0cmVhc3VyeSBhdCBhbnkgZnV0dXJlIHBvaW50LCBldmVuIGxvbmcgYWZ0ZXIgYSBmdWxsIHJlY292ZXJ5IGhhAAAAFWFwcGx5X2d1YXJkaWFuX2ZyZWV6ZQAAAAAAAAAAAAABAAAD6QAAAAIAAAfQAAAAGVNtYXJ0QWNjb3VudFRyZWFzdXJ5RXJyb3IAAAA=",
        "AAAAAAAAAOtDYW5jZWxzIGEgcGVuZGluZyBhZGFwdGVyIGNoYW5nZSBiZWZvcmUgaXQgdGFrZXMgZWZmZWN0LiBPd25lci1nYXRlZCwKbWlycm9yaW5nIGByZWNvdmVyeV9tYW5hZ2VyOjpjYW5jZWxfcmVjb3ZlcnlgJ3MgcGF0dGVybiBvZiBsZXR0aW5nCnRoZSBsZWdpdGltYXRlIG93bmVyIHdhbGsgYmFjayBhbiBlcnJvbmVvdXMgb3Igbm8tbG9uZ2VyLXdhbnRlZApwcm9wb3NhbCBiZWZvcmUgaXRzIGRlbGF5IGVsYXBzZXMuAAAAABVjYW5jZWxfYWRhcHRlcl9jaGFuZ2UAAAAAAAABAAAAAAAAAAlvcGVyYXRpb24AAAAAAAARAAAAAQAAA+kAAAACAAAH0AAAABlTbWFydEFjY291bnRUcmVhc3VyeUVycm9yAAAA",
        "AAAAAAAAASxFeGVjdXRlcyBhIG9uZS10by1tYW55IFNBQyBzcGxpdCBmcm9tIHRoZSB0cmVhc3VyeS4gRWFjaCByZWNpcGllbnQKaXMgaW5kZXBlbmRlbnRseSBwb2xpY3ktY2hlY2tlZCAob3duIHJlY2lwaWVudC1hbGxvd2xpc3QgZW50cnksIG93bgpwZXItcmVjaXBpZW50IGFtb3VudCBjYXApIGJlZm9yZSB0aGUgYWRhcHRlciBjYWxsLCBzbyBhIHNwbGl0IGNhbm5vdApiZSB1c2VkIHRvIG1vdmUgZnVuZHMgdG8gYSBkaXNhbGxvd2VkIHJlY2lwaWVudCBieSBoaWRpbmcgaXQgaW5zaWRlCmFuIGFnZ3JlZ2F0ZS1hcHByb3ZlZCB0b3RhbC4AAAAVZXhlY3V0ZV9zcGxpdF9wYXltZW50AAAAAAAABQAAAAAAAAAFYXNzZXQAAAAAAAATAAAAAAAAAApyZWNpcGllbnRzAAAAAAPqAAAAEwAAAAAAAAAHYW1vdW50cwAAAAPqAAAACwAAAAAAAAAFbm9uY2UAAAAAAAAGAAAAAAAAABdleHBlY3RlZF9wb2xpY3lfdmVyc2lvbgAAAAAEAAAAAQAAA+kAAAACAAAH0AAAABlTbWFydEFjY291bnRUcmVhc3VyeUVycm9yAAAA",
        "AAAAAAAABABQcm9wb3NlcyByZXdpcmluZyB3aGljaCBhZGFwdGVyIGNvbnRyYWN0IGhhbmRsZXMgYSBnaXZlbiBvcGVyYXRpb24uCk93bmVyLWdhdGVkLiBVbmxpa2UgYHBvbGljeV9lbmdpbmVgIC8gYGludGVudF9yZWdpc3RyeWAgLwpgcmVjb3ZlcnlfbWFuYWdlcmAgKHBpbm5lZCBhdCBgaW5pdGlhbGl6ZWAsIHNlZSBBcmNoaXRlY3R1cmUKRGVjaXNpb24gaW4gYGRvY3MvVEVDSE5JQ0FMX0FSQ0hJVEVDVFVSRS5tZGAgwqc2LjkpLCBhZGFwdGVycyBhcmUKbXV0YWJsZSBwb3N0LWluaXQgYmVjYXVzZSB0aGUgZG9jIGV4cGxpY2l0bHkgc2NvcGVzCmBzZXRfYWRhcHRlcl9jb25maWdgIGFzIGFuIG93bmVyLWNvbmZpZ3VyYWJsZSBlbnRyeXBvaW50LgoKVGhpcyBkb2VzIG5vdCB0YWtlIGVmZmVjdCBpbW1lZGlhdGVseSDigJQgc2VlIGBhcHBseV9hZGFwdGVyX2NoYW5nZWAuCkFuIGluZGVwZW5kZW50IHNlY3VyaXR5IHJldmlldyAoYGRvY3MvU01BUlRfQ09OVFJBQ1RfQVVESVRfUkVQT1JULm1kYApmaW5kaW5nIDMpIG5vdGVkIHRoYXQgaW1tZWRpYXRlLCB1bmRlbGF5ZWQgcmVjb25maWd1cmF0aW9uIG9mCmV4ZWN1dGlvbiByb3V0aW5nIGlzIGEgbWVhbmluZ2Z1bCByaXNrIGZvciBhIHRyZWFzdXJ5IGNvbnRyYWN0CnNwZWNpZmljYWxseSAoaXQgZGVjaWRlcyAqd2hlcmUgZnVuZHMgZ28qLCB1bmxpa2UgZS5nLiBgZnJlZXplYCwKd2hpY2ggb25seSBzdG9wcyBtb3ZlbWVudCBhbmQgaXMgZGVsaWJlcmF0ZWx5IGtlcHQgaW1tZWRpYXRlKS4KT3ZlcndyaXRlcyBhbnkgZXhpc3RpbmcgcGVuZGluZyBwcm9wb3NhbCBmb3IgdGhlIHNhbWUgb3BlcmF0aW9uIOKAlAp0aGUgbW9zdCByZWNlbnQgcHJvcG9zYWwgd2lucywgbWF0Y2hpbmcgaG93IHJlLXByb3Bvc2luZyBub3JtYWxseQp3b3JrcyBlbHNld2hlcmUgaW4gdGhpcyB3b3Jrc3BhY2UgKGUuZy4gYHBvbGljeV9lbmdpbmU6OnNldF9hc3NldF9ydWxlYCkuCgpFeHBsaWNpdGx5IGV4dGVuZHMgdGhlIGluc3RhbmNlIFRUTCBvbiB3cml0ZS4gUmV2aWV3IGZpbmRpAAAAFnByb3Bvc2VfYWRhcHRlcl9jaGFuZ2UAAAAAAAIAAAAAAAAACW9wZXJhdGlvbgAAAAAAABEAAAAAAAAAB2FkYXB0ZXIAAAAAEwAAAAEAAAPpAAAAAgAAB9AAAAAZU21hcnRBY2NvdW50VHJlYXN1cnlFcnJvcgAAAA==",
        "AAAAAAAAAItSZXRyaWV2ZXMgdGhlIG51bWJlciBvZiBhbGwgY29udGV4dCBydWxlcywgaW5jbHVkaW5nIGV4cGlyZWQgcnVsZXMuCkRlZmF1bHRzIHRvIDAuCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuAAAAABdnZXRfY29udGV4dF9ydWxlc19jb3VudAAAAAAAAAAAAQAAAAQ=",
        "AAAAAAAAAedDYW5jZWxzIGEgcHJldmlvdXNseSBjcmVhdGVkIHNjaGVkdWxlZCBwYXltZW50LiBSZXF1aXJlcyB0aGlzCmNvbnRyYWN0J3Mgb3duIGF1dGhvcml6YXRpb24sIHNhbWUgYXMgY3JlYXRpb24uCgpUaGlzIGNsb3NlcyBhIHJlYWwgZ2FwOiBgaW50ZW50X3JlZ2lzdHJ5OjpjYW5jZWxfaW50ZW50YCBleGlzdHMgYW5kCmlzIGBlbnN1cmVfYWRtaW5gLWdhdGVkLCBidXQgYGludGVudF9yZWdpc3RyeWAncyBjb25maWd1cmVkIGFkbWluIGlzCnRoaXMgY29udHJhY3QncyBvd24gYWRkcmVzcyDigJQgYmVmb3JlIHRoaXMgZW50cnlwb2ludCBleGlzdGVkLCB0aGVyZQp3YXMgbm8gd2F5IGZvciBhbnlvbmUsIGluY2x1ZGluZyB0aGUgdHJlYXN1cnkncyBvd24gc2lnbmVycywgdG8gZXZlcgpjYWxsIGl0LCBzaW5jZSBvbmx5IGBzbWFydF9hY2NvdW50YCBpdHNlbGYgY2FuIHNhdGlzZnkgdGhhdCBhZG1pbgpjaGVjayB2aWEgYSBuZXN0ZWQgc3ViLWludm9jYXRpb24uAAAAABhjYW5jZWxfc2NoZWR1bGVkX3BheW1lbnQAAAABAAAAAAAAAAlpbnRlbnRfaWQAAAAAAAPuAAAAIAAAAAEAAAPpAAAAAgAAB9AAAAAZU21hcnRBY2NvdW50VHJlYXN1cnlFcnJvcgAAAA==",
        "AAAAAAAABABDcmVhdGVzIGEgc2NoZWR1bGVkL3JlY3VycmluZyBwYXltZW50IGludGVudC4gUmVxdWlyZXMgdGhpcwpjb250cmFjdCdzIG93biBhdXRob3JpemF0aW9uIChzYW1lIHNpZ25lci9jb250ZXh0LXJ1bGUvcG9saWN5IG1vZGVsCmFzIGludGVyYWN0aXZlIHBheW1lbnRzKSwgdGhlbiBkZWxlZ2F0ZXMgY2Fub25pY2FsIGxpZmVjeWNsZSBhbmQKcmVwbGF5IHN0YXRlIHRvIGBpbnRlbnRfcmVnaXN0cnlgIOKAlCBzZWUgQXJjaGl0ZWN0dXJlIERlY2lzaW9uIDIuCmBpbnRlbnRfcmVnaXN0cnlgIG11c3QgaGF2ZSBiZWVuIGNvbmZpZ3VyZWQgd2l0aCB0aGlzIGNvbnRyYWN0J3MKb3duIGFkZHJlc3MgYXMgaXRzIGFkbWluLCBzbyB0aGlzIG5lc3RlZCBjYWxsIHN1Y2NlZWRzIHZpYQpzdWItaW52b2NhdGlvbiB0cmVlIG1lbWJlcnNoaXAgcmF0aGVyIHRoYW4gYSBzZWNvbmQgc2lnbmF0dXJlLgoKVGhlIGNhbGxlci1zdXBwbGllZCBgcG9saWN5X3ZlcnNpb25gIGFuZCBgYWRhcHRlcmAgb24gYGludGVudGAgYXJlCmJvdGggaWdub3JlZCBhbmQgb3ZlcndyaXR0ZW46IGBwb2xpY3lfdmVyc2lvbmAgd2l0aCBgcG9saWN5X2VuZ2luZWAncwpjdXJyZW50IHZlcnNpb24sIGFuZCBgYWRhcHRlcmAgd2l0aCB0aGUgYHRyYW5zZmVyX2FkYXB0ZXJgIGN1cnJlbnRseQpjb25maWd1cmVkICh2aWEgYHByb3Bvc2VfYWRhcHRlcl9jaGFuZ2VgL2BhcHBseV9hZGFwdGVyX2NoYW5nZWApLgpCb3RoIG11c3QgcmVmbGVjdCB3aGF0IHdhcyBhY3R1YWxseSBpbgplZmZlY3Qgd2hlbiB0aGUgc2lnbmVyIGFwcHJvdmVkIHRoaXMgc2NoZWR1bGUsIG5vdCBhIHZhbHVlIHRoZQpjYWxsZXIgY2hvc2Ug4oCUIG90aGVyd2lzZSBhIGxhdGVyIGFkYXB0ZXIgcmVjb25maWd1cmF0aW9uIGNvdWxkCnNpbGVudGx5IHJlZGlyZWN0IGFuIGFscmVhZHktYXBwcm92ZWQgc2NoZWR1bGVkIHBheW1lbnQgdGhyb3VnaCBhCmRpZmZlcmVudCBhZGFwdGVyIGF0IGV4ZWN1dGlvbiB0aW1lIChzZWUgYGV4ZWN1dGVfc2NoZWR1bGVkX3BheW1lbnRgCmZvAAAAGGNyZWF0ZV9zY2hlZHVsZWRfcGF5bWVudAAAAAEAAAAAAAAABmludGVudAAAAAAH0AAAABNTY2hlZHVsZWRJbnRlbnRBcmdzAAAAAAEAAAPpAAAAAgAAB9AAAAAZU21hcnRBY2NvdW50VHJlYXN1cnlFcnJvcgAAAA==",
        "AAAAAAAAAOpFeGVjdXRlcyBhIHNpbmdsZS1yZWNpcGllbnQgU0FDIHBheW1lbnQuIFJlcXVpcmVzIHRoaXMgY29udHJhY3Qncwpvd24gYXV0aG9yaXphdGlvbiAoZGVsZWdhdGVkIGVudGlyZWx5IHRvIHRoZSBPWiBjb250ZXh0LXJ1bGUvc2lnbmVyLwpwb2xpY3kgbW9kZWwgdmlhIGBfX2NoZWNrX2F1dGhgKSwgdGhlbiB0cmVhc3VyeSBwb2xpY3ksIHRoZW4gYQpuYXJyb3dseSBwcmVhdXRob3JpemVkIGFkYXB0ZXIgY2FsbC4AAAAAABhleGVjdXRlX3RyYW5zZmVyX3BheW1lbnQAAAAFAAAAAAAAAAVhc3NldAAAAAAAABMAAAAAAAAAC2Rlc3RpbmF0aW9uAAAAABMAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAFbm9uY2UAAAAAAAAGAAAAAAAAABdleHBlY3RlZF9wb2xpY3lfdmVyc2lvbgAAAAAEAAAAAQAAA+kAAAACAAAH0AAAABlTbWFydEFjY291bnRUcmVhc3VyeUVycm9yAAAA",
        "AAAAAAAAAthVcGRhdGVzIHRoZSBuYW1lIG9mIGFuIGV4aXN0aW5nIGNvbnRleHQgcnVsZSwgcmV0dXJuaW5nIHRoZSB1cGRhdGVkCmBDb250ZXh0UnVsZWAgd2l0aCB0aGUgbmV3IG5hbWUuCgojIEFyZ3VtZW50cwoKKiBgZWAgLSBBY2Nlc3MgdG8gdGhlIFNvcm9iYW4gZW52aXJvbm1lbnQuCiogYGNvbnRleHRfcnVsZV9pZGAgLSBUaGUgSUQgb2YgdGhlIGNvbnRleHQgcnVsZSB0byB1cGRhdGUuCiogYG5hbWVgIC0gVGhlIG5ldyBodW1hbi1yZWFkYWJsZSBuYW1lIGZvciB0aGUgY29udGV4dCBydWxlLgoKIyBFcnJvcnMKCiogW2BTbWFydEFjY291bnRFcnJvcjo6Q29udGV4dFJ1bGVOb3RGb3VuZGBdIC0gV2hlbiBubyBjb250ZXh0IHJ1bGUKZXhpc3RzIHdpdGggdGhlIGdpdmVuIElELgoKIyBFdmVudHMKCiogdG9waWNzIC0gYFsiY29udGV4dF9ydWxlX21ldGFfdXBkYXRlZCIsIGNvbnRleHRfcnVsZV9pZDogdTMyXWAKKiBkYXRhIC0gYFtuYW1lOiBTdHJpbmcsIGNvbnRleHRfdHlwZTogQ29udGV4dFJ1bGVUeXBlLCB2YWxpZF91bnRpbDoKT3B0aW9uPHUzMj5dYAoKIyBOb3RlcwoKRGVmYXVsdHMgdG8gcmVxdWlyaW5nIGF1dGhvcml6YXRpb24gZnJvbSB0aGUgc21hcnQgYWNjb3VudCBpdHNlbGYKKGBlLmN1cnJlbnRfY29udHJhY3RfYWRkcmVzcygpLnJlcXVpcmVfYXV0aCgpYCkgYW5kIHRoZW4gZGVsZWdhdGluZyB0bwpbYHN0b3JhZ2U6OnVwZGF0ZV9jb250ZXh0X3J1bGVfbmFtZWBdLgAAABh1cGRhdGVfY29udGV4dF9ydWxlX25hbWUAAAACAAAAAAAAAA9jb250ZXh0X3J1bGVfaWQAAAAABAAAAAAAAAAEbmFtZQAAABAAAAABAAAH0AAAAAtDb250ZXh0UnVsZQA=",
        "AAAAAAAABABFeGVjdXRlcyBhbiBhbHJlYWR5LWFwcHJvdmVkIHNjaGVkdWxlZCBwYXltZW50LiBQZXJtaXNzaW9ubGVzcyBieQpkZXNpZ24gKG1hdGNoZXMgYHJlY292ZXJ5X21hbmFnZXI6OmZpbmFsaXplX3JlY292ZXJ5YCk6IHRoZSBzaWduZXIKYXBwcm92YWwgYWxyZWFkeSBoYXBwZW5lZCBhdCBgY3JlYXRlX3NjaGVkdWxlZF9wYXltZW50YCB0aW1lLiBTYWZldHkKaGVyZSBjb21lcyBlbnRpcmVseSBmcm9tIGBpbnRlbnRfcmVnaXN0cnlgJ3Mgb3duIGV4ZWN1dGlvbi13aW5kb3csCmNhbmNlbGxhdGlvbiwgYW5kIGN1bXVsYXRpdmUtdXNhZ2UgY2hlY2tzIOKAlCBub3QgZnJvbSBjYWxsZXIKaWRlbnRpdHkg4oCUIGV4Y2VwdCB0aGF0IGBpbnRlbnRfcmVnaXN0cnkubWFya19jaGlsZF9leGVjdXRlZGAgaXRzZWxmCnN0aWxsIHJlcXVpcmVzIGl0cyBjb25maWd1cmVkIGBFeGVjdXRvcmAgYWRkcmVzcyB0byBhdXRob3JpemUgdGhlCmNhbGwsIHNvIGFuIG9wZXJhdGlvbmFsIHJlbGF5ZXIga2V5IChub3QgYSB0cmVhc3VyeSBzaWduZXIpIGdhdGVzCip3aGVuKiB3aXRoaW4gdGhlIHZhbGlkIHdpbmRvdyBleGVjdXRpb24gaGFwcGVucy4KCkRlbGliZXJhdGVseSB0YWtlcyBvbmx5IGBpbnRlbnRfaWRgIGFuZCBgY2hpbGRfc2VxdWVuY2VgIGZyb20gdGhlCmNhbGxlci4gQW4gZWFybGllciByZXZpc2lvbiBhbHNvIGFjY2VwdGVkIGBhc3NldGAsIGBkZXN0aW5hdGlvbmAsCmBhbW91bnRgLCBhbmQgYGV4cGVjdGVkX3BvbGljeV92ZXJzaW9uYCBhcyBjYWxsZXItc3VwcGxpZWQKcGFyYW1ldGVycyBhbmQgdXNlZCB0aGVtIGRpcmVjdGx5IGZvciB0aGUgcG9saWN5IGNoZWNrIGFuZCBhZGFwdGVyCmRpc3BhdGNoIOKAlCBzaW5jZSB0aGlzIGVudHJ5cG9pbnQgaGFzIG5vIGByZXF1aXJlX2F1dGgoKWAgZ2F0ZSBvZiBpdHMKb3duIChieSBkZXNpZ24sIHNlZSBhYm92ZSkgYW5kIHRoZSByZWxheWVyIHJvbGUgaXMgZXhwbGljaXRseQp1bnRydXN0ZWQgKGBkb2NzL1RFQ0hOSUNBTF9BUkNISVRFQ1RVUkUubWRgIMKnMTMuAAAAGWV4ZWN1dGVfc2NoZWR1bGVkX3BheW1lbnQAAAAAAAACAAAAAAAAAAlpbnRlbnRfaWQAAAAAAAPuAAAAIAAAAAAAAAAOY2hpbGRfc2VxdWVuY2UAAAAAAAQAAAABAAAD6QAAAAIAAAfQAAAAGVNtYXJ0QWNjb3VudFRyZWFzdXJ5RXJyb3IAAAA=",
        "AAAAAAAAA1xVcGRhdGVzIHRoZSBleHBpcmF0aW9uIHRpbWUgb2YgYW4gZXhpc3RpbmcgY29udGV4dCBydWxlLCByZXR1cm5pbmcgdGhlCnVwZGF0ZWQgYENvbnRleHRSdWxlYCB3aXRoIHRoZSBuZXcgZXhwaXJhdGlvbiB0aW1lLgoKIyBBcmd1bWVudHMKCiogYGVgIC0gQWNjZXNzIHRvIHRoZSBTb3JvYmFuIGVudmlyb25tZW50LgoqIGBjb250ZXh0X3J1bGVfaWRgIC0gVGhlIElEIG9mIHRoZSBjb250ZXh0IHJ1bGUgdG8gdXBkYXRlLgoqIGB2YWxpZF91bnRpbGAgLSBOZXcgb3B0aW9uYWwgZXhwaXJhdGlvbiBsZWRnZXIgc2VxdWVuY2UuIFVzZSBgTm9uZWAKZm9yIG5vIGV4cGlyYXRpb24uCgojIEVycm9ycwoKKiBbYFNtYXJ0QWNjb3VudEVycm9yOjpDb250ZXh0UnVsZU5vdEZvdW5kYF0gLSBXaGVuIG5vIGNvbnRleHQgcnVsZQpleGlzdHMgd2l0aCB0aGUgZ2l2ZW4gSUQuCiogW2BTbWFydEFjY291bnRFcnJvcjo6UGFzdFZhbGlkVW50aWxgXSAtIFdoZW4gdmFsaWRfdW50aWwgaXMgaW4gdGhlCnBhc3QuCgojIEV2ZW50cwoKKiB0b3BpY3MgLSBgWyJjb250ZXh0X3J1bGVfbWV0YV91cGRhdGVkIiwgY29udGV4dF9ydWxlX2lkOiB1MzJdYAoqIGRhdGEgLSBgW25hbWU6IFN0cmluZywgY29udGV4dF90eXBlOiBDb250ZXh0UnVsZVR5cGUsIHZhbGlkX3VudGlsOgpPcHRpb248dTMyPl1gCgojIE5vdGVzCgpEZWZhdWx0cyB0byByZXF1aXJpbmcgYXV0aG9yaXphdGlvbiBmcm9tIHRoZSBzbWFydCBhY2NvdW50IGl0c2VsZgooYGUuY3VycmVudF9jb250cmFjdF9hZGRyZXNzKCkucmVxdWlyZV9hdXRoKClgKSBhbmQgdGhlbiBkZWxlZ2F0aW5nIHRvCltgc3RvcmFnZTo6dXBkYXRlX2NvbnRleHRfcnVsZV92YWxpZF91bnRpbGBdLgAAAB91cGRhdGVfY29udGV4dF9ydWxlX3ZhbGlkX3VudGlsAAAAAAIAAAAAAAAAD2NvbnRleHRfcnVsZV9pZAAAAAAEAAAAAAAAAAt2YWxpZF91bnRpbAAAAAPoAAAABAAAAAEAAAfQAAAAC0NvbnRleHRSdWxlAA==",
        "AAAAAgAAAONDb250ZXh0IG9mIGEgc2luZ2xlIGF1dGhvcml6ZWQgY2FsbCBwZXJmb3JtZWQgYnkgYW4gYWRkcmVzcy4KCkN1c3RvbSBhY2NvdW50IGNvbnRyYWN0cyB0aGF0IGltcGxlbWVudCBgX19jaGVja19hdXRoYCBzcGVjaWFsIGZ1bmN0aW9uCnJlY2VpdmUgYSBsaXN0IG9mIGBDb250ZXh0YCB2YWx1ZXMgY29ycmVzcG9uZGluZyB0byBhbGwgdGhlIGNhbGxzIHRoYXQKbmVlZCB0byBiZSBhdXRob3JpemVkLgAAAAAAAAAAB0NvbnRleHQAAAAAAwAAAAEAAAAUQ29udHJhY3QgaW52b2NhdGlvbi4AAAAIQ29udHJhY3QAAAABAAAH0AAAAA9Db250cmFjdENvbnRleHQAAAAAAQAAAD1Db250cmFjdCB0aGF0IGhhcyBhIGNvbnN0cnVjdG9yIHdpdGggbm8gYXJndW1lbnRzIGlzIGNyZWF0ZWQuAAAAAAAAFENyZWF0ZUNvbnRyYWN0SG9zdEZuAAAAAQAAB9AAAAAbQ3JlYXRlQ29udHJhY3RIb3N0Rm5Db250ZXh0AAAAAAEAAABEQ29udHJhY3QgdGhhdCBoYXMgYSBjb25zdHJ1Y3RvciB3aXRoIDEgb3IgbW9yZSBhcmd1bWVudHMgaXMgY3JlYXRlZC4AAAAcQ3JlYXRlQ29udHJhY3RXaXRoQ3Rvckhvc3RGbgAAAAEAAAfQAAAAKkNyZWF0ZUNvbnRyYWN0V2l0aENvbnN0cnVjdG9ySG9zdEZuQ29udGV4dAAA",
        "AAAAAQAAAL1BdXRob3JpemF0aW9uIGNvbnRleHQgb2YgYSBzaW5nbGUgY29udHJhY3QgY2FsbC4KClRoaXMgc3RydWN0IGNvcnJlc3BvbmRzIHRvIGEgYHJlcXVpcmVfYXV0aF9mb3JfYXJnc2AgY2FsbCBmb3IgYW4gYWRkcmVzcwpmcm9tIGBjb250cmFjdGAgZnVuY3Rpb24gd2l0aCBgZm5fbmFtZWAgbmFtZSBhbmQgYGFyZ3NgIGFyZ3VtZW50cy4AAAAAAAAAAAAAD0NvbnRyYWN0Q29udGV4dAAAAAADAAAAAAAAAARhcmdzAAAD6gAAAAAAAAAAAAAACGNvbnRyYWN0AAAAEwAAAAAAAAAHZm5fbmFtZQAAAAAR",
        "AAAAAgAAAF9Db250cmFjdCBleGVjdXRhYmxlIHVzZWQgZm9yIGNyZWF0aW5nIGEgbmV3IGNvbnRyYWN0IGFuZCB1c2VkIGluCmBDcmVhdGVDb250cmFjdEhvc3RGbkNvbnRleHRgLgAAAAAAAAAAEkNvbnRyYWN0RXhlY3V0YWJsZQAAAAAAAQAAAAEAAAAAAAAABFdhc20AAAABAAAD7gAAACA=",
        "AAAAAQAAAHZBdXRob3JpemF0aW9uIGNvbnRleHQgZm9yIGBjcmVhdGVfY29udHJhY3RgIGhvc3QgZnVuY3Rpb24gdGhhdCBjcmVhdGVzIGEKbmV3IGNvbnRyYWN0IG9uIGJlaGFsZiBvZiBhdXRob3JpemVyIGFkZHJlc3MuAAAAAAAAAAAAG0NyZWF0ZUNvbnRyYWN0SG9zdEZuQ29udGV4dAAAAAACAAAAAAAAAApleGVjdXRhYmxlAAAAAAfQAAAAEkNvbnRyYWN0RXhlY3V0YWJsZQAAAAAAAAAAAARzYWx0AAAD7gAAACA=",
        "AAAAAQAAANZBdXRob3JpemF0aW9uIGNvbnRleHQgZm9yIGBjcmVhdGVfY29udHJhY3RgIGhvc3QgZnVuY3Rpb24gdGhhdCBjcmVhdGVzIGEKbmV3IGNvbnRyYWN0IG9uIGJlaGFsZiBvZiBhdXRob3JpemVyIGFkZHJlc3MuClRoaXMgaXMgdGhlIHNhbWUgYXMgYENyZWF0ZUNvbnRyYWN0SG9zdEZuQ29udGV4dGAsIGJ1dCBhbHNvIGhhcwpjb250cmFjdCBjb25zdHJ1Y3RvciBhcmd1bWVudHMuAAAAAAAAAAAAKkNyZWF0ZUNvbnRyYWN0V2l0aENvbnN0cnVjdG9ySG9zdEZuQ29udGV4dAAAAAAAAwAAAAAAAAAQY29uc3RydWN0b3JfYXJncwAAA+oAAAAAAAAAAAAAAApleGVjdXRhYmxlAAAAAAfQAAAAEkNvbnRyYWN0RXhlY3V0YWJsZQAAAAAAAAAAAARzYWx0AAAD7gAAACA=",
        "AAAABAAAAAAAAAAAAAAAEVJvbGVUcmFuc2ZlckVycm9yAAAAAAAABAAAAAAAAAARTm9QZW5kaW5nVHJhbnNmZXIAAAAAAAiYAAAAAAAAABZJbnZhbGlkTGl2ZVVudGlsTGVkZ2VyAAAAAAiZAAAAAAAAABVJbnZhbGlkUGVuZGluZ0FjY291bnQAAAAAAAiaAAAAAAAAAA9UcmFuc2ZlckV4cGlyZWQAAAAImw==",
        "AAAABAAAAAAAAAAAAAAADE93bmFibGVFcnJvcgAAAAMAAAAAAAAAC093bmVyTm90U2V0AAAACDQAAAAAAAAAElRyYW5zZmVySW5Qcm9ncmVzcwAAAAAINQAAAAAAAAAPT3duZXJBbHJlYWR5U2V0AAAACDY=",
        "AAAABQAAADZFdmVudCBlbWl0dGVkIHdoZW4gYW4gb3duZXJzaGlwIHRyYW5zZmVyIGlzIGluaXRpYXRlZC4AAAAAAAAAAAART3duZXJzaGlwVHJhbnNmZXIAAAAAAAABAAAAEm93bmVyc2hpcF90cmFuc2ZlcgAAAAAAAwAAAAAAAAAJb2xkX293bmVyAAAAAAAAEwAAAAAAAAAAAAAACW5ld19vd25lcgAAAAAAABMAAAAAAAAAAAAAABFsaXZlX3VudGlsX2xlZGdlcgAAAAAAAAQAAAAAAAAAAg==",
        "AAAABQAAACpFdmVudCBlbWl0dGVkIHdoZW4gb3duZXJzaGlwIGlzIHJlbm91bmNlZC4AAAAAAAAAAAAST3duZXJzaGlwUmVub3VuY2VkAAAAAAABAAAAE293bmVyc2hpcF9yZW5vdW5jZWQAAAAAAQAAAAAAAAAJb2xkX293bmVyAAAAAAAAEwAAAAAAAAAC",
        "AAAABQAAADZFdmVudCBlbWl0dGVkIHdoZW4gYW4gb3duZXJzaGlwIHRyYW5zZmVyIGlzIGNvbXBsZXRlZC4AAAAAAAAAAAAaT3duZXJzaGlwVHJhbnNmZXJDb21wbGV0ZWQAAAAAAAEAAAAcb3duZXJzaGlwX3RyYW5zZmVyX2NvbXBsZXRlZAAAAAEAAAAAAAAACW5ld19vd25lcgAAAAAAABMAAAAAAAAAAg==",
        "AAAABQAAADdFdmVudCBlbWl0dGVkIHdoZW4gYSBwb2xpY3kgaXMgYWRkZWQgdG8gYSBjb250ZXh0IHJ1bGUuAAAAAAAAAAALUG9saWN5QWRkZWQAAAAAAQAAAAxwb2xpY3lfYWRkZWQAAAACAAAAAAAAAA9jb250ZXh0X3J1bGVfaWQAAAAABAAAAAEAAAAAAAAACXBvbGljeV9pZAAAAAAAAAQAAAAAAAAAAg==",
        "AAAABQAAADdFdmVudCBlbWl0dGVkIHdoZW4gYSBzaWduZXIgaXMgYWRkZWQgdG8gYSBjb250ZXh0IHJ1bGUuAAAAAAAAAAALU2lnbmVyQWRkZWQAAAAAAQAAAAxzaWduZXJfYWRkZWQAAAACAAAAAAAAAA9jb250ZXh0X3J1bGVfaWQAAAAABAAAAAEAAAAAAAAACXNpZ25lcl9pZAAAAAAAAAQAAAAAAAAAAg==",
        "AAAABQAAADtFdmVudCBlbWl0dGVkIHdoZW4gYSBwb2xpY3kgaXMgcmVtb3ZlZCBmcm9tIGEgY29udGV4dCBydWxlLgAAAAAAAAAADVBvbGljeVJlbW92ZWQAAAAAAAABAAAADnBvbGljeV9yZW1vdmVkAAAAAAACAAAAAAAAAA9jb250ZXh0X3J1bGVfaWQAAAAABAAAAAEAAAAAAAAACXBvbGljeV9pZAAAAAAAAAQAAAAAAAAAAg==",
        "AAAABQAAADtFdmVudCBlbWl0dGVkIHdoZW4gYSBzaWduZXIgaXMgcmVtb3ZlZCBmcm9tIGEgY29udGV4dCBydWxlLgAAAAAAAAAADVNpZ25lclJlbW92ZWQAAAAAAAABAAAADnNpZ25lcl9yZW1vdmVkAAAAAAACAAAAAAAAAA9jb250ZXh0X3J1bGVfaWQAAAAABAAAAAEAAAAAAAAACXNpZ25lcl9pZAAAAAAAAAQAAAAAAAAAAg==",
        "AAAABQAAACtFdmVudCBlbWl0dGVkIHdoZW4gYSBjb250ZXh0IHJ1bGUgaXMgYWRkZWQuAAAAAAAAAAAQQ29udGV4dFJ1bGVBZGRlZAAAAAEAAAASY29udGV4dF9ydWxlX2FkZGVkAAAAAAAGAAAAAAAAAA9jb250ZXh0X3J1bGVfaWQAAAAABAAAAAEAAAAAAAAABG5hbWUAAAAQAAAAAAAAAAAAAAAMY29udGV4dF90eXBlAAAH0AAAAA9Db250ZXh0UnVsZVR5cGUAAAAAAAAAAAAAAAALdmFsaWRfdW50aWwAAAAD6AAAAAQAAAAAAAAAAAAAAApzaWduZXJfaWRzAAAAAAPqAAAABAAAAAAAAAAAAAAACnBvbGljeV9pZHMAAAAAA+oAAAAEAAAAAAAAAAI=",
        "AAAABQAAAEFFdmVudCBlbWl0dGVkIHdoZW4gYSBwb2xpY3kgaXMgcmVnaXN0ZXJlZCBpbiB0aGUgZ2xvYmFsIHJlZ2lzdHJ5LgAAAAAAAAAAAAAQUG9saWN5UmVnaXN0ZXJlZAAAAAEAAAARcG9saWN5X3JlZ2lzdGVyZWQAAAAAAAACAAAAAAAAAAlwb2xpY3lfaWQAAAAAAAAEAAAAAQAAAAAAAAAGcG9saWN5AAAAAAATAAAAAAAAAAI=",
        "AAAABQAAAEFFdmVudCBlbWl0dGVkIHdoZW4gYSBzaWduZXIgaXMgcmVnaXN0ZXJlZCBpbiB0aGUgZ2xvYmFsIHJlZ2lzdHJ5LgAAAAAAAAAAAAAQU2lnbmVyUmVnaXN0ZXJlZAAAAAEAAAARc2lnbmVyX3JlZ2lzdGVyZWQAAAAAAAACAAAAAAAAAAlzaWduZXJfaWQAAAAAAAAEAAAAAQAAAAAAAAAGc2lnbmVyAAAAAAfQAAAABlNpZ25lcgAAAAAAAAAAAAI=",
        "AAAABAAAAClFcnJvciBjb2RlcyBmb3Igc21hcnQgYWNjb3VudCBvcGVyYXRpb25zLgAAAAAAAAAAAAARU21hcnRBY2NvdW50RXJyb3IAAAAAAAAQAAAAKlRoZSBzcGVjaWZpZWQgY29udGV4dCBydWxlIGRvZXMgbm90IGV4aXN0LgAAAAAAE0NvbnRleHRSdWxlTm90Rm91bmQAAAALuAAAADpUaGUgcHJvdmlkZWQgY29udGV4dCBjYW5ub3QgYmUgdmFsaWRhdGVkIGFnYWluc3QgYW55IHJ1bGUuAAAAAAASVW52YWxpZGF0ZWRDb250ZXh0AAAAAAu6AAAAJ0V4dGVybmFsIHNpZ25hdHVyZSB2ZXJpZmljYXRpb24gZmFpbGVkLgAAAAAaRXh0ZXJuYWxWZXJpZmljYXRpb25GYWlsZWQAAAAAC7sAAAA1Q29udGV4dCBydWxlIG11c3QgaGF2ZSBhdCBsZWFzdCBvbmUgc2lnbmVyIG9yIHBvbGljeS4AAAAAAAAUTm9TaWduZXJzQW5kUG9saWNpZXMAAAu8AAAAKVRoZSB2YWxpZF91bnRpbCB0aW1lc3RhbXAgaXMgaW4gdGhlIHBhc3QuAAAAAAAADlBhc3RWYWxpZFVudGlsAAAAAAu9AAAAI1RoZSBzcGVjaWZpZWQgc2lnbmVyIHdhcyBub3QgZm91bmQuAAAAAA5TaWduZXJOb3RGb3VuZAAAAAALvgAAAC5UaGUgc2lnbmVyIGFscmVhZHkgZXhpc3RzIGluIHRoZSBjb250ZXh0IHJ1bGUuAAAAAAAPRHVwbGljYXRlU2lnbmVyAAAAC78AAAAjVGhlIHNwZWNpZmllZCBwb2xpY3kgd2FzIG5vdCBmb3VuZC4AAAAADlBvbGljeU5vdEZvdW5kAAAAAAvAAAAALlRoZSBwb2xpY3kgYWxyZWFkeSBleGlzdHMgaW4gdGhlIGNvbnRleHQgcnVsZS4AAAAAAA9EdXBsaWNhdGVQb2xpY3kAAAALwQAAACVUb28gbWFueSBzaWduZXJzIGluIHRoZSBjb250ZXh0IHJ1bGUuAAAAAAAADlRvb01hbnlTaWduZXJzAAAAAAvCAAAAJlRvbyBtYW55IHBvbGljaWVzIGluIHRoZSBjb250ZXh0IHJ1bGUuAAAAAAAPVG9vTWFueVBvbGljaWVzAAAAC8MAAACGQW4gaW50ZXJuYWwgSUQgY291bnRlciAoY29udGV4dCBydWxlLCBzaWduZXIsIG9yIHBvbGljeSkgaGFzIHJlYWNoZWQKaXRzIG1heGltdW0gdmFsdWUgKGB1MzI6Ok1BWGApIGFuZCBjYW5ub3QgYmUgaW5jcmVtZW50ZWQgZnVydGhlci4AAAAAAAxNYXRoT3ZlcmZsb3cAAAvEAAAAOkV4dGVybmFsIHNpZ25lciBrZXkgZGF0YSBleGNlZWRzIHRoZSBtYXhpbXVtIGFsbG93ZWQgc2l6ZS4AAAAAAA9LZXlEYXRhVG9vTGFyZ2UAAAALxQAAADxjb250ZXh0X3J1bGVfaWRzIGxlbmd0aCBkb2VzIG5vdCBtYXRjaCBhdXRoX2NvbnRleHRzIGxlbmd0aC4AAAAcQ29udGV4dFJ1bGVJZHNMZW5ndGhNaXNtYXRjaAAAC8YAAAA1Q29udGV4dCBydWxlIG5hbWUgZXhjZWVkcyB0aGUgbWF4aW11bSBhbGxvd2VkIGxlbmd0aC4AAAAAAAALTmFtZVRvb0xvbmcAAAALxwAAAENBIHNpZ25lciBpbiBgQXV0aFBheWxvYWRgIGlzIG5vdCBwYXJ0IG9mIGFueSBzZWxlY3RlZCBjb250ZXh0IHJ1bGUuAAAAABJVbmF1dGhvcml6ZWRTaWduZXIAAAAAC8g=",
        "AAAABQAAAC1FdmVudCBlbWl0dGVkIHdoZW4gYSBjb250ZXh0IHJ1bGUgaXMgcmVtb3ZlZC4AAAAAAAAAAAAAEkNvbnRleHRSdWxlUmVtb3ZlZAAAAAAAAQAAABRjb250ZXh0X3J1bGVfcmVtb3ZlZAAAAAEAAAAAAAAAD2NvbnRleHRfcnVsZV9pZAAAAAAEAAAAAQAAAAI=",
        "AAAABQAAAEVFdmVudCBlbWl0dGVkIHdoZW4gYSBwb2xpY3kgaXMgZGVyZWdpc3RlcmVkIGZyb20gdGhlIGdsb2JhbCByZWdpc3RyeS4AAAAAAAAAAAAAElBvbGljeURlcmVnaXN0ZXJlZAAAAAAAAQAAABNwb2xpY3lfZGVyZWdpc3RlcmVkAAAAAAEAAAAAAAAACXBvbGljeV9pZAAAAAAAAAQAAAABAAAAAg==",
        "AAAABQAAAEVFdmVudCBlbWl0dGVkIHdoZW4gYSBzaWduZXIgaXMgZGVyZWdpc3RlcmVkIGZyb20gdGhlIGdsb2JhbCByZWdpc3RyeS4AAAAAAAAAAAAAElNpZ25lckRlcmVnaXN0ZXJlZAAAAAAAAQAAABNzaWduZXJfZGVyZWdpc3RlcmVkAAAAAAEAAAAAAAAACXNpZ25lcl9pZAAAAAAAAAQAAAABAAAAAg==",
        "AAAABQAAAEJFdmVudCBlbWl0dGVkIHdoZW4gYSBjb250ZXh0IHJ1bGUgbmFtZSBvciB2YWxpZF91bnRpbCBhcmUgdXBkYXRlZC4AAAAAAAAAAAAWQ29udGV4dFJ1bGVNZXRhVXBkYXRlZAAAAAAAAQAAABljb250ZXh0X3J1bGVfbWV0YV91cGRhdGVkAAAAAAAAAwAAAAAAAAAPY29udGV4dF9ydWxlX2lkAAAAAAQAAAABAAAAAAAAAARuYW1lAAAAEAAAAAAAAAAAAAAAC3ZhbGlkX3VudGlsAAAAA+gAAAAEAAAAAAAAAAI=",
        "AAAAAgAAAEJSZXByZXNlbnRzIGRpZmZlcmVudCB0eXBlcyBvZiBzaWduZXJzIGluIHRoZSBzbWFydCBhY2NvdW50IHN5c3RlbS4AAAAAAAAAAAAGU2lnbmVyAAAAAAACAAAAAQAAAD1BIGRlbGVnYXRlZCBzaWduZXIgdGhhdCB1c2VzIGJ1aWx0LWluIHNpZ25hdHVyZSB2ZXJpZmljYXRpb24uAAAAAAAACURlbGVnYXRlZAAAAAAAAAEAAAATAAAAAQAAAHJBbiBleHRlcm5hbCBzaWduZXIgd2l0aCBjdXN0b20gdmVyaWZpY2F0aW9uIGxvZ2ljLgpDb250YWlucyB0aGUgdmVyaWZpZXIgY29udHJhY3QgYWRkcmVzcyBhbmQgdGhlIHB1YmxpYyBrZXkgZGF0YS4AAAAAAAhFeHRlcm5hbAAAAAIAAAATAAAADg==",
        "AAAAAQAABABUaGUgYXV0aG9yaXphdGlvbiBwYXlsb2FkIHBhc3NlZCB0byBgX19jaGVja19hdXRoYCwgYnVuZGxpbmcgY3J5cHRvZ3JhcGhpYwpwcm9vZnMgd2l0aCBjb250ZXh0IHJ1bGUgc2VsZWN0aW9uLgoKVGhpcyBzdHJ1Y3QgY2FycmllcyB0d28gZGlzdGluY3QgcGllY2VzIG9mIGluZm9ybWF0aW9uIHRoYXQgYXJlIGJvdGgKcmVxdWlyZWQgZm9yIGF1dGhvcml6YXRpb24gYnV0IGNhbm5vdCBiZSBkZXJpdmVkIGZyb20gZWFjaCBvdGhlcjoKCi0gYHNpZ25lcnNgIG1hcHMgZWFjaCBbYFNpZ25lcmBdIHRvIGl0cyByYXcgc2lnbmF0dXJlIGJ5dGVzLCBwcm92aWRpbmcKY3J5cHRvZ3JhcGhpYyBwcm9vZiB0aGF0IHRoZSBzaWduZXIgYWN0dWFsbHkgc2lnbmVkIHRoZSB0cmFuc2FjdGlvbgpwYXlsb2FkLiBBIGNvbnRleHQgcnVsZSBzdG9yZXMgd2hpY2ggc2lnbmVyICppZGVudGl0aWVzKiBhcmUgYXV0aG9yaXplZAoodmlhIGBzaWduZXJfaWRzYCksIGJ1dCB0aGUgcnVsZSBkb2VzIG5vdCBjb250YWluIHRoZSBzaWduYXR1cmVzCnRoZW1zZWx2ZXMg4oCUIHRob3NlIG11c3QgYmUgc3VwcGxpZWQgaGVyZS4KCi0gYGNvbnRleHRfcnVsZV9pZHNgIHRlbGxzIHRoZSBzeXN0ZW0gd2hpY2ggcnVsZSB0byB2YWxpZGF0ZSBmb3IgZWFjaCBhdXRoCmNvbnRleHQuIEJlY2F1c2UgbXVsdGlwbGUgcnVsZXMgY2FuIGV4aXN0IGZvciB0aGUgc2FtZSBjb250ZXh0IHR5cGUsIHRoZQpjYWxsZXIgbXVzdCBleHBsaWNpdGx5IHNlbGVjdCBvbmUgcGVyIGNvbnRleHQgcmF0aGVyIHRoYW4gcmVseWluZyBvbgphdXRvLWRpc2NvdmVyeS4gRWFjaCBlbnRyeSBpcyBhbGlnbmVkIGJ5IGluZGV4IHdpdGggdGhlIGBhdXRoX2NvbnRleHRzYApwYXNzZWQgdG8gYF9fY2hlY2tfYXV0aGAuCgpUaGUgbGVuZ3RoIG9mIGBjb250ZXh0X3J1bGVfaWRzYCBtdXN0IGVxdWFsIHRoZSBudW1iZXIgb2YgYXV0aCBjb250ZXh0czsKYSBtaXNtYXRjaCBpcyByZWplY3RlZCB3aXRoCltgU21hcnRBY2NvdW50RXJyb3I6OkNvbnRleHRSdWxlSWRzTGVuAAAAAAAAAAtBdXRoUGF5bG9hZAAAAAACAAAAPFBlci1jb250ZXh0IHJ1bGUgSURzLCBhbGlnbmVkIGJ5IGluZGV4IHdpdGggYGF1dGhfY29udGV4dHNgLgAAABBjb250ZXh0X3J1bGVfaWRzAAAD6gAAAAQAAAAlU2lnbmF0dXJlIGRhdGEgbWFwcGVkIHRvIGVhY2ggc2lnbmVyLgAAAAAAAAdzaWduZXJzAAAAA+wAAAfQAAAABlNpZ25lcgAAAAAADg==",
        "AAAAAQAAADxBIGNvbXBsZXRlIGNvbnRleHQgcnVsZSBkZWZpbmluZyBhdXRob3JpemF0aW9uIHJlcXVpcmVtZW50cy4AAAAAAAAAC0NvbnRleHRSdWxlAAAAAAgAAAApVGhlIHR5cGUgb2YgY29udGV4dCB0aGlzIHJ1bGUgYXBwbGllcyB0by4AAAAAAAAMY29udGV4dF90eXBlAAAH0AAAAA9Db250ZXh0UnVsZVR5cGUAAAAAJ1VuaXF1ZSBpZGVudGlmaWVyIGZvciB0aGUgY29udGV4dCBydWxlLgAAAAACaWQAAAAAAAQAAAApSHVtYW4tcmVhZGFibGUgbmFtZSBmb3IgdGhlIGNvbnRleHQgcnVsZS4AAAAAAAAEbmFtZQAAABAAAAAwTGlzdCBvZiBwb2xpY3kgY29udHJhY3RzIHRoYXQgbXVzdCBiZSBzYXRpc2ZpZWQuAAAACHBvbGljaWVzAAAD6gAAABMAAABKR2xvYmFsIHJlZ2lzdHJ5IElEcyBmb3IgZWFjaCBwb2xpY3ksIHBvc2l0aW9uYWxseSBhbGlnbmVkIHdpdGgKYHBvbGljaWVzYC4AAAAAAApwb2xpY3lfaWRzAAAAAAPqAAAABAAAAElHbG9iYWwgcmVnaXN0cnkgSURzIGZvciBlYWNoIHNpZ25lciwgcG9zaXRpb25hbGx5IGFsaWduZWQgd2l0aApgc2lnbmVyc2AuAAAAAAAACnNpZ25lcl9pZHMAAAAAA+oAAAAEAAAAKExpc3Qgb2Ygc2lnbmVycyBhdXRob3JpemVkIGJ5IHRoaXMgcnVsZS4AAAAHc2lnbmVycwAAAAPqAAAH0AAAAAZTaWduZXIAAAAAADFPcHRpb25hbCBleHBpcmF0aW9uIGxlZGdlciBzZXF1ZW5jZSBmb3IgdGhlIHJ1bGUuAAAAAAAAC3ZhbGlkX3VudGlsAAAAA+gAAAAE",
        "AAAAAgAAAEBUeXBlcyBvZiBjb250ZXh0cyB0aGF0IGNhbiBiZSBhdXRob3JpemVkIGJ5IHNtYXJ0IGFjY291bnQgcnVsZXMuAAAAAAAAAA9Db250ZXh0UnVsZVR5cGUAAAAAAwAAAAAAAAAtRGVmYXVsdCBydWxlcyB0aGF0IGNhbiBhdXRob3JpemUgYW55IGNvbnRleHQuAAAAAAAAB0RlZmF1bHQAAAAAAQAAADBSdWxlcyBzcGVjaWZpYyB0byBjYWxsaW5nIGEgcGFydGljdWxhciBjb250cmFjdC4AAAAMQ2FsbENvbnRyYWN0AAAAAQAAABMAAAABAAAAQlJ1bGVzIHNwZWNpZmljIHRvIGNyZWF0aW5nIGEgY29udHJhY3Qgd2l0aCBhIHBhcnRpY3VsYXIgV0FTTSBoYXNoLgAAAAAADkNyZWF0ZUNvbnRyYWN0AAAAAAABAAAD7gAAACA=",
        "AAAABQAAACpFdmVudCBlbWl0dGVkIHdoZW4gdGhlIGNvbnRyYWN0IGlzIHBhdXNlZC4AAAAAAAAAAAAGUGF1c2VkAAAAAAABAAAABnBhdXNlZAAAAAAAAAAAAAI=",
        "AAAABQAAACxFdmVudCBlbWl0dGVkIHdoZW4gdGhlIGNvbnRyYWN0IGlzIHVucGF1c2VkLgAAAAAAAAAIVW5wYXVzZWQAAAABAAAACHVucGF1c2VkAAAAAAAAAAI=",
        "AAAABAAAAAAAAAAAAAAADVBhdXNhYmxlRXJyb3IAAAAAAAACAAAANFRoZSBvcGVyYXRpb24gZmFpbGVkIGJlY2F1c2UgdGhlIGNvbnRyYWN0IGlzIHBhdXNlZC4AAAANRW5mb3JjZWRQYXVzZQAAAAAAA+gAAAA4VGhlIG9wZXJhdGlvbiBmYWlsZWQgYmVjYXVzZSB0aGUgY29udHJhY3QgaXMgbm90IHBhdXNlZC4AAAANRXhwZWN0ZWRQYXVzZQAAAAAAA+k=" ]),
      options
    )
  }
  public readonly fromJSON = {
    pause: this.txFromJSON<null>,
        freeze: this.txFromJSON<Result<void>>,
        paused: this.txFromJSON<boolean>,
        status: this.txFromJSON<Result<AccountStatus>>,
        unpause: this.txFromJSON<null>,
        get_owner: this.txFromJSON<Option<string>>,
        add_policy: this.txFromJSON<u32>,
        add_signer: this.txFromJSON<u32>,
        initialize: this.txFromJSON<Result<void>>,
        contract_name: this.txFromJSON<string>,
        get_policy_id: this.txFromJSON<u32>,
        get_signer_id: this.txFromJSON<u32>,
        is_nonce_used: this.txFromJSON<boolean>,
        remove_policy: this.txFromJSON<null>,
        remove_signer: this.txFromJSON<null>,
        apply_recovery: this.txFromJSON<Result<string>>,
        accept_ownership: this.txFromJSON<null>,
        add_context_rule: this.txFromJSON<ContextRule>,
        get_context_rule: this.txFromJSON<ContextRule>,
        renounce_ownership: this.txFromJSON<null>,
        transfer_ownership: this.txFromJSON<null>,
        extend_instance_ttl: this.txFromJSON<null>,
        remove_context_rule: this.txFromJSON<null>,
        apply_adapter_change: this.txFromJSON<Result<void>>,
        apply_guardian_freeze: this.txFromJSON<Result<void>>,
        cancel_adapter_change: this.txFromJSON<Result<void>>,
        execute_split_payment: this.txFromJSON<Result<void>>,
        propose_adapter_change: this.txFromJSON<Result<void>>,
        get_context_rules_count: this.txFromJSON<u32>,
        cancel_scheduled_payment: this.txFromJSON<Result<void>>,
        create_scheduled_payment: this.txFromJSON<Result<void>>,
        execute_transfer_payment: this.txFromJSON<Result<void>>,
        update_context_rule_name: this.txFromJSON<ContextRule>,
        execute_scheduled_payment: this.txFromJSON<Result<void>>,
        update_context_rule_valid_until: this.txFromJSON<ContextRule>
  }
}