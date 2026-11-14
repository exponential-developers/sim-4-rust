// Runs before each test file: loads index.html into the jsdom document, so modules that look up
// page elements when they are imported (parse.ts, simState.ts, ...) find them.
import { readFileSync } from "fs";
import { resolve } from "path";

// Node 22+ has an experimental global localStorage that is undefined unless started with
// --localstorage-file, and it hides jsdom's. Use jsdom's (Vitest exposes the jsdom instance).
const { jsdom } = globalThis as typeof globalThis & { jsdom: { window: Window } };
Object.defineProperty(globalThis, "localStorage", { value: jsdom.window.localStorage, configurable: true });

// jsdom has no matchMedia (used to pick the default theme): behave like a light-mode browser
window.matchMedia = (query: string): MediaQueryList => ({
  matches: false,
  media: query,
  onchange: null,
  addListener: () => undefined,
  removeListener: () => undefined,
  addEventListener: () => undefined,
  removeEventListener: () => undefined,
  dispatchEvent: () => false,
});

const html = readFileSync(resolve(process.cwd(), "index.html"), "utf8");
document.documentElement.innerHTML = html.replace(/^<!DOCTYPE html>/i, "");
localStorage.clear();
