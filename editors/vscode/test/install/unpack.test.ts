// Getting the binary out of the archive, and onto disk as something the machine can run.
//
// Both formats the release workflow builds are read here in plain Node, so both are exercised here
// against archives shaped like the ones `bun nv release --package` writes: the binary sits under a
// `nvs-<version>-<name>/` directory beside the notices, and it is the one member wanted.
//
// The disk half is the part with a failure mode worth pinning: an install that is interrupted must
// leave nothing the client's resolution chain would find and run
// (`rule:ide/the-extension-guides-an-install-and-never-bundles-one`).

import * as assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { executableName, installBinary, unpack } from "../../src/install";
import { Entry, tarGz, target, zip } from "./fixtures";

const LINUX = target("linux-x86_64");
const WINDOWS = target("windows-x86_64");
const BINARY = Buffer.from("#!/not/really/an/elf\n");
const NOTICES = Buffer.from("the notices every archive ships\n");

/** An archive laid out the way the release workflow lays one out, for `exe`. */
function members(stem: string, exe: string): Entry[] {
  return [
    { name: "pax_global_header", data: Buffer.from("52 comment=0\n"), kind: "g" },
    { name: `${stem}/`, kind: "5" },
    { name: `${stem}/THIRD-PARTY-NOTICES.md`, data: NOTICES },
    { name: `${stem}/${exe}`, data: BINARY }
  ];
}

describe("the binary inside a release archive", () => {
  it("is read out of a gzipped tar, past the entries that are not it", () => {
    const archive = tarGz(members("nvs-0.4.2-linux-x86_64", "nvs"));
    assert.deepEqual(Buffer.from(unpack(archive, LINUX)), BINARY);
  });

  it("is read out of a deflated zip through its central directory", () => {
    const archive = zip(members("nvs-0.4.2-windows-x86_64", "nvs.exe"));
    assert.deepEqual(Buffer.from(unpack(archive, WINDOWS)), BINARY);
  });

  it("is read out of a zip that stored it uncompressed just the same", () => {
    const archive = zip(members("nvs-0.4.2-windows-x86_64", "nvs.exe"), 0);
    assert.deepEqual(Buffer.from(unpack(archive, WINDOWS)), BINARY);
  });

  it("is named for the platform the archive was built for, not the one reading it", () => {
    assert.equal(executableName(LINUX), "nvs");
    assert.equal(executableName(WINDOWS), "nvs.exe");
  });

  it("is refused, naming the archive, when the archive holds no such member", () => {
    const archive = tarGz([{ name: "nvs-0.4.2-linux-x86_64/THIRD-PARTY-NOTICES.md", data: NOTICES }]);
    assert.throws(() => unpack(archive, LINUX), /The linux-x86_64 archive holds no nvs/);
  });
});

describe("an unpacked binary on disk", () => {
  let dir: string;

  beforeEach(() => {
    dir = join(mkdtempSync(join(tmpdir(), "nvs-install-")), "storage");
  });

  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  it("lands in the directory it was given, under the name this machine runs", async () => {
    const path = await installBinary(tarGz(members("nvs-0.4.2-linux-x86_64", "nvs")), LINUX, dir);
    assert.equal(path, join(dir, "nvs"));
    assert.deepEqual(readFileSync(path), BINARY);
  });

  it("is left executable, which is not something an archive's own mode is trusted for", async function () {
    if (process.platform === "win32") {
      this.skip();
    }
    const path = await installBinary(tarGz(members("nvs-0.4.2-linux-x86_64", "nvs")), LINUX, dir);
    assert.equal(statSync(path).mode & 0o111, 0o111);
  });

  it("leaves no half-written file beside itself", async () => {
    await installBinary(tarGz(members("nvs-0.4.2-linux-x86_64", "nvs")), LINUX, dir);
    assert.deepEqual(readdirSync(dir), ["nvs"]);
  });

  it("is not created at all when the archive holds nothing to install", async () => {
    const archive = tarGz([{ name: "nvs-0.4.2-linux-x86_64/THIRD-PARTY-NOTICES.md", data: NOTICES }]);
    await assert.rejects(installBinary(archive, LINUX, dir), /holds no nvs/);
    assert.equal(existsSync(dir), false);
  });
});
