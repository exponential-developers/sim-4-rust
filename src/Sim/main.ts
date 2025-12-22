import { parseQuery } from "./parse";
import { simulate } from "./simulate";
import { writeSimResponse } from "./write";
import { setSimState } from "../UI/simState";
import { qs, event } from "../Utils/DOMhelpers";

import jsonData from "../Data/data.json" assert { type: "json" };
import init, { main, set_config } from "../../wasm/pkg/wasm";

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
// Removing this any is a bit annoying, will do it later -Mathis
function formatSimResponse(response: any): SimResponse {
    return {
      "responseType": response.type,
      ...response.data
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
    const APIresponse = main(formatSimQuery(query));
    const parsed_response: API_response = JSON.parse(APIresponse)
    if (parsed_response.response_type == "failure") {
      throw parsed_response.data
    }
    else {
      writeSimResponse(formatSimResponse(parsed_response.data));
      output.textContent = "";
    }
  }
  catch (err) {
    output.textContent = global.simulating ? String(err) : "Sim stopped.";
  }
  
  global.simulating = false;
  simulateButton.textContent = "Simulate";
  setSimState();
}

event(simulateButton, "click", simCall);

console.log(set_config(JSON.stringify(jsonData)));