import { expect, test } from "bun:test";
import { bookkeeping } from "../cmd/ci-green.ts";

test("a wrap's records and the Markdown rendered from them are bookkeeping, and code is not", () => {
  for (const path of [
    "data/goals/ci-green.handoff.json",
    "data/goals/ci-green.json",
    "data/chain.json",
    "data/plan/status.json",
    "data/playbook/tooling/a-commit.json",
    "docs/agent/playbook/tooling.md",
    "docs/plan/m9.md",
    "docs/implementation-plan.md",
  ]) {
    expect(bookkeeping(path)).toBe(true);
  }
  for (const path of ["data/chain.json.bak", "data/rules/tooling/x.json", "tools/nv/cmd/ci-green.ts", "crates/nvs-cli/src/main.rs", "docs/implementation-plan.md.orig"]) {
    expect(bookkeeping(path)).toBe(false);
  }
});
