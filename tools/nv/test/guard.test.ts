import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { decide, parse, RULES, type Tool, wiredTools } from "../cmd/guard.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

/** A tracked file over `nv peek`'s line limit, and a tracked one under it. */
const LONG = "tools/nv/cmd/peek.ts";
const SMALL = "package.json";
/** `loop-goal-grep` matches this path by name and never opens it, so the file need not exist. */
const GOAL = "docs/agent/loop-goal.toml";

const event = (tool: Tool, input: Record<string, unknown>) => ({ tool_name: tool, tool_input: input, cwd: ROOT });
const bash = (command: string) => decide(event("Bash", { command }));
const pwsh = (command: string) => decide(event("PowerShell", { command }));

/** Each rule's denied command and its near miss. The rule's own name has to open the reason. */
const CASES: Record<string, { deny: [Tool, Record<string, unknown>][]; allow: [Tool, Record<string, unknown>][] }> = {
  "whole-read": {
    deny: [
      ["Read", { file_path: join(ROOT, LONG) }],
      ["Bash", { command: `cat ${LONG}` }],
      ["Bash", { command: `sed -n '1,9999p' ${LONG}` }],
      ["PowerShell", { command: `Get-Content ${LONG}` }],
    ],
    allow: [
      ["Read", { file_path: join(ROOT, LONG), offset: 100, limit: 40 }],
      ["Read", { file_path: join(ROOT, SMALL) }],
      ["Bash", { command: `cat ${LONG} | head -40` }],
      ["Bash", { command: `sed -n '120,160p' ${LONG}` }],
      ["PowerShell", { command: `Get-Content ${LONG} -TotalCount 40` }],
    ],
  },
  "proof-loop": {
    deny: [
      ["Bash", { command: "for f in docs/examples/str/*.nvs; do target/debug/nvs.exe run $f; done" }],
      ["PowerShell", { command: "Get-ChildItem tests\\hostile\\arr -Filter *.nvs | ForEach-Object { & target\\debug\\nvs.exe run $_.FullName }" }],
    ],
    allow: [["Bash", { command: "target/debug/nvs.exe run docs/examples/str/one.nvs" }]],
  },
  "loop-goal-grep": {
    deny: [
      ["Bash", { command: `grep -n 'stage = "8' ${GOAL}` }],
      ["PowerShell", { command: `Select-String -Path ${GOAL} -Pattern guard` }],
    ],
    allow: [
      ["Bash", { command: "grep -n guard docs/agent/loop-goal.md" }],
      ["Bash", { command: `python tools/peek.py ${GOAL}:re:guard` }],
    ],
  },
  "sleep-poll": {
    deny: [["Bash", { command: 'until grep -q done "C:/Users/u/AppData/Local/Temp/claude/x/tasks/b1.output"; do sleep 5; done' }]],
    allow: [
      ["Bash", { command: 'tail -20 "C:/Users/u/AppData/Local/Temp/claude/x/tasks/b1.output"' }],
      ["Bash", { command: "until curl -s localhost:8080; do sleep 1; done" }],
    ],
  },
  "inline-write": {
    deny: [
      ["Bash", { command: "cat > tools/nv/cmd/x.ts <<'EOF'\nconst a = 1;\nEOF" }],
      ["Bash", { command: `python -c "open('tools/nv/cmd/x.ts', 'w').write('x')"` }],
      ["Bash", { command: `bun -e "Bun.write('tools/nv/cmd/x.ts', 'x')"` }],
      ["PowerShell", { command: "@'\nconst a = 1;\n'@ | Set-Content tools/nv/cmd/x.ts" }],
    ],
    allow: [
      ["Bash", { command: "cat > .agent-tmp/msg.txt <<'EOF'\nfeat: x > tools/nv/cmd/x.ts\nEOF" }],
      ["Bash", { command: "git commit -F - <<'EOF'\nfeat: x\nEOF" }],
      ["Bash", { command: `python -c "print(open('tools/nv/main.ts').read())"` }],
      ["PowerShell", { command: "@'\nx\n'@ | Set-Content .agent-tmp/x.txt" }],
    ],
  },
  "cargo-p": {
    deny: [
      ["Bash", { command: "cargo test -p nvs-stdlib --lib" }],
      ["PowerShell", { command: "cargo build --package nvs-cli" }],
    ],
    allow: [
      ["Bash", { command: "cargo build --release -p nvs-cli" }],
      ["Bash", { command: "cargo test --test meta" }],
      ["Bash", { command: "bun nv verify -p nvs-stdlib" }],
    ],
  },
};

describe("nv guard", () => {
  test("every rule has a denied command and a near miss", () => {
    for (const rule of RULES) {
      expect(CASES[rule.name]?.deny.length ?? 0).toBeGreaterThan(0);
      expect(CASES[rule.name]?.allow.length ?? 0).toBeGreaterThan(0);
    }
    expect(Object.keys(CASES).sort()).toEqual(RULES.map((r) => r.name).sort());
  });

  for (const [name, { deny, allow }] of Object.entries(CASES)) {
    for (const [tool, input] of deny) {
      test(`${name} denies ${tool} ${JSON.stringify(input)}`, () => {
        expect(decide(event(tool, input)) ?? "").toStartWith(`guard: ${name}: `);
      });
    }
    for (const [tool, input] of allow) {
      test(`${name} allows ${tool} ${JSON.stringify(input)}`, () => {
        expect(decide(event(tool, input))).toBeNull();
      });
    }
  }

  test("it fails open on what it cannot read", () => {
    expect(decide(null)).toBeNull();
    expect(decide({ tool_name: "Write", tool_input: { file_path: join(ROOT, LONG) } })).toBeNull();
    expect(decide({ tool_name: "Bash", tool_input: {} })).toBeNull();
    expect(bash(`cat ${LONG} 'never closed`)).toBeNull();
    expect(pwsh(`Get-Content ${LONG} "never closed`)).toBeNull();
    expect(bash("cat no/such/file.txt")).toBeNull();
  });

  test("a heredoc's body is not read as commands", () => {
    const p = parse(`cat > .agent-tmp/a <<'EOF'\ncargo test -p x\nEOF\necho ok`, "Bash")!;
    expect(p.segments.map((s) => s.words.filter((w) => !w.op)[0]!.text)).toEqual(["cat", "echo"]);
  });

  test("the wiring is read from the settings' PreToolUse entries", () => {
    const hook = (matcher: string) => ({ matcher, hooks: [{ type: "command", command: "bun nv guard" }] });
    expect(wiredTools({ hooks: { PreToolUse: [hook("Read|Bash|PowerShell")] } })).toEqual(["Read", "Bash", "PowerShell"]);
    expect(wiredTools({ hooks: { PreToolUse: [hook("Read")] } })).toEqual(["Read"]);
    expect(wiredTools({})).toEqual([]);
  });

  test("the hook prints a deny decision and exits 0, and prints nothing for a payload it cannot read", async () => {
    const main = join(ROOT, "tools", "nv", "main.ts");
    const denied = await run([process.execPath, main, "guard"], { input: JSON.stringify(event("Bash", { command: `cat ${LONG}` })) });
    expect(denied.code).toBe(0);
    const out = JSON.parse(denied.stdout).hookSpecificOutput;
    expect(out.permissionDecision).toBe("deny");
    expect(out.permissionDecisionReason).toStartWith("guard: whole-read: ");
    const garbage = await run([process.execPath, main, "guard"], { input: "not json" });
    expect(garbage.code).toBe(0);
    expect(garbage.stdout).toBe("");
  });
});
