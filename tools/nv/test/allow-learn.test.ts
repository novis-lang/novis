// `bun nv allow --learn` over a scratch repository with a settings file and a dispatch table committed:
// the events file is written here, and git runs for real.

import { afterEach, describe, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { learn, PROMOTE_AFTER, SEEN, SETTINGS, type Seen } from "../cmd/allow.ts";
import { run } from "../lib/proc.ts";
import { type Scratch, scratch } from "./scratch.ts";

let tmp: Scratch | null = null;
afterEach(() => {
  tmp?.cleanup();
  tmp = null;
});

const git = (cwd: string, ...args: string[]) => run(["git", ...args], { cwd, timeoutMs: 60_000 });

const settings = (allow: string[]) => `{\n  "permissions": {\n    "allow": [\n${allow.map((r) => `      ${JSON.stringify(r)}`).join(",\n")}\n    ]\n  }\n}\n`;

/** A repository whose `main.ts` dispatches `commands` and whose settings allow `allow`. */
async function repo(commands: string[], allow: string[]): Promise<string> {
  tmp = scratch();
  tmp.put("tools/nv/main.ts", `const COMMANDS = {\n${commands.map((c) => `  ${c}: () => import("./cmd/${c}.ts"),`).join("\n")}\n};\n`);
  tmp.put(SETTINGS, settings(allow));
  tmp.put(".gitignore", ".loop/\n.agent-tmp/\n");
  const root = tmp.root;
  expect((await git(root, "init", "-q", "-b", "main")).code).toBe(0);
  for (const [k, v] of [["user.name", "t"], ["user.email", "t@example.com"], ["core.hooksPath", ".no-hooks"]]) expect((await git(root, "config", k!, v!)).code).toBe(0);
  expect((await git(root, "add", ".")).code).toBe(0);
  expect((await git(root, "commit", "-q", "-m", "start")).code).toBe(0);
  return root;
}

const use = (id: string, command: string) => ({ type: "assistant", session_id: "s3", message: { content: [{ type: "tool_use", id, name: "Bash", input: { command } }] } });
const result = (id: string, is_error = false, content = "ok") => ({ type: "user", session_id: "s3", message: { content: [{ type: "tool_result", tool_use_id: id, is_error, content }] } });

/** The events of one session: `make all` ran, `zip archive x` was blocked by the classifier. */
function events(root: string): string {
  const lines = [
    { type: "system", subtype: "init", session_id: "s3" },
    use("a", "make all -j4"), result("a"),
    use("b", "zip archive x"), result("b", true, "Blocked by auto mode: [Data Exfiltration]"),
  ];
  const file = join(root, "events.jsonl");
  writeFileSync(file, `${lines.map((l) => JSON.stringify(l)).join("\n")}\nnot json\n`);
  return file;
}

const seenIn = (root: string) => JSON.parse(readFileSync(join(root, SEEN), "utf8")) as Seen;

describe("bun nv allow --learn", () => {
  test("nv allow --learn records the tool calls of a finished session", async () => {
    const root = await repo(["peek"], ["Bash(bun nv peek:*)"]);
    expect(await learn(events(root), root)).toBe(0);
    expect(seenIn(root)).toEqual({
      "Bash\tmake all": { sessions: ["s3"], blocked: false },
      "Bash\tzip archive": { sessions: [], blocked: true },
    });
    expect(readFileSync(join(root, SETTINGS), "utf8")).toBe(settings(["Bash(bun nv peek:*)"]));
    expect((await git(root, "rev-list", "--count", "HEAD")).stdout.trim()).toBe("1");
  });

  test("nv allow --learn adds the owed rules to the settings file", async () => {
    const root = await repo(["peek", "allow"], ["Bash(bun nv peek:*)"]);
    const earlier: Seen = { "Bash\tmake all": { sessions: Array.from({ length: PROMOTE_AFTER - 1 }, (_, i) => `s${i}`), blocked: false } };
    tmp!.put(SEEN, JSON.stringify(earlier));
    expect(await learn(events(root), root)).toBe(0);
    const allow = (JSON.parse(readFileSync(join(root, SETTINGS), "utf8")) as { permissions: { allow: string[] } }).permissions.allow;
    expect(allow).toEqual(["Bash(bun nv peek:*)", "Bash(bun nv allow:*)", "Bash(make all:*)"]);
    expect((await git(root, "rev-list", "--count", "HEAD")).stdout.trim()).toBe("2");
    expect((await git(root, "status", "--porcelain", "--", SETTINGS)).stdout).toBe("");
  });

  test("nv allow --learn needs one events file, and a missing one is red", async () => {
    const { run: allow } = await import("../cmd/allow.ts");
    expect(await allow([])).toBe(2);
    expect(await allow(["--learn"])).toBe(2);
    expect(await allow(["--learn", "a.jsonl", "b.jsonl"])).toBe(2);
    const root = await repo([], []);
    expect(await learn("missing.jsonl", root)).toBe(1);
  });
});
