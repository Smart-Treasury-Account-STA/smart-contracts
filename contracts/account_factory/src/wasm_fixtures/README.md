Real, optimized WASM builds of the six sibling contracts `deploy_account`
deploys, committed as test fixtures so `cargo test -p sta-account-factory`
exercises the actual `env.deployer().deploy_v2` path against real bytecode.

**These go stale.** If you change `policy_engine`, `intent_registry`,
`recovery_manager`, `transfer_adapter`, `split_adapter`, or `smart_account`
and don't regenerate these, `sta-account-factory`'s tests keep passing
against the *old* behavior of whichever contract you changed — a false
green, not a real one. Regenerate after any change to those six crates:

```bash
stellar contract build --optimize --out-dir /tmp/sta_wasm_fixtures
cp /tmp/sta_wasm_fixtures/{sta_policy_engine,sta_intent_registry,sta_recovery_manager,sta_transfer_adapter,sta_split_adapter,sta_smart_account}.wasm \
  contracts/account_factory/src/wasm_fixtures/
```
