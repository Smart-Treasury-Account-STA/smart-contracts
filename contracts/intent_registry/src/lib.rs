#![no_std]

//! IntentRegistry: canonical, replay-safe lifecycle state for scheduled and
//! recurring treasury execution. This is the module SmartAccount defers to
//! for "has this scheduled action already run, and is it still allowed to
//! run" — see Architecture Decision 2 in `docs/TECHNICAL_ARCHITECTURE.md`
//! for why this state does not live in SmartAccount itself.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, symbol_short, Address,
    BytesN, Env, Symbol, Vec,
};

/// See `docs/TECHNICAL_ARCHITECTURE.md` §16 ("Production requirement":
/// extend TTL on successful state-changing calls touching critical
/// entries). Scheduled intents can legitimately sit untouched between
/// creation and their execution window opening, so both write and read
/// paths extend TTL here.
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

#[contract]
pub struct IntentRegistry;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledIntent {
    pub intent_id: BytesN<32>,
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
    pub start_ledger: u32,
    pub end_ledger: u32,
    /// Total number of child executions this intent may ever authorize.
    /// Bounds recurring automation instead of allowing unlimited replay
    /// within an otherwise-valid execution window.
    pub max_executions: u32,
    /// Cumulative count of successfully marked child executions.
    pub execution_count: u32,
    /// PolicyEngine version in effect when this intent was approved, pinned
    /// at creation so a later policy version bump cannot silently change
    /// which rules a previously-approved scheduled payment executes under
    /// (see `docs/TECHNICAL_ARCHITECTURE.md` §13.2: "Policy version changes
    /// cannot silently mutate previously created automation semantics").
    pub policy_version: u32,
    /// The `transfer_adapter` address configured on `smart_account` at the
    /// moment this intent was approved, pinned here for the same reason
    /// `policy_version` is: reconfiguring the adapter after approval (via
    /// `smart_account::propose_adapter_change`/`apply_adapter_change`) must not silently redirect an
    /// already-approved scheduled payment through a different execution
    /// path. Execution reads this field back rather than resolving the
    /// treasury's *current* adapter configuration.
    pub adapter: Address,
    pub cancelled: bool,
}

#[contractevent(topics = ["init"])]
pub struct Initialized {
    #[topic]
    pub admin: Address,
}

#[contractevent(topics = ["execset"])]
pub struct ExecutorUpdated {
    #[topic]
    pub executor: Address,
}

#[contractevent(topics = ["intent"])]
pub struct IntentCreated {
    #[topic]
    pub intent_id: BytesN<32>,
}

#[contractevent(topics = ["cancel"])]
pub struct IntentCancelled {
    #[topic]
    pub intent_id: BytesN<32>,
}

#[contractevent(topics = ["exec"])]
pub struct ChildExecuted {
    #[topic]
    pub intent_id: BytesN<32>,
    pub child_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
    Admin,
    Executor,
    Intent(BytesN<32>),
    ChildExecution(BytesN<32>, u32),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IntentRegistryError {
    AlreadyInitialized = 3000,
    NotInitialized = 3001,
    IntentAlreadyExists = 3002,
    IntentNotFound = 3003,
    InvalidAmount = 3004,
    InvalidWindow = 3005,
    IntentCancelled = 3006,
    ExecutionTooEarly = 3007,
    ExecutionExpired = 3008,
    ChildAlreadyExecuted = 3009,
    UnauthorizedExecutor = 3010,
    InvalidMaxExecutions = 3011,
    ExecutionLimitReached = 3012,
}

#[contractimpl]
impl IntentRegistry {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_int")
    }

    pub fn initialize(env: Env, admin: Address) -> Result<(), IntentRegistryError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(IntentRegistryError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::Executor, &admin);
        bump_ttl(&env, &DataKey::Initialized);
        bump_ttl(&env, &DataKey::Admin);
        bump_ttl(&env, &DataKey::Executor);
        Initialized {
            admin: admin.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_executor(env: Env, executor: Address) -> Result<(), IntentRegistryError> {
        ensure_admin(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::Executor, &executor);
        bump_ttl(&env, &DataKey::Executor);
        ExecutorUpdated {
            executor: executor.clone(),
        }
        .publish(&env);
        Ok(())
    }

    /// Permissionless TTL maintenance for named, already-existing intents
    /// and their child execution records — extending TTL creates no
    /// authority (§16.1), so no admin gate is needed.
    pub fn extend_intent_ttl(env: Env, intent_id: BytesN<32>, child_sequences: Vec<u32>) {
        bump_ttl(&env, &DataKey::Intent(intent_id.clone()));
        for child_sequence in child_sequences.iter() {
            bump_ttl(
                &env,
                &DataKey::ChildExecution(intent_id.clone(), child_sequence),
            );
        }
    }

    pub fn create_intent(env: Env, intent: ScheduledIntent) -> Result<(), IntentRegistryError> {
        ensure_admin(&env)?;
        if intent.amount <= 0 {
            return Err(IntentRegistryError::InvalidAmount);
        }
        if intent.end_ledger < intent.start_ledger {
            return Err(IntentRegistryError::InvalidWindow);
        }
        if intent.max_executions == 0 {
            return Err(IntentRegistryError::InvalidMaxExecutions);
        }

        let key = DataKey::Intent(intent.intent_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(IntentRegistryError::IntentAlreadyExists);
        }

        // Caller-supplied `cancelled` and `execution_count` are ignored: a
        // freshly created intent always starts active with zero usage,
        // regardless of what the caller passed in.
        let stored_intent = ScheduledIntent {
            intent_id: intent.intent_id.clone(),
            asset: intent.asset,
            destination: intent.destination,
            amount: intent.amount,
            start_ledger: intent.start_ledger,
            end_ledger: intent.end_ledger,
            max_executions: intent.max_executions,
            execution_count: 0,
            policy_version: intent.policy_version,
            adapter: intent.adapter,
            cancelled: false,
        };

        env.storage().persistent().set(&key, &stored_intent);
        bump_ttl(&env, &key);
        IntentCreated {
            intent_id: intent.intent_id,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_intent(
        env: Env,
        intent_id: BytesN<32>,
    ) -> Result<ScheduledIntent, IntentRegistryError> {
        ensure_initialized(&env)?;
        let key = DataKey::Intent(intent_id);
        let intent = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(IntentRegistryError::IntentNotFound)?;
        bump_ttl(&env, &key);
        Ok(intent)
    }

    pub fn cancel_intent(env: Env, intent_id: BytesN<32>) -> Result<(), IntentRegistryError> {
        ensure_admin(&env)?;
        let key = DataKey::Intent(intent_id.clone());
        let mut intent: ScheduledIntent = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(IntentRegistryError::IntentNotFound)?;
        intent.cancelled = true;
        env.storage().persistent().set(&key, &intent);
        bump_ttl(&env, &key);
        IntentCancelled {
            intent_id: intent_id.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn mark_child_executed(
        env: Env,
        intent_id: BytesN<32>,
        child_sequence: u32,
    ) -> Result<(), IntentRegistryError> {
        ensure_initialized(&env)?;
        ensure_executor(&env)?;

        let key = DataKey::Intent(intent_id.clone());
        let mut intent: ScheduledIntent = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(IntentRegistryError::IntentNotFound)?;
        if intent.cancelled {
            return Err(IntentRegistryError::IntentCancelled);
        }
        let ledger_sequence = env.ledger().sequence();
        if ledger_sequence < intent.start_ledger {
            return Err(IntentRegistryError::ExecutionTooEarly);
        }
        if ledger_sequence > intent.end_ledger {
            return Err(IntentRegistryError::ExecutionExpired);
        }
        if intent.execution_count >= intent.max_executions {
            return Err(IntentRegistryError::ExecutionLimitReached);
        }

        let child_key = DataKey::ChildExecution(intent_id.clone(), child_sequence);
        if env.storage().persistent().has(&child_key) {
            return Err(IntentRegistryError::ChildAlreadyExecuted);
        }

        // Usage counter and child-replay marker are written together: a
        // reentrant or duplicate call in the same invocation cannot observe
        // a state where the child is marked but the count wasn't advanced,
        // or vice versa.
        intent.execution_count += 1;
        env.storage().persistent().set(&key, &intent);
        env.storage().persistent().set(&child_key, &true);
        bump_ttl(&env, &key);
        bump_ttl(&env, &child_key);
        ChildExecuted {
            intent_id: intent_id.clone(),
            child_sequence,
        }
        .publish(&env);
        Ok(())
    }

    pub fn is_child_executed(
        env: Env,
        intent_id: BytesN<32>,
        child_sequence: u32,
    ) -> Result<bool, IntentRegistryError> {
        ensure_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .has(&DataKey::ChildExecution(intent_id, child_sequence)))
    }
}

fn bump_ttl(env: &Env, key: &DataKey) {
    if env.storage().persistent().has(key) {
        env.storage()
            .persistent()
            .extend_ttl(key, TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    }
}

fn ensure_initialized(env: &Env) -> Result<(), IntentRegistryError> {
    if env.storage().persistent().has(&DataKey::Initialized) {
        Ok(())
    } else {
        Err(IntentRegistryError::NotInitialized)
    }
}

fn ensure_admin(env: &Env) -> Result<Address, IntentRegistryError> {
    ensure_initialized(env)?;
    let admin: Address = env
        .storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(IntentRegistryError::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}

fn ensure_executor(env: &Env) -> Result<Address, IntentRegistryError> {
    let executor: Address = env
        .storage()
        .persistent()
        .get(&DataKey::Executor)
        .ok_or(IntentRegistryError::UnauthorizedExecutor)?;
    executor.require_auth();
    Ok(executor)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::{storage::Persistent as _, Address as _, Ledger, LedgerInfo};

    fn setup() -> (Env, IntentRegistryClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(IntentRegistry, ());
        let client = IntentRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    fn set_ledger(env: &Env, sequence_number: u32) {
        env.ledger().set(LedgerInfo {
            timestamp: sequence_number as u64,
            protocol_version: 26,
            sequence_number,
            network_id: Default::default(),
            base_reserve: 5,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 16,
            max_entry_ttl: 3_110_400, // ~6 years, matches real Stellar network config headroom
        });
    }

    fn intent(env: &Env, id_seed: u8) -> ScheduledIntent {
        ScheduledIntent {
            intent_id: BytesN::from_array(env, &[id_seed; 32]),
            asset: Address::generate(env),
            destination: Address::generate(env),
            amount: 100,
            start_ledger: 10,
            end_ledger: 20,
            max_executions: 3,
            execution_count: 0,
            policy_version: 1,
            adapter: Address::generate(env),
            cancelled: false,
        }
    }

    /// The policy version pinned at creation must be exactly what the
    /// caller supplied and must survive round-trip storage unchanged —
    /// `execute_scheduled_payment` in `smart_account` relies on this value
    /// being canonical rather than trusting a caller-supplied version at
    /// execution time.
    #[test]
    fn policy_version_is_pinned_at_creation_and_returned_verbatim() {
        let (env, client, _admin) = setup();
        let mut scheduled = intent(&env, 7);
        scheduled.policy_version = 4;
        let intent_id = scheduled.intent_id.clone();

        client.create_intent(&scheduled);

        assert_eq!(client.get_intent(&intent_id).policy_version, 4);
    }

    #[test]
    fn creates_intent_and_marks_child_execution_once() {
        let (env, client, _admin) = setup();
        let scheduled = intent(&env, 1);
        let intent_id = scheduled.intent_id.clone();

        client.create_intent(&scheduled);
        set_ledger(&env, 12);
        client.mark_child_executed(&intent_id, &1);

        assert!(client.is_child_executed(&intent_id, &1));
        assert_eq!(client.get_intent(&intent_id).execution_count, 1);
        let replay = client.try_mark_child_executed(&intent_id, &1);
        assert_eq!(replay, Err(Ok(IntentRegistryError::ChildAlreadyExecuted)));
    }

    #[test]
    fn rejects_execution_outside_window() {
        let (env, client, _admin) = setup();
        let scheduled = intent(&env, 2);
        let intent_id = scheduled.intent_id.clone();

        client.create_intent(&scheduled);

        set_ledger(&env, 9);
        let early = client.try_mark_child_executed(&intent_id, &1);
        assert_eq!(early, Err(Ok(IntentRegistryError::ExecutionTooEarly)));

        set_ledger(&env, 21);
        let expired = client.try_mark_child_executed(&intent_id, &1);
        assert_eq!(expired, Err(Ok(IntentRegistryError::ExecutionExpired)));
    }

    #[test]
    fn cancelled_intent_cannot_execute() {
        let (env, client, _admin) = setup();
        let scheduled = intent(&env, 3);
        let intent_id = scheduled.intent_id.clone();

        client.create_intent(&scheduled);
        client.cancel_intent(&intent_id);

        set_ledger(&env, 12);
        let cancelled = client.try_mark_child_executed(&intent_id, &1);
        assert_eq!(cancelled, Err(Ok(IntentRegistryError::IntentCancelled)));
    }

    #[test]
    fn create_intent_ignores_caller_cancelled_flag_and_execution_count() {
        let (env, client, _admin) = setup();
        let mut scheduled = intent(&env, 4);
        scheduled.cancelled = true;
        scheduled.execution_count = 99;
        let intent_id = scheduled.intent_id.clone();

        client.create_intent(&scheduled);

        let stored = client.get_intent(&intent_id);
        assert!(!stored.cancelled);
        assert_eq!(stored.execution_count, 0);
    }

    #[test]
    fn zero_max_executions_is_rejected_at_creation() {
        let (env, client, _admin) = setup();
        let mut scheduled = intent(&env, 5);
        scheduled.max_executions = 0;

        let err = client.try_create_intent(&scheduled);
        assert_eq!(err, Err(Ok(IntentRegistryError::InvalidMaxExecutions)));
    }

    /// Edge case: a recurring intent with distinct child_sequence values
    /// (so no single-child replay is involved) still cannot exceed its
    /// declared max_executions — the cumulative usage cap is enforced
    /// independently of per-child replay protection.
    #[test]
    fn cumulative_execution_limit_is_enforced_across_distinct_children() {
        let (env, client, _admin) = setup();
        let mut scheduled = intent(&env, 6);
        scheduled.max_executions = 2;
        let intent_id = scheduled.intent_id.clone();

        client.create_intent(&scheduled);
        set_ledger(&env, 12);

        client.mark_child_executed(&intent_id, &1);
        client.mark_child_executed(&intent_id, &2);
        assert_eq!(client.get_intent(&intent_id).execution_count, 2);

        let over_limit = client.try_mark_child_executed(&intent_id, &3);
        assert_eq!(
            over_limit,
            Err(Ok(IntentRegistryError::ExecutionLimitReached))
        );
    }

    /// Audit finding (`docs/TECHNICAL_ARCHITECTURE.md` §16): scheduled
    /// intents can sit untouched between creation and their execution
    /// window opening, so both `create_intent`/`mark_child_executed`
    /// writes and `get_intent`/`extend_intent_ttl` reads must extend TTL,
    /// or a long-dormant recurring intent could archive before it's ever
    /// due to run.
    #[test]
    fn intent_and_child_execution_ttl_is_extended_on_write_and_read() {
        let (env, client, contract_id) = {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register(IntentRegistry, ());
            let client = IntentRegistryClient::new(&env, &contract_id);
            let admin = Address::generate(&env);
            client.initialize(&admin);
            (env, client, contract_id)
        };
        let scheduled = intent(&env, 30);
        let intent_id = scheduled.intent_id.clone();
        client.create_intent(&scheduled);

        let ttl_after_create = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Intent(intent_id.clone()))
        });
        assert!(ttl_after_create >= TTL_THRESHOLD_LEDGERS);

        set_ledger(&env, 12);
        client.mark_child_executed(&intent_id, &1);
        let child_key = DataKey::ChildExecution(intent_id.clone(), 1);
        let child_ttl = env.as_contract(&contract_id, || {
            env.storage().persistent().get_ttl(&child_key)
        });
        assert!(child_ttl >= TTL_THRESHOLD_LEDGERS);

        // Decay a long way, then confirm the permissionless maintenance
        // entrypoint (no auth at all) refreshes both entries.
        env.ledger()
            .with_mut(|l| l.sequence_number += TTL_THRESHOLD_LEDGERS / 2);
        env.set_auths(&[]);
        client.extend_intent_ttl(&intent_id, &soroban_sdk::vec![&env, 1u32]);

        let refreshed_intent_ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Intent(intent_id.clone()))
        });
        let refreshed_child_ttl = env.as_contract(&contract_id, || {
            env.storage().persistent().get_ttl(&child_key)
        });
        assert!(refreshed_intent_ttl >= TTL_THRESHOLD_LEDGERS);
        assert!(refreshed_child_ttl >= TTL_THRESHOLD_LEDGERS);
    }

    #[test]
    fn contract_name_reports_expected_symbol() {
        assert_eq!(IntentRegistry::contract_name(), symbol_short!("sta_int"));
    }

    #[test]
    fn calling_before_initialize_rejects_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(IntentRegistry, ());
        let client = IntentRegistryClient::new(&env, &contract_id);

        let err = client.try_get_intent(&BytesN::from_array(&env, &[0u8; 32]));
        assert_eq!(err, Err(Ok(IntentRegistryError::NotInitialized)));
    }

    #[test]
    fn double_initialize_is_rejected() {
        let (_env, client, admin) = setup();
        let err = client.try_initialize(&admin);
        assert_eq!(err, Err(Ok(IntentRegistryError::AlreadyInitialized)));
    }

    #[test]
    fn create_intent_rejects_non_positive_amount() {
        let (env, client, _admin) = setup();
        let mut scheduled = intent(&env, 40);
        scheduled.amount = 0;

        let err = client.try_create_intent(&scheduled);
        assert_eq!(err, Err(Ok(IntentRegistryError::InvalidAmount)));
    }

    #[test]
    fn create_intent_rejects_inverted_window() {
        let (env, client, _admin) = setup();
        let mut scheduled = intent(&env, 41);
        scheduled.start_ledger = 20;
        scheduled.end_ledger = 10;

        let err = client.try_create_intent(&scheduled);
        assert_eq!(err, Err(Ok(IntentRegistryError::InvalidWindow)));
    }

    #[test]
    fn create_intent_rejects_duplicate_intent_id() {
        let (env, client, _admin) = setup();
        let scheduled = intent(&env, 42);

        client.create_intent(&scheduled);
        let err = client.try_create_intent(&scheduled);
        assert_eq!(err, Err(Ok(IntentRegistryError::IntentAlreadyExists)));
    }
}
