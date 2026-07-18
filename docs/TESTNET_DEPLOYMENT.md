# Smart Treasury Account V1 Testnet Deployment

This is the live deployment record for the current V1 workspace (`docs/V1_SCOPE.md`), deployed 2026-07-18, `soroban-sdk 26.1.0`, with the OpenZeppelin Stellar contract composition. The earlier PoC deployment (`smart_account_poc`, `policy_registry_poc`, `intent_registry_poc`, `recovery_guard_poc` on `soroban-sdk 22.0.1`) that used to be recorded in this same file has been moved to [`docs/archive/POC_TESTNET_DEPLOYMENT.md`](archive/POC_TESTNET_DEPLOYMENT.md) — see [§9](#9-prior-poc-deployment).

Reproduce this deployment with `scripts/deploy_testnet.sh` (idempotent-ish: re-running `initialize` calls against already-initialized contracts will fail with `AlreadyInitialized`, which is expected).

## 1. Deployment Summary

| Field | Value |
|---|---|
| Network | Stellar Testnet |
| Network passphrase | `Test SDF Network ; September 2015` |
| Stellar CLI version | `stellar 26.0.0` |
| Deployment date | 2026-07-18 |
| Deployer identity | `sta-testnet-deployer` |
| Deployer public key | `GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB` |

The deployed contracts are the full V1 workspace: all 7 packages, composing real OpenZeppelin Stellar contracts (`stellar-accounts`, `stellar-access`, `stellar-contract-utils` `0.7.2`) for signer/passkey authentication, ownership, and pause state. `smart_account` is initialized with a real founding Ed25519 wallet signer (the deployer's own account, registered as `Signer::Delegated`), and `transfer_adapter`/`split_adapter` perform real Stellar Asset Contract transfers, not simulated ones.

## 2. Contract Addresses

| Contract | Testnet contract ID | Explorer |
|---|---|---|
| `webauthn_verifier` | `CBD3TLL3AWBKV4K4XGLHJKQJXLJENFISR27JCXI3RNS3P7SFTIN4BMQW` | [Explorer](https://stellar.expert/explorer/testnet/contract/CBD3TLL3AWBKV4K4XGLHJKQJXLJENFISR27JCXI3RNS3P7SFTIN4BMQW) |
| `policy_engine` | `CDE5ZGZI2BTA5YH22KDD5WNDJRXS3E4M5QGLGGZD4UYM5A4NHKF7GZP3` | [Explorer](https://stellar.expert/explorer/testnet/contract/CDE5ZGZI2BTA5YH22KDD5WNDJRXS3E4M5QGLGGZD4UYM5A4NHKF7GZP3) |
| `intent_registry` | `CBOH2FR7KRRFPZCZ3V5BF3VL7GGD7D6BQTDN42JH7UA4HGN4JPT2QQXQ` | [Explorer](https://stellar.expert/explorer/testnet/contract/CBOH2FR7KRRFPZCZ3V5BF3VL7GGD7D6BQTDN42JH7UA4HGN4JPT2QQXQ) |
| `recovery_manager` | `CDPU6PUAFUCGIOHAU3UIW2MU3WO5MUFU43UZBVX5DZPODCQCG6DIVBCQ` | [Explorer](https://stellar.expert/explorer/testnet/contract/CDPU6PUAFUCGIOHAU3UIW2MU3WO5MUFU43UZBVX5DZPODCQCG6DIVBCQ) |
| `smart_account` | `CCK7SMLYWIWVTRMIATD6W44QTEK7OMZCZNDSSJPPNQWK7X4IHB5X3W5G` | [Explorer](https://stellar.expert/explorer/testnet/contract/CCK7SMLYWIWVTRMIATD6W44QTEK7OMZCZNDSSJPPNQWK7X4IHB5X3W5G) |
| `transfer_adapter` | `CDTTFTQK5OPJ5BUYZJTWQPISYKTIK5R55MYIWDDCTTCTLPSHRDJO7OJY` | [Explorer](https://stellar.expert/explorer/testnet/contract/CDTTFTQK5OPJ5BUYZJTWQPISYKTIK5R55MYIWDDCTTCTLPSHRDJO7OJY) |
| `split_adapter` | `CB2ZBM2AMBSONEEBTSVONTHYYGX7WESQX6TJDHV6SFSUBP4J6NCZSF3Y` | [Explorer](https://stellar.expert/explorer/testnet/contract/CB2ZBM2AMBSONEEBTSVONTHYYGX7WESQX6TJDHV6SFSUBP4J6NCZSF3Y) |
| `STA` test asset (SAC) | `CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L` | [Explorer](https://stellar.expert/explorer/testnet/contract/CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L) |

Auxiliary testnet identities used only as `Address` values (not funded, not signing anything): guardian `GDXVIRLSBDKT7EZM2RM3FH26W3TPF77IJ7GZBA5IOA6ZJBTW26NNO3AV`, recipient `GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU`.

## 3. WASM Artifacts

Built with `stellar contract build --optimize --out-dir wasm`:

| Contract | WASM hash |
|---|---|
| `webauthn_verifier` | `2a56f2fbdaa56f6b5f138ba05b868116ae105b32b31b7ed4ee122ce9902e67f8` |
| `policy_engine` | `291ae10e032c7219ed48496eb71deb8c6f63ceb4d679f2df68f45a2fcefd4160` |
| `intent_registry` | `ec50b7d59e3af2a69d79d153c0f3d54f96685cfb1125fc8f30da5dc4685b1032` |
| `recovery_manager` | `e6dc1f87b44f411957db7eed5323d5514ca519470dca395cd689022b0c0142c7` |
| `smart_account` | `a3d7a29849c98e5d1a4d397c27d4806121a46f770c6a99e43454975ef0ce3926` |
| `transfer_adapter` | `922f52e51e6cbb973f6030692da76e036aac9ab929731b302a4311948fc5bcf6` |
| `split_adapter` | `63853a3c06519e701f98252c76c5d8d65b52237f92cf16c6f0631e3f1a84a78c` |

## 4. Deployment Transactions

Each deployment has two transactions: WASM upload, then contract instance creation.

| Contract | WASM upload | Contract create |
|---|---|---|
| `webauthn_verifier` | [`8078be84...`](https://stellar.expert/explorer/testnet/tx/8078be8424286250dc05ec900284943f154eb67ffd2cc7c05183e296d4b6c887) | [`56f3e34f...`](https://stellar.expert/explorer/testnet/tx/56f3e34f989adf14430c03f2c649a6ce0fb500d8aaa28881b40d504e4d237db1) |
| `policy_engine` | [`39a61749...`](https://stellar.expert/explorer/testnet/tx/39a61749945a7bdad9f4e767290e12d96a04f3128ce16d7b7a8ca43131d7a62f) | [`63b35ccb...`](https://stellar.expert/explorer/testnet/tx/63b35ccbc656da932b002033151507b472fe223a93f2760fc59cf89c8f72c1e2) |
| `intent_registry` | [`7c2acbc6...`](https://stellar.expert/explorer/testnet/tx/7c2acbc65409056362efbbadd1c7f59d8c7bfc91259ce815cd4d81dcb068f7fb) | [`01292ae0...`](https://stellar.expert/explorer/testnet/tx/01292ae00b649b1499a48ea3d1f78aabd840023094fa51af845db2f3fe2950d5) |
| `recovery_manager` | [`f23406fd...`](https://stellar.expert/explorer/testnet/tx/f23406fdab508b397204399962372c9843865a76fa2342ccaf849e69d1a73f33) | [`badd941e...`](https://stellar.expert/explorer/testnet/tx/badd941e4d240f73ecb2b991195170e6d03b7579f4e0a8b8f6e553e912f6cce1) |
| `smart_account` | [`2ae4cea9...`](https://stellar.expert/explorer/testnet/tx/2ae4cea97473a2e4eed3ac79e4c7d3479a5ef685431602f970d39542a55aa70e) | [`e55adda4...`](https://stellar.expert/explorer/testnet/tx/e55adda4a8ac77fdf56f320d7fd9a695facb4b0eea2c2fd2fff763b557e933e2) |
| `transfer_adapter` | [`4515e27b...`](https://stellar.expert/explorer/testnet/tx/4515e27bcf23647e16a76b5b831772cd7ecaf4aa71a116d2e2a7ba035d24f31f) | [`923f05c9...`](https://stellar.expert/explorer/testnet/tx/923f05c9ad9e6604684a1fe837bb389eb079fdc577e8fcd9a2ef2733a6a18dd0) |
| `split_adapter` | [`327e35d5...`](https://stellar.expert/explorer/testnet/tx/327e35d5626d9df0d07139b3ab472665cfd57b24050847811bdc4717af0e2521) | [`25a74f60...`](https://stellar.expert/explorer/testnet/tx/25a74f60be8aad879136d5cf251896e54e22bf111869ec44909cc3b1edb961b9) |

## 5. Initialization and Wiring Transactions

| Step | Transaction | Result |
|---|---|---|
| `policy_engine.initialize` | [`029850...`](https://stellar.expert/explorer/testnet/tx/029850467cf70e52183404691ffcad6f30be88770f23be3330b2b72e0db7d341) | Admin set to deployer, policy version initialized to `1` |
| `recovery_manager.initialize` | [`e2c443...`](https://stellar.expert/explorer/testnet/tx/e2c44372e5adfa451c313899b408faabea8f0c45d253b6031d3d2dc6d0766172) | Admin set to deployer, guardian threshold set to `1` |
| `smart_account.initialize` | [`4da678...`](https://stellar.expert/explorer/testnet/tx/4da678921489c89a6b5fd6a8588867bc5402c1fb9dae9ffa802b01b4320ac324) | Owner set to deployer; founding signer registered as `Signer::Delegated(deployer)` under context rule `0`; `policy_engine`/`intent_registry`/`recovery_manager` addresses pinned |
| `smart_account.set_adapter(transfer)` | [`b3de63...`](https://stellar.expert/explorer/testnet/tx/b3de63dc06623e8629b64e40f9cf9e510162babfa37b09c7b97304d1e5ba36e8) | `transfer_adapter` wired for the `transfer` operation |
| `smart_account.set_adapter(split)` | [`727efa...`](https://stellar.expert/explorer/testnet/tx/727efabb257340a3a23a70f6d2c80b811f20a89a3b75771e189e982d82737a75) | `split_adapter` wired for the `split` operation |
| `recovery_manager.add_guardian` | [`d6cf5c...`](https://stellar.expert/explorer/testnet/tx/d6cf5cc7aa3c9d22ff87f197dd33779353ed46d0d7b420f8161cf4ef61785e7b) | Guardian registered, active after the standard activation delay |
| `policy_engine.set_asset_rule` | [`2f4a38...`](https://stellar.expert/explorer/testnet/tx/2f4a38d7bfdf2c5cdf585f6d79eb8a166265e22f035dea3cdbb5ea29d5f1de80) | `STA` test asset enabled, max single transfer `10,000,000` |
| `policy_engine.set_recipient_allowed` | [`5627d2...`](https://stellar.expert/explorer/testnet/tx/5627d247f751c5c6b5f5bed020284a8dace0ef69f957e7eb5ea25715bfa01d0b) | Test recipient allowed |
| `policy_engine.set_operation_allowed(transfer)` | [`50b243...`](https://stellar.expert/explorer/testnet/tx/50b243258fcf2c1ed885b53bb8f8ab93963812753126f5f8e578f1b06142e47b) | `transfer` operation enabled |
| `policy_engine.set_operation_allowed(split)` | [`3a606d...`](https://stellar.expert/explorer/testnet/tx/3a606d1d39861f477f02f803ac75b4d98a6cb05f571daf4eaa991fa112b19d30) | `split` operation enabled |
| Mint `STA` to `smart_account` | [`8d0a9c...`](https://stellar.expert/explorer/testnet/tx/8d0a9c996bf00c687a682f4413c278b853949b6c882a708ba17a949596220fb6) | `1,000,000,000` units minted to the treasury |

`intent_registry` is deployed but **deliberately left uninitialized** — see [§7](#7-known-limitation-intent_registry-and-signer-gated-entrypoints).

## 6. Demonstrated Testnet Flows

### 6.1 Valid and invalid `policy_engine.validate_policy` calls, live on-chain

`validate_policy` is permissionless by design (any caller may check whether a hypothetical payment would pass — the actual authorization happens at `smart_account`), which makes it directly demonstrable via CLI without any signer setup:

All three calls were made read-only (`--send=no`, RPC simulation against live testnet state — no transaction hash, since nothing is written for a check that doesn't mutate storage):

| Check | Result |
|---|---|
| `transfer` of `5,000,000` of `STA` to the allowed recipient, version `1` | ✅ Success — `pol_ok` event emitted |
| Same, but destination = deployer (never allowlisted as a recipient) | ❌ Rejected: `Error(Contract, #2004)` (`RecipientNotAllowed`) |
| Same, but amount `20,000,000` (above the `10,000,000` cap) | ❌ Rejected: `Error(Contract, #2005)` (`AmountAboveLimit`) |

This concretely proves, on live testnet rather than only in the local test suite, that policy checks fail closed for both an unapproved recipient and an over-cap amount.

### 6.2 `smart_account.status()`

```json
{"frozen":false,"initialized":true,"paused":false,"policy_version_hint":0}
```

## 7. Known limitation: `intent_registry` and signer-gated entrypoints

Every entrypoint on `smart_account` that spends treasury funds or creates scheduled automation — `execute_transfer_payment`, `execute_split_payment`, `create_scheduled_payment`, `cancel_scheduled_payment`, and the composed `ExecutionEntryPoint::execute` — calls `env.current_contract_address().require_auth()`. Because `smart_account` is a Soroban **custom account** (`CustomAccountInterface::__check_auth` delegating to `stellar_accounts::smart_account::do_check_auth`), satisfying that `require_auth()` requires a correctly-constructed `AuthPayload` (a `Map<Signer, Bytes>` of signer proofs plus the matched `context_rule_ids`) — not a plain Ed25519 transaction signature. Building that payload off-chain (matching the registered signer, whether an Ed25519 wallet key or a passkey) is exactly the job of a wallet/dApp/SDK client — the layer `docs/V1_SCOPE.md` explicitly lists under "Not Yet Included in V1." The `stellar` CLI has no built-in support for constructing third-party custom-account authorization schemes, so it cannot drive these entrypoints on its own.

Two consequences for this deployment:

1. **Everything gated by plain `Address::require_auth()` on a regular account is wired and demonstrated above** — `initialize()` calls (owner or admin auth), `set_adapter` (owner auth via `ownable::enforce_owner_auth`), `add_guardian` (admin auth), and all `policy_engine` configuration. None of these need the custom scheme, so the CLI signs them automatically.
2. **`intent_registry` is deployed but not initialized.** Per the tested design (see `contracts/smart_account/src/test.rs`), `intent_registry`'s `admin` must be `smart_account` itself, so that `create_scheduled_payment`/`cancel_scheduled_payment` satisfy `ensure_admin` via Soroban's direct-caller exemption (a contract's own sub-invocation of another contract implicitly authorizes calls made *as* itself, with no separate signature). Setting that up requires calling `smart_account.execute(intent_registry, "initialize", [smart_account])`, which itself requires the treasury's own signer authorization — the same custom-account signing problem. Rather than initialize `intent_registry` with a different admin that would silently diverge from the tested design, it is left uninitialized and named here explicitly.

The signer-gated flows themselves are proven correct — just not against this specific live deployment — by the 105-test local integration suite (`cargo test --workspace`), which exercises `execute_transfer_payment`, `execute_split_payment`, `create_scheduled_payment`/`cancel_scheduled_payment`, and `execute_scheduled_payment` end-to-end against real instances of every contract using `mock_all_auths()`, plus `webauthn_verifier`'s dedicated real-cryptography test suite (real secp256r1 and Ed25519 signatures) for the signature-verification layer itself.

## 8. Reproduction

```bash
./scripts/deploy_testnet.sh
```

Or step by step, see `scripts/deploy_testnet.sh` directly — every command in §4–§5 above is drawn verbatim from that script.

## 9. Prior PoC Deployment

The earlier partial PoC (`smart_account_poc`, `policy_registry_poc`, `intent_registry_poc`, `recovery_guard_poc` on `soroban-sdk 22.0.1`) was deployed separately and predates everything above. That record has been moved to `docs/archive/POC_TESTNET_DEPLOYMENT.md` to keep this document focused on the current V1 deployment — see `docs/archive/README.md` for why it's archived rather than deleted.
