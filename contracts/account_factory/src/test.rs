extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, Map};
use stellar_accounts::smart_account::Signer;

use crate::{AccountFactory, AccountFactoryClient, WasmHashes};

// Real, optimized WASM for every sibling contract `deploy_account` deploys
// -- built once via `stellar contract build --optimize` and copied in as
// fixtures, so this test exercises the actual `env.deployer().deploy_v2`
// path against real bytecode, not a stand-in. Regenerate with:
//   stellar contract build --optimize --out-dir contracts/account_factory/src/wasm_fixtures
const POLICY_ENGINE_WASM: &[u8] = include_bytes!("wasm_fixtures/sta_policy_engine.wasm");
const INTENT_REGISTRY_WASM: &[u8] = include_bytes!("wasm_fixtures/sta_intent_registry.wasm");
const RECOVERY_MANAGER_WASM: &[u8] = include_bytes!("wasm_fixtures/sta_recovery_manager.wasm");
const TRANSFER_ADAPTER_WASM: &[u8] = include_bytes!("wasm_fixtures/sta_transfer_adapter.wasm");
const SPLIT_ADAPTER_WASM: &[u8] = include_bytes!("wasm_fixtures/sta_split_adapter.wasm");
const SMART_ACCOUNT_WASM: &[u8] = include_bytes!("wasm_fixtures/sta_smart_account.wasm");

fn setup() -> (Env, AccountFactoryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let factory_id = env.register(AccountFactory, ());
    let factory = AccountFactoryClient::new(&env, &factory_id);

    let wasm_hashes = WasmHashes {
        policy_engine: env.deployer().upload_contract_wasm(POLICY_ENGINE_WASM),
        intent_registry: env.deployer().upload_contract_wasm(INTENT_REGISTRY_WASM),
        recovery_manager: env.deployer().upload_contract_wasm(RECOVERY_MANAGER_WASM),
        transfer_adapter: env.deployer().upload_contract_wasm(TRANSFER_ADAPTER_WASM),
        split_adapter: env.deployer().upload_contract_wasm(SPLIT_ADAPTER_WASM),
        smart_account: env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM),
    };
    factory.initialize(&admin, &wasm_hashes);

    (env, factory, admin)
}

#[test]
fn deploy_account_wires_a_fully_usable_treasury_in_one_call() {
    let (env, factory, _admin) = setup();
    let caller = Address::generate(&env);
    let founding_signer = Signer::Delegated(Address::generate(&env));

    let deployed = factory.deploy_account(
        &caller,
        &soroban_sdk::BytesN::from_array(&env, &[7u8; 32]),
        &vec![&env, founding_signer],
        &Map::new(&env),
        &1,
        &caller,
    );

    // Every address is distinct -- no accidental salt collisions between
    // the six sub-deployments.
    let addrs = std::vec![
        deployed.smart_account.clone(),
        deployed.policy_engine.clone(),
        deployed.intent_registry.clone(),
        deployed.recovery_manager.clone(),
        deployed.transfer_adapter.clone(),
        deployed.split_adapter.clone(),
    ];
    for i in 0..addrs.len() {
        for j in (i + 1)..addrs.len() {
            assert_ne!(addrs[i], addrs[j]);
        }
    }

    let smart_account =
        sta_smart_account::SmartAccountTreasuryClient::new(&env, &deployed.smart_account);
    let status = smart_account.status();
    assert!(status.initialized);
    assert!(!status.paused);
    assert!(!status.frozen);

    // Adapters are already bound -- no propose/apply timelock dance needed
    // -- and a payment executes end to end through the freshly deployed
    // stack, on the same ledger `deploy_account` ran on.
    let policy_engine = sta_policy_engine::PolicyEngineClient::new(&env, &deployed.policy_engine);
    let token = env
        .register_stellar_asset_contract_v2(caller.clone())
        .address();
    soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&deployed.smart_account, &1_000);
    let recipient = Address::generate(&env);
    policy_engine.set_operation_allowed(&soroban_sdk::Symbol::new(&env, "transfer"), &true);
    policy_engine.set_asset_rule(
        &token,
        &sta_policy_engine::AssetRule {
            enabled: true,
            max_single_transfer: 10_000,
        },
    );
    policy_engine.set_recipient_allowed(&recipient, &true);

    smart_account.execute_transfer_payment(&token, &recipient, &250, &1, &1);
    assert_eq!(
        soroban_sdk::token::TokenClient::new(&env, &token).balance(&recipient),
        250
    );
}

#[test]
#[should_panic]
fn deploy_account_with_a_reused_salt_for_the_same_caller_fails() {
    let (env, factory, _admin) = setup();
    let caller = Address::generate(&env);
    let salt = soroban_sdk::BytesN::from_array(&env, &[9u8; 32]);
    let founding_signer = Signer::Delegated(Address::generate(&env));

    factory.deploy_account(
        &caller,
        &salt,
        &vec![&env, founding_signer.clone()],
        &Map::new(&env),
        &1,
        &caller,
    );

    // Same caller reusing the same salt: the second deployment's first
    // `deploy_v2` call collides with an address that already exists (from
    // this same caller's own prior deployment) and traps.
    factory.deploy_account(
        &caller,
        &salt,
        &vec![&env, founding_signer],
        &Map::new(&env),
        &1,
        &caller,
    );
}

/// Security review finding on an earlier revision: `sub_salt` hashed only
/// `base || tag`, with no dependence on `caller` — since every deployment
/// goes through this one factory contract, that meant salts were global to
/// the factory, not scoped per caller as the doc comment claimed. Two
/// different callers choosing the identical raw salt would have collided
/// (whoever landed second would fail), which is real squatting/griefing
/// surface, not just an accidental-collision inconvenience. Proves the fix:
/// two distinct callers using the exact same raw salt value both succeed,
/// with six distinct addresses apiece.
#[test]
fn deploy_account_with_the_same_salt_for_different_callers_does_not_collide() {
    let (env, factory, _admin) = setup();
    let salt = soroban_sdk::BytesN::from_array(&env, &[42u8; 32]);

    let caller_a = Address::generate(&env);
    let deployed_a = factory.deploy_account(
        &caller_a,
        &salt,
        &vec![&env, Signer::Delegated(Address::generate(&env))],
        &Map::new(&env),
        &1,
        &caller_a,
    );

    let caller_b = Address::generate(&env);
    let deployed_b = factory.deploy_account(
        &caller_b,
        &salt,
        &vec![&env, Signer::Delegated(Address::generate(&env))],
        &Map::new(&env),
        &1,
        &caller_b,
    );

    assert_ne!(deployed_a.smart_account, deployed_b.smart_account);
    assert_ne!(deployed_a.policy_engine, deployed_b.policy_engine);
    assert_ne!(deployed_a.intent_registry, deployed_b.intent_registry);
    assert_ne!(deployed_a.recovery_manager, deployed_b.recovery_manager);
    assert_ne!(deployed_a.transfer_adapter, deployed_b.transfer_adapter);
    assert_ne!(deployed_a.split_adapter, deployed_b.split_adapter);
}

#[test]
#[should_panic]
fn deploy_account_before_the_factory_is_initialized_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let factory_id = env.register(AccountFactory, ());
    let factory = AccountFactoryClient::new(&env, &factory_id);
    let caller = Address::generate(&env);
    let founding_signer = Signer::Delegated(Address::generate(&env));

    factory.deploy_account(
        &caller,
        &soroban_sdk::BytesN::from_array(&env, &[1u8; 32]),
        &vec![&env, founding_signer],
        &Map::new(&env),
        &1,
        &caller,
    );
}

#[test]
#[should_panic]
fn deploy_account_without_caller_authorization_fails() {
    let (env, factory, _admin) = setup();
    let caller = Address::generate(&env);
    let founding_signer = Signer::Delegated(Address::generate(&env));

    env.set_auths(&[]);
    factory.deploy_account(
        &caller,
        &soroban_sdk::BytesN::from_array(&env, &[3u8; 32]),
        &vec![&env, founding_signer],
        &Map::new(&env),
        &1,
        &caller,
    );
}

#[test]
fn set_wasm_hashes_updates_the_stored_value() {
    let (env, factory, _admin) = setup();
    let mut updated = factory.get_wasm_hashes();
    // A cheap way to get a genuinely different `BytesN<32>` for a "the
    // value actually changed" assertion, without needing a second real
    // WASM fixture just for this one field-level check.
    let mut arr = updated.policy_engine.to_array();
    arr[0] = arr[0].wrapping_add(1);
    updated.policy_engine = soroban_sdk::BytesN::from_array(&env, &arr);

    factory.set_wasm_hashes(&updated);
    assert_eq!(factory.get_wasm_hashes(), updated);
}

#[test]
#[should_panic]
fn set_wasm_hashes_without_admin_authorization_fails() {
    let (env, factory, _admin) = setup();
    let original = factory.get_wasm_hashes();
    env.set_auths(&[]);
    factory.set_wasm_hashes(&original);
}

#[test]
fn extend_instance_ttl_is_permissionless() {
    let (env, factory, _admin) = setup();
    env.set_auths(&[]);
    factory.extend_instance_ttl();
}

#[test]
fn contract_name_reports_expected_symbol() {
    let (_env, factory, _admin) = setup();
    assert_eq!(
        factory.contract_name(),
        soroban_sdk::symbol_short!("sta_fctr")
    );
}
