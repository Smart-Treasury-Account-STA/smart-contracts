extern crate std;

use soroban_sdk::{
    testutils::Address as _, token::StellarAssetClient, token::TokenClient, vec, Address, Env,
};

use crate::{SplitAdapter, SplitAdapterClient, SplitAdapterError};

fn setup(e: &Env) -> (SplitAdapterClient<'static>, Address, Address) {
    e.mock_all_auths();
    let contract_id = e.register(SplitAdapter, ());
    let client = SplitAdapterClient::new(e, &contract_id);
    let admin = Address::generate(e);
    let smart_account = Address::generate(e);
    client.initialize(&admin, &smart_account);
    (client, admin, smart_account)
}

fn issue_token(e: &Env, treasury_admin: &Address) -> Address {
    e.register_stellar_asset_contract_v2(treasury_admin.clone())
        .address()
}

#[test]
fn splits_balance_across_recipients_by_index() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    let token = issue_token(&e, &admin);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);
    let token_client = TokenClient::new(&e, &token);
    // In the real flow, smart_account grants this allowance itself
    // (`approve_adapter` in contracts/smart_account/src/lib.rs) immediately
    // before calling execute_split -- see docs/SECURITY_REVIEW_STRICT.md
    // finding 29.
    token_client.approve(
        &smart_account,
        &client.address,
        &1_000,
        &e.ledger().sequence(),
    );

    client.execute_split(
        &token,
        &vec![&e, alice.clone(), bob.clone()],
        &vec![&e, 300, 700],
    );

    assert_eq!(token_client.balance(&smart_account), 0);
    assert_eq!(token_client.balance(&alice), 300);
    assert_eq!(token_client.balance(&bob), 700);
}

#[test]
#[should_panic(expected = "Error(Contract, #7005)")]
fn rejects_mismatched_recipient_and_amount_lengths() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    let token = issue_token(&e, &admin);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);

    client.execute_split(&token, &vec![&e, alice, bob], &vec![&e, 300]);
}

#[test]
#[should_panic(expected = "Error(Contract, #7003)")]
fn rejects_empty_split() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    let token = issue_token(&e, &admin);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);

    let empty_recipients: soroban_sdk::Vec<Address> = vec![&e];
    let empty_amounts: soroban_sdk::Vec<i128> = vec![&e];
    client.execute_split(&token, &empty_recipients, &empty_amounts);
}

#[test]
#[should_panic(expected = "Error(Contract, #7002)")]
fn rejects_any_non_positive_amount_in_the_batch() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    let token = issue_token(&e, &admin);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);

    // One valid, one zero — the whole batch must fail, not just skip the bad entry.
    client.execute_split(&token, &vec![&e, alice, bob], &vec![&e, 300, 0]);
}

#[test]
#[should_panic(expected = "Error(Contract, #7004)")]
fn rejects_recipient_count_above_the_bound() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    let token = issue_token(&e, &admin);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &10_000);

    let mut recipients = soroban_sdk::Vec::<Address>::new(&e);
    let mut amounts = soroban_sdk::Vec::<i128>::new(&e);
    for _ in 0..21 {
        recipients.push_back(Address::generate(&e));
        amounts.push_back(1);
    }

    client.execute_split(&token, &recipients, &amounts);
}

#[test]
#[should_panic]
fn execute_split_without_smart_account_authorization_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(SplitAdapter, ());
    let client = SplitAdapterClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    let smart_account = Address::generate(&e);
    client.initialize(&admin, &smart_account);
    let token = issue_token(&e, &admin);
    let alice = Address::generate(&e);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);

    e.set_auths(&[]);
    client.execute_split(&token, &vec![&e, alice], &vec![&e, 100]);
}

#[test]
#[should_panic]
fn double_initialize_is_rejected() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    client.initialize(&admin, &smart_account);
}

#[test]
fn contract_name_reports_expected_symbol() {
    let e = Env::default();
    let (client, _admin, _smart_account) = setup(&e);
    assert_eq!(
        client.contract_name(),
        soroban_sdk::symbol_short!("sta_splt")
    );
}

/// Security review finding: same gap as `transfer_adapter`'s identical
/// entrypoint -- no permissionless way to keep instance TTL alive
/// independent of `execute_split` succeeding.
#[test]
fn extend_instance_ttl_is_permissionless() {
    let e = Env::default();
    let (client, _admin, _smart_account) = setup(&e);
    e.set_auths(&[]);
    client.extend_instance_ttl();
}

#[allow(dead_code)]
fn assert_error_type(_: SplitAdapterError) {}
