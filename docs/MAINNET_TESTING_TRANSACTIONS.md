# Mainnet Testing: Full Transaction Record

Every transaction below is real, submitted to Stellar mainnet, and
independently verifiable via the linked explorer. Addresses referenced
throughout are defined in `docs/MAINNET_DEPLOYMENT.md`.

Explorer base: `https://stellar.expert/explorer/public/tx/<hash>`

## 1. WASM uploads (one-time setup)

| # | Contract | Tx hash | Cost (XLM) | Explorer |
|---|---|---|---|---|
| 1 | `transfer_adapter` | `46e27b7d5715f4a1bd92bdee6e066168a7b63f7d789ccd483c3e13a4c98f1f26` | 4.86 | [view](https://stellar.expert/explorer/public/tx/46e27b7d5715f4a1bd92bdee6e066168a7b63f7d789ccd483c3e13a4c98f1f26) |
| 2 | `split_adapter` | `bc83fc6ce58b62df7c2df64f7ec747131276ead7941efdcc178c7c6e6b6aed48` | 3.93 | [view](https://stellar.expert/explorer/public/tx/bc83fc6ce58b62df7c2df64f7ec747131276ead7941efdcc178c7c6e6b6aed48) |
| 3 | `policy_engine` | `714d9e96915ee90e2c02b6667c5e025cdfaf45d8852222a1a6e83362bfacce4f` | 8.60 | [view](https://stellar.expert/explorer/public/tx/714d9e96915ee90e2c02b6667c5e025cdfaf45d8852222a1a6e83362bfacce4f) |
| 4 | `intent_registry` | `a1f42c7f55a43460092d269a5a766879d35a657bf4144c6eca60fd998b5ab486` | 10.68 | [view](https://stellar.expert/explorer/public/tx/a1f42c7f55a43460092d269a5a766879d35a657bf4144c6eca60fd998b5ab486) |
| 5 | `recovery_manager` | `6e73d66e0ec55c4b4899933fd9cfdb27632d7449738d669e30dbf74b565cef35` | 15.17 | [view](https://stellar.expert/explorer/public/tx/6e73d66e0ec55c4b4899933fd9cfdb27632d7449738d669e30dbf74b565cef35) |
| 6 | `smart_account` | `e47fce19545ea03ff721ee6964087d188387caefbcae72d2a32d53dcf327b9ae` | 51.55 | [view](https://stellar.expert/explorer/public/tx/e47fce19545ea03ff721ee6964087d188387caefbcae72d2a32d53dcf327b9ae) |
| 7 | `account_factory` (WASM upload leg of `contract deploy`) | `39ebbafe5fc0254836d48d3e6bfd07a310144fd6a8e3b70b36af2d833a8c8e8b` | 9.64 | [view](https://stellar.expert/explorer/public/tx/39ebbafe5fc0254836d48d3e6bfd07a310144fd6a8e3b70b36af2d833a8c8e8b) |

**Total for the 7 uploads: 104.43 XLM** (exact `fee_charged` sum, verified via Horizon).

Two earlier attempts (`intent_registry` first try, timeout; retry,
`TxInsufficientFee`) never landed on-chain and consumed no funds — omitted
here since they have no real transaction hash. Fixed by resubmitting with
an explicit higher inclusion fee (`--inclusion-fee 10000`).

**What each of these is:** `stellar contract upload` — writes the
contract's compiled WASM bytecode as a new ledger entry, permanently
addressable by its hash. This is the expensive step, explained in full in
the chat record: Soroban's state-rent mechanism (CAP-0066) charges steeply
for new persistent storage once total network state exceeds a 3 GB target,
which mainnet currently appears to be past.

## 2. `account_factory` deploy + initialize

| Step | Tx hash | Explorer |
|---|---|---|
| Deploy instance (from uploaded hash) | `673414b31a86310ff01aa0cf23ec9f26f5680a58f00ef099fca55e0d14a12086` | [view](https://stellar.expert/explorer/public/tx/673414b31a86310ff01aa0cf23ec9f26f5680a58f00ef099fca55e0d14a12086) |
| `initialize(admin, wasm_hashes)` | `604cd17c1c2f9d58f698bf17df3289f45910244fd287501149814d33a55c617c` | [view](https://stellar.expert/explorer/public/tx/604cd17c1c2f9d58f698bf17df3289f45910244fd287501149814d33a55c617c) |

**What this is:** the reusable factory contract itself. Deploying its
*instance* (as opposed to uploading its code, already counted in §1) cost
only a fraction of an XLM — creating an instance from an already-uploaded
hash writes far less data than uploading fresh code.

## 3. First treasury deployment

| Tx hash | Explorer |
|---|---|
| `7f40cecbabedeb13bd7ea886995051980c3870a0259c1a995d367dad5c6ddcfa` | [view](https://stellar.expert/explorer/public/tx/7f40cecbabedeb13bd7ea886995051980c3870a0259c1a995d367dad5c6ddcfa) |

**What this is:** `account_factory.deploy_account(...)` — **one**
transaction, one `caller.require_auth()` (satisfied six times internally,
once per sub-contract, all from the same key in one signing flow), and it
deployed and wired all six treasury contracts in one shot. Event log
observed, in order: five `Initialized` events (one per sub-contract other
than `smart_account`), `signer_registered`, `context_rule_added`,
`smart_account`'s own `Initialized`, and `account_factory`'s
`AccountDeployed`. Cost: ≈0.44 XLM.

## 4. Treasury funding

| Asset | Amount | Tx hash | Explorer |
|---|---|---|---|
| XLM | 100 | `049a8e9a260c679f0b7e211917abcb585b99606e543e96e4ec063f85a184c9f2` | [view](https://stellar.expert/explorer/public/tx/049a8e9a260c679f0b7e211917abcb585b99606e543e96e4ec063f85a184c9f2) |
| USDC | 1 | `2650dc8c6b915cc49b4290bf9390cd1265213fbae86e05db234c28546b83d3cc` | [view](https://stellar.expert/explorer/public/tx/2650dc8c6b915cc49b4290bf9390cd1265213fbae86e05db234c28546b83d3cc) |

**What this is:** a plain SAC `transfer` from `mainnet-deployer` to the
`smart_account` contract address — this is how a Soroban contract receives
a balance; no separate "trustline" step is needed for a contract address
(unlike a classic `G...` account).

## 5. Policy configuration (`policy_engine`, admin = `mainnet-deployer` directly)

| Action | Tx hash | Explorer |
|---|---|---|
| Enable `transfer` operation | `a9c46e9e0e18099eb4386eb8da6ecfa3d846c97a095a61e4d5e3c177c2acad5c` | [view](https://stellar.expert/explorer/public/tx/a9c46e9e0e18099eb4386eb8da6ecfa3d846c97a095a61e4d5e3c177c2acad5c) |
| Enable `split` operation | `5eec03619a9dd640220476dd5ae4713d254378299b27edf1e97835587a3f2599` | [view](https://stellar.expert/explorer/public/tx/5eec03619a9dd640220476dd5ae4713d254378299b27edf1e97835587a3f2599) |
| Enable XLM (cap 50/transfer) | `0e67e99d140f07ebadf6179e57a1786779c0e15d0f616f5af8ac4fd60c6b981e` | [view](https://stellar.expert/explorer/public/tx/0e67e99d140f07ebadf6179e57a1786779c0e15d0f616f5af8ac4fd60c6b981e) |
| Enable USDC (cap 1/transfer) | `8dd796004f27771730ab3a72be1d41588669445ef80065ef7730f3c219f13231` | [view](https://stellar.expert/explorer/public/tx/8dd796004f27771730ab3a72be1d41588669445ef80065ef7730f3c219f13231) |
| Allow recipient test-a | `eea2c87908f2bc4f0a9ca771bccb82616fb713516b0e55a4dfa50c91d7a6eed5` | [view](https://stellar.expert/explorer/public/tx/eea2c87908f2bc4f0a9ca771bccb82616fb713516b0e55a4dfa50c91d7a6eed5) |
| Allow recipient test-b | `03eb865337364971cd74f5f8bb13d6487708f8f34bdc26c630a478ad426fb9f9` | [view](https://stellar.expert/explorer/public/tx/03eb865337364971cd74f5f8bb13d6487708f8f34bdc26c630a478ad426fb9f9) |
| Allow recipient test-c | `cb4fda39e0955251605eee07a97f9c647a43e277ccf8b8e4f98aaae68065ac77` | [view](https://stellar.expert/explorer/public/tx/cb4fda39e0955251605eee07a97f9c647a43e277ccf8b8e4f98aaae68065ac77) |

**What this is:** every asset/operation/recipient in this system is
disabled by default (fail-closed). These seven calls are what actually
made the treasury able to pay anyone anything — without them, every
payment attempt below would have failed with `AssetNotAllowed` /
`OperationNotAllowed` / `RecipientNotAllowed`.

## 6. Signer/payment transactions (all via `smart_account`'s `AuthPayload`, signed by `mainnet-deployer` as the sole rule-0 signer unless noted)

| # | Action | Detail | Tx hash | Explorer |
|---|---|---|---|---|
| 1 | `execute_transfer_payment` | 10 XLM → test-a | `d6b0d14f7ec39684eeb581369612cd5dd8c20caa9bbd4da90b2ac5a744fae63a` | [view](https://stellar.expert/explorer/public/tx/d6b0d14f7ec39684eeb581369612cd5dd8c20caa9bbd4da90b2ac5a744fae63a) |
| 2 | `change_trust` (classic, test-a's own signature) | test-a opens a USDC trustline | `c4e2b9ceea42e81ddfe732e839afa1448330ddf08f025ae173ccbb20a28d1cd3` | [view](https://stellar.expert/explorer/public/tx/c4e2b9ceea42e81ddfe732e839afa1448330ddf08f025ae173ccbb20a28d1cd3) |
| 3 | `execute_transfer_payment` | 0.2 USDC → test-a | `b4edb16d43e24d8145ab3e86e9392aa7e4920e302916a00aee222c6fbbb13318` | [view](https://stellar.expert/explorer/public/tx/b4edb16d43e24d8145ab3e86e9392aa7e4920e302916a00aee222c6fbbb13318) |
| 4 | `execute_split_payment` | 1 XLM → test-a, 1 XLM → test-b (one call, two recipients) | `fafaa290a3e524700444bf3b4822c38f59283ad5ec98d90435f1b09265484a5b` | [view](https://stellar.expert/explorer/public/tx/fafaa290a3e524700444bf3b4822c38f59283ad5ec98d90435f1b09265484a5b) |
| 5 | `create_scheduled_payment` | 0.5 XLM → test-c, approved for later execution | `d9f7e133acacb79163d30440cfc7ad6b7116199d97bb854d24e949fe1c7bc2a7` | [view](https://stellar.expert/explorer/public/tx/d9f7e133acacb79163d30440cfc7ad6b7116199d97bb854d24e949fe1c7bc2a7) |
| 6 | `execute_scheduled_payment` | executed by `mainnet-deployer` acting as the configured `Executor` (relayer role) | `6b07a91b550e63125f1c1857d40e38620eee68d564a7586bc684293890349092` | [view](https://stellar.expert/explorer/public/tx/6b07a91b550e63125f1c1857d40e38620eee68d564a7586bc684293890349092) |
| 7 | `create_scheduled_payment` | a second one, created specifically to be cancelled next | `4e10d9a382d26c7d795735a897cbee6ed7a3f43f68c2b9f8a32ae7d317d97253` | [view](https://stellar.expert/explorer/public/tx/4e10d9a382d26c7d795735a897cbee6ed7a3f43f68c2b9f8a32ae7d317d97253) |
| 8 | `cancel_scheduled_payment` | cancels #7 before its window opens | `0b11a0a1787278ba124453b4a49758b8c5bd56b2450917b14e771a8de616aaea` | [view](https://stellar.expert/explorer/public/tx/0b11a0a1787278ba124453b4a49758b8c5bd56b2450917b14e771a8de616aaea) |
| 9 | `pause` | owner-gated (plain classic auth, not `AuthPayload`) | `17337837d454b1b20c00d2a2ad46ccef41d9ff25dd50b9d4cca3c406fb694594` | [view](https://stellar.expert/explorer/public/tx/17337837d454b1b20c00d2a2ad46ccef41d9ff25dd50b9d4cca3c406fb694594) |
| 10 | `unpause` | same, reverses #9 | `a9e26aab1140b2d9c64d0647a74ac284f00a451654fd14267318e12a6886c5c7` | [view](https://stellar.expert/explorer/public/tx/a9e26aab1140b2d9c64d0647a74ac284f00a451654fd14267318e12a6886c5c7) |
| 11 | `extend_instance_ttl` | permissionless maintenance, no auth needed at all | `9eca30c2293b166cbfc66beb75eb9e0ab2a42d7d1e7a7ecc9137f1e4e7dcfafe` | [view](https://stellar.expert/explorer/public/tx/9eca30c2293b166cbfc66beb75eb9e0ab2a42d7d1e7a7ecc9137f1e4e7dcfafe) |
| 12 | `add_signer` | added test-c as a **second** signer on context rule 0 | `00985864a0b221591627e9b688a698d264efe47fe9333e62fe194533f5d4a358` | [view](https://stellar.expert/explorer/public/tx/00985864a0b221591627e9b688a698d264efe47fe9333e62fe194533f5d4a358) |
| 13 | `add_context_rule` — **first attempt failed, `Error(Contract, #3002)`** | see explanation below | *(never landed — simulation-only rejection)* | n/a |
| 14 | `remove_signer` | removed test-c again, **co-signed by both `mainnet-deployer` and test-c** | `505ad3b25663101013a150c5f8e7d7c30cd961c24b8db68fa9466a6f9c941b26` | [view](https://stellar.expert/explorer/public/tx/505ad3b25663101013a150c5f8e7d7c30cd961c24b8db68fa9466a6f9c941b26) |
| 15 | `add_context_rule` — **retried, succeeded** | new context rule (id `1`), scoped `CallContract(transfer_adapter)`, name `"test-rule"`, signer test-a, no policies — `get_context_rules_count` confirmed `2` afterward | `3b6844d4105ae18e8a54586858c7cb243341f18c99dc247af6489acdec757de9` | [view](https://stellar.expert/explorer/public/tx/3b6844d4105ae18e8a54586858c7cb243341f18c99dc247af6489acdec757de9) |

### Why #13 failed — a real, live demonstration of a known design property

Context rule `0` had **no policy attached**. Per `smart_account`'s
composed OZ logic, a policy-less rule requires **unanimous** agreement from
every registered signer. The moment `add_signer` (#12) added test-c as a
*second* signer to that same rule, rule 0 silently flipped from
"any one signer suffices" to "both signers must co-sign" — for *every*
action gated by that rule, including the administrative calls
(`add_context_rule`, `add_signer`, etc.) that aren't protected by `owner`
at all (finding #4, `docs/SECURITY_REVIEW_STRICT.md`). The very next call
(#13), signed only by `mainnet-deployer`, was rejected with
`Error(Contract, #3002)` (`UnvalidatedContext`) — a live example of the
"weakest rule wins" / lockout dynamic already documented for the dApp
developer in `docs/DAPP_DEVELOPER_QA.md`. Recovered by building a
two-signer `AuthPayload` (both `mainnet-deployer` and test-c signing) to
call `remove_signer` (#14), restoring rule 0 to single-signer control.

## 7. Not exercised on mainnet (explicitly, not an oversight)

- **`governance_account`**: not deployed for this beta (see
  `docs/MAINNET_DEPLOYMENT.md` §8).
- **Recovery flow** (`request_guardian_freeze`, `apply_guardian_freeze`,
  recovery proposal/approval/`finalize_recovery`/`apply_recovery`) and
  **adapter reconfiguration** (`propose_adapter_change`/
  `apply_adapter_change`): both carry real ~24h (`17280`-ledger) timelocks
  on this deployment, matching mainnet production values exactly (not
  shortened, unlike some earlier testnet-only experiments that never
  touched committed source). A same-session, fully completed proof of
  either flow isn't possible on mainnet in one sitting; these entrypoints
  are wired and were exercised end-to-end on testnet
  (`docs/TESTNET_FACTORY_DEPLOYMENT.md` §14), just not re-run here.

## 8. Total real cost of this session's mainnet work

Deployer funded with 250 XLM (on top of a small pre-existing balance).
**104.43 XLM** (exact, verified) spent on the six WASM uploads plus the
factory itself (§1), and well under **1 XLM total** for every other
transaction combined
(factory deploy, treasury creation, policy configuration, funding, and all
fourteen payment/admin transactions in §6) — concretely illustrating the
cost asymmetry explained in chat: uploading new contract *code* is
expensive right now on mainnet; everything that only creates instances or
writes ordinary state is cheap, exactly as Stellar's "low cost" reputation
would suggest.
