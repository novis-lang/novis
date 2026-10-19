// The kit's bun preload, passed to `bun test --preload` by the bun observer in `bun.ts`. It records
// every module the run imported and every path it read or listed through `node:fs` or `Bun.file`, and
// after the last test writes them to the file `LOOPER_READS` names. It ports looper's
// `crates/looper-select/src/bun-preload.js`.
//
// - The record is a `looper-reads 1` line, then one line per distinct path: `m` for a module the run
//   imported, `r` for a path it read or probed, `d` for a directory it listed.
// - It patches `node:fs` through `require`, before any ESM import of it, because an ESM namespace is
//   built once from the module object and a later patch never reaches a named import. So this file
//   imports nothing from `node:fs` itself.

import { afterAll } from "bun:test";

type Fs = typeof import("node:fs");
type FsPromises = typeof import("node:fs/promises");
type Path = typeof import("node:path");
type Url = typeof import("node:url");

const fs = require("node:fs") as Fs;
const fsp = require("node:fs/promises") as FsPromises;
const path = require("node:path") as Path;
const { fileURLToPath } = require("node:url") as Url;

const out = process.env.LOOPER_READS;
const reads = new Set<string>();
const lists = new Set<string>();

function absolute(p: unknown): string | null {
  if (typeof p === "string") return p.startsWith("file:") ? fileURLToPath(p) : path.resolve(p);
  if (p instanceof URL) return p.protocol === "file:" ? fileURLToPath(p) : null;
  if (Buffer.isBuffer(p)) return path.resolve(p.toString());
  return null;
}

const patched = new Set<unknown>();
function wrap(target: object, names: string[], into: Set<string>): void {
  const t = target as Record<string, unknown>;
  for (const name of names) {
    const original = t[name];
    if (typeof original !== "function" || patched.has(original)) continue;
    const wrapper = function (this: unknown, p: unknown, ...rest: unknown[]) {
      const at = absolute(p);
      if (at) into.add(at);
      return original.call(this, p, ...rest);
    };
    patched.add(wrapper);
    t[name] = wrapper;
  }
}

const readNames = ["readFile", "stat", "lstat", "access", "open", "exists", "createReadStream"];
const listNames = ["readdir", "opendir"];
wrap(fs, [...readNames, ...readNames.map((n) => `${n}Sync`)], reads);
wrap(fs, [...listNames, ...listNames.map((n) => `${n}Sync`)], lists);
for (const promises of new Set<object>([fs.promises, fsp])) {
  wrap(promises, readNames, reads);
  wrap(promises, listNames, lists);
}
wrap(Bun, ["file"], reads);

afterAll(() => {
  if (!out) return;
  const lines = ["looper-reads 1"];
  for (const m of Object.keys(require.cache)) if (path.isAbsolute(m)) lines.push(`m ${m}`);
  for (const r of reads) lines.push(`r ${r}`);
  for (const d of lists) lines.push(`d ${d}`);
  fs.writeFileSync(out, `${lines.join("\n")}\n`);
});
