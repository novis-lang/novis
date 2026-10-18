// The gates a change must pass give their verdicts with no looper on PATH: `bun nv guard`, `bun nv prose`
// and `tools/git-hooks/commit-msg`. Each runs as its own process with every PATH directory that holds a
// `looper` program removed, so a check that quietly falls back to looper fails here.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { delimiter, join } from "node:path";
import { ROOT } from "../lib/paths.ts";

const PATH = (process.env.PATH ?? process.env.Path ?? "")
  .split(delimiter)
  .filter((dir) => dir && !["looper", "looper.exe"].some((exe) => existsSync(join(dir, exe))))
  .join(delimiter);
const ENV: Record<string, string> = {
  ...Object.fromEntries(Object.entries(process.env).filter(([k, v]) => k.toUpperCase() !== "PATH" && v !== undefined)),
  PATH,
} as Record<string, string>;

const SCRATCH = join(ROOT, ".agent-tmp", `without-looper-${process.pid}`);
mkdirSync(SCRATCH, { recursive: true });
afterAll(() => rmSync(SCRATCH, { recursive: true, force: true }));

/** Runs `argv` from the root with the stripped PATH, and returns its exit code and what it printed. */
async function bare(argv: string[], input?: string): Promise<{ code: number; out: string }> {
  const exe = Bun.which(argv[0]!, { PATH }) ?? argv[0]!;
  const child = Bun.spawn([exe, ...argv.slice(1)], { cwd: ROOT, env: ENV, stdin: input === undefined ? "ignore" : new Blob([input]), stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
  return { code, out: stdout + stderr };
}

const nv = (args: string[], input?: string) => bare([process.execPath, join(ROOT, "tools", "nv", "main.ts"), ...args], input);

let written = 0;
function scratch(name: string, text: string): string {
  const path = join(SCRATCH, `${written++}-${name}`);
  writeFileSync(path, text);
  return path;
}

describe("with no looper on PATH", () => {
  test("PATH holds no looper", () => {
    expect(Bun.which("looper", { PATH })).toBeNull();
  });

  test("`bun nv guard` denies a whole read of a big file and allows a ranged one", async () => {
    const event = (command: string) => JSON.stringify({ tool_name: "Bash", tool_input: { command }, cwd: ROOT });
    const denied = await nv(["guard"], event("cat tools/nv/cmd/peek.ts"));
    expect(denied.code).toBe(0);
    expect(JSON.parse(denied.out).hookSpecificOutput.permissionDecision).toBe("deny");
    const allowed = await nv(["guard"], event("sed -n '120,160p' tools/nv/cmd/peek.ts"));
    expect(allowed).toEqual({ code: 0, out: "" });
  });

  test("`bun nv prose` flags a long sentence, a joining dash and a banned word, and passes plain text", async () => {
    const long = Array.from({ length: 26 }, (_, i) => `word${i}`).join(" ");
    const bad = scratch("bad.md", `# Title\n\n${long}.\n\nIt starts — it stops.\n\nYou can simply run it.\n`);
    const flagged = await nv(["prose", bad]);
    expect(flagged.code).toBe(1);
    expect(flagged.out).toContain(":3: a sentence of 26 words, over 25");
    expect(flagged.out).toContain(":5: a dash joins two sentences");
    expect(flagged.out).toContain(":7: a banned word: simply");
    const good = scratch("good.md", "# Title\n\nThis program prints `3`.\n\n```nvs\nsimply — anything goes here\n```\n");
    expect((await nv(["prose", good])).code).toBe(0);
  });

  describe("`tools/git-hooks/commit-msg`", () => {
    const hook = (text: string) => bare(["sh", join(ROOT, "tools", "git-hooks", "commit-msg"), scratch("msg.txt", text)]);

    test("passes a message in shape, and git's own subjects", async () => {
      expect((await hook("feat(nv): `bun nv prose` checks the end-user Markdown\n\nThe body.\n")).code).toBe(0);
      expect((await hook("fixup! anything at all\n")).code).toBe(0);
      expect((await hook("wip(loop): the unfinished slice of session 0042\n")).code).toBe(0);
    });

    test("refuses a type it does not list, a long subject, a body on line two and a trailer", async () => {
      const type = await hook("wip: something\n");
      expect(type.code).toBe(1);
      expect(type.out).toContain("is not `type(scope): what is now true`");
      const long = await hook(`fix: ${"x".repeat(200)}\n`);
      expect(long.code).toBe(1);
      expect(long.out).toContain("characters, and may be at most");
      const second = await hook("fix: a thing\nno blank line\n");
      expect(second.code).toBe(1);
      expect(second.out).toContain("the second line is not blank");
      const trailer = await hook("fix: a thing\n\nBody.\n\nSigned-off-by: Somebody <a@example.com>\n");
      expect(trailer.code).toBe(1);
      expect(trailer.out).toContain("no attribution trailers");
    });
  });
});
