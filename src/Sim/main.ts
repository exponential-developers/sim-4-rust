import { parseQuery } from "./parse";
import { writeSimResponse } from "./write";
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

/** At most this many milliseconds between status updates (fast chains send hundreds a second) */
const STATUS_INTERVAL = 100;

/** Elapsed time and progress bar of the running simulation */
const run = {
  start: 0,
  timer: undefined as ReturnType<typeof setInterval> | undefined,
  /** Latest progress not shown yet, and when the status was last updated */
  latest: null as { message: string; fraction: number } | null,
  pending: undefined as ReturnType<typeof setTimeout> | undefined,
  shownAt: 0,
  begin() {
    this.start = performance.now();
    this.latest = null;
    this.shownAt = 0;
    progressBar.hidden = true;
    progressBar.value = 0;
    elapsedOutput.textContent = "0s";
    this.timer = setInterval(() => elapsedOutput.textContent = formatElapsed(performance.now() - this.start), 1000);
  },
  /** Shows the latest progress; the bar only appears when the share done is known */
  show() {
    this.pending = undefined;
    if (this.latest === null) return;
    const { message, fraction } = this.latest;
    this.latest = null;
    this.shownAt = performance.now();
    setStatus(message, "running");
    if (Number.isFinite(fraction)) {
      progressBar.hidden = false;
      progressBar.value = fraction;
    }
  },
  /** A progress message from the sim: shown now, or with the next update if one was just shown */
  progress(message: string, fraction: number) {
    this.latest = { message, fraction };
    if (this.pending !== undefined) return;
    const wait = STATUS_INTERVAL - (performance.now() - this.shownAt);
    if (wait <= 0) this.show();
    else this.pending = setTimeout(() => this.show(), wait);
  },
  /** Stops the clock and returns how long the run took */
  end(): string {
    clearInterval(this.timer);
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
  run.begin();
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
    // Runs in a Web Worker so the page stays responsive; progress messages are shown meanwhile
    // Results finished so far: shown if the simulation is stopped
    const partial = new PartialResults();
    const APIresponse = await runQuery(
      formatSimQuery(filteredQuery),
      (message, fraction) => run.progress(message, fraction),
      (json) => partial.add(json),
    );
    const took = run.end();
    if (APIresponse === null) {
      const response = partial.response(filteredQuery.queryType);
      if (response === null) {
        setStatus(`Sim stopped after ${took}.`);
      } else {
        // Not cached: the next "All" run must simulate the theories that were left out
        writeSimResponse(cacheFilterResponse(filteredQuery, cached, response));
        const what = filteredQuery.queryType === "single"
          ? `the best of the ${partial.count} strats finished`
          : `the ${partial.count} result${partial.count === 1 ? "" : "s"} finished`;
        setStatus(`Sim stopped after ${took}: showing ${what}.`);
      }
    }
    else {
      const parsed_response = JSON.parse(APIresponse) as API_response;
      if (parsed_response.response_type == "failure") {
        throw new Error(parsed_response.data);
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