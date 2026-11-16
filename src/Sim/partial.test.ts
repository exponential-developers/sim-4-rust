import { describe, expect, it } from "vitest";
import { PartialResults } from "./partial";

function result(strat: string, tauH: number): simResult {
  return {
    theory: "T1", sigma: 100, last_pub: 100, pub_rho: 110, delta_tau: 10, pub_multi: 2,
    strat, tau_h: tauH, time: 60, bought_vars: [],
  };
}

function partial(...parts: object[]): PartialResults {
  const p = new PartialResults();
  for (const part of parts) p.add(JSON.stringify(part));
  return p;
}

describe("results finished before a simulation is stopped", () => {
  it("has nothing to show before the first result", () => {
    expect(new PartialResults().response("comparison")).toBeNull();
  });

  it("keeps the best strat of a single sim, the first one on ties", () => {
    const p = partial(
      { kind: "result", result: result("a", 1) },
      { kind: "result", result: result("b", 3) },
      { kind: "result", result: result("c", 3) },
    );
    expect(p.response("single")).toMatchObject({ responseType: "single", result: { strat: "b" } });
    expect(p.count).toBe(3);
  });

  it("sorts comparison results by tau/h and keeps steps in order", () => {
    const p = partial(
      { kind: "result", result: result("a", 1) },
      { kind: "result", result: result("b", 3) },
      { kind: "result", result: result("c", 2) },
    );
    const strats = (r: SimResponse | null) => (r?.responseType === "step" ? r.results.map((x) => x.strat) : []);
    expect(strats(p.response("comparison"))).toEqual(["b", "c", "a"]);
    expect(strats(p.response("step"))).toEqual(["a", "b", "c"]);
  });

  it("gives chains the totals of the last finished publication", () => {
    const p = partial(
      { kind: "pub", result: result("a", 1), delta_tau: 4, average_rate: 2, total_time: 7200 },
      { kind: "pub", result: result("b", 1), delta_tau: 9, average_rate: 3, total_time: 10800 },
    );
    expect(p.response("chain")).toMatchObject({
      responseType: "chain", delta_tau: 9, average_rate: 3, total_time: 10800,
      results: [{ strat: "a" }, { strat: "b" }],
    });
  });

  it("lists the finished theories of an All run and the rows of a pub table", () => {
    const allResult: simAllResult = { theory: "T1", ratio: 1, last_pub: 100, active: result("a", 1), idle: result("b", 1) };
    const all = partial({ kind: "all_result", result: allResult, sigma: 500, strat_type: "all", completed_cts: "no" });
    expect(all.response("all")).toMatchObject({ responseType: "all", sigma: 500, results: [{ theory: "T1" }] });

    const table = partial(
      { kind: "pub_table_row", row: [100, 150], cap: 150, start: 100, step: 5 },
      { kind: "pub_table_row", row: [250, 150], cap: 150, start: 100, step: 5 },
    );
    expect(table.response("pub_table")).toEqual({
      responseType: "pub_table", cap: 150, start: 100, step: 5, pub_table: [[0, 0], [100, 150], [250, 150]],
    });
  });
});
