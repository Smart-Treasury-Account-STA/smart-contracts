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
//!
//! Security review finding (`docs/SECURITY_REVIEW_STRICT.md` finding 29):
//! this used to call the SAC's plain `transfer(from=smart_account, ...)`,
//! which requires `smart_account.require_auth()` *again*, independently of
//! the check above — fine when `smart_account`'s own top-level entrypoint
//! already opened an authorization session covering this exact node (the
//! interactive payment path), but impossible for a permissionless
//! entrypoint that never authenticates `smart_account` at all
//! (`execute_scheduled_payment`): by the time this call is reached,
//! `smart_account`'s own frame is already active on the call stack, so
//! asking the host to invoke its `__check_auth` again is genuine contract
//! reentrancy, which Soroban rejects outright. `smart_account` now
//! `approve`s this adapter for the exact amount immediately before calling
//! it (satisfied by the invoker-shortcut, since `smart_account` is the
//! SAC's direct caller for that `approve` — no `__check_auth` invocation
//! at all, so no reentrancy risk), and this adapter draws via
//! `transfer_from` instead of `transfer`: `transfer_from`'s authorization
//! is on `spender` (this adapter's own address), which is *this contract's*
//! invoker-shortcut to claim, not `smart_account`'s.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, symbol_short, token::TokenClient,
    Address, Env, Symbol,
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

#[contractevent(topics = ["init"])]
pub struct Initialized {
    #[topic]
    pub smart_account: Address,
}

#[contractevent(topics = ["xfer"])]
pub struct TransferExecuted {
    #[topic]
    pub token: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractimpl]
impl TransferAdapter {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_xfer")
    }

    /// Security review finding: this contract's instance TTL was only ever
    /// extended as a side effect of `execute_transfer` succeeding, which
    /// itself requires the treasury's own authorization -- meaning a
    /// treasury that goes fully dormant (no payments at all, entirely
    /// plausible for e.g. a quarterly-disbursement treasury) had no way
    /// for *anyone* to keep this adapter's instance storage alive, since
    /// the one call that touches it can't be driven permissionlessly.
    /// Instance storage archival is not a soft failure -- an archived
    /// contract cannot be invoked at all without a separate restore
    /// operation. Every other contract in this workspace
    /// (`smart_account`, `recovery_manager`, `policy_engine`,
    /// `intent_registry`, `governance_account`) already has a
    /// permissionless maintenance entrypoint for exactly this reason;
    /// this one and `split_adapter` were the two gaps.
    pub fn extend_instance_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
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
        Initialized { smart_account }.publish(&env);
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
        token_client.transfer_from(
            &env.current_contract_address(),
            &smart_account,
            &to,
            &amount,
        );

        TransferExecuted { token, to, amount }.publish(&env);
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
