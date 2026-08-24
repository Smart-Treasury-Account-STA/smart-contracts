#!/usr/bin/env python3
"""Executes `smart_account.execute_scheduled_payment` as the configured
relayer identity, on Stellar testnet.

`execute_scheduled_payment` itself has no `require_auth()` gate on
`smart_account` (it is deliberately permissionless -- see that function's
doc comment in `contracts/smart_account/src/lib.rs`: "the signer approval
already happened at `create_scheduled_payment` time"). The only real
authorization anywhere in this call graph is
`intent_registry::ensure_executor`'s `executor.require_auth()`, two levels
deep (`execute_scheduled_payment -> intent_registry.mark_child_executed ->
ensure_executor`). Soroban's `SourceAccount` credentials (what the bare
`stellar contract invoke --source <id>` CLI auto-generates) only cover
`require_auth()` calls made at the ROOT of the invocation tree; a non-root
call -- even by a plain Ed25519 account, not a custom account -- needs an
explicit `SorobanCredentialsType.SOROBAN_CREDENTIALS_ADDRESS` entry with
its own real signature over that exact sub-invocation (confirmed
empirically: the bare CLI rejects this call with `Error(Auth,
InvalidAction)` / "encountered authorization not tied to the root contract
invocation for an address"). That is the one entry this script builds.

Earlier revisions of this script also had to build a second and third
entry for `smart_account`'s own custom `AuthPayload`, because the adapter
used to call the SAC's plain `transfer(from=smart_account, ...)`, which
needs `smart_account.require_auth()` again -- and since
`execute_scheduled_payment` never opens a root authorization session for
`smart_account`, that second authentication attempt was a genuine Soroban
contract-reentrancy violation (`smart_account`'s own frame was already
active on the stack). See `docs/SECURITY_REVIEW_STRICT.md` finding 29.
Fixed by having `smart_account` `approve` the adapter for the exact amount
immediately before delegating to it (invoker-shortcut, no `__check_auth`
call at all) and having the adapter draw via `transfer_from` instead of
`transfer` (also invoker-shortcut, this time on the adapter's own
address). Neither call needs an authorization entry any more, so this
script only ever needs the relayer's own entry below.
"""
from __future__ import annotations

import argparse
import hashlib
import random
import subprocess
import sys
import time

from stellar_sdk import Address, Keypair, Network, SorobanServer, TransactionBuilder, scval
from stellar_sdk import xdr as stellar_xdr
from stellar_sdk.exceptions import PrepareTransactionException

RPC_URL = "https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE = Network.TESTNET_NETWORK_PASSPHRASE
BASE_FEE = 100_000


def stellar_secret(identity: str) -> str:
    out = subprocess.run(
        ["stellar", "keys", "show", identity], capture_output=True, text=True, check=True
    )
    return out.stdout.strip()


def contract_fn_invocation(contract_id: str, function_name: str, args: list) -> stellar_xdr.SorobanAuthorizedInvocation:
    function = stellar_xdr.SorobanAuthorizedFunction(
        type=stellar_xdr.SorobanAuthorizedFunctionType.SOROBAN_AUTHORIZED_FUNCTION_TYPE_CONTRACT_FN,
        contract_fn=stellar_xdr.InvokeContractArgs(
            contract_address=Address(contract_id).to_xdr_sc_address(),
            function_name=stellar_xdr.SCSymbol(function_name.encode()),
            args=args,
        ),
    )
    return stellar_xdr.SorobanAuthorizedInvocation(function=function, sub_invocations=[])


def signature_payload(invocation: stellar_xdr.SorobanAuthorizedInvocation, nonce: int, expiration_ledger: int) -> bytes:
    network_id = stellar_xdr.Hash(Network(NETWORK_PASSPHRASE).network_id())
    preimage = stellar_xdr.HashIDPreimage(
        type=stellar_xdr.EnvelopeType.ENVELOPE_TYPE_SOROBAN_AUTHORIZATION,
        soroban_authorization=stellar_xdr.HashIDPreimageSorobanAuthorization(
            network_id=network_id,
            nonce=stellar_xdr.Int64(nonce),
            signature_expiration_ledger=stellar_xdr.Uint32(expiration_ledger),
            invocation=invocation,
        ),
    )
    return hashlib.sha256(preimage.to_xdr_bytes()).digest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smart-account", required=True)
    parser.add_argument("--intent-registry", required=True)
    parser.add_argument("--intent-id-hex", required=True)
    parser.add_argument("--child-sequence", type=int, required=True)
    parser.add_argument("--relayer-identity", default="sta-testnet-relayer")
    args = parser.parse_args()

    relayer_secret = stellar_secret(args.relayer_identity)
    relayer_kp = Keypair.from_secret(relayer_secret)

    server = SorobanServer(RPC_URL)
    latest_ledger = server.get_latest_ledger().sequence
    expiration_ledger = latest_ledger + 100

    intent_id = bytes.fromhex(args.intent_id_hex)
    call_args = [scval.to_bytes(intent_id), scval.to_uint32(args.child_sequence)]

    nested_invocation = contract_fn_invocation(args.intent_registry, "mark_child_executed", call_args)
    nonce = random.getrandbits(62)
    sig_payload = signature_payload(nested_invocation, nonce, expiration_ledger)
    raw_sig = relayer_kp.sign(sig_payload)

    classic_sig_scval = scval.to_vec(
        [
            scval.to_map(
                {
                    scval.to_symbol("public_key"): scval.to_bytes(relayer_kp.raw_public_key()),
                    scval.to_symbol("signature"): scval.to_bytes(raw_sig),
                }
            )
        ]
    )
    credentials = stellar_xdr.SorobanCredentials(
        type=stellar_xdr.SorobanCredentialsType.SOROBAN_CREDENTIALS_ADDRESS,
        address=stellar_xdr.SorobanAddressCredentials(
            address=Address(relayer_kp.public_key).to_xdr_sc_address(),
            nonce=stellar_xdr.Int64(nonce),
            signature_expiration_ledger=stellar_xdr.Uint32(expiration_ledger),
            signature=classic_sig_scval,
        ),
    )
    auth_entry = stellar_xdr.SorobanAuthorizationEntry(credentials=credentials, root_invocation=nested_invocation)

    source_account = server.load_account(relayer_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=args.smart_account,
            function_name="execute_scheduled_payment",
            parameters=call_args,
            auth=[auth_entry],
        )
        .build()
    )

    try:
        prepared = server.prepare_transaction(tx)
    except PrepareTransactionException as exc:
        print("Simulation failed:", file=sys.stderr)
        print(exc.simulate_transaction_response, file=sys.stderr)
        return 1

    prepared.sign(relayer_kp)
    send_resp = server.send_transaction(prepared)
    print("submitted:", send_resp.hash, send_resp.status)

    while True:
        time.sleep(3)
        result = server.get_transaction(send_resp.hash)
        if result.status.name != "NOT_FOUND":
            print("status:", result.status)
            if result.status.name != "SUCCESS":
                print(result)
                return 1
            print("execute_scheduled_payment succeeded. tx =", send_resp.hash)
            return 0


if __name__ == "__main__":
    sys.exit(main())
