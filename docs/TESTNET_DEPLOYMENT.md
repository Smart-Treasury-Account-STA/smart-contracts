# Smart Treasury Account V1 Testnet Deployment

This is the live deployment record for the V1 workspace as it stood on 2026-07-23 — the version with all three independent-review findings fixed (adapter pinning, timelocked adapter/guardian changes, the instance-TTL fix on the timelock code itself) and every event migrated to `#[contractevent]`. This **replaces** the 2026-07-18 deployment, which predated all of that and is now stale relative to source (entrypoint names, event shapes, and TTL behavior all differed). `soroban-sdk 26.1.0`, OpenZeppelin Stellar contract composition throughout. The earlier PoC deployment (`smart_account_poc`, `policy_registry_poc`, `intent_registry_poc`, `recovery_guard_poc` on `soroban-sdk 22.0.1`) remains archived separately — see [§9](#9-prior-poc-deployment).

Reproduce with `scripts/deploy_testnet.sh` (idempotent-ish: re-running `initialize` calls against already-initialized contracts fails with `AlreadyInitialized`, which is expected; re-running the test-asset deploy step now looks up the existing deterministic SAC address instead of failing, a fix landed as part of this deployment — see §8).

## 1. Deployment Summary

| Field | Value |
|---|---|
| Network | Stellar Testnet |
| Network passphrase | `Test SDF Network ; September 2015` |
| Stellar CLI version | `stellar 26.0.0` |
| Deployment date | 2026-07-23 |
| Deployer identity | `sta-testnet-deployer` |
| Deployer public key | `GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB` |

The deployed contracts are the full V1 workspace: all 7 packages, composing real OpenZeppelin Stellar contracts (`stellar-accounts`, `stellar-access`, `stellar-contract-utils` `0.7.2`) for signer/passkey authentication, ownership, and pause state. `smart_account` is initialized with a real founding Ed25519 wallet signer (the deployer's own account, registered as `Signer::Delegated`), and `transfer_adapter`/`split_adapter` perform real Stellar Asset Contract transfers, not simulated ones.

## 2. Contract Addresses

| Contract | Testnet contract ID | Explorer |
|---|---|---|
| `webauthn_verifier` | `CD72MKTDNI3HLMNPA4YUOZGLT2NUWLVHTW3A7NQOXGLG7OKIKETSMXXR` | [Explorer](https://stellar.expert/explorer/testnet/contract/CD72MKTDNI3HLMNPA4YUOZGLT2NUWLVHTW3A7NQOXGLG7OKIKETSMXXR) |
| `policy_engine` | `CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC` | [Explorer](https://stellar.expert/explorer/testnet/contract/CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC) |
| `intent_registry` | `CDHTNPBXUMPCKUJ76HQ767MDRD4IVRRH4H5DOF4JUOO36QKSV4GXFRMR` | [Explorer](https://stellar.expert/explorer/testnet/contract/CDHTNPBXUMPCKUJ76HQ767MDRD4IVRRH4H5DOF4JUOO36QKSV4GXFRMR) |
| `recovery_manager` | `CALI5XJASA66LKZPF3ZF7HOLGOFUWZYIHB6SENXCZ5Y7QVT7UQKKR6UM` | [Explorer](https://stellar.expert/explorer/testnet/contract/CALI5XJASA66LKZPF3ZF7HOLGOFUWZYIHB6SENXCZ5Y7QVT7UQKKR6UM) |
| `smart_account` | `CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS` | [Explorer](https://stellar.expert/explorer/testnet/contract/CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS) |
| `transfer_adapter` | `CAX766XYR56WO7Y4HFOHYQUO5AIN5QLHAHKJ3DINXM2WUQY5UE7KGE26` | [Explorer](https://stellar.expert/explorer/testnet/contract/CAX766XYR56WO7Y4HFOHYQUO5AIN5QLHAHKJ3DINXM2WUQY5UE7KGE26) |
| `split_adapter` | `CAFTFU2E4MGZT6BLCVN2FAQB6GIBJRR5ICI7C7LZEMACBDUUTQKVHHV3` | [Explorer](https://stellar.expert/explorer/testnet/contract/CAFTFU2E4MGZT6BLCVN2FAQB6GIBJRR5ICI7C7LZEMACBDUUTQKVHHV3) |
| `STA` test asset (SAC) | `CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L` | [Explorer](https://stellar.expert/explorer/testnet/contract/CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L) |

The `STA` test asset kept the *same* contract ID as the 2026-07-18 deployment — SAC addresses are deterministic (derived from issuer + asset code), so redeploying the same code from the same deployer resolves to the same contract rather than creating a new one; this run looked it up and reused it rather than re-deploying.

Auxiliary testnet identities used only as `Address` values (not funded, not signing anything): guardian `GDXVIRLSBDKT7EZM2RM3FH26W3TPF77IJ7GZBA5IOA6ZJBTW26NNO3AV`, recipient `GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU` (same identities reused from the prior deployment).

## 3. WASM Artifacts

Built with `stellar contract build --optimize --out-dir wasm`:

| Contract | WASM hash |
|---|---|
| `webauthn_verifier` | `2a56f2fbdaa56f6b5f138ba05b868116ae105b32b31b7ed4ee122ce9902e67f8` (unchanged — source hasn't changed since the prior deployment) |
| `policy_engine` | `d5a566d8dec16bf4d2687a0b56d2d759b8e2d759c6000d7f739f45d0eb110191` |
| `intent_registry` | `e67dab201bce0ef769b3d02d9a5f0e23149b97e1f9d2ef9e5b84ad335b03fa31` |
| `recovery_manager` | `a0f22448c139298880af1b33e935ca6c5bf810f37f126e9d190eb466e86b7515` |
| `smart_account` | `1899840dd9c80a272c7141e82d97f88b29f011c1c06cf3fdab14afbdcaae1356` |
| `transfer_adapter` | `49d963dfbe1ce17dd309a4f6502eaf26298b7effa40ef156ed824a0bb28cc1e7` |
| `split_adapter` | `55d8356b78d1fa6a79fc0586d595b20e686495ba87664cde89d7a317a83d1d88` |

## 4. Deployment Transactions

Each deployment has two transactions: WASM upload, then contract instance creation. `webauthn_verifier`'s WASM was already uploaded from the prior deployment (identical source), so it only needed the instance-creation transaction.

| Contract | WASM upload | Contract create |
|---|---|---|
| `webauthn_verifier` | *(reused, already uploaded)* | [`0ef8a0e0...`](https://stellar.expert/explorer/testnet/tx/0ef8a0e0335e67a9b27e856ea90d7603a40ec1a3eaf8f915a732c206410b113e) |
| `policy_engine` | [`b08d7ee4...`](https://stellar.expert/explorer/testnet/tx/b08d7ee46ee9248886d1c6953b46aa7a6d04e130c81ccb18a162e2465eef6666) | [`77bec183...`](https://stellar.expert/explorer/testnet/tx/77bec1830d1c377d0a50d7ea857955c3959e7b1c1f431936c0ab68e12f9b0997) |
| `intent_registry` | [`b115fd30...`](https://stellar.expert/explorer/testnet/tx/b115fd30ea19c4e7c198f59b7238957d41b1603f2542141d0f27e783f66779e3) | [`d2f4a1c9...`](https://stellar.expert/explorer/testnet/tx/d2f4a1c9ddd07dc454be8206e86379d427d2543fd4c4edfad05c3abfe0e8f757) |
| `recovery_manager` | [`0174acb4...`](https://stellar.expert/explorer/testnet/tx/0174acb485cffef3c858d7154ccee1ac94da50e28c7b428706e796e32715afd1) | [`5e0edf45...`](https://stellar.expert/explorer/testnet/tx/5e0edf45983a9709698ca0cd25d823e34e5daa9883adbf3d802688805b317592) |
| `smart_account` | [`48fffb41...`](https://stellar.expert/explorer/testnet/tx/48fffb41934eccac559f0eea59abe1934d8e665f9bf9242268d3c83b79c0f48b) | [`38f93345...`](https://stellar.expert/explorer/testnet/tx/38f9334501d3aa0cf425a1580a9c22a4a6e885180759d9bf91f8009fe9dd733b) |
| `transfer_adapter` | [`f5e076be...`](https://stellar.expert/explorer/testnet/tx/f5e076beee3e5025550ae6e483d535b2dac47d382993df0015dfddf9121fd0fe) | [`b2968b47...`](https://stellar.expert/explorer/testnet/tx/b2968b476192df488bee0fd08d84ad9b5fe200ac8f648926afef2e66f5a4de6c) |
| `split_adapter` | [`ef904342...`](https://stellar.expert/explorer/testnet/tx/ef904342946d7e0b71034463742d3f6a92b7a38ece880af2b37d2c73e9ad777d) | [`5aef3b2c...`](https://stellar.expert/explorer/testnet/tx/5aef3b2c0c7b52c5713691eccdaee369583d0079d458c3aa997883456c2bf483) |

## 5. Initialization and Wiring Transactions

| Step | Transaction | Result |
|---|---|---|
| `policy_engine.initialize` | [`faa050d4...`](https://stellar.expert/explorer/testnet/tx/faa050d4f67fdf66fd533fb5a759cfd852748ee6e091040aaad877b2e70d362a) | Admin set to deployer, policy version initialized to `1` — `Initialized(admin)` event |
| `recovery_manager.initialize` | [`3c3b9dfd...`](https://stellar.expert/explorer/testnet/tx/3c3b9dfd3111d4c7a5c468281dfa6c393924b4aaf38f791134cdee18b7428625) | Admin set to deployer, guardian threshold set to `1` — `Initialized(admin, guardian_threshold)` event |
| `transfer_adapter.initialize` | [`0f3ac883...`](https://stellar.expert/explorer/testnet/tx/0f3ac883c86bbc4c441fb9ad3772c68ed4a46ef8295b4ea11829f28446e816ed) | Pinned to `smart_account` |
| `split_adapter.initialize` | [`7848933c...`](https://stellar.expert/explorer/testnet/tx/7848933c9f36016d93d55b2e32a2755457659a6ab6598985721a5f3efb627915) | Pinned to `smart_account` |
| `smart_account.initialize` | [`1f4fbd12...`](https://stellar.expert/explorer/testnet/tx/1f4fbd12802b0af5e54cff741ad22bb6877fd398a064dbf482fcf3ab7128c01c) | Owner set to deployer; founding signer registered as `Signer::Delegated(deployer)` under context rule `0`; `policy_engine`/`intent_registry`/`recovery_manager` addresses pinned — `SignerRegistered`, `ContextRuleAdded`, `Initialized(owner)` events |
| `smart_account.propose_adapter_change(transfer)` | [`8aac2ebf...`](https://stellar.expert/explorer/testnet/tx/8aac2ebf6696f95b48013473c87b5301a188b221699a175b31fdc4a70fbf475b) | `transfer_adapter` proposed; effective ledger `3768351` (~1 day timelock) |
| `smart_account.propose_adapter_change(split)` | [`5ffeb135...`](https://stellar.expert/explorer/testnet/tx/5ffeb1355fe29cb03f06355fc21ac3a4a53be5806685e506e3d519b369b1aca9) | `split_adapter` proposed; effective ledger `3768353` |
| `recovery_manager.add_guardian` | [`4aa6f6ac...`](https://stellar.expert/explorer/testnet/tx/4aa6f6aceb901e58bd79bce8437b4bb5983a343bedb42cc1a5629fdd8f64450d) | Guardian registered; activates at ledger `3768355` |
| Mint `STA` to `smart_account` | [`a4c7e17d...`](https://stellar.expert/explorer/testnet/tx/a4c7e17dbeeccad4a60fb72245581c251510f75cf061527dcc48f4d7ca1e947f) | `1,000,000,000` units minted to the treasury |
| `policy_engine.set_asset_rule` | [`541be3a0...`](https://stellar.expert/explorer/testnet/tx/541be3a057fa740f199ca6ae964fe340567e180003cb2889cb59c026748fc760) | `STA` test asset enabled, max single transfer `10,000,000` |
| `policy_engine.set_recipient_allowed` | [`124ecc8b...`](https://stellar.expert/explorer/testnet/tx/124ecc8b1debb655c60f525c346f077ee93273f82c2948cce6fcf56d51f1a7c9) | Test recipient allowed |
| `policy_engine.set_operation_allowed(transfer)` | [`bc986459...`](https://stellar.expert/explorer/testnet/tx/bc986459086610eddbc8b9db20024d3f9246f21c3bf21375935b503c6f319dcf) | `transfer` operation enabled |
| `policy_engine.set_operation_allowed(split)` | [`26d8d823...`](https://stellar.expert/explorer/testnet/tx/26d8d8239d81db149b90a86f2bef32f254ae0a5d6faa1f3eb28b90c60d606054) | `split` operation enabled |

Every event above was emitted via the new `#[contractevent]` structs (not the old raw-tuple `Events::publish` calls) — confirmed directly from the CLI's event-decoding output during this deployment, e.g. `Initialized (init), admin: "...", guardian_threshold: 1` for `recovery_manager`, matching the typed struct fields rather than an untyped tuple.

The two `propose_adapter_change` calls took effect once their timelock elapsed (2026-07-25, ledger `3795561` — past both `3768351`/`3768353`). Applied with:

| Step | Transaction | Result |
|---|---|---|
| `smart_account.apply_adapter_change(transfer)` | [`6257c3a5...`](https://stellar.expert/explorer/testnet/tx/6257c3a52127c70747603d32efe5ba71dbf54b5f341627982327b5a488151333) | `AdapterChanged(operation: "transfer", adapter: "CAX766XY...")` — permissionless, no auth required |
| `smart_account.apply_adapter_change(split)` | [`732ac748...`](https://stellar.expert/explorer/testnet/tx/732ac748be8e32bd86e1506456b7601d4e07dc63038aacb78ee6078797e6b3d4) | `AdapterChanged(operation: "split", adapter: "CAFTFU2E...")` |

Both adapters are now genuinely wired into `smart_account` — see §6.4 for a real payment executed through this exact path.

`intent_registry` was initially deployed uninitialized, then bootstrapped separately (admin = `smart_account`) — see §6.3 and [§7](#7-known-limitation-signer-gated-interactive-entrypoints).

## 6. Demonstrated Testnet Flows

### 6.1 Valid and invalid `policy_engine.validate_policy` calls, live on-chain

`validate_policy` is permissionless by design (any caller may check whether a hypothetical payment would pass — the actual authorization happens at `smart_account`), which makes it directly demonstrable via CLI without any signer setup. All calls below were read-only (`--send=no`, RPC simulation against live testnet state — no transaction hash, since nothing is written for a check that doesn't mutate storage):

| Check | Result |
|---|---|
| `transfer` of `5,000,000` of `STA` to the allowed recipient, version `1` | ✅ Success — `PolicyValidated (pol_ok)` event: `operation: "transfer", asset: "CCOUVA6...", destination: "GAK3XIL...", amount: "5000000", expected_version: 1` |
| Same, but destination = deployer (never allowlisted as a recipient) | ❌ Rejected: `Error(Contract, #2004)` (`RecipientNotAllowed`) |
| Same, but amount `20,000,000` (above the `10,000,000` cap) | ❌ Rejected: `Error(Contract, #2005)` (`AmountAboveLimit`) |

This concretely proves, on live testnet rather than only in the local test suite, that policy checks fail closed for both an unapproved recipient and an over-cap amount — and that the new `PolicyValidated` event (from the `#[contractevent]` migration) carries the full named-field data the old raw tuple didn't expose as clearly.

### 6.2 Read-only state checks, live on-chain

```bash
$ stellar contract invoke --id CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS ... -- status
{"frozen":false,"initialized":true,"paused":false,"policy_version_hint":0}

$ stellar contract invoke --id CALI5XJASA66LKZPF3ZF7HOLGOFUWZYIHB6SENXCZ5Y7QVT7UQKKR6UM ... -- is_guardian --guardian GDXVIRLSBDKT7EZM2RM3FH26W3TPF77IJ7GZBA5IOA6ZJBTW26NNO3AV
true

$ stellar contract invoke --id CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC ... -- version
1
```

All three read back exactly the state written during initialization/wiring above: the treasury is initialized, not paused, not frozen; the registered guardian is recognized; the policy version is still `1` (unchanged from initialization, as expected — nothing in this deployment bumped it).

### 6.3 `intent_registry` bootstrapped with a hand-built custom-account authorization

**This section is a historical record of this specific, already-deployed testnet instance.** `smart_account::initialize` now performs this same bootstrap itself, internally, via Soroban's invoker-shortcut — see its doc comment in `contracts/smart_account/src/lib.rs` and `docs/SECURITY_REVIEW_STRICT.md`. Any *new* deployment (via `scripts/deploy_testnet.sh` or `contracts/account_factory`) no longer needs `scripts/bootstrap_intent_registry.py` at all; it remains only as a reference for how this deployment's `intent_registry` (`CDHTNPBXUMPCKUJ76HQ767MDRD4IVRRH4H5DOF4JUOO36QKSV4GXFRMR`) actually got initialized, below.

`intent_registry`'s `admin` must be `smart_account` itself (`contracts/smart_account/src/lib.rs`'s `create_intent`/`cancel_intent` call `IntentRegistryClient::create_intent`/`cancel_intent` directly, both `ensure_admin`-gated on the intent_registry side). So `intent_registry.initialize(admin=smart_account)`'s `admin.require_auth()` requires authorization *from `smart_account`* — a Soroban custom account, not a plain keypair — which the bare `stellar` CLI cannot produce (see §7).

`scripts/bootstrap_intent_registry.py` closes this gap for this one bootstrapping call by hand-constructing the two Soroban authorization entries `do_check_auth`/`authenticate` (`stellar-accounts-0.7.2/src/smart_account/storage.rs`) actually require, without any wallet/SDK layer:

1. An `Address` credentials entry for `smart_account`, whose `signature` field is the contract's own `AuthPayload` struct — `{signers: {Signer::Delegated(deployer): <bytes, unchecked for delegated signers>}, context_rule_ids: [0]}` (context rule `0` is the `Default` rule from `smart_account.initialize`, matching any context).
2. A standard `Address` credentials entry for `deployer` (the registered `Signer::Delegated` wallet), authorizing the nested `deployer.require_auth_for_args((auth_digest,))` call that `authenticate()` makes on `smart_account`'s behalf, where `auth_digest = sha256(signature_payload || context_rule_ids.to_xdr())` — the digest the delegated signer actually has to sign, not the raw host-computed payload.

Both entries were built directly against the XDR types (`stellar_sdk.xdr`/`stellar_sdk.scval` in Python — no `authorize_entry`-style helper exists for a custom account shape in any SDK, since the shape is contract-specific), submitted in a single transaction:

```
$ python3 scripts/bootstrap_intent_registry.py \
    --smart-account CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS \
    --intent-registry CDHTNPBXUMPCKUJ76HQ767MDRD4IVRRH4H5DOF4JUOO36QKSV4GXFRMR
submitted: 2d63d6d16d4f9bf9ba34f3301b72cbff1e4ff44273a49f1bf899051063e683b2 SendTransactionStatus.PENDING
status: GetTransactionStatus.SUCCESS
intent_registry initialized. admin = CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS
```

[Explorer](https://stellar.expert/explorer/testnet/tx/2d63d6d16d4f9bf9ba34f3301b72cbff1e4ff44273a49f1bf899051063e683b2). Verified by re-invoking `initialize` afterward and getting `Error(Contract, #3000)` (`AlreadyInitialized`) — proof the first call actually succeeded rather than silently no-opping. This is a narrow, one-time substitute for the general wallet/SDK/relayer layer named in §7 — it only drives this single bootstrapping call, not arbitrary `smart_account` invocations.

### 6.4 A real signer-authorized SAC payment, executed on-chain

Everything in §6.1 exercised `policy_engine.validate_policy` directly — permissionless, no signer authorization involved. This section goes one step further: a real `smart_account.execute_transfer_payment` call, authorized by the treasury's actual registered signer, moving real (testnet) `STA` balance out of the treasury through `transfer_adapter`.

This needed two pieces of one-time setup beyond what §5 already did:

| Step | Transaction | Result |
|---|---|---|
| Fund `sta-testnet-recipient` (friendbot) | — | Recipient account created on testnet (it never existed on-ledger before this) |
| Recipient establishes an `STA` trustline | [`9fdf9442...`](https://stellar.expert/explorer/testnet/tx/9fdf9442950c86a8ed17a3eabc60eeabc15bbe1fb50dfebf91b377eaed40466e) | Classic `change_trust` — required before any SAC can hold a balance on this account |

With both adapters applied (§6, above) and the recipient able to hold `STA`, `scripts/execute_demo_transfer_payment.py` hand-built the same kind of custom `smart_account` authorization used in §6.3, extended to cover this call's real invocation tree:

```
$ python3 scripts/execute_demo_transfer_payment.py \
    --smart-account CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS \
    --asset CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L \
    --destination GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU \
    --amount 5000000 \
    --nonce 1 \
    --expected-policy-version 1
submitted: f712d5609ca52226746ad9b6776240b763d597246808df1c2a844bf1905d8131 SendTransactionStatus.PENDING
status: GetTransactionStatus.SUCCESS
execute_transfer_payment succeeded. tx = f712d5609ca52226746ad9b6776240b763d597246808df1c2a844bf1905d8131
```

[Explorer](https://stellar.expert/explorer/testnet/tx/f712d5609ca52226746ad9b6776240b763d597246808df1c2a844bf1905d8131). Events emitted, in order: `policy_engine`'s `PolicyValidated (pol_ok)`, the SAC's own `transfer` event, `transfer_adapter`'s `TransferExecuted (xfer)`, and `smart_account`'s `TransferPaid (pay_ok)` — the full call chain, not a shortcut.

One real mechanical finding from building this: `smart_account.execute_transfer_payment`'s own `require_auth()` is not the only place `smart_account` needs to authorize in this call graph. `transfer_adapter::execute_transfer` independently calls `smart_account.require_auth()` again — satisfied automatically by Soroban's invoker-contract shortcut, since `smart_account` is `transfer_adapter`'s *direct* caller, so it needs no declared tree node at all. But the Stellar Asset Contract's own `transfer(from, to, amount)` *also* calls `from.require_auth()` internally, and `transfer_adapter` (not `smart_account`) is the SAC's direct caller — so that one does *not* get the invoker shortcut, and had to be declared as a direct child of the root invocation in the `AuthPayload`'s authorized tree (confirmed empirically: nesting it under an intermediate `transfer_adapter` node, mirroring the naive call graph, fails with `Error(Auth, InvalidAction)`; declaring it as a sibling of the root's direct children succeeds). `scripts/execute_demo_transfer_payment.py`'s comments document this exactly.

Verified state change directly, not just a successful status code:

| | Before | After |
|---|---|---|
| Recipient `STA` balance | `0` (no trustline) | `5000000` |
| Treasury `STA` balance | `1000000000` | `995000000` |
| `smart_account.is_nonce_used(1)` | `false` | `true` |

This closes the remaining gap from earlier in this document: an actual signer-authorized payment, not only the permissionless policy check, now has a live, real, reproducible testnet transaction behind it.

### 6.5 Nonce replay and policy-version pinning, rejected live

§6.4's real payment consumed `nonce = 1` under `expected_policy_version = 1`. Two follow-up calls, otherwise identical and fully signer-authorized (same real `AuthPayload` construction, same registered signer — the rejection is *not* an auth failure), were built and sent to `simulateTransaction` against the live deployment to prove the two guarantees Tranche 1 only had local-test coverage for actually hold on a public ledger:

| Call | Result |
|---|---|
| Same call, `nonce = 1` again (already consumed by §6.4) | ❌ Rejected: `Error(Contract, #8005)` (`NonceAlreadyUsed`) |
| A fresh `nonce = 555`, but `expected_policy_version = 99` (stale/wrong — actual version is `1`) | ❌ Rejected: `Error(Contract, #2006)` (`VersionMismatch`) |

Both were caught at the `simulateTransaction` stage — the same real signer authorization used for §6.4 was supplied, so the host actually executed `smart_account`'s full `execute_transfer_payment` logic (`__check_auth` succeeds, then `consume_nonce`/`policy_engine.validate_policy` reject) rather than failing earlier for an unrelated reason. As with §6.1's checks, a rejected simulation has no transaction hash to link — nothing is submitted to the network for a call that fails before it would mutate state.

This is the concrete evidence Tranche 2 Deliverable 1 asks for specifically: policy-version pinning and nonce-based replay protection enforced against live testnet state, on the exact call path a real wallet-approved payment uses — not simulated in isolation, and not merely asserted from the local test suite.

## 7. Known limitation: signer-gated interactive entrypoints

Every entrypoint on `smart_account` that spends treasury funds — `execute_transfer_payment`, `execute_split_payment`, `create_scheduled_payment`, `cancel_scheduled_payment`, and the composed `ExecutionEntryPoint::execute` — calls `env.current_contract_address().require_auth()`. Because `smart_account` is a Soroban **custom account** (`CustomAccountInterface::__check_auth` delegating to `stellar_accounts::smart_account::do_check_auth`), satisfying that `require_auth()` requires a correctly-constructed `AuthPayload` (a `Map<Signer, Bytes>` of signer proofs plus the matched `context_rule_ids`) — not a plain Ed25519 transaction signature. Building that payload off-chain (matching the registered signer, whether an Ed25519 wallet key or a passkey) is exactly the job of a wallet/dApp/SDK client — the layer `docs/V1_SCOPE.md` explicitly lists under "Not Yet Included in V1." The `stellar` CLI has no built-in support for constructing third-party custom-account authorization schemes, so it cannot drive these entrypoints on its own.

Two consequences for this deployment:

1. **Everything gated by plain `Address::require_auth()` on a regular account is wired and demonstrated above** — `initialize()` calls (owner or admin auth), `propose_adapter_change` (owner auth via `ownable::enforce_owner_auth`), `add_guardian` (admin auth), and all `policy_engine` configuration. None of these need the custom scheme, so the CLI signs them automatically. `apply_adapter_change` is permissionless by design (see §5) once its delay elapses, so it doesn't need any authorization scheme at all.
2. **`intent_registry`'s one-time bootstrap (§6.3) and one real `execute_transfer_payment` call (§6.4) are both solved directly** — hand-built, one-off custom-account authorizations, not the bare CLI. What remains genuinely out of reach of the bare CLI is a *general-purpose, reusable* signing capability: an arbitrary user picking an arbitrary destination/amount/schedule through a wallet UI, with the dApp constructing the right `AuthPayload` on demand for whatever call that turns out to be. §6.3's and §6.4's scripts are each intentionally narrow — one hardcoded call shape apiece, not a library — see `docs/DAPP_INTEGRATION_SPEC.md` for what a general client actually needs to do.

Beyond the one concretely executed transfer in §6.4, the interactive signer-gated flows are proven correct across every contract and edge case by the 120-test local integration suite (`cargo test --workspace`), which exercises `execute_transfer_payment`, `execute_split_payment`, `create_scheduled_payment`/`cancel_scheduled_payment`, and `execute_scheduled_payment` end-to-end against real instances of every contract using `mock_all_auths()`, plus `webauthn_verifier`'s dedicated real-cryptography test suite (real secp256r1 and Ed25519 signatures) for the signature-verification layer itself.

## 8. Reproduction

```bash
./scripts/deploy_testnet.sh
```

Or step by step, see `scripts/deploy_testnet.sh` directly — every command in §4–§5 above is drawn verbatim from that script (except the token-address lookup fix below, done manually for this specific run before being folded back into the script).

This run surfaced one real script bug, now fixed: since Stellar Asset Contract addresses are deterministic (derived from issuer + asset code), re-running the test-asset deploy step against a deployer that already has that asset deployed used to hard-fail with `Error(Storage, ExistingValue)` instead of reusing the existing contract. The script now catches that specific error and looks the existing address up via `stellar contract id asset` instead of failing the whole run.

## 9. Prior PoC Deployment

The earlier partial PoC (`smart_account_poc`, `policy_registry_poc`, `intent_registry_poc`, `recovery_guard_poc` on `soroban-sdk 22.0.1`) was deployed separately and predates everything above. That record has been moved to `docs/archive/POC_TESTNET_DEPLOYMENT.md` to keep this document focused on the current V1 deployment — see `docs/archive/README.md` for why it's archived rather than deleted.
