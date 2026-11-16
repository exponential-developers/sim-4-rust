import { describe, expect, it } from "vitest";
import { describeProgress, formatDuration, splitDetail } from "./progressText";

describe("status line while simulating", () => {
  it("separates the strat being simulated from the rest of the message", () => {
    expect(splitDetail("Step 731/15,001 · 1e85 · BaPAI (1/2)")).toEqual({ outer: "Step 731/15,001 · 1e85", detail: "BaPAI (1/2)" });
    expect(splitDetail("MF (17/23) · active · MFd2Coast (12/41)")).toEqual({ outer: "MF (17/23) · active", detail: "MFd2Coast (12/41)" });
    // Nothing to leave out
    expect(splitDetail("MFd2Coast (11/41)")).toEqual({ outer: "MFd2Coast (11/41)", detail: null });
    expect(splitDetail("Pub 3 · 1e85/1e150")).toEqual({ outer: "Pub 3 · 1e85/1e150", detail: null });
  });

  it("shows the share done and estimates the time left", () => {
    expect(describeProgress(2000, NaN)).toBe("2s");
    // Too early for an estimate
    expect(describeProgress(2000, 0.5)).toBe("2s · 50%");
    expect(describeProgress(16_000, 0.05)).toBe("16s · 5% · ~5m 04s left");
    expect(describeProgress(60_000, 1)).toBe("1:00 · 100%");
  });

  it("formats estimates in seconds, minutes or hours", () => {
    expect(formatDuration(45_400)).toBe("45s");
    expect(formatDuration(290_000)).toBe("4m 50s");
    expect(formatDuration(3_900_000)).toBe("1h 05m");
  });
});
