# PRD: Stellar VRF Service

## Summary

A reusable Soroban smart contract paired with an off-chain oracle service
that together produce **provably-fair random values** for any Stellar
project that needs unpredictable, publicly-verifiable randomness on-chain.
Any contract can request a random value and later receive a result that
it can verify itself, without trusting the oracle's word for it.

## Problem

Smart contracts cannot generate true randomness on their own: every
value derived from ledger state (timestamps, sequence numbers, hashes
of prior transactions) is either predictable in advance or influenceable
by whoever submits the transaction that reads it. This is a well-known
class of vulnerability (see historical "predictable RNG" exploits in
on-chain lotteries and NFT reveals across many chains).

Any Stellar/Soroban project that needs to make a fair, contestable
selection runs into this immediately:

- A raffle or fair-launch mint needs to pick winners in a way entrants
  can independently confirm wasn't rigged after the fact.
- A whitelist/allocation mechanism needs to select a subset of
  applicants without the deploying team being able to favor anyone.
- A dispute-resolution or jury-selection system needs to assign
  reviewers/jurors such that no party (including the platform) can
  steer the assignment.

Today, projects on Stellar solve this ad hoc — usually with a
"trust us" admin-submitted value, a block-hash-based pseudo-random
number that's manipulable by whoever controls transaction ordering, or
by not solving it at all. There is no shared, reusable, verifiable
randomness primitive in the Soroban ecosystem (confirmed by a search of
`soroban-examples` and other public Soroban repos at time of writing —
none combine BLS12-381 host functions into an on-chain VRF verifier).

## Why provable fairness matters on-chain specifically

Off-chain systems can rely on reputation, legal recourse, or a trusted
auditor to vouch for a random draw. On-chain systems are trust-minimized
by design and often adversarial: participants have direct financial
incentive to detect and exploit bias, and the whole point of putting the
mechanism on a public ledger is that anyone can check it. A random value
that isn't *verifiable* — not just "random-looking," but cryptographically
provable to have been derived correctly from an agreed seed and a
committed public key — defeats the purpose of building on-chain at all.

## Target Users

- **Other Stellar/Soroban contract developers** who need a randomness
  primitive and want to call an audited, reusable contract rather than
  build their own crypto.
- **Raffle / fair-launch / whitelist-allocation projects** on Stellar
  that need to publicly prove a draw wasn't rigged.
- **Dispute-resolution or DAO tooling projects** that need unbiased
  juror/reviewer selection.
- **Wave Program contributors** (secondary but important audience):
  the codebase and issue backlog need to be approachable enough for
  outside contributors to pick up scoped work without deep VRF
  cryptography background.

This is infrastructure, not an app: v1 succeeds if a *different* project
can integrate against the deployed contract without talking to us.

## Core Scope for v1

1. **Soroban contract** with two public entry points:
   - `request_randomness(requester, seed)` — records a pending request
     tied to a unique request ID and an application-supplied seed
     (e.g. a raffle ID + block sequence number), emits an event.
   - `fulfill_randomness(request_id, proof)` — accepts the oracle's BLS
     signature over the request's seed, verifies it on-chain via
     `pairing_check` against the oracle's registered public key, and
     if valid, derives and stores the random output (e.g.
     `sha256(proof)`), emits a `RandomnessFulfilled` event, and rejects
     (aborts) if the proof does not verify.
   - Read-only accessor to fetch a fulfilled request's random value and
     status.
2. **Single registered oracle public key** stored in contract config at
   deploy time (owner-set once). Multi-oracle / threshold support is
   explicitly deferred (see Out of Scope).
3. **Off-chain oracle service** that:
   - Watches the contract for `RandomnessRequested` events.
   - Computes a BLS12-381 signature over the request's seed using its
     private key.
   - Submits `fulfill_randomness` back to the contract.
4. **A genuinely working end-to-end demo**: a script/test that calls
   `request_randomness` on a locally-run or testnet-deployed contract,
   runs the oracle service against it, and shows the fulfilled,
   verified random value coming back — proving the full request →
   deliver cycle actually works, not just that each half compiles.
5. **Docs sufficient for external integration**: how another contract
   calls `request_randomness` and reads back a verified result.

## Out of Scope for v1 (explicitly deferred)

- **Multi-oracle / threshold signing** (e.g. requiring M-of-N oracle
  signatures before a result is accepted). v1 is single-oracle,
  single-point-of-trust-in-the-key, clearly documented as such.
- **Decentralized oracle incentives/staking/slashing.** No token
  economics for oracle honesty in v1.
- **On-chain oracle key rotation / governance.** Changing the oracle
  public key in v1 requires a contract owner admin call; no
  timelock/voting mechanism yet.
- **Batch/subscription randomness** (e.g. recurring draws, VRF-as-a-service
  subscriptions). v1 is one request → one fulfillment.
- **A hosted, always-on production oracle operated by us for third
  parties.** v1 ships the *software* for running an oracle; whether we
  operate one as a public good is a later business decision, not a v1
  engineering deliverable.
- **Front-end/UI.** v1 is contract + service + docs, no web app.
- **Cross-chain bridging of randomness** (e.g. serving Stellar-sourced
  randomness to other chains, or vice versa).
- **drand or other public-beacon integration.** Considered for v1 and
  deferred in favor of a self-hosted BLS oracle for the initial working
  version (see ARCHITECTURE.md for the tradeoff); revisiting this as a
  trust-model upgrade is a good candidate for a later-phase issue.
- **Full RFC 9381 ECVRF.** Same reasoning — deferred in favor of a
  BLS12-381-based construction that maps directly onto Soroban's native
  host functions (see ARCHITECTURE.md).

## Success Criteria for v1 (Phase 1)

- A deployed (or locally deployable) Soroban contract that compiles,
  and whose `fulfill_randomness` genuinely rejects invalid/forged
  proofs (negative test) and accepts a real oracle proof (positive
  test).
- An oracle service that can be pointed at a running network and
  autonomously fulfill a real request end-to-end.
- A reproducible test/demo proving the full cycle, runnable by someone
  who is not us.
