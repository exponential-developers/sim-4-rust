/**
 * Web Worker running the Rust sim (WebAssembly), so the page stays responsive during simulations.
 * Messages are described in simWorker.ts.
 */
import jsonData from "../Data/data.json" assert { type: "json" };
import { run_query_with_progress, set_config, ported_theories } from "../../wasm/pkg/wasm";

// postMessage of a worker (the DOM typings describe the window version)
const post = (message: WorkerResponse) => (self as unknown as { postMessage(m: WorkerResponse): void }).postMessage(message);

set_config(JSON.stringify(jsonData));

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;
  if (request.type === "query") {
    const response = run_query_with_progress(request.query, (message: string) => post({ type: "progress", id: request.id, message }));
    post({ type: "result", id: request.id, response });
  }
};

// Tells the page the module is loaded and which theories it supports
post({ type: "ready", portedTheories: JSON.parse(ported_theories()) as string[] });
