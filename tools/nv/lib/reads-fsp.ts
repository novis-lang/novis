// `node:fs/promises` for a process `lib/reads.ts` records: every call that reads, lists or tests a path
// notes it first. Only the calls that read are wrapped; the rest are `node:fs/promises`'s own.

import * as fsp from "node:fs/promises";
import { reading } from "./reads.ts";

export * from "node:fs/promises";

export const readFile = reading("files", fsp.readFile);
export const open = reading("files", fsp.open);
export const stat = reading("files", fsp.stat);
export const lstat = reading("files", fsp.lstat);
export const readlink = reading("files", fsp.readlink);
export const access = reading("exists", fsp.access);
export const readdir = reading("dirs", fsp.readdir);
export const opendir = reading("dirs", fsp.opendir);

export default { ...fsp, readFile, open, stat, lstat, readlink, access, readdir, opendir };
