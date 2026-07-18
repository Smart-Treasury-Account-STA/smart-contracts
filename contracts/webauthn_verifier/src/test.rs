extern crate std;

use ed25519_dalek::{Signer, SigningKey};
use hex_literal::hex;
use p256::{
    ecdsa::{
        signature::hazmat::PrehashSigner, Signature as Secp256r1Signature,
        SigningKey as Secp256r1SigningKey,
    },
    elliptic_curve::sec1::ToEncodedPoint,
    SecretKey as Secp256r1SecretKey,
};
use soroban_sdk::{vec, Bytes, BytesN, Env, IntoVal, TryIntoVal, Val, Vec};

use crate::{WebauthnVerifier, WebauthnVerifierClient};
use stellar_accounts::verifiers::{utils::base64_url_encode, webauthn::WebAuthnSigData};

const AUTH_DATA_FLAGS_UP: u8 = 0x01;
const AUTH_DATA_FLAGS_UV: u8 = 0x04;
const AUTH_DATA_FLAGS_BE: u8 = 0x08;
const AUTH_DATA_FLAGS_BS: u8 = 0x10;

fn setup(e: &Env) -> WebauthnVerifierClient<'static> {
    let contract_id = e.register(WebauthnVerifier, ());
    WebauthnVerifierClient::new(e, &contract_id)
}

fn sign_secp256r1(e: &Env, digest: [u8; 32]) -> (BytesN<65>, BytesN<64>) {
    let secret_key_bytes: [u8; 32] = [
        33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63, 64,
    ];
    let secret_key = Secp256r1SecretKey::from_slice(&secret_key_bytes).unwrap();
    let signing_key = Secp256r1SigningKey::from(&secret_key);

    let pubkey = secret_key
        .public_key()
        .to_encoded_point(false)
        .to_bytes()
        .to_vec();
    let mut pubkey_slice = [0u8; 65];
    pubkey_slice.copy_from_slice(&pubkey);
    let public_key = BytesN::<65>::from_array(e, &pubkey_slice);

    let signature: Secp256r1Signature = signing_key.sign_prehash(&digest).unwrap();
    let sig_slice = signature.normalize_s().unwrap_or(signature).to_bytes();
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&sig_slice);
    let signature = BytesN::<64>::from_array(e, &sig);

    (public_key, signature)
}

fn encode_authenticator_data(e: &Env, flags: u8) -> Bytes {
    let mut data = [0u8; 37];
    data[32] = flags;
    Bytes::from_array(e, &data)
}

fn encode_client_data(e: &Env, challenge: &str) -> Bytes {
    let json_str = std::format!(
        r#"{{"type":"webauthn.get","challenge":"{challenge}","origin":"https://sta.example","crossOrigin":false}}"#
    );
    Bytes::from_slice(e, json_str.as_bytes())
}

fn webauthn_fixture(e: &Env) -> (BytesN<32>, BytesN<65>, WebAuthnSigData) {
    // Arbitrary 32-byte "signature payload" — in production this is the
    // Soroban __check_auth signature_payload (transaction hash).
    let payload: [u8; 32] =
        hex!("4bb7a8b99609b0b8b1d534694bb1f31f129138a2f2a11f8e8702eedbb792922e");

    let mut encoded_challenge = [0u8; 43];
    base64_url_encode(&mut encoded_challenge, &payload);
    let challenge = std::str::from_utf8(&encoded_challenge).unwrap();

    let client_data = encode_client_data(e, challenge);
    let authenticator_data = encode_authenticator_data(
        e,
        AUTH_DATA_FLAGS_UP | AUTH_DATA_FLAGS_UV | AUTH_DATA_FLAGS_BE | AUTH_DATA_FLAGS_BS,
    );

    let mut msg = authenticator_data.clone();
    msg.extend_from_array(&e.crypto().sha256(&client_data).to_array());
    let digest = e.crypto().sha256(&msg).to_array();

    let (public_key, signature) = sign_secp256r1(e, digest);

    let sig_data = WebAuthnSigData {
        signature,
        authenticator_data,
        client_data,
    };
    let signature_payload = BytesN::<32>::from_array(e, &payload);

    (signature_payload, public_key, sig_data)
}

#[test]
fn webauthn_path_verifies_real_passkey_signature() {
    let e = Env::default();
    let client = setup(&e);

    let (signature_payload, public_key, sig_data) = webauthn_fixture(&e);

    let hash: Bytes = signature_payload.into_val(&e);
    let key_data: Val = public_key.into_val(&e);
    let sig_val: Val = sig_data.into_val(&e);

    assert!(client.verify(&hash, &key_data, &sig_val));
}

#[test]
#[should_panic]
fn webauthn_path_rejects_tampered_signature() {
    let e = Env::default();
    let client = setup(&e);

    let (signature_payload, public_key, mut sig_data) = webauthn_fixture(&e);
    // Flip a byte of the real signature — must not verify.
    let mut tampered = sig_data.signature.to_array();
    tampered[0] ^= 0xFF;
    sig_data.signature = BytesN::from_array(&e, &tampered);

    let hash: Bytes = signature_payload.into_val(&e);
    let key_data: Val = public_key.into_val(&e);
    let sig_val: Val = sig_data.into_val(&e);

    client.verify(&hash, &key_data, &sig_val);
}

#[test]
fn webauthn_canonicalize_key_strips_credential_id_suffix() {
    let e = Env::default();
    let client = setup(&e);

    let pub_key = Bytes::from_array(&e, &[7u8; 65]);
    let mut key_with_suffix = pub_key.clone();
    key_with_suffix.extend_from_array(&[9u8; 16]);

    let key_val: Val = key_with_suffix.into_val(&e);
    let canonical: Bytes = client.canonicalize_key(&key_val).try_into_val(&e).unwrap();

    assert_eq!(canonical, pub_key);
}

#[test]
fn ed25519_path_verifies_real_wallet_signature() {
    let e = Env::default();
    let client = setup(&e);

    let secret_bytes: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = signing_key.verifying_key();

    let payload_bytes: [u8; 32] = [42u8; 32];
    let signature = signing_key.sign(&payload_bytes);

    let hash = Bytes::from_array(&e, &payload_bytes);
    let public_key = BytesN::<32>::from_array(&e, &verifying_key.to_bytes());
    let sig = BytesN::<64>::from_array(&e, &signature.to_bytes());

    let key_val: Val = public_key.into_val(&e);
    let sig_val: Val = sig.into_val(&e);

    assert!(client.verify(&hash, &key_val, &sig_val));

    let canonical: Bytes = client.canonicalize_key(&key_val).try_into_val(&e).unwrap();
    let expected = Bytes::from_array(&e, &verifying_key.to_bytes());
    assert_eq!(canonical, expected);
}

#[test]
fn batch_canonicalize_preserves_order_across_schemes() {
    let e = Env::default();
    let client = setup(&e);

    let ed25519_key = Bytes::from_array(&e, &[3u8; 32]);
    let passkey_key = Bytes::from_array(&e, &[4u8; 65]);

    let keys: Vec<Val> = vec![&e, ed25519_key.into_val(&e), passkey_key.into_val(&e)];
    let results = client.batch_canonicalize_key(&keys);

    assert_eq!(results.len(), 2);
    let first: Bytes = results.get(0).unwrap().try_into_val(&e).unwrap();
    let second: Bytes = results.get(1).unwrap().try_into_val(&e).unwrap();
    assert_eq!(first, Bytes::from_array(&e, &[3u8; 32]));
    assert_eq!(second, Bytes::from_array(&e, &[4u8; 65]));
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn rejects_unsupported_key_length() {
    let e = Env::default();
    let client = setup(&e);

    let bogus_key = Bytes::from_array(&e, &[1u8; 10]);
    let sig = BytesN::<64>::from_array(&e, &[0u8; 64]);
    let hash = Bytes::from_array(&e, &[0u8; 32]);

    let key_val: Val = bogus_key.into_val(&e);
    let sig_val: Val = sig.into_val(&e);

    client.verify(&hash, &key_val, &sig_val);
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn canonicalize_key_rejects_unsupported_key_length() {
    let e = Env::default();
    let client = setup(&e);

    let bogus_key = Bytes::from_array(&e, &[1u8; 10]);
    let key_val: Val = bogus_key.into_val(&e);
    client.canonicalize_key(&key_val);
}

#[test]
#[should_panic(expected = "Error(Contract, #5001)")]
fn ed25519_path_rejects_malformed_signature_encoding() {
    let e = Env::default();
    let client = setup(&e);

    let public_key = BytesN::<32>::from_array(&e, &[7u8; 32]);
    // A Bytes value (not a BytesN<64>) fails to decode as an Ed25519 signature.
    let bogus_sig = Bytes::from_array(&e, &[1u8; 10]);

    let hash = Bytes::from_array(&e, &[0u8; 32]);
    let key_val: Val = public_key.into_val(&e);
    let sig_val: Val = bogus_sig.into_val(&e);

    client.verify(&hash, &key_val, &sig_val);
}

#[test]
#[should_panic(expected = "Error(Contract, #5001)")]
fn webauthn_path_rejects_malformed_sig_data_encoding() {
    let e = Env::default();
    let client = setup(&e);

    let public_key = BytesN::<65>::from_array(&e, &[7u8; 65]);
    // A bare Bytes value (not a WebAuthnSigData struct) fails to decode.
    let bogus_sig_data = Bytes::from_array(&e, &[1u8; 10]);

    let hash = Bytes::from_array(&e, &[0u8; 32]);
    let key_val: Val = public_key.into_val(&e);
    let sig_val: Val = bogus_sig_data.into_val(&e);

    client.verify(&hash, &key_val, &sig_val);
}
