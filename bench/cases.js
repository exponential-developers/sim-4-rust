// Test cases shared by the benchmark, the comparison script and the fixture generator.

const STRATS = {
  T1: ["T1", "T1Coast", "T1C34", "T1C34Coast", "T1C4", "T1C4Coast",
    "T1Ratio", "T1RatioCoast", "T1SolarXLII", "T1SolarXLIIOld", "T1SolarXLIIOldCoast"],
  T2: ["T2", "T2MC", "T2MCAlt", "T2MCAlt2", "T2MCAlt3", "T2Haxolotl", "T2MS", "T2QS", "T2ROI", "T2ROIMS"],
};

// [log10 rho of last pub, students]: rough progression of a real account
const STRAT_CASES = {
  T1: [[25, 30], [75, 60], [150, 90], [300, 140], [500, 180], [700, 220], [1000, 280]],
  T2: [[20, 30], [60, 60], [150, 90], [250, 140], [400, 200]],
};

// Very slow strats (many sims per result): checked once for accuracy, left out of timings
const SLOW = new Set(["T2Haxolotl"]);

const CATEGORIES = ["Best Overall", "Best Active", "Best Semi-Idle", "Best Idle"];

const BEST_CASES = {
  T1: [[15, 20], [25, 30], [75, 60], [150, 90], [240, 120], [300, 140], [500, 180], [700, 220], [1000, 280]],
  T2: [[10, 20], [30, 40], [100, 70], [200, 110], [260, 150], [400, 200]],
};

// [theory, strat, sigma, rho, cap, hardCap]
const CHAINS = [
  ["T1", "Best Overall", 60, 10, 150, false],
  ["T1", "Best Idle", 60, 10, 150, false],
  ["T1", "Best Semi-Idle", 90, 100, 300, true],
  ["T1", "Best Active", 140, 200, 400, false],
  ["T1", "T1Ratio", 90, 50, 200, true],
  ["T2", "Best Overall", 60, 15, 150, false],
  ["T2", "Best Semi-Idle", 90, 100, 260, false],
  ["T2", "Best Idle", 120, 200, 320, true],
];

// Other modes: [mode, theory, strat, sigma, rho, extra...]
const MODES = [
  ["step", "T1", "Best Overall", 60, 20, 80, 10],
  ["step", "T2", "Best Semi-Idle", 90, 100, 200, 25],
  ["amount", "T1", "Best Active", 90, 150, 12],
  ["amount", "T2", "Best Overall", 60, 30, 5],
  ["time", "T1", "Best Idle", 120, 250, 3 * 86400],
  ["time", "T2", "T2MC", 150, 260, 5 * 86400],
  ["step_chain", "T1", "Best Overall", 90, 100, 160, 15, false],
  ["step_chain", "T2", "Best Idle", 120, 200, 280, 20, true],
  ["comparison", "T1", null, 140, 300],
  ["comparison", "T2", null, 150, 260],
  ["pub_table", "T1", "Best Overall", 90, 150, 170, 2],
  ["pub_table", "T2", "Best Semi-Idle", 120, 200, 215, 1],
];

module.exports = { STRATS, STRAT_CASES, SLOW, CATEGORIES, BEST_CASES, CHAINS, MODES };
