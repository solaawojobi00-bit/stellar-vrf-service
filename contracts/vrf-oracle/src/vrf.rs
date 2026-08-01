//! BLS12-381 VRF construction used by this contract.
//!
//! Scheme (standard BLS "min-pubkey-size" signature, repurposed as a VRF
//! proof): the oracle holds a secret scalar `sk`. Its public key is
//! `pk = sk * G1_BASE` (a G1 point). A proof for message `m` (the request
//! seed) is `sig = sk * hash_to_g2(m)` (a G2 point). Verification checks
//! `e(G1_BASE, sig) == e(pk, hash_to_g2(m))` via a single pairing_check
//! call, using Soroban's native BLS12-381 host functions.
//!
//! `G1_BASE` is *not* the standard BLS12-381 G1 generator. Hardcoding that
//! generator's raw field-element coordinates by hand is exactly the kind
//! of transcription that's easy to get subtly wrong with no way to
//! visually verify it. Instead, `G1_BASE` is derived deterministically via
//! `hash_to_g1` over a fixed domain-separated message, computed the same
//! way on-chain (this module) and off-chain (the oracle service, using
//! the same message/DST with a standard hash-to-curve library). Any
//! non-identity point works as a basis for a prime-order group, so this
//! is cryptographically equivalent to using the canonical generator, and
//! it only requires trusting a well-known modulus constant (`ORDER`
//! below) rather than transcribing curve point coordinates.

use soroban_sdk::{
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine, Bls12381G2Affine},
    vec, Bytes, BytesN, Env, U256,
};

/// Fixed message hashed (via `hash_to_g1`) to derive this contract's G1
/// base point. Must match the oracle service's `G1_BASE_SEED` exactly.
pub const G1_BASE_SEED: &[u8] = b"stellar-vrf-service:g1-base:v1";
/// Domain separation tag for deriving the G1 base point.
pub const G1_BASE_DST: &[u8] = b"STELLAR-VRF-SERVICE_BLS12381G1_XMD:SHA-256_SSWU_RO_BASE_V1";
/// Domain separation tag used when hashing a request seed to G2 for
/// signing/verification. Must match the oracle service exactly.
pub const G2_SIG_DST: &[u8] = b"STELLAR-VRF-SERVICE_BLS12381G2_XMD:SHA-256_SSWU_RO_SIG_V1";

/// The BLS12-381 scalar field order `r`, big-endian. This is a
/// widely-published, easily cross-checked constant (identical across
/// blst, zkcrypto/bls12_381, arkworks, noble-curves, py_ecc, ...),
/// unlike raw curve point coordinates.
// Cross-checked against @noble/curves' `bls12_381.fields.Fr.ORDER` at
// implementation time (see oracle/), rather than hand-verified, since a
// single wrong hex digit here would silently break every proof.
const ORDER_BE: [u8; 32] = [
    0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8, 0x05,
    0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01,
];

fn neg_one(env: &Env) -> Bls12381Fr {
    let mut order_minus_one = ORDER_BE;
    order_minus_one[31] -= 1;
    Bls12381Fr::from_u256(U256::from_be_bytes(env, &Bytes::from_array(env, &order_minus_one)))
}

/// This contract's fixed G1 base point, `hash_to_g1(G1_BASE_SEED, G1_BASE_DST)`.
pub fn g1_base(env: &Env) -> Bls12381G1Affine {
    let bls = env.crypto().bls12_381();
    let msg = Bytes::from_slice(env, G1_BASE_SEED);
    let dst = Bytes::from_slice(env, G1_BASE_DST);
    bls.hash_to_g1(&msg, &dst)
}

/// Verifies a VRF proof for `seed` against `pubkey`. `pubkey` is a 96-byte
/// uncompressed G1 affine point, `proof` a 192-byte uncompressed G2
/// affine point.
pub fn verify(env: &Env, pubkey: &BytesN<96>, seed: &Bytes, proof: &BytesN<192>) -> bool {
    let bls = env.crypto().bls12_381();

    let pk = Bls12381G1Affine::from_bytes(pubkey.clone());
    let sig = Bls12381G2Affine::from_bytes(proof.clone());
    let hm = bls.hash_to_g2(seed, &Bytes::from_slice(env, G2_SIG_DST));
    let base = g1_base(env);
    let pk_neg = bls.g1_mul(&pk, &neg_one(env));

    // e(base, sig) == e(pk, hm)  <=>  e(base, sig) * e(-pk, hm) == 1
    bls.pairing_check(vec![env, base, pk_neg], vec![env, sig, hm])
}

/// Derives the VRF proof for `seed` given secret scalar `sk`, for use in
/// tests (the real oracle signs off-chain with its own BLS library).
#[cfg(any(test, feature = "testutils"))]
pub fn sign(env: &Env, sk: &Bls12381Fr, seed: &Bytes) -> BytesN<192> {
    let bls = env.crypto().bls12_381();
    let hm = bls.hash_to_g2(seed, &Bytes::from_slice(env, G2_SIG_DST));
    bls.g2_mul(&hm, sk).to_bytes()
}

/// Derives the public key for secret scalar `sk`, for use in tests.
#[cfg(any(test, feature = "testutils"))]
pub fn pubkey(env: &Env, sk: &Bls12381Fr) -> BytesN<96> {
    let bls = env.crypto().bls12_381();
    bls.g1_mul(&g1_base(env), sk).to_bytes()
}
