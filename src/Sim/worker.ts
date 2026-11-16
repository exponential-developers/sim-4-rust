/**
 * Web Worker running the Rust sim (WebAssembly), so the page stays responsive during simulations.
 * Messages are described in simWorker.ts.
 */
import jsonData from "../Data/data.json" with { type: "json" };
import { run_query_with_progress, replay_bought_vars, set_config, ported_theories } from "../../wasm/pkg/wasm";

// postMessage of a worker (the DOM typings describe the window version)
const post = (message: WorkerResponse) => (self as unknown as { postMessage(m: WorkerResponse): void }).postMessage(message);

set_config(JSON.stringify(jsonData));

/** A failure response for an error thrown while simulating (a crash of the wasm module) */
function failure(err: unknown): string {
  const message = err instanceof Error ? err.message : String(err);
  const reason = err instanceof RangeError || /memory/i.test(message)
    ? "the results are too large or the page ran out of memory"
    : message;
  return JSON.stringify({ response_type: "failure", data: `The simulation failed: ${reason}.` });
}

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;
  if (request.type === "query") {
    let response: string;
    let failed = false;
    try {
      response = run_query_with_progress(
        request.query,
        (message: string, fraction: number) => post({ type: "progress", id: request.id, message, fraction }),
        (json: string) => post({ type: "finished", id: request.id, json }),
      );
    } catch (err) {
      response = failure(err);
      failed = true;
    }
    post({ type: "result", id: request.id, response, failed });
  } else if (request.type === "bought_vars") {
    let response: string;
    try {
      response = replay_bought_vars(request.replay);
    } catch (err) {
      response = failure(err);
    }
    post({ type: "bought_vars", id: request.id, response });
  }
};

// Tells the page the module is loaded and which theories it supports
post({ type: "ready", portedTheories: JSON.parse(ported_theories()) as string[] });
