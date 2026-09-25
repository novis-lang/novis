// The release the install suite downloads from: archives built in memory, their `SHA256SUMS`, and a
// transport that serves them.
//
// No case here opens a socket. `install.ts` takes its transport as a parameter for exactly this
// reason, so what these fixtures replace is the release — not the fetch, the hash check or the
// unpack, all of which are the code the extension runs.
//
// The archives are written by hand because the two formats are what is under test: a reader is only
// worth its lines if the bytes it is given are shaped like the ones `bun nv release --package`
// produces. So `tarGz` writes ustar headers with real checksums and `zip` writes a central
// directory, and the cases put a directory entry and a global header in front of the binary the way
// Python's `tarfile` does.

import { createHash } from "node:crypto";
import { deflateRawSync, gzipSync } from "node:zlib";

import { Fetched, Target, TARGETS, Transport } from "../../src/install";

/** One member of an archive: its path inside the archive and, for a file, its bytes. */
export interface Entry {
  readonly name: string;
  readonly data?: Buffer;
  /** A tar type flag — `0` is a file, `5` a directory, `g` a `pax_global_header`. */
  readonly kind?: string;
}

/** The row of `TARGETS` called `name`, so a case names a machine without depending on its own. */
export function target(name: string): Target {
  const found = TARGETS.find((row) => row.name === name);
  if (found === undefined) {
    throw new Error(`there is no ${name} target`);
  }
  return found;
}

/** One 512-byte ustar header, checksummed the way the format defines it. */
function header(name: string, size: number, kind: string): Buffer {
  const block = Buffer.alloc(512, 0);
  block.write(name, 0, 100, "utf8");
  block.write("0000755\0", 100, 8, "utf8");
  block.write("0000000\0", 108, 8, "utf8");
  block.write("0000000\0", 116, 8, "utf8");
  block.write(`${size.toString(8).padStart(11, "0")}\0`, 124, 12, "utf8");
  block.write("00000000000 ", 136, 12, "utf8");
  // The checksum is computed with its own eight bytes read as spaces, then written over them.
  block.write("        ", 148, 8, "utf8");
  block.write(kind, 156, 1, "utf8");
  block.write("ustar\0" + "00", 257, 8, "utf8");
  let sum = 0;
  for (const byte of block) {
    sum += byte;
  }
  block.write(`${sum.toString(8).padStart(6, "0")}\0 `, 148, 8, "utf8");
  return block;
}

/** `entries` as a gzipped tar, ended by the two zero blocks a tar ends with. */
export function tarGz(entries: readonly Entry[]): Buffer {
  const blocks: Buffer[] = [];
  for (const entry of entries) {
    const data = entry.data ?? Buffer.alloc(0);
    blocks.push(header(entry.name, data.length, entry.kind ?? "0"));
    if (data.length > 0) {
      const padded = Buffer.alloc(Math.ceil(data.length / 512) * 512);
      data.copy(padded);
      blocks.push(padded);
    }
  }
  blocks.push(Buffer.alloc(1024));
  return gzipSync(Buffer.concat(blocks));
}

/**
 * `entries` as a zip, deflated like the release workflow's or stored, as `method` says.
 *
 * The CRC fields are left zero: the reader under test checks the sha256 of the whole archive
 * instead, which is a stronger statement about the same bytes, so writing a real CRC here would
 * assert nothing this suite is about.
 */
export function zip(entries: readonly Entry[], method: 0 | 8 = 8): Buffer {
  const locals: Buffer[] = [];
  const directory: Buffer[] = [];
  let offset = 0;
  for (const entry of entries) {
    const raw = entry.data ?? Buffer.alloc(0);
    const body = method === 8 ? deflateRawSync(raw) : raw;
    const name = Buffer.from(entry.name, "utf8");

    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(method, 8);
    local.writeUInt32LE(body.length, 18);
    local.writeUInt32LE(raw.length, 22);
    local.writeUInt16LE(name.length, 26);
    locals.push(local, name, body);

    const listed = Buffer.alloc(46);
    listed.writeUInt32LE(0x02014b50, 0);
    listed.writeUInt16LE(20, 6);
    listed.writeUInt16LE(method, 10);
    listed.writeUInt32LE(body.length, 20);
    listed.writeUInt32LE(raw.length, 24);
    listed.writeUInt16LE(name.length, 28);
    listed.writeUInt32LE(offset, 42);
    directory.push(listed, name);

    offset += 30 + name.length + body.length;
  }
  const listing = Buffer.concat(directory);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(listing.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, listing, end]);
}

/** `sha256sum`'s own output for `files`, which is what the release publishes as `SHA256SUMS`. */
export function sums(files: Readonly<Record<string, Uint8Array>>): string {
  return Object.entries(files)
    .map(([name, bytes]) => `${createHash("sha256").update(bytes).digest("hex")}  ${name}\n`)
    .join("");
}

/** A transport and the record of what was asked of it, which is half of what a case asserts. */
export interface Release {
  readonly transport: Transport;
  /** Every URL the code under test requested, in order. */
  readonly asked: string[];
}

/**
 * A release serving `files` by name, and a 404 for anything else.
 *
 * The names are bare file names, not URLs, so a case says what the release holds and the code under
 * test is left to say where it expects to find it.
 */
export function serving(files: Readonly<Record<string, Uint8Array | string>>): Release {
  const asked: string[] = [];
  const transport: Transport = (url: string) => {
    asked.push(url);
    const name = url.slice(url.lastIndexOf("/") + 1);
    const found = files[name];
    if (found === undefined) {
      return Promise.resolve<Fetched>({ status: 404, bytes: Buffer.alloc(0) });
    }
    return Promise.resolve<Fetched>({
      status: 200,
      bytes: typeof found === "string" ? Buffer.from(found, "utf8") : found
    });
  };
  return { transport, asked };
}
