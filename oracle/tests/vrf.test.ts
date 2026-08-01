import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pubkeyFromSecret, proveRandomness, verifyLocally } from '../src/vrf.js';

test('pubkey and proof serialize to the byte lengths the contract expects', () => {
  const sk = 424242n;
  const pubkey = pubkeyFromSecret(sk);
  const proof = proveRandomness(sk, new TextEncoder().encode('seed'));
  assert.equal(pubkey.length, 96, 'G1 pubkey must be 96 bytes uncompressed');
  assert.equal(proof.length, 192, 'G2 proof must be 192 bytes uncompressed');
});

test('a genuine proof verifies against the matching pubkey and seed', () => {
  const sk = 424242n;
  const pubkey = pubkeyFromSecret(sk);
  const seed = new TextEncoder().encode('request-1');
  const proof = proveRandomness(sk, seed);
  assert.equal(verifyLocally(pubkey, seed, proof), true);
});

test('a proof signed with a different secret is rejected', () => {
  const pubkey = pubkeyFromSecret(424242n);
  const seed = new TextEncoder().encode('request-1');
  const forgedProof = proveRandomness(1n, seed);
  assert.equal(verifyLocally(pubkey, seed, forgedProof), false);
});

test('a valid proof for a different seed is rejected', () => {
  const sk = 424242n;
  const pubkey = pubkeyFromSecret(sk);
  const proofForOtherSeed = proveRandomness(sk, new TextEncoder().encode('other-request'));
  const seed = new TextEncoder().encode('request-1');
  assert.equal(verifyLocally(pubkey, seed, proofForOtherSeed), false);
});

test('proving is deterministic for the same (sk, seed)', () => {
  const sk = 999n;
  const seed = new TextEncoder().encode('deterministic-check');
  const a = proveRandomness(sk, seed);
  const b = proveRandomness(sk, seed);
  assert.deepEqual(a, b);
});
