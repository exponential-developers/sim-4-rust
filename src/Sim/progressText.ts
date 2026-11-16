/**
 * Texts of the status line while a simulation runs (main.ts): the progress message from the sim,
 * and the elapsed time with the share done and an estimate of the time left.
 */
import { formatElapsed } from "../Utils/helpers";

/**
 * Splits a progress message from the sim into its main part and the strat being simulated, when
 * the message ends with one ("Step 731/15,001 · 1e85 · BaPAI (1/2)" -> "BaPAI (1/2)"). In fast
 * runs the strat changes too quickly to read, so the status leaves it out.
 */
export function splitDetail(message: string): { outer: string; detail: string | null } {
  const at = message.lastIndexOf(" · ");
  if (at < 0) return { outer: message, detail: null };
  const last = message.slice(at + 3);
  if (!/\(\d+\/\d+\)$/.test(last)) return { outer: message, detail: null };
  return { outer: message.slice(0, at), detail: last };
}

/** A duration for the estimate: "45s", "4m 50s" or "1h 05m" */
export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const [h, m, s] = [Math.floor(total / 3600), Math.floor(total / 60) % 60, total % 60];
  if (h > 0) return `${h}h ${String(m).padStart(2, "0")}m`;
  if (m > 0) return `${m}m ${String(s).padStart(2, "0")}s`;
  return `${s}s`;
}

/**
 * "16s · 5% · ~4m 50s left": the time elapsed, then the share done and the time left when the
 * share is known. The estimate waits for a few seconds and a percent of progress, and assumes the
 * rest goes as fast as what was done.
 */
export function describeProgress(elapsedMs: number, fraction: number): string {
  let text = formatElapsed(elapsedMs);
  if (!Number.isFinite(fraction)) return text;
  text += ` · ${Math.floor(fraction * 100)}%`;
  if (elapsedMs >= 3000 && fraction >= 0.01 && fraction < 1) {
    text += ` · ~${formatDuration((elapsedMs * (1 - fraction)) / fraction)} left`;
  }
  return text;
}
