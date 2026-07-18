extern crate std;

use soroban_sdk::{
    testutils::Address as _, token::StellarAssetClient, token::TokenClient, Address, Env,
};

use crate::{TransferAdapter, TransferAdapterClient, TransferAdapterError};

fn setup(e: &Env) -> (TransferAdapterClient<'static>, Address, Address) {
    e.mock_all_auths();
    let contract_id = e.register(TransferAdapter, ());
    let client = TransferAdapterClient::new(e, &contract_id);
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
fn moves_funds_from_treasury_to_recipient() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    let token = issue_token(&e, &admin);
    let recipient = Address::generate(&e);

    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);

    client.execute_transfer(&token, &recipient, &400);

    let token_client = TokenClient::new(&e, &token);
    assert_eq!(token_client.balance(&smart_account), 600);
    assert_eq!(token_client.balance(&recipient), 400);
}

#[test]
#[should_panic(expected = "Error(Contract, #6002)")]
fn rejects_non_positive_amount() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    let token = issue_token(&e, &admin);
    let recipient = Address::generate(&e);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);

    client.execute_transfer(&token, &recipient, &0);
}

#[test]
#[should_panic]
fn double_initialize_is_rejected() {
    let e = Env::default();
    let (client, admin, smart_account) = setup(&e);
    client.initialize(&admin, &smart_account);
}

/// Edge case: the adapter requires the *configured* smart_account's
/// authorization for the call, not just "any authorization present" in the
/// transaction. Without `mock_all_auths`, and without an explicit auth for
/// the smart_account address, the call must fail.
#[test]
#[should_panic]
fn execute_transfer_without_smart_account_authorization_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(TransferAdapter, ());
    let client = TransferAdapterClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    let smart_account = Address::generate(&e);
    client.initialize(&admin, &smart_account);
    let token = issue_token(&e, &admin);
    let recipient = Address::generate(&e);
    StellarAssetClient::new(&e, &token).mint(&smart_account, &1_000);

    // Turn auth mocking off and don't provide any auth entries: the
    // require_auth() on smart_account inside execute_transfer must fail.
    e.set_auths(&[]);
    client.execute_transfer(&token, &recipient, &100);
}

#[test]
fn contract_name_reports_expected_symbol() {
    let e = Env::default();
    let (client, _admin, _smart_account) = setup(&e);
    assert_eq!(
        client.contract_name(),
        soroban_sdk::symbol_short!("sta_xfer")
    );
}

#[allow(dead_code)]
fn assert_error_type(_: TransferAdapterError) {}
