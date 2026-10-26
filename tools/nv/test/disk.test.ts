import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, utimesSync } from "node:fs";
import { join } from "node:path";
import { liveKey, liveSet, mintsGeneration, retiredCovwsProof, staleExtensions, staleOptimized, staleScratch, strayProfiles } from "../cmd/disk.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

describe("leftovers", () => {
  test("a default profile at the root or in a package directory is a stray, and a named one is not", () => {
    tmp = scratch();
    tmp.put("default_123_0_456.profraw", "x");
    tmp.put("crates/nvs-cli/default_9_0_1.profraw", "x");
    tmp.put("crates/nvs-cli/src/default_9_0_2.profraw", "x");
    tmp.put("crates/nvs-cli/atom-4.profraw", "x");
    tmp.put("benches/abi-probe/default_1_0_1.profraw", "x");
    const got = strayProfiles(tmp.root).map((p) => p.slice(tmp.root.length + 1).replace(/\\/g, "/"));
    expect(got).toEqual(["benches/abi-probe/default_1_0_1.profraw", "crates/nvs-cli/default_9_0_1.profraw", "default_123_0_456.profraw"]);
  });

  test("the covws build's proof profile is retired whole, and its debug build and the plain proof build are kept", () => {
    tmp = scratch();
    for (const f of ["covws/proof/deps/a", "covws/x86_64-pc-windows-msvc/proof/nvs.exe", "covws/x86_64-pc-windows-msvc/debug/nvs.exe", "covws/debug/deps/b", "proof/nvs.exe"]) tmp.put(f, "x");
    const got = retiredCovwsProof(tmp.root).map((p) => p.slice(tmp.root.length + 1).replace(/\\/g, "/"));
    expect(got.sort()).toEqual(["covws/proof", "covws/x86_64-pc-windows-msvc/proof"]);
  });
});

describe("optimized profiles", () => {
  /** An artifact's files under `deps/`, all written `hoursAgo` hours ago. */
  function artifact(dir: string, files: string[], hoursAgo: number): void {
    const at = Date.now() / 1000 - hoursAgo * 3600;
    for (const f of files) {
      tmp.put(`${dir}/${f}`, "x");
      utimesSync(join(tmp.root, dir, f), at, at);
    }
  }
  const hash = (n: number) => n.toString(16).padStart(16, "0");
  /** A fingerprint value as cargo writes it into the file beside the unit's JSON. */
  const value = (n: number) => {
    const b = Buffer.alloc(8);
    b.writeBigUInt64LE(BigInt(n));
    return b.toString("hex");
  };
  /**
   * One unit cargo built into `profile`: its library under `deps/`, written `hoursAgo` hours ago, and its
   * fingerprint, whose value is `n` and which names each `[crate, value]` in `deps` as built against.
   */
  function unit(profile: string, pkg: string, n: number, hoursAgo: number, deps: [string, number][] = [], kind = "lib", features = "[]"): void {
    const crate = pkg.replace(/-/g, "_");
    artifact(`${profile}/deps`, [`lib${crate}-${hash(n)}.rlib`], hoursAgo);
    const dir = `${profile}/.fingerprint/${pkg}-${hash(n)}`;
    const list = deps.map(([name, v]) => `[1,"${name}",false,${v}]`).join(",");
    tmp.put(`${dir}/${kind}-${crate}.json`, `{"rustc":1,"features":${JSON.stringify(features)},"profile":7,"rustflags":[],"compile_kind":0,"deps":[${list}]}`);
    tmp.put(`${dir}/${kind}-${crate}`, value(n));
    // Cargo writes the fingerprint when it builds the unit.
    const when = Date.now() / 1000 - hoursAgo * 3600;
    utimesSync(join(tmp.root, `${dir}/${kind}-${crate}.json`), when, when);
  }
  /** A `Cargo.lock` at `at` with the packages built from a path and those from the registry. */
  function lock(at: string, local: string[], registry: string[]): string {
    const block = (name: string, source: boolean) =>
      `[[package]]\nname = "${name}"\nversion = "1.0.0"\n${source ? 'source = "registry+https://github.com/rust-lang/crates.io-index"\n' : ""}`;
    tmp.put(at, ["version = 4\n", ...local.map((n) => block(n, false)), ...registry.map((n) => block(n, true))].join("\n"));
    return join(tmp.root, at);
  }
  const rel = (paths: string[]) => paths.map((p) => p.slice(tmp.root.length + 1).replace(/\\/g, "/"));
  const DAY = 24;

  test("the newest build of a workspace unit and what it was built against are kept, and the rest goes once idle", () => {
    tmp = scratch();
    const at = lock("Cargo.lock", ["nvs-runtime"], ["serde"]);
    // Three builds of one library, a day apart, each against its own build of a dependency.
    for (let i = 1; i <= 3; i++) {
      unit("release", "serde", 10 + i, (4 - i) * DAY);
      unit("release", "nvs-runtime", i, (4 - i) * DAY, [["serde", 10 + i]]);
    }
    // A build nothing reaches, inside the grace.
    unit("release", "serde", 20, 1);
    // A copy cargo has no fingerprint for.
    artifact("release/deps", [`libserde-${hash(99)}.rlib`], 30 * DAY);
    expect(rel(staleOptimized(tmp.root, 2, at))).toEqual([
      `release/deps/libnvs_runtime-${hash(1)}.rlib`,
      `release/deps/libnvs_runtime-${hash(2)}.rlib`,
      `release/deps/libserde-${hash(11)}.rlib`,
      `release/deps/libserde-${hash(12)}.rlib`,
      `release/deps/libserde-${hash(99)}.rlib`,
    ]);
  });

  test("two builds that turn on different features of one dependency keep a copy each", () => {
    tmp = scratch();
    const at = lock("Cargo.lock", ["nvs-cli", "nvs-abi-probe"], ["serde"]);
    unit("release", "serde", 11, 5 * DAY);
    unit("release", "serde", 12, 3 * DAY);
    unit("release", "serde", 13, 9 * DAY);
    unit("release", "nvs-cli", 1, 3 * DAY, [["serde", 12]], "bin");
    unit("release", "nvs-abi-probe", 2, 5 * DAY, [["serde", 11]], "lib", '["probe"]');
    expect(rel(staleOptimized(tmp.root, 2, at))).toEqual([`release/deps/libserde-${hash(13)}.rlib`]);
  });

  test("a dependency no recorded fingerprint matches keeps every copy of its crate, and without a lock nothing goes", () => {
    tmp = scratch();
    const at = lock("Cargo.lock", ["nvs-runtime"], ["serde", "rand"]);
    unit("release", "serde", 11, 5 * DAY);
    unit("release", "serde", 12, 4 * DAY);
    unit("release", "rand", 30, 9 * DAY);
    unit("release", "nvs-runtime", 1, 3 * DAY, [["serde", 77]]);
    expect(rel(staleOptimized(tmp.root, 2, at))).toEqual([`release/deps/librand-${hash(30)}.rlib`]);
    expect(staleOptimized(tmp.root, 2, join(tmp.root, "no.lock"))).toEqual([]);
  });

  test("the proof profile is judged on its own, and a debug build is never looked at", () => {
    tmp = scratch();
    const at = lock("Cargo.lock", ["nvs-types"], []);
    for (const profile of ["proof", "release"]) for (let i = 1; i <= 2; i++) unit(profile, "nvs-types", i, (10 - i) * DAY);
    for (let i = 7; i <= 8; i++) artifact("debug/deps", [`libnvs_types-${hash(i)}.rlib`], 30 * DAY);
    expect(rel(staleOptimized(tmp.root, 2, at))).toEqual([`proof/deps/libnvs_types-${hash(1)}.rlib`, `release/deps/libnvs_types-${hash(1)}.rlib`]);
  });

  test("an extension's target is judged in every profile by its own lock, and its old incremental caches go", () => {
    tmp = scratch();
    lock("extensions/pdf/Cargo.lock", ["nvs-pdf"], []);
    for (const profile of ["extensions/pdf/target/debug", "extensions/pdf/target/wasm32-wasip2/release"])
      for (let i = 1; i <= 2; i++) unit(profile, "nvs-pdf", i, (10 - i) * DAY);
    artifact("extensions/pdf/target/debug/incremental/nvs_pdf-old/s-1", ["dep-graph.bin"], 5 * DAY);
    artifact("extensions/pdf/target/debug/incremental/nvs_pdf-mid/s-1", ["dep-graph.bin"], 4 * DAY);
    artifact("extensions/pdf/target/debug/incremental/nvs_pdf-new/s-1", ["dep-graph.bin"], 0);
    // The newest write is what dates an incremental cache directory.
    for (const [name, days] of [["old", 5], ["mid", 4], ["new", 0]] as const) {
      const at = Date.now() / 1000 - days * 86400;
      utimesSync(join(tmp.root, `extensions/pdf/target/debug/incremental/nvs_pdf-${name}`), at, at);
    }
    // A source tree with no target is not looked at.
    tmp.put("extensions/fonts/a.ttf", "x");
    expect(rel(staleExtensions(tmp.root, 2, 2))).toEqual([
      `extensions/pdf/target/debug/deps/libnvs_pdf-${hash(1)}.rlib`,
      "extensions/pdf/target/debug/incremental/nvs_pdf-old",
      `extensions/pdf/target/wasm32-wasip2/release/deps/libnvs_pdf-${hash(1)}.rlib`,
    ]);
  });
});

describe("scratch", () => {
  test("a recorder's directory is judged on its own, and the discard pool and worktrees are never swept", async () => {
    tmp = scratch();
    const old = Date.now() / 1000 - 5 * 86400;
    for (const f of ["select-rec/verify-1/a.profraw", "select-rec/verify-2/a.profraw", "select-rec/discard/nvs-1.profraw", "worktrees/x/a", "old-probe/a"]) {
      tmp.put(f, "x");
      utimesSync(join(tmp.root, f), old, old);
    }
    tmp.put("select-rec/verify-2/b.profraw", "x");
    for (const d of ["select-rec/verify-1", "select-rec/verify-2", "old-probe"]) utimesSync(join(tmp.root, d), old, old);
    const got = (await staleScratch(tmp.root, 2)).map((p) => p.slice(tmp.root.length + 1).replace(/\\/g, "/"));
    expect(got).toEqual(["old-probe", "select-rec/verify-1"]);
  });
});

/** A one-crate cargo workspace of its own, so cargo never attaches it to the repository's. */
function crate(): Scratch {
  const s = scratch();
  s.put("Cargo.toml", '[package]\nname = "probe"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n');
  s.put("Cargo.lock", "version = 4\n");
  s.put("src/lib.rs", "pub fn one() -> i32 { 1 }\n");
  return s;
}

describe("disk live set", () => {
  test("the key holds across a source edit and moves with the lock file, a manifest and a new target", async () => {
    tmp = crate();
    const first = await liveKey(tmp.root);
    expect(first).not.toBeNull();
    expect(await liveKey(tmp.root)).toBe(first);

    tmp.put("src/lib.rs", "pub fn one() -> i32 { 2 }\n");
    expect(await liveKey(tmp.root)).toBe(first);

    tmp.put("Cargo.lock", "version = 4\n# edited\n");
    const afterLock = await liveKey(tmp.root);
    expect(afterLock).not.toBe(first);

    tmp.put("Cargo.toml", '[package]\nname = "probe"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n\n[profile.dev]\nopt-level = 1\n');
    const afterManifest = await liveKey(tmp.root);
    expect(afterManifest).not.toBe(afterLock);

    tmp.put("tests/extra.rs", "#[test]\nfn t() {}\n");
    expect(await liveKey(tmp.root)).not.toBe(afterManifest);
  });

  test("cargo is asked once per key, and again after the key moves", async () => {
    tmp = crate();
    const file = join(tmp.root, "live.json");
    let asks = 0;
    const ask = async () => {
      asks += 1;
      return new Set([`artifact-${asks}`]);
    };

    expect([...(await liveSet({ root: tmp.root, file, ask }))!]).toEqual(["artifact-1"]);
    expect([...(await liveSet({ root: tmp.root, file, ask }))!]).toEqual(["artifact-1"]);
    expect(asks).toBe(1);

    tmp.put("Cargo.lock", "version = 4\n# edited\n");
    expect([...(await liveSet({ root: tmp.root, file, ask }))!]).toEqual(["artifact-2"]);
    expect(asks).toBe(2);
  });

  test("a query may rebuild a workspace package, and stops at a dependency or another build", () => {
    const member = join(process.cwd(), "crates", "nvs-cli");
    const members = new Set([process.platform === "win32" ? member.toLowerCase() : member]);
    expect(mintsGeneration(`   Compiling nvs-cli v0.0.1 (${member})`, members)).toBe(false);
    expect(mintsGeneration(`    Checking nvs-cli v0.0.1 (${member})`, members)).toBe(false);
    expect(mintsGeneration("   Compiling tokio v1.48.0", members)).toBe(true);
    expect(mintsGeneration(`   Compiling patched v0.1.0 (${join(process.cwd(), "vendor", "patched")})`, members)).toBe(true);
    expect(mintsGeneration("    Blocking waiting for file lock on build directory", members)).toBe(true);
    expect(mintsGeneration("    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s", members)).toBe(false);
  });

  test("an answer cargo could not give is never remembered", async () => {
    tmp = crate();
    const file = join(tmp.root, "live.json");
    let asks = 0;
    const ask = async () => {
      asks += 1;
      return null;
    };
    expect(await liveSet({ root: tmp.root, file, ask })).toBeNull();
    expect(existsSync(file)).toBe(false);
    expect(await liveSet({ root: tmp.root, file, ask })).toBeNull();
    expect(asks).toBe(2);
  });
});
