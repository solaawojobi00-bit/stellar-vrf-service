/**
 * Generates a new oracle BLS12-381 keypair. Run once per oracle deployment.
 *
 * Usage: `npm run keygen`
 *
 * Prints:
 * - `ORACLE_BLS_SECRET_HEX` — put this in the oracle's `.env`. Keep it
 *   secret; anyone who has it can forge randomness proofs.
 * - `oracle_pubkey` (hex) — pass this to the contract's `initialize` (or
 *   `set_oracle_pubkey`) call. This is public.
 */
import { bls12_381 } from '@noble/curves/bls12-381.js';
import { pubkeyFromSecret } from '../src/vrf.js';

function toHex(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('hex');
}

const skBytes = bls12_381.utils.randomSecretKey();
const sk = BigInt(`0x${toHex(skBytes)}`);
const pubkey = pubkeyFromSecret(sk);

console.log('Generated a new oracle BLS12-381 keypair.\n');
console.log(`ORACLE_BLS_SECRET_HEX=${toHex(skBytes)}`);
console.log(`\noracle_pubkey (hex, pass to contract initialize/set_oracle_pubkey): ${toHex(pubkey)}`);
console.log('\nKeep the secret above out of version control and chat logs.');
