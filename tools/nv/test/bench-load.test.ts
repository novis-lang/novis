import { expect, test } from "bun:test";
import { defaultSweep } from "../cmd/bench-load.ts";

test("the default sweep straddles the core count and drops duplicate and oversized widths", () => {
  expect(defaultSweep(1)).toEqual([1, 2, 8, 32]);
  expect(defaultSweep(16)).toEqual([1, 2, 16, 32, 128, 512]);
  expect(defaultSweep(128)).toEqual([1, 2, 128, 256, 1024]);
});
