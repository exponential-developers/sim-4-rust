// Compares "Best ..." strat selection and chain mode between sim-3 and the Rust sim, going
// through the same JSON API (set_config + main) that sim-4's website uses.
//
// Build first (see run.js), then: node bench/compare-modes.js
const path = require("path");
const fs = require("fs");
const sim3 = require("./out/sim3-t1.js");
const wasm = require(path.join(__dirname, "out", "wasm-node", "wasm.js"));
const { CATEGORIES, BEST_CASES, CHAINS, MODES } = require("./cases.js");

wasm.set_config(fs.readFileSync(path.join(__dirname, "..", "src", "Data", "data.json"), "utf8"));

const SETTINGS = {
  dt: 1.5, ddt: 1.0001, mf_reset_depth: 0, bought_vars_delta: 5,
  sim_all_strats: "all", completed_cts: "no", show_a23: false, show_unofficials: false,
};

function rust(type, data) {
  const res = JSON.parse(wasm.run_query(JSON.stringify({ type, data: { settings: SETTINGS, ...data } })));
  if (res.response_type !== "success") throw new Error(res.data);
  return res.data.data;
}
const rustSummary = (r) => ({ strat: r.strat, tauH: r.tau_h, pubRho: r.pub_rho, time: r.time, boughtVars: r.bought_vars.length });
const same = (a, b) => a.strat === b.strat && a.tauH === b.tauH && a.pubRho === b.pubRho && a.time === b.time && a.boughtVars === b.boughtVars;

/** Runs one MODES case in sim-3 and in Rust, both reduced to comparable plain objects */
async function runMode([mode, theory, strat, sigma, rho, ...extra]) {
  const chainTotals = (r, a, b, c) => ({ results: r.results, deltaTau: r[a], averageRate: r[b], totalTime: r[c] });
  switch (mode) {
    case "step": {
      const [cap, step] = extra;
      return { expected: await sim3.step(theory, strat, sigma, rho, cap, step),
        actual: { results: rust("step", { theory, strat, sigma, rho, cap, step }).results.map(rustSummary) } };
    }
    case "amount": {
      const [amount] = extra;
      const r = rust("amount", { theory, strat, sigma, rho, amount });
      return { expected: await sim3.amount(theory, strat, sigma, rho, amount),
        actual: chainTotals({ ...r, results: r.results.map(rustSummary) }, "delta_tau", "average_rate", "total_time") };
    }
    case "time": {
      const [time] = extra;
      const r = rust("time", { theory, strat, sigma, rho, time });
      return { expected: await sim3.timeMode(theory, strat, sigma, rho, time),
        actual: chainTotals({ ...r, results: r.results.map(rustSummary) }, "delta_tau", "average_rate", "total_time") };
    }
    case "step_chain": {
      const [cap, step, hardCap] = extra;
      return { expected: await sim3.stepChain(theory, strat, sigma, rho, cap, step, hardCap),
        actual: { results: rust("step_chain", { theory, strat, sigma, rho, cap, step, hard_cap: hardCap }).results.map(rustSummary) } };
    }
    case "comparison":
      return { expected: await sim3.comparison(theory, sigma, rho),
        actual: { results: rust("comparison", { theory, sigma, rho }).results.map(rustSummary) } };
    case "pub_table": {
      const [cap, step] = extra;
      return { expected: await sim3.pubTable(theory, strat, sigma, rho, cap, step),
        actual: rust("pub_table", { theory, strat, sigma, rho, cap, step }) };
    }
  }
}

(async () => {
  let ok = 0, bad = 0;
  for (const theory of Object.keys(BEST_CASES)) {
    for (const cat of CATEGORIES) {
      for (const [rho, sigma] of BEST_CASES[theory]) {
        const a = await sim3.single(theory, cat, sigma, rho, "");
        const b = rustSummary(rust("single", { theory, strat: cat, sigma, rho, cap: null, last_strat: null }).result);
        if (same(a, b)) ok++;
        else { bad++; console.log(`DIFFERENT single ${theory} ${cat} e${rho} sigma=${sigma}\n  sim-3: ${JSON.stringify(a)}\n  rust : ${JSON.stringify(b)}`); }
      }
    }
  }
  console.log(`Single sims with Best strats: ${ok} identical, ${bad} different`);

  let chainOk = 0;
  for (const [theory, strat, sigma, rho, cap, hardCap] of CHAINS) {
    const t0 = performance.now();
    const a = await sim3.chain(theory, strat, sigma, rho, cap, hardCap);
    const t1 = performance.now();
    const b = rust("chain", { theory, strat, sigma, rho, cap, hard_cap: hardCap });
    const t2 = performance.now();
    const bResults = b.results.map(rustSummary);
    const identical = a.results.length === bResults.length && a.results.every((r, i) => same(r, bResults[i]))
      && a.deltaTau === b.delta_tau && a.averageRate === b.average_rate && a.totalTime === b.total_time;
    if (identical) chainOk++;
    console.log(`chain ${theory} ${strat} e${rho}->e${cap} sigma=${sigma}${hardCap ? " hard cap" : ""}: ${a.results.length} pubs, `
      + `${identical ? "identical" : "DIFFERENT"} (sim-3 ${(t1 - t0).toFixed(0)} ms, Rust ${(t2 - t1).toFixed(0)} ms)`);
    if (!identical) {
      const i = a.results.findIndex((r, i) => !bResults[i] || !same(r, bResults[i]));
      console.log(`  first difference at pub ${i}:\n  sim-3: ${JSON.stringify(a.results[i])}\n  rust : ${JSON.stringify(bResults[i])}`);
    }
  }
  console.log(`Chains: ${chainOk}/${CHAINS.length} identical`);

  let modeOk = 0;
  for (const m of MODES) {
    const { expected, actual } = await runMode(m);
    const ok = JSON.stringify(expected) === JSON.stringify(actual);
    if (ok) modeOk++;
    console.log(`${m.join(" ")}: ${ok ? "identical" : "DIFFERENT"}`);
    if (!ok) console.log(`  sim-3: ${JSON.stringify(expected).slice(0, 600)}
  rust : ${JSON.stringify(actual).slice(0, 600)}`);
  }
  console.log(`Other modes: ${modeOk}/${MODES.length} identical`);
  process.exitCode = bad || chainOk !== CHAINS.length || modeOk !== MODES.length ? 1 : 0;
})();
