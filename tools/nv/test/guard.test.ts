// `bun nv guard`: each rule with the calls it denies and its near misses, and the hook's own output.

import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { decide, HOOK, parse, RULES, type Tool, wiredTools } from "../cmd/guard.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

/** A tracked file over `nv peek`'s line limit, and a tracked one under it. */
const LONG = "tools/nv/cmd/peek.ts";
const SMALL = "package.json";

const event = (tool: Tool, input: Record<string, unknown>) => ({ tool_name: tool, tool_input: input, cwd: ROOT });
const bash = (command: string) => decide(event("Bash", { command }));

/** Each rule's denied command and its near miss. The rule's own name has to open the reason. */
const CASES: Record<string, { deny: [Tool, Record<string, unknown>][]; allow: [Tool, Record<string, unknown>][] }> = {
  "whole-read": {
    deny: [
      ["Read", { file_path: join(ROOT, LONG) }],
      ["Bash", { command: `cat ${LONG}` }],
      ["Bash", { command: `sed -n '1,9999p' ${LONG}` }],
    ],
    allow: [
      ["Read", { file_path: join(ROOT, LONG), offset: 100, limit: 40 }],
      ["Read", { file_path: join(ROOT, SMALL) }],
      ["Bash", { command: `cat ${LONG} | head -40` }],
      ["Bash", { command: `sed -n '120,160p' ${LONG}` }],
    ],
  },
  "proof-loop": {
    deny: [
      ["Bash", { command: "for f in docs/examples/str/*.nvs; do target/debug/nvs.exe run $f; done" }],
      ["Bash", { command: "ls tests/hostile/arr/*.nvs | xargs -n1 target/debug/nvs.exe run" }],
    ],
    allow: [
      ["Bash", { command: "target/debug/nvs.exe run docs/examples/str/one.nvs" }],
      ["Bash", { command: "for f in a b; do echo $f; done" }],
    ],
  },
  "shell-write": {
    deny: [
      ["Bash", { command: "cat > tools/nv/cmd/x.ts <<'EOF'\nconst a = 1;\nEOF" }],
      ["Bash", { command: `python -c "open('tools/nv/cmd/x.ts', 'w').write('x')"` }],
      ["Bash", { command: `bun -e "Bun.write('tools/nv/cmd/x.ts', 'x')"` }],
      ["Bash", { command: "echo x > tools/nv/cmd/x.ts" }],
      ["Bash", { command: "git diff >> docs/x.md" }],
      ["Bash", { command: "sed -i 's/a/b/' tools/nv/main.ts" }],
      ["Bash", { command: "echo x | tee tools/nv/cmd/x.ts" }],
      ["Bash", { command: "Set-Content -Path tools/nv/cmd/x.ts -Value x" }],
    ],
    allow: [
      ["Bash", { command: "cat > .agent-tmp/msg.txt <<'EOF'\nfeat: x > tools/nv/cmd/x.ts\nEOF" }],
      ["Bash", { command: "git commit -F - <<'EOF'\nfeat: x\nEOF" }],
      ["Bash", { command: `python -c "print(open('tools/nv/main.ts').read())"` }],
      ["Bash", { command: "cargo build > .agent-tmp/build.log 2>&1" }],
      ["Bash", { command: "git status 2>/dev/null" }],
      ["Bash", { command: "sed -n '1,20p' tools/nv/main.ts" }],
    ],
  },
  "cargo-p": {
    deny: [
      ["Bash", { command: "cargo test -p nvs-stdlib --lib" }],
      ["Bash", { command: "cargo build --package nvs-cli" }],
      ["Bash", { command: "cargo clippy -pnvs-ir" }],
      ["Bash", { command: "RUST_LOG=1 cargo test -p nvs-ir" }],
    ],
    allow: [
      ["Bash", { command: "cargo build --release -p nvs-cli" }],
      ["Bash", { command: "cargo test -r -p nvs-cli" }],
      ["Bash", { command: "cargo test --profile bench -p nvs-cli" }],
      ["Bash", { command: "cargo test --test meta" }],
      ["Bash", { command: "bun nv verify -p nvs-stdlib" }],
    ],
  },
  sleep: {
    deny: [
      ["Bash", { command: "sleep 30" }],
      ["Bash", { command: "until [ -f .agent-tmp/done ]; do sleep 1; done" }],
    ],
    allow: [["Bash", { command: "echo sleep" }]],
  },
  "kill-by-name": {
    deny: [
      ["Bash", { command: "pkill cargo" }],
      ["Bash", { command: "taskkill /IM nvs.exe /F" }],
    ],
    allow: [
      ["Bash", { command: "kill 1234" }],
      ["Bash", { command: "taskkill /PID 1234" }],
    ],
  },
  "trimmed-build": {
    deny: [
      ["Bash", { command: "cargo build 2>&1 | head -50" }],
      ["Bash", { command: "cargo test --test meta 2>&1 | tail -20" }],
      ["Bash", { command: "bun test tools/nv/test/x.test.ts | grep fail | tail" }],
      ["Bash", { command: "bun nv verify | head" }],
    ],
    allow: [
      ["Bash", { command: "cargo build 2>&1 | grep error" }],
      ["Bash", { command: "git log --oneline | head -5" }],
      ["Bash", { command: "cargo build > .agent-tmp/build.log 2>&1; tail -20 .agent-tmp/build.log" }],
    ],
  },
  "heavy-run": {
    deny: [
      ["Bash", { command: "cargo test 2>&1 | tail -20" }],
      ["Bash", { command: "cargo test --workspace --features x" }],
      ["Bash", { command: "timeout 100 looper check > .agent-tmp/out 2>&1" }],
      ["Bash", { command: "looper check" }],
      ["Bash", { command: "bun nv proofs --verify" }],
      ["Bash", { command: "bun nv proofs --run --show" }],
      ["Bash", { command: "bun nv verify --no-cache" }],
      ["Bash", { command: "bun nv select --full" }],
      ["Bash", { command: "bun nv benches" }],
      ["Bash", { command: "bun nv benches --owed" }],
      ["Bash", { command: "bun nv sweep" }],
      ["Bash", { command: "bun nv debt" }],
      ["Bash", { command: "bun tools/nv/main.ts sweep 2>&1" }],
    ],
    allow: [
      ["Bash", { command: "cargo test --lib html::tests 2>&1 | grep FAILED" }],
      ["Bash", { command: "cargo test --test meta" }],
      ["Bash", { command: "cargo test -- --exact spec_registry_coverage" }],
      ["Bash", { command: "cargo test --manifest-path .agent-tmp/probe/Cargo.toml" }],
      ["Bash", { command: "looper check --name links" }],
      ["Bash", { command: "looper checks" }],
      ["Bash", { command: "bun nv proofs --verify --group Core\\\\Html" }],
      ["Bash", { command: "bun nv proofs --run --id Core\\\\Str::length --show" }],
      ["Bash", { command: "bun nv bench --help" }],
      ["Bash", { command: "bun nv affected --run" }],
      ["Bash", { command: "bun nv verify" }],
      ["Bash", { command: "bun nv select --stats" }],
      ["Bash", { command: "bun nv sweep --help" }],
      ["Bash", { command: "bun nv debt --help" }],
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
    expect(decide({ tool_name: "PowerShell", tool_input: { command: `Get-Content ${LONG}` } })).toBeNull();
    expect(decide({ tool_name: "Bash", tool_input: {} })).toBeNull();
    expect(bash(`cat ${LONG} 'never closed`)).toBeNull();
    expect(bash(`cat ${LONG} "never closed`)).toBeNull();
    expect(bash("cat no/such/file.txt")).toBeNull();
  });

  test("a heredoc's body is not read as commands", () => {
    const p = parse(`cat > .agent-tmp/a <<'EOF'\ncargo test -p x\nEOF\necho ok`)!;
    expect(p.segments.map((s) => s.words.filter((w) => !w.op)[0]!.text)).toEqual(["cat", "echo"]);
  });

  test("the wiring is read from the settings' PreToolUse entries", () => {
    const hook = (matcher: string) => ({ matcher, hooks: [{ type: "command", command: HOOK }] });
    expect(wiredTools({ hooks: { PreToolUse: [hook("Read|Bash")] } })).toEqual(["Read", "Bash"]);
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
