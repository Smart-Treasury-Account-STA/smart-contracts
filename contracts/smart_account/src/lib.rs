#![no_std]

//! SmartAccount: the treasury root authority.
//!
//! ## Passkeys are load-bearing here, not a bolted-on auth flow
//!
//! This contract does not implement `__check_auth` itself. It composes
//! `stellar-accounts::smart_account::SmartAccount` (context rules, signer
//! registry, policy attachment) and delegates `__check_auth` entirely to
//! that crate's `do_check_auth`. A "signer" registered against a context
//! rule is a `stellar_accounts::smart_account::Signer`:
//! `Delegated(Address)` for an ordinary Stellar/Freighter/xBull wallet, or
//! `External(verifier_address, key_bytes)` for a passkey — where
//! `verifier_address` points at `contracts/webauthn_verifier` (itself a
//! thin wrapper with zero custom cryptography, see that crate's docs).
//! Every governance and payment entrypoint on this contract funnels through
//! the *same* `e.current_contract_address().require_auth()` call, so
//! whether a given signer is a wallet key or a passkey is transparent to
//! every piece of treasury logic below — passkey integration is carried
//! through the whole authorization surface, not isolated to one "login"
//! step.
//!
//! Weighted/threshold signer math is also integrated, not built: attach a
//! `stellar_accounts::policies::weighted_threshold` (or `simple_threshold`)
//! Policy contract to a context rule at `initialize` and OZ's audited
//! threshold-counting logic decides whether a given signer set satisfies
//! the rule — this contract never re-implements "how many signers is
//! enough."
//!
//! ## What is genuinely new here (see Architecture Decision 4)
//!
//! Everything below `execute_*`'s `require_auth()` call: treasury-directional
//! risk policy pinned to a version (`policy_engine`), replay-safe scheduled
//! execution (`intent_registry`), narrow preauthorized adapters that cannot
//! be reinterpreted into a different action (`transfer_adapter`,
//! `split_adapter`), and recovery explicitly separated from spend authority
//! (`recovery_manager`, pulled — never pushed — into this contract).

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contractclient, contracterror, contractevent, contractimpl, contracttype,
    crypto::Hash,
    panic_with_error, symbol_short, Address, BytesN, Env, Map, String, Symbol, Val, Vec,
};
use stellar_access::ownable::{self, Ownable as OzOwnable};
use stellar_accounts::smart_account::{
    add_context_rule, do_check_auth, AuthPayload, ContextRule, ContextRuleType,
    ExecutionEntryPoint, Signer, SmartAccount as OzSmartAccountTrait,
    SmartAccountError as OzSmartAccountError,
};
use stellar_contract_utils::pausable::{self, Pausable as OzPausable};

#[contract]
pub struct SmartAccountTreasury;

// ################## TYPES ##################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountStatus {
    pub initialized: bool,
    pub paused: bool,
    pub frozen: bool,
    pub policy_version_hint: u32,
}

/// Groups `initialize`'s subordinate-module wiring into one argument —
/// clippy's `too_many_arguments` lint (max 7) is why this exists as a
/// struct rather than five more positional parameters; see `initialize`'s
/// doc comment for what each field actually does.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitConfig {
    pub policy_engine: Address,
    pub intent_registry: Address,
    pub recovery_manager: Address,
    pub initial_adapters: Map<Symbol, Address>,
    pub initial_executor: Address,
}

/// Local mirror of `recovery_manager::RecoveryRequest` — Soroban contract
/// types are decoded structurally by field name/type, so this does not
/// require a shared crate dependency on `recovery_manager` (which would
/// otherwise create a circular workspace dependency, since recovery is
/// pulled by this contract rather than pushed into it).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRequestView {
    pub request_id: BytesN<32>,
    pub replacement_owner: Address,
    pub earliest_ledger: u32,
    pub approvers: Vec<Address>,
    pub cancelled: bool,
    pub finalized: bool,
}

#[contractclient(name = "RecoveryManagerClient")]
#[allow(unused)]
trait RecoveryManagerInterface {
    fn request_status(env: Env, request_id: BytesN<32>) -> RecoveryRequestView;
    fn consume_guardian_freeze_request(env: Env) -> bool;
}

#[contractclient(name = "PolicyEngineClient")]
#[allow(unused)]
trait PolicyEngineInterface {
    fn validate_policy(env: Env, check: PolicyCheck);
    fn version(env: Env) -> u32;
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyCheck {
    pub operation: Symbol,
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
    pub expected_version: u32,
}

#[contractclient(name = "TransferAdapterClient")]
#[allow(unused)]
trait TransferAdapterInterface {
    fn execute_transfer(env: Env, token: Address, to: Address, amount: i128);
}

#[contractclient(name = "SplitAdapterClient")]
#[allow(unused)]
trait SplitAdapterInterface {
    fn execute_split(env: Env, token: Address, recipients: Vec<Address>, amounts: Vec<i128>);
}

/// A proposed adapter change awaiting its timelock. See
/// `propose_adapter_change`/`apply_adapter_change` — added in response to
/// an independent security review (`docs/SMART_CONTRACT_AUDIT_REPORT.md`
/// finding 3) that an owner (or whoever compromised that key) could
/// previously redirect execution routing immediately, with no delay for
/// anyone to notice or react.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdapterChange {
    pub adapter: Address,
    pub effective_ledger: u32,
}

#[contractevent(topics = ["init"])]
pub struct Initialized {
    #[topic]
    pub owner: Address,
}

#[contractevent(topics = ["adapterp"])]
pub struct AdapterChangeProposed {
    #[topic]
    pub operation: Symbol,
    pub adapter: Address,
    pub effective_ledger: u32,
}

#[contractevent(topics = ["adaptrc"])]
pub struct AdapterChangeCancelled {
    #[topic]
    pub operation: Symbol,
}

#[contractevent(topics = ["adapter"])]
pub struct AdapterChanged {
    #[topic]
    pub operation: Symbol,
    pub adapter: Address,
}

#[contractevent(topics = ["frozen"])]
pub struct Frozen {
    /// Distinguishes an owner-triggered `freeze()` from a guardian-pulled
    /// `apply_guardian_freeze()` — both used to publish an identical,
    /// data-free event, which made it impossible to tell which path
    /// actually froze the account from the event log alone.
    pub triggered_by_guardian: bool,
}

#[contractevent(topics = ["pay_ok"])]
pub struct TransferPaid {
    #[topic]
    pub asset: Address,
    #[topic]
    pub destination: Address,
    pub amount: i128,
    pub nonce: u64,
}

#[contractevent(topics = ["splt_ok"])]
pub struct SplitPaid {
    #[topic]
    pub asset: Address,
    pub recipient_count: u32,
    pub nonce: u64,
}

#[contractevent(topics = ["auto_ok"])]
pub struct ScheduledPaymentExecuted {
    #[topic]
    pub intent_id: BytesN<32>,
    pub child_sequence: u32,
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
}

#[contractevent(topics = ["recover"])]
pub struct RecoveryApplied {
    #[topic]
    pub request_id: BytesN<32>,
    pub replacement_owner: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
    PolicyEngine,
    IntentRegistry,
    RecoveryManager,
    Frozen,
    Adapter(Symbol),
    PendingAdapter(Symbol),
    UsedNonce(u64),
    AppliedRecovery(BytesN<32>),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SmartAccountTreasuryError {
    AlreadyInitialized = 8000,
    NotInitialized = 8001,
    Paused = 8002,
    Frozen = 8003,
    AdapterNotConfigured = 8004,
    NonceAlreadyUsed = 8005,
    InvalidAmount = 8006,
    RecipientAmountLengthMismatch = 8007,
    EmptySplit = 8008,
    RecoveryNotFinalized = 8009,
    RecoveryAlreadyApplied = 8010,
    Unauthorized = 8011,
    GuardianFreezeNotRequested = 8012,
    DuplicateRecipient = 8013,
    NoPendingAdapterChange = 8014,
    AdapterChangeDelayNotElapsed = 8015,
}

/// See `docs/TECHNICAL_ARCHITECTURE.md` §16 ("Production requirement").
/// Applies to this contract's own custom entries (instance config plus
/// per-nonce/per-recovery-request persistent keys) — OZ's composed signer/
/// context-rule/policy storage manages its own TTL internally.
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

const OP_TRANSFER: Symbol = symbol_short!("transfer");
const OP_SPLIT: Symbol = symbol_short!("split");

/// Delay between proposing and applying an adapter change — same ~1 day
/// window `recovery_manager` uses for its own timelocks
/// (`MIN_RECOVERY_DELAY_LEDGERS`, `GUARDIAN_ACTIVATION_DELAY_LEDGERS`), so a
/// compromised owner reconfiguring where funds are routed cannot make that
/// change take effect before anyone monitoring the account has a chance to
/// notice and react (e.g. by pausing, or by having guardians freeze).
const ADAPTER_CHANGE_DELAY_LEDGERS: u32 = 17280; // ~1 day

#[contractimpl]
impl SmartAccountTreasury {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_acct")
    }

    /// Permissionless TTL maintenance for this contract's instance storage
    /// (which `PendingAdapter` lives in, among other instance-scoped
    /// config) — extending TTL creates no authority, matching
    /// `recovery_manager::extend_ttl`'s reasoning. Independent security
    /// review finding: `propose_adapter_change` only bumps the instance TTL
    /// once, at proposal time; a proposal left unapplied long enough after
    /// its delay elapses, on an otherwise fully dormant treasury, could
    /// archive before `apply_adapter_change` is ever called. This lets
    /// anyone keep it alive without needing the owner to act.
    pub fn extend_instance_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    }

    /// Bootstraps the treasury: sets the owner, seeds a `Default` context
    /// rule with the founding signers/policies (e.g. wallet signers plus a
    /// `weighted_threshold` policy, or passkey `External` signers), pins the
    /// subordinate module addresses, and bootstraps `intent_registry` in the
    /// same call (see below).
    ///
    /// This calls the OZ storage primitives directly (`ownable::set_owner`,
    /// `add_context_rule`) instead of going through their `Ownable`/
    /// `SmartAccount` trait entrypoints, both of which require an
    /// already-authorized owner/signer to exist — impossible to satisfy
    /// before the very first signer is registered.
    ///
    /// `intent_registry` must be a **freshly deployed, not-yet-initialized**
    /// `IntentRegistry` instance. This function initializes it directly,
    /// passing `env.current_contract_address()` (this treasury) as its
    /// `admin` — which is the whole reason it needs to happen here rather
    /// than as a separate call from outside. `IntentRegistry::initialize`
    /// requires `admin.require_auth()`; with `admin` set to this treasury's
    /// own address and this treasury as the *direct* caller of that
    /// `initialize` call, Soroban's invoker-shortcut satisfies that
    /// `require_auth()` with no signature at all — the same mechanism that
    /// already lets `transfer_adapter`/`split_adapter` check
    /// `smart_account.require_auth()` with no separate auth entry when
    /// *this* contract is their direct caller. Doing this from *outside* (a
    /// plain keypair calling `intent_registry.initialize(smart_account_addr)`
    /// directly) does not get that shortcut — `smart_account_addr` is not
    /// the caller in that call, so it would need a real signed
    /// `AuthPayload` sub-invocation proving the treasury's own signers
    /// approved being named as `admin`, before the treasury has any
    /// registered signers to produce one. That chicken-and-egg problem is
    /// exactly what `scripts/bootstrap_intent_registry.py` existed to work
    /// around; folding the call in here removes the need for it in any new
    /// deployment.
    ///
    /// `initial_adapters` sets the `transfer`/`split`/… adapter bindings
    /// directly, with no timelock — unlike `propose_adapter_change`'s ~1
    /// day delay for every *later* change. That delay exists to stop an
    /// already-trusted binding from being swapped out from under a funded,
    /// operating treasury; it protects nothing here, since this is the
    /// account's first-ever configuration and it holds no funds yet. Empty
    /// is fine — adapters can always be added later via the normal
    /// propose/apply path.
    ///
    /// `initial_executor` sets `intent_registry`'s `Executor` explicitly,
    /// for the same reason `initial_adapters` exists: `IntentRegistry::
    /// initialize` defaults `Executor` to whatever `admin` it was given —
    /// which is *this treasury's own address* (see above). Left alone,
    /// `execute_scheduled_payment`'s call into
    /// `intent_registry.mark_child_executed` would satisfy `Executor`'s
    /// `require_auth()` via the same invoker-shortcut this treasury is the
    /// direct caller of, regardless of who called `execute_scheduled_payment`
    /// itself — silently defeating that entrypoint's documented "an
    /// operational relayer key gates *when* execution happens" property for
    /// any account that skips this. Pass the intended relayer's address
    /// here explicitly; passing `env.current_contract_address()` again is a
    /// valid, deliberate choice too (anyone may trigger execution), but it
    /// should be a choice, not an accident of `IntentRegistry`'s own
    /// generic default.
    pub fn initialize(
        env: Env,
        owner: Address,
        initial_signers: Vec<Signer>,
        initial_policies: Map<Address, Val>,
        config: InitConfig,
    ) -> Result<(), SmartAccountTreasuryError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(SmartAccountTreasuryError::AlreadyInitialized);
        }
        owner.require_auth();

        let InitConfig {
            policy_engine,
            intent_registry,
            recovery_manager,
            initial_adapters,
            initial_executor,
        } = config;

        ownable::set_owner(&env, &owner);
        add_context_rule_root(&env, &initial_signers, &initial_policies);
        let intent_registry_client = IntentRegistryClient::new(&env, &intent_registry);
        intent_registry_client.initialize(&env.current_contract_address());
        intent_registry_client.set_executor(&initial_executor);
        for (operation, adapter) in initial_adapters.iter() {
            env.storage()
                .instance()
                .set(&DataKey::Adapter(operation), &adapter);
        }

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage()
            .instance()
            .set(&DataKey::PolicyEngine, &policy_engine);
        env.storage()
            .instance()
            .set(&DataKey::IntentRegistry, &intent_registry);
        env.storage()
            .instance()
            .set(&DataKey::RecoveryManager, &recovery_manager);
        env.storage().instance().set(&DataKey::Frozen, &false);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);

        Initialized {
            owner: owner.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn status(env: Env) -> Result<AccountStatus, SmartAccountTreasuryError> {
        ensure_initialized(&env)?;
        Ok(AccountStatus {
            initialized: true,
            paused: pausable::paused(&env),
            frozen: is_frozen(&env),
            policy_version_hint: 0,
        })
    }

    pub fn is_nonce_used(env: Env, nonce: u64) -> bool {
        env.storage().persistent().has(&DataKey::UsedNonce(nonce))
    }

    /// Proposes rewiring which adapter contract handles a given operation.
    /// Owner-gated. Unlike `policy_engine` / `intent_registry` /
    /// `recovery_manager` (pinned at `initialize`, see Architecture
    /// Decision in `docs/TECHNICAL_ARCHITECTURE.md` §6.9), adapters are
    /// mutable post-init because the doc explicitly scopes
    /// `set_adapter_config` as an owner-configurable entrypoint.
    ///
    /// This does not take effect immediately — see `apply_adapter_change`.
    /// An independent security review (`docs/SMART_CONTRACT_AUDIT_REPORT.md`
    /// finding 3) noted that immediate, undelayed reconfiguration of
    /// execution routing is a meaningful risk for a treasury contract
    /// specifically (it decides *where funds go*, unlike e.g. `freeze`,
    /// which only stops movement and is deliberately kept immediate).
    /// Overwrites any existing pending proposal for the same operation —
    /// the most recent proposal wins, matching how re-proposing normally
    /// works elsewhere in this workspace (e.g. `policy_engine::set_asset_rule`).
    ///
    /// Explicitly extends the instance TTL on write. Review finding: this
    /// contract's instance storage (which `PendingAdapter` lives in) shares
    /// a single TTL across all of it, normally kept alive by `ensure_initialized`
    /// running on the contract's regular traffic — but a proposal can be
    /// made on an otherwise-dormant treasury, and the ~1 day timelock window
    /// is short enough that "wait for some other call to refresh it" isn't a
    /// safe assumption. Without this, a pending proposal on a quiet treasury
    /// could archive before `apply_adapter_change` is ever called, silently
    /// losing a legitimate governance action rather than just delaying it.
    pub fn propose_adapter_change(
        env: Env,
        operation: Symbol,
        adapter: Address,
    ) -> Result<(), SmartAccountTreasuryError> {
        ownable::enforce_owner_auth(&env);
        let effective_ledger = env
            .ledger()
            .sequence()
            .checked_add(ADAPTER_CHANGE_DELAY_LEDGERS)
            .ok_or(SmartAccountTreasuryError::NoPendingAdapterChange)?;
        env.storage().instance().set(
            &DataKey::PendingAdapter(operation.clone()),
            &PendingAdapterChange {
                adapter: adapter.clone(),
                effective_ledger,
            },
        );
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        AdapterChangeProposed {
            operation,
            adapter,
            effective_ledger,
        }
        .publish(&env);
        Ok(())
    }

    /// Cancels a pending adapter change before it takes effect. Owner-gated,
    /// mirroring `recovery_manager::cancel_recovery`'s pattern of letting
    /// the legitimate owner walk back an erroneous or no-longer-wanted
    /// proposal before its delay elapses.
    pub fn cancel_adapter_change(
        env: Env,
        operation: Symbol,
    ) -> Result<(), SmartAccountTreasuryError> {
        ownable::enforce_owner_auth(&env);
        let key = DataKey::PendingAdapter(operation.clone());
        if !env.storage().instance().has(&key) {
            return Err(SmartAccountTreasuryError::NoPendingAdapterChange);
        }
        env.storage().instance().remove(&key);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        AdapterChangeCancelled { operation }.publish(&env);
        Ok(())
    }

    /// Applies a pending adapter change once its delay has elapsed.
    /// Permissionless, like `apply_recovery`/`apply_guardian_freeze`: the
    /// authorization decision (the owner proposing this specific change)
    /// already happened at `propose_adapter_change` time, so *when* an
    /// already-authorized change actually lands needs no further gate to
    /// be safe — it only needs the delay to have passed.
    pub fn apply_adapter_change(
        env: Env,
        operation: Symbol,
    ) -> Result<(), SmartAccountTreasuryError> {
        let key = DataKey::PendingAdapter(operation.clone());
        let pending: PendingAdapterChange = env
            .storage()
            .instance()
            .get(&key)
            .ok_or(SmartAccountTreasuryError::NoPendingAdapterChange)?;
        if env.ledger().sequence() < pending.effective_ledger {
            return Err(SmartAccountTreasuryError::AdapterChangeDelayNotElapsed);
        }
        env.storage()
            .instance()
            .set(&DataKey::Adapter(operation.clone()), &pending.adapter);
        env.storage().instance().remove(&key);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        AdapterChanged {
            operation,
            adapter: pending.adapter,
        }
        .publish(&env);
        Ok(())
    }

    /// Emergency freeze. One-way by design: there is no `unfreeze()`
    /// entrypoint. Normal operation is restored only via `apply_recovery`,
    /// so a compromised owner cannot both freeze the account and later
    /// unfreeze it unilaterally without going through guardian quorum.
    pub fn freeze(env: Env) -> Result<(), SmartAccountTreasuryError> {
        ownable::enforce_owner_auth(&env);
        env.storage().instance().set(&DataKey::Frozen, &true);
        Frozen {
            triggered_by_guardian: false,
        }
        .publish(&env);
        Ok(())
    }

    /// Real gap this session found: `docs/TECHNICAL_ARCHITECTURE.md` §12.7
    /// documents the canonical recovery workflow as starting with "Guardian
    /// ... triggers freeze," but `freeze()` above is owner-gated only, with
    /// no guardian-facing path at all — if the owner is the one compromised,
    /// guardians had no way to stop ongoing spend authority while a full
    /// recovery (quorum + timelock) is still pending. This pulls a
    /// guardian-raised freeze flag from `recovery_manager` the same way
    /// `apply_recovery` pulls a finalized request: permissionlessly, on
    /// this contract's own terms, with `recovery_manager` carrying no
    /// knowledge of or dependency on this treasury.
    ///
    /// Security review finding: this used to call `recovery_manager`'s
    /// read-only `guardian_freeze_requested`, which checks a flag that is
    /// set once and, without this fix, never cleared. Since this
    /// entrypoint is deliberately permissionless (matching
    /// `apply_recovery`'s "pull an already-authorized fact" pattern), that
    /// meant anyone could re-freeze the treasury at any future point —
    /// even long after a full recovery had resolved the original
    /// incident — simply because a stale flag from months earlier was
    /// still sitting in `recovery_manager`'s storage. `apply_recovery`
    /// already guards against exactly this class of problem with a
    /// request-keyed `AppliedRecovery` replay guard; this now gets the
    /// same guarantee by calling `consume_guardian_freeze_request`
    /// (check-and-clear in one call) instead of the passive read.
    pub fn apply_guardian_freeze(env: Env) -> Result<(), SmartAccountTreasuryError> {
        ensure_initialized(&env)?;
        let recovery_manager = recovery_manager_address(&env)?;
        if !RecoveryManagerClient::new(&env, &recovery_manager).consume_guardian_freeze_request() {
            return Err(SmartAccountTreasuryError::GuardianFreezeNotRequested);
        }
        env.storage().instance().set(&DataKey::Frozen, &true);
        Frozen {
            triggered_by_guardian: true,
        }
        .publish(&env);
        Ok(())
    }

    /// Executes a single-recipient SAC payment. Requires this contract's
    /// own authorization (delegated entirely to the OZ context-rule/signer/
    /// policy model via `__check_auth`), then treasury policy, then a
    /// narrowly preauthorized adapter call.
    pub fn execute_transfer_payment(
        env: Env,
        asset: Address,
        destination: Address,
        amount: i128,
        nonce: u64,
        expected_policy_version: u32,
    ) -> Result<(), SmartAccountTreasuryError> {
        env.current_contract_address().require_auth();
        ensure_active(&env)?;
        consume_nonce(&env, nonce)?;

        if amount <= 0 {
            return Err(SmartAccountTreasuryError::InvalidAmount);
        }

        let policy_engine = policy_engine_address(&env)?;
        PolicyEngineClient::new(&env, &policy_engine).validate_policy(&PolicyCheck {
            operation: OP_TRANSFER,
            asset: asset.clone(),
            destination: destination.clone(),
            amount,
            expected_version: expected_policy_version,
        });

        let adapter = adapter_address(&env, &OP_TRANSFER)?;
        TransferAdapterClient::new(&env, &adapter).execute_transfer(&asset, &destination, &amount);

        TransferPaid {
            asset,
            destination,
            amount,
            nonce,
        }
        .publish(&env);
        Ok(())
    }

    /// Executes a one-to-many SAC split from the treasury. Each recipient
    /// is independently policy-checked (own recipient-allowlist entry, own
    /// per-recipient amount cap) before the adapter call, so a split cannot
    /// be used to move funds to a disallowed recipient by hiding it inside
    /// an aggregate-approved total.
    pub fn execute_split_payment(
        env: Env,
        asset: Address,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        nonce: u64,
        expected_policy_version: u32,
    ) -> Result<(), SmartAccountTreasuryError> {
        env.current_contract_address().require_auth();
        ensure_active(&env)?;
        consume_nonce(&env, nonce)?;

        if recipients.is_empty() {
            return Err(SmartAccountTreasuryError::EmptySplit);
        }
        if recipients.len() != amounts.len() {
            return Err(SmartAccountTreasuryError::RecipientAmountLengthMismatch);
        }

        // Real gap this session found: `docs/TECHNICAL_ARCHITECTURE.md`
        // §12.6 documents SmartAccount as validating "duplicate
        // destinations" for a split action. A repeated recipient isn't
        // exploitable (the total moved still equals the declared sum, to
        // an already-approved recipient), but it's ambiguous — likely a
        // caller/UI bug — and the architecture explicitly calls for
        // rejecting it rather than silently issuing extra transfers.
        let mut seen = Vec::<Address>::new(&env);
        for recipient in recipients.iter() {
            if seen.contains(&recipient) {
                return Err(SmartAccountTreasuryError::DuplicateRecipient);
            }
            seen.push_back(recipient);
        }

        let policy_engine = policy_engine_address(&env)?;
        let policy_client = PolicyEngineClient::new(&env, &policy_engine);
        for (recipient, amount) in recipients.iter().zip(amounts.iter()) {
            if amount <= 0 {
                return Err(SmartAccountTreasuryError::InvalidAmount);
            }
            policy_client.validate_policy(&PolicyCheck {
                operation: OP_SPLIT,
                asset: asset.clone(),
                destination: recipient,
                amount,
                expected_version: expected_policy_version,
            });
        }

        let adapter = adapter_address(&env, &OP_SPLIT)?;
        SplitAdapterClient::new(&env, &adapter).execute_split(&asset, &recipients, &amounts);

        SplitPaid {
            asset,
            recipient_count: recipients.len(),
            nonce,
        }
        .publish(&env);
        Ok(())
    }

    /// Creates a scheduled/recurring payment intent. Requires this
    /// contract's own authorization (same signer/context-rule/policy model
    /// as interactive payments), then delegates canonical lifecycle and
    /// replay state to `intent_registry` — see Architecture Decision 2.
    /// `intent_registry` must have been configured with this contract's
    /// own address as its admin, so this nested call succeeds via
    /// sub-invocation tree membership rather than a second signature.
    ///
    /// The caller-supplied `policy_version` and `adapter` on `intent` are
    /// both ignored and overwritten: `policy_version` with `policy_engine`'s
    /// current version, and `adapter` with the `transfer_adapter` currently
    /// configured (via `propose_adapter_change`/`apply_adapter_change`).
    /// Both must reflect what was actually in
    /// effect when the signer approved this schedule, not a value the
    /// caller chose — otherwise a later adapter reconfiguration could
    /// silently redirect an already-approved scheduled payment through a
    /// different adapter at execution time (see `execute_scheduled_payment`
    /// for why both pins matter there). Requiring `transfer_adapter` to
    /// already be configured at creation time is deliberate: a schedule
    /// should not be approvable before it's known how it will be paid.
    pub fn create_scheduled_payment(
        env: Env,
        mut intent: ScheduledIntentArgs,
    ) -> Result<(), SmartAccountTreasuryError> {
        env.current_contract_address().require_auth();
        ensure_active(&env)?;

        let policy_engine = policy_engine_address(&env)?;
        intent.policy_version = PolicyEngineClient::new(&env, &policy_engine).version();
        intent.adapter = adapter_address(&env, &OP_TRANSFER)?;

        let intent_registry = intent_registry_address(&env)?;
        IntentRegistryClient::new(&env, &intent_registry).create_intent(&intent);
        Ok(())
    }

    /// Cancels a previously created scheduled payment. Requires this
    /// contract's own authorization, same as creation.
    ///
    /// This closes a real gap: `intent_registry::cancel_intent` exists and
    /// is `ensure_admin`-gated, but `intent_registry`'s configured admin is
    /// this contract's own address — before this entrypoint existed, there
    /// was no way for anyone, including the treasury's own signers, to ever
    /// call it, since only `smart_account` itself can satisfy that admin
    /// check via a nested sub-invocation.
    pub fn cancel_scheduled_payment(
        env: Env,
        intent_id: BytesN<32>,
    ) -> Result<(), SmartAccountTreasuryError> {
        env.current_contract_address().require_auth();
        ensure_active(&env)?;

        let intent_registry = intent_registry_address(&env)?;
        IntentRegistryClient::new(&env, &intent_registry).cancel_intent(&intent_id);
        Ok(())
    }

    /// Executes an already-approved scheduled payment. Permissionless by
    /// design (matches `recovery_manager::finalize_recovery`): the signer
    /// approval already happened at `create_scheduled_payment` time. Safety
    /// here comes entirely from `intent_registry`'s own execution-window,
    /// cancellation, and cumulative-usage checks — not from caller
    /// identity — except that `intent_registry.mark_child_executed` itself
    /// still requires its configured `Executor` address to authorize the
    /// call, so an operational relayer key (not a treasury signer) gates
    /// *when* within the valid window execution happens.
    ///
    /// Deliberately takes only `intent_id` and `child_sequence` from the
    /// caller. An earlier revision also accepted `asset`, `destination`,
    /// `amount`, and `expected_policy_version` as caller-supplied
    /// parameters and used them directly for the policy check and adapter
    /// dispatch — since this entrypoint has no `require_auth()` gate of its
    /// own (by design, see above) and the relayer role is explicitly
    /// untrusted (`docs/TECHNICAL_ARCHITECTURE.md` §13.1: relayer MUST NOT
    /// hold authority), that meant any caller able to satisfy
    /// `intent_registry`'s executor auth could redirect an approved
    /// scheduled payment to an arbitrary destination, asset, or amount, and
    /// pick a stale `expected_policy_version` to bypass version pinning.
    /// All four values are now read back from the canonical `ScheduledIntent`
    /// record after `mark_child_executed` succeeds, so execution can only
    /// ever replay exactly what was approved at creation time.
    ///
    /// The adapter dispatched to is likewise the one pinned on the intent
    /// at creation (`intent.adapter`), not whatever `transfer_adapter` is
    /// currently configured. Resolving the adapter fresh
    /// at execution time used to let a reconfiguration made any time
    /// after approval — by the owner, or by whoever compromised that key —
    /// silently redirect an already-approved payment through a different
    /// execution path, breaking the "preauthorized exact action" guarantee
    /// scheduled payments are supposed to have (see
    /// `smart_account_pinned_adapter_survives_reconfiguration_after_approval`
    /// in `test.rs`). If the adapter genuinely needs to change for an
    /// existing schedule, cancel it and create a new one under the
    /// currently configured adapter.
    pub fn execute_scheduled_payment(
        env: Env,
        intent_id: BytesN<32>,
        child_sequence: u32,
    ) -> Result<(), SmartAccountTreasuryError> {
        ensure_active(&env)?;

        let intent_registry = intent_registry_address(&env)?;
        let intent_registry_client = IntentRegistryClient::new(&env, &intent_registry);
        intent_registry_client.mark_child_executed(&intent_id, &child_sequence);
        let intent = intent_registry_client.get_intent(&intent_id);

        let policy_engine = policy_engine_address(&env)?;
        PolicyEngineClient::new(&env, &policy_engine).validate_policy(&PolicyCheck {
            operation: OP_TRANSFER,
            asset: intent.asset.clone(),
            destination: intent.destination.clone(),
            amount: intent.amount,
            expected_version: intent.policy_version,
        });

        TransferAdapterClient::new(&env, &intent.adapter).execute_transfer(
            &intent.asset,
            &intent.destination,
            &intent.amount,
        );

        ScheduledPaymentExecuted {
            intent_id,
            child_sequence,
            asset: intent.asset,
            destination: intent.destination,
            amount: intent.amount,
        }
        .publish(&env);
        Ok(())
    }

    /// Pulls a finalized recovery outcome from the configured
    /// `recovery_manager` and applies it: force-overwrites ownership to
    /// the guardian-approved replacement and lifts the freeze. Never
    /// pushed by `recovery_manager` — this contract decides, on its own
    /// terms, whether to consume a finalized request, exactly once
    /// (`AppliedRecovery` replay guard). Permissionless: anyone may call
    /// this once `recovery_manager` reports the request finalized, mirroring
    /// `finalize_recovery`'s own permissionless design.
    pub fn apply_recovery(
        env: Env,
        request_id: BytesN<32>,
    ) -> Result<Address, SmartAccountTreasuryError> {
        ensure_initialized(&env)?;
        let applied_key = DataKey::AppliedRecovery(request_id.clone());
        if env.storage().persistent().has(&applied_key) {
            return Err(SmartAccountTreasuryError::RecoveryAlreadyApplied);
        }

        let recovery_manager = recovery_manager_address(&env)?;
        let request =
            RecoveryManagerClient::new(&env, &recovery_manager).request_status(&request_id);
        if !request.finalized {
            return Err(SmartAccountTreasuryError::RecoveryNotFinalized);
        }

        env.storage().persistent().set(&applied_key, &true);
        env.storage().persistent().extend_ttl(
            &applied_key,
            TTL_THRESHOLD_LEDGERS,
            TTL_EXTEND_TO_LEDGERS,
        );

        // Direct storage overwrite: recovery is a distinct authorization
        // path (guardian quorum + timelock, verified above) and must not
        // depend on the *current* owner's signature the way Ownable's own
        // `transfer_ownership` does — that would defeat the point of
        // recovery when the owner's signer is exactly what's compromised
        // or unavailable.
        env.storage().instance().set(
            &ownable::OwnableStorageKey::Owner,
            &request.replacement_owner,
        );
        env.storage().instance().set(&DataKey::Frozen, &false);

        RecoveryApplied {
            request_id,
            replacement_owner: request.replacement_owner.clone(),
        }
        .publish(&env);
        Ok(request.replacement_owner)
    }
}

/// Mirror of `intent_registry::ScheduledIntent`'s constructor shape (field
/// names/types must match structurally for cross-contract XDR decoding).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledIntentArgs {
    pub intent_id: BytesN<32>,
    pub asset: Address,
    pub destination: Address,
    pub amount: i128,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub max_executions: u32,
    pub execution_count: u32,
    pub policy_version: u32,
    pub adapter: Address,
    pub cancelled: bool,
}

#[contractclient(name = "IntentRegistryClient")]
#[allow(unused)]
trait IntentRegistryInterface {
    fn initialize(env: Env, admin: Address);
    fn set_executor(env: Env, executor: Address);
    fn create_intent(env: Env, intent: ScheduledIntentArgs);
    fn cancel_intent(env: Env, intent_id: BytesN<32>);
    fn mark_child_executed(env: Env, intent_id: BytesN<32>, child_sequence: u32);
    fn get_intent(env: Env, intent_id: BytesN<32>) -> ScheduledIntentArgs;
}

// ################## OZ TRAIT COMPOSITION ##################
// Zero custom cryptography or threshold math below this point — every
// method is either an unmodified OZ default or a thin delegation into
// `do_check_auth` / `ownable` / `pausable` storage primitives.

#[contractimpl(contracttrait)]
impl OzSmartAccountTrait for SmartAccountTreasury {}

#[contractimpl(contracttrait)]
impl ExecutionEntryPoint for SmartAccountTreasury {}

#[contractimpl(contracttrait)]
impl OzOwnable for SmartAccountTreasury {}

#[contractimpl(contracttrait)]
impl OzPausable for SmartAccountTreasury {
    /// The `Pausable` trait signature returns `()`, not `Result`, so a
    /// caller/owner mismatch can only be signaled by aborting — done via
    /// `panic_with_error!` with this contract's own typed error (matching
    /// every other failure path here) rather than a bare string `panic!`,
    /// which surfaced as an untyped trap a client couldn't match against
    /// the way it can `Error(Contract, #8011)`.
    fn pause(e: &Env, caller: Address) {
        let owner = ownable::enforce_owner_auth(e);
        if caller != owner {
            panic_with_error!(e, SmartAccountTreasuryError::Unauthorized);
        }
        pausable::pause(e);
    }

    fn unpause(e: &Env, caller: Address) {
        let owner = ownable::enforce_owner_auth(e);
        if caller != owner {
            panic_with_error!(e, SmartAccountTreasuryError::Unauthorized);
        }
        pausable::unpause(e);
    }
}

#[contractimpl]
impl CustomAccountInterface for SmartAccountTreasury {
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

// ################## INTERNAL HELPERS ##################

fn add_context_rule_root(env: &Env, signers: &Vec<Signer>, policies: &Map<Address, Val>) {
    // The flat `add_context_rule` re-export is the raw storage primitive —
    // unlike the `SmartAccount` trait's own `add_context_rule` entrypoint,
    // it does not itself call `require_auth()`, which is exactly what makes
    // it usable here: at `initialize` time there is no signer registered
    // yet to authorize against.
    add_context_rule(
        env,
        &ContextRuleType::Default,
        &String::from_str(env, "root"),
        None,
        signers,
        policies,
    );
}

fn ensure_initialized(env: &Env) -> Result<(), SmartAccountTreasuryError> {
    if env.storage().instance().has(&DataKey::Initialized) {
        // Instance storage holds this contract's own config (subordinate
        // module addresses, adapter map, frozen flag) and shares a single
        // TTL across all of it; refreshing here on every call that reaches
        // this far covers every entrypoint uniformly.
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        Ok(())
    } else {
        Err(SmartAccountTreasuryError::NotInitialized)
    }
}

fn is_frozen(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Frozen)
        .unwrap_or(false)
}

fn ensure_active(env: &Env) -> Result<(), SmartAccountTreasuryError> {
    ensure_initialized(env)?;
    if pausable::paused(env) {
        return Err(SmartAccountTreasuryError::Paused);
    }
    if is_frozen(env) {
        return Err(SmartAccountTreasuryError::Frozen);
    }
    Ok(())
}

fn consume_nonce(env: &Env, nonce: u64) -> Result<(), SmartAccountTreasuryError> {
    let key = DataKey::UsedNonce(nonce);
    if env.storage().persistent().has(&key) {
        return Err(SmartAccountTreasuryError::NonceAlreadyUsed);
    }
    env.storage().persistent().set(&key, &true);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    Ok(())
}

fn policy_engine_address(env: &Env) -> Result<Address, SmartAccountTreasuryError> {
    env.storage()
        .instance()
        .get(&DataKey::PolicyEngine)
        .ok_or(SmartAccountTreasuryError::NotInitialized)
}

fn intent_registry_address(env: &Env) -> Result<Address, SmartAccountTreasuryError> {
    env.storage()
        .instance()
        .get(&DataKey::IntentRegistry)
        .ok_or(SmartAccountTreasuryError::NotInitialized)
}

fn recovery_manager_address(env: &Env) -> Result<Address, SmartAccountTreasuryError> {
    env.storage()
        .instance()
        .get(&DataKey::RecoveryManager)
        .ok_or(SmartAccountTreasuryError::NotInitialized)
}

fn adapter_address(env: &Env, operation: &Symbol) -> Result<Address, SmartAccountTreasuryError> {
    env.storage()
        .instance()
        .get(&DataKey::Adapter(operation.clone()))
        .ok_or(SmartAccountTreasuryError::AdapterNotConfigured)
}

#[cfg(test)]
mod test;
