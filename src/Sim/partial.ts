/**
 * Results a simulation has finished so far. The sim sends each finished part of its response
 * while it runs (Rust `Progress::finished`); when a simulation is stopped, they are put together
 * into a response of the usual shape, so what was finished can still be shown.
 */

/** A finished part, as sent by the sim (see the `finished` calls in wasm/src/sim/mod.rs) */
type FinishedPart =
  | { kind: "result"; result: simResult }
  | { kind: "pub"; result: simResult; delta_tau: number; average_rate: number; total_time: number }
  | {
      kind: "all_result";
      result: simAllResult;
      sigma: number;
      strat_type: SettingsSimAllStratsMode;
      completed_cts: SettingsCompletedCTsMode;
    }
  | { kind: "pub_table_row"; row: [number, number]; cap: number; start: number; step: number };

export class PartialResults {
  private parts: FinishedPart[] = [];

  /** Adds a part sent by the sim (JSON) */
  add(json: string) {
    this.parts.push(JSON.parse(json) as FinishedPart);
  }

  /** How many results were finished */
  get count(): number {
    return this.parts.length;
  }

  /**
   * The finished results as a response for `queryType` (the mode that was running), or null
   * when nothing was finished
   */
  response(queryType: SimQuery["queryType"]): SimResponse | null {
    const parts = this.parts;
    if (parts.length === 0) return null;
    const results = parts.flatMap((p) => (p.kind === "result" || p.kind === "pub" ? [p.result] : []));
    switch (queryType) {
      case "single": {
        // The best strat so far; on ties the first one, like the sim
        let best: simResult | null = null;
        for (const r of results) if (r.tau_h > (best?.tau_h ?? 0)) best = r;
        return best ? { responseType: "single", result: best } : null;
      }
      case "comparison":
        // Highest tau/h first, like the sim (a stable sort)
        return { responseType: "step", results: [...results].sort((a, b) => b.tau_h - a.tau_h) };
      case "step":
      case "step_chain":
        return { responseType: "step", results };
      case "chain":
      case "amount":
      case "time": {
        const last = parts[parts.length - 1];
        if (last.kind !== "pub") return null;
        return {
          responseType: "chain",
          results,
          delta_tau: last.delta_tau,
          average_rate: last.average_rate,
          total_time: last.total_time,
        };
      }
      case "all": {
        const all = parts.flatMap((p) => (p.kind === "all_result" ? [p] : []));
        if (all.length === 0) return null;
        const { sigma, strat_type, completed_cts } = all[0];
        return { responseType: "all", sigma, strat_type, completed_cts, results: all.map((p) => p.result) };
      }
      case "pub_table": {
        const rows = parts.flatMap((p) => (p.kind === "pub_table_row" ? [p] : []));
        if (rows.length === 0) return null;
        const { cap, start, step } = rows[0];
        // The first row is the cap itself (reached in 0 time), as in the full table
        return { responseType: "pub_table", cap, start, step, pub_table: [[0, 0], ...rows.map((r) => r.row)] };
      }
    }
  }
}
