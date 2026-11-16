import { beforeEach, describe, expect, it } from "vitest";
import { parseQuery } from "./parse";
import { getR9multiplier } from "../Utils/helpers";
import jsonData from "../Data/data.json" assert { type: "json" };

const q = <T extends HTMLElement>(selector: string) => document.querySelector(selector) as T;

/** Selects a value in a <select>, adding the option if the page hasn't populated it */
function select(selector: string, value: string) {
  const el = q<HTMLSelectElement>(selector);
  if (!Array.from(el.options).some((o) => o.value === value)) el.add(new Option(value, value));
  el.value = value;
}

function inputs(values: { mode: string; theory?: string; strat?: string; sigma?: string; input?: string; cap?: string; extra?: string; hardCap?: boolean }) {
  select(".mode", values.mode);
  select(".theory", values.theory ?? "T1");
  select(".strat", values.strat ?? "Best Overall");
  q<HTMLInputElement>(".sigma").value = values.sigma ?? "90";
  q<HTMLInputElement>(".input").value = values.input ?? "e150";
  q<HTMLInputElement>(".cap").value = values.cap ?? "e200";
  q<HTMLTextAreaElement>(".modeInput").value = values.extra ?? "";
  q<HTMLInputElement>(".hardCap").checked = values.hardCap ?? false;
}

beforeEach(() => {
  q<HTMLTextAreaElement>(".simAllInputArea").value = "";
});

describe("single theory modes", () => {
  it("parses a single sim", () => {
    inputs({ mode: "Single sim", strat: "T1Ratio" });
    const query = parseQuery();
    expect(query).toMatchObject({ queryType: "single", theory: "T1", strat: "T1Ratio", sigma: 90, rho: 150 });
    // settings come from the settings dialog's defaults
    expect(query.settings).toMatchObject({ dt: 1.5, ddt: 1.0001, bought_vars_delta: 5, sim_all_strats: "all" });
  });

  it("accepts rho, tau and multiplier inputs in several formats", () => {
    const rho = (input: string, sigma = "90") => {
      inputs({ mode: "Single sim", input, sigma });
      const query = parseQuery();
      return query.queryType === "single" ? query.rho : NaN;
    };
    expect(rho("e150")).toBe(150);
    expect(rho("150")).toBe(150);
    expect(rho("1.5e150")).toBeCloseTo(150 + Math.log10(1.5), 12);
    expect(rho("150r")).toBe(150);
    expect(rho("150t")).toBe(150); // T1: tau = rho
    // multiplier -> rho, with sim-3's formula (100 students: R9 bonus)
    expect(rho("25m", "100")).toBe((25 + Math.log10(3) - getR9multiplier(100)) / 0.164);
    expect(() => rho("abc")).toThrow(/Invalid currency value/);
  });

  it("requires a valid sigma for main theories", () => {
    inputs({ mode: "Single sim", sigma: "lots" });
    expect(() => parseQuery()).toThrow(/Invalid sigma/);
  });

  it("parses chain, steps and step chain inputs", () => {
    inputs({ mode: "Chain", hardCap: true });
    expect(parseQuery()).toMatchObject({ queryType: "chain", rho: 150, cap: 200, hard_cap: true });
    inputs({ mode: "Steps", extra: "e5" });
    expect(parseQuery()).toMatchObject({ queryType: "step", rho: 150, cap: 200, step: 5 });
    inputs({ mode: "StepChain", extra: "10", hardCap: false });
    expect(parseQuery()).toMatchObject({ queryType: "step_chain", step: 10, hard_cap: false });
  });

  it("parses amount and time modes", () => {
    inputs({ mode: "Amount", extra: "12" });
    expect(parseQuery()).toMatchObject({ queryType: "amount", amount: 12 });
    inputs({ mode: "Amount", extra: "0" });
    expect(() => parseQuery()).toThrow(/amount of pubs/);

    inputs({ mode: "Time", extra: "1d12h30m" });
    expect(parseQuery()).toMatchObject({ queryType: "time", time: 86400 + 12 * 3600 + 30 * 60 });
    inputs({ mode: "Time", extra: "2y" });
    expect(parseQuery()).toMatchObject({ time: 2 * 365 * 86400 });
    inputs({ mode: "Time", extra: "soon" });
    expect(() => parseQuery()).toThrow(/total time/);
  });

  it("parses comparison and pub table modes", () => {
    inputs({ mode: "Comparison" });
    const comparison = parseQuery();
    expect(comparison).toMatchObject({ queryType: "comparison", theory: "T1", rho: 150 });
    expect(comparison).not.toHaveProperty("strat");
    inputs({ mode: "Pub Table", extra: "2" });
    expect(parseQuery()).toMatchObject({ queryType: "pub_table", rho: 150, cap: 200, step: 2 });
  });

  it("rejects modes it doesn't know", () => {
    inputs({ mode: "Time diff." });
    expect(() => parseQuery()).toThrow(/not supported/);
  });
});

describe("All mode", () => {
  const all = (text: string) => {
    select(".mode", "All");
    q<HTMLTextAreaElement>(".simAllInputArea").value = text;
    return parseQuery();
  };

  it("parses sigma and one value per theory (tau by default)", () => {
    // first number is sigma, then T1, T2, T3
    expect(all("500 500 0 500")).toMatchObject({ queryType: "all", sigma: 500, values: [500, 0, 500] });
    expect(all("90 1e150r")).toMatchObject({ values: [150] });
  });

  it("requires sigma and at least one theory", () => {
    expect(() => all("500")).toThrow(/at least one theory/);
    expect(() => all("500 0 0")).toThrow(/at least one theory/);
    expect(() => all("x 500")).toThrow(/Invalid sigma/);
  });

  it("names the first value that doesn't match a theory", () => {
    const theoryCount = Object.keys(jsonData.theories).length;
    const values = Array.from({ length: theoryCount + 2 }, (_, i) => String(i + 1));
    expect(() => all(`500 ${values.join(" ")}`)).toThrow(new RegExp(`Invalid value ${theoryCount + 1} `));
  });

  it("sends each theory's own inputs", () => {
    localStorage.setItem("theoryInputs", JSON.stringify({ MF: { depth: "3" } }));
    const query = all("500 500");
    expect(query.queryType === "all" && query.theory_inputs).toEqual({ MF: { depth: "3" }, NLI: {} });
  });
});

describe("theory inputs", () => {
  beforeEach(() => localStorage.removeItem("theoryInputs"));

  it("sends the selected theory's inputs, with their defaults", () => {
    inputs({ mode: "Single sim", theory: "MF" });
    expect(parseQuery()).toMatchObject({ theory: "MF", theory_inputs: { depth: "0" } });
    localStorage.setItem("theoryInputs", JSON.stringify({ MF: { depth: "2" } }));
    expect(parseQuery()).toMatchObject({ theory_inputs: { depth: "2" } });
    inputs({ mode: "Chain", theory: "T1" });
    expect(parseQuery()).toMatchObject({ theory_inputs: {} });
  });

  it("checks values like sim-3", () => {
    inputs({ mode: "Single sim", theory: "NLI", input: "e150" });
    localStorage.setItem("theoryInputs", JSON.stringify({ NLI: { lifetime_alpha: "e300" } }));
    expect(parseQuery()).toMatchObject({ theory_inputs: { lifetime_alpha: "e300" } });
    localStorage.setItem("theoryInputs", JSON.stringify({ NLI: { lifetime_alpha: "lots" } }));
    expect(() => parseQuery()).toThrow(/Bad input for NLI - Lifetime α/);
    localStorage.setItem("theoryInputs", JSON.stringify({ NLI: { lifetime_alpha: "" } }));
    expect(parseQuery()).toMatchObject({ theory_inputs: {} });
  });

  it("takes tau values for NLI, which has no rho", () => {
    const tau = (input: string) => {
      inputs({ mode: "Single sim", theory: "NLI", input });
      const query = parseQuery();
      return query.queryType === "single" ? query.rho : NaN;
    };
    expect(tau("e150")).toBe(150);
    expect(tau("150t")).toBe(150);
    expect(tau("30m")).toBe(30 / 0.2);
    expect(() => tau("150r")).toThrow(/Cannot use rho/);
  });
});
