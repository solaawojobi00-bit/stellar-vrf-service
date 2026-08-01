/**
 * End-to-end proof that the request -> fulfill -> independently-verify
 * cycle actually works against a real deployed contract.
 *
 * Prerequisites (see oracle/README.md for the full deploy walkthrough):
 * - The contract is deployed and `initialize`d with the oracle_pubkey
 *   derived from ORACLE_BLS_SECRET_HEX below.
 * - `.env` has VRF_CONTRACT_ID, ORACLE_ACCOUNT_SECRET, ORACLE_BLS_SECRET_HEX
 *   pointed at that deployment (defaults to Testnet).
 *
 * Usage: `npm run demo`
 */
import { createHash } from 'node:crypto';
import { Keypair } from '@stellar/stellar-sdk';
import { config, blsSecretFromHex } from '../src/config.js';
import { getClient } from '../src/contractClient.js';
import { proveRandomness, verifyLocally } from '../src/vrf.js';

async function fundIfNeeded(publicKey: string) {
  if (!config.rpcUrl.includes('testnet') && !config.rpcUrl.includes('futurenet')) {
    return; // assume already funded on other networks (e.g. local/mainnet)
  }
  const res = await fetch(`https://friendbot.stellar.org?addr=${publicKey}`);
  if (!res.ok && res.status !== 400) {
    // 400 typically means "already funded" - anything else is worth surfacing.
    console.warn(`[demo] friendbot funding returned ${res.status}: ${await res.text()}`);
  }
}

async function main() {
  const oracleKeypair = Keypair.fromSecret(config.oracleAccountSecret);
  const sk = blsSecretFromHex(config.oracleBlsSecretHex);

  // A fresh, independent requester - distinct from the oracle account -
  // to prove the request/fulfill roles are genuinely separate, not just
  // the oracle talking to itself.
  const requesterKeypair = Keypair.random();
  console.log(`[demo] requester account: ${requesterKeypair.publicKey()}`);
  await fundIfNeeded(requesterKeypair.publicKey());

  const requesterClient = await getClient(requesterKeypair);
  const oracleClient = await getClient(oracleKeypair);

  const seed = Buffer.from(`demo-seed-${Date.now()}`);
  console.log(`[demo] requesting randomness for seed "${seed.toString()}"`);

  const requestTx = await requesterClient.request_randomness({
    requester: requesterKeypair.publicKey(),
    seed,
  });
  const { result: requestId } = await requestTx.signAndSend();
  console.log(`[demo] request_id = ${requestId}`);

  const { result: pending } = await oracleClient.get_request({ request_id: requestId });
  console.log(`[demo] pending request state:`, pending);

  console.log('[demo] oracle computing BLS proof...');
  const proof = proveRandomness(sk, seed);

  console.log('[demo] submitting fulfill_randomness...');
  const fulfillTx = await oracleClient.fulfill_randomness({
    request_id: requestId,
    proof: Buffer.from(proof),
  });
  await fulfillTx.signAndSend();

  const { result: fulfilled } = (await oracleClient.get_request({ request_id: requestId })) as unknown as {
    result: { status: unknown; random?: Buffer };
  };
  console.log('[demo] fulfilled request state:', fulfilled);

  if (!fulfilled.random) {
    throw new Error('DEMO FAILED: request was not fulfilled with a random value');
  }

  const { result: oraclePubkey } = (await oracleClient.get_oracle_pubkey()) as unknown as { result: Buffer };

  console.log('[demo] independently re-verifying the proof (as an outside auditor would)...');
  const proofValid = verifyLocally(new Uint8Array(oraclePubkey), seed, proof);
  const expectedRandom = createHash('sha256').update(proof).digest();
  const randomMatches = Buffer.from(fulfilled.random).equals(expectedRandom);

  console.log(`[demo] pairing check passes locally: ${proofValid}`);
  console.log(`[demo] on-chain random == sha256(proof): ${randomMatches}`);

  if (!proofValid || !randomMatches) {
    throw new Error('DEMO FAILED: independent re-verification did not match');
  }

  console.log('\n[demo] SUCCESS: full request -> fulfill -> independently-verified cycle works.');
}

main().catch((err) => {
  console.error('[demo] FAILED:', err);
  process.exit(1);
});
