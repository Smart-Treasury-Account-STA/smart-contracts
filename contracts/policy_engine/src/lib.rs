#![no_std]

//! PolicyEngine: treasury-directionality risk controls.
//!
//! This is deliberately independent of signer authentication. By the time a
//! call reaches `validate_policy`, `SmartAccount` has already established
//! *who* is authorized to act (via its OZ-composed signer/context-rule
//! model, see `contracts/smart_account`). PolicyEngine answers a different
//! question: even a fully authorized signer set cannot move funds to an
//! asset, recipient, or operation that hasn't been explicitly allowed, above
//! an amount cap, or under a stale policy version. This is the module that
//! turns "N signers approved" into "N signers approved *and* the treasury's
//! own risk rules allow it" — see Architecture Decision 4 in
//! `docs/TECHNICAL_ARCHITECTURE.md` for why this is not reducible to signer
//! math.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, symbol_short, Address, Env,
    Symbol, Vec,
};

const INITIAL_VERSION: u32 = 1;

/// Audit finding (`docs/TECHNICAL_ARCHITECTURE.md` §16, a documented
/// "Production requirement"): Soroban persistent entries that aren't
/// touched for long enough are archived and become inaccessible until
/// explicitly restored. Every critical entry here is extended on write,
/// and asset/recipient/operation rules are also extended whenever they're
/// read by `validate_policy` (the hot path), so actively enforced rules
/// stay alive without needing separate maintenance calls. `extend_ttl`
/// below is the explicit out-of-band maintenance entrypoint for entries
/// not otherwise touched recently.
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

#[contract]
pub struct PolicyEngine;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRule {
    pub enabled: bool,
    pub max_single_transfer: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyCheck {
    pub operation: Symbol,
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
    pub expected_version: u32,
}

#[contractevent(topics = ["init"])]
pub struct Initialized {
    #[topic]
    pub admin: Address,
}

#[contractevent(topics = ["asset"])]
pub struct AssetRuleUpdated {
    #[topic]
    pub asset: Address,
}

#[contractevent(topics = ["rcpt"])]
pub struct RecipientAllowedUpdated {
    #[topic]
    pub recipient: Address,
    pub allowed: bool,
}

#[contractevent(topics = ["op"])]
pub struct OperationAllowedUpdated {
    #[topic]
    pub operation: Symbol,
    pub allowed: bool,
}

#[contractevent(topics = ["policy"])]
pub struct PolicyVersionBumped {
    pub next_version: u32,
}

#[contractevent(topics = ["pol_ok"])]
pub struct PolicyValidated {
    #[topic]
    pub operation: Symbol,
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
    Operation(Symbol),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PolicyEngineError {
    AlreadyInitialized = 2000,
    NotInitialized = 2001,
    InvalidAmount = 2002,
    AssetNotAllowed = 2003,
    RecipientNotAllowed = 2004,
    AmountAboveLimit = 2005,
    VersionMismatch = 2006,
    InvalidVersion = 2007,
    OperationNotAllowed = 2008,
}

#[contractimpl]
impl PolicyEngine {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_pol")
    }

    pub fn initialize(env: Env, admin: Address) -> Result<(), PolicyEngineError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(PolicyEngineError::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Version, &INITIAL_VERSION);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        Initialized {
            admin: admin.clone(),
        }
        .publish(&env);
        Ok(())
    }

    /// Permissionless TTL maintenance. `Initialized`/`Admin`/`Version` live
    /// in instance storage (one shared TTL, refreshed automatically by
    /// `ensure_initialized` on every call that reaches it — see that
    /// function) and are covered by the single `extend_ttl` call below;
    /// `assets`/`recipients`/`operations` are named, already-existing
    /// per-entity persistent entries, extended individually the same way
    /// they always were. Extending TTL does not create authority or alter
    /// execution semantics (`docs/TECHNICAL_ARCHITECTURE.md` §16.1), so
    /// this is intentionally open to any caller — an off-chain monitor can
    /// refresh the entries it knows are still active without needing admin
    /// keys.
    pub fn extend_ttl(
        env: Env,
        assets: Vec<Address>,
        recipients: Vec<Address>,
        operations: Vec<Symbol>,
    ) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        for asset in assets.iter() {
            bump_ttl(&env, &DataKey::Asset(asset));
        }
        for recipient in recipients.iter() {
            bump_ttl(&env, &DataKey::Recipient(recipient));
        }
        for operation in operations.iter() {
            bump_ttl(&env, &DataKey::Operation(operation));
        }
    }

    pub fn version(env: Env) -> Result<u32, PolicyEngineError> {
        ensure_initialized(&env)?;
        Ok(current_version(&env))
    }

    pub fn set_asset_rule(
        env: Env,
        asset: Address,
        rule: AssetRule,
    ) -> Result<(), PolicyEngineError> {
        ensure_admin(&env)?;
        if rule.enabled && rule.max_single_transfer <= 0 {
            return Err(PolicyEngineError::InvalidAmount);
        }

        let key = DataKey::Asset(asset.clone());
        env.storage().persistent().set(&key, &rule);
        bump_ttl(&env, &key);
        AssetRuleUpdated {
            asset: asset.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_recipient_allowed(
        env: Env,
        recipient: Address,
        allowed: bool,
    ) -> Result<(), PolicyEngineError> {
        ensure_admin(&env)?;
        let key = DataKey::Recipient(recipient.clone());
        env.storage().persistent().set(&key, &allowed);
        bump_ttl(&env, &key);
        RecipientAllowedUpdated {
            recipient: recipient.clone(),
            allowed,
        }
        .publish(&env);
        Ok(())
    }

    /// Enables or disables an adapter-level operation (e.g. `transfer`,
    /// `split`). An operation not explicitly enabled fails closed, so
    /// deploying a new adapter never silently grants it spend authority.
    pub fn set_operation_allowed(
        env: Env,
        operation: Symbol,
        allowed: bool,
    ) -> Result<(), PolicyEngineError> {
        ensure_admin(&env)?;
        let key = DataKey::Operation(operation.clone());
        env.storage().persistent().set(&key, &allowed);
        bump_ttl(&env, &key);
        OperationAllowedUpdated {
            operation: operation.clone(),
            allowed,
        }
        .publish(&env);
        Ok(())
    }

    pub fn bump_version(env: Env, next_version: u32) -> Result<(), PolicyEngineError> {
        ensure_admin(&env)?;
        if next_version <= current_version(&env) {
            return Err(PolicyEngineError::InvalidVersion);
        }

        env.storage()
            .instance()
            .set(&DataKey::Version, &next_version);
        PolicyVersionBumped { next_version }.publish(&env);
        Ok(())
    }

    pub fn validate_policy(env: Env, check: PolicyCheck) -> Result<(), PolicyEngineError> {
        ensure_initialized(&env)?;
        if check.expected_version != current_version(&env) {
            return Err(PolicyEngineError::VersionMismatch);
        }
        if check.amount <= 0 {
            return Err(PolicyEngineError::InvalidAmount);
        }

        let operation_key = DataKey::Operation(check.operation.clone());
        let operation_allowed: bool = env
            .storage()
            .persistent()
            .get(&operation_key)
            .unwrap_or(false);
        if !operation_allowed {
            return Err(PolicyEngineError::OperationNotAllowed);
        }
        bump_ttl(&env, &operation_key);

        let asset_key = DataKey::Asset(check.asset.clone());
        let asset_rule: AssetRule = env
            .storage()
            .persistent()
            .get(&asset_key)
            .ok_or(PolicyEngineError::AssetNotAllowed)?;
        if !asset_rule.enabled {
            return Err(PolicyEngineError::AssetNotAllowed);
        }
        if check.amount > asset_rule.max_single_transfer {
            return Err(PolicyEngineError::AmountAboveLimit);
        }
        bump_ttl(&env, &asset_key);

        let recipient_key = DataKey::Recipient(check.destination.clone());
        let recipient_allowed: bool = env
            .storage()
            .persistent()
            .get(&recipient_key)
            .unwrap_or(false);
        if !recipient_allowed {
            return Err(PolicyEngineError::RecipientNotAllowed);
        }
        bump_ttl(&env, &recipient_key);

        PolicyValidated {
            operation: check.operation,
            asset: check.asset,
            destination: check.destination,
            amount: check.amount,
            expected_version: check.expected_version,
        }
        .publish(&env);
        Ok(())
    }
}

fn bump_ttl(env: &Env, key: &DataKey) {
    if env.storage().persistent().has(key) {
        env.storage()
            .persistent()
            .extend_ttl(key, TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    }
}

fn ensure_initialized(env: &Env) -> Result<(), PolicyEngineError> {
    if env.storage().instance().has(&DataKey::Initialized) {
        // Instance storage holds this contract's own singleton config
        // (`Initialized`/`Admin`/`Version`) and shares a single TTL across
        // all of it; refreshing here on every call that reaches this far
        // covers every entrypoint uniformly, matching `smart_account`'s own
        // `ensure_initialized`.
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        Ok(())
    } else {
        Err(PolicyEngineError::NotInitialized)
    }
}

fn ensure_admin(env: &Env) -> Result<Address, PolicyEngineError> {
    ensure_initialized(env)?;
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(PolicyEngineError::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}

fn current_version(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::Version)
        .unwrap_or(INITIAL_VERSION)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::{
        storage::Instance as _, storage::Persistent as _, Address as _, Ledger,
    };
    use soroban_sdk::vec;

    fn setup() -> (Env, PolicyEngineClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyEngine, ());
        let client = PolicyEngineClient::new(&env, &contract_id);
        client.initialize(&Address::generate(&env));
        (env, client)
    }

    fn allow_transfer_op(client: &PolicyEngineClient) {
        client.set_operation_allowed(&symbol_short!("transfer"), &true);
    }

    #[test]
    fn validates_allowed_payment_policy() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);
        allow_transfer_op(&client);

        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        client.validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset,
            destination: recipient,
            amount: 50,
            expected_version: 1,
        });
    }

    #[test]
    fn fails_closed_for_unknown_operation_asset_or_recipient() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);

        let missing_operation = client.try_validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset: asset.clone(),
            destination: recipient.clone(),
            amount: 50,
            expected_version: 1,
        });
        assert_eq!(
            missing_operation,
            Err(Ok(PolicyEngineError::OperationNotAllowed))
        );

        allow_transfer_op(&client);

        let missing_asset = client.try_validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset: asset.clone(),
            destination: recipient.clone(),
            amount: 50,
            expected_version: 1,
        });
        assert_eq!(missing_asset, Err(Ok(PolicyEngineError::AssetNotAllowed)));

        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        let missing_recipient = client.try_validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset,
            destination: recipient,
            amount: 50,
            expected_version: 1,
        });
        assert_eq!(
            missing_recipient,
            Err(Ok(PolicyEngineError::RecipientNotAllowed))
        );
    }

    #[test]
    fn rejects_stale_policy_version_and_amount_above_limit() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);
        allow_transfer_op(&client);

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
            operation: symbol_short!("transfer"),
            asset: asset.clone(),
            destination: recipient.clone(),
            amount: 5,
            expected_version: 1,
        });
        assert_eq!(stale, Err(Ok(PolicyEngineError::VersionMismatch)));

        let above_limit = client.try_validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset,
            destination: recipient,
            amount: 11,
            expected_version: 2,
        });
        assert_eq!(above_limit, Err(Ok(PolicyEngineError::AmountAboveLimit)));
    }

    /// Edge case: an operation disabled *after* being enabled must fail
    /// closed immediately, even if asset/recipient/version are still valid —
    /// operation gating is checked independently, not cached from an earlier
    /// approval.
    #[test]
    fn disabling_operation_after_the_fact_blocks_further_validation() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);
        allow_transfer_op(&client);
        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        client.set_operation_allowed(&symbol_short!("transfer"), &false);

        let blocked = client.try_validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset,
            destination: recipient,
            amount: 10,
            expected_version: 1,
        });
        assert_eq!(blocked, Err(Ok(PolicyEngineError::OperationNotAllowed)));
    }

    /// Audit finding (`docs/TECHNICAL_ARCHITECTURE.md` §16, "Production
    /// requirement"): persistent entries need their TTL extended or they
    /// archive and become inaccessible. Proves `set_asset_rule` and a
    /// `validate_policy` read both actually extend the stored asset rule's
    /// TTL, not just that the extension code compiles.
    #[test]
    fn writes_and_reads_extend_persistent_entry_ttl() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyEngine, ());
        let client = PolicyEngineClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

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
        allow_transfer_op(&client);

        let ttl_after_write = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Asset(asset.clone()))
        });
        assert!(ttl_after_write >= TTL_THRESHOLD_LEDGERS);

        // Let the TTL decay a long way, then confirm a validate_policy read
        // refreshes it back up rather than leaving it to keep decaying.
        env.ledger()
            .with_mut(|l| l.sequence_number += TTL_THRESHOLD_LEDGERS / 2);
        client.validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset: asset.clone(),
            destination: recipient,
            amount: 10,
            expected_version: 1,
        });
        let ttl_after_read = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Asset(asset.clone()))
        });
        assert!(ttl_after_read >= TTL_THRESHOLD_LEDGERS);
    }

    /// Best-practice review finding: `Initialized`/`Admin`/`Version` used to
    /// be separate persistent entries, each bumped individually, despite
    /// never being read or written independently of each other — the
    /// per-entity data (`Asset`/`Recipient`/`Operation`) genuinely benefits
    /// from its own TTL, but this singleton config doesn't. Proves the
    /// migration to instance storage actually happened (not just that the
    /// code compiles): the keys are absent from persistent storage, and a
    /// single admin action (`bump_version`) is enough to refresh the whole
    /// instance's TTL with no separate bump calls, mirroring
    /// `smart_account::ensure_initialized`'s behavior exactly.
    #[test]
    fn singleton_config_lives_in_instance_storage_with_one_shared_ttl() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyEngine, ());
        let client = PolicyEngineClient::new(&env, &contract_id);
        client.initialize(&Address::generate(&env));

        env.as_contract(&contract_id, || {
            assert!(!env.storage().persistent().has(&DataKey::Initialized));
            assert!(!env.storage().persistent().has(&DataKey::Admin));
            assert!(!env.storage().persistent().has(&DataKey::Version));
            assert!(env.storage().instance().has(&DataKey::Initialized));
            assert!(env.storage().instance().has(&DataKey::Admin));
            assert!(env.storage().instance().has(&DataKey::Version));
        });

        env.ledger()
            .with_mut(|l| l.sequence_number += TTL_THRESHOLD_LEDGERS / 2);
        client.bump_version(&2);
        let instance_ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
        assert!(instance_ttl >= TTL_THRESHOLD_LEDGERS);
    }

    /// The permissionless maintenance entrypoint extends TTL for the exact
    /// entries named, and does not require any authorization — matching
    /// §16.1's "extending TTL does not create authority" rule.
    #[test]
    fn permissionless_extend_ttl_refreshes_named_entries() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyEngine, ());
        let client = PolicyEngineClient::new(&env, &contract_id);
        client.initialize(&Address::generate(&env));

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
        allow_transfer_op(&client);

        env.ledger()
            .with_mut(|l| l.sequence_number += TTL_THRESHOLD_LEDGERS / 2);
        env.set_auths(&[]); // no authorization at all — still succeeds
        client.extend_ttl(
            &vec![&env, asset.clone()],
            &vec![&env, recipient],
            &vec![&env, symbol_short!("transfer")],
        );

        let ttl = env.as_contract(&contract_id, || {
            env.storage().persistent().get_ttl(&DataKey::Asset(asset))
        });
        assert!(ttl >= TTL_THRESHOLD_LEDGERS);
    }

    #[test]
    fn contract_name_reports_expected_symbol() {
        assert_eq!(PolicyEngine::contract_name(), symbol_short!("sta_pol"));
    }

    #[test]
    fn calling_before_initialize_rejects_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyEngine, ());
        let client = PolicyEngineClient::new(&env, &contract_id);

        let err = client.try_version();
        assert_eq!(err, Err(Ok(PolicyEngineError::NotInitialized)));
    }

    #[test]
    fn set_asset_rule_rejects_enabled_rule_with_non_positive_cap() {
        let (_env, client) = setup();
        let asset = Address::generate(&_env);

        let err = client.try_set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 0,
            },
        );
        assert_eq!(err, Err(Ok(PolicyEngineError::InvalidAmount)));
    }

    #[test]
    fn bump_version_rejects_non_increasing_version() {
        let (_env, client) = setup();

        let same = client.try_bump_version(&1);
        assert_eq!(same, Err(Ok(PolicyEngineError::InvalidVersion)));

        let lower = client.try_bump_version(&0);
        assert_eq!(lower, Err(Ok(PolicyEngineError::InvalidVersion)));
    }

    #[test]
    fn validate_policy_rejects_non_positive_amount() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);
        allow_transfer_op(&client);
        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: true,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        let err = client.try_validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset,
            destination: recipient,
            amount: 0,
            expected_version: 1,
        });
        assert_eq!(err, Err(Ok(PolicyEngineError::InvalidAmount)));
    }

    /// Distinct from "no asset rule at all": here a rule exists but was
    /// explicitly disabled, which must fail closed the same way.
    #[test]
    fn validate_policy_rejects_explicitly_disabled_asset() {
        let (env, client) = setup();
        let asset = Address::generate(&env);
        let recipient = Address::generate(&env);
        allow_transfer_op(&client);
        client.set_asset_rule(
            &asset,
            &AssetRule {
                enabled: false,
                max_single_transfer: 100,
            },
        );
        client.set_recipient_allowed(&recipient, &true);

        let err = client.try_validate_policy(&PolicyCheck {
            operation: symbol_short!("transfer"),
            asset,
            destination: recipient,
            amount: 10,
            expected_version: 1,
        });
        assert_eq!(err, Err(Ok(PolicyEngineError::AssetNotAllowed)));
    }
}
