#![no_std]

//! Thin, stateless wrapper contract exposing `stellar-accounts`'
//! WebAuthn/passkey and Ed25519 verification as a deployable `Verifier`
//! contract (see `stellar_accounts::verifiers::Verifier`).
//!
//! This contract contains zero custom cryptography. Every check
//! (`webauthn::verify`, `ed25519::verify`, `canonicalize_key`) delegates
//! directly to the audited `stellar-accounts` crate. Its only job is to
//! satisfy the cross-contract `VerifierClientInterface` shape
//! (`verify`/`canonicalize_key`/`batch_canonicalize_key` over `Val`) so a
//! `smart_account` contract can register `Signer::External(this_address,
//! key_bytes)` signers for passkeys, and route Ed25519 external signers
//! through the same reusable instance.
//!
//! One instance of this contract is deployed once and referenced by address
//! from any number of `smart_account` signer records — it is multi-tenant
//! and holds no per-account state.
//!
//! Key data encoding distinguishes the two schemes by length:
//! - 32 bytes -> Ed25519 public key
//! - 65 (or 65 + credential-id suffix) bytes -> WebAuthn/secp256r1 public key
//!
//! Signature data is expected to be the XDR encoding of either a
//! `BytesN<64>` (Ed25519) or a `stellar_accounts::verifiers::webauthn::WebAuthnSigData`
//! (WebAuthn), decoded via `TryFromVal`.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, Bytes, BytesN, Env, TryFromVal, Val,
    Vec,
};
use stellar_accounts::verifiers::{
    ed25519,
    utils::extract_from_bytes,
    webauthn::{self, WebAuthnSigData},
};

#[contract]
pub struct WebauthnVerifier;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VerifierError {
    UnsupportedKeyLength = 5000,
    MalformedSignature = 5001,
}

#[contractimpl]
impl WebauthnVerifier {
    /// Verifies `sig_data` against `key_data` for `hash`, dispatching to the
    /// Ed25519 or WebAuthn/secp256r1 scheme based on `key_data` length.
    pub fn verify(e: Env, hash: Bytes, key_data: Val, sig_data: Val) -> bool {
        let key_bytes = Bytes::try_from_val(&e, &key_data)
            .unwrap_or_else(|_| panic_with_error!(&e, VerifierError::UnsupportedKeyLength));

        match key_bytes.len() {
            32 => {
                let public_key: BytesN<32> = extract_from_bytes(&e, &key_bytes, 0..32)
                    .unwrap_or_else(|| panic_with_error!(&e, VerifierError::UnsupportedKeyLength));
                let signature: BytesN<64> = BytesN::try_from_val(&e, &sig_data)
                    .unwrap_or_else(|_| panic_with_error!(&e, VerifierError::MalformedSignature));
                ed25519::verify(&e, &hash, &public_key, &signature)
            }
            len if len >= 65 => {
                let pub_key: BytesN<65> = extract_from_bytes(&e, &key_bytes, 0..65)
                    .unwrap_or_else(|| panic_with_error!(&e, VerifierError::UnsupportedKeyLength));
                let sig: WebAuthnSigData = WebAuthnSigData::try_from_val(&e, &sig_data)
                    .unwrap_or_else(|_| panic_with_error!(&e, VerifierError::MalformedSignature));
                webauthn::verify(&e, &hash, &pub_key, &sig)
            }
            _ => panic_with_error!(&e, VerifierError::UnsupportedKeyLength),
        }
    }

    /// Canonicalizes `key_data` to its cryptographic-identity bytes, so the
    /// same underlying key cannot be registered twice under different
    /// encodings (e.g. WebAuthn key + trailing credential-id suffix).
    pub fn canonicalize_key(e: Env, key_data: Val) -> Bytes {
        let key_bytes = Bytes::try_from_val(&e, &key_data)
            .unwrap_or_else(|_| panic_with_error!(&e, VerifierError::UnsupportedKeyLength));

        match key_bytes.len() {
            32 => {
                let public_key: BytesN<32> = extract_from_bytes(&e, &key_bytes, 0..32)
                    .unwrap_or_else(|| panic_with_error!(&e, VerifierError::UnsupportedKeyLength));
                ed25519::canonicalize_key(&e, &public_key)
            }
            len if len >= 65 => webauthn::canonicalize_key(&e, &key_bytes),
            _ => panic_with_error!(&e, VerifierError::UnsupportedKeyLength),
        }
    }

    /// Batched variant of [`Self::canonicalize_key`], preserving input order.
    pub fn batch_canonicalize_key(e: Env, key_data: Vec<Val>) -> Vec<Bytes> {
        Vec::from_iter(
            &e,
            key_data
                .iter()
                .map(|k| Self::canonicalize_key(e.clone(), k)),
        )
    }
}

#[cfg(test)]
mod test;
