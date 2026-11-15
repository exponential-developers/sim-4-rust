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
  // T6SnaxIdleRecoveryM* are sim-4's names for sim-3's idleRcvStop strat specific input
  T6: ["T6", "T6C3", "T6C4", "T6C125", "T6C12", "T6C5", "T6C5Coast", "T6Snax", "T6C125dStopC12",
    "T6C125dStopC12Coast", "T6SnaxCoast", "T6SnaxIdleRecovery", "T6SnaxIdleRecoveryCoast",
    "T6SnaxIdleRecoveryM2", "T6SnaxIdleRecoveryM5Coast", "T6C3d", "T6C4d", "T6C125d", "T6C12d", "T6C5d",
    "T6C5dCoast", "T6C5dIdleRecovery", "T6C5dIdleRecoveryCoast", "T6AI", "T6AICoast"],
  T7: ["T7", "T7Coast", "T7C12", "T7C12Coast", "T7C3", "T7C3Coast", "T7noC12", "T7noC12Coast", "T7noC123",
    "T7noC123Coast", "T7noC1234", "T7noC1234Coast", "T7C12d", "T7C12dCoast", "T7C3d", "T7C3dCoast",
    "T7PlaySpqcey", "T7PlaySpqceyCoast"],
  T8: ["T8", "T8Coast", "T8noC3", "T8noC3Coast", "T8noC5", "T8noC5Coast", "T8noC35", "T8noC35Coast",
    "T8Snax", "T8noC3d", "T8noC3dCoast", "T8noC3dSingleMS0", "T8noC3dSingleMS2", "T8noC3dSingleMS0Coast",
    "T8noC3dSingleMS2Coast", "T8noC5d", "T8noC5dCoast", "T8noC35d", "T8noC35dCoast", "T8PlayCoast",
    "T8PlaySolarswap", "T8PlaySolarswapCoast"],
};

// [log10 rho of last pub, students]: rough progression of a real account
const STRAT_CASES = {
  T1: [[25, 30], [75, 60], [150, 90], [300, 140], [500, 180], [700, 220], [1000, 280]],
  T2: [[20, 30], [60, 60], [150, 90], [250, 140], [400, 200]],
  T3: [[30, 40], [100, 70], [160, 95], [200, 110], [280, 140], [400, 180], [600, 230], [800, 280]],
  T4: [[20, 30], [60, 60], [100, 75], [150, 95], [200, 110], [260, 140], [400, 180], [700, 250]],
  T5: [[20, 30], [60, 60], [120, 85], [200, 110], [300, 150], [500, 200], [800, 270]],
  T6: [[20, 30], [60, 60], [120, 85], [200, 110], [400, 170], [800, 240]],
  T7: [[20, 30], [60, 60], [120, 85], [200, 110], [300, 150], [500, 200]],
  T8: [[10, 20], [52, 50], [60, 60], [70, 65], [120, 85], [200, 110], [300, 150], [500, 200]],
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
  T6: [[10, 20], [30, 40], [90, 70], [150, 90], [300, 150], [600, 210]],
  T7: [[10, 20], [30, 40], [90, 70], [160, 95], [260, 130], [400, 180]],
  T8: [[5, 15], [30, 40], [55, 55], [80, 65], [130, 85], [190, 105], [260, 130], [400, 180]],
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
  // T6: the chain stays under e100, where its strat conditions don't use lastStrat (see AGENTS.md)
  ["T6", "Best Overall", 60, 20, 100, false],
  ["T6", "Best Active", 90, 40, 100, false],
  ["T7", "Best Overall", 60, 20, 160, false],
  ["T7", "Best Semi-Idle", 120, 150, 300, true],
  ["T8", "Best Overall", 60, 10, 100, false],
  ["T8", "Best Active", 120, 100, 250, false],
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
  ["step", "T6", "Best Overall", 60, 20, 100, 10],
  ["amount", "T6", "Best Idle", 90, 25, 5],
  ["time", "T6", "T6C3", 60, 10, 86400],
  ["step", "T7", "Best Overall", 100, 100, 200, 25],
  ["amount", "T7", "Best Semi-Idle", 120, 180, 6],
  ["time", "T7", "T7PlaySpqcey", 160, 260, 3 * 86400],
  ["comparison", "T7", null, 140, 230],
  ["step", "T8", "Best Active", 100, 60, 150, 20],
  ["amount", "T8", "Best Overall", 80, 30, 8],
  ["comparison", "T8", null, 80, 100],
  ["pub_table", "T8", "Best Overall", 100, 130, 150, 5],
];

module.exports = { STRATS, STRAT_CASES, SLOW, CATEGORIES, BEST_CASES, CHAINS, MODES };
