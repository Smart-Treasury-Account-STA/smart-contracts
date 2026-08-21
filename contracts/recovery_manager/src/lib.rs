#![no_std]

//! RecoveryManager: guardian-quorum, ledger-delayed recovery, explicitly
//! separated from day-to-day spend authority (see Architecture Decision 4 —
//! this separation is what stops "recover the account" from also being a
//! bypass for treasury policy). `smart_account` treats a finalized request
//! here as a *fact to pull*, not something this contract pushes: anyone may
//! call `smart_account::apply_recovery(request_id)`, which itself reads
//! `request_status` from the configured RecoveryManager, checks
//! `finalized == true`, and applies the owner replacement + unfreeze
//! exactly once. RecoveryManager never calls into `smart_account`, so this
//! contract carries zero knowledge of, or trust dependency on, the treasury
//! it protects.
//!
//! ## Live-recomputed approval accounting
//!
//! Earlier revisions of this contract tracked a simple `approvals: u32`
//! counter that was incremented on `approve_recovery` and never revisited.
//! That is unsound under signer-set changes: a guardian who approves and is
//! *later removed* would still count toward the threshold at finalize time,
//! because the counter had already been incremented and nothing decremented
//! it on removal. This contract instead stores the list of approver
//! addresses on the request and recomputes, at `finalize_recovery` time,
//! how many of them are *currently* registered guardians — mirroring how
//! `smart_account::approved_payment_weight` recomputes signer weight from
//! current state on every call rather than trusting a cached total. The
//! same recomputation applies to the threshold itself: `guardian_threshold`
//! is read fresh at finalize time, so a threshold raised after approvals
//! were cast is honored, not silently grandfathered in.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, symbol_short, Address,
    BytesN, Env, Symbol, Vec,
};

const MAX_APPROVERS: u32 = 20;

/// Minimum enforced delay between opening a recovery request and its
/// earliest eligible finalization ledger (~1 day at a 5s ledger close
/// time). Without a floor here, `open_recovery`'s caller — the same
/// `admin`/owner key that recovery exists to route around if compromised —
/// could set `earliest_ledger` to the current or a past ledger, making the
/// "delayed recovery gives the legitimate owner time to notice and react"
/// property entirely voluntary rather than contract-enforced.
const MIN_RECOVERY_DELAY_LEDGERS: u32 = 17280;

/// Delay before a newly registered guardian's approval counts toward a
/// threshold. Audit finding: `admin` (the recovery-management role) can
/// register guardians unilaterally. Without an activation delay, a
/// compromised `admin` key could register guardian addresses it also
/// controls and complete a full, self-serving recovery — guardian quorum
/// would add no protection beyond the raw timelock, defeating the point of
/// requiring independent approvers at all. This runs concurrently with
/// `MIN_RECOVERY_DELAY_LEDGERS` (both start counting immediately), so the
/// total attack window stays ~1 day rather than requiring a design that
/// sequences them; that is a deliberate scope/complexity tradeoff, not an
/// oversight — see `docs/V1_SCOPE.md`.
const GUARDIAN_ACTIVATION_DELAY_LEDGERS: u32 = 17280;

/// Delay between proposing and applying a guardian removal or a guardian
/// threshold change. Independent security review finding
/// (`docs/SMART_CONTRACT_AUDIT_REPORT.md`): `remove_guardian` and
/// `set_guardian_threshold` used to take effect immediately under
/// single-admin control — a compromised admin could dismantle or weaken
/// the guardian set (removing legitimate guardians, or lowering the
/// threshold) with no delay for anyone to notice and react. Mirrors the
/// existing ~1 day delay pattern already used for recovery/guardian
/// activation above.
const GUARDIAN_CHANGE_DELAY_LEDGERS: u32 = 17280;

/// See `docs/TECHNICAL_ARCHITECTURE.md` §16 ("Production requirement").
/// Guardian records and pending recovery plans are exactly the kind of
/// long-dormant, security-critical state §16 calls out by name — they may
/// go untouched for months and must not archive in the meantime.
const TTL_EXTEND_TO_LEDGERS: u32 = 30 * 17280; // ~30 days
const TTL_THRESHOLD_LEDGERS: u32 = TTL_EXTEND_TO_LEDGERS - 17280; // ~29 days

#[contract]
pub struct RecoveryManager;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRequest {
    pub request_id: BytesN<32>,
    pub replacement_owner: Address,
    pub earliest_ledger: u32,
    pub approvers: Vec<Address>,
    pub cancelled: bool,
    pub finalized: bool,
}

/// A proposed guardian threshold change awaiting its timelock. See
/// `propose_threshold_change`/`apply_threshold_change`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingThresholdChange {
    pub new_threshold: u32,
    pub effective_ledger: u32,
}

#[contractevent(topics = ["init"])]
pub struct Initialized {
    #[topic]
    pub admin: Address,
    pub guardian_threshold: u32,
}

#[contractevent(topics = ["threshp"])]
pub struct ThresholdChangeProposed {
    pub new_threshold: u32,
    pub effective_ledger: u32,
}

#[contractevent(topics = ["threshc"])]
pub struct ThresholdChangeCancelled {}

#[contractevent(topics = ["thresh"])]
pub struct ThresholdChanged {
    pub new_threshold: u32,
}

#[contractevent(topics = ["guard"])]
pub struct GuardianAdded {
    #[topic]
    pub guardian: Address,
    pub activates_at: u32,
}

#[contractevent(topics = ["unguardp"])]
pub struct GuardianRemovalProposed {
    #[topic]
    pub guardian: Address,
    pub effective_ledger: u32,
}

#[contractevent(topics = ["unguardc"])]
pub struct GuardianRemovalCancelled {
    #[topic]
    pub guardian: Address,
}

#[contractevent(topics = ["unguard"])]
pub struct GuardianRemoved {
    #[topic]
    pub guardian: Address,
}

#[contractevent(topics = ["gfreeze"])]
pub struct GuardianFreezeRequested {
    #[topic]
    pub guardian: Address,
}

#[contractevent(topics = ["open"])]
pub struct RecoveryOpened {
    #[topic]
    pub request_id: BytesN<32>,
}

#[contractevent(topics = ["appr"])]
pub struct RecoveryApproved {
    #[topic]
    pub request_id: BytesN<32>,
    #[topic]
    pub guardian: Address,
}

#[contractevent(topics = ["cancel"])]
pub struct RecoveryCancelled {
    #[topic]
    pub request_id: BytesN<32>,
}

#[contractevent(topics = ["final"])]
pub struct RecoveryFinalized {
    #[topic]
    pub request_id: BytesN<32>,
    pub replacement_owner: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
    Admin,
    GuardianThreshold,
    PendingGuardianThreshold,
    Guardian(Address),
    PendingGuardianRemoval(Address),
    Request(BytesN<32>),
    GuardianFreezeRequested,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RecoveryManagerError {
    AlreadyInitialized = 4000,
    NotInitialized = 4001,
    InvalidThreshold = 4002,
    GuardianAlreadyExists = 4003,
    GuardianNotFound = 4004,
    RequestAlreadyExists = 4005,
    RequestNotFound = 4006,
    DuplicateApproval = 4007,
    BelowThreshold = 4008,
    TimelockActive = 4009,
    RequestCancelled = 4010,
    RequestFinalized = 4011,
    TooManyApprovers = 4012,
    DelayTooShort = 4013,
    GuardianNotYetActive = 4014,
    Unauthorized = 4015,
    NoPendingGuardianRemoval = 4016,
    NoPendingThresholdChange = 4017,
    GuardianChangeDelayNotElapsed = 4018,
}

#[contractimpl]
impl RecoveryManager {
    pub fn contract_name() -> Symbol {
        symbol_short!("sta_rec")
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        guardian_threshold: u32,
    ) -> Result<(), RecoveryManagerError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(RecoveryManagerError::AlreadyInitialized);
        }
        if guardian_threshold == 0 {
            return Err(RecoveryManagerError::InvalidThreshold);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::GuardianThreshold, &guardian_threshold);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        Initialized {
            admin: admin.clone(),
            guardian_threshold,
        }
        .publish(&env);
        Ok(())
    }

    /// Permissionless TTL maintenance for named, already-existing guardians
    /// and requests — extending TTL creates no authority (§16.1). Also
    /// covers any pending threshold change and any pending removal for the
    /// given guardians: those are only bumped otherwise at `propose_*` time,
    /// so a proposal left unapplied long enough after its delay elapses
    /// could archive before `apply_*` is ever called, silently losing an
    /// already-authorized governance action rather than just delaying it
    /// (independent security review finding). `bump_ttl` no-ops on keys
    /// that don't exist, so calling this with no pending changes is safe.
    /// `Initialized`/`Admin`/`GuardianThreshold`/`PendingGuardianThreshold`
    /// live in instance storage (one shared TTL, refreshed automatically by
    /// `ensure_initialized` on every call that reaches it), so a single
    /// `extend_ttl` call below covers all of them together.
    pub fn extend_ttl(env: Env, guardians: Vec<Address>, request_ids: Vec<BytesN<32>>) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        for guardian in guardians.iter() {
            bump_ttl(&env, &DataKey::Guardian(guardian.clone()));
            bump_ttl(&env, &DataKey::PendingGuardianRemoval(guardian));
        }
        for request_id in request_ids.iter() {
            bump_ttl(&env, &DataKey::Request(request_id));
        }
    }

    /// Proposes a new guardian threshold. Does not take effect immediately
    /// — see `apply_threshold_change`. Once applied, it takes
    /// effect for any request not yet finalized: `finalize_recovery` always
    /// reads the threshold fresh rather than the value in effect when the
    /// request was opened or approved.
    pub fn propose_threshold_change(
        env: Env,
        new_threshold: u32,
    ) -> Result<(), RecoveryManagerError> {
        ensure_admin(&env)?;
        if new_threshold == 0 {
            return Err(RecoveryManagerError::InvalidThreshold);
        }
        let effective_ledger = env
            .ledger()
            .sequence()
            .checked_add(GUARDIAN_CHANGE_DELAY_LEDGERS)
            .ok_or(RecoveryManagerError::InvalidThreshold)?;
        let key = DataKey::PendingGuardianThreshold;
        env.storage().instance().set(
            &key,
            &PendingThresholdChange {
                new_threshold,
                effective_ledger,
            },
        );
        ThresholdChangeProposed {
            new_threshold,
            effective_ledger,
        }
        .publish(&env);
        Ok(())
    }

    /// Cancels a pending guardian threshold change before it takes effect.
    /// Admin-gated, letting a legitimate admin walk back an erroneous or
    /// no-longer-wanted proposal before its delay elapses.
    pub fn cancel_threshold_change(env: Env) -> Result<(), RecoveryManagerError> {
        ensure_admin(&env)?;
        let key = DataKey::PendingGuardianThreshold;
        if !env.storage().instance().has(&key) {
            return Err(RecoveryManagerError::NoPendingThresholdChange);
        }
        env.storage().instance().remove(&key);
        ThresholdChangeCancelled {}.publish(&env);
        Ok(())
    }

    /// Applies a pending guardian threshold change once its delay has
    /// elapsed. Permissionless: the authorization decision (the admin
    /// proposing this specific change) already happened, so *when* an
    /// already-authorized change lands needs no further gate. Doesn't call
    /// `ensure_initialized` (there is nothing to permission-check), so it
    /// explicitly extends the instance TTL itself on this write, the same
    /// way `smart_account::apply_adapter_change` does for its own
    /// instance-storage write.
    pub fn apply_threshold_change(env: Env) -> Result<(), RecoveryManagerError> {
        let key = DataKey::PendingGuardianThreshold;
        let pending: PendingThresholdChange = env
            .storage()
            .instance()
            .get(&key)
            .ok_or(RecoveryManagerError::NoPendingThresholdChange)?;
        if env.ledger().sequence() < pending.effective_ledger {
            return Err(RecoveryManagerError::GuardianChangeDelayNotElapsed);
        }
        env.storage()
            .instance()
            .set(&DataKey::GuardianThreshold, &pending.new_threshold);
        env.storage().instance().remove(&key);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        ThresholdChanged {
            new_threshold: pending.new_threshold,
        }
        .publish(&env);
        Ok(())
    }

    /// Registers a guardian. The guardian is recorded immediately
    /// (`is_guardian` reflects it right away), but its approval does not
    /// count toward any threshold until `GUARDIAN_ACTIVATION_DELAY_LEDGERS`
    /// have passed — see the constant's doc comment for why. Removing a
    /// guardian, unlike adding one, is itself timelocked — see
    /// `propose_remove_guardian`.
    pub fn add_guardian(env: Env, guardian: Address) -> Result<(), RecoveryManagerError> {
        ensure_admin(&env)?;
        let key = DataKey::Guardian(guardian.clone());
        if env.storage().persistent().has(&key) {
            return Err(RecoveryManagerError::GuardianAlreadyExists);
        }

        let activates_at = env
            .ledger()
            .sequence()
            .checked_add(GUARDIAN_ACTIVATION_DELAY_LEDGERS)
            .ok_or(RecoveryManagerError::GuardianAlreadyExists)?;
        env.storage().persistent().set(&key, &activates_at);
        bump_ttl(&env, &key);
        GuardianAdded {
            guardian: guardian.clone(),
            activates_at,
        }
        .publish(&env);
        Ok(())
    }

    /// Proposes removing a guardian. Does not take effect immediately —
    /// see `apply_guardian_removal`. The guardian remains fully active
    /// (still counted by `is_guardian`, `live_approval_count`, and able to
    /// approve requests or request a freeze) until the proposal is applied.
    /// Independent security review finding: immediate removal under
    /// single-admin control meant a compromised admin could dismantle the
    /// guardian set with no delay for anyone to notice.
    pub fn propose_remove_guardian(
        env: Env,
        guardian: Address,
    ) -> Result<(), RecoveryManagerError> {
        ensure_admin(&env)?;
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Guardian(guardian.clone()))
        {
            return Err(RecoveryManagerError::GuardianNotFound);
        }
        let effective_ledger = env
            .ledger()
            .sequence()
            .checked_add(GUARDIAN_CHANGE_DELAY_LEDGERS)
            .ok_or(RecoveryManagerError::GuardianNotFound)?;
        let key = DataKey::PendingGuardianRemoval(guardian.clone());
        env.storage().persistent().set(&key, &effective_ledger);
        bump_ttl(&env, &key);
        GuardianRemovalProposed {
            guardian: guardian.clone(),
            effective_ledger,
        }
        .publish(&env);
        Ok(())
    }

    /// Cancels a pending guardian removal before it takes effect.
    /// Admin-gated, letting a legitimate admin walk back an erroneous or
    /// no-longer-wanted proposal before its delay elapses.
    pub fn cancel_guardian_removal(
        env: Env,
        guardian: Address,
    ) -> Result<(), RecoveryManagerError> {
        ensure_admin(&env)?;
        let key = DataKey::PendingGuardianRemoval(guardian.clone());
        if !env.storage().persistent().has(&key) {
            return Err(RecoveryManagerError::NoPendingGuardianRemoval);
        }
        env.storage().persistent().remove(&key);
        GuardianRemovalCancelled {
            guardian: guardian.clone(),
        }
        .publish(&env);
        Ok(())
    }

    /// Applies a pending guardian removal once its delay has elapsed.
    /// Permissionless: the authorization decision (the admin proposing this
    /// specific removal) already happened, so *when* an already-authorized
    /// removal lands needs no further gate. Does not touch any open
    /// request's stored `approvers` list — that history is preserved as an
    /// audit trail — but the removed guardian's past approval stops
    /// counting toward `live_approval_count` / `finalize_recovery` from
    /// this point on, because both recompute against current guardian
    /// registration rather than trusting the historical approval.
    pub fn apply_guardian_removal(env: Env, guardian: Address) -> Result<(), RecoveryManagerError> {
        let pending_key = DataKey::PendingGuardianRemoval(guardian.clone());
        let effective_ledger: u32 = env
            .storage()
            .persistent()
            .get(&pending_key)
            .ok_or(RecoveryManagerError::NoPendingGuardianRemoval)?;
        if env.ledger().sequence() < effective_ledger {
            return Err(RecoveryManagerError::GuardianChangeDelayNotElapsed);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::Guardian(guardian.clone()));
        env.storage().persistent().remove(&pending_key);
        GuardianRemoved {
            guardian: guardian.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn is_guardian(env: Env, guardian: Address) -> bool {
        env.storage().persistent().has(&DataKey::Guardian(guardian))
    }

    /// Real gap this session found: `docs/TECHNICAL_ARCHITECTURE.md` §12.7
    /// documents the canonical recovery workflow as starting with "Guardian
    /// ... triggers freeze" — but `smart_account::freeze()` was owner-only,
    /// with no guardian-facing path at all. A single active guardian
    /// requesting freeze is enough to raise the flag (unlike recovery
    /// itself, which needs quorum + timelock to *replace* authority,
    /// stopping active bleeding while recovery is still pending doesn't
    /// need the same bar — the worst case is an unnecessary pause, not a
    /// loss of funds or authority). `smart_account::apply_guardian_freeze`
    /// pulls this flag the same way `apply_recovery` pulls a finalized
    /// request: permissionlessly, and this contract has no knowledge of, or
    /// dependency on, the treasury that pulls it.
    pub fn request_guardian_freeze(
        env: Env,
        guardian: Address,
    ) -> Result<(), RecoveryManagerError> {
        ensure_initialized(&env)?;
        guardian.require_auth();
        if !is_active_guardian(&env, &guardian) {
            return Err(RecoveryManagerError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::GuardianFreezeRequested, &true);
        GuardianFreezeRequested {
            guardian: guardian.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn guardian_freeze_requested(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::GuardianFreezeRequested)
            .unwrap_or(false)
    }

    /// Security review finding: `guardian_freeze_requested` alone is a
    /// read-only check against a flag that, before this function existed,
    /// was set once by `request_guardian_freeze` and never cleared —
    /// meaning `smart_account::apply_guardian_freeze` (deliberately
    /// permissionless, matching `apply_recovery`'s "pull an
    /// already-authorized fact" pattern) could be called again at any
    /// point in the future, by anyone, and would keep succeeding, even
    /// long after the original incident was fully resolved through a
    /// completed recovery. `apply_recovery` already guards against this
    /// exact class of problem with a request-keyed `AppliedRecovery`
    /// replay guard; this flag had no equivalent. Consuming it atomically
    /// here — check and clear in one call, not a separate read then a
    /// separate write — closes the same gap for guardian-triggered freeze
    /// that `AppliedRecovery` already closes for recovery finalization,
    /// and avoids a TOCTOU window between checking and clearing.
    /// Permissionless, like the read it replaces for this purpose:
    /// clearing an already-consumed authorization creates no new
    /// authority.
    pub fn consume_guardian_freeze_request(env: Env) -> bool {
        let key = DataKey::GuardianFreezeRequested;
        let requested = env.storage().instance().get(&key).unwrap_or(false);
        if requested {
            env.storage().instance().remove(&key);
        }
        requested
    }

    /// Opens a recovery request. Callable by `admin`/owner **or by any
    /// currently-active guardian** — this is a deliberate fix, not the
    /// original design: requiring the admin's own authorization to open a
    /// recovery request means recovery is unusable in exactly the scenario
    /// it exists for (the owner's key is lost or destroyed, not merely
    /// "compromised but still controllable by its holder"). A guardian who
    /// suspects the owner is compromised, or who is trying to help an owner
    /// who has genuinely lost access, can start the process unassisted;
    /// `approve_recovery`'s threshold and `finalize_recovery`'s timelock
    /// are what actually gate the outcome, not who was allowed to open the
    /// request.
    pub fn open_recovery(
        env: Env,
        caller: Address,
        request_id: BytesN<32>,
        replacement_owner: Address,
        earliest_ledger: u32,
    ) -> Result<(), RecoveryManagerError> {
        ensure_initialized(&env)?;
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(RecoveryManagerError::NotInitialized)?;
        if caller != admin && !is_active_guardian(&env, &caller) {
            return Err(RecoveryManagerError::Unauthorized);
        }

        let key = DataKey::Request(request_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(RecoveryManagerError::RequestAlreadyExists);
        }
        let min_earliest = env
            .ledger()
            .sequence()
            .checked_add(MIN_RECOVERY_DELAY_LEDGERS)
            .ok_or(RecoveryManagerError::DelayTooShort)?;
        if earliest_ledger < min_earliest {
            return Err(RecoveryManagerError::DelayTooShort);
        }

        let request = RecoveryRequest {
            request_id: request_id.clone(),
            replacement_owner,
            earliest_ledger,
            approvers: Vec::new(&env),
            cancelled: false,
            finalized: false,
        };
        env.storage().persistent().set(&key, &request);
        bump_ttl(&env, &key);
        RecoveryOpened {
            request_id: request_id.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn approve_recovery(
        env: Env,
        request_id: BytesN<32>,
        guardian: Address,
    ) -> Result<(), RecoveryManagerError> {
        ensure_initialized(&env)?;
        guardian.require_auth();
        let activates_at: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Guardian(guardian.clone()))
            .ok_or(RecoveryManagerError::GuardianNotFound)?;
        if env.ledger().sequence() < activates_at {
            return Err(RecoveryManagerError::GuardianNotYetActive);
        }

        let key = DataKey::Request(request_id.clone());
        let mut request: RecoveryRequest = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RecoveryManagerError::RequestNotFound)?;
        ensure_request_open(&request)?;

        if request.approvers.contains(&guardian) {
            return Err(RecoveryManagerError::DuplicateApproval);
        }
        if request.approvers.len() >= MAX_APPROVERS {
            return Err(RecoveryManagerError::TooManyApprovers);
        }

        request.approvers.push_back(guardian.clone());
        env.storage().persistent().set(&key, &request);
        bump_ttl(&env, &key);
        RecoveryApproved {
            request_id: request_id.clone(),
            guardian: guardian.clone(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn cancel_recovery(env: Env, request_id: BytesN<32>) -> Result<(), RecoveryManagerError> {
        ensure_admin(&env)?;
        let key = DataKey::Request(request_id.clone());
        let mut request: RecoveryRequest = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RecoveryManagerError::RequestNotFound)?;
        ensure_request_open(&request)?;

        request.cancelled = true;
        env.storage().persistent().set(&key, &request);
        bump_ttl(&env, &key);
        RecoveryCancelled {
            request_id: request_id.clone(),
        }
        .publish(&env);
        Ok(())
    }

    /// Recomputes, from current guardian registrations, how many of this
    /// request's historical approvers are still valid right now. This is
    /// the number `finalize_recovery` actually checks against the
    /// threshold — never the raw `approvers.len()`.
    pub fn live_approval_count(
        env: Env,
        request_id: BytesN<32>,
    ) -> Result<u32, RecoveryManagerError> {
        ensure_initialized(&env)?;
        let request: RecoveryRequest = env
            .storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .ok_or(RecoveryManagerError::RequestNotFound)?;
        Ok(live_approval_count(&env, &request))
    }

    pub fn finalize_recovery(
        env: Env,
        request_id: BytesN<32>,
    ) -> Result<Address, RecoveryManagerError> {
        ensure_initialized(&env)?;
        let key = DataKey::Request(request_id.clone());
        let mut request: RecoveryRequest = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RecoveryManagerError::RequestNotFound)?;
        ensure_request_open(&request)?;

        if live_approval_count(&env, &request) < guardian_threshold(&env) {
            return Err(RecoveryManagerError::BelowThreshold);
        }
        if env.ledger().sequence() < request.earliest_ledger {
            return Err(RecoveryManagerError::TimelockActive);
        }

        request.finalized = true;
        env.storage().persistent().set(&key, &request);
        bump_ttl(&env, &key);
        RecoveryFinalized {
            request_id: request_id.clone(),
            replacement_owner: request.replacement_owner.clone(),
        }
        .publish(&env);
        Ok(request.replacement_owner)
    }

    pub fn request_status(
        env: Env,
        request_id: BytesN<32>,
    ) -> Result<RecoveryRequest, RecoveryManagerError> {
        ensure_initialized(&env)?;
        let key = DataKey::Request(request_id);
        let request = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RecoveryManagerError::RequestNotFound)?;
        bump_ttl(&env, &key);
        Ok(request)
    }
}

fn bump_ttl(env: &Env, key: &DataKey) {
    if env.storage().persistent().has(key) {
        env.storage()
            .persistent()
            .extend_ttl(key, TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
    }
}

fn ensure_initialized(env: &Env) -> Result<(), RecoveryManagerError> {
    if env.storage().instance().has(&DataKey::Initialized) {
        // Instance storage holds this contract's own singleton config
        // (`Initialized`/`Admin`/`GuardianThreshold`/
        // `PendingGuardianThreshold`/`GuardianFreezeRequested`) and shares a
        // single TTL across all of it; refreshing here on every call that
        // reaches this far covers every entrypoint uniformly, matching
        // `smart_account`'s own `ensure_initialized`.
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
        Ok(())
    } else {
        Err(RecoveryManagerError::NotInitialized)
    }
}

fn ensure_admin(env: &Env) -> Result<Address, RecoveryManagerError> {
    ensure_initialized(env)?;
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(RecoveryManagerError::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}

fn guardian_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::GuardianThreshold)
        .unwrap_or(1)
}

fn live_approval_count(env: &Env, request: &RecoveryRequest) -> u32 {
    let mut count = 0_u32;
    for guardian in request.approvers.iter() {
        if is_active_guardian(env, &guardian) {
            count += 1;
        }
    }
    count
}

/// True only if `guardian` is currently registered *and* past its
/// activation delay. A removed guardian's key is deleted entirely (so this
/// returns `false` for them too), meaning this single check covers both
/// "removed" and "not yet active" — the two ways a historical approval can
/// stop counting toward the current threshold.
fn is_active_guardian(env: &Env, guardian: &Address) -> bool {
    match env
        .storage()
        .persistent()
        .get::<_, u32>(&DataKey::Guardian(guardian.clone()))
    {
        Some(activates_at) => env.ledger().sequence() >= activates_at,
        None => false,
    }
}

fn ensure_request_open(request: &RecoveryRequest) -> Result<(), RecoveryManagerError> {
    if request.cancelled {
        return Err(RecoveryManagerError::RequestCancelled);
    }
    if request.finalized {
        return Err(RecoveryManagerError::RequestFinalized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::{
        storage::Instance as _, storage::Persistent as _, Address as _, Ledger, LedgerInfo,
    };

    fn setup() -> (Env, RecoveryManagerClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RecoveryManager, ());
        let client = RecoveryManagerClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &2);
        (env, client, admin)
    }

    fn id(env: &Env, seed: u8) -> BytesN<32> {
        BytesN::from_array(env, &[seed; 32])
    }

    fn set_ledger(env: &Env, sequence_number: u32) {
        env.ledger().set(LedgerInfo {
            timestamp: sequence_number as u64,
            protocol_version: 26,
            sequence_number,
            network_id: Default::default(),
            base_reserve: 5,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 16,
            max_entry_ttl: 3_110_400, // ~6 years, matches real Stellar network config headroom
        });
    }

    #[test]
    fn finalizes_after_threshold_and_timelock() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 1);
        let replacement = Address::generate(&env);
        let guardian_a = Address::generate(&env);
        let guardian_b = Address::generate(&env);

        client.add_guardian(&guardian_a);
        client.add_guardian(&guardian_b);
        client.open_recovery(&admin, &request_id, &replacement, &17280);
        // Guardians activate at the same ledger the recovery timelock
        // opens (both constants are 17280 in this test build) — advance
        // once, which satisfies guardian activation for approval AND the
        // recovery timelock for finalization.
        set_ledger(&env, 17280);
        client.approve_recovery(&request_id, &guardian_a);
        client.approve_recovery(&request_id, &guardian_b);

        assert_eq!(client.live_approval_count(&request_id), 2);
        assert_eq!(client.finalize_recovery(&request_id), replacement);
    }

    #[test]
    fn rejects_duplicate_guardian_approval_and_below_threshold() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 4);
        let guardian = Address::generate(&env);

        client.add_guardian(&guardian);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);
        set_ledger(&env, 17280);
        client.approve_recovery(&request_id, &guardian);

        let duplicate = client.try_approve_recovery(&request_id, &guardian);
        assert_eq!(duplicate, Err(Ok(RecoveryManagerError::DuplicateApproval)));

        let below_threshold = client.try_finalize_recovery(&request_id);
        assert_eq!(
            below_threshold,
            Err(Ok(RecoveryManagerError::BelowThreshold))
        );
    }

    /// Audit finding: a newly registered guardian's approval must not count
    /// until `GUARDIAN_ACTIVATION_DELAY_LEDGERS` has passed — otherwise a
    /// compromised `admin` key could register self-controlled guardians and
    /// complete an unassisted recovery, making the guardian-quorum
    /// requirement meaningless.
    #[test]
    fn newly_added_guardian_cannot_approve_before_activation_delay() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 22);
        let guardian = Address::generate(&env);

        client.add_guardian(&guardian);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);

        // Still ledger 0: guardian activates at 17280, not yet active.
        let too_early = client.try_approve_recovery(&request_id, &guardian);
        assert_eq!(
            too_early,
            Err(Ok(RecoveryManagerError::GuardianNotYetActive))
        );

        set_ledger(&env, 17280);
        client.approve_recovery(&request_id, &guardian);
    }

    #[test]
    fn removed_guardian_cannot_approve_and_double_removal_fails() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 5);
        let guardian = Address::generate(&env);

        client.add_guardian(&guardian);
        client.propose_remove_guardian(&guardian);
        set_ledger(&env, GUARDIAN_CHANGE_DELAY_LEDGERS);
        client.apply_guardian_removal(&guardian);

        client.open_recovery(
            &admin,
            &request_id,
            &Address::generate(&env),
            &(GUARDIAN_CHANGE_DELAY_LEDGERS * 2),
        );
        let revoked = client.try_approve_recovery(&request_id, &guardian);
        assert_eq!(revoked, Err(Ok(RecoveryManagerError::GuardianNotFound)));

        let double_removal = client.try_propose_remove_guardian(&guardian);
        assert_eq!(
            double_removal,
            Err(Ok(RecoveryManagerError::GuardianNotFound))
        );
    }

    /// Independent security review finding (`docs/SMART_CONTRACT_AUDIT_REPORT.md`):
    /// guardian removal used to take effect immediately under single-admin
    /// control. This proves the timelock is real, not just present in the
    /// happy path — applying before the delay has elapsed is rejected, and
    /// the guardian remains registered.
    #[test]
    fn guardian_removal_cannot_be_applied_before_the_delay_elapses() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        client.propose_remove_guardian(&guardian);

        let too_early = client.try_apply_guardian_removal(&guardian);
        assert_eq!(
            too_early,
            Err(Ok(RecoveryManagerError::GuardianChangeDelayNotElapsed))
        );
        assert!(client.is_guardian(&guardian));
    }

    /// The legitimate admin can walk back an erroneous or no-longer-wanted
    /// removal proposal before it takes effect — mirrors
    /// `cancel_recovery`'s existing pattern for open recovery requests.
    #[test]
    fn guardian_removal_can_be_cancelled_before_it_takes_effect() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        client.propose_remove_guardian(&guardian);
        client.cancel_guardian_removal(&guardian);

        set_ledger(&env, GUARDIAN_CHANGE_DELAY_LEDGERS);
        let no_pending = client.try_apply_guardian_removal(&guardian);
        assert_eq!(
            no_pending,
            Err(Ok(RecoveryManagerError::NoPendingGuardianRemoval))
        );
        assert!(client.is_guardian(&guardian));
    }

    #[test]
    fn cancel_guardian_removal_rejects_when_nothing_is_pending() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        let err = client.try_cancel_guardian_removal(&guardian);
        assert_eq!(err, Err(Ok(RecoveryManagerError::NoPendingGuardianRemoval)));
    }

    /// Same two properties as the guardian-removal timelock, for the
    /// threshold-change timelock: too-early application is rejected, and a
    /// cancelled proposal cannot later be applied.
    #[test]
    fn threshold_change_cannot_be_applied_before_the_delay_elapses() {
        let (_env, client, _admin) = setup();
        client.propose_threshold_change(&3);

        let too_early = client.try_apply_threshold_change();
        assert_eq!(
            too_early,
            Err(Ok(RecoveryManagerError::GuardianChangeDelayNotElapsed))
        );
    }

    #[test]
    fn threshold_change_can_be_cancelled_before_it_takes_effect() {
        let (env, client, _admin) = setup();
        client.propose_threshold_change(&3);
        client.cancel_threshold_change();

        set_ledger(&env, GUARDIAN_CHANGE_DELAY_LEDGERS);
        let no_pending = client.try_apply_threshold_change();
        assert_eq!(
            no_pending,
            Err(Ok(RecoveryManagerError::NoPendingThresholdChange))
        );
    }

    #[test]
    fn cancel_threshold_change_rejects_when_nothing_is_pending() {
        let (_env, client, _admin) = setup();
        let err = client.try_cancel_threshold_change();
        assert_eq!(err, Err(Ok(RecoveryManagerError::NoPendingThresholdChange)));
    }

    /// The authorization decision already happened at proposal time (an
    /// admin-gated call); applying an already-authorized, delay-elapsed
    /// change is deliberately permissionless — same pattern as
    /// `apply_recovery`/`apply_guardian_freeze` in `smart_account`. Proven
    /// here with zero mocked or real authorization at all.
    #[test]
    fn applying_guardian_removal_and_threshold_change_needs_no_authorization() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        client.propose_remove_guardian(&guardian);
        client.propose_threshold_change(&1);

        set_ledger(&env, GUARDIAN_CHANGE_DELAY_LEDGERS);
        env.set_auths(&[]);
        client.apply_guardian_removal(&guardian);
        client.apply_threshold_change();
        assert!(!client.is_guardian(&guardian));
    }

    #[test]
    fn cancelled_request_cannot_finalize() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 7);

        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);
        client.cancel_recovery(&request_id);

        set_ledger(&env, 17280);
        let cancelled = client.try_finalize_recovery(&request_id);
        assert_eq!(cancelled, Err(Ok(RecoveryManagerError::RequestCancelled)));
    }

    #[test]
    fn rejects_finalization_before_timelock() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 10);
        let guardian_a = Address::generate(&env);
        let guardian_b = Address::generate(&env);

        client.add_guardian(&guardian_a);
        client.add_guardian(&guardian_b);
        // A recovery timelock longer than the guardian activation delay,
        // so there's a window where guardians are active but the
        // request's own timelock has not yet opened.
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &30_000);
        set_ledger(&env, 17280);
        client.approve_recovery(&request_id, &guardian_a);
        client.approve_recovery(&request_id, &guardian_b);

        set_ledger(&env, 29_999);
        let timelock = client.try_finalize_recovery(&request_id);
        assert_eq!(timelock, Err(Ok(RecoveryManagerError::TimelockActive)));
    }

    /// Edge case (explicit reviewer concern: "signer set changes mid-
    /// transaction"): a guardian approves while still registered, is then
    /// removed by the admin, and the request must NOT be finalizable off
    /// the strength of that now-stale approval, even though the raw
    /// approval was validly cast at the time.
    #[test]
    fn guardian_removed_after_approving_no_longer_counts_toward_threshold() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 11);
        let guardian_a = Address::generate(&env);
        let guardian_b = Address::generate(&env);

        client.add_guardian(&guardian_a);
        client.add_guardian(&guardian_b);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);
        set_ledger(&env, 17280);
        client.approve_recovery(&request_id, &guardian_a);
        client.approve_recovery(&request_id, &guardian_b);
        assert_eq!(client.live_approval_count(&request_id), 2);

        // guardian_a's device is compromised; admin proposes removal — this
        // does not take effect immediately (independent security review
        // finding; see `docs/SMART_CONTRACT_AUDIT_REPORT.md`), so the
        // approval still counts until the delay elapses and the removal is
        // actually applied.
        client.propose_remove_guardian(&guardian_a);
        assert_eq!(client.live_approval_count(&request_id), 2);

        set_ledger(&env, 17280 + GUARDIAN_CHANGE_DELAY_LEDGERS);
        client.apply_guardian_removal(&guardian_a);
        assert_eq!(client.live_approval_count(&request_id), 1);

        let err = client.try_finalize_recovery(&request_id);
        assert_eq!(err, Err(Ok(RecoveryManagerError::BelowThreshold)));

        // A fresh, currently-valid guardian's approval restores quorum,
        // once past its own activation delay.
        let guardian_c = Address::generate(&env);
        client.add_guardian(&guardian_c);
        set_ledger(
            &env,
            17280 + GUARDIAN_CHANGE_DELAY_LEDGERS + GUARDIAN_ACTIVATION_DELAY_LEDGERS,
        );
        client.approve_recovery(&request_id, &guardian_c);
        assert_eq!(client.live_approval_count(&request_id), 2);
        assert!(client.try_finalize_recovery(&request_id).is_ok());
    }

    /// Edge case (explicit reviewer concern: "partial approvals" / rule
    /// changes mid-flight): raising the guardian threshold after approvals
    /// were already cast must retroactively apply — a request that was
    /// sufficient under the old threshold is not grandfathered in.
    #[test]
    fn raising_threshold_after_approvals_invalidates_a_previously_sufficient_request() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 12);
        let guardian_a = Address::generate(&env);
        let guardian_b = Address::generate(&env);

        client.add_guardian(&guardian_a);
        client.add_guardian(&guardian_b);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);
        set_ledger(&env, 17280);
        client.approve_recovery(&request_id, &guardian_a);
        client.approve_recovery(&request_id, &guardian_b);

        // Threshold was 2 at open time and both guardians approved — would
        // have finalized. Admin proposes raising it to 3, and the delay
        // elapses, before anyone finalizes.
        client.propose_threshold_change(&3);
        set_ledger(&env, 17280 + GUARDIAN_CHANGE_DELAY_LEDGERS);
        client.apply_threshold_change();

        let err = client.try_finalize_recovery(&request_id);
        assert_eq!(err, Err(Ok(RecoveryManagerError::BelowThreshold)));
    }

    /// Edge case: an already-finalized or cancelled request is immutable —
    /// approvals, threshold changes, and further finalize calls must all be
    /// rejected, never silently re-applied.
    #[test]
    fn finalized_request_rejects_further_approval_and_refinalization() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 13);
        let guardian_a = Address::generate(&env);
        let guardian_b = Address::generate(&env);

        client.add_guardian(&guardian_a);
        client.add_guardian(&guardian_b);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);
        set_ledger(&env, 17280);
        client.approve_recovery(&request_id, &guardian_a);
        client.approve_recovery(&request_id, &guardian_b);
        client.finalize_recovery(&request_id);

        let guardian_c = Address::generate(&env);
        client.add_guardian(&guardian_c);
        set_ledger(&env, 17280 + GUARDIAN_ACTIVATION_DELAY_LEDGERS);
        let late_approval = client.try_approve_recovery(&request_id, &guardian_c);
        assert_eq!(
            late_approval,
            Err(Ok(RecoveryManagerError::RequestFinalized))
        );

        let refinalize = client.try_finalize_recovery(&request_id);
        assert_eq!(refinalize, Err(Ok(RecoveryManagerError::RequestFinalized)));
    }

    /// Audit finding: `open_recovery` previously accepted any
    /// `earliest_ledger`, including the current or a past ledger — the same
    /// `admin` key recovery exists to route around if compromised could set
    /// an effectively zero delay. `MIN_RECOVERY_DELAY_LEDGERS` makes the
    /// floor contract-enforced rather than voluntary.
    #[test]
    fn open_recovery_rejects_delay_below_the_enforced_minimum() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 20);

        let too_soon = client.try_open_recovery(
            &admin,
            &request_id,
            &Address::generate(&env),
            &(MIN_RECOVERY_DELAY_LEDGERS - 1),
        );
        assert_eq!(too_soon, Err(Ok(RecoveryManagerError::DelayTooShort)));

        let in_the_past =
            client.try_open_recovery(&admin, &request_id, &Address::generate(&env), &0);
        assert_eq!(in_the_past, Err(Ok(RecoveryManagerError::DelayTooShort)));

        // Exactly the minimum is accepted.
        client.open_recovery(
            &admin,
            &request_id,
            &Address::generate(&env),
            &MIN_RECOVERY_DELAY_LEDGERS,
        );
    }

    /// The minimum delay is measured from the *current* ledger at
    /// `open_recovery` time, not from ledger zero — opening a second
    /// request later in the chain's life must still require a full delay
    /// from then, not an absolute ledger number that's already in the past.
    #[test]
    fn minimum_delay_is_relative_to_current_ledger_not_absolute() {
        let (env, client, admin) = setup();
        set_ledger(&env, 100_000);
        let request_id = id(&env, 21);

        let too_soon = client.try_open_recovery(
            &admin,
            &request_id,
            &Address::generate(&env),
            &(100_000 + MIN_RECOVERY_DELAY_LEDGERS - 1),
        );
        assert_eq!(too_soon, Err(Ok(RecoveryManagerError::DelayTooShort)));

        client.open_recovery(
            &admin,
            &request_id,
            &Address::generate(&env),
            &(100_000 + MIN_RECOVERY_DELAY_LEDGERS),
        );
    }

    /// Audit finding (`docs/TECHNICAL_ARCHITECTURE.md` §16): guardian
    /// records and pending recovery plans are named explicitly as
    /// long-dormant, security-critical state that must not archive.
    #[test]
    fn guardian_and_request_ttl_is_extended_on_write_and_permissionless_maintenance() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RecoveryManager, ());
        let client = RecoveryManagerClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &1);

        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        let request_id = id(&env, 99);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);

        let guardian_ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Guardian(guardian.clone()))
        });
        let request_ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Request(request_id.clone()))
        });
        assert!(guardian_ttl >= TTL_THRESHOLD_LEDGERS);
        assert!(request_ttl >= TTL_THRESHOLD_LEDGERS);

        // Decay a long way, then confirm the permissionless maintenance
        // entrypoint (no auth at all) refreshes both.
        set_ledger(&env, TTL_THRESHOLD_LEDGERS / 2);
        env.set_auths(&[]);
        client.extend_ttl(
            &soroban_sdk::vec![&env, guardian.clone()],
            &soroban_sdk::vec![&env, request_id.clone()],
        );

        let refreshed_guardian_ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Guardian(guardian))
        });
        let refreshed_request_ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::Request(request_id))
        });
        assert!(refreshed_guardian_ttl >= TTL_THRESHOLD_LEDGERS);
        assert!(refreshed_request_ttl >= TTL_THRESHOLD_LEDGERS);
    }

    /// Independent security review finding: a pending threshold change or
    /// guardian removal is only TTL-bumped once, at `propose_*` time. Left
    /// unapplied long enough after its delay elapses, it could archive
    /// before `apply_*` is ever called. `extend_ttl` must keep both alive
    /// too, not just guardians/requests.
    #[test]
    fn pending_governance_changes_are_refreshed_by_permissionless_maintenance() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RecoveryManager, ());
        let client = RecoveryManagerClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &1);

        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        client.propose_threshold_change(&2);
        client.propose_remove_guardian(&guardian);

        set_ledger(&env, TTL_THRESHOLD_LEDGERS / 2);
        env.set_auths(&[]);
        client.extend_ttl(
            &soroban_sdk::vec![&env, guardian.clone()],
            &soroban_sdk::vec![&env],
        );

        // `PendingGuardianThreshold` now lives in instance storage (see the
        // singleton-config migration below), so its TTL is the instance
        // TTL, not a per-key persistent one.
        let instance_ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
        let removal_ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&DataKey::PendingGuardianRemoval(guardian))
        });
        assert!(instance_ttl >= TTL_THRESHOLD_LEDGERS);
        assert!(removal_ttl >= TTL_THRESHOLD_LEDGERS);
    }

    #[test]
    fn contract_name_reports_expected_symbol() {
        assert_eq!(RecoveryManager::contract_name(), symbol_short!("sta_rec"));
    }

    #[test]
    fn calling_before_initialize_rejects_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RecoveryManager, ());
        let client = RecoveryManagerClient::new(&env, &contract_id);

        let err = client.try_request_status(&id(&env, 0));
        assert_eq!(err, Err(Ok(RecoveryManagerError::NotInitialized)));
    }

    #[test]
    fn double_initialize_is_rejected() {
        let (env, client, _admin) = setup();
        let err = client.try_initialize(&Address::generate(&env), &2);
        assert_eq!(err, Err(Ok(RecoveryManagerError::AlreadyInitialized)));
    }

    /// Best-practice review finding: `Initialized`/`Admin`/`GuardianThreshold`
    /// used to be separate persistent entries, each bumped individually,
    /// despite never being read or written independently of each other —
    /// unlike `Guardian(addr)`/`Request(id)`, which genuinely are per-entity
    /// and correctly stay persistent. Proves the migration to instance
    /// storage actually happened, and that touching only `GuardianThreshold`
    /// (via `propose_threshold_change`) is enough to refresh the whole
    /// instance's shared TTL.
    #[test]
    fn singleton_config_lives_in_instance_storage_with_one_shared_ttl() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RecoveryManager, ());
        let client = RecoveryManagerClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &2);

        env.as_contract(&contract_id, || {
            assert!(!env.storage().persistent().has(&DataKey::Initialized));
            assert!(!env.storage().persistent().has(&DataKey::Admin));
            assert!(!env.storage().persistent().has(&DataKey::GuardianThreshold));
            assert!(env.storage().instance().has(&DataKey::Initialized));
            assert!(env.storage().instance().has(&DataKey::Admin));
            assert!(env.storage().instance().has(&DataKey::GuardianThreshold));
        });

        env.ledger()
            .with_mut(|l| l.sequence_number += TTL_THRESHOLD_LEDGERS / 2);
        client.propose_threshold_change(&3);
        let instance_ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
        assert!(instance_ttl >= TTL_THRESHOLD_LEDGERS);
    }

    #[test]
    fn initialize_rejects_zero_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RecoveryManager, ());
        let client = RecoveryManagerClient::new(&env, &contract_id);

        let err = client.try_initialize(&Address::generate(&env), &0);
        assert_eq!(err, Err(Ok(RecoveryManagerError::InvalidThreshold)));
    }

    #[test]
    fn propose_threshold_change_rejects_zero() {
        let (_env, client, _admin) = setup();
        let err = client.try_propose_threshold_change(&0);
        assert_eq!(err, Err(Ok(RecoveryManagerError::InvalidThreshold)));
    }

    #[test]
    fn add_guardian_rejects_duplicate() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);

        let err = client.try_add_guardian(&guardian);
        assert_eq!(err, Err(Ok(RecoveryManagerError::GuardianAlreadyExists)));
    }

    #[test]
    fn is_guardian_reflects_registration_and_removal() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);

        assert!(!client.is_guardian(&guardian));
        client.add_guardian(&guardian);
        assert!(client.is_guardian(&guardian));

        client.propose_remove_guardian(&guardian);
        // Proposing removal does not take effect immediately — the
        // guardian remains fully registered until the delay elapses and
        // the removal is actually applied.
        assert!(client.is_guardian(&guardian));

        set_ledger(&env, GUARDIAN_CHANGE_DELAY_LEDGERS);
        client.apply_guardian_removal(&guardian);
        assert!(!client.is_guardian(&guardian));
    }

    #[test]
    fn open_recovery_rejects_duplicate_request_id() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 50);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);

        let err = client.try_open_recovery(&admin, &request_id, &Address::generate(&env), &17280);
        assert_eq!(err, Err(Ok(RecoveryManagerError::RequestAlreadyExists)));
    }

    /// Real, if generous, cap: a single request cannot accumulate more than
    /// `MAX_APPROVERS` distinct approvers, bounding the resources any one
    /// `finalize_recovery` call needs to walk.
    #[test]
    fn approve_recovery_rejects_beyond_max_approvers() {
        let (env, client, admin) = setup();
        let request_id = id(&env, 51);
        client.open_recovery(&admin, &request_id, &Address::generate(&env), &17280);
        set_ledger(&env, 17280);

        let mut guardians = std::vec::Vec::new();
        for _ in 0..MAX_APPROVERS {
            let guardian = Address::generate(&env);
            client.add_guardian(&guardian);
            guardians.push(guardian);
        }
        // Guardians just added activate at 17280 + GUARDIAN_ACTIVATION_DELAY_LEDGERS.
        set_ledger(&env, 17280 + GUARDIAN_ACTIVATION_DELAY_LEDGERS);
        for guardian in guardians.iter() {
            client.approve_recovery(&request_id, guardian);
        }

        let one_more = Address::generate(&env);
        client.add_guardian(&one_more);
        set_ledger(&env, 17280 + 2 * GUARDIAN_ACTIVATION_DELAY_LEDGERS);
        let err = client.try_approve_recovery(&request_id, &one_more);
        assert_eq!(err, Err(Ok(RecoveryManagerError::TooManyApprovers)));
    }

    /// Real gap this session found and fixed: `open_recovery` originally
    /// required the *admin's own* authorization, which makes recovery
    /// unusable in exactly the scenario it exists for — the owner's key is
    /// lost or destroyed, not merely held by someone who might misuse it.
    /// An active guardian must be able to open a request unassisted; the
    /// threshold and timelock (not who was allowed to open it) are what
    /// actually gate the outcome.
    #[test]
    fn active_guardian_can_open_recovery_without_admin() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        set_ledger(&env, GUARDIAN_ACTIVATION_DELAY_LEDGERS);

        let request_id = id(&env, 60);
        client.open_recovery(
            &guardian,
            &request_id,
            &Address::generate(&env),
            &(GUARDIAN_ACTIVATION_DELAY_LEDGERS + MIN_RECOVERY_DELAY_LEDGERS),
        );

        let status = client.request_status(&request_id);
        assert!(!status.finalized);
    }

    /// A guardian not yet past its activation delay cannot open a request
    /// either — the same activation gate applies uniformly to opening and
    /// approving, closing the same sybil-guardian path either way.
    #[test]
    fn not_yet_active_guardian_cannot_open_recovery() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        // Still ledger 0: guardian activates at GUARDIAN_ACTIVATION_DELAY_LEDGERS.

        let request_id = id(&env, 61);
        let err = client.try_open_recovery(
            &guardian,
            &request_id,
            &Address::generate(&env),
            &MIN_RECOVERY_DELAY_LEDGERS,
        );
        assert_eq!(err, Err(Ok(RecoveryManagerError::Unauthorized)));
    }

    /// A caller who is neither the admin nor any kind of guardian cannot
    /// open a recovery request.
    #[test]
    fn unrelated_third_party_cannot_open_recovery() {
        let (env, client, _admin) = setup();
        let stranger = Address::generate(&env);

        let request_id = id(&env, 62);
        let err = client.try_open_recovery(
            &stranger,
            &request_id,
            &Address::generate(&env),
            &MIN_RECOVERY_DELAY_LEDGERS,
        );
        assert_eq!(err, Err(Ok(RecoveryManagerError::Unauthorized)));
    }

    #[test]
    fn active_guardian_can_request_freeze_and_it_is_visible() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        set_ledger(&env, GUARDIAN_ACTIVATION_DELAY_LEDGERS);

        assert!(!client.guardian_freeze_requested());
        client.request_guardian_freeze(&guardian);
        assert!(client.guardian_freeze_requested());
    }

    /// Security review finding: `guardian_freeze_requested` alone never
    /// cleared the flag it read, so a caller relying only on that getter
    /// (as `smart_account::apply_guardian_freeze` used to) could act on an
    /// already-stale request indefinitely. `consume_guardian_freeze_request`
    /// must check and clear atomically: true exactly once, false after
    /// that, with no separate request in between.
    #[test]
    fn consume_guardian_freeze_request_clears_it_exactly_once() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        set_ledger(&env, GUARDIAN_ACTIVATION_DELAY_LEDGERS);
        client.request_guardian_freeze(&guardian);

        assert!(client.consume_guardian_freeze_request());
        assert!(!client.guardian_freeze_requested());
        assert!(!client.consume_guardian_freeze_request());
    }

    /// Consuming with nothing pending is a safe no-op, not an error --
    /// mirrors `guardian_freeze_requested()`'s own `unwrap_or(false)`
    /// default rather than panicking on absent state.
    #[test]
    fn consume_guardian_freeze_request_with_nothing_pending_returns_false() {
        let (_env, client, _admin) = setup();
        assert!(!client.consume_guardian_freeze_request());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4015)")]
    fn not_yet_active_guardian_cannot_request_freeze() {
        let (env, client, _admin) = setup();
        let guardian = Address::generate(&env);
        client.add_guardian(&guardian);
        // Still ledger 0, not yet active.
        client.request_guardian_freeze(&guardian);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4015)")]
    fn non_guardian_cannot_request_freeze() {
        let (env, client, _admin) = setup();
        let stranger = Address::generate(&env);
        client.request_guardian_freeze(&stranger);
    }
}
