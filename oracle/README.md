# vrf-oracle-service

Off-chain oracle for `stellar-vrf-service`: watches the deployed
`vrf-oracle` Soroban contract for `randomness_requested` events and
fulfills each one with a BLS12-381 proof. See the repo root
[ARCHITECTURE.md](../ARCHITECTURE.md) for the full design and why this
construction was chosen.

## Setup

```
npm install
cp .env.example .env
```

## 1. Generate an oracle BLS keypair

```
npm run keygen
```

This prints `ORACLE_BLS_SECRET_HEX` (put it in `.env` - keep it secret)
and `oracle_pubkey` (public, needed for the contract's `initialize` call
below).

## 2. Build and deploy the contract

From the repo root, using the [Stellar CLI](https://developers.stellar.org/docs/tools/cli/stellar-cli):

```
cd contracts/vrf-oracle
stellar contract build

# A funded Testnet identity to act as contract admin and oracle account:
stellar keys generate oracle --network testnet --fund

stellar contract deploy \
  --wasm target/wasm32v1-none/release/vrf_oracle.wasm \
  --source oracle --network testnet
# -> prints the deployed CONTRACT_ID

stellar contract invoke --id <CONTRACT_ID> --source oracle --network testnet -- \
  initialize --admin $(stellar keys address oracle) --oracle_pubkey <oracle_pubkey from step 1>
```

## 3. Fill in `.env`

- `VRF_CONTRACT_ID` - the `CONTRACT_ID` from the deploy step above.
- `ORACLE_ACCOUNT_SECRET` - `stellar keys show oracle` (the same
  identity used as admin above; it's the account the service signs
  transactions with).
- `ORACLE_BLS_SECRET_HEX` - from step 1. Must correspond to the
  `oracle_pubkey` passed to `initialize`, or every proof will fail
  on-chain.

## 4. Run the end-to-end demo

```
npm run demo
```

Requests randomness from a fresh, independently-funded account,
fulfills it, reads back the result, and independently re-runs the
pairing check and `sha256(proof)` derivation itself - the same thing an
outside auditor could do with nothing but the contract's public state
- to prove the value wasn't just asserted by the oracle.

## 5. Run the watcher (long-running)

```
npm start
```

Polls for new `randomness_requested` events and fulfills them
automatically. Tracks the last-processed ledger in `STATE_FILE` so a
restart doesn't reprocess or skip requests.

## Tests

```
npm test
```

Self-contained checks of the BLS12-381 proof/verify math (no network
required). Cross-implementation compatibility with the contract's
on-chain host functions is proven by `npm run demo` against a real
deployment, not by these unit tests alone.
