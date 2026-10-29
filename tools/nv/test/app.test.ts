import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { apps, listens } from "../cmd/app.ts";
import { ROOT } from "../lib/paths.ts";

describe("nv app", () => {
  test("an application's address is its nvs.toml's [server] listen", () => {
    expect(listens('[server]\nlisten = ["127.0.0.1:8140"]\n')).toEqual(["127.0.0.1:8140"]);
  });

  test("an nvs.toml with no listen gets nvs serve's default", () => {
    expect(listens("[log]\nclient_ip = false\n")).toEqual(["127.0.0.1:8000"]);
    expect(listens("")).toEqual(["127.0.0.1:8000"]);
  });

  test("every application listens on its own address, so all of them run at once", () => {
    expect(apps()).toEqual(expect.arrayContaining(["notes", "paperwork", "registry", "telemetry"]));
    const addresses = apps().flatMap((name) => listens(readFileSync(join(ROOT, "apps", name, "nvs.toml"), "utf8")));
    expect(new Set(addresses).size).toBe(addresses.length);
  });
});
