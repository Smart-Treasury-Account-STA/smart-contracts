//! Integration tests wiring `smart_account` to real instances of every
//! subordinate contract (`policy_engine`, `intent_registry`,
//! `transfer_adapter`, `split_adapter`, `recovery_manager`) and a real
//! Stellar Asset Contract — this exercises the actual cross-contract
//! dispatch chain, not mocks of it.
//!
//! ## What these tests do and do not prove about authorization
//!
//! These tests run under `env.mock_all_auths()`. Soroban's recording-auth
//! mode satisfies every `require_auth()` / `require_auth_for_args()` call
//! automatically *without invoking the target address's `__check_auth`* —
//! so `execute_transfer_payment`'s `env.current_contract_address().require_auth()`
//! never actually runs `do_check_auth` here. That is a deliberate, standard
//! Soroban testing boundary (the OZ `stellar-accounts` crate's own ~2000
//! lines of `smart_account` tests exercise context-rule/signer/policy logic
//! directly via `storage::*` calls under `e.as_contract(..)`, not through
//! full signed authorization entries either — see
//! `stellar-accounts-0.7.2/src/smart_account/test/`). What these tests
//! prove is that, given an authorized caller, the treasury's own logic
//! (policy checks, nonce replay, pause/freeze, adapter dispatch, recovery
//! pull) is wired correctly end to end against real deployed contracts.
//!
//! The authentication layer itself is covered two other ways in this
//! workspace: `contracts/webauthn_verifier` proves the actual
//! secp256r1/WebAuthn and Ed25519 signature verification with real
//! cryptographic fixtures (not mocked), and one test below
//! (`execute_transfer_payment_without_any_authorization_fails`) uses
//! `set_auths(&[])` — genuinely no mocked or real authorization — to prove
//! the `require_auth()` gate is not accidentally a no-op.

extern crate std;

use soroban_sdk::{
    testutils::{
        storage::Instance as _, storage::Persistent as _, Address as _, Ledger, MockAuth,
        MockAuthInvoke,
    },
    token::StellarAssetClient,
    token::TokenClient,
    vec, Address, Env, IntoVal, Map, String, Symbol,
};
use stellar_accounts::smart_account::{ContextRuleType, Signer};

use crate::{
    AccountStatus, InitConfig, ScheduledIntentArgs, SmartAccountTreasury,
    SmartAccountTreasuryClient, SmartAccountTreasuryError,
};

fn addr(e: &Env) -> Address {
    Address::generate(e)
}

struct Harness {
    env: Env,
    smart_account: SmartAccountTreasuryClient<'static>,
    policy_engine: sta_policy_engine::PolicyEngineClient<'static>,
    intent_registry: sta_intent_registry::IntentRegistryClient<'static>,
    recovery_manager: sta_recovery_manager::RecoveryManagerClient<'static>,
    token: Address,
    owner: Address,
    relayer: Address,
}

// The workspace already has `sta-policy-engine` / `sta-intent-registry` /
// `sta-transfer-adapter` / `sta-split-adapter` / `sta-recovery-manager` as
// sibling crates (dev-dependencies), letting this test register *real*
// instances of each rather than hand-rolled mocks.

fn setup() -> Harness {
    let env = Env::default();
    env.mock_all_auths();

    let owner = addr(&env);
    let relayer = addr(&env);

    let policy_engine_id = env.register(sta_policy_engine::PolicyEngine, ());
    let policy_engine = sta_policy_engine::PolicyEngineClient::new(&env, &policy_engine_id);
    policy_engine.initialize(&owner);

    // Deployed but deliberately left uninitialized here: `smart_account`'s
    // own `initialize` bootstraps it directly, passing itself as `admin` via
    // the invoker-shortcut (see that function's doc comment) — no separate
    // hand-built authorization needed.
    let intent_registry_id = env.register(sta_intent_registry::IntentRegistry, ());
    let intent_registry = sta_intent_registry::IntentRegistryClient::new(&env, &intent_registry_id);

    let recovery_manager_id = env.register(sta_recovery_manager::RecoveryManager, ());
    let recovery_manager =
        sta_recovery_manager::RecoveryManagerClient::new(&env, &recovery_manager_id);
    recovery_manager.initialize(&owner, &1);

    let smart_account_id = env.register(SmartAccountTreasury, ());
    let smart_account = SmartAccountTreasuryClient::new(&env, &smart_account_id);

    let transfer_adapter_id = env.register(sta_transfer_adapter::TransferAdapter, ());
    let transfer_adapter =
        sta_transfer_adapter::TransferAdapterClient::new(&env, &transfer_adapter_id);
    transfer_adapter.initialize(&owner, &smart_account_id);

    let split_adapter_id = env.register(sta_split_adapter::SplitAdapter, ());
    let split_adapter = sta_split_adapter::SplitAdapterClient::new(&env, &split_adapter_id);
    split_adapter.initialize(&owner, &smart_account_id);

    let founding_signer = Signer::Delegated(addr(&env));
    smart_account.initialize(
        &owner,
        &vec![&env, founding_signer],
        &Map::new(&env),
        &InitConfig {
            policy_engine: policy_engine_id.clone(),
            intent_registry: intent_registry_id.clone(),
            recovery_manager: recovery_manager_id.clone(),
            initial_adapters: Map::new(&env),
            initial_executor: relayer.clone(),
        },
    );
    // Adapter changes are timelocked (independent security review finding;
    // see `docs/SMART_CONTRACT_AUDIT_REPORT.md`): propose, advance past the
    // delay, apply, then restore the ledger to its starting point so this
    // setup step doesn't leak into every test's own ledger expectations.
    let starting_ledger = env.ledger().sequence();
    smart_account.propose_adapter_change(&Symbol::new(&env, "transfer"), &transfer_adapter_id);
    smart_account.propose_adapter_change(&Symbol::new(&env, "split"), &split_adapter_id);
    env.ledger()
        .with_mut(|l| l.sequence_number = starting_ledger + 17280);
    smart_account.apply_adapter_change(&Symbol::new(&env, "transfer"));
    smart_account.apply_adapter_change(&Symbol::new(&env, "split"));
    env.ledger()
        .with_mut(|l| l.sequence_number = starting_ledger);

    let token = env
        .register_stellar_asset_contract_v2(owner.clone())
        .address();
    StellarAssetClient::new(&env, &token).mint(&smart_account_id, &1_000_000);

    Harness {
        env,
        smart_account,
        policy_engine,
        intent_registry,
        recovery_manager,
        token,
        owner,
        relayer,
    }
}

fn allow_payment(h: &Harness, recipient: &Address) {
    h.policy_engine
        .set_operation_allowed(&Symbol::new(&h.env, "transfer"), &true);
    h.policy_engine
        .set_operation_allowed(&Symbol::new(&h.env, "split"), &true);
    h.policy_engine.set_asset_rule(
        &h.token,
        &sta_policy_engine::AssetRule {
            enabled: true,
            max_single_transfer: 10_000,
        },
    );
    h.policy_engine.set_recipient_allowed(recipient, &true);
}

#[test]
fn initializes_and_reports_status() {
    let h = setup();
    let status: AccountStatus = h.smart_account.status();
    assert!(status.initialized);
    assert!(!status.paused);
    assert!(!status.frozen);
}

/// Proves the invoker-shortcut claim in `initialize`'s doc comment for
/// real, rather than taking it on faith: `env.mock_all_auths()` would
/// satisfy *any* `require_auth()` regardless of whether the shortcut
/// actually applies, so it cannot tell the two apart. This test instead
/// authorizes *only* `owner` for the exact `initialize` call, with an empty
/// `sub_invokes` list — no separate auth entry for `intent_registry`'s own
/// `admin.require_auth()` at all. If the shortcut did not hold, this call
/// would panic with a missing-authorization error instead of succeeding.
#[test]
fn initialize_bootstraps_intent_registry_with_no_separate_authorization() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = addr(&env);

    let policy_engine_id = env.register(sta_policy_engine::PolicyEngine, ());
    sta_policy_engine::PolicyEngineClient::new(&env, &policy_engine_id).initialize(&owner);

    let intent_registry_id = env.register(sta_intent_registry::IntentRegistry, ());

    let recovery_manager_id = env.register(sta_recovery_manager::RecoveryManager, ());
    sta_recovery_manager::RecoveryManagerClient::new(&env, &recovery_manager_id)
        .initialize(&owner, &1);

    let smart_account_id = env.register(SmartAccountTreasury, ());
    let smart_account = SmartAccountTreasuryClient::new(&env, &smart_account_id);
    let initial_signers = vec![&env, Signer::Delegated(addr(&env))];
    let initial_policies = Map::new(&env);

    let config = InitConfig {
        policy_engine: policy_engine_id.clone(),
        intent_registry: intent_registry_id.clone(),
        recovery_manager: recovery_manager_id.clone(),
        initial_adapters: Map::new(&env),
        initial_executor: owner.clone(),
    };

    smart_account
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &smart_account_id,
                fn_name: "initialize",
                args: (
                    owner.clone(),
                    initial_signers.clone(),
                    initial_policies.clone(),
                    config.clone(),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(&owner, &initial_signers, &initial_policies, &config);

    let status = smart_account.status();
    assert!(status.initialized);

    // Restore blanket auth mocking (the call above deliberately used a
    // precise, narrower list) so the check below can only fail for
    // `AlreadyInitialized`, not for a missing auth entry of its own.
    env.mock_all_auths();
    let intent_registry = sta_intent_registry::IntentRegistryClient::new(&env, &intent_registry_id);
    // A second `initialize` call is only reachable if the first one already
    // landed — proves `intent_registry` is genuinely initialized, not just
    // that `smart_account.initialize` itself returned Ok.
    let already_initialized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        intent_registry.initialize(&owner);
    }))
    .is_err();
    assert!(already_initialized);
}

/// Proves `initial_adapters` genuinely binds adapters with zero delay, in
/// contrast to `propose_adapter_change`/`apply_adapter_change`'s ~1 day
/// timelock for every later change: a payment executes successfully on the
/// very same ledger `initialize` ran on, no `apply_adapter_change` call or
/// ledger advance anywhere in this test.
#[test]
fn initial_adapters_are_usable_immediately_with_no_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = addr(&env);

    let policy_engine_id = env.register(sta_policy_engine::PolicyEngine, ());
    let policy_engine = sta_policy_engine::PolicyEngineClient::new(&env, &policy_engine_id);
    policy_engine.initialize(&owner);

    let intent_registry_id = env.register(sta_intent_registry::IntentRegistry, ());

    let recovery_manager_id = env.register(sta_recovery_manager::RecoveryManager, ());
    sta_recovery_manager::RecoveryManagerClient::new(&env, &recovery_manager_id)
        .initialize(&owner, &1);

    let smart_account_id = env.register(SmartAccountTreasury, ());
    let smart_account = SmartAccountTreasuryClient::new(&env, &smart_account_id);

    let transfer_adapter_id = env.register(sta_transfer_adapter::TransferAdapter, ());
    sta_transfer_adapter::TransferAdapterClient::new(&env, &transfer_adapter_id)
        .initialize(&owner, &smart_account_id);

    let founding_signer = Signer::Delegated(addr(&env));
    let mut initial_adapters = Map::new(&env);
    initial_adapters.set(Symbol::new(&env, "transfer"), transfer_adapter_id);
    smart_account.initialize(
        &owner,
        &vec![&env, founding_signer],
        &Map::new(&env),
        &InitConfig {
            policy_engine: policy_engine_id,
            intent_registry: intent_registry_id,
            recovery_manager: recovery_manager_id,
            initial_adapters,
            initial_executor: owner.clone(),
        },
    );

    let token = env
        .register_stellar_asset_contract_v2(owner.clone())
        .address();
    StellarAssetClient::new(&env, &token).mint(&smart_account_id, &1_000);

    let recipient = addr(&env);
    policy_engine.set_operation_allowed(&Symbol::new(&env, "transfer"), &true);
    policy_engine.set_asset_rule(
        &token,
        &sta_policy_engine::AssetRule {
            enabled: true,
            max_single_transfer: 10_000,
        },
    );
    policy_engine.set_recipient_allowed(&recipient, &true);

    smart_account.execute_transfer_payment(&token, &recipient, &400, &1, &1);

    assert_eq!(TokenClient::new(&env, &token).balance(&recipient), 400);
}

#[test]
fn executes_transfer_payment_end_to_end_through_policy_and_adapter() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &500, &1, &1);

    let token_client = TokenClient::new(&h.env, &h.token);
    assert_eq!(token_client.balance(&recipient), 500);
    assert!(h.smart_account.is_nonce_used(&1));
}

#[test]
#[should_panic(expected = "Error(Contract, #8005)")]
fn rejects_replayed_nonce() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &7, &1);
    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &7, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #2003)")]
fn payment_fails_closed_when_asset_not_allowed_by_policy_engine() {
    let h = setup();
    let recipient = addr(&h.env);
    // Deliberately skip allow_payment's asset rule.
    h.policy_engine
        .set_operation_allowed(&Symbol::new(&h.env, "transfer"), &true);
    h.policy_engine.set_recipient_allowed(&recipient, &true);

    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &1, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #8002)")]
fn paused_treasury_rejects_payment() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    h.smart_account.pause(&h.owner);
    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &1, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #8003)")]
fn frozen_treasury_rejects_payment_and_has_no_direct_unfreeze() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    h.smart_account.freeze();
    // No `unfreeze()` entrypoint exists on this contract at all — that is
    // the point (see module docs on `freeze`). The only way out is a
    // completed recovery, exercised separately below.
    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &1, &1);
}

#[test]
fn split_payment_fans_out_and_policy_checks_each_recipient_independently() {
    let h = setup();
    let alice = addr(&h.env);
    let bob = addr(&h.env);
    allow_payment(&h, &alice);
    h.policy_engine.set_recipient_allowed(&bob, &true);

    h.smart_account.execute_split_payment(
        &h.token,
        &vec![&h.env, alice.clone(), bob.clone()],
        &vec![&h.env, 300, 200],
        &1,
        &1,
    );

    let token_client = TokenClient::new(&h.env, &h.token);
    assert_eq!(token_client.balance(&alice), 300);
    assert_eq!(token_client.balance(&bob), 200);
}

#[test]
#[should_panic(expected = "Error(Contract, #2004)")]
fn split_payment_fails_closed_if_any_single_recipient_is_not_allowed() {
    let h = setup();
    let alice = addr(&h.env);
    let bob = addr(&h.env); // never allow-listed
    allow_payment(&h, &alice);

    h.smart_account.execute_split_payment(
        &h.token,
        &vec![&h.env, alice, bob],
        &vec![&h.env, 300, 200],
        &1,
        &1,
    );
}

#[test]
fn scheduled_payment_creation_and_relayer_triggered_execution() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    let intent_id = soroban_sdk::BytesN::from_array(&h.env, &[9u8; 32]);
    h.env.ledger().with_mut(|l| l.sequence_number = 10);
    h.smart_account
        .create_scheduled_payment(&ScheduledIntentArgs {
            intent_id: intent_id.clone(),
            asset: h.token.clone(),
            destination: recipient.clone(),
            amount: 250,
            start_ledger: 10,
            end_ledger: 20,
            interval_ledgers: 0,
            max_executions: 1,
            execution_count: 0,
            // Overwritten by `create_scheduled_payment` with policy_engine's
            // actual current version regardless of what's supplied here.
            policy_version: 999,
            // Overwritten by `create_scheduled_payment` with the currently
            // configured transfer_adapter regardless of what's supplied here.
            adapter: addr(&h.env),
            cancelled: false,
        });
    assert_eq!(h.intent_registry.get_intent(&intent_id).policy_version, 1);

    // `mark_child_executed`'s `executor.require_auth()` fires as a nested
    // call (relayer -> smart_account -> intent_registry), not at the
    // transaction root, so the strict `mock_all_auths()` recording mode
    // (which only auto-satisfies root-tied authorizations) rejects it. The
    // non-root variant mirrors what a real relayer transaction does: the
    // relayer signs the top-level call, and that authorization is expected
    // to cover this specific nested `require_auth()`.
    h.env.mock_all_auths_allowing_non_root_auth();
    // Deliberately does NOT pass asset/destination/amount/policy_version —
    // see the audit note on `execute_scheduled_payment` for why those are
    // read back from `intent_registry` instead of trusted from the caller.
    h.smart_account.execute_scheduled_payment(&intent_id, &1);

    let token_client = TokenClient::new(&h.env, &h.token);
    assert_eq!(token_client.balance(&recipient), 250);
    assert!(h.intent_registry.is_child_executed(&intent_id, &1));
    let _ = h.relayer; // relayer address is the configured intent_registry executor
}

/// Security review finding: `IntentRegistry::initialize` defaults
/// `Executor` to whatever `admin` it's given, which is this treasury's own
/// address — and `execute_scheduled_payment`'s nested call into
/// `intent_registry.mark_child_executed` would satisfy that default
/// `Executor`'s `require_auth()` via the invoker-shortcut regardless of who
/// called `execute_scheduled_payment`, silently making it callable by
/// anyone. `setup()`'s harness passes `relayer` as `initial_executor`
/// specifically to avoid that default; this test proves the gate is real
/// with a genuinely unauthorized caller — no mocked or real authorization
/// for `relayer` at all.
#[test]
#[should_panic]
fn execute_scheduled_payment_without_executor_authorization_fails() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    let intent_id = soroban_sdk::BytesN::from_array(&h.env, &[11u8; 32]);
    h.env.ledger().with_mut(|l| l.sequence_number = 10);
    h.smart_account
        .create_scheduled_payment(&ScheduledIntentArgs {
            intent_id: intent_id.clone(),
            asset: h.token.clone(),
            destination: recipient,
            amount: 250,
            start_ledger: 10,
            end_ledger: 20,
            interval_ledgers: 0,
            max_executions: 1,
            execution_count: 0,
            policy_version: 999,
            adapter: addr(&h.env),
            cancelled: false,
        });

    h.env.set_auths(&[]);
    h.smart_account.execute_scheduled_payment(&intent_id, &1);
}

/// Audit finding, now fixed: `execute_scheduled_payment` must ignore any
/// caller intuition about "what the intent probably says" and use only the
/// canonical stored intent. This test proves it by creating an intent for
/// one (asset, destination, amount) and confirming execution moves exactly
/// that amount of that asset to that destination — there is no parameter on
/// `execute_scheduled_payment` through which a caller could substitute a
/// different one.
#[test]
fn execute_scheduled_payment_only_ever_uses_the_canonical_intent_data() {
    let h = setup();
    let approved_recipient = addr(&h.env);
    let other_recipient = addr(&h.env);
    allow_payment(&h, &approved_recipient);
    h.policy_engine
        .set_recipient_allowed(&other_recipient, &true);

    let intent_id = soroban_sdk::BytesN::from_array(&h.env, &[41u8; 32]);
    h.env.ledger().with_mut(|l| l.sequence_number = 10);
    h.smart_account
        .create_scheduled_payment(&ScheduledIntentArgs {
            intent_id: intent_id.clone(),
            asset: h.token.clone(),
            destination: approved_recipient.clone(),
            amount: 250,
            start_ledger: 10,
            end_ledger: 20,
            interval_ledgers: 0,
            max_executions: 1,
            execution_count: 0,
            policy_version: 0,
            adapter: addr(&h.env),
            cancelled: false,
        });

    h.env.mock_all_auths_allowing_non_root_auth();
    h.smart_account.execute_scheduled_payment(&intent_id, &1);

    let token_client = TokenClient::new(&h.env, &h.token);
    // The approved recipient got exactly the approved amount; a relayer
    // has no way to route funds to `other_recipient` instead, because
    // `execute_scheduled_payment` never accepts a destination parameter.
    assert_eq!(token_client.balance(&approved_recipient), 250);
    assert_eq!(token_client.balance(&other_recipient), 0);
}

/// Security review finding: recovery used to replace only `Ownable`'s
/// `owner`, never the context-rule signer registry — day-to-day spend
/// authority is a completely separate system `owner` doesn't gate at all.
/// This proves the fix at the data level: the pre-recovery context rule
/// (whatever `setup()`'s founding signer produced) is gone after recovery
/// — a fresh rule exists with a *different* ID, carrying exactly the
/// guardian-approved replacement signer and nothing else.
#[test]
fn apply_recovery_replaces_owner_lifts_freeze_and_replaces_signers_exactly_once() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);
    let new_owner = addr(&h.env);
    let guardian = addr(&h.env);
    let request_id = soroban_sdk::BytesN::from_array(&h.env, &[3u8; 32]);

    let rules_before = h.smart_account.get_context_rules_count();
    assert_eq!(rules_before, 1);

    // Freeze the treasury (e.g. the old owner's signer is suspected
    // compromised) — normal payments now fail, and there is no direct
    // unfreeze.
    h.smart_account.freeze();
    let frozen_payment = h
        .smart_account
        .try_execute_transfer_payment(&h.token, &recipient, &100, &1, &1);
    assert!(frozen_payment.is_err());

    // Guardian-driven recovery against the *same* recovery_manager
    // smart_account was configured with.
    h.recovery_manager.add_guardian(&guardian);
    let new_signer = Signer::Delegated(addr(&h.env));
    // recovery_manager enforces a minimum ~1-day delay for both the
    // recovery timelock (MIN_RECOVERY_DELAY_LEDGERS) and newly added
    // guardian activation (GUARDIAN_ACTIVATION_DELAY_LEDGERS) — both 17280
    // ledgers in this build, both counted from ledger 0 here, so a single
    // advance before approving satisfies both.
    h.recovery_manager.open_recovery(
        &h.owner,
        &request_id,
        &new_owner,
        &vec![&h.env, new_signer.clone()],
        &Map::new(&h.env),
        &17280,
    );
    h.env.ledger().with_mut(|l| l.sequence_number = 17280);
    h.recovery_manager.approve_recovery(&request_id, &guardian);
    h.recovery_manager.finalize_recovery(&request_id);

    let applied = h.smart_account.apply_recovery(&request_id);
    assert_eq!(applied, new_owner);

    // Freeze is lifted and the new owner now controls owner-gated actions.
    let status: AccountStatus = h.smart_account.status();
    assert!(!status.frozen);
    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &2, &1);

    // Exactly one context rule still exists, but it is a genuinely new one
    // -- old rule 0 is gone, replaced by a fresh rule carrying only the
    // recovery-approved signer.
    assert_eq!(h.smart_account.get_context_rules_count(), 1);
    let recovered_rule = h.smart_account.get_context_rule(&1);
    assert_eq!(recovered_rule.signers.len(), 1);
    assert_eq!(recovered_rule.signers.get(0).unwrap(), new_signer);
    let old_rule_gone = h.smart_account.try_get_context_rule(&0);
    assert!(old_rule_gone.is_err());

    // Replay guard: applying the same finalized request twice is rejected.
    let replay = h.smart_account.try_apply_recovery(&request_id);
    assert!(replay.is_err());
}

/// Edge case: recovery cannot be applied before `recovery_manager` reports
/// it finalized — the pull is gated on real state, not just "a request
/// exists".
#[test]
fn apply_recovery_before_finalization_is_rejected() {
    let h = setup();
    let new_owner = addr(&h.env);
    let guardian = addr(&h.env);
    let request_id = soroban_sdk::BytesN::from_array(&h.env, &[4u8; 32]);

    h.recovery_manager.add_guardian(&guardian);
    h.recovery_manager.open_recovery(
        &h.owner,
        &request_id,
        &new_owner,
        &vec![&h.env, Signer::Delegated(addr(&h.env))],
        &Map::new(&h.env),
        &17280,
    );
    // No approval yet — threshold is 1, so this alone would suffice, but
    // we never call finalize_recovery at all here.

    let err = h.smart_account.try_apply_recovery(&request_id);
    assert!(err.is_err());
}

/// Audit finding (`docs/TECHNICAL_ARCHITECTURE.md` §16): this contract's
/// own instance-level config (subordinate module addresses, adapter map,
/// frozen flag) and its per-nonce persistent replay records must not
/// archive from disuse — proves both are actually extended, not just
/// wired to compile.
#[test]
fn own_instance_and_nonce_ttl_is_extended_on_use() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    let instance_ttl_before = h.env.as_contract(&h.smart_account.address, || {
        h.env.storage().instance().get_ttl()
    });
    assert!(instance_ttl_before >= crate::TTL_THRESHOLD_LEDGERS);

    h.env
        .ledger()
        .with_mut(|l| l.sequence_number += crate::TTL_THRESHOLD_LEDGERS / 2);
    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &1, &1);

    let instance_ttl_after = h.env.as_contract(&h.smart_account.address, || {
        h.env.storage().instance().get_ttl()
    });
    assert!(instance_ttl_after >= crate::TTL_THRESHOLD_LEDGERS);

    let nonce_ttl = h.env.as_contract(&h.smart_account.address, || {
        h.env
            .storage()
            .persistent()
            .get_ttl(&crate::DataKey::UsedNonce(1))
    });
    assert!(nonce_ttl >= crate::TTL_THRESHOLD_LEDGERS);
}

/// Edge case: the require_auth() gate on interactive payments is not a
/// no-op — with *no* mocked auth and *no* real authorization entries at
/// all, the call must fail hard.
#[test]
#[should_panic]
fn execute_transfer_payment_without_any_authorization_fails() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    h.env.set_auths(&[]);
    h.smart_account
        .execute_transfer_payment(&h.token, &recipient, &100, &1, &1);
}

#[test]
fn contract_name_reports_expected_symbol() {
    let h = setup();
    assert_eq!(
        h.smart_account.contract_name(),
        Symbol::new(&h.env, "sta_acct")
    );
}

/// `pause`/`unpause` are OZ `Pausable` trait entrypoints this contract
/// overrides specifically to add an owner check beyond OZ's own
/// `enforce_owner_auth` (see module docs) — proves the mismatched-caller
/// branch actually panics, not just that it compiles.
#[test]
#[should_panic(expected = "Error(Contract, #8011)")]
fn pause_rejects_caller_that_does_not_match_owner() {
    let h = setup();
    let impostor = addr(&h.env);
    h.smart_account.pause(&impostor);
}

#[test]
#[should_panic(expected = "Error(Contract, #8011)")]
fn unpause_rejects_caller_that_does_not_match_owner() {
    let h = setup();
    h.smart_account.pause(&h.owner);
    let impostor = addr(&h.env);
    h.smart_account.unpause(&impostor);
}

/// Governance entrypoints below come from unmodified OZ trait defaults
/// (`SmartAccount`, `Ownable`) composed via
/// `#[contractimpl(contracttrait)] impl X for SmartAccountTreasury {}` —
/// exercising them here proves the composition is wired correctly on this
/// specific contract, not re-testing OZ's own already-tested logic.
/// `ExecutionEntryPoint` is deliberately not composed (see the doc comment
/// where the OZ trait composition block is declared in `lib.rs`) — its
/// generic `execute(target, target_fn, target_args)` would let any signer
/// call any function on any contract, bypassing every payment-specific
/// safeguard (nonce replay, policy checks, adapter allowlisting, pause/
/// freeze). Its absence is a compile-time guarantee, not something a
/// runtime test can meaningfully assert: `SmartAccountTreasuryClient` has
/// no `.execute()` method at all, so any code that tried to call it simply
/// wouldn't build.
#[test]
fn get_owner_reflects_the_configured_owner() {
    let h = setup();
    assert_eq!(h.smart_account.get_owner(), Some(h.owner.clone()));
}

#[test]
fn ownership_transfers_through_the_two_step_flow() {
    let h = setup();
    let new_owner = addr(&h.env);

    h.smart_account.transfer_ownership(&new_owner, &1_000_000);
    // Not yet transferred until the new owner accepts.
    assert_eq!(h.smart_account.get_owner(), Some(h.owner.clone()));

    h.smart_account.accept_ownership();
    assert_eq!(h.smart_account.get_owner(), Some(new_owner));
}

#[test]
fn add_context_rule_and_signer_persist_through_the_composed_registry() {
    let h = setup();
    let new_signer = Signer::Delegated(addr(&h.env));

    let rule = h.smart_account.add_context_rule(
        &ContextRuleType::Default,
        &String::from_str(&h.env, "extra"),
        &None,
        &vec![&h.env, new_signer.clone()],
        &Map::new(&h.env),
    );
    assert_eq!(rule.signers.len(), 1);

    let signer_id = h
        .smart_account
        .add_signer(&rule.id, &Signer::Delegated(addr(&h.env)));
    let updated = h.smart_account.get_context_rule(&rule.id);
    assert_eq!(updated.signers.len(), 2);

    h.smart_account.remove_signer(&rule.id, &signer_id);
    let after_removal = h.smart_account.get_context_rule(&rule.id);
    assert_eq!(after_removal.signers.len(), 1);
}

/// Cross-contract edge case: a policy version bump between intent creation
/// and execution must NOT be silently absorbed. The intent pinned
/// `policy_version=1` at creation; bumping `policy_engine` to version 2
/// before execution must make the *current* validation fail with
/// `VersionMismatch`, because `execute_scheduled_payment` checks the
/// intent's *pinned* version against `policy_engine`'s *current* one — it
/// does not silently re-approve the payment under new rules, and it does
/// not silently keep honoring the old rules either. A stale-pinned
/// schedule simply stops executing until it is recreated.
#[test]
#[should_panic(expected = "Error(Contract, #2006)")]
fn policy_version_bump_after_intent_creation_blocks_execution() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    let intent_id = soroban_sdk::BytesN::from_array(&h.env, &[50u8; 32]);
    h.env.ledger().with_mut(|l| l.sequence_number = 10);
    h.smart_account
        .create_scheduled_payment(&ScheduledIntentArgs {
            intent_id: intent_id.clone(),
            asset: h.token.clone(),
            destination: recipient.clone(),
            amount: 100,
            start_ledger: 10,
            end_ledger: 1000,
            interval_ledgers: 0,
            max_executions: 1,
            execution_count: 0,
            policy_version: 0,
            adapter: addr(&h.env),
            cancelled: false,
        });
    assert_eq!(h.intent_registry.get_intent(&intent_id).policy_version, 1);

    // Policy changes after the schedule was approved.
    h.policy_engine.bump_version(&2);

    h.env.mock_all_auths_allowing_non_root_auth();
    h.smart_account.execute_scheduled_payment(&intent_id, &1);
}

/// Cross-contract edge case: freezing the treasury must block scheduled
/// execution exactly like interactive payments, even though
/// `execute_scheduled_payment` has no `require_auth()` of its own —
/// `ensure_active` is still checked first.
#[test]
#[should_panic(expected = "Error(Contract, #8003)")]
fn frozen_treasury_blocks_scheduled_execution() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    let intent_id = soroban_sdk::BytesN::from_array(&h.env, &[51u8; 32]);
    h.env.ledger().with_mut(|l| l.sequence_number = 10);
    h.smart_account
        .create_scheduled_payment(&ScheduledIntentArgs {
            intent_id: intent_id.clone(),
            asset: h.token.clone(),
            destination: recipient,
            amount: 100,
            start_ledger: 10,
            end_ledger: 1000,
            interval_ledgers: 0,
            max_executions: 1,
            execution_count: 0,
            policy_version: 0,
            adapter: addr(&h.env),
            cancelled: false,
        });

    h.smart_account.freeze();

    h.env.mock_all_auths_allowing_non_root_auth();
    h.smart_account.execute_scheduled_payment(&intent_id, &1);
}

/// Cross-contract edge case, and a real feature gap this session closed:
/// `cancel_scheduled_payment` must actually stop a scheduled payment from
/// executing, exercised through the full `smart_account` -> `intent_registry`
/// path (not just `intent_registry`'s own unit test).
#[test]
#[should_panic(expected = "Error(Contract, #3006)")]
fn cancelled_scheduled_payment_cannot_execute_end_to_end() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    let intent_id = soroban_sdk::BytesN::from_array(&h.env, &[52u8; 32]);
    h.env.ledger().with_mut(|l| l.sequence_number = 10);
    h.smart_account
        .create_scheduled_payment(&ScheduledIntentArgs {
            intent_id: intent_id.clone(),
            asset: h.token.clone(),
            destination: recipient,
            amount: 100,
            start_ledger: 10,
            end_ledger: 1000,
            interval_ledgers: 0,
            max_executions: 1,
            execution_count: 0,
            policy_version: 0,
            adapter: addr(&h.env),
            cancelled: false,
        });

    h.smart_account.cancel_scheduled_payment(&intent_id);
    assert!(h.intent_registry.get_intent(&intent_id).cancelled);

    h.env.mock_all_auths_allowing_non_root_auth();
    h.smart_account.execute_scheduled_payment(&intent_id, &1);
}

/// Cross-contract edge case: a split payment must fail closed for the
/// *entire* batch if any single recipient's amount exceeds the asset cap,
/// even when every recipient is individually allow-listed and other
/// recipients' amounts are well within limits. Proves per-recipient
/// enforcement isn't just about the allowlist — the amount cap is checked
/// per recipient too, not against some aggregate total.
#[test]
#[should_panic(expected = "Error(Contract, #2005)")]
fn split_payment_fails_closed_when_one_recipient_amount_exceeds_the_cap() {
    let h = setup();
    let alice = addr(&h.env);
    let bob = addr(&h.env);
    allow_payment(&h, &alice); // asset cap = 10_000
    h.policy_engine.set_recipient_allowed(&bob, &true);

    // alice's share alone exceeds the configured max_single_transfer.
    h.smart_account.execute_split_payment(
        &h.token,
        &vec![&h.env, alice, bob],
        &vec![&h.env, 20_000, 50],
        &1,
        &1,
    );
}

/// Cross-contract edge case: the interactive-payment nonce space is shared
/// across operations — a nonce already consumed by a transfer cannot be
/// reused by a split, or vice versa. This is a single global replay space
/// per treasury, not a per-operation one.
#[test]
#[should_panic(expected = "Error(Contract, #8005)")]
fn nonce_replay_protection_is_shared_across_transfer_and_split_operations() {
    let h = setup();
    let alice = addr(&h.env);
    let bob = addr(&h.env);
    allow_payment(&h, &alice);
    h.policy_engine.set_recipient_allowed(&bob, &true);

    h.smart_account
        .execute_transfer_payment(&h.token, &alice, &100, &9, &1);

    // Same nonce (9), different operation — must still be rejected.
    h.smart_account.execute_split_payment(
        &h.token,
        &vec![&h.env, alice, bob],
        &vec![&h.env, 50, 50],
        &9,
        &1,
    );
}

/// Cross-contract edge case, and a real gap an independent security review
/// (`docs/SMART_CONTRACT_AUDIT_REPORT.md` finding 1) confirmed: the adapter
/// used to execute a scheduled payment used to be resolved at *execution*
/// time via the treasury's then-current adapter configuration, not pinned
/// when the payment was approved — so reconfiguring `transfer`'s adapter
/// between creation and execution could silently redirect an
/// already-approved payment through a different adapter. Fixed by pinning
/// the adapter address on the `ScheduledIntent` at `create_scheduled_payment`
/// time, mirroring how `policy_version` is already pinned.
///
/// This test proves the fix survives even a *fully applied* reconfiguration,
/// not just a pending one (adapter changes are separately timelocked per
/// finding 3 — see the `propose_adapter_change`/`apply_adapter_change`
/// tests below for that mechanism on its own): `transfer`'s adapter is
/// proposed and, once the change's own delay elapses, applied to an
/// address that is not a valid `TransferAdapter` at all (so
/// `execute_transfer` would trap if it were ever actually invoked).
/// Execution still succeeds — proof it never consults the treasury's
/// current adapter configuration and instead dispatches to the adapter
/// pinned on the intent at creation.
#[test]
fn scheduled_payment_uses_the_adapter_pinned_at_approval_not_a_later_reconfiguration() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);

    let intent_id = soroban_sdk::BytesN::from_array(&h.env, &[53u8; 32]);
    h.env.ledger().with_mut(|l| l.sequence_number = 10);
    h.smart_account
        .create_scheduled_payment(&ScheduledIntentArgs {
            intent_id: intent_id.clone(),
            asset: h.token.clone(),
            destination: recipient.clone(),
            amount: 100,
            start_ledger: 10,
            end_ledger: 20_000,
            interval_ledgers: 0,
            max_executions: 1,
            execution_count: 0,
            policy_version: 0,
            // Overwritten by `create_scheduled_payment` with the currently
            // configured transfer_adapter regardless of what's supplied here.
            adapter: addr(&h.env),
            cancelled: false,
        });

    // Propose, then (after the adapter-change delay elapses) apply a
    // reconfiguration of `transfer` to an address with no `TransferAdapter`
    // (or any) contract behind it at all — if execution ever resolved the
    // adapter fresh instead of using the pinned one, this would trap.
    let not_an_adapter = addr(&h.env);
    h.smart_account
        .propose_adapter_change(&Symbol::new(&h.env, "transfer"), &not_an_adapter);
    h.env.ledger().with_mut(|l| l.sequence_number = 10 + 17280);
    h.smart_account
        .apply_adapter_change(&Symbol::new(&h.env, "transfer"));

    h.env.mock_all_auths_allowing_non_root_auth();
    h.smart_account.execute_scheduled_payment(&intent_id, &1);

    let token_client = TokenClient::new(&h.env, &h.token);
    assert_eq!(token_client.balance(&recipient), 100);
}

/// The mirror image of the test above: creating a schedule before any
/// `transfer` adapter has ever been configured must fail fast, at creation
/// time, rather than silently approving a schedule with nothing to pin.
#[test]
#[should_panic(expected = "Error(Contract, #8004)")]
fn creating_a_scheduled_payment_before_an_adapter_is_configured_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = addr(&env);
    let policy_engine_id = env.register(sta_policy_engine::PolicyEngine, ());
    sta_policy_engine::PolicyEngineClient::new(&env, &policy_engine_id).initialize(&owner);
    let intent_registry_id = env.register(sta_intent_registry::IntentRegistry, ());
    let recovery_manager_id = env.register(sta_recovery_manager::RecoveryManager, ());
    sta_recovery_manager::RecoveryManagerClient::new(&env, &recovery_manager_id)
        .initialize(&owner, &1);

    let smart_account_id = env.register(SmartAccountTreasury, ());
    let smart_account = SmartAccountTreasuryClient::new(&env, &smart_account_id);
    let founding_signer = Signer::Delegated(addr(&env));
    smart_account.initialize(
        &owner,
        &vec![&env, founding_signer],
        &Map::new(&env),
        &InitConfig {
            policy_engine: policy_engine_id,
            intent_registry: intent_registry_id,
            recovery_manager: recovery_manager_id,
            initial_adapters: Map::new(&env),
            initial_executor: owner.clone(),
        },
    );
    // Deliberately never proposing/applying an adapter for "transfer".

    smart_account.create_scheduled_payment(&ScheduledIntentArgs {
        intent_id: soroban_sdk::BytesN::from_array(&env, &[77u8; 32]),
        asset: addr(&env),
        destination: addr(&env),
        amount: 100,
        start_ledger: 0,
        end_ledger: 1000,
        interval_ledgers: 0,
        max_executions: 1,
        execution_count: 0,
        policy_version: 0,
        adapter: addr(&env),
        cancelled: false,
    });
}

/// Independent security review finding (`docs/SMART_CONTRACT_AUDIT_REPORT.md`
/// finding 3): adapter changes used to take effect immediately under
/// single-owner control. This proves the timelock is real — applying
/// before the delay has elapsed is rejected.
#[test]
fn adapter_change_cannot_be_applied_before_the_delay_elapses() {
    let h = setup();
    let new_adapter = addr(&h.env);
    h.smart_account
        .propose_adapter_change(&Symbol::new(&h.env, "transfer"), &new_adapter);

    let too_early = h
        .smart_account
        .try_apply_adapter_change(&Symbol::new(&h.env, "transfer"));
    assert_eq!(
        too_early,
        Err(Ok(SmartAccountTreasuryError::AdapterChangeDelayNotElapsed))
    );
}

/// The owner can walk back an erroneous or no-longer-wanted proposal
/// before it takes effect — mirrors `recovery_manager::cancel_recovery`'s
/// existing pattern for open recovery requests.
#[test]
fn adapter_change_can_be_cancelled_before_it_takes_effect() {
    let h = setup();
    let new_adapter = addr(&h.env);
    h.smart_account
        .propose_adapter_change(&Symbol::new(&h.env, "transfer"), &new_adapter);
    h.smart_account
        .cancel_adapter_change(&Symbol::new(&h.env, "transfer"));

    h.env.ledger().with_mut(|l| l.sequence_number = 17280);
    let no_pending = h
        .smart_account
        .try_apply_adapter_change(&Symbol::new(&h.env, "transfer"));
    assert_eq!(
        no_pending,
        Err(Ok(SmartAccountTreasuryError::NoPendingAdapterChange))
    );
}

#[test]
fn cancel_adapter_change_rejects_when_nothing_is_pending() {
    let h = setup();
    let err = h
        .smart_account
        .try_cancel_adapter_change(&Symbol::new(&h.env, "transfer"));
    assert_eq!(
        err,
        Err(Ok(SmartAccountTreasuryError::NoPendingAdapterChange))
    );
}

/// The authorization decision already happened at proposal time
/// (an owner-gated call); applying an already-authorized, delay-elapsed
/// change is deliberately permissionless — same pattern as
/// `apply_recovery`/`apply_guardian_freeze`. Proven here with zero mocked
/// or real authorization at all.
#[test]
fn applying_an_adapter_change_needs_no_authorization() {
    let h = setup();
    let new_adapter = addr(&h.env);
    h.smart_account
        .propose_adapter_change(&Symbol::new(&h.env, "transfer"), &new_adapter);

    h.env.ledger().with_mut(|l| l.sequence_number = 17280);
    h.env.set_auths(&[]);
    h.smart_account
        .apply_adapter_change(&Symbol::new(&h.env, "transfer"));
}

/// Independent review finding: `propose_adapter_change` wrote the pending
/// proposal into instance storage without explicitly extending the
/// instance TTL. Since instance storage shares one TTL across all of this
/// contract's own config, a proposal made on an otherwise-dormant treasury
/// could archive before anyone calls `apply_adapter_change`, silently
/// losing a legitimate governance action instead of merely delaying it —
/// a liveness bug, not a fund-draining one, but still a real gap in the
/// "delay then apply" contract the timelock is supposed to provide. This
/// proves `propose_adapter_change` (and, since they touch the same shared
/// TTL, `apply_adapter_change`) actually refresh it.
#[test]
fn propose_and_apply_adapter_change_extend_the_instance_ttl() {
    let h = setup();
    let new_adapter = addr(&h.env);

    let ttl_before = h.env.as_contract(&h.smart_account.address, || {
        h.env.storage().instance().get_ttl()
    });
    assert!(ttl_before >= crate::TTL_THRESHOLD_LEDGERS);

    h.env
        .ledger()
        .with_mut(|l| l.sequence_number += crate::TTL_THRESHOLD_LEDGERS / 2);
    h.smart_account
        .propose_adapter_change(&Symbol::new(&h.env, "transfer"), &new_adapter);

    let ttl_after_propose = h.env.as_contract(&h.smart_account.address, || {
        h.env.storage().instance().get_ttl()
    });
    assert!(ttl_after_propose >= crate::TTL_THRESHOLD_LEDGERS);

    h.env
        .ledger()
        .with_mut(|l| l.sequence_number += crate::TTL_THRESHOLD_LEDGERS / 2);
    h.smart_account
        .apply_adapter_change(&Symbol::new(&h.env, "transfer"));

    let ttl_after_apply = h.env.as_contract(&h.smart_account.address, || {
        h.env.storage().instance().get_ttl()
    });
    assert!(ttl_after_apply >= crate::TTL_THRESHOLD_LEDGERS);
}

/// Independent review finding: an otherwise fully dormant treasury (no
/// owner action of any kind) has no way to keep a pending adapter proposal
/// alive beyond the one bump `propose_adapter_change` makes at creation
/// time. `extend_instance_ttl` closes that: permissionless, no auth
/// required, extends no authority.
#[test]
fn extend_instance_ttl_is_permissionless_and_refreshes_ttl() {
    let h = setup();
    let new_adapter = addr(&h.env);
    h.smart_account
        .propose_adapter_change(&Symbol::new(&h.env, "transfer"), &new_adapter);

    h.env
        .ledger()
        .with_mut(|l| l.sequence_number += crate::TTL_THRESHOLD_LEDGERS / 2);
    h.env.set_auths(&[]);
    h.smart_account.extend_instance_ttl();

    let ttl_after = h.env.as_contract(&h.smart_account.address, || {
        h.env.storage().instance().get_ttl()
    });
    assert!(ttl_after >= crate::TTL_THRESHOLD_LEDGERS);
}

/// Real gap this session found: `docs/TECHNICAL_ARCHITECTURE.md` §12.7's
/// canonical recovery workflow starts with "Guardian ... triggers freeze,"
/// but `freeze()` was owner-only. This exercises the full cross-contract
/// path proving a guardian can independently stop spend authority — no
/// owner cooperation needed — while a full recovery is still pending.
#[test]
fn guardian_can_independently_freeze_the_treasury() {
    let h = setup();
    let recipient = addr(&h.env);
    allow_payment(&h, &recipient);
    let guardian = addr(&h.env);

    h.recovery_manager.add_guardian(&guardian);
    h.env.ledger().with_mut(|l| l.sequence_number = 17280);

    // No owner involvement at all in raising the flag or applying it.
    h.recovery_manager.request_guardian_freeze(&guardian);
    h.smart_account.apply_guardian_freeze();

    let status: AccountStatus = h.smart_account.status();
    assert!(status.frozen);
    let blocked = h
        .smart_account
        .try_execute_transfer_payment(&h.token, &recipient, &100, &1, &1);
    assert!(blocked.is_err());
}

/// Security review finding on an earlier revision: `recovery_manager`
/// exposed a public `consume_guardian_freeze_request` with no
/// authorization at all, so any third party — not just this contract —
/// could call it directly and clear a guardian's freeze request before
/// `apply_guardian_freeze` ever ran, a front-run/grief. That function is
/// gone; `guardian_freeze_epoch` is a plain, permissionless *view* with no
/// side effects, so a third party reading it (however many times, from
/// wherever) cannot disturb it. Proven directly: a bystander reads the
/// epoch first, then the legitimate `apply_guardian_freeze` call still
/// succeeds exactly as if the read never happened.
#[test]
fn a_bystander_reading_the_freeze_epoch_cannot_grief_it() {
    let h = setup();
    let guardian = addr(&h.env);
    h.recovery_manager.add_guardian(&guardian);
    h.env.ledger().with_mut(|l| l.sequence_number = 17280);
    h.recovery_manager.request_guardian_freeze(&guardian);

    // A bystander -- not smart_account, not a guardian, not the owner --
    // reads the epoch. This is a plain view; nothing to authorize, nothing
    // it could clear.
    let bystander = addr(&h.env);
    h.env.set_auths(&[]);
    let observed_epoch = h.recovery_manager.guardian_freeze_epoch();
    assert_eq!(observed_epoch, 1);
    let _ = bystander;

    h.env.mock_all_auths();
    h.smart_account.apply_guardian_freeze();
    assert!(h.smart_account.status().frozen);
}

/// A guardian who never actually requested a freeze cannot have
/// `apply_guardian_freeze` succeed off of nothing — the pull is gated on
/// real state from `recovery_manager`, same discipline as `apply_recovery`.
#[test]
#[should_panic(expected = "Error(Contract, #8012)")]
fn apply_guardian_freeze_without_a_request_is_rejected() {
    let h = setup();
    h.smart_account.apply_guardian_freeze();
}

/// Security review finding: `apply_guardian_freeze` used to read a flag in
/// `recovery_manager` that was set once and never cleared -- meaning a
/// stale, long-resolved freeze request could be replayed by anyone,
/// indefinitely, including after a full recovery had already restored
/// normal operation. This proves the fix: a second call, with no new
/// `request_guardian_freeze` in between, is rejected exactly like the
/// "never requested" case above -- `recovery_manager`'s freeze epoch hasn't
/// advanced since the last one this contract applied, tracked locally,
/// matching `apply_recovery`'s own request-keyed replay guard.
#[test]
#[should_panic(expected = "Error(Contract, #8012)")]
fn apply_guardian_freeze_cannot_replay_an_already_consumed_request() {
    let h = setup();
    let guardian = addr(&h.env);
    h.recovery_manager.add_guardian(&guardian);
    h.env.ledger().with_mut(|l| l.sequence_number = 17280);

    h.recovery_manager.request_guardian_freeze(&guardian);
    h.smart_account.apply_guardian_freeze();
    assert!(h.smart_account.status().frozen);

    // No new request_guardian_freeze call -- this must fail, not silently
    // re-freeze off the same stale flag.
    h.smart_account.apply_guardian_freeze();
}

/// Real gap this session found: `docs/TECHNICAL_ARCHITECTURE.md` §12.6
/// documents SmartAccount as validating "duplicate destinations" for a
/// split — a repeated recipient in the same split call must be rejected
/// outright, not silently split across two separate transfers.
#[test]
#[should_panic(expected = "Error(Contract, #8013)")]
fn split_payment_rejects_duplicate_recipient() {
    let h = setup();
    let alice = addr(&h.env);
    allow_payment(&h, &alice);

    h.smart_account.execute_split_payment(
        &h.token,
        &vec![&h.env, alice.clone(), alice],
        &vec![&h.env, 100, 200],
        &1,
        &1,
    );
}

#[allow(dead_code)]
fn assert_error_type(_: SmartAccountTreasuryError) {}
