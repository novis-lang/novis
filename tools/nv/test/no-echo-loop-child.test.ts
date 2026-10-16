// A Windows child that writes a lot is a `cmd.exe` loop, and its body is `@type` of a file, never
// `@echo`. Each pass of a `for /l` loop costs `cmd` a parse and a write, a millisecond or so on an idle
// machine and ten times that beside the loop's pooled sweeps, so an `echo` loop writes a few kilobytes
// a second: a 16 MB memory ceiling is hours away and a 64 KB `max_output` most of a minute, and the
// case or attack that waits for it is killed at its deadline with any build. `type` copies a file in
// whole buffers, so `for /l %i in (0,0,1) do @type C:\Windows\System32\cmd.exe` writes megabytes a
// second without end and needs no file of its own, and a child that must write an exact size types a
// file the program wrote at that size first. This holds every program the pipeline runs to that.

import { describe, expect, test } from "bun:test";
import { ROOT } from "../lib/paths.ts";

/** A `for /l` loop whose body runs `echo`, in any case `cmd` accepts. */
const ECHO_LOOP = /for\s+\/l\b.*\becho\b/i;
const ECHO_LOOP_ERE = "for[[:space:]]+/l[[:space:]].*echo";

/** Where the pipeline's own code and the programs it runs live. */
const PIPELINE = ["tools/nv", "examples", "tests", "docs/examples", "benches/members", "crates", "website/snippets"];

/** A line that is only a comment, in any of the languages under `PIPELINE`. */
const COMMENT = /^\s*(?:\/\/|\/\*|\*|#)/;

describe("a Windows child never writes its output with an echo loop", () => {
  test("the pattern matches the loop it exists for and not the one that replaces it", () => {
    expect(ECHO_LOOP.test(`["/c", "for /l %i in (0,0,1) do @echo a line that repeats"]`)).toBe(true);
    expect(ECHO_LOOP.test(`'((for /L %%i in (1,1,%d) do @echo %s)%s) 1>&2'`)).toBe(true);
    expect(ECHO_LOOP.test(`["/c", "for /l %i in (0,0,1) do @type C:\\\\Windows\\\\System32\\\\cmd.exe"]`)).toBe(false);
  });

  test("no case, attack, bench, example or test starts a for /l loop that echoes", () => {
    const r = Bun.spawnSync(["git", "grep", "-n", "-I", "-i", "--untracked", "-E", ECHO_LOOP_ERE, "--", ...PIPELINE], {
      cwd: ROOT,
      stdout: "pipe",
      stderr: "pipe",
    });
    // git grep exits 1 when nothing matches, which is the answer this wants.
    expect(r.exitCode === 0 || r.exitCode === 1).toBe(true);
    const found = r.stdout
      .toString()
      .split("\n")
      .filter((hit) => hit !== "")
      .filter((hit) => {
        const [file = "", , ...rest] = hit.split(":");
        const line = rest.join(":");
        return !file.startsWith("tools/nv/test/") && ECHO_LOOP.test(line) && !COMMENT.test(line);
      });
    expect(found).toEqual([]);
  });
});
