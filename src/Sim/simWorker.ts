/**
 * Runs sim queries in a Web Worker (worker.ts) so long simulations don't freeze the page.
 * Stopping a simulation terminates the worker and starts a fresh one.
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

function startWorker() {
  worker = new Worker(new URL("./worker.ts", import.meta.url));
  ready = new Promise((resolve) => {
    worker.addEventListener("message", (event: MessageEvent<WorkerResponse>) => {
      const msg = event.data;
      if (msg.type === "ready") resolve(msg.portedTheories);
      else if (pending && msg.id === pending.id) {
        if (msg.type === "progress") pending.onProgress(msg.message, msg.fraction);
        else if (msg.type === "finished") pending.onFinished(msg.json);
        else if (msg.type === "result") {
          const { resolve: done } = pending;
          pending = null;
          done(msg.response);
        }
      }
    });
  });
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
