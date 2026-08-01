import { Keypair, contract, rpc } from '@stellar/stellar-sdk';
import { config } from './config.js';

export interface VrfContractMethods {
  request_randomness: (args: { requester: string; seed: Buffer }) => Promise<contract.AssembledTransaction<bigint>>;
  fulfill_randomness: (args: { request_id: bigint; proof: Buffer }) => Promise<contract.AssembledTransaction<null>>;
  get_request: (args: { request_id: bigint }) => Promise<contract.AssembledTransaction<Record<string, unknown>>>;
  get_oracle_pubkey: () => Promise<contract.AssembledTransaction<Buffer>>;
}

export async function getClient(
  keypair: Keypair,
): Promise<contract.Client & VrfContractMethods> {
  return contract.Client.from<VrfContractMethods>({
    contractId: config.contractId,
    networkPassphrase: config.networkPassphrase,
    rpcUrl: config.rpcUrl,
    publicKey: keypair.publicKey(),
    signTransaction: keypair,
  });
}

export function getServer(): rpc.Server {
  return new rpc.Server(config.rpcUrl);
}
