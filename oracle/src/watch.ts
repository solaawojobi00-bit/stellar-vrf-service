import { Keypair, scValToNative, xdr } from '@stellar/stellar-sdk';
import { config, blsSecretFromHex } from './config.js';
import { getClient, getServer } from './contractClient.js';
import { loadState, saveState } from './state.js';
import { proveRandomness } from './vrf.js';

const REQUESTED_TOPIC = xdr.ScVal.scvSymbol('randomness_requested').toXDR('base64');

async function processNewRequests(
  sk: bigint,
  keypair: Keypair,
  fromLedger: number,
): Promise<number> {
  const server = getServer();
  const client = await getClient(keypair);

  let cursorLedger = fromLedger;
  const { events, latestLedger } = await server.getEvents({
    startLedger: fromLedger,
    filters: [
      {
        type: 'contract',
        contractIds: [config.contractId],
        topics: [[REQUESTED_TOPIC, '*']],
      },
    ],
  });

  for (const ev of events) {
    const requestId = scValToNative(ev.topic[1]) as bigint;
    const data = scValToNative(ev.value) as { requester: string; seed: Buffer };

    console.log(`[watch] fulfilling request ${requestId} (requester ${data.requester})`);
    const proof = proveRandomness(sk, new Uint8Array(data.seed));

    try {
      const tx = await client.fulfill_randomness({ request_id: requestId, proof: Buffer.from(proof) });
      await tx.signAndSend();
      console.log(`[watch] fulfilled request ${requestId}`);
    } catch (err) {
      console.error(`[watch] failed to fulfill request ${requestId}:`, err);
    }

    cursorLedger = Math.max(cursorLedger, ev.ledger + 1);
  }

  return Math.max(cursorLedger, latestLedger + 1);
}

async function main() {
  const keypair = Keypair.fromSecret(config.oracleAccountSecret);
  const sk = blsSecretFromHex(config.oracleBlsSecretHex);

  const server = getServer();
  const state = loadState(config.stateFile);
  let fromLedger = state.lastProcessedLedger;
  if (fromLedger === 0) {
    const latest = await server.getLatestLedger();
    fromLedger = latest.sequence;
    console.log(`[watch] no saved state, starting from current ledger ${fromLedger}`);
  }

  console.log(`[watch] watching contract ${config.contractId} on ${config.rpcUrl}`);

  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      fromLedger = await processNewRequests(sk, keypair, fromLedger);
      saveState(config.stateFile, { lastProcessedLedger: fromLedger });
    } catch (err) {
      console.error('[watch] poll iteration failed:', err);
    }
    await new Promise((resolve) => setTimeout(resolve, config.pollIntervalMs));
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
