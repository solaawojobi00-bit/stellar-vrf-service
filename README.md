# stellar-vrf-service

A reusable, verifiable random function (VRF) service for the Stellar
ecosystem: a Soroban smart contract paired with an off-chain oracle that
together let any contract request a random value and later verify —
independently, without trusting the oracle's word — that the result was
derived correctly.

Use cases: raffles, whitelist/fair-launch allocation, jury/reviewer
selection for disputes, or anywhere an on-chain process needs an
unpredictable outcome that participants can contest and verify after
the fact.

See [PRD.md](./PRD.md) for the problem statement, scope, and non-goals.
See [ARCHITECTURE.md](./ARCHITECTURE.md) for the contract/oracle design,
the BLS12-381 VRF construction, and why it was chosen over drand or
ECVRF.

## Status

Phase 1 (a genuinely working end-to-end request → fulfill cycle) is in
progress on [`feat/phase-1`](../../tree/feat/phase-1). See the
[issues](../../issues) for the Phase 2+ backlog.

## Layout

```
contracts/vrf-oracle/   Soroban contract (Rust)
oracle/                 off-chain oracle service (Node.js/TypeScript)
PRD.md                  problem, users, scope, non-goals
ARCHITECTURE.md         design and technical rationale
CONTRIBUTING.md         dev setup, branch naming, tests, PR process
```

See [CONTRIBUTING.md](./CONTRIBUTING.md) to get set up locally and pick
up an issue.

## License

[MIT](./LICENSE)
