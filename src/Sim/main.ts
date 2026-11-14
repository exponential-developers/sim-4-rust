import { parseQuery } from "./parse";
import { writeSimResponse } from "./write";
import { setSimState } from "../UI/simState";
import { qs, event } from "../Utils/DOMhelpers";

import { runQuery, stopQuery, getPortedTheories } from "./simWorker";
import { loadSaveFile } from "../UI/buttonEvents";
import { cacheFilterQuery, cacheFilterResponse, setCache } from "./cache";
import jsonData from "../Data/data.json" assert { type: "json" };

const output = qs(".output");

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
// Removing this any is a bit annoying, will do it later -Mathis
function formatSimResponse(response: any): SimResponse {
    return {
      "responseType": response.type,
      ...response.data
    }
}

export async function simCall() {
  if (global.simulating) {
    // Stops the worker; the pending runQuery below then resolves with null
    stopQuery();
    return;
  }

  global.simulating = true;
  output.textContent = "";
  simulateButton.textContent = "Stop simulating";

  try {
    // A game save pasted in the "All" input is loaded first (like sim-3)
    if (modeSelector.value === "All" && /^[A-Za-z0-9+/=]{20,}$/.test(simAllInputArea.value.trim())) {
      output.textContent = "Loading save...";
      await loadSaveFile();
    }
    const query = parseQuery();
    // "All" mode: theories unchanged since the last run reuse their previous result
    const { query: filteredQuery, cached } = cacheFilterQuery(query);
    // Runs in a Web Worker so the page stays responsive; progress messages are shown meanwhile
    const APIresponse = await runQuery(formatSimQuery(filteredQuery), (message) => output.textContent = message);
    if (APIresponse === null) {
      output.textContent = "Sim stopped.";
    }
    else {
      const parsed_response: API_response = JSON.parse(APIresponse)
      if (parsed_response.response_type == "failure") {
        throw parsed_response.data
      }
      const response = cacheFilterResponse(filteredQuery, cached, formatSimResponse(parsed_response.data));
      setCache(query, response);
      writeSimResponse(response);
      output.textContent = "";
    }
  }
  catch (err) {
    output.textContent = String(err);
  }

  global.simulating = false;
  simulateButton.textContent = "Simulate";
  setSimState();
}

event(simulateButton, "click", simCall);

// "Test all supported theories" (All mode): sigma 500 and 500 for every theory the Rust sim supports
const testPortedButton = qs<HTMLButtonElement>(".testPorted");

event(testPortedButton, "click", async () => {
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
});