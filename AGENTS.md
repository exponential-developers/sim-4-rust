# AGENTS.md

Guidance for AI coding agents (and people) working on sim-4.0-rust: the Exponential Idle theory
simulator, rewritten so the simulation engine runs in Rust compiled to WebAssembly. Theories are
ported one by one from sim-3.0 (TypeScript) and must produce exactly the same results.

## Rules for every change

These apply to every change, and are checked in CI (`.github/workflows/tests.yml`).

**Keep all checks passing.** Before finishing, run:

```sh
cargo fmt --manifest-path wasm/Cargo.toml --check
cargo clippy --release --all-targets --manifest-path wasm/Cargo.toml -- -D warnings
cargo test --release --manifest-path wasm/Cargo.toml
wasm-pack test --node --release wasm --test sim3_parity
npm run lint
npm run build
```

**Lint.**
- Never introduce new lint errors or warnings (Clippy or ESLint). Fix the code; don't silence the
  lint. An `#[allow(...)]`, `eslint-disable` comment or new entry in `eslint-suppressions.json` is
  only acceptable with a comment explaining why the code is right as written.
- `eslint-suppressions.json` lists errors that existed before linting was set up. When you fix any of
  them, run `npx eslint src --prune-suppressions` so the file shrinks.
- Format Rust with `cargo fmt`.

**Types (TypeScript).**
- No `any`: use the real type, or `unknown` narrowed with a type check.
- No `@ts-ignore` / `@ts-expect-error`, and no `as` casts that hide a real type mismatch.
- New data passed between Rust and TypeScript gets a type in `src/Utils/global.types.ts` that
  matches the Rust struct.

**Tests.**
- Every new or changed function that computes something (theory maths, cost/value models, helpers,
  sim modes, parsing) gets unit tests in the same change. Aim for 100% coverage of `wasm/src`:
  it is easier to delete a weak test later than to write the missing ones. Coverage must not go
  down (see "Coverage").
- Test our own behavior against expected values worked out independently: by hand, from sim-3, or
  from the game's formulas. Don't compute the expected value by calling the function under test.
- Cover edge cases that matter here: 0 and negative values, `-inf` (log of 0), NaN, equal values,
  level 0 / first level, caps.
- Don't write tests that only exercise the language or libraries: that a struct stores what it was
  given, that `Vec::push` or serde work, that a constant equals itself, that a getter returns its
  field. Every test should be able to fail because of a bug in this project's code.
- Ported theories must pass the sim-3 parity tests (see "Porting a theory").

## Layout

| Path | What it is |
|---|---|
| `wasm/src/theories/` | Theory simulations. `traditional.rs` is the shared base (sim-3's `theory.ts` + `traditionalTheory.ts`); `t1.rs`, `t2.rs` are ported theories. |
| `wasm/src/sim/mod.rs` | Sim modes (single, chain, step, all, step_chain, amount, time, comparison, pub_table), "Best ..." strat selection, and the list of ported theories (`PORTED_THEORIES`, `run_strat`). |
| `wasm/src/api/` | JSON request/response types (`query.rs`, `response.rs`), `data.json` config (`config.rs`), and the parser for strat conditions (`condition.rs`). |
| `wasm/src/utils/` | `LogNum`, cost/value models, `Variable`, and `fastmath.rs` (see "Exact results" below). |
| `wasm/src/lib.rs` | Functions exported to JavaScript: `set_config`, `run_query`, `run_query_with_progress`, `ported_theories`, `sim_strat` (benchmarks/tests). |
| `wasm/tests/sim3_parity.rs` + `wasm/tests/fixtures/sim3_results.json` | Parity tests against saved sim-3 results. |
| `src/` | Website (TypeScript). `Sim/worker.ts` runs the wasm module in a Web Worker; `Sim/simWorker.ts` talks to it from the page; `Sim/main.ts` handles Simulate/Stop; `Sim/parse.ts` builds queries; `Sim/write.ts` renders results; `Sim/cache.ts` is the "All" mode cache. |
| `src/Data/data.json` | Theory config: `tau_factor`, strats and their conditions, modes, themes. Also sent to the wasm module at startup. |
| `src/Sim/simulate.ts`, `src/Theories/` | The old TypeScript simulator. **Not used by the page** (kept as a reference); it is still type-checked, so it must keep compiling. |
| `bench/` | Node scripts that compare the Rust sim with sim-3 and time both, and generate the parity fixtures. |
| `Styles/` | CSS. Every color is a token defined per theme in `themes.css`. |
| `eslint.config.mjs`, `eslint-suppressions.json` | ESLint setup, and the list of errors that predate it (see "Rules for every change"). |

## Build and run

Requirements: Node.js, Rust with the `wasm32-unknown-unknown` target, and `wasm-pack`
(`cargo install wasm-pack`).

```sh
npm install
npm run build          # wasm-pack build + webpack (development)
npm run webpack-dev    # TypeScript only, when no Rust file changed
npm run prod           # production build (what CI deploys)
```

Serve the repository root with any static file server (e.g. `python -m http.server 8001`) and open
`index.html`. The page cannot be opened from `file://` (module scripts and WebAssembly need HTTP).

On Windows PowerShell with a restricted execution policy, use `npm.cmd` / `npx.cmd` instead of
`npm` / `npx`.

## Tests

```sh
cargo test --release --manifest-path wasm/Cargo.toml          # unit tests (native)
wasm-pack test --node --release wasm --test sim3_parity       # parity with sim-3 (~1 min)
cargo clippy --release --all-targets --manifest-path wasm/Cargo.toml -- -D warnings
cargo fmt --manifest-path wasm/Cargo.toml                     # format Rust
npm run lint                                                  # ESLint (eslint.config.mjs)
```

All of these run in CI (`.github/workflows/tests.yml`), along with `npm run build`.

### Coverage

```sh
cargo install cargo-llvm-cov && rustup component add llvm-tools-preview   # once
cd wasm && cargo llvm-cov --release -- --include-ignored                 # per-file summary
cargo llvm-cov --release --html -- --include-ignored                    # browsable report in target/llvm-cov/html
```

Coverage is measured natively, so it includes the unit tests and `tests/coverage_cases.rs`, an
ignored-by-default test that runs every saved sim-3 case through the public API so the theory and
sim-mode code is counted. (The parity tests run as WebAssembly, which `cargo llvm-cov` can't
measure.) CI fails if line coverage of `wasm/src` drops below the `--fail-under-lines` threshold in
the coverage job: raise it when coverage improves, never lower it. The parity tests compare every result with sim-3
**bit for bit** (`==` on floats, no tolerance): strat results, "Best ..." selection, chains and the
other modes. They only pass in WebAssembly, which is why they run with `wasm-pack test --node`.

Comparing against a live sim-3 checkout (expects it at `../sim-3.0`, or set `SIM3_DIR`):

```sh
cd ../sim-3.0 && npx webpack --config ../sim-4-rust/bench/webpack.sim3.config.js   # bundles sim-3's code for Node
cd ../sim-4-rust && wasm-pack build wasm --target nodejs --out-dir ../bench/out/wasm-node
node bench/run.js [iterations] [theory...]   # per-strat parity + timings
node bench/compare-modes.js                  # Best selection, chains, other modes
node bench/make-fixtures.js                  # regenerate wasm/tests/fixtures/sim3_results.json
```

Test cases live in `bench/cases.js`. After changing them, or when sim-3 changes, regenerate the
fixtures and commit them.

**Node version matters.** Results are bit-identical only within the same JavaScript engine: V8
versions differ in the last digit of `Math.log10` / `Math.pow` (in the browser, the Rust sim uses
the browser's `Math`, so it matches sim-3 *in that browser*). The fixtures record the Node version
they were generated with (`source` field), and the parity tests must run on the same Node major
version. CI pins it in `.github/workflows/tests.yml`; update both together.

## Exact results: rules for theory code

A port is only correct if it matches sim-3 exactly. Tiny floating point differences change when a
variable gets bought and then the whole publication. To keep results identical:

- **All logs and powers go through `utils::fastmath`** (`l10()` in `traditional.rs`,
  `fastmath::exp10`, `fastmath::pow`, `fastmath::log2`), *including constants* such as `log10(5)`
  in a buying condition. In WebAssembly these call JavaScript's `Math.log10` / `Math.pow`, which is
  what sim-3 uses. Rust's own `f64::log10`, `powf`, or a constant folded by the compiler can differ
  in the last digit.
- **Parse cost parameters with `parse_value`**, like sim-3's `parseValue(String(x))`: numbers
  >= 1e21 become `exponent + log10(mantissa)` (e.g. `3e25` -> `25 + log10(3)`).
- **Keep sim-3's operation order.** `a + b - c` and `a - c + b` round differently. Port expressions
  as written, including sim-3's quirks (e.g. JavaScript `Math.max` returns NaN if any argument is
  NaN: use `js_max`).
- **Recompute values from the level, don't accumulate** (see `ExponentialValue::compute_next` and
  `StepwisePowerSumValue::compute_from_zero`, which follow sim-3's formulas).
- Values are log10 numbers (`f64`) as in sim-3; `LogNum` is used for currencies and variable
  costs/values.
- Set `core.track_cur_mult = false` in theories that never read the publication multiplier during
  a sim (saves a `pow` per tick).
- Call `core.collect_pub_table()` at the end of each tick (needed by the pub table mode).

## Porting a theory

1. Read sim-3's `src/Theories/<group>/<Theory>.ts` and the base classes it uses.
2. Create `wasm/src/theories/<theory>.rs` following `t1.rs` / `t2.rs`: a strat enum with
   `name()` / `from_name()` matching `data.json`, a sim struct holding a `TraditionalCore`, and a
   `simulate_<theory>(strat, input)` function reproducing sim-3's top-level function (multi-pass
   coasting, forks, etc.).
3. Register it in `wasm/src/theories/mod.rs`, and in `wasm/src/sim/mod.rs` (`PORTED_THEORIES` and
   `run_strat`).
4. Make the theory's strats in `src/Data/data.json` match sim-3's (names, order, conditions). The
   order decides ties between strats with equal tau/h.
5. Add the theory to `bench/sim3-entry.js` (`THEORIES`) and `bench/cases.js`, check it with
   `bench/run.js` and `bench/compare-modes.js`, then regenerate the fixtures.
6. Very slow strats (many sims per result, e.g. T2Haxolotl) go in `SLOW` in `bench/cases.js`.

## Website notes

- The TypeScript finds elements by **class name**. Keep class names when changing `index.html`.
  `render.ts` uses the first `.controls` row as the theory inputs, so keep the two `.controls` rows
  in their current order (CSS shows the mode row first).
- The wasm module only runs in the worker (`src/Sim/worker.ts`). Stopping a simulation terminates
  the worker and starts a new one.
- Theme ids must match the `themes` list in `data.json`; each theme sets the full set of tokens in
  `Styles/themes.css`.
- `wasm/pkg/` is generated by `wasm-pack` and not tracked by git.
- `data.json` strat names are also used as types by the old TypeScript simulator, so adding a strat
  may require adding it to the matching `src/Theories/*.ts` conditions to keep it compiling.

## Known differences from sim-3

- sim-3 lowercases strat conditions, including quoted names, so conditions like
  `lastStrat !== "T4C3"` never match a real strat name there. `condition.rs` keeps string case, so
  once T4/T6 are ported their results can differ from sim-3 on those conditions.
- Time and chain modes return an error instead of looping forever when no strat makes progress.
