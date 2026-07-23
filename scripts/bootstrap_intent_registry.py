#!/usr/bin/env python3
"""One-off bootstrap for `intent_registry.initialize(admin=smart_account)` on
Stellar testnet.

Why this exists: `intent_registry`'s admin *must* be `smart_account` itself
(see `contracts/smart_account/src/lib.rs`'s `create_intent`/`cancel_intent`,
which call `IntentRegistryClient::create_intent`/`cancel_intent` directly,
both `ensure_admin`-gated). So `initialize`'s `admin.require_auth()` requires
authorization from `smart_account` -- a custom-account contract (OpenZeppelin
`stellar-accounts` `SmartAccount`), not a plain keypair. The bare `stellar`
CLI can only sign for classic accounts, so it cannot drive this call; see
docs/TESTNET_DEPLOYMENT.md's "Known limitation" section.

What this script does instead: hand-builds the two Soroban authorization
entries `__check_auth` actually needs, replicating
`stellar-accounts-0.7.2/src/smart_account/storage.rs`'s `do_check_auth`/
`authenticate` exactly:

  1. An `Address` credentials entry for `smart_account`, whose `signature`
     field is not a real signature but the contract's own `AuthPayload`
     struct: `{signers: {Signer::Delegated(deployer): <unchecked bytes>},
     context_rule_ids: [0]}` (rule 0 is the `Default` rule set up at
     `smart_account.initialize`, which authorizes any context).
  2. A standard `Address` credentials entry for `deployer` (a plain Ed25519
     account), authorizing the nested call `authenticate()` makes on that
     signer's behalf: `deployer.require_auth_for_args((auth_digest,))`,
     where `auth_digest = sha256(signature_payload || context_rule_ids.to_xdr())`
     -- the digest `do_check_auth` actually asks the delegated signer to
     prove control over, not the raw host-computed payload.

This is a narrow, one-time substitute for the general off-chain
wallet/SDK/relayer layer that `docs/V1_SCOPE.md` names as out of V1 scope --
it only handles this one bootstrapping call, not arbitrary smart_account
invocations.
"""
from __future__ import annotations

import argparse
import hashlib
import random
import subprocess
import sys
import time

from stellar_sdk import Address, Keypair, Network, SorobanServer, TransactionBuilder
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
    contract_id: str, function_name: str, args: list
) -> stellar_xdr.SorobanAuthorizedInvocation:
    function = stellar_xdr.SorobanAuthorizedFunction(
        type=stellar_xdr.SorobanAuthorizedFunctionType.SOROBAN_AUTHORIZED_FUNCTION_TYPE_CONTRACT_FN,
        contract_fn=stellar_xdr.InvokeContractArgs(
            contract_address=Address(contract_id).to_xdr_sc_address(),
            function_name=stellar_xdr.SCSymbol(function_name.encode()),
            args=args,
        ),
    )
    return stellar_xdr.SorobanAuthorizedInvocation(function=function, sub_invocations=[])


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
    deployer_kp: Keypair, smart_account: str, intent_registry: str, expiration_ledger: int
) -> list[stellar_xdr.SorobanAuthorizationEntry]:
    from stellar_sdk import scval

    deployer_addr = deployer_kp.public_key

    # Entry #1: smart_account's own custom-account credential for the
    # top-level `admin.require_auth()` inside intent_registry.initialize.
    root_invocation = contract_fn_invocation(
        intent_registry, "initialize", [scval.to_address(smart_account)]
    )
    nonce1 = random.getrandbits(62)
    sig_payload = signature_payload(root_invocation, nonce1, expiration_ledger)

    context_rule_ids_scval = scval.to_vec([scval.to_uint32(0)])
    context_rule_ids_xdr = context_rule_ids_scval.to_xdr_bytes()
    auth_digest = hashlib.sha256(sig_payload + context_rule_ids_xdr).digest()

    # scval.to_map() takes a Python dict, which requires hashable keys --
    # but an enum-shaped SCVal (SCV_VEC, used for Signer::Delegated) wraps a
    # plain list and isn't hashable. Build the single-entry SCMap directly.
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
    # makes from within smart_account's own execution (contract_address=
    # smart_account, function_name="__check_auth" -- the frame active when
    # that nested call happens).
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
    parser.add_argument("--intent-registry", required=True)
    parser.add_argument("--deployer-identity", default="sta-testnet-deployer")
    args = parser.parse_args()

    deployer_secret = stellar_secret(args.deployer_identity)
    deployer_kp = Keypair.from_secret(deployer_secret)

    server = SorobanServer(RPC_URL)
    latest_ledger = server.get_latest_ledger().sequence
    expiration_ledger = latest_ledger + 100

    auth_entries = build_auth_entries(
        deployer_kp, args.smart_account, args.intent_registry, expiration_ledger
    )

    source_account = server.load_account(deployer_kp.public_key)
    tx = (
        TransactionBuilder(source_account, NETWORK_PASSPHRASE, base_fee=BASE_FEE)
        .set_timeout(120)
        .append_invoke_contract_function_op(
            contract_id=args.intent_registry,
            function_name="initialize",
            parameters=[__import__("stellar_sdk").scval.to_address(args.smart_account)],
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
            print("intent_registry initialized. admin =", args.smart_account)
            return 0


if __name__ == "__main__":
    sys.exit(main())
