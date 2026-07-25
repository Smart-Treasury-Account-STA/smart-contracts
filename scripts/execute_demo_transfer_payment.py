#!/usr/bin/env python3
"""One-off demonstration: executes a real, signer-authorized
`smart_account.execute_transfer_payment` on Stellar testnet.

Same custom-account authorization mechanics as
`bootstrap_intent_registry.py` (see that script's docstring for the full
explanation of `AuthPayload`/`do_check_auth`/`authenticate`), applied here
to prove the actual payment path end to end on the live deployment: a real
Signer::Delegated wallet (the deployer) authorizes a real
`execute_transfer_payment` call, which passes through `policy_engine`'s
live policy checks and `transfer_adapter`'s real Stellar Asset Contract
transfer, moving real (testnet) STA balance out of the treasury.
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
        ["stellar", "keys", "show", identity],
        capture_output=True,
        text=True,
        check=True,
    )
    return out.stdout.strip()


def contract_fn_invocation(
    contract_id: str,
    function_name: str,
    args: list,
    sub_invocations: list | None = None,
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
    deployer_kp: Keypair,
    smart_account: str,
    call_args: list,
    asset: str,
    destination: str,
    amount: int,
    expiration_ledger: int,
) -> list[stellar_xdr.SorobanAuthorizationEntry]:
    deployer_addr = deployer_kp.public_key

    # smart_account.require_auth() is called THREE times in this call
    # chain, but only TWO of them need a declared tree node:
    #   1. execute_transfer_payment's own top-level require_auth() -- what
    #      smart_account's __check_auth is actually invoked against.
    #   2. transfer_adapter::execute_transfer's smart_account.require_auth()
    #      -- satisfied automatically via Soroban's invoker-contract
    #      shortcut (smart_account is transfer_adapter's *direct* caller,
    #      so this needs no signed entry at all and never appears as an
    #      auth context) -- confirmed empirically: this succeeded even
    #      with an empty sub_invocations list.
    #   3. the SAC token's own transfer(from, to, amount), calling
    #      from.require_auth() internally. This does NOT get the invoker
    #      shortcut, because the SAC's direct caller is transfer_adapter,
    #      not smart_account -- smart_account is only a grandparent caller
    #      here. This one must be declared, and as a DIRECT child of root
    #      (not nested under an adapter node), since the invoker-shortcut
    #      hop in between never advances the tree-matching position.
    sac_transfer_invocation = contract_fn_invocation(
        asset,
        "transfer",
        [scval.to_address(smart_account), scval.to_address(destination), scval.to_int128(amount)],
    )
    root_invocation = contract_fn_invocation(
        smart_account,
        "execute_transfer_payment",
        call_args,
        sub_invocations=[sac_transfer_invocation],
    )
    nonce1 = random.getrandbits(62)
    sig_payload = signature_payload(root_invocation, nonce1, expiration_ledger)

    # Two auth contexts reach __check_auth: the root call and the SAC
    # transfer (the adapter's own requirement is invoker-satisfied and
    # never becomes an auth context at all) -- both validated against rule
    # 0, the founding Default rule.
    context_rule_ids_scval = scval.to_vec([scval.to_uint32(0), scval.to_uint32(0)])
    context_rule_ids_xdr = context_rule_ids_scval.to_xdr_bytes()
    auth_digest = hashlib.sha256(sig_payload + context_rule_ids_xdr).digest()

    signer_scval = scval.to_enum("Delegated", scval.to_address(deployer_addr))
    signers_map_scval = stellar_xdr.SCVal(
        type=stellar_xdr.SCValType.SCV_MAP,
        map=stellar_xdr.SCMap(
            sc_map=[stellar_xdr.SCMapEntry(key=signer_scval, val=scval.to_bytes(b""))]
        ),
    )
    auth_payload_scval = scval.to_struct(
        {
            "signers": signers_map_scval,
            "context_rule_ids": context_rule_ids_scval,
        }
    )
    entry1 = address_credentials_entry(
        smart_account, nonce1, expiration_ledger, root_invocation, auth_payload_scval
    )

    # Entry #2: deployer's standard Ed25519 credential for the nested
    # `deployer.require_auth_for_args((auth_digest,))` call `authenticate()`
    # makes on smart_account's behalf.
    nested_invocation = contract_fn_invocation(
        smart_account, "__check_auth", [scval.to_bytes(auth_digest)]
    )
    nonce2 = random.getrandbits(62)
    sig_payload2 = signature_payload(nested_invocation, nonce2, expiration_ledger)
    raw_sig = deployer_kp.sign(sig_payload2)

    classic_sig_scval = scval.to_vec(
        [
            scval.to_map(
                {
                    scval.to_symbol("public_key"): scval.to_bytes(deployer_kp.raw_public_key()),
                    scval.to_symbol("signature"): scval.to_bytes(raw_sig),
                }
            )
        ]
    )
    entry2 = address_credentials_entry(
        deployer_addr, nonce2, expiration_ledger, nested_invocation, classic_sig_scval
    )

    return [entry1, entry2]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smart-account", required=True)
    parser.add_argument("--asset", required=True, help="SAC contract ID")
    parser.add_argument("--destination", required=True, help="Recipient G-address")
    parser.add_argument("--amount", type=int, required=True)
    parser.add_argument("--nonce", type=int, required=True)
    parser.add_argument("--expected-policy-version", type=int, required=True)
    parser.add_argument("--deployer-identity", default="sta-testnet-deployer")
    args = parser.parse_args()

    deployer_secret = stellar_secret(args.deployer_identity)
    deployer_kp = Keypair.from_secret(deployer_secret)

    server = SorobanServer(RPC_URL)
    latest_ledger = server.get_latest_ledger().sequence
    expiration_ledger = latest_ledger + 100

    call_args = [
        scval.to_address(args.asset),
        scval.to_address(args.destination),
        scval.to_int128(args.amount),
        scval.to_uint64(args.nonce),
        scval.to_uint32(args.expected_policy_version),
    ]

    auth_entries = build_auth_entries(
        deployer_kp,
        args.smart_account,
        call_args,
        args.asset,
        args.destination,
        args.amount,
        expiration_ledger,
    )

    source_account = server.load_account(deployer_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=args.smart_account,
            function_name="execute_transfer_payment",
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

    prepared.sign(deployer_kp)

    send_resp = server.send_transaction(prepared)
    print("submitted:", send_resp.hash, send_resp.status)

    while True:
        time.sleep(3)
        result = server.get_transaction(send_resp.hash)
        if result.status != "NOT_FOUND":
            print("status:", result.status)
            if result.status != "SUCCESS":
                print(result)
                return 1
            print("execute_transfer_payment succeeded. tx =", send_resp.hash)
            return 0


if __name__ == "__main__":
    sys.exit(main())
