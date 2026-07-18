# Smart Treasury Account — PoC Testnet Deployment (Archived)

> **Status: superseded, archived.** This describes the earlier partial PoC revision — `smart_account_poc`, `policy_registry_poc`, `intent_registry_poc`, `recovery_guard_poc` on `soroban-sdk 22.0.1`. None of the WASM hashes, contract IDs, or transaction links below correspond to the current V1 source in `contracts/`. This file is retained only as a historical record of the PoC deployment process; it is not part of the current implementation and should not be used when reviewing V1. See `../TESTNET_DEPLOYMENT.md` for the current, live V1 testnet deployment.

## 1. Deployment Summary

| Field | Value |
|---|---|
| Network | Stellar Testnet |
| Network passphrase | `Test SDF Network ; September 2015` |
| Network ID | `cee0302d59844d32bdca915c8203dd44b33fbb7edc19051ea37abedf28ecd472` |
| Protocol version observed | `26` |
| Stellar CLI version | `stellar 26.0.0` |
| Deployment date | 2026-06-12 |
| Deployer public key | `GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB` |

## 2. Contract Addresses

| Contract | Testnet contract ID |
|---|---|
| SmartAccount PoC | `CBYU6KK6IJTVM2632FN6ZRUIOHBIRRLIFQLXPBR7N5NLUAF3RUDFFWQO` |
| PolicyRegistry PoC | `CCYR3AK5C7WUVJNZXVOWZHMIGYLO37ODKLTABRE2D7FM2XFODWRG2KEV` |
| IntentRegistry PoC | `CDKCIFSZQZZKEAL6YKSOUWIXR4GMW6PBQVSSWNNM7WQFUBIT57447Z5I` |
| RecoveryGuard PoC | `CCCQRDPAIFCMTE5TIPD6SU7FSOKVWVPD5RIJ5B3RZC2MXXKJTZZSQYH2` |

## 3. WASM Artifacts

| Contract | WASM hash |
|---|---|
| SmartAccount PoC | `5921b9aed5bbcf3034d32d73f0dd97c47ed85ec975b3543716c35d96eef72a60` |
| PolicyRegistry PoC | `9cf169bdef4a4bc054f86fb3e6e00fa26cf607430a219c1991ac6658d893c667` |
| IntentRegistry PoC | `6e0abff31fbd308dc753cbe1c773f06e9100a8e055a2ab26a4faaacd4275d907` |
| RecoveryGuard PoC | `c08f31e2eb28e2e3310fbb46aef024af8bc1839ed4a379e63a4b0e516d35d509` |

Full transaction-level detail for the PoC (deployment, initialization, and the four demonstrated flows: SmartAccount payment validation, PolicyRegistry validation, IntentRegistry scheduled execution, RecoveryGuard approval/finalization) is not reproduced here; it remains available in this repository's git history prior to the V1 rewrite (the commit that renamed `*_poc` packages to their current V1 names).
