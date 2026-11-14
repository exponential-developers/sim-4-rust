// Compares each ported theory's strats between sim-3 (JavaScript) and the Rust sim compiled to
// WebAssembly, both in Node (V8, the same engine as Chrome): checks results match, then times both.
//
// Build first:
//   cd ../sim-3.0 && npx.cmd webpack --config ../sim-4-rust/bench/webpack.sim3.config.js
//   cd ../sim-4-rust && wasm-pack build wasm --target nodejs --out-dir ../bench/out/wasm-node
// Run:
//   node bench/run.js [iterations] [theory...]
const path = require("path");
const sim3 = require("./out/sim3-t1.js");
const wasm = require(path.join(__dirname, "out", "wasm-node", "wasm.js"));
const { STRATS, STRAT_CASES, SLOW } = require("./cases.js");

const ITER = Number(process.argv[2] || 3);
const THEORIES = process.argv.slice(3).length ? process.argv.slice(3) : Object.keys(STRATS);

const rel = (a, b) => Math.abs(a - b) / Math.max(Math.abs(a), Math.abs(b), 1e-300);
const rust = (theory, strat, sigma, rho) => {
  const r = JSON.parse(wasm.sim_strat(theory, strat, sigma, rho));
  if (r.response_type === "failure") throw new Error(r.data);
  return r;
};

async function time(fn) {
  const t0 = performance.now();
  for (let i = 0; i < ITER; i++) await fn();
  return (performance.now() - t0) / ITER;
}

(async () => {
  let failures = 0;
  for (const theory of THEORIES) {
    const strats = STRATS[theory];
    // Warm up both engines
    for (const s of strats.filter((s) => !SLOW.has(s))) { await sim3.simStrat(theory, s, 90, 150); rust(theory, s, 90, 150); }

    let ok = 0, n = 0, totalJs = 0, totalWasm = 0;
    const rows = [];
    for (const [rho, sigma] of STRAT_CASES[theory]) {
      let jsMs = 0, wasmMs = 0;
      for (const strat of strats) {
        const a = await sim3.simStrat(theory, strat, sigma, rho);
        const b = rust(theory, strat, sigma, rho);
        n++;
        if (a.strat === b.strat && a.boughtVars === b.boughtVars && a.time === b.time && a.tauH === b.tauH && a.pubRho === b.pubRho) ok++;
        else {
          failures++;
          console.log(`DIFFERENT ${theory} e${rho} sigma=${sigma} ${strat} (rel diff ${rel(a.tauH, b.tauH).toExponential(2)})`
            + `\n  sim-3: ${JSON.stringify(a)}\n  rust : ${JSON.stringify(b)}`);
        }
        if (SLOW.has(strat)) continue;
        jsMs += await time(() => sim3.simStrat(theory, strat, sigma, rho));
        wasmMs += await time(() => rust(theory, strat, sigma, rho));
      }
      totalJs += jsMs; totalWasm += wasmMs;
      rows.push({ rho: `e${rho}`, sigma, "sim-3 JS (ms)": +jsMs.toFixed(2), "Rust wasm (ms)": +wasmMs.toFixed(2), speedup: +(jsMs / wasmMs).toFixed(2) });
    }
    rows.push({ rho: "TOTAL", sigma: "", "sim-3 JS (ms)": +totalJs.toFixed(2), "Rust wasm (ms)": +totalWasm.toFixed(2), speedup: +(totalJs / totalWasm).toFixed(2) });
    console.log(`\n${theory}: ${ok}/${n} strat results identical to sim-3`);
    console.log(`Time for all ${theory} strats per case${[...SLOW].some((s) => strats.includes(s)) ? " (excluding " + strats.filter((s) => SLOW.has(s)).join(", ") + ")" : ""}, average of ${ITER} runs (Node ${process.version}):`);
    console.table(rows);
  }
  process.exitCode = failures ? 1 : 0;
})();
