import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { posix } from "node:path";
import { type CiRun, EMBEDDED_TEXT, greenBase, LANES, lanesFor, manifestGraph, releaseNeeded, UNBUILT } from "../cmd/ci-changes.ts";
import { metadata } from "../keys/graph.ts";
import { ROOT } from "../lib/paths.ts";

const green = (head_sha: string, event = "push", conclusion: string | null = "success", head_branch: string | null = "main"): CiRun => ({ event, conclusion, head_sha, head_branch });

test("a push diffs against the newest green CI run on main whose commit is an ancestor of HEAD", async () => {
  const ancestors = new Set(["a1", "a2", "a3", "a4", "a5"]);
  const asked: string[] = [];
  const isAncestor = async (sha: string) => (asked.push(sha), ancestors.has(sha));
  const runs = [
    green("head"),
    green("x1", "pull_request"),
    green("a1", "push", "failure"),
    green("a1", "push", "cancelled"),
    green("a1", "push", "success", "next"),
    green("gone"),
    green("a2", "schedule"),
    green("a3"),
  ];
  // The run on HEAD itself, a pull request's, a red, a cancelled, another branch's and a commit off HEAD's history are passed over.
  expect(await greenBase(runs, "head", isAncestor)).toBe("a2");
  expect(asked).toEqual(["gone", "a2"]);
  expect(await greenBase([green("a3", "workflow_dispatch"), green("a4")], "head", isAncestor)).toBe("a3");
  expect(await greenBase([green("a5"), green("a4", "schedule")], "head", isAncestor)).toBe("a5");
  // No green ancestor: no base is known, and the caller turns every lane on.
  expect(await greenBase([green("gone"), green("head"), green("a1", "pull_request")], "head", isAncestor)).toBe(null);
  expect(await greenBase([], "head", isAncestor)).toBe(null);
});

test("a diff of docs alone needs no dev build, and anything else or an unknown diff does", () => {
  expect(releaseNeeded(["docs/goals/release-run.md", "AGENTS.md", "website/src/pages/index.astro"])).toBe(false);
  expect(releaseNeeded(["docs/goals/release-run.md", "crates/nvs-cli/src/main.rs"])).toBe(true);
  expect(releaseNeeded(["docs/reference/core/Str.md"])).toBe(true);
  expect(releaseNeeded(["CHANGELOG.md"])).toBe(true);
  expect(releaseNeeded(["a-directory-added-later/x.txt"])).toBe(true);
  expect(releaseNeeded([])).toBe(false);
  expect(releaseNeeded(null)).toBe(true);
});

/** Every path, from the repository's root, that a file matching `glob` names through `pattern`'s first
 * group, resolved against that file's directory. */
function named(glob: string, pattern: RegExp): string[] {
  const out: string[] = [];
  for (const file of new Bun.Glob(glob).scanSync({ cwd: ROOT })) {
    const rel = file.replaceAll("\\", "/");
    for (const m of readFileSync(posix.join(ROOT, rel), "utf8").matchAll(pattern)) {
      out.push(posix.normalize(posix.join(posix.dirname(rel), m[1]!)));
    }
  }
  return out;
}

const embedded = () => named("crates/**/*.rs", /include_(?:str|bytes)!\(\s*"([^"]+)"/g);

const inCode = (path: string) => lanesFor({ code: LANES.code! }, [path])[0]![1];

// A path crate outside the `code` lane would let a push change compiled code and run none of its tests.
test("the code lane covers every path crate", () => {
  const pathCrates = ["Cargo.toml", "crates/*/Cargo.toml"].flatMap((glob) => named(glob, /\bpath\s*=\s*"([^"]+)"/g));
  expect(pathCrates.some((dir) => dir.startsWith("vendor/"))).toBe(true);
  expect(pathCrates.map((dir) => `${dir}/Cargo.toml`).filter((path) => !inCode(path))).toEqual([]);
});

// Each file a crate embeds is code, or text a push may change without the platform matrix. A new
// embed outside both is a choice somebody has to make, so it fails here until it is made.
test("every file a crate embeds is in the code lane or on the embedded text list", () => {
  const files = embedded();
  expect(files.filter((path) => !inCode(path) && !EMBEDDED_TEXT.some((p) => (p.endsWith("/") ? path.startsWith(p) : path === p)))).toEqual([]);
  expect(EMBEDDED_TEXT.filter((path) => inCode(path))).toEqual([]);
});

// A file the binary embeds under a path `UNBUILT` names would leave the dev build stale after a push
// that changed it.
test("no file a crate embeds is on the list the dev build skips", () => {
  const files = embedded();
  expect(files.length).toBeGreaterThan(0);
  expect(files.filter((path) => !releaseNeeded([path]))).toEqual([]);
  expect(UNBUILT.filter((p) => p.startsWith("crates/") || p === "Cargo.toml" || p === "Cargo.lock")).toEqual([]);
});

// The `changes` job reads the graph off the manifests because it runs before cargo is installed, so
// this holds that reading to the one cargo gives wherever cargo is present.
test("the graph read off the manifests is the one cargo metadata gives", async () => {
  const cargo = await metadata();
  if (!cargo) return;
  const shape = (g: NonNullable<typeof cargo>) =>
    Object.fromEntries([...g].sort(([a], [b]) => (a < b ? -1 : 1)).map(([n, p]) => [n, { dir: p.dir, deps: Object.fromEntries([...p.deps].sort()) }]));
  expect(shape(manifestGraph()!)).toEqual(shape(cargo));
}, 120_000);
