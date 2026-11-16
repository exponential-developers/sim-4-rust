import jsonData from "../Data/data.json" with { type: "json" };

/** Returns the index of the first occurrence of `val` in `arr`, or -1 if not found */
export function findIndex<T>(arr: T[], val: T): number {
  for (let i = 0; i < arr.length; i++) if (val === arr[i]) return i;
  return -1;
}
/** Sleeps the given number of seconds */
export function sleep(time = 0) {
  return new Promise((resolve) => setTimeout(resolve, time));
}

/** Alias of Math.log10 */
export const l10 = Math.log10;
/** Alias of Math.log2 */
export const l2 = Math.log2;

/** Returns the index matching the theory */
export function getIndexFromTheory(theory: string): number {
  return findIndex(Object.keys(jsonData.theories), theory);
}
/** Returns the theory matching the index */
export function getTheoryFromIndex(index: number): theoryType {
  return Object.keys(jsonData.theories)[index] as theoryType;
}

/** Parses a value of the form x.xex to a log10 value */
export function parseLog10String(num: string): number {
  const split = num.split("e");
  const result = Number(split[1]) + l10(Math.max(1, Number(split[0])));
  return result;
}

/** Formats a log10 value as a string */
export function logToExp(num: number, dec = 3): string {
  const wholePart = Math.floor(num);
  const fractionalPart = num - wholePart;
  const frac1 = round(10 ** fractionalPart, dec);
  return (frac1 >= 10 ? frac1 / 10 : frac1) + "e" + (frac1 >= 10 ? wholePart + 1 : wholePart);
}
/** Returns a time string for the given number of seconds */
export function convertTime(secs: number): string {
  const mins = Math.floor((secs / 60) % 60);
  const hrs = Math.floor((secs / 3600) % 24);
  const days = Math.floor((secs / 86400) % 365);
  const years = Math.floor(secs / 31536000);
  let result = "";
  if (years > 0) {
    result += years < 1e6 ? years : logToExp(l10(years));
    result += "y";
  }
  if (days > 0) result += days + "d";
  result += (hrs < 10 ? "0" : "") + hrs + "h";
  if (years === 0) result += (mins < 10 ? "0" : "") + mins + "m";
  return result;
}

/** 
 * Formats a number to the given precision (default 6 digits)
 * 
 * This function removes the `+` in the scientific form
 * */
export function formatNumber(value: number, precision = 6): string {
  return value.toPrecision(precision).replace(/[+]/, "");
}

/** Rounds `number` to `decimals` decimals */
export function round(number: number, decimals: number): number {
  return Math.round(number * 10 ** decimals) / 10 ** decimals;
}

/** Converts an element to a function with no parameter returning this element, if it is not already a function */
export function toCallable<T>(val: T | (() => T)): () => T {
  return typeof val === "function" ? (val as () => T) : () => val;
}
/** Converts all elements in an array to a function with no parameter returning this element, if it is not already a function */
export function toCallables<T>(arr: (T | (() => T))[]): (() => T)[] {
  return arr.map((val) => toCallable(val));
}

export function add_old(value1: number, value2: number): number {
  const max = value1 > value2 ? value1 : value2;
  const min = value1 > value2 ? value2 : value1;
  const wholePart1 = Math.floor(max);
  const fractionalPart1 = 10 ** (max - wholePart1);
  const wholePart2 = Math.floor(min);
  const fractionalPart2 = 10 ** (min - wholePart2);
  return wholePart1 + l10(fractionalPart1 + fractionalPart2 / 10 ** (wholePart1 - wholePart2));
}

/** Adds two log10 values 
 * @returns the result as a log10 value
*/
export function add2(value1: number, value2: number): number {
  const max = value1 > value2 ? value1 : value2;
  const min = value1 > value2 ? value2 : value1;
  return max != -Infinity ? max + l10(1 + 10**(min-max)) : max;
}

/** Adds multiple log10 values 
 * @returns the result as a log10 value
*/
export function add(...values: number[]): number {
  if (values.length === 0) return -Infinity;
  if (values.length === 1) return values[0];
  if (values.length === 2) return add2(values[0], values[1]);
  let sum = add2(values[0], values[1]);
  for (let i = 2; i < values.length; i++) {
    sum = add2(sum, values[i]);
  }
  return sum;
}

/** Subtracts two log10 values
 * @returns the result as a log10 value
 */
export function subtract(value1: number, value2: number): number {
  const max = value1 > value2 ? value1 : value2;
  const min = value1 > value2 ? value2 : value1;
  return max != -Infinity ? max + l10(1 - 10**(min-max)) : max;
}

/**
 * Returns the index `target` would have if inserted in `arr`
 * 
 * This is equivalent to count how many elements in `arr` are strictly lower than `target`
 * @param arr Number array sorted in increasing order with no repetitions
 * @param target Target number
 */
export function binaryInsertionSearch(arr: number[], target: number): number {
  if (target < arr[0]) return 0;
  let l = 0;
  let r = arr.length - 1;
  while (l < r) {
    const m = Math.ceil((l + r) / 2);
    if (arr[m] <= target) l = m;
    else r = m - 1;
  }
  return l + 1;
}

/** Returns a default simResult */
export function defaultResult(): simResult {
  return {
      theory: "T1",
      sigma: 0,
      last_pub: 0,
      pub_rho: 0,
      delta_tau: 0,
      pub_multi: 0,
      strat: "Result undefined",
      tau_h: 0,
      time: 0,
      bought_vars: []
    };
}

/**
 * Returns the result with the highest tau/hr.
 * 
 * Null values are replaced with a default result of tau/hr = 0.
 * 
 * If tau/hr are identical, the first result is returned.
 */
export function getBestResult(res1: simResult | null, res2: simResult | null): simResult {
  if (res1 == null) res1 = defaultResult();
  if (res2 == null) res2 = defaultResult();
  return res1.tau_h >= res2.tau_h ? res1 : res2;
}

/**
 * Returns the last bought level of a variable in a variable buy list
 * @param variable The variable name to search
 * @param arr The variable buy list
 * @returns The last level bought of the variable, or 0 if the variable was not found
 */
export function getLastLevel(variable: string, arr: varBuy[]): number {
  for (let i = arr.length - 1; i >= 0; i--) {
    if (arr[i].var_name == variable) {
      return arr[i].level;
    }
  }
  return 0;
}

/**
 * @param {number} sigma Number of students
 * @returns the R9 multiplier as a log value
*/
export function getR9multiplier(sigma: number): number {
  return l10((sigma / 20) ** (sigma < 65 ? 0 : sigma < 75 ? 1 : sigma < 85 ? 2 : 3))
}

/**
 * Returns a value of dt for the slider value.
 * @param val slider value (between 0 and 10)
 */
export function getdtFromSlider(val: number): number {
  return val == 0 ? 0.15 : val == 10 ? 5 : 0.15 + (2 ** val) * (4.85 / 2 ** 10);
}

/**
 * Returns a value of ddt for the slider value.
 * @param val slider value (between 0 and 10)
 */
export function getddtFromSlider(val: number): number {
  return val == 0 ? 1 : val == 10 ? 1.3 : round(1 + (3 ** val) * (0.3 / 3 ** 10), 7)
}

/**
 * Returns rho from the multiplier.
 * @param theory The theory to return rho for
 * @param value The multiplier as a log10 value
 * @param sigma Number of students
 * @returns 
 */
export function reverseMulti(theory: string, value: number, sigma: number) {
  const R9 = getR9multiplier(sigma);
  // Ported theories use sim-3's exact formula and operation order (traditionalConverter:
  // (value - multFactor - r9) / multExponent), so results match sim-3 bit for bit.
  const sim3Multi = (multExponent: number, multFactor: number) => (value - multFactor - R9) / multExponent;
  // Custom theories: the multiplier to tau (no R9), then tau to rho (sim-3's tauToRho)
  const sim3CtMulti = (multExponent: number, multFactor: number, tauFactor: number) =>
    ((value - multFactor) / multExponent) / tauFactor;
  switch (theory) {
    case "T1":
      return sim3Multi(0.164, -l10(3));
    case "T2":
      return sim3Multi(0.198, -l10(100));
    case "T3":
      return sim3Multi(0.147, l10(3));
    case "T4":
      return sim3Multi(0.165, -l10(4));
    case "T5":
      return sim3Multi(0.159, 0);
    case "T6":
      return sim3Multi(0.196, -l10(50));
    case "T7":
      return sim3Multi(0.152, 0);
    case "T8":
      return sim3Multi(0.15, 0);
    case "WSP":
    case "SL":
      return sim3CtMulti(0.375, 0, 0.4);
    case "EF":
      return sim3CtMulti(0.09675, 0, 1.6);
    case "CSR2":
      return sim3CtMulti(0.55075, -l10(200), 0.4);
    case "FI":
      return sim3CtMulti(0.1625, 0, 0.4);
    case "FP":
      return sim3CtMulti(0.331, l10(5), 0.3);
    case "RZ":
      return sim3CtMulti(21.02 / 40, l10(2), 0.4);
    case "MF":
      return sim3CtMulti(0.17, 0, 1);
    case "BaP":
      return sim3CtMulti(0.132075, l10(5), 0.4);
    case "BT":
      return sim3CtMulti(1.25, 0, 0.4);
    case "TC":
      return sim3CtMulti(0.2 / 0.45, -l10(2), 0.45);
    case "FS": {
      const phi = (1 + Math.sqrt(5)) / 2;
      return sim3CtMulti(1 / Math.sqrt(5), 0, 1 / phi);
    }
    case "BD":
      return sim3CtMulti(0.375, 0, 0.4);
    case "ILC":
      return sim3CtMulti(0.39, -7 * l10(Math.E), 2);
    case "NLI":
      // No rho: sim-4 gives NLI a tau factor of 1
      return sim3CtMulti(0.2, 0, 1);
  }
  throw new Error(`Failed parsing multiplier. Please contact the author of the sim.`);
}

/** A duration as "42s", "3:05" or "1:02:05" (the elapsed time of a simulation) */
export function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000);
  const [h, m, s] = [Math.floor(total / 3600), Math.floor(total / 60) % 60, total % 60];
  const pad = (n: number) => String(n).padStart(2, "0");
  if (h > 0) return `${h}:${pad(m)}:${pad(s)}`;
  if (m > 0) return `${m}:${pad(s)}`;
  return `${s}s`;
}

/** Checks if a string corresponds to a main theory */
export function isMainTheory(theory: string): boolean {
  return /T[1-8]/.test(theory);
}