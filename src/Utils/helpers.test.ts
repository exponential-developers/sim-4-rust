import { describe, expect, it } from "vitest";
import {
  add,
  binaryInsertionSearch,
  convertTime,
  getIndexFromTheory,
  getR9multiplier,
  getTheoryFromIndex,
  isMainTheory,
  logToExp,
  parseLog10String,
  reverseMulti,
  round,
  subtract,
} from "./helpers";

describe("number formatting and parsing", () => {
  it("formats log10 values like 3.16e2", () => {
    expect(logToExp(2.5, 2)).toBe("3.16e2");
    expect(logToExp(150, 0)).toBe("1e150");
    expect(logToExp(2.9999999, 2)).toBe("1e3"); // rounding up carries into the exponent
  });

  it("parses x.xexxx strings to log10 values", () => {
    expect(parseLog10String("2e5")).toBeCloseTo(5 + Math.log10(2), 12);
    expect(parseLog10String("0.5e3")).toBe(3); // mantissas below 1 count as 1, like sim-3
  });

  it("formats durations", () => {
    expect(convertTime(3661)).toBe("01h01m");
    expect(convertTime(86400 + 3600 + 60)).toBe("1d01h01m");
    expect(convertTime(31536000 + 2 * 86400)).toBe("1y2d00h");
  });

  it("rounds to a number of decimals", () => {
    expect(round(1.23456, 2)).toBe(1.23);
    expect(round(1.235, 1)).toBe(1.2);
  });
});

describe("log10 arithmetic", () => {
  it("adds and subtracts values given as log10", () => {
    expect(add(2, 2)).toBeCloseTo(Math.log10(200), 12);
    expect(add(3, 2, 1)).toBeCloseTo(Math.log10(1110), 12);
    expect(add()).toBe(-Infinity);
    expect(subtract(3, 2)).toBeCloseTo(Math.log10(900), 12);
  });

  it("counts elements below a target in a sorted list", () => {
    expect(binaryInsertionSearch([10, 20, 30], 5)).toBe(0);
    expect(binaryInsertionSearch([10, 20, 30], 20)).toBe(2);
    expect(binaryInsertionSearch([10, 20, 30], 99)).toBe(3);
  });
});

describe("theories", () => {
  it("maps theories to their index in data.json and back", () => {
    expect(getTheoryFromIndex(0)).toBe("T1");
    expect(getIndexFromTheory("T2")).toBe(1);
    expect(getTheoryFromIndex(getIndexFromTheory("BaP"))).toBe("BaP");
  });

  it("recognizes main theories", () => {
    expect(isMainTheory("T1") && isMainTheory("T8")).toBe(true);
    expect(isMainTheory("WSP")).toBe(false);
  });

  it("gives the R9 bonus by students", () => {
    expect(getR9multiplier(64)).toBe(0);
    expect(getR9multiplier(65)).toBeCloseTo(Math.log10(65 / 20), 12);
    expect(getR9multiplier(100)).toBeCloseTo(3 * Math.log10(5), 12);
  });
});

describe("multiplier inputs (reverseMulti)", () => {
  // sim-3's traditionalConverter: multiplier = tau * multExponent + multFactor + r9, tau = rho here
  const sim3Mult = (rho: number, multExponent: number, multFactor: number, sigma: number) =>
    Math.max(0, rho * multExponent + multFactor + getR9multiplier(sigma));

  it("inverts sim-3's T1, T2 and T3 multiplier formulas", () => {
    for (const sigma of [30, 70, 200]) {
      expect(reverseMulti("T1", sim3Mult(150, 0.164, -Math.log10(3), sigma), sigma)).toBeCloseTo(150, 9);
      expect(reverseMulti("T2", sim3Mult(250, 0.198, -Math.log10(100), sigma), sigma)).toBeCloseTo(250, 9);
      expect(reverseMulti("T3", sim3Mult(300, 0.147, Math.log10(3), sigma), sigma)).toBeCloseTo(300, 9);
    }
  });

  it("uses sim-3's operation order for ported theories, so results are bit-identical", () => {
    const value = 25.7, sigma = 90;
    const r9 = getR9multiplier(sigma);
    expect(reverseMulti("T1", value, sigma)).toBe((value - -Math.log10(3) - r9) / 0.164);
    expect(reverseMulti("T2", value, sigma)).toBe((value - -Math.log10(100) - r9) / 0.198);
    expect(reverseMulti("T3", value, sigma)).toBe((value - Math.log10(3) - r9) / 0.147);
  });

  it("rejects unknown theories", () => {
    expect(() => reverseMulti("T99", 10, 10)).toThrow();
  });
});
