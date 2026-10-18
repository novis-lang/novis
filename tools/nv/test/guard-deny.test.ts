// `looper.toml`'s `[[guard.deny]]` rules, each with the commands it refuses and its near misses. looper
// matches a pattern with Rust's regex against the whole command and against each part between `;`,
// `&&`, `||`, `|` and newlines. This test does the same with a JavaScript `RegExp`, which reads every
// pattern here the same way once a leading `(?ms)` becomes the `m` and `s` flags.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parse } from "smol-toml";
import { ROOT } from "../lib/paths.ts";

interface DenyRule {
  pattern: string;
  reason: string;
}

const RULES = ((parse(readFileSync(join(ROOT, "looper.toml"), "utf8")) as { guard?: { deny?: DenyRule[] } }).guard?.deny ?? []).map((r) => {
  const inline = /^\(\?([a-z]+)\)/.exec(r.pattern);
  return { ...r, re: new RegExp(inline ? r.pattern.slice(inline[0].length) : r.pattern, inline?.[1] ?? "") };
});

/** The reason of the first rule that refuses `cmd`, or null. */
function refused(cmd: string): string | null {
  const parts = [cmd.trim(), ...cmd.split(/&&|\|\||[;|\n]/).map((p) => p.trim())];
  return RULES.find((r) => parts.some((p) => r.re.test(p)))?.reason ?? null;
}

/** Each rule, named by a word its reason holds, with the commands it refuses and lets through. */
const CASES: { says: string; deny: string[]; allow: string[] }[] = [
  {
    says: "in a loop over the feature proofs",
    deny: [
      "for f in docs/examples/str/*.nvs; do target/debug/nvs.exe run $f; done",
      "ls tests/hostile/arr/*.nvs | xargs -n1 target/debug/nvs.exe run",
    ],
    allow: ["target/debug/nvs.exe run docs/examples/str/one.nvs", "for f in a b; do echo $f; done"],
  },
  {
    says: "a second copy of every workspace crate",
    deny: ["cargo test -p nvs-stdlib --lib", "cargo build --package nvs-cli", "cargo clippy -pnvs-ir", "RUST_LOG=1 cargo test -p nvs-ir"],
    allow: [
      "cargo build --release -p nvs-cli",
      "cargo test -r -p nvs-cli",
      "cargo test --profile bench -p nvs-cli",
      "cargo test --test meta",
      "bun nv verify -p nvs-stdlib",
    ],
  },
  {
    says: "more than your change reaches",
    deny: [
      "bun nv proofs --verify",
      "bun nv proofs --run --show",
      "bun nv verify --no-cache",
      "bun nv select --full",
      "bun nv benches",
      "bun nv benches --owed",
      "bun nv sweep",
      "bun nv debt --goal lean-loop",
      "bun tools/nv/main.ts sweep 2>&1",
    ],
    allow: [
      "bun nv proofs --verify --group Core\\Html",
      "bun nv proofs --run --id Core\\Str::length --show",
      "bun nv affected --run",
      "bun nv verify",
      "bun nv select --stats",
      "bun nv sweep --help",
      "bun nv debt --help",
      "bun nv bench --help",
    ],
  },
  {
    says: "whole acceptance",
    deny: ["looper check", "timeout 100 looper check > .agent-tmp/out 2>&1"],
    allow: ["looper check --name links", "looper checks"],
  },
];

describe("looper.toml's guard.deny", () => {
  test("every rule has a case, and every case names a rule", () => {
    expect(RULES.map((r) => CASES.filter((c) => r.reason.includes(c.says)).length)).toEqual(RULES.map(() => 1));
  });

  for (const { says, deny, allow } of CASES) {
    for (const cmd of deny) {
      test(`refuses ${cmd}`, () => expect(refused(cmd) ?? "").toContain(says));
    }
    for (const cmd of allow) {
      test(`lets ${cmd} through`, () => expect(refused(cmd)).toBeNull());
    }
  }
});
