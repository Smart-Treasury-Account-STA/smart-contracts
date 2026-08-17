# Tranche 2 Review Guide — Smart Treasury Account (STA)

SCF submission: <https://communityfund.stellar.org/submissions/recmj5cqlrqKyd1Bc>
This document is for Stellar Community Fund reviewers validating **Tranche 2 — Testnet**. It explains what was built for each of the three deliverables and gives exact, reproducible steps to test and validate each one — no trust required, everything below can be re-run independently against live testnet state.

Two repositories make up Tranche 2:

| Deliverable | Repository |
|---|---|
| 1. Testnet Smart Contracts Deployment | `smart-contracts` (this repo) |
| 2. Testnet dApp and Wallet Flow | `dApp` |
| 3. Testnet Scheduled Payment Relayer | `dApp` |

Contract source lives entirely in this repo; the operator-facing app and the relayer service both live in `dApp`. This file is identical in both repos so reviewers land on it regardless of which one they open first.

**Two ways to validate.** Everything below can be checked either (a) directly against the contracts with the Stellar CLI — no dependency on the dApp, no wallet, no allowlisting, works for anyone (Deliverable 1's section) — or (b) interactively through the deployed dApp with a browser wallet (Deliverable 2 and 3's sections). Path (a) covers reads and rejections; path (b) additionally covers the wallet UX itself, but **writing** through the dApp needs your wallet address registered as a treasury signer first — see the allowlisting note under Deliverable 2.

---

## Deliverable 1 — Testnet Smart Contracts Deployment

**What "done" means (verbatim from the SCF milestone):**
> Deploy the smart contracts to Stellar testnet and validate treasury setup, signer management, policy configuration, transaction simulation, and SAC-based payment execution.
>
> Completion measure: *"Testnet contract addresses are available. A developer can configure or inspect a testnet treasury account, execute an approved payment, and confirm that invalid actions are rejected."*

### What was built

All 7 contracts of the V1 workspace deployed on Stellar testnet, composing real OpenZeppelin Stellar contracts (`stellar-accounts`, `stellar-access`) for signer/passkey auth rather than a bespoke auth layer. `smart_account` is a genuine Soroban custom account (`CustomAccountInterface`) with policy-version pinning and nonce-based replay protection.

Full deployment record, including every WASM hash and every upload/create/initialize transaction: [`docs/TESTNET_DEPLOYMENT.md`](docs/TESTNET_DEPLOYMENT.md).

### Contract addresses (Stellar Testnet)

| Contract | Contract ID | Explorer |
|---|---|---|
| `smart_account` | `CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS) |
| `policy_engine` | `CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC) |
| `intent_registry` | `CDHTNPBXUMPCKUJ76HQ767MDRD4IVRRH4H5DOF4JUOO36QKSV4GXFRMR` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CDHTNPBXUMPCKUJ76HQ767MDRD4IVRRH4H5DOF4JUOO36QKSV4GXFRMR) |
| `recovery_manager` | `CALI5XJASA66LKZPF3ZF7HOLGOFUWZYIHB6SENXCZ5Y7QVT7UQKKR6UM` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CALI5XJASA66LKZPF3ZF7HOLGOFUWZYIHB6SENXCZ5Y7QVT7UQKKR6UM) |
| `transfer_adapter` | `CAX766XYR56WO7Y4HFOHYQUO5AIN5QLHAHKJ3DINXM2WUQY5UE7KGE26` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CAX766XYR56WO7Y4HFOHYQUO5AIN5QLHAHKJ3DINXM2WUQY5UE7KGE26) |
| `split_adapter` | `CAFTFU2E4MGZT6BLCVN2FAQB6GIBJRR5ICI7C7LZEMACBDUUTQKVHHV3` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CAFTFU2E4MGZT6BLCVN2FAQB6GIBJRR5ICI7C7LZEMACBDUUTQKVHHV3) |
| `webauthn_verifier` | `CD72MKTDNI3HLMNPA4YUOZGLT2NUWLVHTW3A7NQOXGLG7OKIKETSMXXR` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CD72MKTDNI3HLMNPA4YUOZGLT2NUWLVHTW3A7NQOXGLG7OKIKETSMXXR) |
| `STA` test asset (SAC) | `CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L) |

Network passphrase: `Test SDF Network ; September 2015`. Deployer: `GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB`.

### Key on-chain transactions

| What | Tx hash | Explorer |
|---|---|---|
| `smart_account.initialize` | `1f4fbd12802b0af5e54cff741ad22bb6877fd398a064dbf482fcf3ab7128c01c` | [link](https://stellar.expert/explorer/testnet/tx/1f4fbd12802b0af5e54cff741ad22bb6877fd398a064dbf482fcf3ab7128c01c) |
| `intent_registry.initialize` (admin = `smart_account`, hand-built custom-account auth) | `2d63d6d16d4f9bf9ba34f3301b72cbff1e4ff44273a49f1bf899051063e683b2` | [link](https://stellar.expert/explorer/testnet/tx/2d63d6d16d4f9bf9ba34f3301b72cbff1e4ff44273a49f1bf899051063e683b2) |
| `apply_adapter_change(transfer)` — real adapter wired in | `6257c3a52127c70747603d32efe5ba71dbf54b5f341627982327b5a488151333` | [link](https://stellar.expert/explorer/testnet/tx/6257c3a52127c70747603d32efe5ba71dbf54b5f341627982327b5a488151333) |
| **Real signer-authorized `execute_transfer_payment`** — treasury `STA` balance moved `1,000,000,000 → 995,000,000`, `is_nonce_used(1)` flipped `false → true` | `f712d5609ca52226746ad9b6776240b763d597246808df1c2a844bf1905d8131` | [link](https://stellar.expert/explorer/testnet/tx/f712d5609ca52226746ad9b6776240b763d597246808df1c2a844bf1905d8131) |

Full transaction list (all WASM uploads, contract creates, policy/signer wiring): see §4–§5 of [`docs/TESTNET_DEPLOYMENT.md`](docs/TESTNET_DEPLOYMENT.md).

### How to test and validate — reproduce these yourself

Requires the [Stellar CLI](https://developers.stellar.org/docs/tools/cli) (`stellar --version` ≥ 23) and any funded testnet identity as `--source-account` (a fresh one via `stellar keys generate <name> --network testnet --fund` works fine — none of the reads below need any specific signer).

**1. Inspect the treasury (live state, run any time):**

```bash
stellar contract invoke --id CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS \
  --source-account <your-identity> --network testnet --send=no -- status
# => {"frozen":false,"initialized":true,"paused":false,"policy_version_hint":0}

stellar contract invoke --id CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC \
  --source-account <your-identity> --network testnet --send=no -- version
# => 1
```

**2. Confirm the approved payment executed (§Transactions above) actually moved funds and consumed its nonce:**

```bash
stellar contract invoke --id CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS \
  --source-account <your-identity> --network testnet --send=no -- is_nonce_used --nonce 1
# => true — this nonce was spent by tx f712d560… above
```

**3. Confirm invalid actions are rejected — three independent policy checks, live against current on-chain state:**

```bash
# Stale pinned policy version (current live version is 1; pin the check to 99)
stellar contract invoke --id CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC \
  --source-account <your-identity> --network testnet --send=no -- validate_policy \
  --check '{"amount":"5000000","asset":"CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L","destination":"GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU","expected_version":99,"operation":"transfer"}'
# => Error(Contract, #2006)  VersionMismatch

# Destination not on the allowlist
stellar contract invoke --id CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC \
  --source-account <your-identity> --network testnet --send=no -- validate_policy \
  --check '{"amount":"5000000","asset":"CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L","destination":"GCWFJKLE45TMVZS42TMIYKAORKGBWE74753YPOSCC5ESJR2G2UMBXBDB","expected_version":1,"operation":"transfer"}'
# => Error(Contract, #2004)  RecipientNotAllowed

# Amount above the configured cap (10,000,000)
stellar contract invoke --id CC5FSUNWBNH3EIEELHO3A4ZPJZRAZCVMFNP3PVXO2YBWDNNLTFLWBVHC \
  --source-account <your-identity> --network testnet --send=no -- validate_policy \
  --check '{"amount":"20000000","asset":"CCOUVA654JH2V6B7LNTKHJP5DF3QA553RS2IIWXSGPDFH2N3QILIVU5L","destination":"GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU","expected_version":1,"operation":"transfer"}'
# => Error(Contract, #2005)  AmountAboveLimit
```

Re-run right before review — all three were re-verified live against current chain state while writing this document (2026-08-17) and returned exactly the codes above.

**4. Local contract test suite** (proves every edge case the two live rejections above don't individually cover — replay, frozen treasury, paused state — via `mock_all_auths()` against real contract instances):

```bash
cargo test --workspace
# => 120 tests, 0 failed (re-verified 2026-08-17)
```

### Known nuance (not blocking) — why the two rejections above carry no transaction hash

The two live rejections shown on the public ledger above (§3, step 3) are read-only `validate_policy` simulations (`--send=no`). This isn't a gap in what was demonstrated; it's a consequence of how Soroban's transaction-preparation flow works, and it applies to every client, not just this one.

**The mechanism:** on Soroban, `simulateTransaction`/`prepareTransaction` isn't just a preview — it's how the RPC computes the resource footprint (ledger keys read/written, instructions, fees) a transaction needs in order to be well-formed and signable. When the underlying contract call is going to fail, that simulation itself fails and returns an error with **no footprint at all**. Without a footprint there's nothing to build a valid transaction envelope from, so it's impossible to ever reach the sign-and-send step for a call that's designed to be rejected — the RPC never gets far enough to hand back something submittable.

**Confirmed directly**, not just reasoned about: `scripts/execute_demo_transfer_payment.py` was run twice against this deployment with a second, independently registered signer (`muwp-freighter`, delegated signer on context rule 1 — added after the original deployment, distinct from the founding deployer key on rule 0):

- `--nonce 1` (already consumed) → `__check_auth` genuinely succeeded (the custom-account authorization is real and correct), and the call was then rejected with `Error(Contract, #8005)` (`NonceAlreadyUsed`) — exactly as expected — but only at the simulation step, before `send_transaction` was ever reached.
- `--expected-policy-version 99` → same pattern, rejected with `Error(Contract, #2006)` (`VersionMismatch`), again only at simulation.

Both runs prove the rejection logic is correct on live testnet, with a second signer under a second context rule as independent confirmation — they just can't produce a tx hash, because Soroban itself won't hand back a submittable envelope for a call it can already tell will fail. The dApp hits the identical wall for the same reason (`discoverTreasuryInvocation` and `prepareTransaction` in `dApp/src/lib/stellarClient.ts` both simulate before ever asking a wallet to sign).

**What would be needed for a hash anyway:** bypass simulation-based preparation entirely — harvest a resource footprint from a throwaway *valid* simulate (never sent), manually attach it to the failing call, and submit directly without going through `prepareTransaction`. That's additional one-off tooling, not something any Soroban client does by default, and the milestone's completion measure only requires confirming "that invalid actions are rejected" — which is met by the local test suite plus the live, reproducible commands in step 3 above.

---

## Deliverable 2 — Testnet dApp and Wallet Flow

**What "done" means (verbatim from the SCF milestone):**
> Build the testnet dApp for treasury operators. This includes wallet connection, treasury dashboard, signer and policy screens, payment preparation, transaction simulation, wallet approval, transaction submission, status tracking.
>
> Completion measure: *"A user can open the testnet dApp, connect Freighter, configure or inspect a treasury account, prepare a payment, simulate it, approve it, submit it, and view the result."*

Repository: `dApp` (branch `testnet`).

### What was built

- **Wallet connection**: Stellar Wallets Kit, Freighter + xBull selectable, no bespoke wallet protocol. Explicit error surfaced if the connected wallet lacks `signAuthEntry`.
- **Treasury dashboard**: SmartAccount status (initialized/paused/frozen), live policy version, contract addresses with explorer links, context rules, signer records with roles/weights/thresholds — all backed by live RPC reads/simulations, no seeded demo data.
- **Signer and policy screens**: read *and write* (add/revoke/re-weight signers, update asset rules, recipient allowlist, amount caps), all routed through the same SmartAccount custom-authorization model as payments.
- **Payment pipeline**: address/amount validation → live `policy_engine.version()` read → `validate_policy` simulation → `is_nonce_used` check → build `execute_transfer_payment` root invocation → assemble the SmartAccount `AuthPayload` + delegated-signer auth entries → prepare via Soroban RPC → wallet signature → submit → poll to terminal status (`ERROR`/`TRY_AGAIN_LATER`/`DUPLICATE` handled explicitly) → visible step timeline with the resulting tx hash and explorer link.
- **Scheduled payment creation**: 32-byte intent ID / ledger-window / amount validation, existing-intent check, simulation before submit, queued for the relayer only after terminal success.

### How to test and validate

**1. Setup**

```bash
git clone git@github.com:Smart-Treasury-Account-STA/dApp.git
cd dApp
pnpm install
cp .env.example .env.local   # already points at the deployment addresses above
pnpm dev
```

Open `http://localhost:3000`. Install [Freighter](https://www.freighter.app/) or xBull, switch it to **Testnet**, and fund a fresh keypair via [Friendbot](https://friendbot.stellar.org).

> **Note — signer allowlisting.** Connecting a wallet and reading treasury state (dashboard, signer/policy screens, `Policy check` / `Nonce check` / `Simulate` buttons — steps 1–5 below) works with **any** testnet wallet address; nothing is gated. But the **`Approve & submit` button on a write** (a payment, a scheduled payment, a signer/policy change — step 6–7 below) will fail with *"Connected wallet is not a delegated signer on any SmartAccount context rule"* unless the connected address is one of the treasury's registered signers (`src/lib/stellarClient.ts` checks `matchedRule.signerAddresses.includes(wallet.address)` before ever asking the wallet to sign). Today that's only the deployer key (context rule 0) and one internal test signer (context rule 1). **If you want to test a write end-to-end with your own wallet, send us your testnet public key first so we can register it as a delegated signer** — either directly, or we can add it live from an already-registered wallet through the dApp's own signer-add screen.

**2. Manual flow to exercise** (matches the milestone's completion measure exactly):

1. Connect Freighter — wallet modal, public key shown.
2. Open the treasury dashboard — confirm it shows the *live* state from step "Inspect the treasury" above (not placeholder data).
3. Open the signer/policy screens — confirm the registered signer and the `STA` asset rule/allowlist/cap match what's on-chain.
4. Prepare a payment (amount ≤ the cap, destination = the allowlisted recipient `GAK3XILRBYBMBOCZMSLL2CLR6WPQLEIOC6ZCYYPTE4OIAX3PCFFO2YMU`) — watch the simulate step run `validate_policy` before any wallet prompt.
5. Try an amount above the cap or an unlisted destination — confirm the dApp surfaces the rejection (`AmountAboveLimit` / `RecipientNotAllowed`) *before* asking for a signature.
6. Approve and submit a valid payment (**requires an allowlisted signer, see note above**) — confirm the step timeline shows simulate → sign → submit → confirmed, with a tx hash linking to `stellar.expert`.
7. Create a scheduled payment (**requires an allowlisted signer**) — confirm it validates the ledger window and gets queued for the relayer (see Deliverable 3).

**3. Automated checks**

```bash
pnpm lint       # eslint — clean as of 2026-08-17
pnpm typecheck  # tsc --noEmit — clean as of 2026-08-17
pnpm test       # vitest — 123/123 passing as of 2026-08-17
pnpm build      # production build
```

CI runs the same four checks on every push (`.github/workflows/ci.yml`).

### Known gaps (not blocking, documented for transparency)

- No network switch — testnet only in this build, by design for Tranche 2.
- No cancel-intent / recovery / audit-history UI — out of Tranche 2 scope, listed under Tranche 3.
- The dApp configures an *existing* treasury; it does not bootstrap a brand-new `smart_account` (that's CLI/ops tooling, per `docs/DAPP_INTEGRATION_SPEC.md` §9 in this repo — the milestone text only requires "configure or inspect **a** treasury account").

---

## Deliverable 3 — Testnet Scheduled Payment Relayer

**What "done" means (verbatim from the SCF milestone):**
> Build a relayer service for one scheduled treasury payment flow. The relayer submits valid transactions, tracks execution status, logs failures, and cannot custody assets or bypass SmartAccount policy checks.
>
> Completion measure: *"A user can configure a scheduled payment on testnet and verify that the relayer executes it only when SmartAccount policy allows execution."*

Also lives in the `dApp` repository (`src/lib/relayer/`, `src/app/api/relayer/*`).

### What was built

- Uses `@stellar/stellar-sdk`'s `rpc.Server` throughout — no hand-rolled RPC client.
- Before every submission, re-reads `intent_registry.get_intent` and `intent_registry.is_child_executed` **on-chain** — never trusts client-supplied schedule bounds. This is the "exactly once" guarantee the deliverable is specifically about.
- Calls the two-argument `execute_scheduled_payment(intent_id, child_sequence)` — the contract itself resolves asset/destination/amount/policy, the relayer cannot override any of it.
- The relayer key (`GBFOESUTANPJZVQUZVKC5YCS4FB6AE5SX2R3JEDRK5JHKR22JXV4VNIV`) is registered in `intent_registry` purely as an `Executor` — no custody, no policy authority. Confirmed by direct ledger-entry read (the contract exposes no getter for this, so it can't be queried via a simple `invoke`; see the Developer Guide in this repo's sibling docs for the read method).
- Mutating relayer endpoints require `x-relayer-token`; the executor secret and admin token never reach a `NEXT_PUBLIC_` variable, the client bundle, or a log line — the admin token is exchanged once for an HMAC-signed httpOnly session cookie, never held in browser state.
- Local child-sequence counter advances only after a terminal successful transaction or a confirmed already-executed child — RPC retry/pending/duplicate/failure states leave it unchanged.
- Standalone CLI runner (`pnpm relayer:run`) for cron/operator use, on top of the protected HTTP endpoints.

### How to test and validate

**1. Setup** (continuing from the Deliverable 2 setup):

```bash
# in .env.local, set:
RELAYER_EXECUTOR_SECRET=<a funded testnet secret registered as intent_registry Executor>
RELAYER_ADMIN_TOKEN=<any string>
pnpm dev   # keep running
```

**2. End-to-end flow:**

1. In the dApp, create a scheduled payment with a near-future ledger window (use the ledger-window helper to compute one a few minutes out).
2. Confirm the intent was created on-chain: `intent_registry.get_intent --intent-id <id>` returns it.
3. Before the window opens, run the relayer and confirm it refuses the job (out-of-window):
   ```bash
   RELAYER_APP_URL=http://localhost:3000 pnpm relayer:run
   ```
4. Once the window opens, run it again — confirm it submits `execute_scheduled_payment`, the job status moves to a terminal success, and the resulting tx hash/explorer link is recorded in the job entry.
5. Run it a third time for the same job — confirm it refuses to re-execute the already-consumed child sequence (checked against `intent_registry.is_child_executed`, not just local state).
6. Grep the running process output and `.env.local` handling to confirm `RELAYER_EXECUTOR_SECRET` never appears in a browser network request (Network tab) or client bundle (`grep -r RELAYER_EXECUTOR_SECRET .next/static` should return nothing after `pnpm build`).

**3. Automated checks:** relayer-specific unit tests run as part of `pnpm test` (`src/lib/relayer/auth.test.ts`, `store.test.ts`, `session.test.ts`, `executor.test.ts`).

`src/lib/relayer/executor.test.ts` (18 tests) covers the replay-protection paths directly, with the Soroban RPC boundary mocked and the branching logic exercised for real: execution-limit reached, window not yet open, window expired, intent cancelled on-chain, child sequence already executed on-chain (and the local `childSequence` correctly advancing past it), and every terminal/non-terminal `sendTransaction`/`getTransaction` outcome (`ERROR`, `TRY_AGAIN_LATER`, `DUPLICATE`, `SUCCESS`, `FAILED`, still-pending after all poll attempts) — confirming the child sequence advances only on a genuine terminal success, never on a retryable or ambiguous state.

### Known limitation (not blocking, documented for transparency)

The relayer's job store is a single-instance JSON file — documented as a testnet-scale limitation, not a Tranche 2 requirement (multi-instance storage is explicitly out of scope for this tranche per the spec).

---

## Bottom line

All three "how to measure completion" criteria are met on live testnet, reproducible with the commands above:

- Deliverable 1: contract addresses published, treasury inspectable, an approved payment genuinely executed on-chain, invalid actions demonstrably rejected.
- Deliverable 2: a user can connect Freighter, configure/inspect the treasury, and run the full prepare → simulate → approve → submit → view pipeline.
- Deliverable 3: the relayer executes a scheduled payment only when SmartAccount policy allows it, verifiable against on-chain state before and after.

One open, non-blocking item: a hash-bearing (rather than simulated) rejection transaction for the stale-version/nonce-replay cases (Deliverable 1). The relayer's replay-protection paths (Deliverable 3) are now covered by `src/lib/relayer/executor.test.ts`.
