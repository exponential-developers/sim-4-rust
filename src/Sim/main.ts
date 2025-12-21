import { parseQuery } from "./parse";
import { simulate } from "./simulate";
import { writeSimResponse } from "./write";
import { setSimState } from "../UI/simState";
import { qs, event } from "../Utils/DOMhelpers";

import init, { test } from "../../wasm/pkg/wasm";

const output = qs(".output");

//Buttons
const simulateButton = qs(".simulate");

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
function formatSimResponse(response: string): SimResponse {
    let data = JSON.parse(response)
    return {
      "responseType": data.type,
      ...data.data
    }
}

async function simCall() {
  if (global.simulating) {
    global.simulating = false;
    output.textContent = "Sim stopped.";
    return;
  }

  global.simulating = true;
  output.textContent = "";
  simulateButton.textContent = "Stop simulating";

  try {
    const query = parseQuery();
    //console.log(test(formatSimQuery(query)));
    const response = await simulate(query);
    //writeSimResponse(response);
    writeSimResponse(formatSimResponse(test(formatSimQuery(query))));
    output.textContent = "";
  }
  catch (err) {
    output.textContent = global.simulating ? String(err) : "Sim stopped.";
  }
  
  global.simulating = false;
  simulateButton.textContent = "Simulate";
  setSimState();
}

event(simulateButton, "click", simCall);