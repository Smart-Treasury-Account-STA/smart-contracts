# Smart Treasury Account PoC Testnet Deployment

## 1. Deployment Summary

This document records the Smart Treasury Account PoC deployment on Stellar testnet.

| Field | Value |
|---|---|
| Network | Stellar Testnet |
| Network passphrase | `Test SDF Network ; September 2015` |
| Network ID | `cee0302d59844d32bdca915c8203dd44b33fbb7edc19051ea37abedf28ecd472` |
| Protocol version observed | `26` |
| Stellar CLI version | `stellar 26.0.0` |
| Deployment date | 2026-06-12 |
| Deployer public key | `GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB` |

The deployed contracts are a focused PoC. They demonstrate selected onchain patterns from the Smart Treasury Account architecture: signer policy, policy validation, scheduled intent replay protection, and recovery controls. They do not execute production SAC transfers and do not represent the complete production contract suite.

## 2. Contract Addresses

| Contract | Testnet contract ID | Explorer | Stellar Lab |
|---|---|---|---|
| SmartAccount PoC | `CBYU6KK6IJTVM2632FN6ZRUIOHBIRRLIFQLXPBR7N5NLUAF3RUDFFWQO` | [Explorer](https://stellar.expert/explorer/testnet/contract/CBYU6KK6IJTVM2632FN6ZRUIOHBIRRLIFQLXPBR7N5NLUAF3RUDFFWQO) | [Lab](https://lab.stellar.org/r/testnet/contract/CBYU6KK6IJTVM2632FN6ZRUIOHBIRRLIFQLXPBR7N5NLUAF3RUDFFWQO) |
| PolicyRegistry PoC | `CCYR3AK5C7WUVJNZXVOWZHMIGYLO37ODKLTABRE2D7FM2XFODWRG2KEV` | [Explorer](https://stellar.expert/explorer/testnet/contract/CCYR3AK5C7WUVJNZXVOWZHMIGYLO37ODKLTABRE2D7FM2XFODWRG2KEV) | [Lab](https://lab.stellar.org/r/testnet/contract/CCYR3AK5C7WUVJNZXVOWZHMIGYLO37ODKLTABRE2D7FM2XFODWRG2KEV) |
| IntentRegistry PoC | `CDKCIFSZQZZKEAL6YKSOUWIXR4GMW6PBQVSSWNNM7WQFUBIT57447Z5I` | [Explorer](https://stellar.expert/explorer/testnet/contract/CDKCIFSZQZZKEAL6YKSOUWIXR4GMW6PBQVSSWNNM7WQFUBIT57447Z5I) | [Lab](https://lab.stellar.org/r/testnet/contract/CDKCIFSZQZZKEAL6YKSOUWIXR4GMW6PBQVSSWNNM7WQFUBIT57447Z5I) |
| RecoveryGuard PoC | `CCCQRDPAIFCMTE5TIPD6SU7FSOKVWVPD5RIJ5B3RZC2MXXKJTZZSQYH2` | [Explorer](https://stellar.expert/explorer/testnet/contract/CCCQRDPAIFCMTE5TIPD6SU7FSOKVWVPD5RIJ5B3RZC2MXXKJTZZSQYH2) | [Lab](https://lab.stellar.org/r/testnet/contract/CCCQRDPAIFCMTE5TIPD6SU7FSOKVWVPD5RIJ5B3RZC2MXXKJTZZSQYH2) |

## 3. WASM Artifacts

The contracts were built with:

```bash
stellar contract build --optimize --out-dir wasm
```

| Contract | WASM artifact | WASM hash |
|---|---|---|
| SmartAccount PoC | `wasm/sta_smart_account_poc.wasm` | `5921b9aed5bbcf3034d32d73f0dd97c47ed85ec975b3543716c35d96eef72a60` |
| PolicyRegistry PoC | `wasm/sta_policy_registry_poc.wasm` | `9cf169bdef4a4bc054f86fb3e6e00fa26cf607430a219c1991ac6658d893c667` |
| IntentRegistry PoC | `wasm/sta_intent_registry_poc.wasm` | `6e0abff31fbd308dc753cbe1c773f06e9100a8e055a2ab26a4faaacd4275d907` |
| RecoveryGuard PoC | `wasm/sta_recovery_guard_poc.wasm` | `c08f31e2eb28e2e3310fbb46aef024af8bc1839ed4a379e63a4b0e516d35d509` |

## 4. Deployment Transactions

Each deployment has two transactions: one transaction uploads the WASM and one transaction creates the contract instance.

| Contract | WASM upload transaction | Contract deploy transaction |
|---|---|---|
| SmartAccount PoC | [`e6dc5fa1867f650d239af2d1d0e4c59e25e3e97e8a2212aab7fb7ae310304f8d`](https://stellar.expert/explorer/testnet/tx/e6dc5fa1867f650d239af2d1d0e4c59e25e3e97e8a2212aab7fb7ae310304f8d) | [`bd7b7328c0cb71d092b4bd730da40e6d4acad8e3459fb1b2b0c75eb1c66aee2d`](https://stellar.expert/explorer/testnet/tx/bd7b7328c0cb71d092b4bd730da40e6d4acad8e3459fb1b2b0c75eb1c66aee2d) |
| PolicyRegistry PoC | [`6e039203fef729f9581d2b987d7bcd0ad3558635fefbef30c35a772379419088`](https://stellar.expert/explorer/testnet/tx/6e039203fef729f9581d2b987d7bcd0ad3558635fefbef30c35a772379419088) | [`cf0c863739fb12b33257ee90819ba7cc89f12cfefe90c6634c73e2060850ce4e`](https://stellar.expert/explorer/testnet/tx/cf0c863739fb12b33257ee90819ba7cc89f12cfefe90c6634c73e2060850ce4e) |
| IntentRegistry PoC | [`9b6727d34e551199a1f307a3c766ba990022180e84fd9905e6e586ec2f1d8d5f`](https://stellar.expert/explorer/testnet/tx/9b6727d34e551199a1f307a3c766ba990022180e84fd9905e6e586ec2f1d8d5f) | [`02b9434418b1ae3ceeb234d9fbf54c05380dbe6213ccfa1649b3bb5d32766cde`](https://stellar.expert/explorer/testnet/tx/02b9434418b1ae3ceeb234d9fbf54c05380dbe6213ccfa1649b3bb5d32766cde) |
| RecoveryGuard PoC | [`e7beb915310ba010ba6872cdae299e4ff693605f6ec2e22c9d3629e126fc4ec2`](https://stellar.expert/explorer/testnet/tx/e7beb915310ba010ba6872cdae299e4ff693605f6ec2e22c9d3629e126fc4ec2) | [`d65220d526bf0a38bcfb8e24487150dbe73e665c2dd5e61dda606d25f83aab0e`](https://stellar.expert/explorer/testnet/tx/d65220d526bf0a38bcfb8e24487150dbe73e665c2dd5e61dda606d25f83aab0e) |

## 5. Initialization Transactions

| Contract | Initialization transaction | Result |
|---|---|---|
| SmartAccount PoC | [`c61b79fadefbc700289cf5b0a461488f38a16319508aa57aaec1d24ce3f920a4`](https://stellar.expert/explorer/testnet/tx/c61b79fadefbc700289cf5b0a461488f38a16319508aa57aaec1d24ce3f920a4) | Admin set, payment threshold set to `1` |
| PolicyRegistry PoC | [`0e864879b467b33e70eef3b9f00f38e0c9e6d463b4ae5f67249a4133030b1b07`](https://stellar.expert/explorer/testnet/tx/0e864879b467b33e70eef3b9f00f38e0c9e6d463b4ae5f67249a4133030b1b07) | Admin set, policy version initialized to `1` |
| IntentRegistry PoC | [`ce99d5b6724892ec2082d138d55f64326c5e20f5dd9458add10aa2fa065c9e33`](https://stellar.expert/explorer/testnet/tx/ce99d5b6724892ec2082d138d55f64326c5e20f5dd9458add10aa2fa065c9e33) | Admin set, executor initialized to admin |
| RecoveryGuard PoC | [`1384191b74cce1486a178721f02bc32b2db8f6e60a666a22b3a23b61983f0427`](https://stellar.expert/explorer/testnet/tx/1384191b74cce1486a178721f02bc32b2db8f6e60a666a22b3a23b61983f0427) | Admin set, guardian threshold set to `1` |

## 6. Demonstrated Testnet Flows

### 6.1 SmartAccount Payment Validation

This flow demonstrates signer registration, treasury policy setup, nonce replay protection, and payment intent validation.

| Step | Transaction | Explanation |
|---|---|---|
| Add signer | [`13025491b0b144500e4b96de4f558d1becd7fb61b2b779ab557f76c39cbf41dc`](https://stellar.expert/explorer/testnet/tx/13025491b0b144500e4b96de4f558d1becd7fb61b2b779ab557f76c39cbf41dc) | Adds signer ID `1111...1111` with payment role and weight `1` |
| Set asset policy | [`d48e59d99379d1bacdd2182a97bdbfe0ac727399aa942edec4cd944aba5b8d1e`](https://stellar.expert/explorer/testnet/tx/d48e59d99379d1bacdd2182a97bdbfe0ac727399aa942edec4cd944aba5b8d1e) | Enables the demo asset address with max transfer `100` |
| Allow recipient | [`1c2589ef771f3b4c9472e15bf1446abb7e82fa11cd42079b4944786bbd54a117`](https://stellar.expert/explorer/testnet/tx/1c2589ef771f3b4c9472e15bf1446abb7e82fa11cd42079b4944786bbd54a117) | Allows the demo recipient address |
| Validate payment | [`c07fb9b78c766bc1cb27c2dd5bbf5eaa30635261fafaed3a5885350337a12aab`](https://stellar.expert/explorer/testnet/tx/c07fb9b78c766bc1cb27c2dd5bbf5eaa30635261fafaed3a5885350337a12aab) | Validates amount `25`, policy version `1`, signer threshold, and consumes nonce `1` |

Read-only status after the flow:

```json
{
  "frozen": false,
  "initialized": true,
  "paused": false,
  "payment_threshold": 1,
  "payment_weight": 1,
  "policy_version": 1
}
```

### 6.2 PolicyRegistry Validation

This flow demonstrates standalone policy validation. The demo uses the deployer public key as an address placeholder for both asset and destination. In the production architecture, this path is replaced with SAC asset addresses and real treasury recipients.

| Step | Transaction | Explanation |
|---|---|---|
| Set asset rule | [`79c5e50ec5af9ed708a362df634c334341eaee7f6d846ab0c59026f1ece67ce3`](https://stellar.expert/explorer/testnet/tx/79c5e50ec5af9ed708a362df634c334341eaee7f6d846ab0c59026f1ece67ce3) | Enables the demo asset with max transfer `100` |
| Allow recipient | [`bbd8deb967a867ddd1fcd6dd89771c784dd69d70a380d7c6271117baf50bd018`](https://stellar.expert/explorer/testnet/tx/bbd8deb967a867ddd1fcd6dd89771c784dd69d70a380d7c6271117baf50bd018) | Allows the demo recipient |
| Validate policy | [`c89db60280f328f52a05a57473dd0c07b1bf4b39e89fadf58d5e29f8cc9f14c8`](https://stellar.expert/explorer/testnet/tx/c89db60280f328f52a05a57473dd0c07b1bf4b39e89fadf58d5e29f8cc9f14c8) | Validates amount `40`, allowed asset, allowed recipient, and policy version `1` |

### 6.3 IntentRegistry Scheduled Execution

This flow demonstrates scheduled intent creation and child execution replay protection.

| Step | Transaction | Explanation |
|---|---|---|
| Create scheduled intent | [`7282348ad731ef68a0bb39184d017498df0ccde0f8b422ff617d74010a1095e2`](https://stellar.expert/explorer/testnet/tx/7282348ad731ef68a0bb39184d017498df0ccde0f8b422ff617d74010a1095e2) | Creates intent ID `2222...2222` with execution window `3054500` to `3056000` |
| Mark child execution | [`9cbcfb719480342c4289752ca927b9fa65c1db564a01d166f5480519f73ac95b`](https://stellar.expert/explorer/testnet/tx/9cbcfb719480342c4289752ca927b9fa65c1db564a01d166f5480519f73ac95b) | Marks child sequence `1` as executed using the authorized executor |

The create call intentionally passed `cancelled: true` in the input. The contract stores new intents with `cancelled: false`, proving that caller-provided creation state is sanitized.

Read-only intent state after creation:

```json
{
  "amount": "50",
  "asset": "GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB",
  "cancelled": false,
  "destination": "GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB",
  "end_ledger": 3056000,
  "intent_id": "2222222222222222222222222222222222222222222222222222222222222222",
  "start_ledger": 3054500
}
```

Read-only child execution status:

```json
true
```

### 6.4 RecoveryGuard Approval and Finalization

This flow demonstrates authenticated guardian approval and ledger-based recovery finalization.

| Step | Transaction | Explanation |
|---|---|---|
| Add guardian | [`5f6019d4b77d0605941e46f3648a594526f9185540dee0025ebb60d68bdfdb27`](https://stellar.expert/explorer/testnet/tx/5f6019d4b77d0605941e46f3648a594526f9185540dee0025ebb60d68bdfdb27) | Registers the deployer address as guardian |
| Open recovery | [`7e0e6d89bedd592bbdb24630cabb94c71dae065124214036971cf062c9c8548d`](https://stellar.expert/explorer/testnet/tx/7e0e6d89bedd592bbdb24630cabb94c71dae065124214036971cf062c9c8548d) | Opens request ID `4444...4444` with replacement signer `3333...3333` |
| Approve recovery | [`c8aa0d4c8a3e978357b42913de3c73ee3c2170a843dcd65cb8e9bab88e7dad77`](https://stellar.expert/explorer/testnet/tx/c8aa0d4c8a3e978357b42913de3c73ee3c2170a843dcd65cb8e9bab88e7dad77) | Guardian signs and approves the recovery request |
| Finalize recovery | [`236734f44dbaf4568723f697845ee5b738c9e83c3829e4ec24aa376f1661052e`](https://stellar.expert/explorer/testnet/tx/236734f44dbaf4568723f697845ee5b738c9e83c3829e4ec24aa376f1661052e) | Finalizes after threshold and ledger delay checks and returns the replacement signer |

Read-only recovery state after finalization:

```json
{
  "approvals": 1,
  "cancelled": false,
  "earliest_ledger": 3054500,
  "finalized": true,
  "replacement_signer": "3333333333333333333333333333333333333333333333333333333333333333",
  "request_id": "4444444444444444444444444444444444444444444444444444444444444444"
}
```

## 7. Reproduction Commands

Use the same network and source identity:

```bash
stellar network health --network testnet
stellar keys public-key sta-testnet-deployer
```

Build artifacts:

```bash
stellar contract build --optimize --out-dir wasm
```

Example read-only checks:

```bash
stellar contract invoke \
  --id CBYU6KK6IJTVM2632FN6ZRUIOHBIRRLIFQLXPBR7N5NLUAF3RUDFFWQO \
  --source-account sta-testnet-deployer \
  --network testnet \
  --send no \
  -- status
```

```bash
stellar contract invoke \
  --id CDKCIFSZQZZKEAL6YKSOUWIXR4GMW6PBQVSSWNNM7WQFUBIT57447Z5I \
  --source-account sta-testnet-deployer \
  --network testnet \
  --send no \
  -- is_child_executed \
  --intent_id 2222222222222222222222222222222222222222222222222222222222222222 \
  --child_sequence 1
```

## 8. Notes and Limitations

- This is a testnet PoC deployment, not a mainnet deployment.
- Stellar testnet state may be reset by network operators.
- Contract instance and storage TTLs should be extended if the deployment must remain available for a long review period.
- The PoC validates policy and execution state but does not perform real SAC transfers.
- The production architecture adds full SmartAccount `__check_auth`, SAC execution adapters, dApp, SDK, relayer, deployment scripts, monitoring, and broader integration tests.
