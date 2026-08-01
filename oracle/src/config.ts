import 'dotenv/config';

function required(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`Missing required environment variable: ${name} (see .env.example)`);
  }
  return value;
}

export const config = {
  rpcUrl: process.env.SOROBAN_RPC_URL ?? 'https://soroban-testnet.stellar.org',
  networkPassphrase: process.env.STELLAR_NETWORK_PASSPHRASE ?? 'Test SDF Network ; September 2015',
  contractId: required('VRF_CONTRACT_ID'),
  /** The oracle's Stellar account secret (`S...`), used to sign and submit transactions. */
  oracleAccountSecret: required('ORACLE_ACCOUNT_SECRET'),
  /**
   * The oracle's BLS12-381 secret scalar, as a hex string (see
   * `scripts/keygen.ts`). This is a *different* key from the account
   * secret above: it authorizes randomness proofs, not transactions.
   */
  oracleBlsSecretHex: required('ORACLE_BLS_SECRET_HEX'),
  pollIntervalMs: Number(process.env.POLL_INTERVAL_MS ?? 5000),
  stateFile: process.env.STATE_FILE ?? './state/last-ledger.json',
};

export function blsSecretFromHex(hex: string): bigint {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  return BigInt(`0x${clean}`);
}
