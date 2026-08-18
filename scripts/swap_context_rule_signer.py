#!/usr/bin/env python3
"""One-off: swaps a context rule's signer for a different one, on Stellar
testnet -- e.g. replacing a generated dev-test key with a developer's own
wallet address.

Runs as two sequential transactions, in this order deliberately:
  1. add_signer(new)    -- rule temporarily has both signers.
  2. remove_signer(old) -- back down to one.

Order matters: `remove_signer`'s own doc comment states removing the last
signer is only allowed if the rule has at least one policy. This rule has
none (see `add_dev_test_signer.py`), so removing the sole existing signer
first would fail `SmartAccountError::NoSignersAndPolicies` -- add the
replacement first so the rule never has zero signers and zero policies at
once.

Same hand-built custom-account authorization as the other scripts here
(`add_context_rule` and friends are all smart_account-auth-gated).
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


def build_auth_entries(deployer_kp: Keypair, smart_account: str, function_name: str, call_args: list, expiration_ledger: int):
    deployer_addr = deployer_kp.public_key

    root_invocation = contract_fn_invocation(smart_account, function_name, call_args)
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


def submit(server, deployer_kp, smart_account, function_name, call_args, label):
    expiration_ledger = server.get_latest_ledger().sequence + 100
    auth_entries = build_auth_entries(deployer_kp, smart_account, function_name, call_args, expiration_ledger)

    source_account = server.load_account(deployer_kp.public_key)
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
        print(f"[{label}] simulation failed:", exc.simulate_transaction_response, file=sys.stderr)
        return None

    prepared.sign(deployer_kp)
    send_resp = server.send_transaction(prepared)
    print(f"[{label}] submitted:", send_resp.hash, send_resp.status)

    while True:
        time.sleep(3)
        result = server.get_transaction(send_resp.hash)
        if result.status.name != "NOT_FOUND":
            print(f"[{label}] status:", result.status)
            if result.status.name != "SUCCESS":
                print(result)
                return None
            return send_resp.hash


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smart-account", required=True)
    parser.add_argument("--context-rule-id", type=int, required=True)
    parser.add_argument("--old-signer-id", type=int, required=True)
    parser.add_argument("--new-signer", required=True, help="Public address of the replacement signer")
    parser.add_argument("--deployer-identity", default="sta-testnet-deployer")
    args = parser.parse_args()

    deployer_kp = Keypair.from_secret(stellar_secret(args.deployer_identity))
    server = SorobanServer(RPC_URL)

    add_args = [
        scval.to_uint32(args.context_rule_id),
        scval.to_enum("Delegated", scval.to_address(args.new_signer)),
    ]
    if not submit(server, deployer_kp, args.smart_account, "add_signer", add_args, "add_signer"):
        return 1

    remove_args = [
        scval.to_uint32(args.context_rule_id),
        scval.to_uint32(args.old_signer_id),
    ]
    if not submit(server, deployer_kp, args.smart_account, "remove_signer", remove_args, "remove_signer"):
        print("WARNING: new signer was added but old signer removal failed -- rule now has both.", file=sys.stderr)
        return 1

    print(f"rule {args.context_rule_id} now signed solely by {args.new_signer}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
