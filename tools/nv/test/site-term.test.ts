import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { snippetShape, termProblems, termRecording } from "../cmd/site.ts";
import { CACHE, ROOT } from "../lib/paths.ts";
import { mask, parseTerm, sameTerm } from "../lib/term.ts";
import { scratch, type Scratch } from "./scratch.ts";

// The real binary, so a recording here is what `nvs` prints: the build the acceptance runner names when
// it is built, or the debug build `nv verify` makes.
const NVS = [process.env.NVS_BIN, join(ROOT, `target/debug/nvs${process.platform === "win32" ? ".exe" : ""}`)].find((p) => p !== undefined && existsSync(p)) ?? "nvs";

let s: Scratch;
afterEach(() => s?.cleanup());

const T = "website/snippets/tools/cli/check.term";
const TERM = "nvs check main.nvs\nnvs run main.nvs\nnvs check missing.nvs\n";

/** A scratch site with the `.term` at `T`, a fixture with one program, and a `.term.out` when given. */
function site(out?: string): Scratch {
  s?.cleanup();
  s = scratch();
  s.put(T, TERM);
  s.put("website/snippets/tools/cli/check/main.nvs", '<?nvs\necho "hi\\n";\n');
  if (out !== undefined) s.put(`${T}.out`, out);
  return s;
}

describe("a .term", () => {
  test("is one command per non-blank line, each starting with nvs", () => {
    expect(parseTerm("nvs check\n\n  nvs run 'a b.nvs' \"--x\"\n")).toEqual([
      { line: "nvs check", argv: ["check"] },
      { line: "nvs run 'a b.nvs' \"--x\"", argv: ["run", "a b.nvs", "--x"] },
    ]);
    expect(parseTerm("ls\n")).toBe("line 1: `ls` does not start with `nvs`");
    expect(parseTerm("nvs run 'a.nvs\n")).toBe("line 1: a quote is not closed");
    expect(parseTerm("\n")).toBe("it has no command");
  });

  test("is compared with its values that differ per run masked", () => {
    expect(mask("took 12ms at 2026-10-26T10:00:01Z on 127.0.0.1:51234, id 0af7651916cd43dd8448eb211c80319c")).toBe("took <duration> at <date> on 127.0.0.1:<port>, id <id>");
    expect(mask("in 1.5s, file /tmp/nv-91/a.nvs\r\n  ")).toBe("in <duration>, file <tmp>");
    expect(sameTerm("$ nvs test\n3 passed in 41ms\nexit 0\n", "$ nvs test\n3 passed in 7ms\nexit 0")).toBe(true);
    expect(sameTerm("$ nvs test\n3 passed\nexit 0\n", "$ nvs test\n2 passed\nexit 0\n")).toBe(false);
  });

  test("records each command's output and exit code from a copy of its fixture, and deletes the copy", async () => {
    site();
    const before = existsSync(join(CACHE, "nv-term")) ? readdirSync(join(CACHE, "nv-term")).length : 0;
    const got = await termRecording(T, NVS, s.root);
    if ("failure" in got) throw new Error(got.failure);
    const blocks = got.recording.split("\n\n");
    expect(blocks.slice(0, 2)).toEqual(["$ nvs check main.nvs\nno errors\nexit 0", "$ nvs run main.nvs\nhi\nexit 0"]);
    expect(blocks[2]).toStartWith("$ nvs check missing.nvs\n");
    expect(blocks[2]).not.toEndWith("exit 0\n");
    expect(got.recording).not.toContain(s.root);
    expect(readdirSync(join(CACHE, "nv-term")).length).toBe(before);
    expect(existsSync(join(s.root, "website/snippets/tools/cli/check/main.nvs"))).toBe(true);
  });

  test("passes with a fresh .term.out, and fails with a stale or a missing one", async () => {
    site();
    const got = await termRecording(T, NVS, s.root);
    if ("failure" in got) throw new Error(got.failure);
    site(got.recording);
    expect(await termProblems(T, NVS, s.root)).toEqual([]);
    site(got.recording.replace("hi\n", "hello\n"));
    expect(await termProblems(T, NVS, s.root)).toEqual([`${T}: its .term.out is not what \`nvs\` prints for its commands now`]);
    site();
    expect(await termProblems(T, NVS, s.root)).toEqual([`${T}: no .term.out beside it; run \`bun nv site --bless ${T}\``]);
  });

  test("has a fixture whose programs are not snippets, and is shown by a page", () => {
    site();
    s.put(`${T.replace(/\.term$/, "")}-old.term.out`, "$ nvs check\nexit 0\n");
    const { problems, snippets, terms } = snippetShape(s.root);
    expect(terms).toEqual([T]);
    expect(snippets).toEqual([]);
    expect(problems).toEqual([`${T}: no page shows it`, "website/snippets/tools/cli/check-old.term.out: a .term.out and no .term beside it"]);
  });
});
