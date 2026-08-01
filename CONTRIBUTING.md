# Contributing

Thanks for helping build `stellar-vrf-service`. This doc covers local
setup, branch naming, running tests, and how PRs get reviewed and merged.

## Dev environment setup

You need two toolchains: Rust (for the Soroban contract) and Node.js
(for the off-chain oracle service).

### Contract (`contracts/vrf-oracle/`)

1. Install Rust via [rustup](https://rustup.rs/).
2. Add the WASM targets:
   ```
   rustup target add wasm32-unknown-unknown wasm32v1-none
   ```
3. Install the [Stellar CLI](https://developers.stellar.org/docs/tools/cli/stellar-cli):
   ```
   cargo install --locked stellar-cli
   ```
4. On Windows, Rust needs a working host linker to compile proc-macro
   crates (used by `soroban-sdk`'s macros) even when the final artifact
   targets WASM. If `cargo check` fails with `linker link.exe not
   found`, either install the "Desktop development with C++" workload
   from Visual Studio Build Tools, or install a MinGW-w64 toolchain
   (e.g. `winget install BrechtSanders.WinLibs.POSIX.UCRT`) and switch
   your default host to the GNU target:
   ```
   rustup toolchain install stable-x86_64-pc-windows-gnu
   rustup default stable-x86_64-pc-windows-gnu
   ```
   macOS/Linux contributors shouldn't hit this.

### Oracle service (`oracle/`)

1. Install Node.js 20+.
2. `cd oracle && npm install`.
3. Copy `.env.example` to `.env` and fill in the values described there
   (RPC URL, contract ID, oracle account secret, oracle BLS secret).
   Never commit `.env` — it's gitignored, and `.env.example` must only
   ever contain placeholders.

## Branch naming

- `feat/<short-description>` — new functionality (e.g. `feat/phase-1`,
  `feat/multi-oracle-threshold`).
- `fix/<short-description>` — bug fixes.
- `docs/<short-description>` — documentation-only changes.
- `chore/<short-description>` — tooling, CI, dependency bumps.

Branch off `main`; don't commit directly to `main`.

## Running tests

Contract unit tests (no network required — runs entirely in the
Soroban test environment):

```
cd contracts/vrf-oracle
cargo test
```

Oracle service (BLS math self-checks, no network required):

```
cd oracle
npm test
```

Full end-to-end demo (deploys/uses a real contract on Stellar Testnet —
see `oracle/README.md` for the deploy steps and required `.env`
values):

```
cd oracle
npm run demo
```

Please add or update tests for any behavior change, especially
anything touching the VRF verification path in
`contracts/vrf-oracle/src/vrf.rs` or `oracle/src/vrf.ts` — a mismatch
between those two is a silent, hard-to-debug failure (proofs stop
verifying), so changes to either need a test that exercises both sides
together where practical.

## PR process

1. Pick up an issue (see the repo's [issues](../../issues) — each is
   tagged with a complexity tier and has concrete acceptance criteria)
   or open one first for anything not already tracked.
2. Branch from `main` using the naming convention above.
3. Commit as you go; keep commits reasonably scoped and use descriptive
   messages.
4. Open a PR into `main` describing what changed and why, and link the
   issue it closes (`Closes #123`).
5. Make sure `cargo test` (contract) and `npm test` (oracle) pass.
6. A maintainer reviews and merges — please don't merge your own PR.
