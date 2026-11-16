// Saves sim-3's results for the cases in cases.js to wasm/tests/fixtures/sim3_results.json.
// The Rust test wasm/tests/sim3_parity.rs checks the Rust sim against this file, so the check runs
// without a sim-3 checkout (e.g. in CI). Re-run after changing cases.js or when sim-3 changes:
//   cd ../sim-3.0 && npx.cmd webpack --config ../sim-4-rust/bench/webpack.sim3.config.js
//   cd ../sim-4-rust && node bench/make-fixtures.js
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const sim3 = require("./out/sim3-t1.js");
const { STRATS, STRAT_CASES, SLOW, CATEGORIES, BEST_CASES, CHAINS, MODES } = require("./cases.js");

const SIM3 = path.resolve(process.env.SIM3_DIR || path.join(__dirname, "..", "..", "sim-3.0"));
let commit = "unknown";
try { commit = execSync("git rev-parse --short HEAD", { cwd: SIM3 }).toString().trim(); } catch { /* not a git checkout */ }

(async () => {
  const out = {
    // The Node version matters: results are only bit-identical within the same JavaScript engine
    source: `sim-3.0 @ ${commit}, Node ${process.version}`,
    settings: { dt: 1.5, ddt: 1.0001, bought_vars_delta: 5 },
    strats: [],
    best: [],
    chains: [],
    modes: [],
  };

  // The theory's own inputs (e.g. MF's reset depth) are only saved when a case has some
  const withInputs = (inputs) => (inputs && Object.keys(inputs).length > 0 ? { theory_inputs: inputs } : {});

  for (const theory of Object.keys(STRATS)) {
    STRAT_CASES[theory].forEach(([rho, sigma, inputs], caseIndex) => {
      for (const strat of STRATS[theory]) {
        // Slow strats are only recorded for the first case to keep the test fast
        if (SLOW.has(strat) && caseIndex > 0) continue;
        out.strats.push({ theory, strat, sigma, rho, ...withInputs(inputs) });
      }
    });
  }
  for (const c of out.strats) c.expected = await sim3.simStrat(c.theory, c.strat, c.sigma, c.rho, c.theory_inputs);

  for (const theory of Object.keys(BEST_CASES)) {
    for (const strat of CATEGORIES) {
      for (const [rho, sigma, inputs] of BEST_CASES[theory]) {
        out.best.push({ theory, strat, sigma, rho, ...withInputs(inputs), expected: await sim3.single(theory, strat, sigma, rho, "", inputs) });
      }
    }
  }

  for (const [theory, strat, sigma, rho, cap, hardCap, inputs] of CHAINS) {
    out.chains.push({ theory, strat, sigma, rho, cap, hard_cap: hardCap, ...withInputs(inputs), expected: await sim3.chain(theory, strat, sigma, rho, cap, hardCap, inputs) });
  }

  // Other modes, in the same reduced form bench/compare-modes.js compares. A case may end with
  // the theory's own inputs.
  for (const [mode, theory, strat, sigma, rho, ...extra] of MODES) {
    const inputs = typeof extra[extra.length - 1] === "object" ? extra.pop() : {};
    let expected;
    switch (mode) {
      case "step": expected = await sim3.step(theory, strat, sigma, rho, ...extra, inputs); break;
      case "amount": expected = await sim3.amount(theory, strat, sigma, rho, ...extra, inputs); break;
      case "time": expected = await sim3.timeMode(theory, strat, sigma, rho, ...extra, inputs); break;
      case "step_chain": expected = await sim3.stepChain(theory, strat, sigma, rho, ...extra, inputs); break;
      case "comparison": expected = await sim3.comparison(theory, sigma, rho, inputs); break;
      case "pub_table": expected = await sim3.pubTable(theory, strat, sigma, rho, ...extra); break;
    }
    out.modes.push({ mode, theory, strat, sigma, rho, extra, ...withInputs(inputs), expected });
  }

  const file = path.join(__dirname, "..", "wasm", "tests", "fixtures", "sim3_results.json");
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify(out, null, 1) + "\n");
  console.log(`Wrote ${out.strats.length} strat cases, ${out.best.length} Best cases, ${out.chains.length} chains and ${out.modes.length} other mode cases from ${out.source} to ${path.relative(process.cwd(), file)}`);
})();
