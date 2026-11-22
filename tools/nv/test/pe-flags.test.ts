// `bun nv pe-flags` on headers built in memory: one with every protection, one lacking each, and files
// that are not PE32+ images. No real binary is read.

import { describe, expect, test } from "bun:test";
import { inspect, REQUIRED_FLAGS, STACK_RESERVE } from "../cmd/pe-flags.ts";

const PE = 0x80;
const OPTIONAL = PE + 24;
const OPTIONAL_SIZE = 240;

type Shape = { flags?: number; stack?: number; resource?: boolean; magic?: number };

/** A DOS stub, a PE signature, a COFF header and a PE32+ optional header with 16 data directories. */
function image({ flags = 0x0160, stack = STACK_RESERVE, resource = true, magic = 0x20b }: Shape = {}): Uint8Array {
  const bytes = new Uint8Array(OPTIONAL + OPTIONAL_SIZE);
  const view = new DataView(bytes.buffer);
  view.setUint16(0, 0x5a4d, true);
  view.setUint32(0x3c, PE, true);
  view.setUint32(PE, 0x4550, true);
  view.setUint16(PE + 4, 0x8664, true);
  view.setUint16(PE + 20, OPTIONAL_SIZE, true);
  view.setUint16(OPTIONAL, magic, true);
  view.setUint16(OPTIONAL + 70, flags, true);
  view.setBigUint64(OPTIONAL + 72, BigInt(stack), true);
  view.setUint32(OPTIONAL + 108, 16, true);
  if (resource) {
    view.setUint32(OPTIONAL + 112 + 16, 0x5000, true);
    view.setUint32(OPTIONAL + 112 + 20, 0x400, true);
  }
  return bytes;
}

describe("pe-flags", () => {
  test("a PE32+ header with every protection passes", () => {
    expect(inspect(image())).toEqual({ missing: [] });
  });

  test("each missing DllCharacteristics flag is named", () => {
    for (const [name, bit] of REQUIRED_FLAGS) {
      expect(inspect(image({ flags: 0x0160 & ~bit }))).toEqual({ missing: [name] });
    }
  });

  test("a stack reserve other than 8 MiB is named with the size found", () => {
    expect(inspect(image({ stack: 1024 * 1024 }))).toEqual({
      missing: [`SizeOfStackReserve of ${STACK_RESERVE} (found 1048576)`],
    });
  });

  test("an empty resource directory is named", () => {
    expect(inspect(image({ resource: false }))).toEqual({
      missing: ["a resource directory (icon and version information)"],
    });
  });

  test("a header lacking everything names all five", () => {
    const verdict = inspect(image({ flags: 0, stack: 0, resource: false }));
    expect("missing" in verdict && verdict.missing.length).toBe(5);
  });

  test("a file that is not PE32+ is refused with its reason", () => {
    expect(inspect(image({ magic: 0x10b }))).toEqual({ refused: "optional header magic 0x10b, not PE32+ (0x20b)" });
    expect(inspect(new TextEncoder().encode("#!/bin/sh\necho not a binary\n".padEnd(80)))).toEqual({
      refused: "no MZ header",
    });
    const noSignature = image();
    noSignature[PE] = 0;
    expect(inspect(noSignature)).toEqual({ refused: "no PE signature" });
    expect(inspect(image().slice(0, OPTIONAL + 40))).toEqual({ refused: "optional header is truncated" });
  });
});
