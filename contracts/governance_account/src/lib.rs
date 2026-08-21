#![no_std]

//! GovernanceAccount: a minimal N-of-M multisig, for use as the `owner`
//! of `smart_account` and/or the `admin` of `recovery_manager`/
//! `policy_engine`, instead of a single bare keypair.
//!
//! ## Why this closes the single-key governance gap with zero changes
//! elsewhere
//!
//! `owner`/`admin` throughout this workspace are plain `soroban_sdk::Address`
//! values, checked with an ordinary `some_address.require_auth()` call (see
//! `stellar-access-0.7.2/src/ownable/storage.rs::enforce_owner_auth`).
//! Soroban does not distinguish a classic Ed25519 account from a contract
//! address implementing `CustomAccountInterface` at that call site. Pointing
//! `owner`/`admin` at this contract's address instead of a bare keypair
//! distributes that authority across N-of-M signers without touching
//! `smart_account` or `recovery_manager` at all — see
//! `docs/GOVERNANCE_MULTISIG_DESIGN.md` for the full design rationale and
//! the alternatives considered.
//!
//! ## Deliberately the same composition `smart_account` itself uses
//!
//! This contract composes OpenZeppelin's `SmartAccount` (context rules,
//! signer registry, `__check_auth`) — the exact same primitive
//! `contracts/smart_account` uses for its own payment-approval
//! authorization. That choice is deliberate, not incidental: it means a
//! client that already knows how to build the `AuthPayload`/nested-signer
//! authorization for a treasury payment (see `docs/DAPP_INTEGRATION_SPEC.md`
//! §5) needs no second, parallel authorization system to approve a
//! governance action — same mechanics, same client code, just pointed at
//! this contract's address and a different target call.
//!
//! Deliberately minimal otherwise: no `Ownable`, no `Pausable`, no treasury
//! logic, no `ExecutionEntryPoint` (nothing here needs to *initiate* calls
//! as itself — its only job is answering `require_auth()` checks other
//! contracts make against it). Attach `contracts/threshold_policy` to its
//! context rule for real N-of-M enforcement; with no policy attached, the
//! same "every listed signer must approve" default `stellar-accounts`
//! applies everywhere else in this workspace applies here too.
//!
//! ## Recovery, deliberately not built here
//!
//! Unlike `smart_account`, which has `recovery_manager` as an explicit
//! escape hatch for a compromised or lost owner, this contract has no
//! recovery path of its own. If its own signer set or threshold is ever
//! misconfigured into a state nobody can satisfy, there is no way to route
//! around it — even `transfer_ownership` on whatever this contract governs
//! itself needs this contract's own `require_auth()` to succeed first, so a
//! bricked governance account cannot repoint itself at a replacement.
//!
//! The mitigation is operational, not code: keep the threshold comfortably
//! below the full signer count (e.g. 3-of-5, not 5-of-5) and keep enough
//! signers registered that simultaneous loss is implausible — exactly how
//! real-world multisigs are operated everywhere, on any chain. Building a
//! second recovery subsystem for the governance layer itself was judged not
//! worth the added complexity given that standard mitigation is available
//! and cheap. Revisit this if operational experience says otherwise.

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractevent, contractimpl, contracttype,
    crypto::Hash,
    symbol_short, Address, Env, Map, String, Symbol, Val, Vec,
};
use stellar_accounts::smart_account::{
    add_context_rule, do_check_auth, AuthPayload, ContextRule, ContextRuleType, Signer,
    SmartAccount as OzSmartAccountTrait, SmartAccountError as OzSmartAccountError,
};

#[contract]
pub struct GovernanceAccount;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GovernanceAccountError {
    AlreadyInitialized = 9000,
    NotInitialized = 9001,
}

#[contractevent(topics = ["init"])]
pub struct Initialized {
    pub signer_count: u32,
}

// See `docs/TECHNICAL_ARCHITECTURE.md` §16 ("Production requirement").
// This contract's own instance storage (just the `Initialized` flag) is
// expected to go untouched for long stretches between governance actions —
// same reasoning as `smart_account`'s identical constants.
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

#[contractimpl]
impl GovernanceAccount {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_gov")
    }

    /// Permissionless TTL maintenance — extending TTL creates no authority,
    /// same reasoning as `smart_account::extend_instance_ttl` and
    /// `recovery_manager::extend_ttl`.
    pub fn extend_instance_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    }

    /// Bootstraps the governance account: seeds a single `Default` context
    /// rule with the founding signers and, normally, a `threshold_policy`
    /// installed for real N-of-M enforcement (pass its address in
    /// `initial_policies` with a `SimpleThresholdAccountParams { threshold }`
    /// install value — installing no policy at all falls back to
    /// `stellar-accounts`' own default of requiring every listed signer,
    /// which is only appropriate for a single-signer governance account).
    ///
    /// Calls the raw `add_context_rule` storage primitive directly instead
    /// of the `SmartAccount` trait's own entrypoint, which requires an
    /// already-authorized signer to exist — impossible to satisfy before
    /// the very first one is registered. Mirrors
    /// `smart_account::initialize`'s identical bootstrapping problem and
    /// solution exactly.
    pub fn initialize(
        env: Env,
        initial_signers: Vec<Signer>,
        initial_policies: Map<Address, Val>,
    ) -> Result<(), GovernanceAccountError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(GovernanceAccountError::AlreadyInitialized);
        }

        add_context_rule(
            &env,
            &ContextRuleType::Default,
            &String::from_str(&env, "governance"),
            None,
            &initial_signers,
            &initial_policies,
        );

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);

        Initialized {
            signer_count: initial_signers.len(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn status(env: Env) -> Result<bool, GovernanceAccountError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            Ok(true)
        } else {
            Err(GovernanceAccountError::NotInitialized)
        }
    }
}

// ################## OZ TRAIT COMPOSITION ##################
// Zero custom authorization logic below this point — every method is an
// unmodified OZ default or a one-line delegation into `do_check_auth`,
// identical in spirit to `contracts/smart_account`'s own composition.

#[contractimpl(contracttrait)]
impl OzSmartAccountTrait for GovernanceAccount {}

#[contractimpl]
impl CustomAccountInterface for GovernanceAccount {
    type Signature = AuthPayload;
    type Error = OzSmartAccountError;

    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: AuthPayload,
        auth_contexts: Vec<Context>,
    ) -> Result<(), OzSmartAccountError> {
        do_check_auth(&env, &signature_payload, &signatures, &auth_contexts)
    }
}

#[cfg(test)]
mod test;
