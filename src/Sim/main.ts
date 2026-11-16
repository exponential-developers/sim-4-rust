import { parseQuery } from "./parse";
import { writeLiveResults, writeSimResponse } from "./write";
import { describeProgress, splitDetail } from "./progressText";
import { setSimState } from "../UI/simState";
import { qs, event } from "../Utils/DOMhelpers";

import { runQuery, stopQuery, getPortedTheories } from "./simWorker";
import { loadSaveFile } from "../UI/buttonEvents";
import { cacheFilterQuery, cacheFilterResponse, setCache } from "./cache";
import { PartialResults } from "./partial";
import jsonData from "../Data/data.json" with { type: "json" };
import { formatElapsed } from "../Utils/helpers";

const output = qs(".output");
const progressBar = qs<HTMLProgressElement>(".simProgress");
const elapsedOutput = qs(".simElapsed");

//Buttons
const simulateButton = qs(".simulate");
const modeSelector = qs<HTMLSelectElement>(".mode");
const simAllInputArea = qs<HTMLTextAreaElement>(".simAllInputArea");

export const global = {
  stratFilter: true,
  simulating: false,
};

/**
 * Formats a sim query to a JSON string the wasm module can parse
 * @param query SimQuery
 * @returns JSON string
 */
function formatSimQuery(query: SimQuery): string {
    return JSON.stringify({
        "type": query.queryType,
        "data": query
    })
}
/** Converts the wasm module's `{ type, data }` response to the page's SimResponse */
function formatSimResponse(response: WasmSimResponse): SimResponse {
  switch (response.type) {
    case "single": return { responseType: "single", ...response.data };
    case "chain": return { responseType: "chain", ...response.data };
    case "step": return { responseType: "step", ...response.data };
    case "all": return { responseType: "all", ...response.data };
    case "pub_table": return { responseType: "pub_table", ...response.data };
  }
}

/** Sets the status line next to Simulate; the state drives its styling (spinner, error color) */
function setStatus(text: string, state: "running" | "error" | "" = "") {
  output.textContent = text;
  output.dataset.state = state;
}

/** At most this many milliseconds between progress bar updates (fast runs send hundreds a second) */
const BAR_INTERVAL = 100;
/** At most this many milliseconds between status text updates, so it can be read */
const TEXT_INTERVAL = 500;
/** The strat being simulated is shown once the rest of the message stayed the same this long */
const DETAIL_AFTER = 500;
/** How often finished results are added to the table while simulating */
const LIVE_INTERVAL = 1000;

/** Status, progress bar, elapsed time and results shown while a simulation runs */
const run = {
  start: 0,
  timers: [] as ReturnType<typeof setInterval>[],
  /** Latest progress not shown yet */
  latest: null as { message: string; fraction: number } | null,
  pending: undefined as ReturnType<typeof setTimeout> | undefined,
  barAt: 0,
  textAt: 0,
  /** Message without the strat being simulated, and since when it is the same */
  outer: "",
  outerSince: 0,
  fraction: NaN,
  /** Results finished so far, and whether some were already put in the table */
  partial: new PartialResults(),
  live: false,
  queryType: "single" as SimQuery["queryType"],
  begin(queryType: SimQuery["queryType"], partial: PartialResults) {
    Object.assign(this, { start: performance.now(), latest: null, barAt: 0, textAt: 0, outer: "", fraction: NaN, partial, live: false, queryType });
    progressBar.hidden = true;
    progressBar.value = 0;
    this.updateClock();
    this.timers = [
      setInterval(() => this.updateClock(), 1000),
      setInterval(() => this.addLiveResults(), LIVE_INTERVAL),
    ];
  },
  /** Elapsed time and share done */
  updateClock() {
    elapsedOutput.textContent = describeProgress(performance.now() - this.start, this.fraction);
    // With the share done the text is longer: a fixed width keeps the status after it in place
    elapsedOutput.toggleAttribute("data-wide", Number.isFinite(this.fraction));
  },
  /** Adds the results finished since the last call to the table */
  addLiveResults() {
    const parts = this.partial.takeNew();
    if (parts.length === 0) return;
    writeLiveResults(this.queryType, parts, !this.live);
    this.live = true;
  },
  /** Shows the latest progress: the bar right away, the text at most every TEXT_INTERVAL */
  show() {
    this.pending = undefined;
    const latest = this.latest;
    if (latest === null) return;
    const now = performance.now();
    this.barAt = now;
    if (Number.isFinite(latest.fraction)) {
      progressBar.hidden = false;
      progressBar.value = latest.fraction;
      this.fraction = latest.fraction;
    }
    const { outer, detail } = splitDetail(latest.message);
    if (outer !== this.outer) {
      this.outer = outer;
      this.outerSince = now;
    }
    const textWait = TEXT_INTERVAL - (now - this.textAt);
    if (textWait > 0) {
      this.pending = setTimeout(() => this.show(), textWait);
      return;
    }
    this.textAt = now;
    // In fast runs the strat changes too quickly to read: show it once the rest stays put
    const steady = now - this.outerSince >= DETAIL_AFTER;
    setStatus(detail !== null && steady ? latest.message : outer, "running");
    if (detail !== null && !steady) this.pending = setTimeout(() => this.show(), DETAIL_AFTER - (now - this.outerSince));
    else this.latest = null;
  },
  /** A progress message from the sim */
  progress(message: string, fraction: number) {
    this.latest = { message, fraction };
    if (this.pending !== undefined) return;
    this.pending = setTimeout(() => this.show(), Math.max(0, BAR_INTERVAL - (performance.now() - this.barAt)));
  },
  /** Stops the clock and returns how long the run took */
  end(): string {
    this.timers.forEach(clearInterval);
    this.timers = [];
    clearTimeout(this.pending);
    this.pending = undefined;
    this.latest = null;
    progressBar.hidden = true;
    elapsedOutput.textContent = "";
    return formatElapsed(performance.now() - this.start);
  },
};

export async function simCall() {
  if (global.simulating) {
    // Stops the worker; the pending runQuery below then resolves with null
    stopQuery();
    return;
  }

  global.simulating = true;
  setStatus("Simulating...", "running");
  simulateButton.textContent = "Stop simulating";

  try {
    // A game save pasted in the "All" input is loaded first (like sim-3)
    if (modeSelector.value === "All" && /^[A-Za-z0-9+/=]{20,}$/.test(simAllInputArea.value.trim())) {
      setStatus("Loading save...", "running");
      await loadSaveFile();
    }
    const query = parseQuery();
    // "All" mode: theories unchanged since the last run reuse their previous result
    const { query: filteredQuery, cached } = cacheFilterQuery(query);
    // Runs in a Web Worker so the page stays responsive; progress messages are shown meanwhile.
    // Finished results are added to the table while it runs, and kept if it is stopped or fails.
    const partial = new PartialResults();
    run.begin(filteredQuery.queryType, partial);
    const APIresponse = await runQuery(
      formatSimQuery(filteredQuery),
      (message, fraction) => run.progress(message, fraction),
      (json) => partial.add(json),
    );
    const took = run.end();
    /** Shows the results finished before a stop or a failure; returns what was shown */
    const showFinished = (): string | null => {
      const response = partial.response(filteredQuery.queryType);
      if (response === null) return null;
      // Not cached: the next "All" run must simulate the theories that were left out
      writeSimResponse(cacheFilterResponse(filteredQuery, cached, response));
      return filteredQuery.queryType === "single"
        ? `the best of the ${partial.count} strats finished`
        : `the ${partial.count} result${partial.count === 1 ? "" : "s"} finished`;
    };
    if (APIresponse === null) {
      const shown = showFinished();
      setStatus(shown === null ? `Sim stopped after ${took}.` : `Sim stopped after ${took}: showing ${shown}.`);
    }
    else {
      const parsed_response = JSON.parse(APIresponse) as API_response;
      if (parsed_response.response_type == "failure") {
        const shown = showFinished();
        throw new Error(shown === null ? parsed_response.data : `${parsed_response.data} Showing ${shown}.`);
      }
      const response = cacheFilterResponse(filteredQuery, cached, formatSimResponse(parsed_response.data));
      setCache(query, response);
      setStatus(`Done in ${took}.`);
      writeSimResponse(response);
    }
  }
  catch (err) {
    run.end();
    setStatus(err instanceof Error ? err.message : String(err), "error");
  }

  global.simulating = false;
  simulateButton.textContent = "Simulate";
  setSimState();
}

// Handlers return nothing: simCall shows its own errors in the status line
event(simulateButton, "click", () => void simCall());

// "Test all supported theories" (All mode): sigma 500 and 500 for every theory the Rust sim supports
const testPortedButton = qs<HTMLButtonElement>(".testPorted");

async function testPortedTheories() {
  if (global.simulating) return;
  const ported = await getPortedTheories();
  // "All" input order: sigma, then every theory in data.json order (0 = don't simulate)
  const theories = Object.keys(jsonData.theories);
  const lastPorted = Math.max(...ported.map((t) => theories.indexOf(t)));
  const values = theories.slice(0, lastPorted + 1).map((t) => (ported.includes(t) ? "500" : "0"));
  // The test values are only used for this run: the user's own input is put back afterwards and
  // never replaced in the saved inputs (setting .value from code fires no input event).
  const userInput = simAllInputArea.value;
  simAllInputArea.value = ["500", ...values].join(" ");
  try {
    await simCall();
  } finally {
    simAllInputArea.value = userInput;
  }
}

event(testPortedButton, "click", () => void testPortedTheories());