// Test cases shared by the benchmark, the comparison script and the fixture generator.

const STRATS = {
  T1: ["T1", "T1Coast", "T1C34", "T1C34Coast", "T1C4", "T1C4Coast",
    "T1Ratio", "T1RatioCoast", "T1SolarXLII", "T1SolarXLIIOld", "T1SolarXLIIOldCoast"],
  T2: ["T2", "T2MC", "T2MCAlt", "T2MCAlt2", "T2MCAlt3", "T2Haxolotl", "T2MS", "T2QS", "T2ROI", "T2ROIMS"],
  T3: ["T3Play2", "T3Play", "T3Snax", "T3SnaxCoast", "T3SnaxdC12", "T3Snax2", "T3P2C23d", "T3P2C23C33d",
    "T3P2C23", "T3P2C23C33", "T3P2C23C33Coast", "T3noC11C13C21C33d", "T3noC11C13C21C33", "T3noC13C33d",
    "T3noC13C33", "T3noC11C13C33d", "T3noC11C13C33", "T3noC13C32C33d", "T3noC13C32C33", "T3C11C12C21d",
    "T3C11C12C21", "T3"],
  T4: ["T4C3dCoast", "T4C3d", "T4C3Coast", "T4C3", "T4C3dC12rcv", "T4C356dC12rcv", "T4C356dC12rcvCoast",
    "T4C456dC12rcvMS", "T4C456dC12rcvMSCoast", "T4C123d", "T4C123", "T4C12d", "T4C12", "T4C56", "T4C56Coast",
    "T4C4", "T4C4d", "T4C5", "T4"],
  T5: ["T5", "T5Idle", "T5IdleCoast", "T5Idle2Coast", "T5AI2", "T5AI2Coast", "T5AI2PT", "T5AI2PTCoast"],
};

// [log10 rho of last pub, students]: rough progression of a real account
const STRAT_CASES = {
  T1: [[25, 30], [75, 60], [150, 90], [300, 140], [500, 180], [700, 220], [1000, 280]],
  T2: [[20, 30], [60, 60], [150, 90], [250, 140], [400, 200]],
  T3: [[30, 40], [100, 70], [160, 95], [200, 110], [280, 140], [400, 180], [600, 230], [800, 280]],
  T4: [[20, 30], [60, 60], [100, 75], [150, 95], [200, 110], [260, 140], [400, 180], [700, 250]],
  T5: [[20, 30], [60, 60], [120, 85], [200, 110], [300, 150], [500, 200], [800, 270]],
};

// Very slow strats (many sims per result): checked once for accuracy, left out of timings
const SLOW = new Set(["T2Haxolotl"]);

const CATEGORIES = ["Best Overall", "Best Active", "Best Semi-Idle", "Best Idle"];

const BEST_CASES = {
  T1: [[15, 20], [25, 30], [75, 60], [150, 90], [240, 120], [300, 140], [500, 180], [700, 220], [1000, 280]],
  T2: [[10, 20], [30, 40], [100, 70], [200, 110], [260, 150], [400, 200]],
  T3: [[20, 30], [120, 80], [160, 95], [180, 100], [260, 130], [300, 150], [450, 190], [700, 250]],
  T4: [[10, 20], [30, 40], [80, 65], [130, 85], [180, 100], [230, 120], [280, 140], [400, 180], [700, 250]],
  T5: [[10, 20], [50, 50], [150, 90], [300, 150], [600, 230]],
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
  ["T3", "Best Overall", 60, 20, 180, false],
  ["T3", "Best Semi-Idle", 120, 175, 300, true],
  ["T3", "Best Active", 160, 300, 420, false],
  ["T4", "Best Overall", 60, 20, 160, false],
  ["T4", "Best Semi-Idle", 120, 150, 280, true],
  ["T4", "Best Active", 150, 220, 330, false],
  ["T5", "Best Active", 90, 50, 200, false],
  ["T5", "Best Idle", 150, 200, 350, true],
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
  ["step", "T3", "Best Active", 100, 150, 250, 25],
  ["amount", "T3", "Best Semi-Idle", 120, 200, 6],
  ["time", "T3", "T3P2C23", 150, 300, 4 * 86400],
  ["step_chain", "T3", "Best Overall", 120, 175, 260, 20, false],
  ["comparison", "T3", null, 150, 300],
  ["pub_table", "T3", "Best Semi-Idle", 150, 300, 312, 2],
  ["step", "T4", "Best Overall", 100, 100, 200, 25],
  ["amount", "T4", "Best Idle", 120, 180, 6],
  ["time", "T4", "T4C3", 160, 260, 3 * 86400],
  ["comparison", "T4", null, 140, 230],
  ["step", "T5", "Best Semi-Idle", 120, 150, 300, 50],
  ["amount", "T5", "T5AI2PT", 140, 250, 5],
  ["step_chain", "T5", "Best Active", 120, 150, 260, 30, false],
  ["comparison", "T5", null, 150, 300],
  ["pub_table", "T5", "Best Overall", 150, 300, 310, 2],
];

module.exports = { STRATS, STRAT_CASES, SLOW, CATEGORIES, BEST_CASES, CHAINS, MODES };
