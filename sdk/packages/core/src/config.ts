/**
 * Network configuration for the Smart Treasury Account SDK.
 *
 * `testnet` is populated with the live, currently-deployed contract set --
 * the `smart_account` address is the account_factory-deployed treasury from
 * docs/TESTNET_FACTORY_DEPLOYMENT.md §13.2, which runs the fixed contract
 * code (see docs/SECURITY_REVIEW_STRICT.md finding 29). The older
 * hand-deployed treasury from docs/TESTNET_DEPLOYMENT.md predates that fix
 * and is not the default here.
 *
 * `mainnet` is deliberately `undefined` -- no mainnet deployment exists yet.
 * Once one does, add an entry here with the same shape; nothing else in
 * this SDK assumes `testnet` specifically, every helper takes a
 * `NetworkConfig` as a parameter.
 */

export interface ContractAddresses {
  smartAccount: string;
  policyEngine: string;
  intentRegistry: string;
  recoveryManager: string;
  accountFactory: string;
  /** Not bound by this SDK's generated clients -- never called directly by
   * a client (see docs/DAPP_INTEGRATION_SPEC.md §1) -- recorded for
   * reference and for building auth-entry sub-invocations by hand. */
  transferAdapter: string;
  splitAdapter: string;
}

export interface NetworkConfig {
  network: "testnet" | "mainnet";
  rpcUrl: string;
  networkPassphrase: string;
  contracts: ContractAddresses;
}

export const TESTNET: NetworkConfig = {
  network: "testnet",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  contracts: {
    smartAccount: "CD6GY4UUTNPW4TUV7LDL5SELN4BBHJG4KDDT3W6G23DY6XCGM75MULMQ",
    policyEngine: "CCOP7NRMST5K6TL7FBDMX25LDEPW3DSFBOGBIKVFIDNAAZY7GBMVP3M4",
    intentRegistry: "CAFIATSIZQSBILZJWVT4PVDXPVITJHLP6LPAVKDRHCA7I7XPZSLTRPUS",
    recoveryManager: "CCHC4YKVYS3CAZUOUYWYTEMQ6TZDW75WB2BGENUC2CDWDX5RH7NMKZWU",
    accountFactory: "CAQQTRRYNXIQGFVNCTMTBJDXW3PN7O44KPT7GWCCE4FRKTOHDBCWGUZO",
    transferAdapter: "CBRYGIR3ORDW5LE6J7AVPSKRNTMRUYHD6FVPHQMJGPQLQ5FQUZ2U6GFH",
    splitAdapter: "CBQA7UI7QN6RN4IZT7WPDHWTK2OO7J4FH2KMCVJGKMKVFGDURD63UQ7U",
  },
};

/** No mainnet deployment exists yet -- see this module's doc comment. */
export const MAINNET: NetworkConfig | undefined = undefined;

export const NETWORKS = {
  testnet: TESTNET,
  mainnet: MAINNET,
} as const;
