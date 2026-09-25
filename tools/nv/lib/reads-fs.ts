// `node:fs` for a process `lib/reads.ts` records: every call that reads, lists or tests a path notes it
// first. Only the calls that read are wrapped; the rest are `node:fs`'s own.

import * as fs from "node:fs";
import { reading } from "./reads.ts";

export * from "node:fs";

export const readFileSync = reading("files", fs.readFileSync);
export const readFile = reading("files", fs.readFile);
export const openSync = reading("files", fs.openSync);
export const open = reading("files", fs.open);
export const createReadStream = reading("files", fs.createReadStream);
export const statSync = reading("files", fs.statSync);
export const stat = reading("files", fs.stat);
export const lstatSync = reading("files", fs.lstatSync);
export const lstat = reading("files", fs.lstat);
export const readlinkSync = reading("files", fs.readlinkSync);
export const existsSync = reading("exists", fs.existsSync);
export const accessSync = reading("exists", fs.accessSync);
export const access = reading("exists", fs.access);
export const readdirSync = reading("dirs", fs.readdirSync);
export const readdir = reading("dirs", fs.readdir);
export const opendirSync = reading("dirs", fs.opendirSync);

export default {
  ...fs,
  readFileSync,
  readFile,
  openSync,
  open,
  createReadStream,
  statSync,
  stat,
  lstatSync,
  lstat,
  readlinkSync,
  existsSync,
  accessSync,
  access,
  readdirSync,
  readdir,
  opendirSync,
};
