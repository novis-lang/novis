import { describe, expect, test } from "bun:test";
import { changeOf, reason, reasons } from "../cmd/affected.ts";
import { isHeavy as heavy } from "../driver/accept.ts";
import type { Selected } from "../select/select.ts";

describe("nv affected", () => {
  test("--paths names exactly its paths, and --since a commit git does not know is refused", async () => {
    expect(await changeOf({ paths: ["./crates/a.rs", "docs\\b.md", "c/"] })).toEqual({ base: "HEAD", why: "--paths", paths: ["c", "crates/a.rs", "docs/b.md"] });
    expect(await changeOf({ since: "no-such-rev-anywhere" })).toBe("`no-such-rev-anywhere` names no commit");
  });

  test("the checks that cost minutes wait for the floor gate", () => {
    const c = (o: Record<string, unknown>) => ({ id: "x", kind: "command", stage: 1, ...o });
    expect(heavy(c({ argv: ["bun", "nv", "bench", "--warm-start"] }))).toBe(true);
    expect(heavy(c({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-abi-probe"] }))).toBe(true);
    expect(heavy(c({ argv: ["wsl.exe", "--", "bash", "-lc", "cargo +nightly fuzz run prefix"] }))).toBe(true);
    expect(heavy(c({ argv: ["wsl.exe", "--", "bash", "-lc", "bash tools/tsan.sh"] }))).toBe(true);
    expect(heavy(c({ argv: ["bun", "nv", "db-matrix", "--all"] }))).toBe(true);
    expect(heavy(c({ argv: ["bun", "nv", "plan", "--check"], memoize: false }))).toBe(true);
    expect(heavy(c({ argv: ["bun", "nv", "plan", "--check"] }))).toBe(false);
    expect(heavy(c({ argv: ["git", "grep", "-n", "cargo +nightly fuzz run", "--", "docs"] }))).toBe(false);
    expect(heavy(c({ kind: "cargo-named", args: ["test", "-p", "nvs-stdlib"] }))).toBe(false);
  });

  test("a check's reasons are its picked atoms', and only owed, red or new ones are from before the change", () => {
    const pick = (id: string, why: Selected["why"], path = ""): [string, Selected] => [
      id,
      { id, kind: "case", why, keys: path ? [{ key: `fn:${path}#f`, origin: { path, item: "f", how: "changed" } }] : [] },
    ];
    const picked = new Map([pick("case:a", "key", "crates/x.rs"), pick("case:b", "owed"), pick("case:c", "new"), pick("case:d", "red")]);
    expect(reason(picked.get("case:a")!)).toBe("crates/x.rs#f (item changed)");
    expect(reason(picked.get("case:c")!)).toBe("never recorded");
    expect(reasons(["case:a", "case:b", "case:z"], picked)).toEqual({ by: ["crates/x.rs#f (item changed)", "owed"], before: false });
    expect(reasons(["case:b", "case:c", "case:d"], picked)).toEqual({ by: ["never recorded", "owed", "red"], before: true });
    expect(reasons(["case:z"], picked)).toEqual({ by: [], before: false });
  });
});
