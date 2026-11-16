// Exposes sim-3's theories (unmodified source) with sim-3's default settings, plus its strat
// selection (getStrats), for comparing against the Rust sim. Built by webpack.sim3.config.js.
import EF from "SIM3/Theories/CTs/EF";
import CSR2 from "SIM3/Theories/CTs/CSR2";
import BaP from "SIM3/Theories/CTs/BaP";
import FI from "SIM3/Theories/CTs/FI";
import FP from "SIM3/Theories/CTs/FP";
import BT from "SIM3/Theories/Unofficial-CTs/BT";
import TC from "SIM3/Theories/Unofficial-CTs/TC";
import FS from "SIM3/Theories/Unofficial-CTs/FS";
import BD from "SIM3/Theories/Unofficial-CTs/BD";
import ILC from "SIM3/Theories/Unofficial-CTs/ILC";
import NLI from "SIM3/Theories/Unofficial-CTs/NLI";
import MF from "SIM3/Theories/CTs/MF";
import RZ from "SIM3/Theories/CTs/RZ";
import T1 from "SIM3/Theories/T1-T8/T1";
import T2 from "SIM3/Theories/T1-T8/T2";
import T3 from "SIM3/Theories/T1-T8/T3";
import T4 from "SIM3/Theories/T1-T8/T4";
import T5 from "SIM3/Theories/T1-T8/T5";
import T6 from "SIM3/Theories/T1-T8/T6";
import T7 from "SIM3/Theories/T1-T8/T7";
import T8 from "SIM3/Theories/T1-T8/T8";
import WSP from "SIM3/Theories/CTs/WSP";
import SL from "SIM3/Theories/CTs/SL";
import { getStrats } from "SIM3/Sim/strats";

const THEORIES = { RZ, MF, TC, BT, FS, BD, ILC, NLI, FP, FI, BaP, CSR2, EF, T1, T2, T3, T4, T5, T6, T7, T8, WSP, SL };
const SETTINGS = { dt: 1.5, ddt: 1.0001, boughtVarsDelta: 5 };
const CATEGORIES = ["Best Active", "Best Overall", "Best Semi-Idle", "Best Idle"];

// Theories without rho (NLI) take tau inputs; sim-4 gives them a tau factor of 1
const rhoLess = (theory) => !THEORIES[theory].converter.supportsRho;
const asInput = (theory, input) =>
  typeof input === "object" ? input : { valueType: rhoLess(theory) ? "tau" : "rho", value: input };

/** `inputs`: the theory's specific inputs (e.g. { depth: "2" } for MF) */
function run(theory, strat, sigma, rho, cap, inputs = {}) {
  // The Rust sim names T6's idle recovery stops in the strat (T6SnaxIdleRecoveryM3Coast): sim-3
  // takes the stop as a strat specific input
  const stop = /^(T6\w*IdleRecovery)M(\d)(Coast)?$/.exec(strat);
  const stratSpecificInputs = {};
  if (stop) {
    strat = stop[1] + (stop[3] ?? "");
    stratSpecificInputs.idleRcvStop = stop[2];
  }
  return THEORIES[theory].simulate({
    theory,
    specificInputs: inputs,
    stratSpecificInputs,
    sigma,
    input: asInput(theory, rho),
    strat,
    cap,
    recursionValue: null,
    settings: SETTINGS,
  });
}

const summary = (res) => ({
  strat: res.strat,
  tauH: res.tauH,
  // NLI has no rho: sim-4 reports its pub tau. sim-3's "Result undefined" (no strat could run)
  // has no pub rho; the Rust sim reports 0
  pubRho: res.pubPointRho ?? res.pubPointTau ?? 0,
  time: res.time,
  boughtVars: res.boughtVars.length,
});

/** One named strat */
export async function simStrat(theory, strat, sigma, rho, inputs = {}) {
  return summary(await run(theory, strat, sigma, rho, undefined, inputs));
}

const conv = (theory) => THEORIES[theory].converter;
const toTau = (theory, rho, sigma) => rhoLess(theory) ? rho : conv(theory).convertTo({ valueType: "rho", value: rho }, "tau", sigma);
const toRho = (theory, tau, sigma) => rhoLess(theory) ? tau : conv(theory).convertTo({ valueType: "tau", value: tau }, "rho", sigma);

/** Same logic as sim-3 `singleSim` (src/Sim/simulate.ts); `input` is a rho number or a ProgressValue */
async function singleSim(theory, strat, sigma, input, lastStrat, cap, inputs = {}) {
  const inp = asInput(theory, input);
  const strats = CATEGORIES.includes(strat)
    ? getStrats(theory, conv(theory).convertTo(inp, "tau", sigma), rhoLess(theory) ? 0 : conv(theory).convertTo(inp, "rho", sigma), strat, lastStrat ?? "")
    : [strat];
  let best = { tauH: 0, strat: "Result undefined", pubPointRho: 0, pubPointTau: 0, time: 0, boughtVars: [] };
  for (const s of strats) {
    const res = await run(theory, s, sigma, inp, cap, inputs);
    best = best.tauH >= res.tauH ? best : res; // getBestResult(bestRes, res)
  }
  return best;
}

export async function single(theory, strat, sigma, rho, lastStrat, inputs = {}) {
  return summary(await singleSim(theory, strat, sigma, rho, lastStrat, undefined, inputs));
}

/**
 * sim-3 `chainSim`: every sim starts from the tau of the previous one, with the theory inputs a
 * result passes on (NLI's lifetime α)
 */
async function chainFromTau(theory, strat, sigma, startTau, cap, hardCap, inputs = {}) {
  const capTau = toTau(theory, cap, sigma);
  let tau = startTau, time = 0, lastStrat = "";
  const raw = [];
  let theoryInputs = inputs;
  while (tau < capTau) {
    const res = await singleSim(theory, strat, sigma, { valueType: "tau", value: tau }, lastStrat, hardCap ? asInput(theory, cap) : undefined, theoryInputs);
    raw.push(res);
    tau = res.pubPointTau;
    lastStrat = res.strat.split(" ")[0];
    time += res.time;
    if (res.theorySpecificInputs) theoryInputs = { ...theoryInputs, ...res.theorySpecificInputs };
  }
  return { raw, tau, time };
}

export async function chain(theory, strat, sigma, rho, cap, hardCap, inputs = {}) {
  const start = toTau(theory, rho, sigma);
  const { raw, tau, time } = await chainFromTau(theory, strat, sigma, start, cap, hardCap, inputs);
  return chainLike(raw.map(summary), start, tau, time);
}

// ---- Other modes: same logic as sim-3's src/Sim/simulate.ts (amountSim, timeSim, stepSim,
// stepChainSim, comparisonSim, pubTableSim) ----
import { StepPubTableCollector, collectorCache, noopCollector } from "SIM3/Utils/pubTableCollector";

const chainLike = (results, start, end, time) => ({ results, deltaTau: end - start, averageRate: (end - start) / (time / 3600), totalTime: time });

export async function amount(theory, strat, sigma, rho, amount, inputs = {}) {
  const start = toTau(theory, rho, sigma);
  let tau = start, time = 0, lastStrat = ""; const results = [];
  for (let i = 0; i < amount; i++) {
    const res = await singleSim(theory, strat, sigma, { valueType: "tau", value: tau }, lastStrat, undefined, inputs);
    results.push(summary(res)); tau = res.pubPointTau; lastStrat = res.strat.split(" ")[0]; time += res.time;
  }
  return chainLike(results, start, tau, time);
}

export async function timeMode(theory, strat, sigma, rho, total, inputs = {}) {
  const start = toTau(theory, rho, sigma);
  let tau = start, time = 0, lastStrat = ""; const results = [];
  while (time < total) {
    const res = await singleSim(theory, strat, sigma, { valueType: "tau", value: tau }, lastStrat, undefined, inputs);
    results.push(summary(res)); tau = res.pubPointTau; lastStrat = res.strat.split(" ")[0]; time += res.time;
  }
  return chainLike(results, start, tau, time);
}

/** sim-3's `calculateNextStep`: adds the step (times mult) to rho (to tau without rho) */
const nextStep = (theory, sigma, tau, stepLen, mult = 1) => rhoLess(theory)
  ? tau + stepLen * mult
  : conv(theory).convertTo({ valueType: "rho", value: toRho(theory, tau, sigma) + stepLen * mult }, "tau", sigma);

export async function step(theory, strat, sigma, rho, cap, stepLen, inputs = {}) {
  const capTau = toTau(theory, cap, sigma);
  let tau = toTau(theory, rho, sigma), lastStrat = ""; const results = [];
  while (tau < capTau + 0.00001) {
    const res = await singleSim(theory, strat, sigma, { valueType: "tau", value: tau }, lastStrat, undefined, inputs);
    results.push(summary(res));
    tau = nextStep(theory, sigma, tau, stepLen);
    lastStrat = res.strat.split(" ")[0];
  }
  return { results };
}

export async function stepChain(theory, strat, sigma, rho, cap, stepLen, hardCap, inputs = {}) {
  const capTau = toTau(theory, cap, sigma);
  let tau = toTau(theory, rho, sigma); const results = [];
  while (tau < nextStep(theory, sigma, capTau, stepLen, -1) + 0.000001) {
    const { raw } = await chainFromTau(theory, strat, sigma, tau, cap, hardCap, inputs);
    let tauAcc = 0, timeAcc = 0, n = 0;
    let best = { tauH: 0, strat: "Result undefined", pubRho: 0, time: 0, boughtVars: 0 };
    for (const res of raw) {
      tauAcc += res.deltaTau; timeAcc += res.time; n++;
      const cur = { strat: n + " pub" + (n > 1 ? "s" : ""), tauH: tauAcc / (timeAcc / 3600), pubRho: res.pubPointRho ?? res.pubPointTau, time: timeAcc, boughtVars: 0 };
      best = best.tauH >= cur.tauH ? best : cur;
    }
    results.push(best);
    tau += stepLen;
  }
  return { results };
}

export async function comparison(theory, sigma, rho, inputs = {}) {
  const strats = getStrats(theory, toTau(theory, rho, sigma), rho, "", "", false);
  const results = [];
  for (const s of strats) results.push(await run(theory, s, sigma, rho, undefined, inputs));
  return { results: results.sort((a, b) => b.tauH - a.tauH).map(summary) };
}

export async function pubTable(theory, strat, sigma, rho, cap, stepLen) {
  let current = cap - stepLen, lastStrat = "";
  const pubTable = [[0, 0]];
  while (current > rho - 0.00001) {
    const collector = new StepPubTableCollector(current, stepLen, cap);
    collectorCache.currentCollector = collector;
    const res = await singleSim(theory, strat, sigma, current, lastStrat, undefined);
    current -= stepLen;
    lastStrat = res.strat.split(" ")[0];
    const t = collector.timings;
    let minTime = Infinity, minTarget = Infinity;
    for (let i = t.length - 1; i > 0; i--) {
      if (current + i * stepLen > cap + 0.0000001) continue;
      if (pubTable[pubTable.length - i][0] + t[i] < minTime) {
        minTime = pubTable[pubTable.length - i][0] + t[i];
        minTarget = cap - (pubTable.length - i) * stepLen;
      }
    }
    pubTable.push([minTime, minTarget]);
  }
  collectorCache.currentCollector = noopCollector;
  return { cap, start: rho, step: stepLen, pub_table: pubTable };
}
