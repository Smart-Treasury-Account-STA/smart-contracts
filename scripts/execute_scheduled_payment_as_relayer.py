#!/usr/bin/env python3
"""Executes `smart_account.execute_scheduled_payment` as the configured
relayer identity, on Stellar testnet.

`execute_scheduled_payment` itself has no `require_auth()` gate on
`smart_account` (it is deliberately permissionless -- see that function's
doc comment in `contracts/smart_account/src/lib.rs`: "the signer approval
already happened at `create_scheduled_payment` time"). But that does NOT
mean the whole call graph needs no authorization at execution time.
Building this script empirically surfaced two real, independent
authorization points, not one:

1. `intent_registry::ensure_executor`'s `executor.require_auth()`, two
   levels deep (`execute_scheduled_payment -> intent_registry
   .mark_child_executed -> ensure_executor`). Soroban's `SourceAccount`
   credentials (what the bare `stellar contract invoke --source <id>` CLI
   auto-generates) only cover `require_auth()` calls made at the ROOT of
   the invocation tree; a non-root call -- even by a plain Ed25519
   account, not a custom account -- needs an explicit
   `SorobanCredentialsType.SOROBAN_CREDENTIALS_ADDRESS` entry with its own
   real signature over that exact sub-invocation (confirmed empirically:
   the bare CLI rejects this call with `Error(Auth, InvalidAction)` /
   "encountered authorization not tied to the root contract invocation for
   an address").
2. The Stellar Asset Contract's own `transfer(from, to, amount)`, which
   internally calls `from.require_auth()` where `from = smart_account`.
   `smart_account` is never the *direct* caller of the SAC here
   (`transfer_adapter` is -- `smart_account` only calls `transfer_adapter`,
   two levels above the SAC), so Soroban's invoker-contract shortcut never
   applies to it, exactly the same empirical finding
   `execute_demo_transfer_payment.py`'s docstring already documents for
   the interactive transfer path. Because `execute_scheduled_payment` has
   no top-level `smart_account.require_auth()` call to root this entry
   under (unlike the interactive path), this entry's `root_invocation` is
   the SAC `transfer` call itself -- and it still needs the full custom
   `AuthPayload`/`do_check_auth` treatment (context rule + a real signer
   signature covering the nested `__check_auth` call), because
   `smart_account` is a custom account and this is a genuine, freestanding
   `require_auth()` on it, not merely a formality. In short: an "already
   approved" scheduled payment's *relayer* dispatch needs no signer key,
   but the token movement it triggers still does, at execution time -- the
   signer's role at `create_scheduled_payment` time approves the
   schedule's terms, not the individual on-chain movement of funds.
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


def classic_address_entry(
    kp: Keypair, invocation: stellar_xdr.SorobanAuthorizedInvocation, expiration_ledger: int
) -> tuple[stellar_xdr.SorobanAuthorizationEntry, int]:
    nonce = random.getrandbits(62)
    sig_payload = signature_payload(invocation, nonce, expiration_ledger)
    raw_sig = kp.sign(sig_payload)
    classic_sig_scval = scval.to_vec(
        [
            scval.to_map(
                {
                    scval.to_symbol("public_key"): scval.to_bytes(kp.raw_public_key()),
                    scval.to_symbol("signature"): scval.to_bytes(raw_sig),
                }
            )
        ]
    )
    credentials = stellar_xdr.SorobanCredentials(
        type=stellar_xdr.SorobanCredentialsType.SOROBAN_CREDENTIALS_ADDRESS,
        address=stellar_xdr.SorobanAddressCredentials(
            address=Address(kp.public_key).to_xdr_sc_address(),
            nonce=stellar_xdr.Int64(nonce),
            signature_expiration_ledger=stellar_xdr.Uint32(expiration_ledger),
            signature=classic_sig_scval,
        ),
    )
    return stellar_xdr.SorobanAuthorizationEntry(credentials=credentials, root_invocation=invocation), nonce


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smart-account", required=True)
    parser.add_argument("--intent-registry", required=True)
    parser.add_argument("--intent-id-hex", required=True)
    parser.add_argument("--child-sequence", type=int, required=True)
    parser.add_argument("--relayer-identity", default="sta-testnet-relayer")
    parser.add_argument("--asset", required=True, help="SAC contract ID pinned on the intent")
    parser.add_argument("--destination", required=True, help="Destination pinned on the intent")
    parser.add_argument("--amount", type=int, required=True, help="Amount pinned on the intent")
    parser.add_argument("--owner-identity", default="sta-testnet-factory-owner")
    parser.add_argument("--context-rule-id", type=int, default=0)
    args = parser.parse_args()

    relayer_secret = stellar_secret(args.relayer_identity)
    relayer_kp = Keypair.from_secret(relayer_secret)
    owner_secret = stellar_secret(args.owner_identity)
    owner_kp = Keypair.from_secret(owner_secret)

    server = SorobanServer(RPC_URL)
    latest_ledger = server.get_latest_ledger().sequence
    expiration_ledger = latest_ledger + 100

    intent_id = bytes.fromhex(args.intent_id_hex)
    call_args = [scval.to_bytes(intent_id), scval.to_uint32(args.child_sequence)]

    # Entry 1: relayer authorizes intent_registry.mark_child_executed
    # (rooted there directly -- no smart_account require_auth() gate
    # exists above it to nest under).
    mark_executed_invocation = contract_fn_invocation(args.intent_registry, "mark_child_executed", call_args)
    entry1, _ = classic_address_entry(relayer_kp, mark_executed_invocation, expiration_ledger)

    # Entry 2: smart_account's custom AuthPayload, rooted at the SAC's own
    # transfer(from=smart_account, ...) call -- the only place
    # smart_account.require_auth() is actually invoked in this call graph
    # (see module docstring point 2).
    sac_invocation = contract_fn_invocation(
        args.asset,
        "transfer",
        [scval.to_address(args.smart_account), scval.to_address(args.destination), scval.to_int128(args.amount)],
    )
    nonce2 = random.getrandbits(62)
    sig_payload2 = signature_payload(sac_invocation, nonce2, expiration_ledger)
    context_rule_ids_scval = scval.to_vec([scval.to_uint32(args.context_rule_id)])
    auth_digest = hashlib.sha256(sig_payload2 + context_rule_ids_scval.to_xdr_bytes()).digest()

    signer_scval = scval.to_enum("Delegated", scval.to_address(owner_kp.public_key))
    signers_map_scval = stellar_xdr.SCVal(
        type=stellar_xdr.SCValType.SCV_MAP,
        map=stellar_xdr.SCMap(
            sc_map=[stellar_xdr.SCMapEntry(key=signer_scval, val=scval.to_bytes(b""))]
        ),
    )
    auth_payload_scval = scval.to_struct(
        {"signers": signers_map_scval, "context_rule_ids": context_rule_ids_scval}
    )
    credentials2 = stellar_xdr.SorobanCredentials(
        type=stellar_xdr.SorobanCredentialsType.SOROBAN_CREDENTIALS_ADDRESS,
        address=stellar_xdr.SorobanAddressCredentials(
            address=Address(args.smart_account).to_xdr_sc_address(),
            nonce=stellar_xdr.Int64(nonce2),
            signature_expiration_ledger=stellar_xdr.Uint32(expiration_ledger),
            signature=auth_payload_scval,
        ),
    )
    entry2 = stellar_xdr.SorobanAuthorizationEntry(credentials=credentials2, root_invocation=sac_invocation)

    # Entry 3: owner's classic signature over the nested
    # smart_account.__check_auth(auth_digest) call authenticate() makes.
    check_auth_invocation = contract_fn_invocation(
        args.smart_account, "__check_auth", [scval.to_bytes(auth_digest)]
    )
    entry3, _ = classic_address_entry(owner_kp, check_auth_invocation, expiration_ledger)

    auth_entries = [entry1, entry2, entry3]

    source_account = server.load_account(relayer_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=args.smart_account,
            function_name="execute_scheduled_payment",
            parameters=call_args,
            auth=auth_entries,
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
