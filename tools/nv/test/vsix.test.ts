import { expect, test } from "bun:test";
import { due, run as vsix } from "../cmd/vsix.ts";

test("a .vsix build is due when the file or its recorded tree is missing, or the tree moved", () => {
  expect(due({ tree: "a1", stamp: "a1", vsix: true })).toBeNull();
  expect(due({ tree: "a1", stamp: "a1", vsix: false })).not.toBeNull();
  expect(due({ tree: "a1", stamp: null, vsix: true })).not.toBeNull();
  expect(due({ tree: "b2", stamp: "a1", vsix: true })).not.toBeNull();
});

test("vsix --hook builds nothing in a side loop's lane, and an unknown flag is a usage error", async () => {
  expect(await vsix(["--hook"], { LOOPER_LOOP_KIND: "side" })).toBe(0);
  expect(await vsix(["--soon"], {})).toBe(2);
});
