import { describe, expect, test } from "bun:test";
import { before, changeOf, heavy } from "../cmd/affected.ts";
import { Tree } from "../keys/tree.ts";

describe("nv affected", () => {
  test("the tree before a change holds each changed path as the base does, and an unchanged path is no change", async () => {
    const now = (await Tree.read()).edited({ LICENSE: "changed" });
    const { tree, paths } = before(now, { base: "HEAD", why: "--paths", paths: ["LICENSE", "rust-toolchain.toml"] });
    expect(paths).toEqual(["LICENSE"]);
    expect(tree.raw("LICENSE")).not.toBe(now.raw("LICENSE"));
    expect(tree.raw("rust-toolchain.toml")).toBe(now.raw("rust-toolchain.toml"));
  });

  test("a path the base does not have is deleted in the tree before", async () => {
    const now = (await Tree.read()).edited({ "docs/zz-new-page.md": "new" });
    const { tree, paths } = before(now, { base: "HEAD", why: "--paths", paths: ["docs/zz-new-page.md"] });
    expect(paths).toEqual(["docs/zz-new-page.md"]);
    expect(tree.has("docs/zz-new-page.md")).toBe(false);
  });

  test("--paths names exactly its paths, and --since a commit git does not know is refused", async () => {
    expect(await changeOf({ paths: ["./crates/a.rs", "docs\\b.md", "c/"] })).toEqual({ base: "HEAD", why: "--paths", paths: ["c", "crates/a.rs", "docs/b.md"] });
    expect(await changeOf({ since: "no-such-rev-anywhere" })).toBe("`no-such-rev-anywhere` names no commit");
  });

  test("the checks that cost minutes wait for the floor gate", () => {
    const c = (o: Record<string, unknown>) => ({ id: "x", kind: "command", stage: 1, ...o });
    expect(heavy(c({ argv: ["bun", "nv", "bench", "--warm-start"] }))).toBe(true);
    expect(heavy(c({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-abi-probe"] }))).toBe(true);
    expect(heavy(c({ argv: ["wsl.exe", "--", "bash", "-lc", "cargo +nightly fuzz run prefix"] }))).toBe(true);
    expect(heavy(c({ argv: ["bun", "nv", "db-matrix", "--all"] }))).toBe(true);
    expect(heavy(c({ argv: ["bun", "nv", "plan", "--check"], memoize: false }))).toBe(true);
    expect(heavy(c({ argv: ["bun", "nv", "plan", "--check"] }))).toBe(false);
    expect(heavy(c({ kind: "cargo-named", args: ["test", "-p", "nvs-stdlib"] }))).toBe(false);
  });
});
