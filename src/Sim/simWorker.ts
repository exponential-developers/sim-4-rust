/**
 * Runs sim queries in a Web Worker (worker.ts) so long simulations don't freeze the page.
 * Stopping a simulation terminates the worker and starts a fresh one; so does a crash.
 */

let worker: Worker;
let ready: Promise<string[]>;
let nextId = 0;
/** Receives a status text and the share of the work done (0 to 1, NaN when unknown) */
type ProgressHandler = (message: string, fraction: number) => void;

/** Receives each finished part of the response (JSON) while the simulation runs */
type FinishedHandler = (json: string) => void;

let pending: {
  id: number;
  resolve: (response: string | null) => void;
  onProgress: ProgressHandler;
  onFinished: FinishedHandler;
} | null = null;

/**
 * Variable purchases asked for, by request id. They are sent again to a new worker if the worker
 * is replaced before answering (a stopped simulation), since a result can always be replayed.
 */
const boughtVarsRequests = new Map<number, { replay: string; resolve: (response: string) => void }>();

const failure = (message: string) => JSON.stringify({ response_type: "failure", data: message });

/** Ends the running query with a failure and starts a new worker (after a crash) */
function crashed(message: string) {
  const current = pending;
  pending = null;
  current?.resolve(failure(message));
  worker.terminate();
  startWorker();
}

function startWorker() {
  worker = new Worker(new URL("./worker.ts", import.meta.url));
  ready = new Promise((resolve) => {
    worker.addEventListener("message", (event: MessageEvent<WorkerResponse>) => {
      const msg = event.data;
      if (msg.type === "ready") resolve(msg.portedTheories);
      else if (msg.type === "bought_vars") {
        boughtVarsRequests.get(msg.id)?.resolve(msg.response);
        boughtVarsRequests.delete(msg.id);
      }
      else if (pending && msg.id === pending.id) {
        if (msg.type === "progress") pending.onProgress(msg.message, msg.fraction);
        else if (msg.type === "finished") pending.onFinished(msg.json);
        else if (msg.type === "result") {
          const { resolve: done } = pending;
          pending = null;
          done(msg.response);
          // The WebAssembly module may be broken after a failure: start over with a new one
          if (msg.failed) {
            worker.terminate();
            startWorker();
          }
        }
      }
    });
  });
  // Errors the worker couldn't catch itself (e.g. out of memory)
  worker.addEventListener("error", (event) => {
    event.preventDefault();
    crashed(`The simulation failed${event.message ? `: ${event.message}` : ""}. It may have run out of memory.`);
  });
  // Purchases still waiting for an answer from the previous worker
  for (const [id, { replay }] of boughtVarsRequests) {
    worker.postMessage({ type: "bought_vars", id, replay } satisfies WorkerRequest);
  }
}

startWorker();

/**
 * Runs a JSON sim query in the worker.
 * Resolves with the JSON response, or null if the simulation was stopped (then `onFinished` has
 * received the results finished until then).
 */
export async function runQuery(query: string, onProgress: ProgressHandler, onFinished: FinishedHandler): Promise<string | null> {
  await ready;
  return new Promise((resolve) => {
    pending = { id: ++nextId, resolve, onProgress, onFinished };
    worker.postMessage({ type: "query", id: pending.id, query } satisfies WorkerRequest);
  });
}

/**
 * The variable purchases of a result, simulated again from its `replay` (JSON). Waits for a
 * running simulation to finish first. Resolves with the JSON response of the wasm module.
 */
export async function requestBoughtVars(replay: string): Promise<string> {
  await ready;
  return new Promise((resolve) => {
    const id = ++nextId;
    boughtVarsRequests.set(id, { replay, resolve });
    worker.postMessage({ type: "bought_vars", id, replay } satisfies WorkerRequest);
  });
}

/** Stops the running simulation, if any */
export function stopQuery() {
  if (!pending) return;
  worker.terminate();
  const { resolve } = pending;
  pending = null;
  resolve(null);
  startWorker();
}

/** Names of the theories supported by the Rust sim (e.g. ["T1", "T2"]) */
export function getPortedTheories(): Promise<string[]> {
  return ready;
}
