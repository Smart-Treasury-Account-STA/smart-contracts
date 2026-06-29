#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
    Symbol, Vec,
};

#[contract]
pub struct RecoveryGuardPoc;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRequest {
    pub request_id: BytesN<32>,
    pub replacement_signer: BytesN<32>,
    pub earliest_ledger: u32,
    pub approvals: u32,
    pub cancelled: bool,
    pub finalized: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
    Admin,
    GuardianThreshold,
    Guardian(Address),
    Request(BytesN<32>),
    Approval(BytesN<32>, Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RecoveryGuardError {
    AlreadyInitialized = 4000,
    NotInitialized = 4001,
    InvalidThreshold = 4002,
    GuardianAlreadyExists = 4003,
    GuardianNotFound = 4004,
    RequestAlreadyExists = 4005,
    RequestNotFound = 4006,
    DuplicateApproval = 4007,
    BelowThreshold = 4008,
    TimelockActive = 4009,
    RequestCancelled = 4010,
    RequestFinalized = 4011,
}

#[contractimpl]
impl RecoveryGuardPoc {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_rec")
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        guardian_threshold: u32,
    ) -> Result<(), RecoveryGuardError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(RecoveryGuardError::AlreadyInitialized);
        }
        if guardian_threshold == 0 {
            return Err(RecoveryGuardError::InvalidThreshold);
        }

        admin.require_auth();
        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::GuardianThreshold, &guardian_threshold);
        env.events()
            .publish((symbol_short!("init"),), (admin, guardian_threshold));
        Ok(())
    }

    pub fn add_guardian(env: Env, guardian: Address) -> Result<(), RecoveryGuardError> {
        ensure_admin(&env)?;
        let key = DataKey::Guardian(guardian.clone());
        if env.storage().persistent().has(&key) {
            return Err(RecoveryGuardError::GuardianAlreadyExists);
        }

        env.storage().persistent().set(&key, &true);
        env.events().publish((symbol_short!("guard"),), guardian);
        Ok(())
    }

    pub fn remove_guardian(env: Env, guardian: Address) -> Result<(), RecoveryGuardError> {
        ensure_admin(&env)?;
        let key = DataKey::Guardian(guardian.clone());
        if !env.storage().persistent().has(&key) {
            return Err(RecoveryGuardError::GuardianNotFound);
        }

        env.storage().persistent().remove(&key);
        env.events().publish((symbol_short!("unguard"),), guardian);
        Ok(())
    }

    pub fn open_recovery(
        env: Env,
        request_id: BytesN<32>,
        replacement_signer: BytesN<32>,
        earliest_ledger: u32,
    ) -> Result<(), RecoveryGuardError> {
        ensure_admin(&env)?;
        let key = DataKey::Request(request_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(RecoveryGuardError::RequestAlreadyExists);
        }

        let request = RecoveryRequest {
            request_id: request_id.clone(),
            replacement_signer,
            earliest_ledger,
            approvals: 0,
            cancelled: false,
            finalized: false,
        };
        env.storage().persistent().set(&key, &request);
        env.events().publish((symbol_short!("open"),), request_id);
        Ok(())
    }

    pub fn approve_recovery(
        env: Env,
        request_id: BytesN<32>,
        guardian: Address,
    ) -> Result<(), RecoveryGuardError> {
        ensure_initialized(&env)?;
        guardian.require_auth();
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Guardian(guardian.clone()))
        {
            return Err(RecoveryGuardError::GuardianNotFound);
        }

        let key = DataKey::Request(request_id.clone());
        let mut request: RecoveryRequest = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RecoveryGuardError::RequestNotFound)?;
        ensure_request_open(&request)?;

        let approval_key = DataKey::Approval(request_id.clone(), guardian.clone());
        if env.storage().persistent().has(&approval_key) {
            return Err(RecoveryGuardError::DuplicateApproval);
        }

        env.storage().persistent().set(&approval_key, &true);
        request.approvals = request.approvals.saturating_add(1);
        env.storage().persistent().set(&key, &request);
        env.events()
            .publish((symbol_short!("appr"),), (request_id, guardian));
        Ok(())
    }

    pub fn cancel_recovery(env: Env, request_id: BytesN<32>) -> Result<(), RecoveryGuardError> {
        ensure_admin(&env)?;
        let key = DataKey::Request(request_id.clone());
        let mut request: RecoveryRequest = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RecoveryGuardError::RequestNotFound)?;
        ensure_request_open(&request)?;

        request.cancelled = true;
        env.storage().persistent().set(&key, &request);
        env.events().publish((symbol_short!("cancel"),), request_id);
        Ok(())
    }

    pub fn finalize_recovery(
        env: Env,
        request_id: BytesN<32>,
    ) -> Result<BytesN<32>, RecoveryGuardError> {
        ensure_initialized(&env)?;
        let key = DataKey::Request(request_id.clone());
        let mut request: RecoveryRequest = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RecoveryGuardError::RequestNotFound)?;
        ensure_request_open(&request)?;

        if request.approvals < guardian_threshold(&env) {
            return Err(RecoveryGuardError::BelowThreshold);
        }
        if env.ledger().sequence() < request.earliest_ledger {
            return Err(RecoveryGuardError::TimelockActive);
        }

        request.finalized = true;
        env.storage().persistent().set(&key, &request);
        env.events().publish(
            (symbol_short!("final"),),
            (request_id, request.replacement_signer.clone()),
        );
        Ok(request.replacement_signer)
    }

    pub fn request_status(
        env: Env,
        request_id: BytesN<32>,
    ) -> Result<RecoveryRequest, RecoveryGuardError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(RecoveryGuardError::RequestNotFound)
    }

    pub fn guardian_counted(
        env: Env,
        request_id: BytesN<32>,
        guardians: Vec<Address>,
    ) -> Result<u32, RecoveryGuardError> {
        ensure_initialized(&env)?;
        let mut counted = 0_u32;
        let mut seen = Vec::<Address>::new(&env);

        for guardian in guardians.iter() {
            if seen.contains(&guardian) {
                continue;
            }
            seen.push_back(guardian.clone());

            if env
                .storage()
                .persistent()
                .has(&DataKey::Approval(request_id.clone(), guardian))
            {
                counted = counted.saturating_add(1);
            }
        }
        Ok(counted)
    }
}

fn ensure_initialized(env: &Env) -> Result<(), RecoveryGuardError> {
    if env.storage().persistent().has(&DataKey::Initialized) {
        Ok(())
    } else {
        Err(RecoveryGuardError::NotInitialized)
    }
}

fn ensure_admin(env: &Env) -> Result<Address, RecoveryGuardError> {
    ensure_initialized(env)?;
    let admin: Address = env
        .storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(RecoveryGuardError::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}

fn guardian_threshold(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::GuardianThreshold)
        .unwrap_or(1)
}

fn ensure_request_open(request: &RecoveryRequest) -> Result<(), RecoveryGuardError> {
    if request.cancelled {
        return Err(RecoveryGuardError::RequestCancelled);
    }
    if request.finalized {
        return Err(RecoveryGuardError::RequestFinalized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        vec,
    };

    fn setup() -> (Env, RecoveryGuardPocClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RecoveryGuardPoc, ());
        let client = RecoveryGuardPocClient::new(&env, &contract_id);
        client.initialize(&Address::generate(&env), &2);
        (env, client)
    }

    fn id(env: &Env, seed: u8) -> BytesN<32> {
        BytesN::from_array(env, &[seed; 32])
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

    #[test]
    fn finalizes_after_threshold_and_timelock() {
        let (env, client) = setup();
        let request_id = id(&env, 1);
        let replacement = id(&env, 9);
        let guardian_a = Address::generate(&env);
        let guardian_b = Address::generate(&env);

        client.add_guardian(&guardian_a);
        client.add_guardian(&guardian_b);
        client.open_recovery(&request_id, &replacement, &50);
        client.approve_recovery(&request_id, &guardian_a);
        client.approve_recovery(&request_id, &guardian_b);

        assert_eq!(
            client.guardian_counted(
                &request_id,
                &vec![&env, guardian_a.clone(), guardian_a, guardian_b]
            ),
            2
        );
        set_ledger(&env, 50);
        assert_eq!(client.finalize_recovery(&request_id), replacement);
    }

    #[test]
    fn rejects_duplicate_guardian_approval_and_below_threshold() {
        let (env, client) = setup();
        let request_id = id(&env, 4);
        let guardian = Address::generate(&env);

        client.add_guardian(&guardian);
        client.open_recovery(&request_id, &id(&env, 6), &10);
        client.approve_recovery(&request_id, &guardian);

        let duplicate = client.try_approve_recovery(&request_id, &guardian);
        assert_eq!(duplicate, Err(Ok(RecoveryGuardError::DuplicateApproval)));

        set_ledger(&env, 10);
        let below_threshold = client.try_finalize_recovery(&request_id);
        assert_eq!(below_threshold, Err(Ok(RecoveryGuardError::BelowThreshold)));
    }

    #[test]
    fn removed_guardian_cannot_approve_and_double_removal_fails() {
        let (env, client) = setup();
        let request_id = id(&env, 5);
        let guardian = Address::generate(&env);

        client.add_guardian(&guardian);
        client.remove_guardian(&guardian);

        client.open_recovery(&request_id, &id(&env, 6), &10);
        let revoked = client.try_approve_recovery(&request_id, &guardian);
        assert_eq!(revoked, Err(Ok(RecoveryGuardError::GuardianNotFound)));

        let double_removal = client.try_remove_guardian(&guardian);
        assert_eq!(double_removal, Err(Ok(RecoveryGuardError::GuardianNotFound)));
    }

    #[test]
    fn cancelled_request_cannot_finalize() {
        let (env, client) = setup();
        let request_id = id(&env, 7);

        client.open_recovery(&request_id, &id(&env, 8), &10);
        client.cancel_recovery(&request_id);

        set_ledger(&env, 10);
        let cancelled = client.try_finalize_recovery(&request_id);
        assert_eq!(cancelled, Err(Ok(RecoveryGuardError::RequestCancelled)));
    }

    #[test]
    fn rejects_finalization_before_timelock() {
        let (env, client) = setup();
        let request_id = id(&env, 10);
        let guardian_a = Address::generate(&env);
        let guardian_b = Address::generate(&env);

        client.add_guardian(&guardian_a);
        client.add_guardian(&guardian_b);
        client.open_recovery(&request_id, &id(&env, 11), &30);
        client.approve_recovery(&request_id, &guardian_a);
        client.approve_recovery(&request_id, &guardian_b);

        set_ledger(&env, 29);
        let timelock = client.try_finalize_recovery(&request_id);
        assert_eq!(timelock, Err(Ok(RecoveryGuardError::TimelockActive)));
    }
}
