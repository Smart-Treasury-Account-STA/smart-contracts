extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};
use stellar_accounts::{
    policies::simple_threshold::SimpleThresholdAccountParams,
    smart_account::{ContextRule, ContextRuleType, Signer},
};

use crate::{ThresholdPolicy, ThresholdPolicyClient};

/// This wrapper delegates to already-tested OZ logic — these tests prove
/// the delegation itself is wired correctly (right functions, right
/// arguments, errors propagate), not the threshold math itself, which is
/// `stellar-accounts`' own tested and audited responsibility.
fn setup(e: &Env) -> ThresholdPolicyClient<'static> {
    let contract_id = e.register(ThresholdPolicy, ());
    ThresholdPolicyClient::new(e, &contract_id)
}

/// A synthetic rule good enough for `simple_threshold`'s own logic, which
/// only reads `id` and `signers.len()` — no real registered context rule
/// on a real SmartAccount composition is needed to exercise this wrapper.
fn rule(e: &Env, id: u32, signer_count: u32) -> ContextRule {
    let mut signers = vec![e];
    for _ in 0..signer_count {
        signers.push_back(Signer::Delegated(Address::generate(e)));
    }
    ContextRule {
        id,
        context_type: ContextRuleType::Default,
        name: String::from_str(e, "test"),
        signers,
        signer_ids: vec![e],
        policies: vec![e],
        policy_ids: vec![e],
        valid_until: None,
    }
}

#[test]
fn install_sets_threshold_and_get_threshold_reflects_it() {
    let e = Env::default();
    e.mock_all_auths();
    let client = setup(&e);
    let smart_account = Address::generate(&e);
    let r = rule(&e, 0, 3);

    client.install(
        &SimpleThresholdAccountParams { threshold: 2 },
        &r,
        &smart_account,
    );

    assert_eq!(client.get_threshold(&r.id, &smart_account), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3201)")]
fn install_rejects_threshold_above_signer_count() {
    let e = Env::default();
    e.mock_all_auths();
    let client = setup(&e);
    let smart_account = Address::generate(&e);
    let r = rule(&e, 0, 2);

    // InvalidThreshold (#3201) -- threshold exceeds the rule's signer count.
    client.install(
        &SimpleThresholdAccountParams { threshold: 3 },
        &r,
        &smart_account,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3203)")]
fn install_twice_for_the_same_rule_is_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    let client = setup(&e);
    let smart_account = Address::generate(&e);
    let r = rule(&e, 0, 3);

    client.install(
        &SimpleThresholdAccountParams { threshold: 1 },
        &r,
        &smart_account,
    );
    // AlreadyInstalled (#3203).
    client.install(
        &SimpleThresholdAccountParams { threshold: 2 },
        &r,
        &smart_account,
    );
}

#[test]
fn enforce_succeeds_when_authenticated_signers_meet_the_threshold() {
    let e = Env::default();
    e.mock_all_auths();
    let client = setup(&e);
    let smart_account = Address::generate(&e);
    let r = rule(&e, 0, 3);
    client.install(
        &SimpleThresholdAccountParams { threshold: 2 },
        &r,
        &smart_account,
    );

    let context = fake_context(&e, &smart_account);
    let two_signers = vec![&e, r.signers.get(0).unwrap(), r.signers.get(1).unwrap()];
    client.enforce(&context, &two_signers, &r, &smart_account);
}

#[test]
#[should_panic(expected = "Error(Contract, #3202)")]
fn enforce_rejects_when_authenticated_signers_are_short_of_the_threshold() {
    let e = Env::default();
    e.mock_all_auths();
    let client = setup(&e);
    let smart_account = Address::generate(&e);
    let r = rule(&e, 0, 3);
    client.install(
        &SimpleThresholdAccountParams { threshold: 2 },
        &r,
        &smart_account,
    );

    let context = fake_context(&e, &smart_account);
    let one_signer = vec![&e, r.signers.get(0).unwrap()];
    // NotAllowed (#3202).
    client.enforce(&context, &one_signer, &r, &smart_account);
}

#[test]
fn set_threshold_updates_the_stored_value() {
    let e = Env::default();
    e.mock_all_auths();
    let client = setup(&e);
    let smart_account = Address::generate(&e);
    let r = rule(&e, 0, 3);
    client.install(
        &SimpleThresholdAccountParams { threshold: 1 },
        &r,
        &smart_account,
    );

    client.set_threshold(&2, &r, &smart_account);

    assert_eq!(client.get_threshold(&r.id, &smart_account), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #3200)")]
fn uninstall_removes_the_policy_and_get_threshold_then_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let client = setup(&e);
    let smart_account = Address::generate(&e);
    let r = rule(&e, 0, 3);
    client.install(
        &SimpleThresholdAccountParams { threshold: 1 },
        &r,
        &smart_account,
    );

    client.uninstall(&r, &smart_account);

    // SmartAccountNotInstalled (#3200).
    client.get_threshold(&r.id, &smart_account);
}

fn fake_context(e: &Env, smart_account: &Address) -> soroban_sdk::auth::Context {
    soroban_sdk::auth::Context::Contract(soroban_sdk::auth::ContractContext {
        contract: smart_account.clone(),
        fn_name: soroban_sdk::symbol_short!("noop"),
        args: vec![e],
    })
}
