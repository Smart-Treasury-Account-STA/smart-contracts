# dApp Developer Guide: Smart Treasury Account on Mainnet

This is the mainnet counterpart to `docs/TESTNET_FACTORY_DEPLOYMENT.md` —
written for whoever is integrating the dApp against the real, live mainnet
deployment. For the full authorization/payment-flow mechanics (the
`AuthPayload`/"Entry A/Entry B" construction, event reference, error
reference), see `docs/DAPP_INTEGRATION_SPEC.md` — everything there applies
unchanged to mainnet; only the addresses and network config below differ.

## 1. Network config

| Setting | Value |
|---|---|
| Network passphrase | `Public Global Stellar Network ; September 2015` |
| RPC | No free SDF-hosted mainnet RPC exists. This deployment used `https://soroban-rpc.mainnet.stellar.gateway.fm`, confirmed working, but pick your own provider for production (QuickNode, Ankr, Tatum, Blockdaemon, etc. — see https://developers.stellar.org/docs/data/apis/rpc/providers). Two independent providers were cross-checked during this deployment and returned identical results. |
| Horizon | `https://horizon.stellar.org` (SDF-hosted, free, standard) |

The SDK's `sdk/packages/core/src/config.ts` already has `buildMainnetConfig(contracts, rpcUrl)` ready for exactly this — call it with the addresses below once you've picked an RPC provider.

## 2. Factory contract (for deploying additional treasuries)

| Contract | Address |
|---|---|
| `account_factory` | `CCFIPN4TIF5XOJ7SZCQNET7YSXUXS7ERLJHX3JHNVZPTJTFI4HTKQAAV` |

Call `deploy_account(caller, salt, initial_signers, initial_policies, guardian_threshold, executor)` to deploy a brand new, fully wired treasury (6 contracts) in one transaction — see `docs/DAPP_INTEGRATION_SPEC.md` §12 for the full walkthrough, and `docs/MAINNET_DEPLOYMENT.md` §5 for a worked real example. Creating a new treasury this way is cheap (well under 1 XLM) — the expensive part (uploading contract code) is already done once, for good, by this factory's own setup.

`webauthn_verifier` is **not** deployed on mainnet yet — only needed if you plan to register a passkey (`Signer::External`) signer. Ask before you need it; it's a single, cheap, stateless shared instance to add.

## 3. Example treasury (already deployed, funded, and exercised — safe to point a test build at)

| Contract | Address |
|---|---|
| **`smart_account`** | `CDTE6DBMGFPTLBLPS7O32FMZGEKUM6KAW7GPACSI6RPY73DDFJIBVL7W` |
| `policy_engine` | `CAMM3LIYYW57HU2YF3FEQS5ARYN7ZILF4ETFHTBYHUBNU4SF6MHIOORM` |
| `intent_registry` | `CDCTQSJEGOVTD63M34VSUYKIX6KMZL53P646G4QCER75Q4V4EKWRL3F6` |
| `recovery_manager` | `CB6SOTBREWNAYNUYKIXTNTAYDKBOBIH4MUDRU36X326NVYQDBCMIKIBP` |
| `transfer_adapter` | `CAJYFE76XIRQVEUPI6O3JMH2LZCS3VXHYEF3VO3L5FID4ACNPBK4HVOC` |
| `split_adapter` | `CDZMKHLBRLEHES62KSQE6UJBHV4H2VHJ6K2D6I6K66NJBMROSRLPOLC7` |

**Owner / founding signer (context rule 0):** `GAHNF7XS5D2YLNK3PPW5DVEDJDKVEE5NJ6POQVMM36HODCNVALTSNIY7` — a single key for now (no multisig `governance_account` on this beta treasury). **Do not treat this as a production-secure key** — its recovery phrase has been exposed in an internal chat session and should be considered non-confidential; this treasury is for integration testing, not for holding real value long-term.

**Executor (relayer role for scheduled payments):** same address as owner, for now — reassignable any time via `intent_registry.set_executor`, rooted with `smart_account`'s own auth (see `docs/DAPP_INTEGRATION_SPEC.md` §12.7).

**Enabled today** (via `policy_engine`): `transfer` and `split` operations; XLM (cap 50/transfer) and USDC (cap 1/transfer); three allow-listed recipients (ask if you need a new one added — it's a single admin call, cheap and fast). A second context rule (`id 1`, scoped `CallContract(transfer_adapter)`) also exists on this treasury, for reference on how scoped rules look in practice — treat rule `0` as the one that matters for anything not specific to `transfer_adapter`.

## 4. Assets

| Asset | Contract (SAC) | Notes |
|---|---|---|
| XLM (native) | `CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA` | No trustline needed ever, for anyone |
| USDC | `CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75` | Issuer `GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` (Circle). `AUTH_REQUIRED` is **false** on this issuer — unlike the testnet `STA` asset you may have worked with, a classic recipient only needs to open a trustline (`change_trust`) once; there is no separate issuer-authorization step to worry about. |

If you're testing payouts to a fresh classic (`G...`) address, remember it needs: (1) to exist on-ledger at all (funded with a minimum XLM reserve), and (2) a USDC trustline if you're sending USDC — neither is automatic. Soroban contract addresses (`C...`, e.g. another treasury) need neither.

## 5. What's proven working on mainnet, right now, on the example treasury above

Every item below has a real, verified transaction — see
`docs/MAINNET_TESTING_TRANSACTIONS.md` for hashes and explorer links:

- Direct payments (`execute_transfer_payment`) in both XLM and USDC
- Split payments to multiple recipients in one call
- Scheduled payments: create, relayer-execute, and cancel
- `pause`/`unpause`
- Permissionless TTL maintenance
- Signer management (`add_signer`, `remove_signer`) — including a real,
  live demonstration of the "policy-less rule requires unanimous
  signers" behavior described in `docs/DAPP_DEVELOPER_QA.md`
- Context rule creation (`add_context_rule`)

**Not yet exercised on mainnet** (real ~24h timelocks make same-session
proof impractical, not a functional gap): guardian freeze/recovery, and
adapter reconfiguration (`propose_adapter_change`/`apply_adapter_change`).
Both were proven end-to-end on testnet
(`docs/TESTNET_FACTORY_DEPLOYMENT.md` §14) on the same contract code.

## 6. One cost note worth knowing before you plan anything that deploys new contract code

Uploading brand-new contract WASM to mainnet right now costs significantly
more than you'd expect from testnet experience — confirmed, independently
verified, real network-wide phenomenon (Soroban state rent, not specific
to these contracts). It does **not** affect you if you're just calling
`deploy_account` on the already-deployed factory above, or invoking any
already-deployed contract — those remain cheap. It only matters if you're
uploading genuinely new contract code. Ask if this is relevant to
something you're planning.
