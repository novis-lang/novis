import { afterAll, describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { CACHE, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV, readLog } from "../lib/reads.ts";
import { markerKeys, scannedFor } from "../proofs/markers.ts";
import { anchorScan, type Entry, isAnchorFile, type Kind } from "../proofs/roster.ts";

describe("the keys a proof scan reads a file by", () => {
  test("a case names the features its markers cover and the calls it makes", () => {
    const text = "// covers: Core\\Str::length, `lang:x`\n--FILE--\nCore\\Str::upper(\"a\");\n";
    expect(markerKeys("tests/conformance/core/a.nvst", text).sort()).toEqual(["calls:#static:Str::upper", "covers:#Core\\Str::length", "covers:#lang:x"]);
    expect(markerKeys("docs/examples/core/a.nvs", text)).toEqual([]);
    expect(scannedFor("crates/nvs-stdlib/src/a.rs")).toEqual({ markers: true, calls: false });
  });

  test("a stdlib file names the members its class declares, and its name consts sign it", () => {
    const text = 'const NAME: &str = r"Core\\Demo";\nconst CLASS: CoreClass = CoreClass { name: NAME, methods: &[CoreMethod { name: "run" }] };\n';
    const got = anchorScan("crates/nvs-stdlib/src/demo.rs", text);
    expect(got.keys).toEqual(["anchor:#Core\\Demo::run"]);
    expect(anchorScan("crates/nvs-stdlib/src/demo.rs", text.replace("Demo", "Other")).consts).not.toBe(got.consts);
    expect(isAnchorFile("crates/nvs-runtime/src/x.rs")).toBe(true);
    expect(isAnchorFile("crates/nvs-ir/src/x.rs")).toBe(false);
  });
});

describe("the keys a verdict reads of `nvs meta --json`", () => {
  const LOG = join(CACHE, `roster-keys-${process.pid}.ndjson`);
  afterAll(() => rmSync(LOG, { force: true }));
  const entry = (id: string, kind: Kind, group: string): Entry => ({ id, kind, group, path: "", anchor: "", twin: [], summary: "", help: "" });

  /** The keys `noteRoster(scope, whole)` notes in a recorded process of its own. */
  async function noted(scope: Entry[], whole: boolean): Promise<string[]> {
    rmSync(LOG, { force: true });
    const url = (p: string) => JSON.stringify(pathToFileURL(join(ROOT, "tools", "nv", ...p.split("/"))).href);
    const script = [
      `import { install } from ${url("lib/reads.ts")};`,
      `import { noteRoster } from ${url("proofs/roster.ts")};`,
      `install(process.env.${ENV});`,
      `noteRoster(${JSON.stringify(scope)}, ${whole});`,
    ].join("\n");
    const r = await run([process.execPath, "-e", script], { cwd: ROOT, env: { [ENV]: LOG } });
    expect(r.stderr).toBe("");
    return readLog(LOG)?.keys ?? [];
  }

  test("a group of members depends on its class and its card, and on no other class", async () => {
    const scope = [entry("Core\\Str::length", "member", "Core\\Str"), entry("Core\\Str::upper", "member", "Core\\Str"), entry("lang:x/y", "lang", "lang:x")];
    expect(await noted(scope, false)).toEqual(["card:core\\str", "class:core\\str"]);
  });

  test("an enum, exception, interface or directive, and the whole roster, depend on every class and card", async () => {
    expect(await noted([entry("Core\\Str::length", "member", "Core\\Str"), entry("Core\\Order", "enum", "types:enum")], false)).toEqual(["card:*", "class:*"]);
    expect(await noted([entry("directive:cache.local", "directive", "config:directives")], false)).toEqual(["card:*", "class:*"]);
    expect(await noted([], true)).toEqual(["card:*", "class:*"]);
  });
});
