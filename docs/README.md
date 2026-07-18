# Documentation Index

This folder documents the current **V1 implementation** in `contracts/`. Everything listed below is current and reviewable against today's source. Historical material from the earlier, partial PoC revision lives separately in `archive/` and is **not** part of what to review — see `archive/README.md`.

## Start here, in order

1. **[`V1_SCOPE.md`](V1_SCOPE.md)** — what's implemented, what's integrated from OpenZeppelin's Stellar contracts rather than built, what's genuinely new, and what's explicitly deferred. Read this first: it's the authoritative statement of scope and directly answers "what am I actually reviewing here."
2. **[`V1_REVIEW_GUIDE.md`](V1_REVIEW_GUIDE.md)** — how to walk the code, which files matter, verification commands (`cargo test`, `cargo llvm-cov`, `stellar contract build`, `scripts/deploy_testnet.sh`), and a full per-file test breakdown.
3. **[`SMART_CONTRACT_SPECIFICATION.md`](SMART_CONTRACT_SPECIFICATION.md)** — per-module contract design and responsibilities, with a mapping table showing which `contracts/*` package implements which module.
4. **[`TECHNICAL_ARCHITECTURE.md`](TECHNICAL_ARCHITECTURE.md)** — the full target-state Smart Treasury Account architecture (dApp, SDK, relayer, and all onchain modules). This document describes the complete product, not just this repository's scope — §1.2 states exactly which pieces this repository implements today.
5. **[`TESTNET_DEPLOYMENT.md`](TESTNET_DEPLOYMENT.md)** — live V1 deployment record on Stellar testnet: real contract addresses, transactions, and one explicitly named limitation on what a bare CLI deployment can and can't exercise.

## Not part of the current review

- **[`archive/`](archive/)** — the earlier partial PoC's testnet deployment record, kept only for historical traceability. Different contract names, different `soroban-sdk` version, no OpenZeppelin composition. If you're reviewing V1, you can skip this folder entirely.

## Quick reference

| Question | Where to look |
|---|---|
| "What did the security review find?" | `V1_SCOPE.md` §4 and §5 |
| "How do I run the tests and check coverage?" | `V1_REVIEW_GUIDE.md` → Verification Commands |
| "Which contract implements module X?" | `SMART_CONTRACT_SPECIFICATION.md` §1.1 |
| "Is this deployed anywhere I can check independently?" | `TESTNET_DEPLOYMENT.md` |
| "What's deliberately not built yet, and why?" | `V1_SCOPE.md` → Not Yet Included in V1 |
