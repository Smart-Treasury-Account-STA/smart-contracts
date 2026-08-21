#![no_std]

//! Thin wrapper exposing `stellar-accounts`' `simple_threshold` policy
//! (plain M-of-N signer count, equal weight per signer) as a deployable
//! `Policy` contract.
//!
//! Zero custom threshold-counting logic below: every method delegates
//! directly to `stellar_accounts::policies::simple_threshold`. Attach one
//! instance to a context rule on any `SmartAccount`-composed contract
//! (`smart_account`, `governance_account`, or a future one) via
//! `add_context_rule`'s `policies` map or `add_policy`, with install
//! parameter `SimpleThresholdAccountParams { threshold }`.
//!
//! One instance is deployed once and referenced by address from any
//! number of context rules across any number of contracts — it is
//! multi-tenant and holds no state beyond `(smart_account,
//! context_rule_id) -> threshold`.
//!
//! # Security Warning: Signer Set Divergence
//!
//! The threshold is **not** automatically revalidated when signers are
//! added to or removed from the context rule it's attached to — see
//! `simple_threshold`'s own module documentation for the full
//! explanation (a rule with 3 signers and threshold 3 that gains 2 more
//! signers silently becomes a 3-of-5 instead of staying a 3-of-3; a rule
//! that loses signers below its threshold becomes permanently
//! unsatisfiable). Call `set_threshold` explicitly, ideally in the same
//! transaction, whenever the attached rule's signer set changes.

use soroban_sdk::{auth::Context, contract, contractimpl, symbol_short, Address, Env, Symbol, Vec};
use stellar_accounts::{
    policies::{
        simple_threshold::{self, SimpleThresholdAccountParams, SimpleThresholdError},
        Policy,
    },
    smart_account::{ContextRule, Signer},
};

#[contract]
pub struct ThresholdPolicy;

#[contractimpl]
impl Policy for ThresholdPolicy {
    type AccountParams = SimpleThresholdAccountParams;

    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::enforce(
            e,
            &context,
            &authenticated_signers,
            &context_rule,
            &smart_account,
        );
    }

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::install(e, &install_params, &context_rule, &smart_account);
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        simple_threshold::uninstall(e, &context_rule, &smart_account);
    }
}

#[contractimpl]
impl ThresholdPolicy {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_thr")
    }

    /// Reads the currently configured threshold for a
    /// `(smart_account, context_rule_id)` pair.
    pub fn get_threshold(env: Env, context_rule_id: u32, smart_account: Address) -> u32 {
        simple_threshold::get_threshold(&env, context_rule_id, &smart_account)
    }

    /// Updates the threshold. Authorization is enforced inside
    /// `simple_threshold::set_threshold` itself
    /// (`smart_account.require_auth()`) — the caller must be the
    /// `smart_account` this policy is installed against, satisfying its
    /// own `__check_auth` the same way any other administrative call on
    /// it does. See the Signer Set Divergence warning above for when
    /// this must be called.
    pub fn set_threshold(
        env: Env,
        threshold: u32,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> Result<(), SimpleThresholdError> {
        simple_threshold::set_threshold(&env, threshold, &context_rule, &smart_account);
        Ok(())
    }
}

#[cfg(test)]
mod test;
