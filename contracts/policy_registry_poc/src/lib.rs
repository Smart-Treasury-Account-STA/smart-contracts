#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

const INITIAL_VERSION: u32 = 1;

#[contract]
pub struct PolicyRegistryPoc;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRule {
    pub enabled: bool,
    pub max_single_transfer: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyCheck {
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
    pub expected_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
    Admin,
    Version,
    Asset(Address),
    Recipient(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PolicyRegistryError {
    AlreadyInitialized = 2000,
    NotInitialized = 2001,
    InvalidAmount = 2002,
    AssetNotAllowed = 2003,
    RecipientNotAllowed = 2004,
    AmountAboveLimit = 2005,
    VersionMismatch = 2006,
    InvalidVersion = 2007,
}

#[contractimpl]
impl PolicyRegistryPoc {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_pol")
    }

    pub fn initialize(env: Env, admin: Address) -> Result<(), PolicyRegistryError> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(PolicyRegistryError::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::Version, &INITIAL_VERSION);
        env.events().publish((symbol_short!("init"),), admin);
        Ok(())
    }

    pub fn version(env: Env) -> Result<u32, PolicyRegistryError> {
        ensure_initialized(&env)?;
        Ok(current_version(&env))
    }

    pub fn set_asset_rule(
        env: Env,
        asset: Address,
        rule: AssetRule,
    ) -> Result<(), PolicyRegistryError> {
        ensure_admin(&env)?;
        if rule.enabled && rule.max_single_transfer <= 0 {
            return Err(PolicyRegistryError::InvalidAmount);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Asset(asset.clone()), &rule);
        env.events().publish((symbol_short!("asset"),), asset);
        Ok(())
    }

    pub fn set_recipient_allowed(
        env: Env,
        recipient: Address,
        allowed: bool,
    ) -> Result<(), PolicyRegistryError> {
        ensure_admin(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::Recipient(recipient.clone()), &allowed);
        env.events()
            .publish((symbol_short!("rcpt"),), (recipient, allowed));
        Ok(())
    }

    pub fn bump_version(env: Env, next_version: u32) -> Result<(), PolicyRegistryError> {
        ensure_admin(&env)?;
        if next_version <= current_version(&env) {
            return Err(PolicyRegistryError::InvalidVersion);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Version, &next_version);
        env.events()
            .publish((symbol_short!("policy"),), next_version);
        Ok(())
    }

    pub fn validate_policy(env: Env, check: PolicyCheck) -> Result<(), PolicyRegistryError> {
        ensure_initialized(&env)?;
        if check.expected_version != current_version(&env) {
            return Err(PolicyRegistryError::VersionMismatch);
        }
        if check.amount <= 0 {
            return Err(PolicyRegistryError::InvalidAmount);
        }

        let asset_rule: AssetRule = env
            .storage()
            .persistent()
            .get(&DataKey::Asset(check.asset.clone()))
            .ok_or(PolicyRegistryError::AssetNotAllowed)?;
        if !asset_rule.enabled {
            return Err(PolicyRegistryError::AssetNotAllowed);
        }
        if check.amount > asset_rule.max_single_transfer {
            return Err(PolicyRegistryError::AmountAboveLimit);
        }

        let recipient_allowed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Recipient(check.destination.clone()))
            .unwrap_or(false);
        if !recipient_allowed {
            return Err(PolicyRegistryError::RecipientNotAllowed);
        }

        env.events().publish(
            (symbol_short!("pol_ok"),),
            (
                check.asset,
                check.destination,
                check.amount,
                check.expected_version,
            ),
        );
        Ok(())
    }
}

fn ensure_initialized(env: &Env) -> Result<(), PolicyRegistryError> {
    if env.storage().persistent().has(&DataKey::Initialized) {
        Ok(())
    } else {
        Err(PolicyRegistryError::NotInitialized)
    }
}

fn ensure_admin(env: &Env) -> Result<Address, PolicyRegistryError> {
    ensure_initialized(env)?;
    let admin: Address = env
        .storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(PolicyRegistryError::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}

fn current_version(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::Version)
        .unwrap_or(INITIAL_VERSION)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, PolicyRegistryPocClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyRegistryPoc, ());
        let client = PolicyRegistryPocClient::new(&env, &contract_id);
        client.initialize(&Address::generate(&env));
        (env, client)
    }

    #[test]
    fn validates_allowed_payment_policy() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        client.validate_policy(&PolicyCheck {
            asset,
            destination: recipient,
            amount: 50,
            expected_version: 1,
        });
    }

    #[test]
    fn fails_closed_for_unknown_asset_or_recipient() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        let missing_asset = client.try_validate_policy(&PolicyCheck {
            asset: asset.clone(),
            destination: recipient.clone(),
            amount: 50,
            expected_version: 1,
        });
        assert_eq!(missing_asset, Err(Ok(PolicyRegistryError::AssetNotAllowed)));

        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        let missing_recipient = client.try_validate_policy(&PolicyCheck {
            asset,
            destination: recipient,
            amount: 50,
            expected_version: 1,
        });
        assert_eq!(
            missing_recipient,
            Err(Ok(PolicyRegistryError::RecipientNotAllowed))
        );
    }

    #[test]
    fn rejects_stale_policy_version_and_amount_above_limit() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 10,
            },
        );
        client.set_recipient_allowed(&recipient, &true);
        client.bump_version(&2);

        let stale = client.try_validate_policy(&PolicyCheck {
            asset: asset.clone(),
            destination: recipient.clone(),
            amount: 5,
            expected_version: 1,
        });
        assert_eq!(stale, Err(Ok(PolicyRegistryError::VersionMismatch)));

        let above_limit = client.try_validate_policy(&PolicyCheck {
            asset,
            destination: recipient,
            amount: 11,
            expected_version: 2,
        });
        assert_eq!(above_limit, Err(Ok(PolicyRegistryError::AmountAboveLimit)));
    }
}
