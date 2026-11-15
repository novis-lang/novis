import { expect, test } from "bun:test";
import { floorLine, median, parse, phases, readFloor, render } from "../cmd/start-cost.ts";

test("start-cost takes 25 runs by default, and --runs a whole number of 1 or more", () => {
  expect(parse([])).toEqual({ runs: 25, floor: false });
  expect(parse(["--runs", "5"])).toEqual({ runs: 5, floor: false });
  expect(parse(["--runs"])).toBe("--runs needs a number");
  expect(parse(["--runs", "0"])).toBe("--runs needs a whole number of 1 or more, not 0");
  expect(parse(["--runs", "2.5"])).toBe("--runs needs a whole number of 1 or more, not 2.5");
  expect(parse(["--cold"])).toBe("unknown argument --cold");
});

test("--floor takes --runs beside it, in either order", () => {
  expect(parse(["--floor"])).toEqual({ runs: 25, floor: true });
  expect(parse(["--floor", "--runs", "3"])).toEqual({ runs: 3, floor: true });
  expect(parse(["--runs", "3", "--floor"])).toEqual({ runs: 3, floor: true });
  expect(parse(["--floor", "--runs"])).toBe("--runs needs a number");
});

test("the floor line runs both binaries in each round, and its output reads back by binary", () => {
  const line = floorLine("/var/tmp/t/release", 4);
  expect(line).toContain("$(seq 4)");
  expect(line).toContain("/var/tmp/t/release/nvs --version");
  expect(line).toContain("/var/tmp/t/release/nvs-serve-probe --help");
  const table = readFloor(
    "size nvs 1000\nsize nvs-serve-probe 10\nrun nvs 2000000 300 1\nrun nvs-serve-probe 1000000 90 0\nnoise\nrun nvs 4000000 310 0\n",
  );
  expect(table.get("nvs")).toEqual({ bytes: 1000, ms: [2, 4], minor: [300, 310], major: [1, 0] });
  expect(table.get("nvs-serve-probe")?.minor).toEqual([90]);
  expect(readFloor("").size).toBe(0);
});

test("the median of an even count is the mean of its two middle values", () => {
  expect(median([3, 1, 2])).toBe(2);
  expect(median([4, 1, 3, 2])).toBe(2.5);
  expect(median([])).toBeNaN();
});

test("a phases line reads into a table by phase, and each row gives its share of the allocations", () => {
  const stderr = "compile: tokens=2\r\nphases: configuration.allocations=25 configuration.read=0 core.allocations=75 core.read=1\n";
  const table = phases(stderr);
  expect([...table.keys()]).toEqual(["configuration", "core"]);
  expect(table.get("core")?.get("read")).toBe(1);
  const rows = render(table);
  expect(rows[0]).toContain("share");
  expect(rows[1]).toMatch(/^configuration\s+25\b.*25\.0%$/);
  expect(rows[2]).toMatch(/^core\s+75\b.*75\.0%$/);
  expect(phases("compile: tokens=2\n").size).toBe(0);
});
