import { describe, expect, test } from "bun:test";
import { changeOf, reason, reasons, splitBenches } from "../cmd/affected.ts";
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

  test("a check's reasons are its picked atoms', and only owed, red or new ones the change did not reach are from before it", () => {
    const pick = (id: string, why: Selected["why"], path = ""): [string, Selected] => [
      id,
      { id, kind: "case", why, keys: path ? [{ key: `fn:${path}#f`, origin: { path, item: "f", how: "changed" } }] : [] },
    ];
    const picked = new Map([pick("case:a", "key", "crates/x.rs"), pick("case:b", "owed"), pick("case:c", "new"), pick("case:d", "red"), pick("case:e", "owed", "crates/y.rs")]);
    expect(reason(picked.get("case:a")!)).toBe("crates/x.rs#f (item changed)");
    expect(reason(picked.get("case:c")!)).toBe("never recorded");
    expect(reasons(["case:a", "case:b", "case:z"], picked)).toEqual({ by: ["crates/x.rs#f (item changed)", "owed"], before: false });
    expect(reasons(["case:b", "case:c", "case:d"], picked)).toEqual({ by: ["never recorded", "owed", "red"], before: true });
    expect(reasons(["case:z"], picked)).toEqual({ by: [], before: false });
    // An owed atom under a key the change moved is reached by it, and so is a new one whose own file changed.
    expect(reasons(["case:b", "case:e"], picked).before).toBe(false);
    expect(reasons(["case:c"], picked, (id) => id === "case:c").before).toBe(false);
  });

  test("the plan judges the benches the change reaches and counts the ones owed from before", () => {
    const bench = (id: string, why: Selected["why"], key = ""): Selected => ({ id, kind: "bench", why, keys: key ? [{ key, origin: { path: "crates/x.rs", item: "f", how: "changed" } }] : [] });
    const picked = [
      bench("bench:b", "owed"),
      bench("bench:a", "key", "fn:crates/x.rs#f"),
      bench("bench:c", "red"),
      bench("bench:d", "new"),
      bench("bench:e", "new"),
      bench("bench:f", "def"),
      bench("bench:g", "owed", "fn:crates/x.rs#f"),
      bench("bench:h", "owed"),
      { id: "case:x", kind: "case", why: "key", keys: [] } as Selected,
    ];
    // `bench:e` is new and `bench:h` owed, and the change edited the files of both.
    const got = splitBenches(picked, (id) => id === "bench:e" || id === "bench:h");
    expect(got.judged.map((b) => b.id)).toEqual(["bench:a", "bench:e", "bench:f", "bench:g", "bench:h"]);
    expect(got.judged[0]!.why).toBe("crates/x.rs#f (item changed)");
    expect(got.before).toBe(3);
  });
});
