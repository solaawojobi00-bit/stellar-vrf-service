# Architecture: Stellar VRF Service

## Overview

Two components, one repo:

- **`contracts/vrf-oracle/`** — a Soroban (Rust/WASM) smart contract that
  tracks randomness requests and verifies oracle proofs on-chain.
- **`oracle/`** — a Node.js/TypeScript off-chain service that watches for
  requests and submits signed proofs back to the contract.

```
                          ┌────────────────────────┐
   1. request_randomness  │                        │
  ───────────────────────>│   VRF Oracle Contract  │
   (requester, seed)      │   (Soroban / Rust)     │
                          │                        │
                          │  stores: request_id -> │
                          │  {requester, seed,      │
                          │   status: Pending}      │
                          └───────────┬────────────┘
                                      │ emits RandomnessRequested
                                      │ {request_id, seed}
                                      ▼
                          ┌────────────────────────┐
                          │   Off-chain Oracle      │
                          │   (Node/TS service)     │
                          │                        │
                          │  2. sign(seed) with BLS │
                          │     secret key -> proof │
                          └───────────┬────────────┘
                                      │ 3. fulfill_randomness
                                      │    (request_id, proof)
                                      ▼
                          ┌────────────────────────┐
                          │   VRF Oracle Contract  │
                          │                        │
                          │  4. pairing_check(      │
                          │     proof, oracle_pk,   │
                          │     H(seed)) on-chain   │
                          │  5. random = sha256(    │
                          │     proof)              │
                          │  6. store + emit        │
                          │     RandomnessFulfilled │
                          └────────────────────────┘
```

Any third-party contract can be the "requester": it calls
`request_randomness` (directly, or the pattern generalizes to a
cross-contract call), gets back a `request_id`, and later reads the
fulfilled random value via a read-only call. Nothing about verification
depends on trusting the oracle's *word* — the contract itself refuses
to store a result unless the pairing check passes.

## Why BLS12-381 (and not ECVRF or drand)

Soroban shipped native BLS12-381 host functions in **Protocol 22**
(CAP-0059): field/curve arithmetic, hash-to-curve, and `pairing_check`,
originally aimed at zk-proof and signature-aggregation use cases. This
matters for a VRF specifically because on-chain proof verification is
normally the expensive part — if we had to implement elliptic-curve
arithmetic ourselves inside contract code (the situation for a
from-scratch RFC 9381 ECVRF verifier over ed25519, since Soroban only
exposes ed25519 *signature verify*, not raw scalar/point ops as a host
function), we'd be paying tens of millions of WASM instructions for
math the host could do natively. A BLS pairing check that would cost
tens of millions of instructions if implemented in WASM runs as a
single host call under this scheme; a full BLS signature verification
using these host functions has been measured at ~26M instructions
against Soroban's ~100M per-transaction budget — comfortably inside
budget with room for the rest of the contract logic.

Construction used (standard BLS "min-pubkey-size" signature scheme,
repurposed as a VRF: the signature *is* the deterministic, unpredictable,
verifiable proof):

- Oracle secret key: `sk` (a scalar in the BLS12-381 scalar field, `Fr`).
- Oracle public key: `pk = g1_mul(G1_generator, sk)` — a `G1` point,
  registered in the contract at deploy time.
- Proof for a request with seed `m`: `sig = g2_mul(hash_to_g2(m, DST), sk)`
  — a `G2` point.
- Verification (on-chain): `pairing_check([pk, -G1_generator], [H(m), sig])`
  must equal the pairing identity, using Soroban's native
  `Bls12_381::pairing_check`, `Bls12_381::hash_to_g2`, `Bls12_381::g1_mul` /
  `g2_mul` host functions (`soroban-sdk` `crypto::bls12_381` module,
  SDK v27+).
- Random output: `sha256(sig)` — deterministic given `m` and `sk`,
  unpredictable without `sk`, and independently verifiable by anyone
  who knows `pk` and re-runs the pairing check.

This is the same class of construction as drand's randomness beacon
(BLS threshold signatures verified via pairing), minus the threshold
network. **We deliberately did not use drand for v1**: drand gives a
stronger trust model (no single party's secret key to protect) but
couples request fulfillment to drand's fixed round schedule (rounds
every 3s or 30s depending on network) and adds a live external network
dependency, which complicates having a clean, reproducible,
locally-runnable end-to-end demo for Phase 1. Swapping the oracle's
"sign with our own key" step for "fetch and verify the next drand round
signature" is a natural, well-scoped later-phase upgrade — the on-chain
pairing-check verifier barely changes; see the Phase 2+ issue backlog.

We also deliberately did not use full RFC 9381 ECVRF: it's the more
"textbook" VRF term (used by e.g. Algorand), but Soroban has no host
functions for ed25519 curve arithmetic beyond signature verification, so
a compliant on-chain verifier would mean implementing curve math from
scratch in contract code — high implementation risk and instruction
cost for a Phase 1 that needs to actually work within real resource
limits, for no cryptographic benefit over the BLS construction above in
this single-oracle trust model.

**Risk to validate early (tracked as the first Phase 1 milestone):**
the off-chain library's `hash_to_g2` must produce bit-identical output
to Soroban's host `hash_to_g2` for the same message and DST (both
should implement RFC 9380's `BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_`
suite, but this is only confirmed by an actual passing end-to-end test,
not by documentation alone).

## Soroban contract design

Location: `contracts/vrf-oracle/`. Rust, `soroban-sdk` v27+.

**Storage:**
- Instance storage: `Config { oracle_pubkey: BytesN<96> }` (a G1 point,
  compressed serialization), set once via `initialize(admin, oracle_pubkey)`.
- Persistent storage, keyed by `request_id: u64`:
  `Request { requester: Address, seed: Bytes, status: Status, random: Option<BytesN<32>> }`
  where `Status` is `Pending | Fulfilled`.
- A simple instance counter for allocating `request_id`s.

**Entry points:**
- `initialize(admin: Address, oracle_pubkey: BytesN<96>)` — one-time
  setup, `admin` required to auth.
- `request_randomness(requester: Address, seed: Bytes) -> u64` —
  `requester` must auth; stores a `Pending` request, returns
  `request_id`, emits `("randomness_requested", request_id, seed)`.
- `fulfill_randomness(request_id: u64, proof: BytesN<192>)` —
  no auth required from a specific address (anyone can relay a valid
  proof — the security comes from the pairing check, not from who
  submits the transaction); looks up the request, computes
  `hash_to_g2(seed, DST)`, runs `pairing_check` against the stored
  `oracle_pubkey` and submitted `proof`, aborts with an error if it
  fails, otherwise sets `status = Fulfilled`, `random = sha256(proof)`,
  emits `("randomness_fulfilled", request_id, random)`.
- `get_request(request_id: u64) -> Request` — read-only.
- `set_oracle_pubkey(admin: Address, new_pubkey: BytesN<96>)` — admin-gated
  key rotation, included in Phase 1 since it's a handful of lines and
  otherwise a deploy-and-you're-stuck footgun; the *process/governance*
  around rotation is explicitly out of scope (see PRD) and left as a
  later issue.

**Errors:** a `VrfError` enum (`NotFound`, `AlreadyFulfilled`,
`InvalidProof`, `NotInitialized`) mapped to contract errors via
`#[contracterror]`.

## Off-chain oracle service design

Location: `oracle/`. Node.js + TypeScript.

**Why TypeScript for the oracle (and not Rust, matching the contract):**
Soroban contracts have to be Rust — that's not a choice. The oracle
service is a plain off-chain process with no such constraint, and
`@stellar/stellar-sdk` (the standard, actively-maintained JS/TS client
for Soroban RPC — building/signing/submitting transactions, parsing
events) plus `@noble/curves` (an audited, RFC-9380-compliant BLS12-381
implementation) is a well-trodden, approachable stack. This also
broadens who on the Wave contributor side can pick up oracle-side
issues without first learning Rust.

**Responsibilities:**
1. Poll Soroban RPC (`getEvents`) for `randomness_requested` events
   from the deployed contract, from the last-processed ledger onward.
2. For each new event, compute `sig = G2.hashToCurve(seed, {DST}).multiply(sk)`
   using `@noble/curves/bls12-381`.
3. Build, sign (with the oracle's Stellar account key — a distinct key
   from the BLS key; one authorizes *transactions*, the other proves
   *randomness*), and submit a `fulfill_randomness(request_id, proof)`
   invocation.
4. Track last-processed ledger sequence locally (a small JSON/state
   file for Phase 1 — durable queuing/retry is a later-phase concern,
   see backlog) so a restart doesn't reprocess or skip requests.

**Configuration** (via `.env`, see `.env.example`): Soroban RPC URL,
network passphrase, contract ID, oracle Stellar account secret (for
submitting transactions), oracle BLS secret key (for signing proofs).
These are two genuinely different keys with different jobs and must
never be confused in code or docs.

## Data flow for one full request → deliver cycle

1. A requesting party (in Phase 1, a test script standing in for a
   real third-party contract) calls `request_randomness(requester, seed)`.
2. Contract assigns `request_id`, stores `Pending`, emits
   `randomness_requested`.
3. Oracle service's poll loop picks up the event on its next tick.
4. Oracle computes `proof = hash_to_g2(seed, DST) * sk`.
5. Oracle submits `fulfill_randomness(request_id, proof)`.
6. Contract recomputes `hash_to_g2(seed, DST)` itself, runs
   `pairing_check` against the stored `oracle_pubkey` and the submitted
   `proof`. If it fails, the call aborts and `Pending` is unchanged (the
   oracle can retry, or a different relayer could submit a *correct*
   proof — nothing about submission is privileged). If it succeeds,
   status becomes `Fulfilled`, `random = sha256(proof)` is stored, event
   emitted.
7. Anyone (the original requester, or an independent auditor) can call
   `get_request(request_id)` and — separately, off-chain, using only
   the public `oracle_pubkey` and the original `seed` — recompute the
   same pairing check themselves to confirm the result wasn't forged.
   That independent re-verification path is exactly what "provably
   fair" means here, and Phase 1's end-to-end test exercises it
   explicitly.

## Tech choices summary

| Concern | Choice | Why |
|---|---|---|
| Contract language | Rust, `soroban-sdk` v27+ | Only option for Soroban |
| VRF construction | BLS12-381 signature-as-proof | Maps directly onto native Soroban host functions; avoids implementing EC math in contract code |
| On-chain verification | `Bls12_381::pairing_check` etc. (host functions) | ~26M instructions measured for a full verify vs. ~100M budget; anything hand-rolled in WASM would be far more expensive |
| Oracle service language | Node.js/TypeScript | Not constrained like the contract; `@stellar/stellar-sdk` + `@noble/curves` is a standard, audited, contributor-approachable stack |
| Oracle → chain auth | Separate Stellar account key (tx auth) vs. BLS key (proof) | Conflating "who may submit a transaction" with "what makes randomness valid" would break the trust model |
| Multi-oracle / threshold | Deferred | Single-oracle is enough for a genuinely working v1; threshold is a clear, well-scoped later upgrade |
| drand integration | Deferred | Stronger trust model, but external network dependency and fixed round schedule complicate a clean Phase 1 demo |
