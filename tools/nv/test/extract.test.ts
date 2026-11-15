// `executedNames` reads an atom's functions from one `llvm-profdata merge -sparse -text`. This pins that
// the one process names exactly the functions `merge -sparse` and then `show --all-functions` named,
// over ten atoms the covws `nvs` recorded, each from two processes.

import { afterAll, expect, test } from "bun:test";
import { existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { COVWS_TARGET, hostTriple, llvmTool } from "../lib/covws.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { executedNames, mergeProfiles, namesFromShow, namesFromText, recordedIn } from "../select/extract.ts";
import { scratch } from "./scratch.ts";

const tree = scratch();
afterAll(() => tree.cleanup());

const PROGRAMS = ["hello", "arith", "arrays", "bigint", "callable", "calls", "collect", "control", "enums", "errors"];

test("a text profile's records are the lines `# Func Hash:` follows, CRLF or not", () => {
  const text = [":ir", "_RNvCs1_4demo4main", "# Func Hash:", "12", "# Num Counters:", "1", "# Counter Values:", "3", "", "D:\\a\\lib.rs;_RNvCs2_1a6helper\r", "# Func Hash:\r", "7\r", ""].join("\n");
  expect(namesFromText(text)).toEqual(["_RNvCs1_4demo4main", "D:\\a\\lib.rs;_RNvCs2_1a6helper"]);
});

const nvs = abs(`${COVWS_TARGET}/${process.platform === "win32" ? "x86_64-pc-windows-msvc" : hostTriple()}/debug/nvs${process.platform === "win32" ? ".exe" : ""}`);
test.skipIf(!existsSync(nvs))(
  "one merge names the same functions the two calls named",
  async () => {
    await Promise.all(
      PROGRAMS.map(async (name) => {
        const dir = join(tree.root, name);
        mkdirSync(dir, { recursive: true });
        const env = { LLVM_PROFILE_FILE: join(dir, `${name}-%p.profraw`), NOVIS_NO_FILE_CACHE: "1" };
        // The exit status does not matter: the profiles are what is compared.
        await run([nvs, "check", `examples/${name}.nvs`], { cwd: ROOT, env });
        await run([nvs, "run", `examples/${name}.nvs`], { cwd: ROOT, env });
      }),
    );
    for (const name of PROGRAMS) {
      const rec = recordedIn(join(tree.root, name)).get(name)!;
      expect(rec.profraws.length).toBe(2);
      const indexed = join(tree.root, `${name}.profdata`);
      await mergeProfiles(rec.profraws, indexed);
      const shown = await run([llvmTool("llvm-profdata"), "show", "--all-functions", indexed], { timeoutMs: 600_000 });
      expect(shown.code).toBe(0);
      const two = namesFromShow(shown.stdout).sort();
      const one = (await executedNames(rec.profraws, join(tree.root, `${name}.proftext`))).sort();
      expect(two.length).toBeGreaterThan(100);
      expect(one).toEqual(two);
    }
  },
  600_000,
);
