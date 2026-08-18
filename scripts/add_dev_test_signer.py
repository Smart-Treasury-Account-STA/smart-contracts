#!/usr/bin/env python3
"""One-off: registers a dedicated test signer for dApp development, under
its own new context rule on `smart_account`, on Stellar testnet.

Why a *new* rule rather than adding this signer to the existing founding
rule (rule 0): rule 0 has no attached policy, so `do_check_auth` requires
*every* signer listed on it to co-sign (see `get_validated_context_by_id`
in `stellar-accounts-0.7.2`). Adding a second signer there would force
every future payment to be co-signed by both the deployer and the new
test key -- breaking the deployer's existing ability to approve alone,
and requiring the deployer's key for the developer's own testing. A
separate `Default` rule containing only the new signer lets the
developer approve payments independently, referencing this rule's ID in
`AuthPayload.context_rule_ids` instead of `[0]` -- see
`docs/DAPP_INTEGRATION_SPEC.md` SS5 for the AuthPayload mechanics this
still requires (this call is itself smart_account-auth-gated, so it uses
the same hand-built-authorization technique as the other scripts here).
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


def contract_fn_invocation(contract_id: str, function_name: str, args: list):
    function = stellar_xdr.SorobanAuthorizedFunction(
        type=stellar_xdr.SorobanAuthorizedFunctionType.SOROBAN_AUTHORIZED_FUNCTION_TYPE_CONTRACT_FN,
        contract_fn=stellar_xdr.InvokeContractArgs(
            contract_address=Address(contract_id).to_xdr_sc_address(),
            function_name=stellar_xdr.SCSymbol(function_name.encode()),
            args=args,
        ),
    )
    return stellar_xdr.SorobanAuthorizedInvocation(function=function, sub_invocations=[])


def signature_payload(invocation, nonce: int, expiration_ledger: int) -> bytes:
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


def address_credentials_entry(address, nonce, expiration_ledger, invocation, signature_scval):
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


def build_auth_entries(deployer_kp: Keypair, smart_account: str, call_args: list, expiration_ledger: int):
    deployer_addr = deployer_kp.public_key

    root_invocation = contract_fn_invocation(smart_account, "add_context_rule", call_args)
    nonce1 = random.getrandbits(62)
    sig_payload = signature_payload(root_invocation, nonce1, expiration_ledger)

    context_rule_ids_scval = scval.to_vec([scval.to_uint32(0)])
    context_rule_ids_xdr = context_rule_ids_scval.to_xdr_bytes()
    auth_digest = hashlib.sha256(sig_payload + context_rule_ids_xdr).digest()

    signer_scval = scval.to_enum("Delegated", scval.to_address(deployer_addr))
    signers_map_scval = stellar_xdr.SCVal(
        type=stellar_xdr.SCValType.SCV_MAP,
        map=stellar_xdr.SCMap(sc_map=[stellar_xdr.SCMapEntry(key=signer_scval, val=scval.to_bytes(b""))]),
    )
    auth_payload_scval = scval.to_struct(
        {"signers": signers_map_scval, "context_rule_ids": context_rule_ids_scval}
    )
    entry1 = address_credentials_entry(
        smart_account, nonce1, expiration_ledger, root_invocation, auth_payload_scval
    )

    nested_invocation = contract_fn_invocation(smart_account, "__check_auth", [scval.to_bytes(auth_digest)])
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
    parser.add_argument("--dev-signer", required=True, help="Public address of the new test signer")
    parser.add_argument("--rule-name", default="dapp_dev_test")
    parser.add_argument("--deployer-identity", default="sta-testnet-deployer")
    args = parser.parse_args()

    deployer_kp = Keypair.from_secret(stellar_secret(args.deployer_identity))
    server = SorobanServer(RPC_URL)
    expiration_ledger = server.get_latest_ledger().sequence + 100

    call_args = [
        scval.to_enum("Default", None),           # context_type
        scval.to_string(args.rule_name),           # name
        scval.to_void(),                            # valid_until: None
        scval.to_vec([scval.to_enum("Delegated", scval.to_address(args.dev_signer))]),  # signers
        scval.to_map({}),                           # policies
    ]

    auth_entries = build_auth_entries(deployer_kp, args.smart_account, call_args, expiration_ledger)

    source_account = server.load_account(deployer_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=args.smart_account,
            function_name="add_context_rule",
            parameters=call_args,
            auth=auth_entries,
        )
        .build()
    )

    try:
        prepared = server.prepare_transaction(tx)
    except PrepareTransactionException as exc:
        print("Simulation failed:", exc.simulate_transaction_response, file=sys.stderr)
        return 1

    prepared.sign(deployer_kp)
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
            print("context rule created for", args.dev_signer)
            return 0


if __name__ == "__main__":
    sys.exit(main())
