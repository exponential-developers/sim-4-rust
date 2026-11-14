/**
 * "All" mode cache (same rules as sim-3's src/Sim/cache.ts): when "All" is run again, theories
 * whose input and relevant settings didn't change reuse the previous result instead of being
 * simulated again.
 */
import { getTheoryFromIndex, isMainTheory } from "../Utils/helpers";

const QUERY_KEY = "cacheQuery";
const RESPONSE_KEY = "cacheResponse";

function load<T>(key: string): T | null {
  try {
    const json = localStorage.getItem(key);
    return json === null ? null : (JSON.parse(json) as T);
  } catch {
    return null;
  }
}

/** Settings that, when changed, invalidate the whole cache */
function didSettingsChange(a: Settings, b: Settings): boolean {
  return a.dt !== b.dt
    || a.ddt !== b.ddt
    || a.bought_vars_delta !== b.bought_vars_delta
    || a.sim_all_strats !== b.sim_all_strats
    || a.mf_reset_depth !== b.mf_reset_depth;
}

/**
 * Returns the query to send to the sim, with cached theories set to 0 (not simulated),
 * and which theory indices are taken from the cache.
 */
export function cacheFilterQuery(query: SimQuery): { query: SimQuery; cached: boolean[] } {
  const cached: boolean[] = [];
  if (query.queryType !== "all") return { query, cached };
  const prev = load<SimAllQuery>(QUERY_KEY);
  if (prev === null
    || didSettingsChange(query.settings, prev.settings)
    || query.semi_idle !== prev.semi_idle
    || query.very_active !== prev.very_active) {
    return { query, cached };
  }

  const values = query.values.map((value, i) => {
    const theory = getTheoryFromIndex(i);
    const prevValue = prev.values[i];
    let reuse = prevValue !== undefined && value > 0 && value === prevValue;
    if (isMainTheory(theory) && query.sigma !== prev.sigma) reuse = false;
    if (!isMainTheory(theory) && (query.settings.completed_cts === "no") !== (prev.settings.completed_cts === "no")) reuse = false;
    if (theory === "EF" && query.settings.show_a23 !== prev.settings.show_a23) reuse = false;
    cached[i] = reuse;
    return reuse ? 0 : value;
  });
  return { query: { ...query, values }, cached };
}

/** Adds the cached results back into an "All" response, in theory order */
export function cacheFilterResponse(query: SimQuery, cached: boolean[], response: SimResponse): SimResponse {
  if (query.queryType !== "all" || response.responseType !== "all" || !cached.some((c) => c)) return response;
  const prev = load<SimAllResponse>(RESPONSE_KEY);
  if (prev === null) return response;

  const fresh = new Map(response.results.map((r) => [r.theory, r]));
  const old = new Map(prev.results.map((r) => [r.theory, r]));
  const results: simAllResult[] = [];
  for (let i = 0; i < query.values.length; i++) {
    const theory = getTheoryFromIndex(i);
    const result = fresh.get(theory) ?? (cached[i] ? old.get(theory) : undefined);
    if (result) results.push(result);
  }
  return { ...response, results };
}

/** Saves an "All" query (as entered, before filtering) and its complete response */
export function setCache(query: SimQuery, response: SimResponse) {
  if (query.queryType !== "all" || response.responseType !== "all") return;
  try {
    localStorage.setItem(QUERY_KEY, JSON.stringify(query));
    localStorage.setItem(RESPONSE_KEY, JSON.stringify(response));
  } catch {
    // Storage full or unavailable: caching is optional
  }
}
