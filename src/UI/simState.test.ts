import { beforeEach, describe, expect, it, vi } from "vitest";
import { restoreInputs, saveInputs } from "./simState";

const q = <T extends HTMLElement>(selector: string) => document.querySelector(selector) as T;

function options(selector: string, values: string[]) {
  const el = q<HTMLSelectElement>(selector);
  el.innerHTML = "";
  for (const v of values) el.add(new Option(v, v));
}

beforeEach(() => {
  localStorage.clear();
  options(".mode", ["All", "Single sim", "Chain"]);
  options(".theory", ["T1", "T2"]);
  options(".strat", ["Best Overall", "T1Ratio"]);
});

describe("remembered inputs", () => {
  it("restores what was saved", () => {
    q<HTMLSelectElement>(".mode").value = "Chain";
    q<HTMLSelectElement>(".theory").value = "T2";
    q<HTMLSelectElement>(".strat").value = "T1Ratio";
    q<HTMLInputElement>(".sigma").value = "120";
    q<HTMLInputElement>(".input").value = "e150";
    q<HTMLInputElement>(".cap").value = "e300";
    q<HTMLInputElement>(".hardCap").checked = true;
    q<HTMLTextAreaElement>(".modeInput").value = "5";
    q<HTMLTextAreaElement>(".simAllInputArea").value = "500 400";
    saveInputs();

    // a new visit: fields start empty
    for (const s of [".sigma", ".input", ".cap"]) q<HTMLInputElement>(s).value = "";
    q<HTMLTextAreaElement>(".modeInput").value = "";
    q<HTMLTextAreaElement>(".simAllInputArea").value = "";
    q<HTMLInputElement>(".hardCap").checked = false;
    q<HTMLSelectElement>(".mode").value = "All";
    q<HTMLSelectElement>(".theory").value = "T1";

    const onModeChange = vi.fn();
    const onTheoryChange = vi.fn();
    restoreInputs(onModeChange, onTheoryChange);

    expect(q<HTMLSelectElement>(".mode").value).toBe("Chain");
    expect(q<HTMLSelectElement>(".theory").value).toBe("T2");
    expect(q<HTMLSelectElement>(".strat").value).toBe("T1Ratio");
    expect(q<HTMLInputElement>(".sigma").value).toBe("120");
    expect(q<HTMLInputElement>(".input").value).toBe("e150");
    expect(q<HTMLInputElement>(".cap").value).toBe("e300");
    expect(q<HTMLInputElement>(".hardCap").checked).toBe(true);
    expect(q<HTMLTextAreaElement>(".modeInput").value).toBe("5");
    expect(q<HTMLTextAreaElement>(".simAllInputArea").value).toBe("500 400");
    // dependent fields get repopulated for the restored mode and theory
    expect(onModeChange).toHaveBeenCalledOnce();
    expect(onTheoryChange).toHaveBeenCalledOnce();
  });

  it("ignores saved choices that no longer exist", () => {
    localStorage.setItem("simInputs", JSON.stringify({ mode: "Removed mode", theory: "T99", strat: "Old strat", sigma: "50" }));
    const onModeChange = vi.fn();
    restoreInputs(onModeChange, vi.fn());
    expect(q<HTMLSelectElement>(".mode").value).toBe("All");
    expect(onModeChange).not.toHaveBeenCalled();
    expect(q<HTMLSelectElement>(".theory").value).toBe("T1");
    expect(q<HTMLInputElement>(".sigma").value).toBe("50");
  });

  it("does nothing with nothing saved, or with unreadable data", () => {
    q<HTMLInputElement>(".sigma").value = "77";
    restoreInputs(vi.fn(), vi.fn());
    expect(q<HTMLInputElement>(".sigma").value).toBe("77");
    localStorage.setItem("simInputs", "{not json");
    expect(() => restoreInputs(vi.fn(), vi.fn())).not.toThrow();
    expect(q<HTMLInputElement>(".sigma").value).toBe("77");
  });
});
