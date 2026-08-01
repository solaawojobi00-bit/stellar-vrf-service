import { bls12_381 } from '@noble/curves/bls12-381.js';

/**
 * Must exactly match contracts/vrf-oracle/src/vrf.rs. Any mismatch here
 * means the oracle's proofs will fail the contract's pairing check.
 */
export const G1_BASE_SEED = 'stellar-vrf-service:g1-base:v1';
export const G1_BASE_DST = 'STELLAR-VRF-SERVICE_BLS12381G1_XMD:SHA-256_SSWU_RO_BASE_V1';
export const G2_SIG_DST = 'STELLAR-VRF-SERVICE_BLS12381G2_XMD:SHA-256_SSWU_RO_SIG_V1';

const utf8 = new TextEncoder();

/** This service's fixed G1 base point, matching the contract's `vrf::g1_base`. */
export function g1Base() {
  return bls12_381.G1.hashToCurve(utf8.encode(G1_BASE_SEED), { DST: G1_BASE_DST });
}

/** Derives the G1 public key (96-byte uncompressed) for a secret scalar. */
export function pubkeyFromSecret(sk: bigint): Uint8Array {
  return g1Base().multiply(sk).toBytes(false);
}

/**
 * Produces the VRF proof (192-byte uncompressed G2 point) for `seed` under
 * secret scalar `sk`. This is the exact off-chain counterpart to the
 * contract's on-chain `vrf::verify`.
 */
export function proveRandomness(sk: bigint, seed: Uint8Array): Uint8Array {
  const hm = bls12_381.G2.hashToCurve(seed, { DST: G2_SIG_DST });
  return hm.multiply(sk).toBytes(false);
}

/**
 * Locally re-verifies a proof the same way the contract does, without
 * touching the network. Used by the demo/tests to sanity check a proof
 * before submitting it on-chain.
 */
export function verifyLocally(pubkey: Uint8Array, seed: Uint8Array, proof: Uint8Array): boolean {
  const pk = bls12_381.G1.Point.fromBytes(pubkey);
  const sig = bls12_381.G2.Point.fromBytes(proof);
  const hm = bls12_381.G2.hashToCurve(seed, { DST: G2_SIG_DST });
  const base = g1Base();
  const Fp12 = bls12_381.fields.Fp12;
  const check = bls12_381.pairingBatch([
    { g1: base, g2: sig },
    { g1: pk.negate(), g2: hm },
  ]);
  return Fp12.eql(check, Fp12.ONE);
}
