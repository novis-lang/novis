import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { AllowWatch, NV_NEVER, PROMOTE_AFTER, SETTINGS, covered, nvCommands, owed, prefixOf, prefixesOf, record, type Seen, withRules } from "../driver/allow.ts";
import { ROOT } from "../lib/paths.ts";

describe("prefixOf", () => {
  test("takes two plain words, three for `bun nv` and `gh`, and a binary's path alone", () => {
    expect(prefixOf("git commit -q -F .agent-tmp/msg.txt")).toBe("git commit");
    expect(prefixOf("bun nv proofs --gate")).toBe("bun nv proofs");
    expect(prefixOf("just build --release")).toBe("just build");
    expect(prefixOf("./target/debug/nvs.exe test tests/x")).toBe("./target/debug/nvs.exe");
    expect(prefixOf(".\\target\\debug\\nvs.exe run a.nvs")).toBe(".\\target\\debug\\nvs.exe");
  });

  test("makes no rule from a delete, an interpreter, a history rewrite or shell syntax", () => {
    for (const c of ["rm -rf crates", "Remove-Item x", "python -c 1", "bun -e 1", "bun .agent-tmp/x.ts", "git push origin", "git reset --hard", "cargo install x", "bun nv bg cargo test", "bun nv loop", "N=1; x", "$env:X='1'", "for f in a; do", "wsl.exe -- bash", "target/../x.exe"]) {
      expect(prefixOf(c)).toBeNull();
    }
  });

  test("counts no read-only command, since auto mode runs those unasked", () => {
    for (const c of ["grep -rn x", "sed -n 1,2p f", "Get-Content f", "cd /e/mwl"]) expect(prefixOf(c)).toBeNull();
  });

  test("splits a command line on its operators", () => {
    expect(prefixesOf("cd /e/mwl && cargo fmt 2>&1 | tail -3; git add a.rs")).toEqual(["cargo fmt", "git add"]);
  });

  test("never splits inside quotes, and stops at a heredoc's body", () => {
    expect(prefixesOf('git grep -n "pub fn\\|pub struct" | head; just build')).toEqual(["git grep", "just build"]);
    expect(prefixesOf("python - <<'EOF'\nimport json\nEOF")).toEqual([]);
    expect(prefixesOf("echo 'a; make all'")).toEqual([]);
  });
});

describe("covered", () => {
  test("matches a rule's whole words, never a longer command name", () => {
    expect(covered(["Bash(bun nv bench:*)"], "Bash", "bun nv bench")).toBe(true);
    expect(covered(["Bash(bun nv bench:*)"], "Bash", "bun nv bench-load")).toBe(false);
    expect(covered(["Bash(git:*)"], "Bash", "git status")).toBe(true);
    expect(covered(["Bash(git status:*)"], "PowerShell", "git status")).toBe(false);
  });
});

const use = (id: string, command: string, name = "Bash") => ({ type: "assistant", message: { content: [{ type: "tool_use", id, name, input: { command } }] } });
const result = (id: string, is_error = false, content = "ok") => ({ type: "user", message: { content: [{ type: "tool_result", tool_use_id: id, is_error, content }] } });

describe("AllowWatch", () => {
  test("counts a call that ran, and a refused one as blocked even when another call ran it too", () => {
    const w = new AllowWatch();
    const events = [
      use("a", "make all"), result("a", true, "Exit code 2"),
      use("b", "just build"), result("b", true, "Blocked by auto mode: [Data Exfiltration]"),
      use("c", "just build --x"), result("c"),
      use("d", "zip archive x"), { type: "result", permission_denials: [{ tool_use_id: "d" }] },
    ];
    for (const e of events) w.note(e as Record<string, unknown>);
    const o = w.outcome();
    expect(o.approved).toEqual(["Bash\tmake all"]);
    expect(o.blocked.map((b) => b.key).sort()).toEqual(["Bash\tjust build", "Bash\tzip archive"]);
  });

  test("leaves out a call whose result never came", () => {
    const w = new AllowWatch();
    w.note(use("a", "make all") as Record<string, unknown>);
    expect(w.outcome()).toEqual({ approved: [], blocked: [] });
  });
});

describe("owed", () => {
  test("promotes a prefix after PROMOTE_AFTER sessions, and never one that was blocked", () => {
    const seen: Seen = {};
    for (let i = 1; i <= PROMOTE_AFTER; i++) record(seen, `s${i}`, { approved: ["Bash\tmake all", "Bash\tjust build"], blocked: [] });
    expect(record(seen, "s9", { approved: [], blocked: [{ key: "Bash\tjust build", command: "just build" }] })).toEqual(["Bash\tjust build"]);
    expect(owed(seen, [], [])).toEqual(["Bash(make all:*)"]);
  });

  test("waits for the threshold", () => {
    const seen: Seen = {};
    record(seen, "s1", { approved: ["Bash\tmake all"], blocked: [] });
    record(seen, "s1", { approved: ["Bash\tmake all"], blocked: [] });
    expect(owed(seen, [], [])).toEqual([]);
  });

  test("owes every dispatched `bun nv` command but NV_NEVER, and nothing the tree's settings already allow", () => {
    expect(owed({}, [], ["peek", "bg"])).toEqual(["Bash(bun nv peek:*)", "PowerShell(bun nv peek:*)"]);
    const rules = JSON.parse(readFileSync(join(ROOT, SETTINGS), "utf8")).permissions.allow as string[];
    expect(owed({}, rules, nvCommands())).toEqual([]);
    expect(nvCommands()).toContain("peek");
    for (const n of NV_NEVER) expect(rules.some((r) => r.startsWith(`Bash(bun nv ${n}:`))).toBe(false);
  });
});

describe("withRules", () => {
  test("appends to permissions.allow and keeps the rest of the file as it was", () => {
    const text = '{\n  "hooks": {},\n  "permissions": {\n    "allow": [\n      "Bash(a:*)"\n    ]\n  }\n}\n';
    const out = withRules(text, ["Bash(b:*)"]);
    expect(out).toBe('{\n  "hooks": {},\n  "permissions": {\n    "allow": [\n      "Bash(a:*)",\n      "Bash(b:*)"\n    ]\n  }\n}\n');
    expect(withRules(text, [])).toBe(text);
  });
});
