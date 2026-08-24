#!/usr/bin/env python3
"""Generalized signer-authorized `smart_account` call driver, for
demonstrating the factory-deployed treasury's four payment-authoring
entrypoints on Stellar testnet: `execute_transfer_payment`,
`execute_split_payment`, `create_scheduled_payment`, and
`cancel_scheduled_payment`.

Same custom-account authorization mechanics as
`bootstrap_intent_registry.py`/`execute_demo_transfer_payment.py` (see
those scripts' docstrings for the full `AuthPayload`/`do_check_auth`/
`authenticate` explanation), generalized here to one script covering all
four `smart_account` entrypoints that gate on
`env.current_contract_address().require_auth()`, rather than duplicating
the ~150 lines of auth-entry plumbing per call shape. All four need only
the root entry (`smart_account`'s own `AuthPayload` for the top-level
call), with an empty `sub_invocations` list -- no declared SAC node at
all: `transfer_adapter`/`split_adapter` now draw funds via `transfer_from`
(the SAC's `spender` authorization, satisfied by *their own*
invoker-shortcut, not `smart_account`'s) after `smart_account` `approve`s
them for the exact amount immediately beforehand (also invoker-shortcut,
since `smart_account` is the SAC's direct caller for `approve`) -- see
`docs/SECURITY_REVIEW_STRICT.md` finding 29. Earlier revisions of this
script declared one `SAC.transfer(from=smart_account, ...)` sub-invocation
per moved balance for `transfer`/`split`, back when the adapters called
the SAC's plain `transfer` directly; that node no longer exists in the
real call graph.
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


def contract_fn_invocation(
    contract_id: str, function_name: str, args: list, sub_invocations: list | None = None
) -> stellar_xdr.SorobanAuthorizedInvocation:
    function = stellar_xdr.SorobanAuthorizedFunction(
        type=stellar_xdr.SorobanAuthorizedFunctionType.SOROBAN_AUTHORIZED_FUNCTION_TYPE_CONTRACT_FN,
        contract_fn=stellar_xdr.InvokeContractArgs(
            contract_address=Address(contract_id).to_xdr_sc_address(),
            function_name=stellar_xdr.SCSymbol(function_name.encode()),
            args=args,
        ),
    )
    return stellar_xdr.SorobanAuthorizedInvocation(
        function=function, sub_invocations=sub_invocations or []
    )


def signature_payload(
    invocation: stellar_xdr.SorobanAuthorizedInvocation, nonce: int, expiration_ledger: int
) -> bytes:
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


def address_credentials_entry(
    address: str,
    nonce: int,
    expiration_ledger: int,
    invocation: stellar_xdr.SorobanAuthorizedInvocation,
    signature_scval: stellar_xdr.SCVal,
) -> stellar_xdr.SorobanAuthorizationEntry:
    credentials = stellar_xdr.SorobanCredentials(
        type=stellar_xdr.SorobanCredentialsType.SOROBAN_CREDENTIALS_ADDRESS,
        address=stellar_xdr.SorobanAddressCredentials(
            address=Address(address).to_xdr_sc_address(),
            nonce=stellar_xdr.Int64(nonce),
            signature_expiration_ledger=stellar_xdr.Uint32(expiration_ledger),
            signature=signature_scval,
        ),
    )
    return stellar_xdr.SorobanAuthorizationEntry(credentials=credentials, root_invocation=invocation)


def build_auth_entries(
    owner_kp: Keypair,
    smart_account: str,
    function_name: str,
    call_args: list,
    sac_sub_invocations: list[stellar_xdr.SorobanAuthorizedInvocation],
    expiration_ledger: int,
    context_rule_id: int = 0,
) -> list[stellar_xdr.SorobanAuthorizationEntry]:
    owner_addr = owner_kp.public_key

    root_invocation = contract_fn_invocation(
        smart_account, function_name, call_args, sub_invocations=sac_sub_invocations
    )
    nonce1 = random.getrandbits(62)
    sig_payload = signature_payload(root_invocation, nonce1, expiration_ledger)

    # One context-rule-id entry per auth context that reaches __check_auth:
    # the root call, plus one per declared SAC sub-invocation (the adapter's
    # own require_auth() is always invoker-shortcut-satisfied and never
    # becomes a context) -- all validated against the same context rule.
    num_contexts = 1 + len(sac_sub_invocations)
    context_rule_ids_scval = scval.to_vec(
        [scval.to_uint32(context_rule_id) for _ in range(num_contexts)]
    )
    context_rule_ids_xdr = context_rule_ids_scval.to_xdr_bytes()
    auth_digest = hashlib.sha256(sig_payload + context_rule_ids_xdr).digest()

    signer_scval = scval.to_enum("Delegated", scval.to_address(owner_addr))
    signers_map_scval = stellar_xdr.SCVal(
        type=stellar_xdr.SCValType.SCV_MAP,
        map=stellar_xdr.SCMap(
            sc_map=[stellar_xdr.SCMapEntry(key=signer_scval, val=scval.to_bytes(b""))]
        ),
    )
    auth_payload_scval = scval.to_struct(
        {"signers": signers_map_scval, "context_rule_ids": context_rule_ids_scval}
    )
    entry1 = address_credentials_entry(
        smart_account, nonce1, expiration_ledger, root_invocation, auth_payload_scval
    )

    nested_invocation = contract_fn_invocation(
        smart_account, "__check_auth", [scval.to_bytes(auth_digest)]
    )
    nonce2 = random.getrandbits(62)
    sig_payload2 = signature_payload(nested_invocation, nonce2, expiration_ledger)
    raw_sig = owner_kp.sign(sig_payload2)

    classic_sig_scval = scval.to_vec(
        [
            scval.to_map(
                {
                    scval.to_symbol("public_key"): scval.to_bytes(owner_kp.raw_public_key()),
                    scval.to_symbol("signature"): scval.to_bytes(raw_sig),
                }
            )
        ]
    )
    entry2 = address_credentials_entry(
        owner_addr, nonce2, expiration_ledger, nested_invocation, classic_sig_scval
    )

    return [entry1, entry2]


def scheduled_intent_args_scval(
    intent_id: bytes,
    asset: str,
    destination: str,
    amount: int,
    start_ledger: int,
    end_ledger: int,
    interval_ledgers: int,
    max_executions: int,
    placeholder_adapter: str,
):
    return scval.to_struct(
        {
            "intent_id": scval.to_bytes(intent_id),
            "asset": scval.to_address(asset),
            "destination": scval.to_address(destination),
            "amount": scval.to_int128(amount),
            "start_ledger": scval.to_uint32(start_ledger),
            "end_ledger": scval.to_uint32(end_ledger),
            "interval_ledgers": scval.to_uint32(interval_ledgers),
            "max_executions": scval.to_uint32(max_executions),
            # Caller-supplied execution_count/policy_version/adapter/
            # cancelled are ignored/overwritten server-side (see
            # `create_scheduled_payment`'s doc comment) -- placeholders.
            "execution_count": scval.to_uint32(0),
            "policy_version": scval.to_uint32(0),
            "adapter": scval.to_address(placeholder_adapter),
            "cancelled": scval.to_bool(False),
        }
    )


def submit(server: SorobanServer, owner_kp: Keypair, smart_account: str, function_name: str, call_args: list, auth_entries: list) -> int:
    source_account = server.load_account(owner_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=smart_account,
            function_name=function_name,
            parameters=call_args,
            auth=auth_entries,
        )
        .build()
    )
    try:
        prepared = server.prepare_transaction(tx)
    except PrepareTransactionException as exc:
        print("Simulation failed -- auth entries were rejected:", file=sys.stderr)
        print(exc.simulate_transaction_response, file=sys.stderr)
        return 1

    prepared.sign(owner_kp)
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
            print(f"{function_name} succeeded. tx =", send_resp.hash)
            return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--call", required=True, choices=["transfer", "split", "create_scheduled", "cancel_scheduled"])
    parser.add_argument("--smart-account", required=True)
    parser.add_argument("--owner-identity", default="sta-testnet-factory-owner")
    parser.add_argument("--context-rule-id", type=int, default=0)

    # transfer / split / create_scheduled
    parser.add_argument("--asset", help="SAC contract ID")
    parser.add_argument("--destination", help="Single recipient (transfer / create_scheduled)")
    parser.add_argument("--amount", type=int, help="Single amount (transfer / create_scheduled)")
    parser.add_argument("--recipients", help="Comma-separated recipients (split)")
    parser.add_argument("--amounts", help="Comma-separated amounts (split)")
    parser.add_argument("--nonce", type=int, help="transfer / split")
    parser.add_argument("--expected-policy-version", type=int, help="transfer / split")

    # create_scheduled / cancel_scheduled
    parser.add_argument("--intent-id-hex", help="64 hex chars (32 bytes)")
    parser.add_argument("--start-ledger", type=int)
    parser.add_argument("--end-ledger", type=int)
    parser.add_argument("--interval-ledgers", type=int, default=0)
    parser.add_argument("--max-executions", type=int, default=1)
    parser.add_argument("--placeholder-adapter", help="Any valid contract address; ignored server-side")

    args = parser.parse_args()

    owner_secret = stellar_secret(args.owner_identity)
    owner_kp = Keypair.from_secret(owner_secret)

    server = SorobanServer(RPC_URL)
    latest_ledger = server.get_latest_ledger().sequence
    expiration_ledger = latest_ledger + 100

    if args.call == "transfer":
        call_args = [
            scval.to_address(args.asset),
            scval.to_address(args.destination),
            scval.to_int128(args.amount),
            scval.to_uint64(args.nonce),
            scval.to_uint32(args.expected_policy_version),
        ]
        sub_invocations = []
        function_name = "execute_transfer_payment"

    elif args.call == "split":
        recipients = args.recipients.split(",")
        amounts = [int(a) for a in args.amounts.split(",")]
        call_args = [
            scval.to_address(args.asset),
            scval.to_vec([scval.to_address(r) for r in recipients]),
            scval.to_vec([scval.to_int128(a) for a in amounts]),
            scval.to_uint64(args.nonce),
            scval.to_uint32(args.expected_policy_version),
        ]
        sub_invocations = []
        function_name = "execute_split_payment"

    elif args.call == "create_scheduled":
        intent_id = bytes.fromhex(args.intent_id_hex)
        intent_scval = scheduled_intent_args_scval(
            intent_id,
            args.asset,
            args.destination,
            args.amount,
            args.start_ledger,
            args.end_ledger,
            args.interval_ledgers,
            args.max_executions,
            args.placeholder_adapter,
        )
        call_args = [intent_scval]
        sub_invocations = []
        function_name = "create_scheduled_payment"

    else:  # cancel_scheduled
        intent_id = bytes.fromhex(args.intent_id_hex)
        call_args = [scval.to_bytes(intent_id)]
        sub_invocations = []
        function_name = "cancel_scheduled_payment"

    auth_entries = build_auth_entries(
        owner_kp, args.smart_account, function_name, call_args, sub_invocations, expiration_ledger, args.context_rule_id
    )

    return submit(server, owner_kp, args.smart_account, function_name, call_args, auth_entries)


if __name__ == "__main__":
    sys.exit(main())
