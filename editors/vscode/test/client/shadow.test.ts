// The copy `nvs lsp` is started from, held against the one thing it must always be: the same
// program as the binary the user named.
//
// `src/shadow.ts` exists so that binary can be replaced while an editor is open. The failure worth
// pinning is the quiet one — a server still answering from a build that is no longer on disk, or
// from a copy taken halfway through a write — so every case here is about the copy and its source
// agreeing, or about the moment they stop.

import * as assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

import { Copies, Stamp, Watch, changed, copyName, locate, shadow, stamp, sweep } from "../../src/shadow";

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";
const FIRST = Buffer.from("the first build\n");
const SECOND = Buffer.from("the second build, which is longer\n");

/** Write `bytes` to `path` with a modification time of `seconds`, so two builds never share a stamp. */
function build(path: string, bytes: Buffer, seconds: number): void {
  writeFileSync(path, bytes);
  utimesSync(path, seconds, seconds);
}

describe("the copy the server runs from", () => {
  let root: string;
  let source: string;
  let storage: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "nvs-shadow-"));
    source = join(root, "bin", EXE);
    storage = join(root, "storage", "server");
    mkdirSync(dirname(source));
    build(source, FIRST, 1_000_000);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  it("holds the source's bytes, somewhere that is not the source", async () => {
    const copied = await shadow(source, storage);
    assert.notEqual(copied.path, source);
    assert.equal(dirname(copied.path), storage);
    assert.deepEqual(readFileSync(copied.path), FIRST);
    assert.deepEqual(copied.stamp, await stamp(source));
  });

  it("is one file however many times the same build is asked for", async () => {
    const once = await shadow(source, storage);
    const twice = await shadow(source, storage);
    assert.equal(twice.path, once.path);
    assert.deepEqual(readdirSync(storage), [basename(once.path)]);
  });

  it("is a different file, with the new bytes, once the source is a new build", async () => {
    const before = await shadow(source, storage);
    build(source, SECOND, 2_000_000);
    const after = await shadow(source, storage);
    assert.notEqual(after.path, before.path);
    assert.deepEqual(readFileSync(after.path), SECOND);
    assert.deepEqual(readFileSync(before.path), FIRST);
  });

  it("is made again when a file under its name does not hold the source's bytes", async () => {
    const at = (await stamp(source)) as Stamp;
    mkdirSync(storage, { recursive: true });
    writeFileSync(join(storage, copyName(source, at, process.platform)), "not that build");
    const copied = await shadow(source, storage);
    assert.deepEqual(readFileSync(copied.path), FIRST);
  });

  it("leaves nothing unfinished beside it", async () => {
    await shadow(source, storage);
    assert.deepEqual(readdirSync(storage).filter((name) => name.endsWith(".part")), []);
  });

  it("is refused, naming the source, when there is no such file", async () => {
    rmSync(source);
    await assert.rejects(shadow(source, storage), /is not a file/);
    assert.equal(existsSync(storage), false);
  });

  it("outlives its source being replaced and deleted, which is the point of it", async () => {
    const copied = await shadow(source, storage);
    build(source, SECOND, 2_000_000);
    rmSync(source);
    assert.deepEqual(readFileSync(copied.path), FIRST);
  });
});

describe("the copy handed to each spawn", () => {
  let root: string;
  let source: string;
  let storage: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "nvs-copies-"));
    source = join(root, EXE);
    storage = join(root, "server");
    build(source, FIRST, 1_000_000);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  it("is the same file for as long as the source is the same build", async () => {
    const copies = new Copies(storage);
    const once = await copies.of(source);
    const twice = await copies.of(source);
    assert.equal(twice.path, once.path);
  });

  it("is the new build on the very next call after the source is replaced", async () => {
    const copies = new Copies(storage);
    const before = await copies.of(source);
    build(source, SECOND, 2_000_000);
    const after = await copies.of(source);
    assert.notEqual(after.path, before.path);
    assert.deepEqual(readFileSync(after.path), SECOND);
  });

  it("is made again when the remembered copy was deleted", async () => {
    const copies = new Copies(storage);
    const once = await copies.of(source);
    rmSync(once.path);
    const twice = await copies.of(source);
    assert.deepEqual(readFileSync(twice.path), FIRST);
  });

  it("is made again when the remembered copy no longer holds the source's bytes", async () => {
    const copies = new Copies(storage);
    const once = await copies.of(source);
    build(once.path, Buffer.from("tampered with\n"), 3_000_000);
    const twice = await copies.of(source);
    assert.deepEqual(readFileSync(twice.path), FIRST);
  });

  it("is one file when many spawns ask at once", async () => {
    const copies = new Copies(storage);
    const all = await Promise.all([1, 2, 3, 4, 5].map(() => copies.of(source)));
    assert.deepEqual([...new Set(all.map((copy) => copy.path))], [all[0].path]);
    assert.deepEqual(readdirSync(storage), [basename(all[0].path)]);
  });

  it("removes the earlier build's copy once the new one is made", async () => {
    const copies = new Copies(storage);
    const before = await copies.of(source);
    build(source, SECOND, 2_000_000);
    const after = await copies.of(source);
    // The sweep is not awaited by `of`, so give it the moment it needs.
    await new Promise((done) => setTimeout(done, 100));
    assert.equal(existsSync(before.path), false);
    assert.equal(existsSync(after.path), true);
  });
});

describe("what a sweep of the storage deletes", () => {
  let root: string;
  let source: string;
  let storage: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "nvs-sweep-"));
    source = join(root, EXE);
    storage = join(root, "server");
    build(source, FIRST, 1_000_000);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  it("is every earlier build of this source, and never the one about to run", async () => {
    const old = await shadow(source, storage);
    build(source, SECOND, 2_000_000);
    const current = await shadow(source, storage);
    await sweep(storage, source, current.path);
    assert.equal(existsSync(old.path), false);
    assert.equal(existsSync(current.path), true);
  });

  it("is not another source's copy until that copy is old", async () => {
    const other = join(root, `other-${EXE}`);
    build(other, SECOND, 1_000_000);
    const foreign = await shadow(other, storage);
    const current = await shadow(source, storage);

    await sweep(storage, source, current.path);
    assert.equal(existsSync(foreign.path), true);

    const aMonthOn = Date.now() + 30 * 24 * 60 * 60 * 1000;
    await sweep(storage, source, current.path, aMonthOn);
    assert.equal(existsSync(foreign.path), false);
    assert.equal(existsSync(current.path), true);
  });

  it("is not a copy another window has only just started writing", async () => {
    const current = await shadow(source, storage);
    const partial = `${current.path}.99999.part`;
    writeFileSync(partial, "half of a build");
    await sweep(storage, source, current.path);
    assert.equal(existsSync(partial), true);
  });

  it("is nothing this module did not write", async () => {
    const current = await shadow(source, storage);
    const installed = join(storage, EXE);
    writeFileSync(installed, "a binary the install command put here");
    await sweep(storage, source, current.path, Date.now() + 365 * 24 * 60 * 60 * 1000);
    assert.equal(existsSync(installed), true);
  });
});

describe("finding the file a command names", () => {
  let root: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "nvs-locate-"));
    mkdirSync(join(root, "empty"));
    mkdirSync(join(root, "bin"));
    writeFileSync(join(root, "bin", EXE), FIRST);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  function where(path: string | undefined) {
    return { path, pathext: ".COM;.EXE", cwd: root, platform: process.platform };
  }

  it("searches PATH for a bare name, in order", async () => {
    const path = [join(root, "empty"), join(root, "bin")].join(process.platform === "win32" ? ";" : ":");
    assert.equal(await locate("nvs", where(path)), join(root, "bin", EXE));
  });

  it("reads a name with a separator in it as a path, relative to where the server is spawned", async () => {
    assert.equal(await locate("bin/nvs", where(undefined)), join(root, "bin", EXE));
    assert.equal(await locate(join(root, "bin", EXE), where(undefined)), join(root, "bin", EXE));
  });

  it("answers nothing when no file does, so the caller spawns the name as it always has", async () => {
    assert.equal(await locate("nvs", where(join(root, "empty"))), undefined);
    assert.equal(await locate("nvs", where(undefined)), undefined);
  });
});

describe("when a source counts as changed", () => {
  const running: Stamp = { size: 10, modified: 1 };
  const next: Stamp = { size: 12, modified: 2 };

  it("is not while it is the build that is running", () => {
    assert.equal(changed(running, running, running), false);
  });

  it("is not on the first reading of a new build, which may be half written", () => {
    assert.equal(changed(running, running, next), false);
  });

  it("is on the second equal reading of a new build", () => {
    assert.equal(changed(running, next, next), true);
  });

  it("is not while the file is missing, which is what a build does before it writes", () => {
    assert.equal(changed(running, running, undefined), false);
    assert.equal(changed(running, undefined, next), false);
  });

  it("is on the first settled file when there was none to start from", () => {
    assert.equal(changed(undefined, next, next), true);
  });
});

describe("a watch on the source", () => {
  let root: string;
  let source: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "nvs-watch-"));
    source = join(root, EXE);
    build(source, FIRST, 1_000_000);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  function wait(ms: number): Promise<void> {
    return new Promise((done) => setTimeout(done, ms));
  }

  it("fires once after the source becomes a new build, and not before", async () => {
    let fired = 0;
    const watch = new Watch(source, await stamp(source), () => { fired += 1; }, 10);
    try {
      await wait(80);
      assert.equal(fired, 0);
      build(source, SECOND, 2_000_000);
      await wait(200);
      assert.equal(fired, 1);
    } finally {
      watch.close();
    }
  });

  it("says nothing once it is closed", async () => {
    let fired = 0;
    const watch = new Watch(source, await stamp(source), () => { fired += 1; }, 10);
    watch.close();
    build(source, SECOND, 2_000_000);
    await wait(100);
    assert.equal(fired, 0);
  });
});
