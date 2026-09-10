# Mainnet Deployment: Setup Record

This is the live record of deploying `contracts/account_factory` and a first
example treasury to Stellar **mainnet** (Public Network), on branch
`main`, using `stellar 26.0.0`, `soroban-sdk 26.1.0`,
`Public Global Stellar Network ; September 2015`.

This deployment deliberately mirrors `docs/TESTNET_FACTORY_DEPLOYMENT.md`'s
process (one `account_factory` call instead of ten-plus manual steps), on
the current, fully security-reviewed contract code
(`docs/SECURITY_REVIEW_STRICT.md`, 29 findings, all fixed except finding #4,
which is a documented, explicitly accepted risk for this beta — see §6).

## 1. Identities

| Role | Address | Source |
|---|---|---|
| Deployer / treasury owner / founding signer / executor | `GAHNF7XS5D2YLNK3PPW5DVEDJDKVEE5NJ6POQVMM36HODCNVALTSNIY7` | Freighter wallet, imported locally as `mainnet-deployer` (HD index 0 of a shared recovery phrase) |
| Test recipient A | `GCOMBIYWZ3HIMAS7FLFERQMVZIDVIMFZAAV7A2KMCDV3RAKP5NL5R5BM` | Same phrase, HD index 1, local alias `mainnet-test-a` |
| Test recipient B | `GAVGI2TA5S4PDPYLD5GDQIALIOIBXP44VL65LEEVQ4YKB6RBOZ7WFVS6` | Same phrase, HD index 2, local alias `mainnet-test-b` |
| Test recipient C | `GDHF73TQO774WWC2W46CIYUZDZUB372OIQHZ5KYWKQCFUHEIJPEX2BQ3` | Same phrase, HD index 3, local alias `mainnet-test-c` |

**Security note:** the recovery phrase behind these four addresses was
shared in a chat session to enable local signing for this deployment. It
should be treated as compromised — migrate any funds meant to be held
long-term to a freshly generated, never-shared wallet once mainnet testing
is complete.

## 2. Assets used

| Asset | Contract (SAC) | Issuer | Notes |
|---|---|---|---|
| XLM (native) | `CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA` | n/a (native) | No trustline needed, ever |
| USDC | `CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75` | `GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` (Circle) | `AUTH_REQUIRED: false` — confirmed on issuer flags; a classic account only needs a trustline (`change_trust`), no separate issuer authorization step |

## 3. WASM hashes uploaded

All six contracts were built via `stellar contract build --optimize` from
this repo's current source (branch `main`, based on
`feat/governance-account-and-security-hardening`) and uploaded individually
so `account_factory` can reference them by hash:

| Contract | WASM hash | Size (optimized) |
|---|---|---|
| `policy_engine` | `f05348d6cbe90796c2b89296697f340d2fba6a1b070f3830f17edf32c2cb8eb8` | 7 792 bytes |
| `intent_registry` | `43567e68bc87211cbdcd08a13e8a6de094ed978261b59b07d942a831ce527b70` | 10 742 bytes |
| `recovery_manager` | `0eb4038ea129be586d5b324c62c672c8203547b61146f17452693e587a9f92d8` | 20 561 bytes |
| `transfer_adapter` | `785870924ad04930512a5494cbf442b159c912a1f233491d1b0f696fe19f4dae` | 3 695 bytes |
| `split_adapter` | `e32ad0c7bf83d265848adc2ee4874aa0b3d27fb9b97558c4f8753b40a0ba329b` | 4 042 bytes |
| `smart_account` | `886bcd312f3cf973a8a037479ebf9febe57139e63f109947acce20bd835f3a76` | 68 336 bytes |
| `account_factory` | `15f635356ddf3497788d4f6aab7f6e93cd2fbdaab36ecbe77d0c5521e403a3dd` | 8 838 bytes |

**Cost note:** these uploads cost far more than the equivalent testnet
operation — a real, verified, network-wide Soroban state-rent phenomenon
(CAP-0066: fees ramp steeply once total mainnet Soroban state exceeds its
3 GB target), not a bug or something specific to these contracts. Total
spent on the six sub-contract uploads plus the factory itself: **104.43
XLM** (exact, verified via Horizon — see the per-transaction costs in
`docs/MAINNET_TESTING_TRANSACTIONS.md` §1). By contrast, creating a new
contract *instance* from an already-uploaded hash (what every subsequent
`deploy_account` call does) costs only ≈0.02–0.4 XLM — cheap, because it
writes a tiny fraction of the bytes a fresh code upload does.

### 3.1 Reproducing these hashes

The hash Stellar assigns to uploaded WASM is the SHA-256 of the file, so
"reproducible" means: build from this commit and get these exact bytes.
`scripts/verify_build.sh` automates the comparison (it reads the table above
as its expected values; `--fixtures-only` checks the six committed
`contracts/account_factory/src/wasm_fixtures/` without building, `--container`
rebuilds in the reference environment, `--wasm-dir` compares any build
output). What that reproduction depends on, measured on 2026-09-09:

| Environment | Matches |
|---|---|
| GitHub Actions `ubuntu-latest` (x86_64), Rust 1.94.1, stellar-cli 26.0.0 built by `cargo +1.96.0 install --locked` — `.github/workflows/ci.yml` on this commit, run 34246998029 | 6 / 6 fixtures byte for byte (the CI staleness check) |
| macOS arm64, Rust 1.94.1, stellar-cli 26.0.0 — built the same way **or** the official release binary (identical output) | 5 / 7 (`recovery_manager`, `smart_account` differ inside the code section, identical meta) |
| Linux arm64 container, Rust 1.94.1, stellar-cli 26.0.0 release binary | 2 / 7 (`transfer_adapter`, `account_factory`) |
| linux/amd64 container mirroring `ci.yml` (`scripts/verify_build.sh --container`) | not runnable from the Apple Silicon machine used here: qemu-user emulation of amd64 crashes `rustc`. Needs a native x86_64 host or a Rosetta-enabled runtime. |
| linux/arm64 container, same recipe (`CONTAINER_PLATFORM=linux/arm64`), stellar-cli 26.0.0 compiled from source | not completed here either: the local container VM has 2 GiB of memory and `rustc` is SIGKILLed compiling stellar-cli's `stellar-xdr` crate even with `CONTAINER_JOBS=1`. The recipe needs a VM with more memory. |
| macOS arm64, Rust 1.94.1, stellar-cli **28.0.0** | 0 / 7 — every size identical, only the `cliver` meta entry differs |

Two things follow. First, the stellar CLI version is part of the artifact:
`stellar contract build` writes it into the `cliver` contract-meta entry
(`stellar contract info meta --wasm <file>`), so 26.0.0 exactly is required
and the script refuses any other version. Before this note the version was
pinned only by `ci.yml`'s install step and never stated as a reproducibility
requirement (the docs site lists "Rust: stable"). Second, with the CLI
version held fixed the output still depends on the host the build runs on,
and the divergence is not confined to one stage: the same 26.0.0, whether
installed from source or as the release binary, yields identical bytes on
one host and different bytes on another OS/architecture. Comparing the
**unoptimized** output (`stellar contract build` without `--optimize`, i.e.
what `rustc` 1.94.1 emits) between macOS/arm64 and linux/arm64 already shows
five of nine crates differing (`smart_account`, `policy_engine`,
`intent_registry`, `split_adapter`, `governance_account`) — so part of it is
compiler codegen; and `recovery_manager`, identical before optimization on
both, differs from the deployed bytes after it — so part of it is
`wasm-opt`. Every difference observed is inside the code section with
identical meta and, for six of seven contracts, identical size: equivalent
code, not identical bytes. Rust 1.94.1, soroban-sdk 26.1.0, and the
release profile (`Cargo.toml`) are pinned by the repository; the host is
pinned only by the CI recipe (`ubuntu-latest`, x86_64). That recipe is
therefore the reference environment: `scripts/verify_build.sh --container`
follows `ci.yml` step for step in a linux/amd64 container and is expected
to reproduce all seven hashes, but this has not yet been demonstrated from
an arm64 machine (see the table); the CI workflow on this commit has
demonstrated the six fixtures.

`account_factory` (not a committed fixture) was reproduced from macOS by the
source-built 26.0.0, so every recorded hash has been rebuilt from source at
least once outside CI.

`webauthn_verifier` was **not** deployed — it is stateless and only needed
if a treasury registers a passkey (`Signer::External`) signer; this
deployment only uses `Signer::Delegated` (plain wallet keys), matching
`account_factory`'s own module doc comment that a shared instance is only
needed for that case.

## 4. `account_factory`

- **Deployed at:** `CCFIPN4TIF5XOJ7SZCQNET7YSXUXS7ERLJHX3JHNVZPTJTFI4HTKQAAV`
- **Admin:** `GAHNF7XS5D2YLNK3PPW5DVEDJDKVEE5NJ6POQVMM36HODCNVALTSNIY7`
- Initialized with the six WASM hashes above.

## 5. First treasury — deployed via `deploy_account`

One call, one transaction, six sub-contracts deployed and wired together
(see `docs/MAINNET_TESTING_TRANSACTIONS.md` §3 for the transaction and its
full event log):

| Contract | Address |
|---|---|
| **`smart_account`** | `CDTE6DBMGFPTLBLPS7O32FMZGEKUM6KAW7GPACSI6RPY73DDFJIBVL7W` |
| `policy_engine` | `CAMM3LIYYW57HU2YF3FEQS5ARYN7ZILF4ETFHTBYHUBNU4SF6MHIOORM` |
| `intent_registry` | `CDCTQSJEGOVTD63M34VSUYKIX6KMZL53P646G4QCER75Q4V4EKWRL3F6` |
| `recovery_manager` | `CB6SOTBREWNAYNUYKIXTNTAYDKBOBIH4MUDRU36X326NVYQDBCMIKIBP` |
| `transfer_adapter` | `CAJYFE76XIRQVEUPI6O3JMH2LZCS3VXHYEF3VO3L5FID4ACNPBK4HVOC` |
| `split_adapter` | `CDZMKHLBRLEHES62KSQE6UJBHV4H2VHJ6K2D6I6K66NJBMROSRLPOLC7` |

**Deploy parameters:**
- `caller` / `owner`: `GAHNF7XS...NIY7` (mainnet-deployer)
- `initial_signers`: `[Delegated(GAHNF7XS...NIY7)]` — one founding signer, registered under context rule `0` (type `Default`, no policy attached — see §6 for why that matters)
- `initial_policies`: `{}` (empty at deploy time; configured afterward, see §6)
- `guardian_threshold`: `1`
- `executor`: `GAHNF7XS...NIY7` (mainnet-deployer acts as its own relayer for this deployment — see `docs/DAPP_DEVELOPER_QA.md`'s relayer explanation for why this is safely reconfigurable later via `set_executor`)
- Adapters bound immediately (no timelock — first-ever binding on a fresh treasury, per `smart_account::initialize`'s own design)
- `intent_registry` bootstrapped automatically via `smart_account::initialize`'s invoker-shortcut (no separate hand-built authorization needed)

## 6. Post-deploy policy configuration

`policy_engine`'s admin is the `caller` directly (`GAHNF7XS...NIY7`), **not**
`smart_account` — confirmed in `contracts/account_factory/src/lib.rs`
(`PolicyEngineClient::new(&env, &policy_engine).initialize(&caller)`), so
these calls use a plain classic signature, not `smart_account`'s
`AuthPayload`:

- `set_operation_allowed("transfer", true)`
- `set_operation_allowed("split", true)`
- `set_asset_rule(XLM, {enabled: true, max_single_transfer: 500000000})` — 50 XLM cap per transfer
- `set_asset_rule(USDC, {enabled: true, max_single_transfer: 10000000})` — 1 USDC cap per transfer
- `set_recipient_allowed(test-a, true)`, same for test-b and test-c

**Known, deliberately unfixed residual risk (finding #4, accepted for this
beta):** `add_signer`/`add_context_rule`/`add_policy` (and their
removal/update counterparts) on `smart_account` are gated by the treasury's
own signer/context-rule system, **not** by `owner`. Concretely: whoever
satisfies context rule `0` today can add new signers, new rules, or new
policies with zero `owner` involvement. This was demonstrated live during
testing (see `docs/MAINNET_TESTING_TRANSACTIONS.md` §6) — adding a second
signer to rule `0` (which has no policy attached) immediately flipped it
from "any one signer suffices" to "all registered signers must
co-sign," locking `mainnet-deployer` out of further rule-`0`-gated actions
until the second signer's key co-signed a fix. Explicitly accepted as a
beta-scope risk per the user's decision — not corrected in this deployment.

## 7. Funding

The treasury (`smart_account`) was funded directly via the SAC `transfer`
function (source: `mainnet-deployer`):

- **100 XLM** — see `docs/MAINNET_TESTING_TRANSACTIONS.md` §4
- **1 USDC** — same section

## 8. What was deliberately not exercised

- **`governance_account`** (the N-of-M multisig option for `owner`,
  `docs/GOVERNANCE_MULTISIG_DESIGN.md`) was not deployed for this beta —
  `owner` is a single key (`mainnet-deployer`), matching the "skip for
  beta" decision.
- **Recovery flow** (guardian freeze, recovery proposal/approval/finalize)
  and **adapter reconfiguration** (`propose_adapter_change`/
  `apply_adapter_change`) both carry real, production-value timelocks
  (~24h, `17280` ledgers) on this deployment — unlike earlier testnet runs,
  these were **not** temporarily shortened for this mainnet deployment (the
  reduced-delay pattern used on testnet was explicitly testnet-only and
  never applied to committed source). These entrypoints exist and are
  wired correctly, but a same-session end-to-end proof of a completed
  recovery is not possible on mainnet within a single sitting — only
  initiation was practical to demonstrate, if at all (see
  `docs/MAINNET_TESTING_TRANSACTIONS.md` for what was actually run).
