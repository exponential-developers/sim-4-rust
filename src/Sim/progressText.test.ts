import { describe, expect, it } from "vitest";
import { describeProgress, splitDetail } from "./progressText";

describe("status line while simulating", () => {
  it("separates the strat being simulated from the rest of the message", () => {
    expect(splitDetail("Step 731/15,001 · 1e85 · BaPAI (1/2)")).toEqual({ outer: "Step 731/15,001 · 1e85", detail: "BaPAI (1/2)" });
    expect(splitDetail("MF (17/23) · active · MFd2Coast (12/41)")).toEqual({ outer: "MF (17/23) · active", detail: "MFd2Coast (12/41)" });
    // Nothing to leave out
    expect(splitDetail("MFd2Coast (11/41)")).toEqual({ outer: "MFd2Coast (11/41)", detail: null });
    expect(splitDetail("Pub 3 · 1e85/1e150")).toEqual({ outer: "Pub 3 · 1e85/1e150", detail: null });
  });

  it("shows the time elapsed and the share done, without a time-left estimate", () => {
    expect(describeProgress(2000, NaN)).toBe("2s");
    expect(describeProgress(16_000, 0.05)).toBe("16s · 5%");
    expect(describeProgress(60_000, 1)).toBe("1:00 · 100%");
  });
});
