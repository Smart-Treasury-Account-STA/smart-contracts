# SDK and Testnet Release

This documents a deliverable whose original brief was written for mainnet
("Deploy the smart contracts on Stellar mainnet and release the initial
public TypeScript SDK aligned with the deployed contracts") and was
executed here **against Stellar testnet instead, deliberately** — no
mainnet deployment exists. Everything else — reproducible build,
network configuration, contract deployment, SDK helpers,
transaction-preparation helpers, event-parsing utilities, and examples —
was built and verified for real, live, on testnet.

## What was built, mapped to the original deliverable

- **Reproducible contract build.** `stellar contract build --optimize
  --out-dir wasm` (unchanged, already the project's standard — see
  `docs/V1_REVIEW_GUIDE.md`), plus `sdk/scripts/generate-bindings.sh`,
  which rebuilds WASM and regenerates every contract's TypeScript client
  from that fresh build in one step — so the SDK's generated clients can
  never silently drift from contract source, mirroring the WASM-fixture
  staleness check already in `.github/workflows/ci.yml`.

- **Network configuration.** `sdk/packages/core/src/config.ts` is
  network-keyed (`testnet` / `mainnet`), not testnet-only code with
  addresses hardcoded elsewhere. `testnet` is populated with the live,
  currently-deployed, fixed contract set (the account_factory-deployed
  treasury from `docs/TESTNET_FACTORY_DEPLOYMENT.md` §13.2, which runs
  the code with finding 29 fixed — not the older hand-deployed one from
  `docs/TESTNET_DEPLOYMENT.md`, which predates that fix). `mainnet` is
  explicitly `undefined` with a doc comment explaining why — no
  fabricated addresses.

- **Contract deployment.** Already live (`docs/TESTNET_FACTORY_DEPLOYMENT.md`,
  `docs/TESTNET_DEPLOYMENT.md`) — this deliverable additionally proves the
  self-service `account_factory` path end to end *through the SDK itself*
  (not just the `stellar` CLI), deploying a brand new treasury in one
  call: [`37f72eb0...`](https://stellar.expert/explorer/testnet/tx/37f72eb06e55f6070118c793c8db5f3168023d40653838c96b6ba573da6050a6)
  (`sdk/examples/05-deploy-treasury-via-factory.ts`).

- **SDK helpers.** `sdk/generated/{smart_account,policy_engine,
  intent_registry,recovery_manager,account_factory}` — typed,
  simulate/sign/submit-ready clients via the Stellar CLI's own
  `stellar contract bindings typescript` generator, curated (renamed into
  the `@sta/*-bindings` scope, wired into an npm workspace) rather than
  hand-written. `transfer_adapter`/`split_adapter` deliberately excluded
  — never called directly by a client (`docs/DAPP_INTEGRATION_SPEC.md` §1).

- **Transaction-preparation helpers.** `sdk/packages/core/src/auth.ts` +
  `payments.ts`: `smart_account`'s custom `AuthPayload` ("Entry A"/"Entry
  B") construction — the one piece the generated bindings and no version
  of `@stellar/stellar-sdk` provide out of the box — built directly from
  the contract's own spec, plus prepare/simulate/sign/submit helpers for
  transfer, split, and scheduled payments, and the scheduled-payment
  relayer's separate (non-custom-account) authorization path.

- **Event-parsing utilities.** `sdk/packages/core/src/events.ts` — the
  generated bindings expose function signatures and errors from a
  contract's spec, but not its `#[contractevent]` definitions, so these
  are hand-typed against each contract's actual event structs and merge
  `#[topic]`-tagged fields with the data map correctly (verified live —
  see below).

- **First-class state types.** `sdk/packages/core/src/state.ts` — typed
  reads for `AccountStatus`, policy version, nonce-used (replay state),
  `ContextRule`, and `RecoveryRequest` (recovery state), not raw XDR.

- **Examples.** `sdk/examples/01`–`06`, covering every documented flow:
  read state, an interactive transfer payment, an interactive split
  payment, the full scheduled-payment lifecycle (signer creates, relayer
  executes), self-service treasury deployment, and recovery-state reads.

## Correction found while building this

Building the relayer example (`04`) surfaced a real inaccuracy in
`docs/DAPP_INTEGRATION_SPEC.md` §8: it claimed `prepareTransaction`
auto-fills the relayer's `Executor` requirement as a `SOURCE_ACCOUNT`
credential. It does not, for either the bare `stellar` CLI or
`@stellar/stellar-sdk ^14.5.0` — both reject with `Error(Auth,
InvalidAction)` because that requirement is two levels deep, not at the
invocation root. Fixed in the spec doc itself, and in `auth.ts`'s
`buildExecutorAuthEntry` (the actual working implementation, exercised in
example `04`).

## Verification — every example actually run against live testnet

Not just written: each of these produced a real, independently-checkable
transaction while this SDK was being built.

| Example | What it proves | Transaction |
|---|---|---|
| `01-read-treasury-state.ts` | Reads match the live deployment exactly (status, owner, context rule, policy version) | read-only, no tx |
| `02-execute-transfer-payment.ts` | Entry A/B construction, `TransferPaid` event decoded with topic-tagged fields merged correctly | [`e6bbc723...`](https://stellar.expert/explorer/testnet/tx/e6bbc723f7a17267317cb172775f046d202c9e5a6fbafc1aac583ddadc0a4f16) |
| `03-execute-split-payment.ts` | Same, for a two-recipient split | [`edf4231d...`](https://stellar.expert/explorer/testnet/tx/edf4231d3244641b8dcd121804849ff9b1c11ba24ad91a629c2e88d484437ee7) |
| `04-...create-and-relayer-execute.ts` | Full schedule lifecycle: signer creates, relayer executes with the corrected auth entry | create [`04a6dfd7...`](https://stellar.expert/explorer/testnet/tx/04a6dfd746c823137e0bddb1b05c86cf693d60f515f1c09e7a4beeef02900717), execute [`7367d9b9...`](https://stellar.expert/explorer/testnet/tx/7367d9b9ec4b33ce6f3396670760640a9ee38273a3a043db33e23e6b2604512f) |
| `05-deploy-treasury-via-factory.ts` | Generated-client convenience path (`basicNodeSigner`/`signAndSend`), a fresh treasury deployed through the SDK | [`37f72eb0...`](https://stellar.expert/explorer/testnet/tx/37f72eb06e55f6070118c793c8db5f3168023d40653838c96b6ba573da6050a6) |
| `06-read-recovery-state.ts` | `RecoveryRequest` read back matches the completed recovery in `docs/TESTNET_FACTORY_DEPLOYMENT.md` §14.3 exactly | read-only, no tx |

`npm run build` across the full `sdk/` workspace (generated packages +
`sta-sdk`) is clean.

## How to measure completion (mapped from the original brief)

*"Mainnet contract addresses are published"* → testnet contract addresses
are published (`docs/TESTNET_FACTORY_DEPLOYMENT.md`,
`sdk/packages/core/src/config.ts`).

*"A developer can inspect the mainnet configuration and run SDK examples
for documented flows"* → a developer can inspect `sdk/packages/core/src/config.ts`
(testnet, honestly labeled) and run every example in `sdk/examples/` — see
`sdk/README.md` for exact commands.
