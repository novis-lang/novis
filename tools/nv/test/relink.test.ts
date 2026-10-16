import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { free, held, linked, pinned, sweep } from "../lib/relink.ts";
import { scratch } from "./scratch.ts";

test("a held binary is cargo's remove failure naming that binary, and nothing else", () => {
  const exe = join("target", "release", "nvs.exe");
  expect(held("error: failed to remove file `D:\\t\\target\\release\\nvs.exe`\n\nCaused by: Access is denied.", exe)).toBe(true);
  expect(held("error: failed to remove file `D:\\t\\target\\release\\other.exe`", exe)).toBe(false);
  expect(held("error: linking with `link.exe` failed: nvs.exe", exe)).toBe(false);
});

test("free moves the binary to the first unused aside name, and sweep deletes every aside copy", () => {
  const s = scratch();
  try {
    s.put("nvs.exe", "one");
    s.put("nvs.exe.held-0", "older");
    const exe = join(s.root, "nvs.exe");
    const aside = free(exe);
    expect(aside).toBe(`${exe}.held-1`);
    expect(existsSync(exe)).toBe(false);
    expect(readFileSync(aside!, "utf8")).toBe("one");
    expect(free(exe)).toBeNull();
    s.put("nvs.exe", "two");
    expect(sweep(exe)).toBe(2);
    expect(readFileSync(exe, "utf8")).toBe("two");
  } finally {
    s.cleanup();
  }
});

test("linked retries a build once after moving a held binary aside, and never retries another failure", async () => {
  const s = scratch();
  try {
    s.put("nvs.exe", "old");
    const exe = join(s.root, "nvs.exe");
    let calls = 0;
    const heldOnce = await linked(exe, async () => ({ code: calls++ === 0 ? 101 : 0, err: "failed to remove file nvs.exe" }), (r) => r.err);
    expect([heldOnce.code, calls]).toEqual([0, 2]);
    calls = 0;
    const other = await linked(exe, async () => ({ code: (calls++, 101), err: "error[E0308]: mismatched types" }), (r) => r.err);
    expect([other.code, calls]).toEqual([101, 1]);
  } finally {
    s.cleanup();
  }
});

test("pinned links one copy per build, keeps it when the original moves, and deletes an older copy nobody claims", () => {
  const s = scratch();
  try {
    s.put("nvs.exe", "one");
    const exe = join(s.root, "nvs.exe");
    const first = pinned(exe);
    expect(pinned(exe)).toBe(first);
    expect(free(exe)).not.toBeNull();
    expect(readFileSync(first, "utf8")).toBe("one");
    s.put("nvs.exe", "build two");
    s.put("nvs-run-old.exe", "abandoned");
    s.put("nvs-run-old.exe.claim-999999999", "");
    s.put("nvs-run-kept.exe", "running");
    s.put(`nvs-run-kept.exe.claim-${process.pid}`, "");
    const second = pinned(exe);
    expect(second).not.toBe(first);
    expect(readFileSync(second, "utf8")).toBe("build two");
    expect(existsSync(join(s.root, "nvs-run-old.exe"))).toBe(false);
    expect(existsSync(join(s.root, "nvs-run-old.exe.claim-999999999"))).toBe(false);
    expect(existsSync(join(s.root, "nvs-run-kept.exe"))).toBe(true);
    expect(existsSync(first)).toBe(true);
  } finally {
    s.cleanup();
  }
});
