import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";
import type { FileItems } from "../keys/scan.ts";
import { scanItems } from "../keys/scan.ts";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { CovMap, covFile, extract, ItemIndex, namesFromShow, namesToKeys, parseExport, recordedIn } from "../select/extract.ts";
import { logKeys, repoPath } from "../select/keys.ts";
import { scratch } from "./scratch.ts";

const tree = scratch();
afterAll(() => tree.cleanup());

const item = (id: string, start: number, end: number, kind = "fn") => ({ id, kind, start, end, test: false, digest: `d-${id}`, refs: [], defines: [] });

describe("extraction", () => {
  test("the names llvm-profdata shows are its two-space lines less their last colon, CRLF or not", () => {
    const shown = [
      "Counters:",
      "  _RNvCs1_4demo4main:",
      "    Hash: 0x0000",
      "    Counters: 1",
      "  D:\\repo\\crates\\a\\src\\lib.rs;_RNvCs2_1a6helper:",
      "Instrumentation level: Front-end",
    ].join("\r\n");
    expect(namesFromShow(shown)).toEqual(["_RNvCs1_4demo4main", "D:\\repo\\crates\\a\\src\\lib.rs;_RNvCs2_1a6helper"]);
  });

  test("an export gives each function the file and lines of its own regions", () => {
    const file = join(ROOT, "crates", "a", "src", "lib.rs");
    const doc = {
      data: [
        {
          functions: [
            { name: "f", filenames: [file, join(ROOT, "crates", "b", "src", "x.rs")], regions: [[10, 1, 12, 2, 1, 0, 0, 0], [40, 1, 41, 1, 1, 1, 0, 0], [11, 5, 15, 1, 1, 0, 0, 0]] },
            { name: "g", filenames: ["/rustc/abc/library/core/src/ops.rs"], regions: [[1, 1, 2, 1, 0, 0, 0, 0]] },
            { name: "h", filenames: [join(ROOT, "target", "covws", "x", "debug", "build", "nvs-stdlib-0123456789abcdef", "out", "intros.rs")], regions: [[3, 1, 4, 1, 0, 0, 0, 0]] },
          ],
        },
      ],
    };
    expect(parseExport(JSON.stringify(doc))).toEqual({ f: ["crates/a/src/lib.rs", 10, 15], g: [null, 1, 2], h: ["gen:nvs-stdlib/intros.rs", 3, 4] });
    expect(covFile(join(ROOT, "target", "debug", "x.rs"))).toBeNull();
  });

  test("a function maps to the innermost item holding its first line, a generated file to its includer, and an unknown name to *", () => {
    const files: FileItems[] = [{ file: "crates/a/src/lib.rs", parsed: true, raw: "r", items: [item("Outer::{impl}", 1, 30, "impl-rest"), item("Outer::run", 5, 20), item("helper!{inner}", 8, 12, "macro")] }];
    const index = new ItemIndex(files);
    const map = new Map<string, [string | null, number, number]>([
      ["run", ["crates/a/src/lib.rs", 6, 7]],
      ["inner", ["crates/a/src/lib.rs", 9, 10]],
      ["stray", ["crates/a/src/lib.rs", 25, 26]],
      ["dep", [null, 1, 2]],
      ["gen", ["gen:demo/intros.rs", 1, 2]],
    ]);
    const { keys, unmapped } = namesToKeys(["run", "inner", "stray", "dep", "gen", "nobody"], { get: (n) => map.get(n) }, index, [{ pkg: "demo", name: "intros.rs", file: "crates/a/src/reg.rs", id: "intros::include!" }]);
    expect([...keys].sort()).toEqual([
      ["*", ""],
      ["fn:crates/a/src/lib.rs#*", ""],
      ["fn:crates/a/src/lib.rs#Outer::run", "d-Outer::run"],
      ["fn:crates/a/src/lib.rs#helper!{inner}", "d-helper!{inner}"],
      ["fn:crates/a/src/reg.rs#intros::include!", ""],
    ]);
    expect(unmapped).toBe(1);
  });

  test("a footprint log becomes keys for repo paths alone, with class and card names lowercased", () => {
    const at = (p: string) => join(ROOT, p).replace(/\\/g, "/");
    const log = [
      "class\tCore\\Math",
      "class\t*",
      "card\tCore\\Process\\Result",
      `file\t${at("docs/examples/x.nvs")}`,
      `dir\t${at("docs/examples")}`,
      `exists\t${at("target/debug/nvs.exe")}`,
      "file\tC:/Windows/system32/x.dll",
      `tree\t${ROOT.replace(/\\/g, "/")}`,
      "named\tCargo.toml",
    ].join("\r\n");
    expect([...logKeys(log)].sort()).toEqual(["card:core\\process\\result", "class:*", "class:core\\math", "dir:docs/examples", "file:docs/examples/x.nvs", "named:Cargo.toml", "tree:."]);
    expect(logKeys("garbage without a tab\n").has("*")).toBe(true);
    expect(repoPath(`${ROOT}/crates/../Cargo.toml`)).toBe("Cargo.toml");
  });

  test("a record directory groups each atom's profiles, merge-pool ones included, with its log", () => {
    tree.put("rec/case~a.nvst-123.profraw", "");
    tree.put("rec/case~a.nvst-456.profraw", "");
    tree.put("rec/case~a.nvst.log", "");
    tree.put("rec/test~b-15375336203065930143_0.profraw", "");
    const got = recordedIn(join(tree.root, "rec"));
    expect(got.get("case~a.nvst")?.profraws.length).toBe(2);
    expect(got.get("case~a.nvst")?.log).toEndWith("case~a.nvst.log");
    expect(got.get("test~b")?.profraws.length).toBe(1);
  });

  const nvs = abs(`${COVWS_TARGET}/${process.platform === "win32" ? "x86_64-pc-windows-msvc" : hostTriple()}/debug/nvs${process.platform === "win32" ? ".exe" : ""}`);
  test.skipIf(!existsSync(nvs))(
    "a recorded run of the covws nvs extracts to the items it ran and the files it read",
    async () => {
      const dir = join(tree.root, "hello");
      mkdirSync(dir, { recursive: true });
      const r = await run([nvs, "run", "examples/hello.nvs"], {
        cwd: ROOT,
        env: { LLVM_PROFILE_FILE: join(dir, "hello-%p.profraw"), NVS_FOOTPRINT_LOG: join(dir, "hello.log"), NOVIS_NO_FILE_CACHE: "1" },
      });
      expect(r.code).toBe(0);
      const rec = recordedIn(dir).get("hello")!;
      const files = scanItems(["crates/nvs-cli/src/main.rs", "crates/nvs-syntax/src/lexer.rs"]);
      const got = await extract(rec, [nvs], new CovMap(join(tree.root, "covmap")), new ItemIndex(files), [], dir);
      expect([...got.keys.keys()].some((k) => k.startsWith("fn:crates/nvs-cli/src/main.rs#"))).toBe(true);
      expect(got.keys.has("file:examples/hello.nvs")).toBe(true);
      expect(got.executed).toBeGreaterThan(100);
      expect(readdirSync(dir)).toEqual([]);
    },
    300_000,
  );
});
