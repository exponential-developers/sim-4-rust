import { beforeEach, describe, expect, it } from "vitest";
import { cacheFilterQuery, cacheFilterResponse, setCache } from "./cache";

const settings: Settings = {
  dt: 1.5,
  ddt: 1.0001,
  bought_vars_delta: 5,
  theme: "dark",
  sim_all_strats: "all",
  completed_cts: "no",
  show_a23: false,
  show_unofficials: false,
};

function allQuery(values: number[], overrides: Partial<SimAllQuery> = {}): SimAllQuery {
  return { queryType: "all", values, sigma: 100, very_active: false, semi_idle: true, strat_type: "all", settings, theory_inputs: {}, ...overrides };
}

function result(theory: theoryType, tauH: number): simAllResult {
  const res: simResult = {
    theory, sigma: 100, last_pub: 100, pub_rho: 110, delta_tau: 10, pub_multi: 2,
    strat: `${theory} strat`, tau_h: tauH, time: 60, bought_vars: [],
  };
  return { theory, ratio: 1, last_pub: 100, active: res, idle: res };
}

function allResponse(results: simAllResult[]): SimAllResponse {
  return { responseType: "all", sigma: 100, strat_type: "all", completed_cts: "no", results };
}

beforeEach(() => localStorage.clear());

describe("All mode cache", () => {
  it("sends the query unchanged when nothing is cached", () => {
    const query = allQuery([500, 500]);
    const { query: filtered, cached } = cacheFilterQuery(query);
    expect(filtered).toEqual(query);
    expect(cached).toEqual([]);
  });

  it("skips theories whose input didn't change and puts their previous result back", () => {
    setCache(allQuery([500, 400]), allResponse([result("T1", 1), result("T2", 2)]));

    // T1 unchanged, T2 changed
    const { query: filtered, cached } = cacheFilterQuery(allQuery([500, 450]));
    expect(filtered.queryType === "all" && filtered.values).toEqual([0, 450]); // 0 = not simulated
    expect(cached).toEqual([true, false]);

    // the sim only returns T2; T1 comes from the cache, in theory order
    const merged = cacheFilterResponse(filtered, cached, allResponse([result("T2", 3)]));
    expect(merged.responseType === "all" && merged.results.map((r) => [r.theory, r.active.tau_h])).toEqual([["T1", 1], ["T2", 3]]);
  });

  it("recomputes everything when a setting that affects results changes", () => {
    setCache(allQuery([500, 400]), allResponse([result("T1", 1), result("T2", 2)]));
    for (const change of [{ dt: 2 }, { ddt: 1.0002 }, { bought_vars_delta: 10 }, { sim_all_strats: "active" as const }]) {
      const { cached } = cacheFilterQuery(allQuery([500, 400], { settings: { ...settings, ...change } }));
      expect(cached, JSON.stringify(change)).toEqual([]);
    }
    expect(cacheFilterQuery(allQuery([500, 400], { semi_idle: false })).cached).toEqual([]);
    expect(cacheFilterQuery(allQuery([500, 400], { very_active: true })).cached).toEqual([]);
  });

  it("recomputes a theory when one of its own inputs changes", () => {
    const values: number[] = [...new Array<number>(15).fill(0), 300]; // MF only
    const response = allResponse([result("MF", 1)]);
    setCache(allQuery(values, { theory_inputs: { MF: { depth: "0" } } }), response);
    const mf = values.length - 1;
    expect(cacheFilterQuery(allQuery(values, { theory_inputs: { MF: { depth: "0" } } })).cached[mf]).toBe(true);
    expect(cacheFilterQuery(allQuery(values, { theory_inputs: { MF: { depth: "2" } } })).cached[mf]).toBe(false);
  });

  it("recomputes main theories when sigma changes", () => {
    setCache(allQuery([500, 400]), allResponse([result("T1", 1), result("T2", 2)]));
    expect(cacheFilterQuery(allQuery([500, 400], { sigma: 120 })).cached).toEqual([false, false]);
  });

  it("never caches theories that are not simulated (0)", () => {
    setCache(allQuery([0, 400]), allResponse([result("T2", 2)]));
    expect(cacheFilterQuery(allQuery([0, 400])).cached).toEqual([false, true]);
  });

  it("leaves other modes alone", () => {
    const single: SingleSimQuery = { queryType: "single", theory: "T1", strat: "T1", sigma: 10, rho: 50, settings };
    setCache(single, { responseType: "single", result: result("T1", 1).active });
    expect(localStorage.getItem("cacheQuery")).toBeNull();
    expect(cacheFilterQuery(single)).toEqual({ query: single, cached: [] });
  });
});
