#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
    Symbol, Vec,
};

const ROLE_PAYMENT: u32 = 1;
const DEFAULT_POLICY_VERSION: u32 = 1;

#[contract]
pub struct SmartAccountPoc;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerRecord {
    pub signer_id: BytesN<32>,
    pub roles: u32,
    pub weight: u32,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPolicy {
    pub enabled: bool,
    pub max_single_transfer: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentAction {
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
    pub nonce: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountStatus {
    pub initialized: bool,
    pub paused: bool,
    pub frozen: bool,
    pub payment_threshold: u32,
    pub payment_weight: u32,
    pub policy_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
    Admin,
    PaymentThreshold,
    PolicyVersion,
    PaymentWeight,
    Paused,
    Frozen,
    Signer(BytesN<32>),
    Asset(Address),
    Recipient(Address),
    UsedNonce(u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SmartAccountPocError {
    AlreadyInitialized = 1000,
    NotInitialized = 1001,
    Unauthorized = 1002,
    InvalidThreshold = 1003,
    InvalidSigner = 1004,
    AssetNotAllowed = 1005,
    RecipientNotAllowed = 1006,
    InvalidAmount = 1007,
    AmountAboveLimit = 1008,
    NonceAlreadyUsed = 1009,
    Paused = 1010,
    Frozen = 1011,
    PolicyVersionMismatch = 1012,
    DuplicateSigner = 1013,
    SignerNotFound = 1014,
    SignerInactive = 1015,
    MissingPaymentRole = 1016,
    WeightOverflow = 1017,
    InvalidPolicyVersion = 1018,
}

#[contractimpl]
impl SmartAccountPoc {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_poc")
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        payment_threshold: u32,
    ) -> Result<(), SmartAccountPocError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(SmartAccountPocError::AlreadyInitialized);
        }
        if payment_threshold == 0 {
            return Err(SmartAccountPocError::InvalidThreshold);
        }

        admin.require_auth();

        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::PaymentThreshold, &payment_threshold);
        env.storage()
            .persistent()
            .set(&DataKey::PolicyVersion, &DEFAULT_POLICY_VERSION);
        env.storage()
            .persistent()
            .set(&DataKey::PaymentWeight, &0_u32);
        env.storage().persistent().set(&DataKey::Paused, &false);
        env.storage().persistent().set(&DataKey::Frozen, &false);

        env.events()
            .publish((symbol_short!("init"),), (admin, payment_threshold));
        Ok(())
    }

    pub fn status(env: Env) -> Result<AccountStatus, SmartAccountPocError> {
        ensure_initialized(&env)?;
        Ok(AccountStatus {
            initialized: true,
            paused: env
                .storage()
                .persistent()
                .get(&DataKey::Paused)
                .unwrap_or(false),
            frozen: env
                .storage()
                .persistent()
                .get(&DataKey::Frozen)
                .unwrap_or(false),
            payment_threshold: payment_threshold(&env),
            payment_weight: payment_weight(&env),
            policy_version: policy_version(&env),
        })
    }

    pub fn get_signer(
        env: Env,
        signer_id: BytesN<32>,
    ) -> Result<SignerRecord, SmartAccountPocError> {
        ensure_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::Signer(signer_id))
            .ok_or(SmartAccountPocError::SignerNotFound)
    }

    pub fn is_nonce_used(env: Env, nonce: u64) -> Result<bool, SmartAccountPocError> {
        ensure_initialized(&env)?;
        Ok(env.storage().persistent().has(&DataKey::UsedNonce(nonce)))
    }

    pub fn add_signer(env: Env, signer: SignerRecord) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;
        if signer.weight == 0 || !signer.active {
            return Err(SmartAccountPocError::InvalidSigner);
        }

        let key = DataKey::Signer(signer.signer_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(SmartAccountPocError::DuplicateSigner);
        }

        if signer.roles & ROLE_PAYMENT != 0 {
            let next_weight = payment_weight(&env)
                .checked_add(signer.weight)
                .ok_or(SmartAccountPocError::WeightOverflow)?;
            env.storage()
                .persistent()
                .set(&DataKey::PaymentWeight, &next_weight);
        }

        env.storage().persistent().set(&key, &signer);
        env.events()
            .publish((symbol_short!("signer"),), signer.signer_id);
        Ok(())
    }

    pub fn revoke_signer(env: Env, signer_id: BytesN<32>) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;

        let key = DataKey::Signer(signer_id.clone());
        let mut signer: SignerRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(SmartAccountPocError::SignerNotFound)?;
        if !signer.active {
            return Err(SmartAccountPocError::SignerInactive);
        }

        signer.active = false;
        if signer.roles & ROLE_PAYMENT != 0 {
            let next_weight = payment_weight(&env)
                .checked_sub(signer.weight)
                .ok_or(SmartAccountPocError::InvalidSigner)?;
            env.storage()
                .persistent()
                .set(&DataKey::PaymentWeight, &next_weight);
        }

        env.storage().persistent().set(&key, &signer);
        env.events().publish((symbol_short!("revoke"),), signer_id);
        Ok(())
    }

    pub fn update_policy_version(
        env: Env,
        next_policy_version: u32,
    ) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;
        if next_policy_version <= policy_version(&env) {
            return Err(SmartAccountPocError::InvalidPolicyVersion);
        }
        env.storage()
            .persistent()
            .set(&DataKey::PolicyVersion, &next_policy_version);
        env.events()
            .publish((symbol_short!("policy"),), next_policy_version);
        Ok(())
    }

    pub fn set_asset_policy(
        env: Env,
        asset: Address,
        policy: AssetPolicy,
    ) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;
        if policy.enabled && policy.max_single_transfer <= 0 {
            return Err(SmartAccountPocError::InvalidAmount);
        }
        env.storage()
            .persistent()
            .set(&DataKey::Asset(asset.clone()), &policy);
        env.events().publish((symbol_short!("asset"),), asset);
        Ok(())
    }

    pub fn set_recipient_allowed(
        env: Env,
        recipient: Address,
        allowed: bool,
    ) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::Recipient(recipient.clone()), &allowed);
        env.events()
            .publish((symbol_short!("rcpt"),), (recipient, allowed));
        Ok(())
    }

    pub fn pause(env: Env) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;
        env.storage().persistent().set(&DataKey::Paused, &true);
        Ok(())
    }

    pub fn unpause(env: Env) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;
        env.storage().persistent().set(&DataKey::Paused, &false);
        Ok(())
    }

    pub fn freeze(env: Env) -> Result<(), SmartAccountPocError> {
        ensure_admin(&env)?;
        env.storage().persistent().set(&DataKey::Frozen, &true);
        Ok(())
    }

    pub fn validate_payment(
        env: Env,
        action: PaymentAction,
        expected_policy_version: u32,
        approvers: Vec<BytesN<32>>,
    ) -> Result<(), SmartAccountPocError> {
        ensure_initialized(&env)?;
        ensure_active(&env)?;

        if expected_policy_version != policy_version(&env) {
            return Err(SmartAccountPocError::PolicyVersionMismatch);
        }
        if approved_payment_weight(&env, &approvers)? < payment_threshold(&env) {
            return Err(SmartAccountPocError::Unauthorized);
        }
        if action.amount <= 0 {
            return Err(SmartAccountPocError::InvalidAmount);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::UsedNonce(action.nonce))
        {
            return Err(SmartAccountPocError::NonceAlreadyUsed);
        }

        let asset_policy: AssetPolicy = env
            .storage()
            .persistent()
            .get(&DataKey::Asset(action.asset.clone()))
            .ok_or(SmartAccountPocError::AssetNotAllowed)?;
        if !asset_policy.enabled {
            return Err(SmartAccountPocError::AssetNotAllowed);
        }
        if action.amount > asset_policy.max_single_transfer {
            return Err(SmartAccountPocError::AmountAboveLimit);
        }

        let recipient_allowed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Recipient(action.destination.clone()))
            .unwrap_or(false);
        if !recipient_allowed {
            return Err(SmartAccountPocError::RecipientNotAllowed);
        }

        env.storage()
            .persistent()
            .set(&DataKey::UsedNonce(action.nonce), &true);
        env.events().publish(
            (symbol_short!("pay_ok"),),
            (
                action.asset,
                action.destination,
                action.amount,
                action.nonce,
            ),
        );
        Ok(())
    }
}

fn ensure_initialized(env: &Env) -> Result<(), SmartAccountPocError> {
    if env.storage().persistent().has(&DataKey::Initialized) {
        Ok(())
    } else {
        Err(SmartAccountPocError::NotInitialized)
    }
}

fn ensure_admin(env: &Env) -> Result<Address, SmartAccountPocError> {
    ensure_initialized(env)?;
    let admin: Address = env
        .storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(SmartAccountPocError::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}

fn ensure_active(env: &Env) -> Result<(), SmartAccountPocError> {
    if env
        .storage()
        .persistent()
        .get(&DataKey::Paused)
        .unwrap_or(false)
    {
        return Err(SmartAccountPocError::Paused);
    }
    if env
        .storage()
        .persistent()
        .get(&DataKey::Frozen)
        .unwrap_or(false)
    {
        return Err(SmartAccountPocError::Frozen);
    }
    Ok(())
}

fn payment_threshold(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::PaymentThreshold)
        .unwrap_or(1)
}

fn payment_weight(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::PaymentWeight)
        .unwrap_or(0)
}

fn policy_version(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::PolicyVersion)
        .unwrap_or(DEFAULT_POLICY_VERSION)
}

fn approved_payment_weight(
    env: &Env,
    approvers: &Vec<BytesN<32>>,
) -> Result<u32, SmartAccountPocError> {
    let mut seen = Vec::<BytesN<32>>::new(env);
    let mut total = 0_u32;

    for signer_id in approvers.iter() {
        if seen.contains(&signer_id) {
            return Err(SmartAccountPocError::DuplicateSigner);
        }
        seen.push_back(signer_id.clone());

        let record: SignerRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Signer(signer_id))
            .ok_or(SmartAccountPocError::SignerNotFound)?;
        if !record.active {
            return Err(SmartAccountPocError::SignerInactive);
        }
        if record.roles & ROLE_PAYMENT == 0 {
            return Err(SmartAccountPocError::MissingPaymentRole);
        }
        total = total
            .checked_add(record.weight)
            .ok_or(SmartAccountPocError::WeightOverflow)?;
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::{testutils::Address as _, vec};

    fn setup() -> (Env, Address, SmartAccountPocClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SmartAccountPoc, ());
        let client = SmartAccountPocClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &1);
        (env, admin, client)
    }

    fn signer(env: &Env, seed: u8, weight: u32) -> SignerRecord {
        SignerRecord {
            signer_id: BytesN::from_array(env, &[seed; 32]),
            roles: ROLE_PAYMENT,
            weight,
            active: true,
        }
    }

    fn valid_action(asset: &Address, recipient: &Address, nonce: u64) -> PaymentAction {
        PaymentAction {
            asset: asset.clone(),
            destination: recipient.clone(),
            amount: 25,
            nonce,
        }
    }

    #[test]
    fn initializes_and_reports_status() {
        let (_env, _admin, client) = setup();

        let status = client.status();

        assert!(status.initialized);
        assert!(!status.paused);
        assert!(!status.frozen);
        assert_eq!(status.payment_threshold, 1);
        assert_eq!(status.payment_weight, 0);
        assert_eq!(status.policy_version, 1);
    }

    #[test]
    fn validates_allowed_payment_and_consumes_nonce() {
        let (env, _admin, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.add_signer(&signer(&env, 1, 1));
        assert_eq!(client.status().payment_weight, 1);
        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        client.validate_payment(
            &valid_action(&asset, &recipient, 7),
            &1,
            &vec![&env, signer(&env, 1, 1).signer_id],
        );
        assert!(client.is_nonce_used(&7));

        let replay = client.try_validate_payment(
            &valid_action(&asset, &recipient, 7),
            &1,
            &vec![&env, signer(&env, 1, 1).signer_id],
        );
        assert_eq!(replay, Err(Ok(SmartAccountPocError::NonceAlreadyUsed)));
    }

    #[test]
    fn rejects_disallowed_asset_and_recipient() {
        let (env, _admin, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.add_signer(&signer(&env, 2, 1));
        let approvers = vec![&env, signer(&env, 2, 1).signer_id];

        let missing_asset =
            client.try_validate_payment(&valid_action(&asset, &recipient, 1), &1, &approvers);
        assert_eq!(
            missing_asset,
            Err(Ok(SmartAccountPocError::AssetNotAllowed))
        );

        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );

        let missing_recipient =
            client.try_validate_payment(&valid_action(&asset, &recipient, 2), &1, &approvers);
        assert_eq!(
            missing_recipient,
            Err(Ok(SmartAccountPocError::RecipientNotAllowed))
        );
    }

    #[test]
    fn rejects_amount_above_asset_limit() {
        let (env, _admin, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.add_signer(&signer(&env, 3, 1));
        let approvers = vec![&env, signer(&env, 3, 1).signer_id];
        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 10,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        let err = client.try_validate_payment(&valid_action(&asset, &recipient, 3), &1, &approvers);
        assert_eq!(err, Err(Ok(SmartAccountPocError::AmountAboveLimit)));
    }

    #[test]
    fn rejects_when_paused_or_frozen() {
        let (env, _admin, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.add_signer(&signer(&env, 4, 1));
        let approvers = vec![&env, signer(&env, 4, 1).signer_id];
        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        client.pause();
        let paused =
            client.try_validate_payment(&valid_action(&asset, &recipient, 4), &1, &approvers);
        assert_eq!(paused, Err(Ok(SmartAccountPocError::Paused)));

        client.unpause();
        client.freeze();
        let frozen =
            client.try_validate_payment(&valid_action(&asset, &recipient, 5), &1, &approvers);
        assert_eq!(frozen, Err(Ok(SmartAccountPocError::Frozen)));
    }

    #[test]
    fn rejects_when_payment_weight_is_below_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SmartAccountPoc, ());
        let client = SmartAccountPocClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.initialize(&admin, &2);
        client.add_signer(&signer(&env, 5, 1));
        let approvers = vec![&env, signer(&env, 5, 1).signer_id];
        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        let err = client.try_validate_payment(&valid_action(&asset, &recipient, 6), &1, &approvers);
        assert_eq!(err, Err(Ok(SmartAccountPocError::Unauthorized)));
    }

    #[test]
    fn accepts_multiple_approvers_meeting_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SmartAccountPoc, ());
        let client = SmartAccountPocClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);
        let signer_a = signer(&env, 6, 1);
        let signer_b = signer(&env, 7, 1);

        client.initialize(&admin, &2);
        client.add_signer(&signer_a);
        client.add_signer(&signer_b);
        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        client.validate_payment(
            &valid_action(&asset, &recipient, 8),
            &1,
            &vec![&env, signer_a.signer_id, signer_b.signer_id],
        );
    }

    #[test]
    fn rejects_duplicate_approver_entries() {
        let (env, _admin, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);
        let signer_record = signer(&env, 8, 1);

        client.add_signer(&signer_record);
        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        let err = client.try_validate_payment(
            &valid_action(&asset, &recipient, 9),
            &1,
            &vec![
                &env,
                signer_record.signer_id.clone(),
                signer_record.signer_id.clone(),
            ],
        );
        assert_eq!(err, Err(Ok(SmartAccountPocError::DuplicateSigner)));
    }

    #[test]
    fn revoking_signer_removes_payment_weight_and_blocks_approval() {
        let (env, _admin, client) = setup();
        let signer_record = signer(&env, 9, 1);
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.add_signer(&signer_record);
        assert_eq!(client.status().payment_weight, 1);
        client.revoke_signer(&signer_record.signer_id);

        let stored = client.get_signer(&signer_record.signer_id);
        assert!(!stored.active);
        assert_eq!(client.status().payment_weight, 0);

        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        let err = client.try_validate_payment(
            &valid_action(&asset, &recipient, 10),
            &1,
            &vec![&env, signer_record.signer_id],
        );
        assert_eq!(err, Err(Ok(SmartAccountPocError::SignerInactive)));
    }

    #[test]
    fn policy_version_update_rejects_stale_payment_context() {
        let (env, _admin, client) = setup();
        let signer_record = signer(&env, 10, 1);
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.add_signer(&signer_record);
        client.set_asset_policy(
            &asset,
            &AssetPolicy {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);
        client.update_policy_version(&2);

        let stale = client.try_validate_payment(
            &valid_action(&asset, &recipient, 11),
            &1,
            &vec![&env, signer_record.signer_id.clone()],
        );
        assert_eq!(stale, Err(Ok(SmartAccountPocError::PolicyVersionMismatch)));

        client.validate_payment(
            &valid_action(&asset, &recipient, 11),
            &2,
            &vec![&env, signer_record.signer_id],
        );
    }
}
