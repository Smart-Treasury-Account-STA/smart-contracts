#![no_std]

//! TransferAdapter: the only contract in this workspace that ever calls a
//! Stellar Asset Contract's `transfer`. It is deliberately narrow — one
//! action shape (single SAC transfer out of the configured `smart_account`),
//! nothing else.
//!
//! Security rule (see `docs/TECHNICAL_ARCHITECTURE.md` §6.6): "No adapter
//! may move treasury assets unless SmartAccount has preauthorized the exact
//! operation." This is enforced cryptographically, not by trusting the
//! immediate caller's identity: `execute_transfer` calls
//! `smart_account.require_auth()` on the *configured* treasury address. In
//! Soroban, a nested `require_auth()` call is satisfied only if this exact
//! (contract, function, args) node was part of the sub-invocation tree the
//! signer(s) originally authorized — so a relayer or compromised caller
//! cannot swap the token, destination, or amount between wallet simulation
//! and submission; doing so produces a different tree node with no matching
//! authorization.

use soroban_sdk::{
    contract, contracterror, contractimpl, symbol_short, token::TokenClient, Address, Env,
    MuxedAddress, Symbol,
};

/// See `docs/TECHNICAL_ARCHITECTURE.md` §16 ("Production requirement").
/// This contract's only state is the configured treasury address in
/// instance storage; extending it on every entry point call keeps a
/// long-lived, rarely-reconfigured adapter from archiving.
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

#[contract]
pub struct TransferAdapter;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TransferAdapterError {
    AlreadyInitialized = 6000,
    NotInitialized = 6001,
    InvalidAmount = 6002,
}

#[contractimpl]
impl TransferAdapter {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_xfer")
    }

    /// Binds this adapter instance to exactly one treasury `SmartAccount`.
    /// An adapter instance is not shared across unrelated treasuries: doing
    /// so would let one treasury's approved transfer authorize funds moving
    /// out of a different treasury with the same adapter configured.
    pub fn initialize(
        env: Env,
        admin: Address,
        smart_account: Address,
    ) -> Result<(), TransferAdapterError> {
        if env.storage().instance().has(&DataKey::SmartAccount) {
            return Err(TransferAdapterError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::SmartAccount, &smart_account);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        env.events()
            .publish((symbol_short!("init"),), smart_account);
        Ok(())
    }

    /// Executes a single SAC transfer of `amount` of `token` from the
    /// configured treasury to `to`. Requires the treasury's own
    /// authorization for this exact call.
    pub fn execute_transfer(
        env: Env,
        token: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), TransferAdapterError> {
        let smart_account = ensure_initialized(&env)?;
        smart_account.require_auth();

        if amount <= 0 {
            return Err(TransferAdapterError::InvalidAmount);
        }

        let token_client = TokenClient::new(&env, &token);
        let to_muxed: MuxedAddress = to.clone().into();
        token_client.transfer(&smart_account, &to_muxed, &amount);

        env.events()
            .publish((symbol_short!("xfer"),), (token, to, amount));
        Ok(())
    }
}

#[derive(Clone)]
#[soroban_sdk::contracttype]
enum DataKey {
    SmartAccount,
}

fn ensure_initialized(env: &Env) -> Result<Address, TransferAdapterError> {
    let smart_account = env
        .storage()
        .instance()
        .get(&DataKey::SmartAccount)
        .ok_or(TransferAdapterError::NotInitialized)?;
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    Ok(smart_account)
}

#[cfg(test)]
mod test;
