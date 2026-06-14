#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol,
};

#[contract]
pub struct IntentRegistryPoc;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledIntent {
    pub intent_id: BytesN<32>,
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub cancelled: bool,
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
}

#[contractimpl]
impl IntentRegistryPoc {
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
        env.events().publish((symbol_short!("init"),), admin);
        Ok(())
    }

    pub fn set_executor(env: Env, executor: Address) -> Result<(), IntentRegistryError> {
        ensure_admin(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::Executor, &executor);
        env.events().publish((symbol_short!("execset"),), executor);
        Ok(())
    }

    pub fn create_intent(env: Env, intent: ScheduledIntent) -> Result<(), IntentRegistryError> {
        ensure_admin(&env)?;
        if intent.amount <= 0 {
            return Err(IntentRegistryError::InvalidAmount);
        }
        if intent.end_ledger < intent.start_ledger {
            return Err(IntentRegistryError::InvalidWindow);
        }

        let key = DataKey::Intent(intent.intent_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(IntentRegistryError::IntentAlreadyExists);
        }

        let stored_intent = ScheduledIntent {
            intent_id: intent.intent_id.clone(),
            asset: intent.asset,
            destination: intent.destination,
            amount: intent.amount,
            start_ledger: intent.start_ledger,
            end_ledger: intent.end_ledger,
            cancelled: false,
        };

        env.storage().persistent().set(&key, &stored_intent);
        env.events()
            .publish((symbol_short!("intent"),), intent.intent_id);
        Ok(())
    }

    pub fn get_intent(
        env: Env,
        intent_id: BytesN<32>,
    ) -> Result<ScheduledIntent, IntentRegistryError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::Intent(intent_id))
            .ok_or(IntentRegistryError::IntentNotFound)
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
        env.events().publish((symbol_short!("cancel"),), intent_id);
        Ok(())
    }

    pub fn mark_child_executed(
        env: Env,
        intent_id: BytesN<32>,
        child_sequence: u32,
    ) -> Result<(), IntentRegistryError> {
        ensure_initialized(&env)?;
        ensure_executor(&env)?;

        let intent: ScheduledIntent = env
            .storage()
            .persistent()
            .get(&DataKey::Intent(intent_id.clone()))
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

        let child_key = DataKey::ChildExecution(intent_id.clone(), child_sequence);
        if env.storage().persistent().has(&child_key) {
            return Err(IntentRegistryError::ChildAlreadyExecuted);
        }

        env.storage().persistent().set(&child_key, &true);
        env.events()
            .publish((symbol_short!("exec"),), (intent_id, child_sequence));
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
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

    fn setup() -> (Env, IntentRegistryPocClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(IntentRegistryPoc, ());
        let client = IntentRegistryPocClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    fn set_ledger(env: &Env, sequence_number: u32) {
        env.ledger().set(LedgerInfo {
            timestamp: sequence_number as u64,
            protocol_version: 22,
            sequence_number,
            network_id: Default::default(),
            base_reserve: 5,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 16,
            max_entry_ttl: 500_000,
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
            cancelled: false,
        }
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
    fn create_intent_ignores_caller_cancelled_flag() {
        let (env, client, _admin) = setup();
        let mut scheduled = intent(&env, 4);
        scheduled.cancelled = true;
        let intent_id = scheduled.intent_id.clone();

        client.create_intent(&scheduled);

        assert!(!client.get_intent(&intent_id).cancelled);
    }
}
