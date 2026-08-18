#!/usr/bin/env python3
"""One-off: rotates `intent_registry`'s `Executor` to a narrowly-scoped
relayer identity, on Stellar testnet.

Why this needs a hand-built authorization: `intent_registry.set_executor`
is `ensure_admin`-gated, and `intent_registry`'s configured admin is
`smart_account` itself (see `bootstrap_intent_registry.py`) -- a Soroban
custom account, not a plain keypair. Same mechanics as that script and
`execute_demo_transfer_payment.py`: an `AuthPayload` credential for
`smart_account`, plus a nested classic Ed25519 credential for the
registered `Signer::Delegated` wallet that authorizes it.

Why this call exists at all: the relayer role must not hold authority
beyond submitting already-approved scheduled-payment executions
(`docs/TECHNICAL_ARCHITECTURE.md` SS13.1). Leaving `Executor` pointed at
the deployer -- which is also the treasury owner and every other
contract's admin -- would hand a relayer integration far more power than
it needs. This rotates it to a dedicated identity that can only ever
satisfy `intent_registry.mark_child_executed`'s auth check.
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


def build_auth_entries(deployer_kp: Keypair, smart_account: str, intent_registry: str, new_executor: str, expiration_ledger: int):
    deployer_addr = deployer_kp.public_key

    root_invocation = contract_fn_invocation(
        intent_registry, "set_executor", [scval.to_address(new_executor)]
    )
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
    parser.add_argument("--intent-registry", required=True)
    parser.add_argument("--new-executor", required=True)
    parser.add_argument("--deployer-identity", default="sta-testnet-deployer")
    args = parser.parse_args()

    deployer_kp = Keypair.from_secret(stellar_secret(args.deployer_identity))
    server = SorobanServer(RPC_URL)
    expiration_ledger = server.get_latest_ledger().sequence + 100

    auth_entries = build_auth_entries(
        deployer_kp, args.smart_account, args.intent_registry, args.new_executor, expiration_ledger
    )

    source_account = server.load_account(deployer_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=args.intent_registry,
            function_name="set_executor",
            parameters=[scval.to_address(args.new_executor)],
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
            print("executor rotated to", args.new_executor)
            return 0


if __name__ == "__main__":
    sys.exit(main())
