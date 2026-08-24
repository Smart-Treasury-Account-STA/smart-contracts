# Factory-Deployed Treasury: Testnet Setup and Transaction Walkthrough

This is the live record of deploying `contracts/account_factory` to Stellar
testnet, using it to stand up one complete example treasury in a single
call, and then exercising every payment-authoring entrypoint the deployed
`smart_account` exposes — real transactions against real testnet state, not
simulated in isolation. This is a **separate, additive** deployment from
`docs/TESTNET_DEPLOYMENT.md` (which predates `account_factory` and was
deployed by hand, contract by contract, via `scripts/deploy_testnet.sh`);
that document's deployment is untouched by anything here.

`soroban-sdk 26.1.0`, `stellar 26.0.0`, `Test SDF Network ; September 2015`.

**Status: the critical finding this record surfaced is fixed and
re-verified live (§13).** §7.4 and §8 below are kept as the original,
unedited record of the bug as first found — read them for the diagnosis;
read §13 for the fix and its live-testnet proof, on a second,
independently deployed treasury running the fixed contract code.

## 1. Why a factory deployment, and what this proves

`docs/TESTNET_DEPLOYMENT.md` proved the hand-wired path works. This
document proves the *self-service* path does: one caller, one signature,
one transaction, a fully wired treasury back — no manual sequencing of
seven separate deployments and ten-plus `invoke` calls. It then goes
further than the original deployment record on one axis: it exercises
**every** `smart_account` entrypoint that authors a payment or automation
(`execute_transfer_payment`, `execute_split_payment`,
`create_scheduled_payment`, `cancel_scheduled_payment`,
`execute_scheduled_payment`), not just the interactive transfer path — and
in doing so, surfaced a genuine, previously-undetected bug (§7.4–§8),
**now fixed and re-verified live (§13)**.

## 2. WASM Artifacts

Built with `stellar contract build --optimize --out-dir wasm` from the
current `feat/governance-account-and-security-hardening` source (the same
tree the fourth security-review pass just verified: 164 tests, clean
clippy/fmt):

| Contract | WASM hash |
|---|---|
| `policy_engine` | `f05348d6cbe90796c2b89296697f340d2fba6a1b070f3830f17edf32c2cb8eb8` |
| `intent_registry` | `43567e68bc87211cbdcd08a13e8a6de094ed978261b59b07d942a831ce527b70` |
| `recovery_manager` | `0eb4038ea129be586d5b324c62c672c8203547b61146f17452693e587a9f92d8` |
| `transfer_adapter` | `0b6552600b480467c835467f4a2afb5385fa0fb71cb48369ecfc87f190568ab2` |
| `split_adapter` | `3b8effbc2cc2bfcad697d677a5fc29e48eb6c325a31941e65c6eb54ffc995686` |
| `smart_account` | `d39eb5bdb86b78dfcfd23682f50740615555b042c0c6f34a19fb2e830c5fb03c` |
| `account_factory` | `15f635356ddf3497788d4f6aab7f6e93cd2fbdaab36ecbe77d0c5521e403a3dd` |

The first six were `stellar contract upload`ed individually (install-only,
no instance) so `account_factory` can reference them by hash in
`deploy_account`'s `WasmHashes`; each upload's returned hash matched the
local build exactly — confirmed byte-for-byte, the same guarantee
`account_factory`'s CI staleness check enforces for its own fixtures.

## 3. Contract Addresses

| Contract | Testnet contract ID | Explorer |
|---|---|---|
| `account_factory` | `CAQQTRRYNXIQGFVNCTMTBJDXW3PN7O44KPT7GWCCE4FRKTOHDBCWGUZO` | [Explorer](https://stellar.expert/explorer/testnet/contract/CAQQTRRYNXIQGFVNCTMTBJDXW3PN7O44KPT7GWCCE4FRKTOHDBCWGUZO) |
| `smart_account` (deployed) | `CDD2WLADWV5JACV4UOPA5K3DSH7UR24EVG352BAHDEEYVMQR4WSX2ZRZ` | [Explorer](https://stellar.expert/explorer/testnet/contract/CDD2WLADWV5JACV4UOPA5K3DSH7UR24EVG352BAHDEEYVMQR4WSX2ZRZ) |
| `policy_engine` (deployed) | `CBTLRRVOAGT7BC3UA7YZSZI6EFCXBL3ZL4JN75VTT6ILGUA6LLIMK4WH` | [Explorer](https://stellar.expert/explorer/testnet/contract/CBTLRRVOAGT7BC3UA7YZSZI6EFCXBL3ZL4JN75VTT6ILGUA6LLIMK4WH) |
| `intent_registry` (deployed) | `CCCWE2AFNNRHJY7KKGMUJ7EUKYASO7X4YFGRCNY4WGNBDOLMMRGGZVQ3` | [Explorer](https://stellar.expert/explorer/testnet/contract/CCCWE2AFNNRHJY7KKGMUJ7EUKYASO7X4YFGRCNY4WGNBDOLMMRGGZVQ3) |
| `recovery_manager` (deployed) | `CD6YUL7YWXYHPD4V72OKBF2WGSH75CTVXADY37KGZYGEZ4H6S2CYO2GA` | [Explorer](https://stellar.expert/explorer/testnet/contract/CD6YUL7YWXYHPD4V72OKBF2WGSH75CTVXADY37KGZYGEZ4H6S2CYO2GA) |
| `transfer_adapter` (deployed) | `CA4B52LFWN2KQ6PPHOCTTSKOK2K4IBKGGUTK5MCEEKWTB3ZESJRVVOKO` | [Explorer](https://stellar.expert/explorer/testnet/contract/CA4B52LFWN2KQ6PPHOCTTSKOK2K4IBKGGUTK5MCEEKWTB3ZESJRVVOKO) |
| `split_adapter` (deployed) | `CCQEUUJRBZZNWMYQU67B5BOAF7RJZ4Q6NBEPYMX6WML6EJNVFQ3SZYN7` | [Explorer](https://stellar.expert/explorer/testnet/contract/CCQEUUJRBZZNWMYQU67B5BOAF7RJZ4Q6NBEPYMX6WML6EJNVFQ3SZYN7) |
| `STA` test asset (SAC) | `CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L` | reused from `docs/TESTNET_DEPLOYMENT.md` — same issuer, same deterministic SAC address |

The six "(deployed)" addresses are the output of **one** `deploy_account`
call (§5) — none were deployed individually.

### Identities

| Role | Identity | Address |
|---|---|---|
| Factory admin / WASM uploader | `sta-testnet-deployer` (reused) | `GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB` |
| Treasury owner / founding signer | `sta-testnet-factory-owner` (new) | `GDU3LDGZOCJIB6MIKQWK5AICFFEWVC2FJ2BDEVXG37XXSKWIZ7OXVCAS` |
| Guardian | `sta-testnet-guardian` (reused) | `GDXVIRLSBDKT7EZM2RM3FH26W3TPF77IJ7GZBA5IOA6ZJBTW26NNO3AV` |
| Recipient 1 | `sta-testnet-recipient` (reused) | `GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU` |
| Recipient 2 | `sta-testnet-factory-recipient2` (new) | `GBR3Q5ZGBZC5IIUAWMNDXNMWWTISFRNVKCIOO5L7U44GOZJMOVR467VM` |
| Scheduled-payment relayer | `sta-testnet-relayer` (reused) | `GBFOESUTANPJZVQUZVKC5YCS4FB6AE5SX2R3JEDRK5JHKR22JXV4VNIV` |
| Recovery replacement-owner target | `sta-testnet-factory-recovered-owner` (new) | `GBF7NXHS7YKXFA4D2XBFAWPPHUBMUKSES2QHE7XHENPIJE6HZU7AFNP6` |

"Reused" identities are the same ones already funded and recorded in
`docs/TESTNET_DEPLOYMENT.md`; "new" ones were generated and friendbot-funded
for this run.

## 4. `account_factory` deployment and initialization

```
$ stellar contract deploy --wasm wasm/sta_account_factory.wasm --source sta-testnet-deployer --network testnet
```
Upload: [`4d518e8d...`](https://stellar.expert/explorer/testnet/tx/4d518e8d8a08562f7390b1b7299700c6e061a85a8ae209d93fe32cde18547719) · Instance create: [`0b47c9e5...`](https://stellar.expert/explorer/testnet/tx/0b47c9e54617fef11ae5b659f90ab493aca4c2c61ae365418329f14bedde2b35)

```
$ stellar contract invoke --id CAQQTRRYNXIQGFVNCTMTBJDXW3PN7O44KPT7GWCCE4FRKTOHDBCWGUZO --source sta-testnet-deployer --network testnet -- \
  initialize --admin GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB \
  --wasm_hashes '{"policy_engine":"f05348d6...","intent_registry":"43567e68...","recovery_manager":"0eb4038e...","transfer_adapter":"0b655260...","split_adapter":"3b8effbc...","smart_account":"d39eb5bd..."}'
```
[`5b0e21ae...`](https://stellar.expert/explorer/testnet/tx/5b0e21aef3e6bc77256edd6fda0cd779c0afec5514bd4d77a4d9b5a1d9bf9026) — `Initialized(admin)` event.

## 5. `deploy_account`: one call, one signature, a complete treasury

```
$ stellar contract invoke --id CAQQTRRYNXIQGFVNCTMTBJDXW3PN7O44KPT7GWCCE4FRKTOHDBCWGUZO --source sta-testnet-factory-owner --network testnet -- \
  deploy_account \
  --caller GDU3LDGZOCJIB6MIKQWK5AICFFEWVC2FJ2BDEVXG37XXSKWIZ7OXVCAS \
  --salt d70b7b01a8f980932e550fe19b00333699bfe481932ee54278addd00cf7d35af \
  --initial_signers '[{"Delegated":"GDU3LDGZOCJIB6MIKQWK5AICFFEWVC2FJ2BDEVXG37XXSKWIZ7OXVCAS"}]' \
  --initial_policies '{}' \
  --guardian_threshold 1 \
  --executor GBFOESUTANPJZVQUZVKC5YCS4FB6AE5SX2R3JEDRK5JHKR22JXV4VNIV
```

[`07a86c8f...`](https://stellar.expert/explorer/testnet/tx/07a86c8fe8b00788c356a8f583cf6f99f8f04da641469121496b5812f00055a7) — **one transaction, one `caller.require_auth()`**, deploying and wiring
`policy_engine`, `intent_registry` (left uninitialized, then bootstrapped
by `smart_account.initialize`'s own invoker-shortcut), `recovery_manager`,
`transfer_adapter`, `split_adapter`, and `smart_account` — adapters already
bound (no timelock, first-ever binding), guardian threshold set to `1`,
founding signer `Signer::Delegated(owner)` registered under context rule
`0`. Events observed, in order: five `Initialized(init)` events (one per
sub-contract other than `smart_account`), `signer_registered`,
`context_rule_added`, `smart_account`'s own `Initialized(init)`, and the
factory's `AccountDeployed(caller, smart_account)`.

Compare this to `docs/TESTNET_DEPLOYMENT.md` §4–§5: the same end state
(six wired contracts, one founding signer, adapters bound) that document
built across **fourteen** separate transactions now takes **one**.

## 6. Post-deploy configuration

The factory intentionally leaves policy rules, guardians, and funding to
the caller — those are treasury-specific operational decisions, not
deployment-time defaults. All owner-authorized (plain `Address::require_auth()`,
no custom `AuthPayload` needed):

| Step | Transaction |
|---|---|
| `policy_engine.set_asset_rule` (STA enabled, max single transfer 10,000,000) | [`e5a99a02...`](https://stellar.expert/explorer/testnet/tx/e5a99a0283ae759786ed7f04650a2e3983e6ea21ae5a9d674d826aa42b48b781) |
| `policy_engine.set_recipient_allowed` (recipient 1) | [`26a93c87...`](https://stellar.expert/explorer/testnet/tx/26a93c870ced50619806c2a6f6548d1117c9e7f422a6f17530c5f02bd955e470) |
| `policy_engine.set_recipient_allowed` (recipient 2) | [`5fff17f1...`](https://stellar.expert/explorer/testnet/tx/5fff17f1de9be715ce8a42ace8a83de65ca3ca357453b1a4e8b703762377d29b) |
| `policy_engine.set_operation_allowed(transfer)` | [`f6894b84...`](https://stellar.expert/explorer/testnet/tx/f6894b84e23739f9b94b46e7555ed1dac75965db482e8dc393a673550f7b258e) |
| `policy_engine.set_operation_allowed(split)` | [`4a75771f...`](https://stellar.expert/explorer/testnet/tx/4a75771f6b87d07e16982465097923ea63a1e7ed3d58ba5baea69ad918d0a09a) |
| `recovery_manager.add_guardian` (activates at ledger `4318327`, ~1 day) | [`513b095f...`](https://stellar.expert/explorer/testnet/tx/513b095fe201c5d86848414908b247254b00dd3c413f635e259fde615e1d440b) |

Funding hit a real, undocumented (in the original deployment) wrinkle: the
`STA` SAC's issuer has `AUTH_REQUIRED`/`AUTH_REVOCABLE` set (confirmed via
Horizon), so **every new holder** — including a brand-new `smart_account`
contract address — needs an explicit issuer `set_authorized` call before it
can hold a balance at all; a classic account additionally needs its own
`change_trust` first. This wasn't visible in `docs/TESTNET_DEPLOYMENT.md`
because that deployment reused already-authorized identities throughout.

| Step | Transaction |
|---|---|
| `STA.set_authorized(smart_account, true)` | [`bfdb13e4...`](https://stellar.expert/explorer/testnet/tx/bfdb13e4089f9541b81074e8fff5396f1f1c454a5bc0967f778fba34c336c562) |
| `STA.set_authorized(recipient 1, true)` | [`4ba75f7a...`](https://stellar.expert/explorer/testnet/tx/4ba75f7ade7387ea685c55e9c1356dda7340ee0ce29196b125a771d3b21fbea1) |
| recipient 2 `change_trust(STA)` (classic op) | [`7b9c18ce...`](https://stellar.expert/explorer/testnet/tx/7b9c18ce5eb08a2c47575ee263acb2484365ac573393074cd95e9abf2e4bae1a) |
| `STA.set_authorized(recipient 2, true)` | [`f3ba72ea...`](https://stellar.expert/explorer/testnet/tx/f3ba72ea1e1e2211d04dfb56aa9422666bf302cf27725a988038bdbfc3132175) |
| `STA.mint(smart_account, 1,000,000,000)` | [`f3ac024b...`](https://stellar.expert/explorer/testnet/tx/f3ac024b9e791547e7af19514cda7bebb152d756ab31c0511a20b8579472afe7) |

## 7. Payment entrypoints: real, signer-authorized transactions

All four use a new generalized script, `scripts/execute_signer_authorized_call.py`,
hand-building the same `AuthPayload`/`do_check_auth`/`authenticate` custom-account
authorization `docs/TESTNET_DEPLOYMENT.md` §6.3–§6.4 and
`scripts/bootstrap_intent_registry.py`/`execute_demo_transfer_payment.py`
already documented — generalized to one script covering all four call
shapes instead of duplicating the auth-entry plumbing per call.

### 7.1 `execute_transfer_payment` — real SAC transfer

```
$ python3 scripts/execute_signer_authorized_call.py --call transfer \
    --smart-account CDD2WLADWV5JACV4UOPA5K3DSH7UR24EVG352BAHDEEYVMQR4WSX2ZRZ \
    --asset CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L \
    --destination GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU \
    --amount 5000000 --nonce 1 --expected-policy-version 1
```
[`aef9b355...`](https://stellar.expert/explorer/testnet/tx/aef9b355342123df3683c44cf8cbb8aa7c43c37c697eeb9565b5f95c02645614) — 5,000,000 STA moved from the treasury to recipient 1.

### 7.2 `execute_split_payment` — real one-to-many SAC split

```
$ python3 scripts/execute_signer_authorized_call.py --call split \
    --smart-account CDD2WLADWV5JACV4UOPA5K3DSH7UR24EVG352BAHDEEYVMQR4WSX2ZRZ \
    --asset CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L \
    --recipients GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU,GBR3Q5ZGBZC5IIUAWMNDXNMWWTISFRNVKCIOO5L7U44GOZJMOVR467VM \
    --amounts 2000000,3000000 --nonce 2 --expected-policy-version 1
```
[`6b90af94...`](https://stellar.expert/explorer/testnet/tx/6b90af94bbaaa8596b2a4975742c61d7c80fc1f4556779b197b54964a1682791) — 2,000,000 STA to recipient 1, 3,000,000 STA to recipient 2, in one transaction. The generalized script declares one `SAC.transfer` sub-invocation per recipient under the same `AuthPayload` entry (extending the single-recipient pattern §7.1 uses).

### 7.3 `create_scheduled_payment` / `cancel_scheduled_payment` — real, work cleanly

```
$ python3 scripts/execute_signer_authorized_call.py --call create_scheduled \
    --smart-account CDD2WLADWV5JACV4UOPA5K3DSH7UR24EVG352BAHDEEYVMQR4WSX2ZRZ \
    --asset CCOUVA654... --destination GAK3XILR... --amount 1000000 \
    --intent-id-hex 849873c8... --start-ledger <now+5> --end-ledger <now+5000> \
    --interval-ledgers 0 --max-executions 3 --placeholder-adapter CA4B52LF...
```
[`310bba47...`](https://stellar.expert/explorer/testnet/tx/310bba472866dafb1d5b72372bd0f1a30a44593526eb9379d578068a00e50b46) — intent created. A second intent, created then immediately cancelled to prove `cancel_scheduled_payment` too:
create [`4fb4e60f...`](https://stellar.expert/explorer/testnet/tx/4fb4e60f466c8d9df3ebf460054868187a7759660959ac6d7c00dfc648c3e211), cancel [`39b331ae...`](https://stellar.expert/explorer/testnet/tx/39b331ae8a7b83a16f12f759be04921de93188adb3a74fa20c6210926d229fa9).

Both calls need only `smart_account`'s own `AuthPayload` — no sub-invocations at all, since `intent_registry`'s `admin.require_auth()` (`admin == smart_account`) is satisfied by Soroban's invoker-shortcut, the same mechanism `smart_account::initialize`'s own bootstrap uses.

### 7.4 `execute_scheduled_payment` — **did not work on first deployment**, real bug found (fixed, see §13)

See §8 — this is the one entrypoint that could not be completed on this first treasury, and not because of a script mistake. **Now fixed** — §13 re-runs this exact call on a second, independently deployed treasury running the fixed contract code, and it succeeds.

## 8. Real bug found: `execute_scheduled_payment` cannot move funds through an adapter (historical — fixed in §13)

**This section describes the bug as originally found, on the first treasury deployed in §5, before the fix.** It is kept unedited as the diagnosis record. See §13 for the fix itself and its live re-verification.

Executing intent `849873c8...`'s first child, as the configured relayer:

```
$ stellar contract invoke --id CDD2WLADWV5JACV4UOPA5K3DSH7UR24EVG352BAHDEEYVMQR4WSX2ZRZ --source sta-testnet-relayer --network testnet -- \
  execute_scheduled_payment --intent_id 849873c8... --child_sequence 1
❌ error: transaction simulation failed: HostError: Error(Auth, InvalidAction)
```

Two authorization problems, discovered in sequence:

1. **Non-root relayer auth.** `execute_scheduled_payment` has no
   `require_auth()` gate of its own (deliberately permissionless — the
   signer approval already happened at `create_scheduled_payment` time).
   The only authorization anywhere in this call graph is
   `intent_registry::ensure_executor`'s `executor.require_auth()`, two
   levels deep (`execute_scheduled_payment -> intent_registry
   .mark_child_executed -> ensure_executor`). Soroban's `SourceAccount`
   credentials — what the bare `stellar contract invoke --source <id>` CLI
   auto-generates — only cover `require_auth()` calls at the ROOT of the
   invocation tree; a non-root call, even by a plain Ed25519 account,
   needs an explicit signed `Address` credentials entry. Fixed by building
   one (`scripts/execute_scheduled_payment_as_relayer.py`).

2. **A hard Soroban reentrancy wall — not fixable from the client side.**
   With the relayer's entry correct, `mark_child_executed` succeeds (confirmed
   in the simulation diagnostics: `execution_count` advances to `1`), but
   the transaction still fails once it reaches
   `transfer_adapter.execute_transfer`'s nested `SAC.transfer(from=smart_account, ...)`
   call. That call's `from.require_auth()` targets `smart_account` itself —
   but `smart_account` is a **custom account**, and by this point its own
   `execute_scheduled_payment` frame is already active on the call stack
   (it never authenticated itself at the top, unlike `execute_transfer_payment`,
   which calls `env.current_contract_address().require_auth()` as its
   first statement). Asking the host to invoke `smart_account.__check_auth`
   again here is a genuine reentrant call into a contract that is already
   mid-execution on the stack, and Soroban rejects it outright:

   ```
   Error(Context, InvalidAction), data: "Contract re-entry is not allowed"
   Error(Auth, InvalidAction), data: "failed account authentication with error"
   ```

   This is not an artifact of how the demo script builds authorization
   entries — no valid `SorobanAuthorizationEntry` construction can satisfy
   it, because the constraint is enforced by the host's call-stack
   tracking, not by anything expressible in an auth entry. The interactive
   path (`execute_transfer_payment`) avoids this specific trap only because
   its own root-level self-`require_auth()` establishes a single
   already-open authorization session that its later `SAC.transfer`
   sub-invocation can ride on (one `__check_auth` call covering the whole
   declared subtree); `execute_scheduled_payment` never opens that session
   at all, and by the time the SAC needs `smart_account`'s authorization,
   there is no way to open one without re-entering a contract that is
   already on the stack.

**Consequence:** as currently implemented, a scheduled/recurring payment
can be created and cancelled, but **its actual execution — the one step
that moves funds — cannot complete**, for any adapter that performs a real
`SAC.transfer(from=smart_account, ...)`. This is invisible to the local
integration suite because every test in `contracts/smart_account/src/test.rs`
uses `env.mock_all_auths()`, which bypasses real authorization (and
therefore this reentrancy check) entirely — a real gap in test coverage,
not just in the contract. Balance evidence confirms no funds moved for
this attempt: the treasury's SAC balance after §7.1–§7.2 is exactly
`1,000,000,000 - 5,000,000 - 5,000,000 = 990,000,000`, with nothing
further deducted.

**Likely fix** (not implemented here — this is a real design change, out
of scope for a deployment/testing pass): have `transfer_adapter`/`split_adapter`
move funds via a pre-authorized SAC allowance (`approve` once, with real
signer authorization, then `transfer_from` at execution) instead of a raw
`transfer` that needs `smart_account`'s live re-authentication. Under
`transfer_from(spender=transfer_adapter, from=smart_account, ...)`, the
`require_auth()` call is on `spender` (`transfer_adapter`), which the
invoker shortcut already satisfies for free — `smart_account` would never
need to re-authenticate itself mid-execution at all. This is a genuine
design question, not a bug fix in the usual sense (approval sizing,
revocation, and per-schedule scoping all need real thought), so it is
recorded here rather than attempted unilaterally.

## 9. Rejected-payment demonstrations, live

Reusing §7.1's already-consumed `nonce = 1`, and two fresh calls with a
stale policy version and an over-cap amount — all caught at
`simulateTransaction`, with the *same* real signer authorization used for
§7.1 (so these are genuine policy/replay rejections, not auth failures):

| Call | Result |
|---|---|
| `nonce = 1` again (already consumed by §7.1) | ❌ `Error(Contract, #8005)` (`NonceAlreadyUsed`) |
| Fresh `nonce = 555`, `expected_policy_version = 99` (stale) | ❌ `Error(Contract, #2006)` (`VersionMismatch`) |
| Fresh `nonce = 556`, `amount = 20,000,000` (cap is 10,000,000) | ❌ `Error(Contract, #2005)` (`AmountAboveLimit`) |

## 10. Guardian and recovery flow

Every entrypoint here gates on a **plain** `Address::require_auth()`
(guardian or owner, both ordinary keypairs) — no custom `AuthPayload`
needed, so the bare `stellar` CLI drives all of it directly.

`recovery_manager` enforces two independent ~1 day (`17280`-ledger) delays
by design (`MIN_RECOVERY_DELAY_LEDGERS`, `GUARDIAN_ACTIVATION_DELAY_LEDGERS`)
— a freshly registered guardian cannot approve, freeze, or open anything
until its activation ledger passes, and a freshly opened recovery cannot
finalize before its `earliest_ledger`. This deployment's guardian was only
just added (§6), so this section demonstrates what's reachable *before*
those delays elapse, live, plus the delay enforcement itself:

| Step | Result |
|---|---|
| `request_guardian_freeze` (guardian not yet active) | ❌ `Error(Contract, #4015)` (`Unauthorized`) — expected, by design |
| `apply_guardian_freeze` (nothing was requested) | ❌ `Error(Contract, #8012)` (`GuardianFreezeNotRequested`) — expected, since the above never succeeded |
| `open_recovery` (owner-authorized; owner can open without waiting on guardian activation) | ✅ [`8b4d299d...`](https://stellar.expert/explorer/testnet/tx/8b4d299dcf17296a7388f7c1f6090a21c60d9014025181433fcc68cb6a4dbaa8) — request `73efba2a...`, replacement owner `GBF7NXHS...`, `earliest_ledger = 4318658` |
| `request_status` read-back | ✅ Confirms the request: `finalized: false, cancelled: false, approvers: []` |
| `approve_recovery` (guardian not yet active) | ❌ `Error(Contract, #4014)` (`GuardianNotYetActive`) — expected, by design |
| `propose_threshold_change(1)` (owner-authorized, no guardian dependency) | ✅ [`04778a88...`](https://stellar.expert/explorer/testnet/tx/04778a88d48a5456ef94ff64b54aec88af1b1cf6898bbd92de91951188a29958) — effective at ledger `4318647` |

**Not yet completed, by design:** `approve_recovery`, `finalize_recovery`,
`smart_account.apply_recovery`, a real `request_guardian_freeze` +
`apply_guardian_freeze`, and `apply_threshold_change` all require waiting
past the guardian's activation ledger (`4318327`) and/or the recovery
request's `earliest_ledger` (`4318658`) — genuinely ~1 real day away, the
same delay `docs/TESTNET_DEPLOYMENT.md` §5 hit for its adapter timelock
(there, resolved by returning to apply it two days later). The two rejected
calls above prove the enforcement is real and live, not merely asserted by
the local test suite.

## 11. Final state

| | Before this run | After |
|---|---|---|
| `smart_account` STA balance | `0` (unfunded) | `990,000,000` (`1,000,000,000` minted − `5,000,000` transfer − `5,000,000` split; nothing further moved, since §7.4/§8 never completed) |
| `nonce 1`, `nonce 2` | unused | both `used` |
| `smart_account.status()` | uninitialized | `{"frozen":false,"initialized":true,"paused":false,"policy_version_hint":0}` |
| Recovery request `73efba2a...` | — | open, unapproved, unfinalized |
| Guardian threshold change | — | proposed (`1`), pending |

## 12. Reproduction

```bash
stellar contract build --optimize --out-dir wasm
# upload the 6 sub-contracts, deploy + initialize account_factory (§4)
# deploy_account (§5), then §6's policy/guardian/funding calls
python3 scripts/execute_signer_authorized_call.py --call transfer ...
python3 scripts/execute_signer_authorized_call.py --call split ...
python3 scripts/execute_signer_authorized_call.py --call create_scheduled ...
python3 scripts/execute_signer_authorized_call.py --call cancel_scheduled ...
python3 scripts/execute_scheduled_payment_as_relayer.py ...   # now succeeds -- see §13
```

`scripts/execute_signer_authorized_call.py` and
`scripts/execute_scheduled_payment_as_relayer.py` are new, general-purpose
additions alongside the existing `scripts/execute_demo_transfer_payment.py`
and `scripts/bootstrap_intent_registry.py` — all four share the same
`AuthPayload`/`do_check_auth` construction, documented once in
`execute_demo_transfer_payment.py`'s docstring and extended in each script
that builds on it. Both scripts were simplified after the §13 fix: neither
needs to declare a `SAC.transfer` sub-invocation or a `smart_account`
`AuthPayload` entry for the scheduled-payment execution path any more (see
their current docstrings for why).

## 13. Fix verified live: a second treasury, same call, now succeeds

`docs/SECURITY_REVIEW_STRICT.md` finding 29 fixes §8's bug:
`transfer_adapter`/`split_adapter` now draw funds via the SAC's
`transfer_from(spender, from, to, amount)` instead of its plain
`transfer(from, to, amount)`. `transfer_from`'s authorization is on
`spender` — the adapter's own address — satisfied by Soroban's
invoker-shortcut (the adapter is the SAC's direct caller for this call),
never `smart_account`'s. `smart_account` grants the adapter an exact,
one-shot allowance immediately before delegating to it, via a new
`approve_adapter` helper used by all three of `execute_transfer_payment`,
`execute_split_payment`, and `execute_scheduled_payment` — that `approve`
call is *also* invoker-shortcut-satisfied (`smart_account` is the SAC's
direct caller for `approve`), so it triggers no `__check_auth` invocation
at all, and therefore never risks the reentrancy §8 hit, regardless of
whether `smart_account`'s own frame is already active on the stack.

### 13.1 Rebuild, re-upload, and point `account_factory` at the fixed code

```
$ stellar contract build --optimize --out-dir wasm
```
Only three of the six sub-contract hashes changed (`policy_engine`,
`intent_registry`, `recovery_manager` were untouched by this fix):

| Contract | New WASM hash |
|---|---|
| `transfer_adapter` | `785870924ad04930512a5494cbf442b159c912a1f233491d1b0f696fe19f4dae` |
| `split_adapter` | `e32ad0c7bf83d265848adc2ee4874aa0b3d27fb9b97558c4f8753b40a0ba329b` |
| `smart_account` | `886bcd312f3cf973a8a037479ebf9febe57139e63f109947acce20bd835f3a76` |

All three re-uploaded ([`198308ea...`](https://stellar.expert/explorer/testnet/tx/198308ea4eec1cee590dfb6645af1b9ca80044e67fff794dc06da9ff47d74ebf), [`b49b0e7c...`](https://stellar.expert/explorer/testnet/tx/b49b0e7c3801044daffbbf8d237c9786a2154f997bfe8ef375a7060a29f20d78), [`e0f8db1e...`](https://stellar.expert/explorer/testnet/tx/e0f8db1eb153cd15dac9e32952a873cdcee685653284abfcda816efe58e5b34d)), each matching the local build byte-for-byte. `contracts/account_factory/src/wasm_fixtures/*.wasm` updated to match (the tracked fixtures CI checks — §2). The existing `account_factory` instance (§3–§4) doesn't need redeploying — its whole purpose is exactly this: `set_wasm_hashes` points all *future* `deploy_account` calls at the new code, deployed treasuries are unaffected either way:

```
$ stellar contract invoke --id CAQQTRRYNXIQGFVNCTMTBJDXW3PN7O44KPT7GWCCE4FRKTOHDBCWGUZO --source sta-testnet-deployer --network testnet -- \
  set_wasm_hashes --wasm_hashes '{"policy_engine":"f05348d6...","intent_registry":"43567e68...","recovery_manager":"0eb4038e...","transfer_adapter":"78587092...","split_adapter":"e32ad0c7...","smart_account":"886bcd31..."}'
```
[`9dbcc8fa...`](https://stellar.expert/explorer/testnet/tx/9dbcc8fa4c37e65d61a77995eedf3839069811e4585fa3312a85e0de1b74efcf) — `WasmHashesUpdated` event.

### 13.2 A second treasury, deployed with the fixed code

Same `deploy_account` flow as §5, same caller and executor, fresh salt:

[`744150f4...`](https://stellar.expert/explorer/testnet/tx/744150f4671e7bad81454ac09ad2927d9205e8659d2bf9df34d13209ba171395) —

| Contract | Address |
|---|---|
| `smart_account` | `CD6GY4UUTNPW4TUV7LDL5SELN4BBHJG4KDDT3W6G23DY6XCGM75MULMQ` |
| `policy_engine` | `CCOP7NRMST5K6TL7FBDMX25LDEPW3DSFBOGBIKVFIDNAAZY7GBMVP3M4` |
| `intent_registry` | `CAFIATSIZQSBILZJWVT4PVDXPVITJHLP6LPAVKDRHCA7I7XPZSLTRPUS` |
| `recovery_manager` | `CCHC4YKVYS3CAZUOUYWYTEMQ6TZDW75WB2BGENUC2CDWDX5RH7NMKZWU` |
| `transfer_adapter` | `CBRYGIR3ORDW5LE6J7AVPSKRNTMRUYHD6FVPHQMJGPQLQ5FQUZ2U6GFH` |
| `split_adapter` | `CBQA7UI7QN6RN4IZT7WPDHWTK2OO7J4FH2KMCVJGKMKVFGDURD63UQ7U` |

Configured identically to §6: `policy_engine` rules, guardian registered, `smart_account` authorized on the `STA` SAC and funded with `1,000,000,000`.

### 13.3 Payments re-tested — including the one that used to fail

Transfer and split, exactly as §7.1–§7.2, now with **no `SAC.transfer` sub-invocation declared at all** (the simplified script — §12):

| Call | Transaction |
|---|---|
| `execute_transfer_payment` (5,000,000 to recipient 1) | [`e9acc126...`](https://stellar.expert/explorer/testnet/tx/e9acc126c0a0612b6ae4fe3472a85aa1a9da18122067facd1c93a020b4df3b28) |
| `execute_split_payment` (2,000,000 / 3,000,000) | [`a97d05f0...`](https://stellar.expert/explorer/testnet/tx/a97d05f0d9585444054e8f8217e42f701f572728d6a2f172652b8328baa467ab) |
| `create_scheduled_payment` (1,000,000, single execution) | [`b3a00fbe...`](https://stellar.expert/explorer/testnet/tx/b3a00fbebeb595345f4ed4b34ccd556e07675130b68b8cf23076695f72c38808) |

Then the call that failed in §8, unchanged in every argument, run again against the fixed code:

```
$ stellar contract invoke --id CD6GY4UUTNPW4TUV7LDL5SELN4BBHJG4KDDT3W6G23DY6XCGM75MULMQ --source sta-testnet-relayer --network testnet -- \
  execute_scheduled_payment --intent_id ad961b1c... --child_sequence 1
```
✅ [`16608966...`](https://stellar.expert/explorer/testnet/tx/16608966fd79f94f8f76e1943e88b91cab2c88fb8dd786dbabb8d64e5d5da81c) — **succeeds**, with only the relayer's own `intent_registry.mark_child_executed` authorization entry; no `smart_account` entry of any kind is needed any more (`scripts/execute_scheduled_payment_as_relayer.py`, simplified — §12).

Balance and allowance evidence, not just a success status code:

| | Before this section | After |
|---|---|---|
| `smart_account` STA balance | `1,000,000,000` (freshly funded) | `988,000,000` (`-5,000,000` transfer `-5,000,000` split `-1,000,000` scheduled `-1,000,000` a second transfer rejection-demo control call — exactly accounted for) |
| `smart_account → transfer_adapter` SAC allowance | `0` | `0` — the `approve_adapter` grant was for the exact amount moved and fully consumed by `transfer_from` in the same transaction, nothing left outstanding |

The two rejected-payment demonstrations (§9) were re-run on this treasury too — `nonce = 1` replay (`Error(Contract, #8005)`) and an over-cap amount (`Error(Contract, #2005)`) — both still reject exactly as before; the fix changed *how* funds move, not the policy/replay guarantees around it.

### 13.4 Verification

`cargo build --workspace`, `cargo test --workspace` (164 passed, 0 failed — two `transfer_adapter`/`split_adapter` unit tests updated to grant the same allowance `smart_account` now grants in the real flow; `mock_all_auths()` cannot exercise the reentrancy this fix closes, so the real proof is §13.3 above, not a new unit test), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check` all clean after the fix. See `docs/SECURITY_REVIEW_STRICT.md` finding 29 for the full writeup.
