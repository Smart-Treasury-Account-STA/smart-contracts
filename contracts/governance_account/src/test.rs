//! Integration/wiring tests, mirroring `smart_account`'s own testing
//! boundary: run under `env.mock_all_auths()`, which satisfies every
//! `require_auth()` without invoking `__check_auth` -- these prove the
//! bootstrap/composition wiring is correct (initialize seeds the right
//! signers and policy, double-init is rejected, `threshold_policy`
//! actually gets installed at initialize time), not the authorization
//! logic itself. `__check_auth` here is a one-line delegation to
//! `do_check_auth`, already covered by `stellar-accounts`' own test suite
//! and by `threshold_policy`'s tests for the policy layer.

extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, IntoVal, Map, String};
use stellar_accounts::{
    policies::simple_threshold::SimpleThresholdAccountParams,
    smart_account::{ContextRuleType, Signer},
};

use crate::{GovernanceAccount, GovernanceAccountClient, GovernanceAccountError};

fn addr(e: &Env) -> Address {
    Address::generate(e)
}

fn setup(e: &Env) -> GovernanceAccountClient<'static> {
    let contract_id = e.register(GovernanceAccount, ());
    GovernanceAccountClient::new(e, &contract_id)
}

#[test]
fn contract_name_reports_expected_symbol() {
    let e = Env::default();
    let gov = setup(&e);
    assert_eq!(gov.contract_name(), soroban_sdk::symbol_short!("sta_gov"));
}

#[test]
fn initializes_with_founding_signers_and_reports_status() {
    let e = Env::default();
    e.mock_all_auths();
    let gov = setup(&e);
    let s1 = Signer::Delegated(addr(&e));
    let s2 = Signer::Delegated(addr(&e));

    gov.initialize(&addr(&e), &vec![&e, s1, s2], &Map::new(&e));

    assert!(gov.status());
    let rule = gov.get_context_rule(&0);
    assert_eq!(rule.signers.len(), 2);
    assert_eq!(rule.name, String::from_str(&e, "governance"));
    assert_eq!(rule.context_type, ContextRuleType::Default);
}

#[test]
fn double_initialize_is_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let gov = setup(&e);
    let signers = vec![&e, Signer::Delegated(addr(&e))];
    let caller = addr(&e);

    gov.initialize(&caller, &signers, &Map::new(&e));
    let err = gov
        .try_initialize(&caller, &signers, &Map::new(&e))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, GovernanceAccountError::AlreadyInitialized);
}

#[test]
fn status_before_initialize_reports_not_initialized() {
    let e = Env::default();
    let gov = setup(&e);

    let err = gov.try_status().unwrap_err().unwrap();
    assert_eq!(err, GovernanceAccountError::NotInitialized);
}

/// The real point of this contract: a `threshold_policy` installed at
/// initialize time turns "every signer must approve" into a genuine
/// N-of-M gate, exactly the fix for the signer-set-divergence class of
/// bug this design exists to close.
#[test]
fn initializes_with_a_threshold_policy_installed() {
    let e = Env::default();
    e.mock_all_auths();
    let gov = setup(&e);
    let policy_id = e.register(sta_threshold_policy::ThresholdPolicy, ());

    let signers = vec![
        &e,
        Signer::Delegated(addr(&e)),
        Signer::Delegated(addr(&e)),
        Signer::Delegated(addr(&e)),
    ];
    let mut policies = Map::new(&e);
    policies.set(
        policy_id.clone(),
        SimpleThresholdAccountParams { threshold: 2 }.into_val(&e),
    );

    gov.initialize(&addr(&e), &signers, &policies);

    let rule = gov.get_context_rule(&0);
    assert_eq!(rule.policies.len(), 1);
    assert_eq!(rule.policies.get(0).unwrap(), policy_id);

    let policy_client = sta_threshold_policy::ThresholdPolicyClient::new(&e, &policy_id);
    assert_eq!(policy_client.get_threshold(&rule.id, &gov.address), 2);
}

/// Adding a signer to an already-thresholded rule doesn't need to touch
/// the threshold -- confirms the composed OZ registry entrypoints work
/// against this contract exactly as they do against `smart_account`'s
/// identical composition.
#[test]
fn add_and_remove_signer_persist_through_the_composed_registry() {
    let e = Env::default();
    e.mock_all_auths();
    let gov = setup(&e);
    gov.initialize(
        &addr(&e),
        &vec![&e, Signer::Delegated(addr(&e)), Signer::Delegated(addr(&e))],
        &Map::new(&e),
    );

    let new_signer = Signer::Delegated(addr(&e));
    let signer_id = gov.add_signer(&0, &new_signer);
    assert_eq!(gov.get_context_rule(&0).signers.len(), 3);

    gov.remove_signer(&0, &signer_id);
    assert_eq!(gov.get_context_rule(&0).signers.len(), 2);
}

#[test]
fn extend_instance_ttl_is_permissionless() {
    let e = Env::default();
    e.mock_all_auths();
    let gov = setup(&e);
    gov.initialize(
        &addr(&e),
        &vec![&e, Signer::Delegated(addr(&e))],
        &Map::new(&e),
    );

    e.set_auths(&[]);
    gov.extend_instance_ttl();
}

/// Security review finding on an earlier revision: `initialize` took no
/// address and required no authorization at all, so anyone could bootstrap
/// a freshly deployed instance with signers of their own choosing.
#[test]
#[should_panic]
fn initialize_without_caller_authorization_fails() {
    let e = Env::default();
    let gov = setup(&e);
    e.set_auths(&[]);
    gov.initialize(
        &addr(&e),
        &vec![&e, Signer::Delegated(addr(&e))],
        &Map::new(&e),
    );
}
