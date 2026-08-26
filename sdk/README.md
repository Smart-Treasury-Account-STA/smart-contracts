# Smart Treasury Account SDK

TypeScript SDK for Smart Treasury Account: typed contract clients generated
from the deployed contracts' own specs, transaction-preparation helpers for
`smart_account`'s custom authorization, event parsing, and first-class
policy-version/replay-state/recovery-state types.

Everything here is exercised against **Stellar testnet**. See
`docs/SDK_TESTNET_RELEASE.md` (repo root) for what that maps to versus a
future mainnet release, and how to verify each piece independently.

## Layout

```
generated/            Per-contract TypeScript clients, from
                       `stellar contract bindings typescript` (regenerate
                       with scripts/generate-bindings.sh). Minimally
                       touched -- just renamed into the @sta scope.
packages/core/         "sta-sdk" -- the curated layer on top:
  src/config.ts         Network config: testnet populated with the live
                         deployment; mainnet has a real, ready-to-use
                         shape (verified passphrase, `buildMainnetConfig`)
                         but no contract addresses, since no mainnet
                         deployment exists yet -- see "Mainnet" below.
  src/auth.ts            smart_account's custom AuthPayload ("Entry
                          A"/"Entry B") construction, and the relayer's
                          non-root executor authorization.
  src/payments.ts        prepare -> simulate -> approve helpers for
                          transfer/split/scheduled payments, plus
                          sign+submit+poll.
  src/events.ts           Typed parsers for TransferPaid/SplitPaid/
                           ScheduledPaymentExecuted/PolicyValidated/
                           IntentCreated/RecoveryOpened/etc. -- the
                           generated bindings don't expose event types at
                           all, only function signatures, so this is
                           hand-curated against each contract's actual
                           `#[contractevent]` structs.
  src/state.ts            First-class typed reads: AccountStatus, policy
                           version, nonce-used, ContextRule,
                           RecoveryRequest.
examples/               Runnable end-to-end scripts, each verified against
                         live testnet while building this SDK (see
                         docs/SDK_TESTNET_RELEASE.md for the transaction
                         evidence).
```

## Setup

```bash
cd sdk
npm install
npm run build
```

(The root `build` script deliberately builds `generated/*` before
`packages/core` in two explicit passes, not a single `--workspaces` call --
`packages/core` imports the generated packages' compiled output, and plain
npm workspaces does not order builds by dependency graph on its own; a
single-pass build can fail on a truly fresh clone depending on workspace
discovery order.)

To regenerate the bindings from current contract source (after any Rust
change):

```bash
bash scripts/generate-bindings.sh   # from the repo root, or:
sdk/scripts/generate-bindings.sh
npm install   # re-links the regenerated packages
```

## Running the examples

Each example reads its secrets from environment variables -- see
`examples/.env.example` for the full list. Get real testnet secret keys
with `stellar keys show <identity>`; never commit a populated `.env`.

```bash
npx tsx examples/01-read-treasury-state.ts

OWNER_SECRET_KEY=... FEE_SOURCE_SECRET_KEY=... \
ASSET=... DESTINATION=... AMOUNT=1000000 NONCE=<unused-nonce> \
npx tsx examples/02-execute-transfer-payment.ts
```

`01` and `06` are read-only (no env vars needed beyond the optional
overrides `06` documents). `02`-`05` submit real transactions.

## Two authorization patterns, by design

`smart_account` is a Soroban custom account -- most of its entrypoints
need the `AuthPayload`/Entry-A/Entry-B construction in `auth.ts`
(`payments.ts` wraps this). Everything else in this workspace (the
factory's `deploy_account`, a relayer's `mark_child_executed`) uses plain
account authorization, and either the generated client's own
`basicNodeSigner`/`signAndSend` convenience (example `05`) or one
explicit authorization entry (`buildExecutorAuthEntry`, example `04`) is
enough -- no custom payload involved. Reach for `payments.ts` only for
calls that actually touch `smart_account`'s spend/schedule authority;
everything else can use the generated client directly.

See `docs/DAPP_INTEGRATION_SPEC.md` (repo root) for the full mechanics
this SDK implements, including one correction made while building it
(§8's relayer auto-fill claim -- see that section).

## Mainnet

No mainnet deployment exists yet, and this SDK does not fabricate one --
`NETWORKS.mainnet` stays `undefined` until it's given real, deployed
contract addresses. What's ready now is everything *except* those
addresses:

- `MAINNET_NETWORK_PASSPHRASE` -- the real, verified mainnet passphrase
  (`Public Global Stellar Network ; September 2015`), a fixed protocol
  constant independent of any deployment.
- `buildMainnetConfig(contracts, rpcUrl?)` -- returns a real `NetworkConfig`
  for mainnet once you have real contract addresses. Every helper in this
  SDK (`auth.ts`, `payments.ts`, `state.ts`) already takes a `NetworkConfig`
  as a parameter rather than assuming testnet, so nothing else needs to
  change to point them at mainnet.

**No default RPC URL is hardcoded.** Unlike testnet (SDF hosts
`https://soroban-testnet.stellar.org` for free), SDF does not operate a
free public mainnet Soroban RPC endpoint -- every mainnet RPC is a
third-party ecosystem provider (see
[developers.stellar.org/docs/data/apis/rpc/providers](https://developers.stellar.org/docs/data/apis/rpc/providers)).
Pick one and either pass its URL to `buildMainnetConfig` directly or set
`STA_MAINNET_RPC_URL`.

```ts
import { buildMainnetConfig } from "sta-sdk";

const MAINNET = buildMainnetConfig(
  {
    smartAccount: "C...",       // from a real mainnet deploy_account call
    policyEngine: "C...",
    intentRegistry: "C...",
    recoveryManager: "C...",
    accountFactory: "C...",
    transferAdapter: "C...",
    splitAdapter: "C...",
  },
  "https://your-chosen-provider.example/soroban/rpc", // or set STA_MAINNET_RPC_URL
);
```

A `NetworkConfig` with `network: "mainnet"` and real signing keys submits
real, fee-paying, fund-moving transactions against Stellar's production
ledger -- don't build one just to "try it out" with testnet addresses.
Before actually deploying and using this in anger: a real mainnet
deployment needs its own funded deployer key, its own security review
sign-off, and careful handling entirely outside this SDK's scope.
