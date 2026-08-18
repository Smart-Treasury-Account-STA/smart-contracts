#!/usr/bin/env python3
"""Run this yourself, on your own machine, with your own two secret keys.
Neither secret is ever sent anywhere -- they're only used locally to sign.

What this does: removes the two extra signers that got added to
smart_account's context rules 0 and 1 during testing, restoring both back
to a single signer each. It authorizes all three removals through rule 1
(context_rule_ids=[1]) -- since you control both of rule 1's current
signers (your own wallet and the second test key), you can self-authorize
this fix without needing anyone else's key, including the deployer's.

Same custom-account authorization mechanics documented in
docs/DAPP_INTEGRATION_SPEC.md SS5 -- this just has TWO Signer::Delegated
entries in the AuthPayload / TWO nested classic auth entries instead of
one, since rule 1 currently requires both of its signers to co-sign
(no policy attached yet -- see the conversation this script came out of
for why).

Requirements: `pip install stellar-sdk`
"""
from __future__ import annotations

import getpass
import hashlib
import random
import sys
import time

from stellar_sdk import Address, Keypair, Network, SorobanServer, TransactionBuilder, scval
from stellar_sdk import xdr as stellar_xdr
from stellar_sdk.exceptions import PrepareTransactionException

RPC_URL = "https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE = Network.TESTNET_NETWORK_PASSPHRASE
BASE_FEE = 100_000

SMART_ACCOUNT = "CB4KZJ3I4XANE6GWPAMXCNXQ34PTQWPXVKFBBMLNKV25GAOXQC7RQUMS"
AUTH_RULE_ID = 1  # the rule we authorize through -- you control both its signers

# The three cleanup calls: (context_rule_id being modified, signer_id to remove)
REMOVALS = [
    (0, 3, "GAA73RFL... from rule 0"),
    (0, 4, "GBYIFYJM... from rule 0"),
    (1, 3, "GAA73RFL... from rule 1"),
]


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


def build_auth_entries(signer_kps: list[Keypair], function_name: str, call_args: list, expiration_ledger: int):
    """Builds one AuthPayload entry (covering ALL signers in signer_kps) plus
    one nested classic authorization entry PER signer -- everyone signs the
    same auth_digest independently."""
    root_invocation = contract_fn_invocation(SMART_ACCOUNT, function_name, call_args)
    nonce1 = random.getrandbits(62)
    sig_payload = signature_payload(root_invocation, nonce1, expiration_ledger)

    context_rule_ids_scval = scval.to_vec([scval.to_uint32(AUTH_RULE_ID)])
    context_rule_ids_xdr = context_rule_ids_scval.to_xdr_bytes()
    auth_digest = hashlib.sha256(sig_payload + context_rule_ids_xdr).digest()

    # Soroban requires SCMap entries in canonical sorted-by-key order -- an
    # unsorted map is rejected by the host during validation (this is what
    # produced "Error(Object, InvalidInput)"). scval.to_map() would sort
    # correctly, but it takes a Python dict, which requires hashable keys --
    # and Signer::Delegated(...) encodes as SCV_VEC, which isn't hashable.
    # Sort manually with the same comparator instead.
    import functools

    signers_map_entries = []
    for kp in signer_kps:
        signer_scval = scval.to_enum("Delegated", scval.to_address(kp.public_key))
        signers_map_entries.append(stellar_xdr.SCMapEntry(key=signer_scval, val=scval.to_bytes(b"")))
    signers_map_entries.sort(key=functools.cmp_to_key(lambda a, b: scval._compare_sc_val(a.key, b.key)))
    signers_map_scval = stellar_xdr.SCVal(
        type=stellar_xdr.SCValType.SCV_MAP,
        map=stellar_xdr.SCMap(sc_map=signers_map_entries),
    )
    auth_payload_scval = scval.to_struct(
        {"signers": signers_map_scval, "context_rule_ids": context_rule_ids_scval}
    )
    entries = [
        address_credentials_entry(
            SMART_ACCOUNT, nonce1, expiration_ledger, root_invocation, auth_payload_scval
        )
    ]

    for kp in signer_kps:
        nested_invocation = contract_fn_invocation(SMART_ACCOUNT, "__check_auth", [scval.to_bytes(auth_digest)])
        nonce = random.getrandbits(62)
        sig_payload_n = signature_payload(nested_invocation, nonce, expiration_ledger)
        raw_sig = kp.sign(sig_payload_n)
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
        entries.append(
            address_credentials_entry(kp.public_key, nonce, expiration_ledger, nested_invocation, classic_sig_scval)
        )

    return entries


def submit(server, source_kp, signer_kps, function_name, call_args, label):
    expiration_ledger = server.get_latest_ledger().sequence + 200
    auth_entries = build_auth_entries(signer_kps, function_name, call_args, expiration_ledger)

    source_account = server.load_account(source_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=SMART_ACCOUNT,
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
        return False

    prepared.sign(source_kp)
    send_resp = server.send_transaction(prepared)
    print(f"[{label}] submitted:", send_resp.hash, send_resp.status)

    while True:
        time.sleep(3)
        result = server.get_transaction(send_resp.hash)
        if result.status.name != "NOT_FOUND":
            print(f"[{label}] status:", result.status)
            if result.status.name != "SUCCESS":
                print(result)
                return False
            return True


def main() -> int:
    print("This signs and submits with your own keys only -- nothing is sent anywhere.")
    print("Enter your two secret keys (input is hidden, nothing is echoed or logged).\n")

    secret1 = getpass.getpass("Secret key for GB2KHXT4...EQ4T (your wallet): ").strip()
    secret2 = getpass.getpass("Secret key for GAA73RFL... (the second test key): ").strip()

    kp1 = Keypair.from_secret(secret1)
    kp2 = Keypair.from_secret(secret2)
    print(f"\nLoaded: {kp1.public_key} and {kp2.public_key}")

    server = SorobanServer(RPC_URL)

    for rule_id, signer_id, description in REMOVALS:
        call_args = [scval.to_uint32(rule_id), scval.to_uint32(signer_id)]
        ok = submit(server, kp1, [kp1, kp2], "remove_signer", call_args, f"remove {description}")
        if not ok:
            print(f"\nStopped -- '{description}' failed. Earlier steps (if any) already succeeded on-chain.")
            return 1

    print("\nDone. Rule 0 and rule 1 are both back to one signer each.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
