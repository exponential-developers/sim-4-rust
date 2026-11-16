/**
 * Texts of the status line while a simulation runs (main.ts): the progress message from the sim,
 * and the elapsed time with the share done.
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

/**
 * "16s · 5%": the time elapsed, then the share done when it is known. (No estimate of the time
 * left: later steps and publications usually take longer than earlier ones, so it was misleading.)
 */
export function describeProgress(elapsedMs: number, fraction: number): string {
  const elapsed = formatElapsed(elapsedMs);
  return Number.isFinite(fraction) ? `${elapsed} · ${Math.floor(fraction * 100)}%` : elapsed;
}
