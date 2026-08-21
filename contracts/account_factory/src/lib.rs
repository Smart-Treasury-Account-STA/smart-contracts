#![no_std]

//! AccountFactory: self-service deployment for a full Smart Treasury
//! Account in one transaction.
//!
//! ## Why this exists
//!
//! Before this contract, standing up a treasury meant seven separate
//! contract deployments plus ten-plus `stellar contract invoke` calls, run
//! by hand in a fixed order (`scripts/deploy_testnet.sh`) — workable for
//! this project's own testnet, not something a new user could realistically
//! do themselves. `deploy_account` collapses that into one call: it deploys
//! fresh `policy_engine`, `intent_registry`, `recovery_manager`,
//! `transfer_adapter`, and `split_adapter` instances, wires them together,
//! and returns a ready-to-use `smart_account` address — with adapters
//! already bound (no timelock; see `smart_account::initialize`'s doc
//! comment for why the *first* binding is safe to do immediately) and
//! `intent_registry` already initialized (via `smart_account::initialize`'s
//! own invoker-shortcut bootstrap — no separate hand-built authorization
//! step, see that function's doc comment).
//!
//! `webauthn_verifier` is deliberately **not** deployed here — it is
//! stateless and holds no per-treasury configuration (see its own module
//! docs), so this workspace already treats one shared instance as correct;
//! a passkey signer just references its address directly inside
//! `Signer::External(verifier_address, key_bytes)` in `initial_signers`.
//!
//! ## Scope: one private stack per treasury, not shared infrastructure
//!
//! Every treasury deployed here gets its **own** `policy_engine`,
//! `intent_registry`, and `recovery_manager` — the same 1:1 pinning
//! `smart_account::initialize` already required before this contract
//! existed. This is deliberately the simpler of two designs considered:
//! the alternative, a *shared, multi-tenant* `policy_engine`/
//! `intent_registry`/`recovery_manager` serving every treasury (folding
//! each treasury's address into their storage keys), is real infrastructure
//! savings at scale but requires redesigning storage keys across three
//! already-shipped, already-reviewed contracts — a separate, larger
//! decision, not something to fold into a factory silently. See
//! `docs/SMART_ACCOUNT_IMPROVEMENT_PROPOSAL.md` Priority 2 for the
//! multi-tenant option and why it was left for a dedicated decision instead
//! of being decided here.
//!
//! ## Authorization model
//!
//! `deploy_account` takes an explicit `caller`, who becomes `admin` for
//! `policy_engine`/`recovery_manager`/`transfer_adapter`/`split_adapter`
//! and `owner` for `smart_account` — the same single-key-holds-every-admin
//! -role default `scripts/deploy_testnet.sh` already uses today.
//! Distributing that role across a multisig afterward is exactly what
//! `contracts/governance_account` is for (see
//! `docs/GOVERNANCE_MULTISIG_DESIGN.md`); this contract does not duplicate
//! that, it just gets a new treasury to the same single-key starting point
//! the existing manual flow already produces, in one call instead of ten.
//!
//! `caller.require_auth()` is called explicitly once in this function's own
//! body, and again, separately, inside each sub-contract's own `initialize`
//! (`policy_engine`, `recovery_manager`, `transfer_adapter`,
//! `split_adapter`, `smart_account` each check `admin`/`owner`
//! `.require_auth()` where that address is `caller`) — six
//! `(contract, function, args)` nodes in total, not one. Each still needs
//! its own matching authorization entry; this is *not* a case where one
//! signature covers every node for free, unlike `intent_registry`'s
//! bootstrap below. What it does *not* need is `caller` to be a
//! not-yet-existing signer set the way `smart_account` was mid-bootstrap:
//! `caller` is a real, already-existing Stellar address, so a wallet/SDK
//! can produce all six entries from a single simulate-and-sign flow — the
//! ordinary multi-node authorization case Soroban tooling already handles,
//! not a special chicken-and-egg problem like `intent_registry`'s.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, symbol_short, Address,
    Bytes, BytesN, Env, Map, Symbol, Val, Vec,
};
use stellar_accounts::smart_account::Signer;

#[contract]
pub struct AccountFactory;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccountFactoryError {
    AlreadyInitialized = 10000,
    NotInitialized = 10001,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmHashes {
    pub policy_engine: BytesN<32>,
    pub intent_registry: BytesN<32>,
    pub recovery_manager: BytesN<32>,
    pub transfer_adapter: BytesN<32>,
    pub split_adapter: BytesN<32>,
    pub smart_account: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployedAccount {
    pub smart_account: Address,
    pub policy_engine: Address,
    pub intent_registry: Address,
    pub recovery_manager: Address,
    pub transfer_adapter: Address,
    pub split_adapter: Address,
}

#[contractevent(topics = ["init"])]
pub struct Initialized {
    #[topic]
    pub admin: Address,
}

#[contractevent(topics = ["wasm_set"])]
pub struct WasmHashesUpdated {}

#[contractevent(topics = ["deployed"])]
pub struct AccountDeployed {
    #[topic]
    pub caller: Address,
    #[topic]
    pub smart_account: Address,
}

#[derive(Clone)]
#[soroban_sdk::contracttype]
enum DataKey {
    Admin,
    WasmHashes,
}

/// See `docs/TECHNICAL_ARCHITECTURE.md` §16 ("Production requirement").
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

#[contractimpl]
impl AccountFactory {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_fctr")
    }

    /// Permissionless TTL maintenance — extending TTL creates no authority,
    /// matching the reasoning already used by every other contract in this
    /// workspace (`docs/TECHNICAL_ARCHITECTURE.md` §16.1).
    pub fn extend_instance_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        wasm_hashes: WasmHashes,
    ) -> Result<(), AccountFactoryError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(AccountFactoryError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::WasmHashes, &wasm_hashes);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        Initialized { admin }.publish(&env);
        Ok(())
    }

    /// Updates the WASM hashes used for every *future* `deploy_account`
    /// call. Admin-gated and deliberately mutable — unlike `smart_account`'s
    /// own pinned subordinate-module addresses (which protect one already
    /// -deployed treasury's established trust), this only ever affects
    /// treasuries deployed *after* the update; already-deployed treasuries
    /// are unaffected, since their addresses were already fixed at their
    /// own deploy time. Shared infrastructure code getting patched over
    /// time is the ordinary case, not a residual risk to name.
    pub fn set_wasm_hashes(env: Env, wasm_hashes: WasmHashes) -> Result<(), AccountFactoryError> {
        let admin = ensure_admin(&env)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::WasmHashes, &wasm_hashes);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        WasmHashesUpdated {}.publish(&env);
        Ok(())
    }

    pub fn get_wasm_hashes(env: Env) -> Result<WasmHashes, AccountFactoryError> {
        env.storage()
            .instance()
            .get(&DataKey::WasmHashes)
            .ok_or(AccountFactoryError::NotInitialized)
    }

    /// Deploys and wires a complete, ready-to-use treasury stack in one
    /// call. See the module doc comment for the authorization model and
    /// what "ready to use" means (adapters already bound, no timelock;
    /// `intent_registry` already initialized).
    ///
    /// `salt` only needs to be unique per `caller` — sub-contract addresses
    /// are derived from `sha256(salt || per-contract tag)`, so one caller
    /// -chosen salt is enough to place all six deployments deterministically
    /// and collision-free against each other.
    ///
    /// `executor` is `intent_registry`'s scheduled-payment relayer address
    /// (see `smart_account::initialize`'s doc comment on `initial_executor`
    /// for why this needs to be explicit rather than defaulted). Pass
    /// `caller` again if there is no separate relayer identity yet.
    pub fn deploy_account(
        env: Env,
        caller: Address,
        salt: BytesN<32>,
        initial_signers: Vec<Signer>,
        initial_policies: Map<Address, Val>,
        guardian_threshold: u32,
        executor: Address,
    ) -> Result<DeployedAccount, AccountFactoryError> {
        caller.require_auth();
        let hashes: WasmHashes = env
            .storage()
            .instance()
            .get(&DataKey::WasmHashes)
            .ok_or(AccountFactoryError::NotInitialized)?;

        let policy_engine = deploy(&env, &sub_salt(&env, &salt, "pol"), &hashes.policy_engine);
        PolicyEngineClient::new(&env, &policy_engine).initialize(&caller);

        let intent_registry = deploy(&env, &sub_salt(&env, &salt, "int"), &hashes.intent_registry);
        // Left uninitialized: `smart_account::initialize` (below)
        // initializes it directly via the invoker-shortcut.

        let recovery_manager = deploy(
            &env,
            &sub_salt(&env, &salt, "rec"),
            &hashes.recovery_manager,
        );
        RecoveryManagerClient::new(&env, &recovery_manager)
            .initialize(&caller, &guardian_threshold);

        let smart_account = deploy(&env, &sub_salt(&env, &salt, "acc"), &hashes.smart_account);

        let transfer_adapter = deploy(
            &env,
            &sub_salt(&env, &salt, "xfer"),
            &hashes.transfer_adapter,
        );
        TransferAdapterClient::new(&env, &transfer_adapter).initialize(&caller, &smart_account);

        let split_adapter = deploy(&env, &sub_salt(&env, &salt, "splt"), &hashes.split_adapter);
        SplitAdapterClient::new(&env, &split_adapter).initialize(&caller, &smart_account);

        let mut initial_adapters = Map::new(&env);
        initial_adapters.set(Symbol::new(&env, "transfer"), transfer_adapter.clone());
        initial_adapters.set(Symbol::new(&env, "split"), split_adapter.clone());

        SmartAccountClient::new(&env, &smart_account).initialize(
            &caller,
            &initial_signers,
            &initial_policies,
            &SmartAccountInitConfig {
                policy_engine: policy_engine.clone(),
                intent_registry: intent_registry.clone(),
                recovery_manager: recovery_manager.clone(),
                initial_adapters,
                initial_executor: executor,
            },
        );

        AccountDeployed {
            caller,
            smart_account: smart_account.clone(),
        }
        .publish(&env);

        Ok(DeployedAccount {
            smart_account,
            policy_engine,
            intent_registry,
            recovery_manager,
            transfer_adapter,
            split_adapter,
        })
    }
}

fn ensure_admin(env: &Env) -> Result<Address, AccountFactoryError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(AccountFactoryError::NotInitialized)
}

fn sub_salt(env: &Env, base: &BytesN<32>, tag: &str) -> BytesN<32> {
    let mut bytes = Bytes::from_array(env, &base.to_array());
    bytes.append(&Bytes::from_slice(env, tag.as_bytes()));
    env.crypto().sha256(&bytes).into()
}

fn deploy(env: &Env, salt: &BytesN<32>, wasm_hash: &BytesN<32>) -> Address {
    env.deployer()
        .with_current_contract(salt.clone())
        .deploy_v2(wasm_hash.clone(), ())
}

// ################## SIBLING CONTRACT CLIENTS ##################
// Local, minimal client trait declarations rather than depending on each
// sibling crate directly — the same pattern `contracts/smart_account`
// already uses for these exact contracts, and for the same reason: only
// the ABI needs to match, not a full crate dependency.

#[soroban_sdk::contractclient(name = "PolicyEngineClient")]
#[allow(unused)]
trait PolicyEngineInterface {
    fn initialize(env: Env, admin: Address);
}

#[soroban_sdk::contractclient(name = "RecoveryManagerClient")]
#[allow(unused)]
trait RecoveryManagerInterface {
    fn initialize(env: Env, admin: Address, guardian_threshold: u32);
}

#[soroban_sdk::contractclient(name = "TransferAdapterClient")]
#[allow(unused)]
trait TransferAdapterInterface {
    fn initialize(env: Env, admin: Address, smart_account: Address);
}

#[soroban_sdk::contractclient(name = "SplitAdapterClient")]
#[allow(unused)]
trait SplitAdapterInterface {
    fn initialize(env: Env, admin: Address, smart_account: Address);
}

/// Local mirror of `smart_account::InitConfig` — Soroban struct types are
/// decoded structurally by field name/type (see that type's own doc
/// comment), so this does not require a dependency on `sta-smart-account`
/// itself, matching this module's whole "local client, not a crate dep"
/// pattern.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartAccountInitConfig {
    pub policy_engine: Address,
    pub intent_registry: Address,
    pub recovery_manager: Address,
    pub initial_adapters: Map<Symbol, Address>,
    pub initial_executor: Address,
}

#[soroban_sdk::contractclient(name = "SmartAccountClient")]
#[allow(unused)]
trait SmartAccountInterface {
    fn initialize(
        env: Env,
        owner: Address,
        initial_signers: Vec<Signer>,
        initial_policies: Map<Address, Val>,
        config: SmartAccountInitConfig,
    );
}

#[cfg(test)]
mod test;
