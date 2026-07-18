#![no_std]

//! SplitAdapter: splits a single SAC balance across multiple approved
//! recipients in one preauthorized action (e.g. revenue splits, payroll
//! batches). Same narrow-adapter security rule as `transfer_adapter`: every
//! call requires `smart_account.require_auth()` on the configured treasury
//! address, so the exact token/recipients/amounts are cryptographically
//! part of what was signed — a caller cannot add, remove, or resize a
//! recipient after the signer approved the split.

use soroban_sdk::{
    contract, contracterror, contractimpl, symbol_short, token::TokenClient, Address, Env,
    MuxedAddress, Symbol, Vec,
};

const MAX_RECIPIENTS: u32 = 20;

/// See `docs/TECHNICAL_ARCHITECTURE.md` §16 ("Production requirement").
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

#[contract]
pub struct SplitAdapter;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SplitAdapterError {
    AlreadyInitialized = 7000,
    NotInitialized = 7001,
    InvalidAmount = 7002,
    EmptySplit = 7003,
    TooManyRecipients = 7004,
    RecipientAmountLengthMismatch = 7005,
    TotalAmountOverflow = 7006,
}

#[contractimpl]
impl SplitAdapter {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_splt")
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        smart_account: Address,
    ) -> Result<(), SplitAdapterError> {
        if env.storage().instance().has(&DataKey::SmartAccount) {
            return Err(SplitAdapterError::AlreadyInitialized);
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

    /// Splits `token` from the treasury across `recipients[i]` receiving
    /// `amounts[i]`, for every index. Positionally aligned by index;
    /// `recipients` and `amounts` must have the same length, each amount
    /// must be positive, and the recipient count is bounded so a single
    /// call cannot be used to exhaust the transaction's resource budget or
    /// silently balloon the preauthorized action's blast radius.
    pub fn execute_split(
        env: Env,
        token: Address,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> Result<(), SplitAdapterError> {
        let smart_account = ensure_initialized(&env)?;
        smart_account.require_auth();

        if recipients.is_empty() {
            return Err(SplitAdapterError::EmptySplit);
        }
        if recipients.len() > MAX_RECIPIENTS {
            return Err(SplitAdapterError::TooManyRecipients);
        }
        if recipients.len() != amounts.len() {
            return Err(SplitAdapterError::RecipientAmountLengthMismatch);
        }

        let mut total: i128 = 0;
        for amount in amounts.iter() {
            if amount <= 0 {
                return Err(SplitAdapterError::InvalidAmount);
            }
            total = total
                .checked_add(amount)
                .ok_or(SplitAdapterError::TotalAmountOverflow)?;
        }

        let token_client = TokenClient::new(&env, &token);
        for (recipient, amount) in recipients.iter().zip(amounts.iter()) {
            let to_muxed: MuxedAddress = recipient.clone().into();
            token_client.transfer(&smart_account, &to_muxed, &amount);
        }

        env.events()
            .publish((symbol_short!("split"),), (token, recipients.len(), total));
        Ok(())
    }
}

#[derive(Clone)]
#[soroban_sdk::contracttype]
enum DataKey {
    SmartAccount,
}

fn ensure_initialized(env: &Env) -> Result<Address, SplitAdapterError> {
    let smart_account = env
        .storage()
        .instance()
        .get(&DataKey::SmartAccount)
        .ok_or(SplitAdapterError::NotInitialized)?;
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    Ok(smart_account)
}

#[cfg(test)]
mod test;
