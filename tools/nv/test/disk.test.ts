import { afterEach, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { liveKey, liveSet, retiredCovwsProof, strayProfiles } from "../cmd/disk.ts";
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
