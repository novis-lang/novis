// `bun nv webp-lib`: the committed library is held to its recorded digest, and a source tarball is
// held to its pin before anything is extracted from it. Neither test downloads or compiles anything.

import { afterAll, expect, test } from "bun:test";
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { checkLibrary, LIB_DIR, PinMismatch, readRecord, verifyPin } from "../cmd/webp-lib.ts";
import { ROOT } from "../lib/paths.ts";

const SCRATCH = join(ROOT, ".agent-tmp", `webp-lib-test-${process.pid}`);

afterAll(() => rmSync(SCRATCH, { recursive: true, force: true }));

test("the committed library matches its recorded digest", () => {
  expect(checkLibrary(LIB_DIR)).toBe("");
});

test("a library whose bytes differ from its recorded digest fails the check", () => {
  mkdirSync(SCRATCH, { recursive: true });
  copyFileSync(join(LIB_DIR, "SOURCE.json"), join(SCRATCH, "SOURCE.json"));
  writeFileSync(join(SCRATCH, "libwebp.a"), "!<arch>\nnot the library\n");
  expect(checkLibrary(SCRATCH)).toContain("SOURCE.json records");
});

test("a source tarball whose digest differs from the pin is refused", () => {
  const record = readRecord(LIB_DIR);
  expect(() => verifyPin("libwebp source", new TextEncoder().encode("a different tarball"), record.source)).toThrow(PinMismatch);
  const r = Bun.spawnSync(["bun", "tools/nv/main.ts", "webp-lib", "--source", join(LIB_DIR, "COPYING")], { cwd: ROOT });
  expect(r.exitCode).toBe(1);
  expect(r.stderr.toString()).toContain("refused: the libwebp source tarball has sha256");
});
