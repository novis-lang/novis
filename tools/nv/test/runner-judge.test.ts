import { describe, expect, test } from "bun:test";
import { binaryGreen } from "../driver/runner.ts";

describe("a test binary's verdict for the sweep that judges it", () => {
  test("the sweep's own run decides, whatever another process wrote in the store since", () => {
    expect(binaryGreen("green", "owed")).toBe(true);
    expect(binaryGreen("green", "red")).toBe(true);
    expect(binaryGreen("green", undefined)).toBe(true);
    expect(binaryGreen("red", "green")).toBe(false);
  });

  test("a binary the sweep did not run is judged from the store, and only green is green", () => {
    expect(binaryGreen(undefined, "green")).toBe(true);
    expect(binaryGreen(undefined, "owed")).toBe(false);
    expect(binaryGreen(undefined, "red")).toBe(false);
    expect(binaryGreen(undefined, "")).toBe(false);
    expect(binaryGreen(undefined, undefined)).toBe(false);
  });
});
