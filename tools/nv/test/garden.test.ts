import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { options } from "../cmd/garden.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ArgError } from "../lib/py.ts";

const NVS = [process.env.NVS_BIN, join(ROOT, `target/debug/nvs${process.platform === "win32" ? ".exe" : ""}`)].find((p) => p !== undefined && existsSync(p)) ?? "nvs";

test("nv garden defaults to 2000 rounds at seed 1 against hyper and Node", () => {
  expect(options([])).toEqual({ rounds: 2000, seed: 1, refs: ["hyper", "node_stdlib"], nvsBin: null, keepUp: false, down: false });
  expect(options(["--refs", "hyper", "--rounds", "50", "--seed", "9"])).toMatchObject({ rounds: 50, seed: 9, refs: ["hyper"] });
  expect(options(["--help"])).toBeNull();
});

test("nv garden refuses a round count that is not a positive whole number, and an empty reference list", () => {
  expect(() => options(["--rounds", "0"])).toThrow(ArgError);
  expect(() => options(["--rounds", "many"])).toThrow(ArgError);
  expect(() => options(["--refs", ","])).toThrow(ArgError);
});

describe.skipIf(!existsSync(NVS))("tests/garden/echo.nvs", () => {
  test("answers a request with the Garden's JSON: every field base64, each header line its own pair", async () => {
    const dir = join(ROOT, ".agent-tmp", "garden-echo-test");
    mkdirSync(dir, { recursive: true });
    try {
      const request = join(dir, "request.nvsr");
      writeFileSync(
        request,
        "--METHOD--\nPOST\n--PATH--\n/a/b\n--HEADERS--\naccept: one\naccept: two\n--BODY--\nhello <body>",
      );
      const r = await run([NVS, "run", "--request", request, join(ROOT, "tests", "garden", "echo.nvs")], { cwd: dir, timeoutMs: 120_000 });
      expect(r.stderr).toBe("");
      expect(r.code).toBe(0);
      const doc = JSON.parse(r.stdout);
      const text = (b64: string) => Buffer.from(b64, "base64").toString("utf8");
      expect(text(doc.method)).toBe("POST");
      expect(text(doc.uri)).toBe("/a/b");
      expect(text(doc.version)).toBe("1.1");
      expect(text(doc.body)).toBe("hello <body>");
      const headers = (doc.headers as [string, string][]).map(([k, v]) => [text(k), text(v)]);
      expect(headers.filter(([k]) => k === "accept")).toEqual([
        ["accept", "one"],
        ["accept", "two"],
      ]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
