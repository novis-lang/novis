// Which release archive this machine needs, which release it comes from, and getting it here intact.
//
// `rule:ide/the-extension-guides-an-install-and-never-bundles-one`: the `.vsix` carries no `nvs` on
// any platform, so a machine without one is offered the archive the release workflow already built
// for it. This file is that offer end to end: which archive the machine needs, which release to
// take it from, a download that hands back no bytes the release's own `SHA256SUMS` does not vouch
// for, and the unpack that puts the binary in the extension's own storage.
//
// It reads no configuration and starts nothing by itself — every function here is reached from a
// command a user invoked. The network is reached only through a `Transport` the caller supplies, so
// the suite drives the same code path the extension does with fixtures in place of a release, and
// `overHttps` is both the one the extension passes and the one thing under `test/install/` that no
// case calls.
//
// Nothing here imports `vscode`, for the reason `version.ts` gives: both halves are checkable in
// plain Node, and one of them is checkable from outside the extension entirely.
// `crates/nvs-lsp/tests/extension_release.rs` reads `TARGETS` below as *text* and holds it against
// `.github/workflows/release.yml`'s build matrix, because the two lists drifting apart is a 404 the
// user meets only after asking for an install. **That test parses one row per line with the fields
// in the order `Target` declares them** — a row wrapped across lines compiles and fails the pin.

import { createHash } from "node:crypto";
import { chmod, mkdir, rename, rm, writeFile } from "node:fs/promises";
import { get as httpsGet } from "node:https";
import { join } from "node:path";
import { gunzipSync, inflateRawSync } from "node:zlib";

import { parts, series } from "./version";

/** Which C library a Linux binary is linked against. Nowhere else is this a question. */
export type Libc = "gnu" | "musl";

/** One archive the release workflow builds, and the machine it is for. */
export interface Target {
  /** The workflow's own `matrix.name`, which is also the archive's filename after the version. */
  readonly name: string;
  /** The `process.platform` this row answers for. */
  readonly platform: string;
  /** The `process.arch` this row answers for. */
  readonly arch: string;
  /** The archive format, spelled as the workflow's `matrix.archive` spells it. */
  readonly archive: "tar.gz" | "zip";
  /** Which libc this row is linked against, on the one platform that has two of them. */
  readonly libc?: Libc;
}

/**
 * Every machine this extension can fetch a binary for.
 *
 * The names are `.github/workflows/release.yml`'s and are not this file's to choose. What is
 * missing is as deliberate as what is here: there is no musl row for `arm64`, because the workflow
 * builds no such archive, so an Alpine machine on ARM resolves to nothing — the same answer as any
 * other unsupported pair, and the same one command reports it.
 */
export const TARGETS: readonly Target[] = [
  { name: "linux-x86_64", platform: "linux", arch: "x64", archive: "tar.gz", libc: "gnu" },
  { name: "linux-aarch64", platform: "linux", arch: "arm64", archive: "tar.gz", libc: "gnu" },
  { name: "linux-x86_64-musl", platform: "linux", arch: "x64", archive: "tar.gz", libc: "musl" },
  { name: "windows-x86_64", platform: "win32", arch: "x64", archive: "zip" },
  { name: "windows-aarch64", platform: "win32", arch: "arm64", archive: "zip" },
  { name: "macos-x86_64", platform: "darwin", arch: "x64", archive: "tar.gz" },
  { name: "macos-aarch64", platform: "darwin", arch: "arm64", archive: "tar.gz" }
];

/**
 * The archive built for this `(platform, arch, libc)`, or `undefined` when there is none.
 *
 * A machine nothing was built for is a supported answer rather than an error: the caller says so in
 * the status item and offers the release page, which is a better end than a throw from activation.
 */
export function targetFor(platform: string, arch: string, libc: Libc | undefined): Target | undefined {
  return TARGETS.find(
    (target) => target.platform === platform && target.arch === arch && target.libc === libc
  );
}

/**
 * Which libc the running machine has, and `undefined` where the question does not arise.
 *
 * Node's own diagnostic report names the glibc it is linked against, and its absence is what a musl
 * system looks like from inside Node — there is no positive test, so this is the one that exists. A
 * report that cannot be read at all is read as the same absence.
 *
 * This is the *remote's* libc, not the editor's, and that is the point: the extension is
 * `extensionKind: ["workspace"]` (`rule:ide/the-extension-runs-where-the-binary-is`), so an Alpine
 * devcontainer answers here even when the window is a Windows VS Code.
 */
export function currentLibc(): Libc | undefined {
  if (process.platform !== "linux") {
    return undefined;
  }
  let glibc: string | undefined;
  try {
    const report = process.report?.getReport() as { header?: { glibcVersionRuntime?: string } } | undefined;
    glibc = report?.header?.glibcVersionRuntime;
  } catch {
    glibc = undefined;
  }
  return glibc === undefined ? "musl" : "gnu";
}

/** The archive built for the machine this is running on, or `undefined` when there is none. */
export function currentTarget(): Target | undefined {
  return targetFor(process.platform, process.arch, currentLibc());
}

/**
 * The file `version` was released as for `target`.
 *
 * `bun nv release --package` builds the name, and this is the same stem read from the other end:
 * `nvs-0.1.0-linux-x86_64.tar.gz`, `.zip` where the workflow says `zip`.
 */
export function archiveName(version: string, target: Target): string {
  return `nvs-${version}-${target.name}.${target.archive}`;
}

/**
 * The release number a tag names — `v0.1.0` and `0.1.0` both give `0.1.0`, and anything else gives
 * `undefined`.
 *
 * The releases carry a `v` (`bun nv release`'s `vX.Y.Z`) and the archives inside them do not, so
 * something has to strip it, and doing it here keeps the callers holding whichever form they need.
 */
export function releaseVersion(tag: string): string | undefined {
  const bare = tag.trim().startsWith("v") ? tag.trim().slice(1) : tag.trim();
  return parts(bare) === undefined ? undefined : bare;
}

/** Whether `left` is a later release than `right`. Both are bare release numbers. */
function isNewer(left: string, right: string): boolean {
  const a = parts(left);
  const b = parts(right);
  if (a === undefined || b === undefined) {
    return false;
  }
  if (a.major !== b.major) {
    return a.major > b.major;
  }
  if (a.minor !== b.minor) {
    return a.minor > b.minor;
  }
  if (a.patch !== b.patch) {
    return a.patch > b.patch;
  }
  // Same number, so one of them is a pre-release of the other: a plain release wins, and between
  // two pre-releases the later text does. `rc.10` sorts under `rc.9` for it, which is a wrong
  // answer this project cannot reach — a series never gets ten candidates.
  if ((a.pre === "") !== (b.pre === "")) {
    return a.pre === "";
  }
  return a.pre > b.pre;
}

/**
 * The newest release in `client`'s own `major.minor` series, as a bare release number, or
 * `undefined` when the list holds none.
 *
 * `tags` is a release list as GitHub reports it, newest first or in any order at all. The series is
 * the client's own because `rule:ide/the-extension-refuses-a-binary-it-does-not-understand` will
 * refuse anything else at `initialize`, and a download that ends in a refusal is a worse first run
 * than no offer at all. A release list with no matching series is the same kind of answer as an
 * unsupported platform: nothing, for the caller to report.
 */
export function newestInSeries(client: string, tags: readonly string[]): string | undefined {
  const wanted = series(client);
  if (wanted === undefined) {
    return undefined;
  }
  let best: string | undefined;
  for (const tag of tags) {
    const candidate = releaseVersion(tag);
    if (candidate === undefined || series(candidate) !== wanted) {
      continue;
    }
    if (best === undefined || isNewer(candidate, best)) {
      best = candidate;
    }
  }
  return best;
}

/** What a `Transport` answers with: the status the server gave, and the body it sent. */
export interface Fetched {
  readonly status: number;
  readonly bytes: Uint8Array;
}

/**
 * How this file reaches the network, in the one shape it needs: a URL in, the whole body out.
 *
 * It is a parameter rather than a call because the release this extension downloads from is the
 * only thing about the download that a case cannot supply. Everything else — which archive, which
 * release, whether the bytes match — is decided here, so a case that hands over a fixture archive
 * and a `SHA256SUMS` exercises the code the extension actually runs.
 */
export type Transport = (url: string) => Promise<Fetched>;

/** The release the archives and their `SHA256SUMS` are published to. */
const RELEASES = "https://github.com/novis-lang/novis/releases";

/** The file the release workflow's `sha256sum nvs-* > SHA256SUMS` step publishes beside them. */
export const SUMS = "SHA256SUMS";

/**
 * Where `file` sits in the release of `version`, which carries the `v` its archives do not.
 *
 * `bun nv release` tags `vX.Y.Z` and `.github/workflows/release.yml` uploads `dist/SHA256SUMS`
 * and `dist/nvs-*` to it, so both halves of a verified download are two names under one URL.
 */
export function assetUrl(version: string, file: string): string {
  return `${RELEASES}/download/v${version}/${file}`;
}

/**
 * The sha256 `sums` publishes for `file`, lower case, or `undefined` when it names no such file.
 *
 * `sums` is `sha256sum`'s own output: a hex digest, whitespace, and the name — with a `*` before
 * the name where the file was read as binary, which is a distinction only `sha256sum` itself cares
 * about. A line whose digest is not 64 hex characters is not a digest, and is passed over rather
 * than returned, because the caller's next move on a `undefined` is to refuse the download.
 */
export function expectedHash(sums: string, file: string): string | undefined {
  for (const line of sums.split("\n")) {
    const fields = line.trim().split(/\s+/);
    if (fields.length < 2 || fields.slice(1).join(" ").replace(/^\*/, "") !== file) {
      continue;
    }
    if (!/^[0-9a-fA-F]{64}$/.test(fields[0])) {
      continue;
    }
    return fields[0].toLowerCase();
  }
  return undefined;
}

/** The body of `url`, or a throw that names `file` and why it did not arrive. */
async function body(transport: Transport, url: string, file: string): Promise<Uint8Array> {
  let answer: Fetched;
  try {
    answer = await transport(url);
  } catch (cause) {
    const why = cause instanceof Error ? cause.message : String(cause);
    throw new Error(`${file} could not be downloaded from ${url}: ${why}`);
  }
  if (answer.status !== 200) {
    throw new Error(`${file} could not be downloaded from ${url}: the server answered ${answer.status}.`);
  }
  return answer.bytes;
}

/**
 * The archive of `version` for `target`, checked against the release's own `SHA256SUMS` before it
 * is handed back, or a throw naming the file that failed.
 *
 * The sums come first and from the same release, so a release that lists no archive for this
 * machine costs one small request rather than a download that is refused at the end of it. Nothing
 * here writes anything: a mismatch returns no bytes to a caller who has nothing to remove yet,
 * which is what makes "verify before unpacking" a property of the call rather than a discipline the
 * caller has to keep (`rule:ide/the-extension-guides-an-install-and-never-bundles-one`, and
 * [ADR 0155](../../../docs/decisions/0155.md) § 5).
 */
export async function downloadVerified(
  version: string,
  target: Target,
  transport: Transport
): Promise<Uint8Array> {
  const name = archiveName(version, target);
  const sums = new TextDecoder().decode(await body(transport, assetUrl(version, SUMS), SUMS));
  const wanted = expectedHash(sums, name);
  if (wanted === undefined) {
    throw new Error(
      `The release v${version} publishes no sha256 for ${name}, so there is nothing to check a download of it against.`
    );
  }
  const bytes = await body(transport, assetUrl(version, name), name);
  const found = createHash("sha256").update(bytes).digest("hex");
  if (found !== wanted) {
    throw new Error(
      `${name} does not match the sha256 the release v${version} publishes for it — expected ${wanted}, got ${found}. Nothing was kept.`
    );
  }
  return bytes;
}

/** What the binary is called inside an archive for `target`, and on disk once it is unpacked. */
export function executableName(target: Target): string {
  return target.platform === "win32" ? "nvs.exe" : "nvs";
}

/**
 * The `nvs` inside `archive`, or a throw saying which archive did not hold one.
 *
 * `bun nv release --package` puts the binary under a `nvs-<version>-<name>/` directory beside the
 * notices every archive ships, so this reads the one member it wants by name rather than extracting
 * a tree. Both formats are read here in plain Node — `zlib` is in the runtime and an archive reader
 * is not something to add a dependency for, when the extension's dependency list is a thing its own
 * suite asserts (`rule:ide/contributions-are-frozen-and-only-ever-added`).
 *
 * The archive is the one the caller already hashed against the release's `SHA256SUMS`: nothing here
 * is a defence against hostile bytes, and a malformed archive is a broken release, which throws.
 */
export function unpack(archive: Uint8Array, target: Target): Uint8Array {
  const exe = executableName(target);
  const found = target.archive === "zip" ? fromZip(archive, exe) : fromTarGz(archive, exe);
  if (found === undefined) {
    throw new Error(`The ${target.name} archive holds no ${exe}, so this release cannot be installed.`);
  }
  return found;
}

/** A NUL-terminated fixed-width field of a tar header, as the string it holds. */
function field(header: Buffer, from: number, to: number): string {
  const bytes = header.subarray(from, to);
  const end = bytes.indexOf(0);
  return bytes.toString("utf8", 0, end < 0 ? bytes.length : end).trim();
}

/**
 * Walk a gzipped tar's headers for a regular file named `exe`.
 *
 * A tar is 512-byte blocks: a header, its data rounded up to the next block, the next header, and
 * two zero blocks at the end. Anything that is not a regular file — the directory the archive is
 * rooted at, and the `pax_global_header` Python's `tarfile` writes — is stepped over by its own
 * recorded size, which is how a reader that understands two type flags reads an archive using more.
 */
function fromTarGz(archive: Uint8Array, exe: string): Uint8Array | undefined {
  const tar = gunzipSync(archive);
  for (let at = 0; at + 512 <= tar.length; ) {
    const header = tar.subarray(at, at + 512);
    const name = field(header, 0, 100);
    if (name === "") {
      return undefined;
    }
    const size = Number.parseInt(field(header, 124, 136), 8);
    if (!Number.isSafeInteger(size) || size < 0) {
      throw new Error(`A tar header in the archive gives no readable size for ${name}.`);
    }
    const data = at + 512;
    const kind = String.fromCharCode(header[156]);
    if ((kind === "0" || kind === "\0") && name.split("/").pop() === exe) {
      return tar.subarray(data, data + size);
    }
    at = data + Math.ceil(size / 512) * 512;
  }
  return undefined;
}

/**
 * Read a zip's central directory for an entry named `exe`.
 *
 * The central directory at the end of the file is the authority on what a zip holds; the local
 * header before each entry's bytes may legally omit the sizes. So the sizes and the method come
 * from the directory entry and only the two variable-length fields, which say where the data
 * starts, come from the local header.
 */
function fromZip(archive: Uint8Array, exe: string): Uint8Array | undefined {
  const view = Buffer.from(archive.buffer, archive.byteOffset, archive.byteLength);
  // The end-of-central-directory record is last, but a trailing comment of up to 64 KiB may follow
  // it, so it is found by scanning back for its signature rather than by arithmetic.
  let end = view.length - 22;
  while (end >= 0 && view.readUInt32LE(end) !== 0x06054b50) {
    end -= 1;
  }
  if (end < 0) {
    throw new Error("The archive has no zip end-of-central-directory record, so it is not a zip.");
  }
  const count = view.readUInt16LE(end + 10);
  let entry = view.readUInt32LE(end + 16);
  for (let i = 0; i < count; i += 1) {
    if (view.readUInt32LE(entry) !== 0x02014b50) {
      throw new Error("The archive's zip central directory ends earlier than its own entry count.");
    }
    const method = view.readUInt16LE(entry + 10);
    const compressed = view.readUInt32LE(entry + 20);
    const names = view.readUInt16LE(entry + 28);
    const extras = view.readUInt16LE(entry + 30);
    const comments = view.readUInt16LE(entry + 32);
    const local = view.readUInt32LE(entry + 42);
    const name = view.toString("utf8", entry + 46, entry + 46 + names);
    if (name.split("/").pop() === exe) {
      if (method !== 0 && method !== 8) {
        throw new Error(`${name} is stored with zip compression method ${method}, which this extension cannot read.`);
      }
      const at = local + 30 + view.readUInt16LE(local + 26) + view.readUInt16LE(local + 28);
      const data = view.subarray(at, at + compressed);
      return method === 0 ? data : inflateRawSync(data);
    }
    entry += 46 + names + extras + comments;
  }
  return undefined;
}

/**
 * Unpack `archive` into `dir` and return the path of the binary now sitting there.
 *
 * `dir` is the extension's own `globalStorageUri` and never anywhere on `PATH`: a copy this
 * extension installed is the last candidate the client tries and is not written into `nvs.path`,
 * so a toolchain the user installs later still wins
 * (`rule:ide/the-extension-guides-an-install-and-never-bundles-one`).
 *
 * The bytes land under a `.part` name and are renamed once they are whole and executable, so an
 * interrupted install leaves no half-written file for the resolution chain to find and run. The
 * mode is set here rather than taken from the archive because a zip records one this reader does
 * not read, and a tar's is not a mode the extraction applied.
 */
export async function installBinary(archive: Uint8Array, target: Target, dir: string): Promise<string> {
  const binary = unpack(archive, target);
  const path = join(dir, executableName(target));
  const partial = `${path}.part`;
  await mkdir(dir, { recursive: true });
  try {
    await writeFile(partial, binary);
    if (target.platform !== "win32") {
      await chmod(partial, 0o755);
    }
    await rename(partial, path);
  } catch (cause) {
    await rm(partial, { force: true });
    throw cause;
  }
  return path;
}

/** How many redirects `overHttps` follows before it decides the release is not answering. */
const HOPS = 5;

/**
 * The transport the extension itself passes: HTTPS, following redirects.
 *
 * A release asset is a redirect by design — the URL is on `github.com` and the bytes are on
 * `objects.githubusercontent.com` behind a signed query — so a transport that does not follow one
 * downloads an empty body and a 302. `node:https` is used rather than a client library because the
 * extension adds no dependency it can avoid, and because the redirect is the only thing a client
 * library would be doing here.
 */
export const overHttps: Transport = (url: string) => hop(url, HOPS);

function hop(url: string, left: number): Promise<Fetched> {
  return new Promise((resolve, reject) => {
    const request = httpsGet(url, { headers: { "user-agent": "novis-vscode" } }, (response) => {
      const status = response.statusCode ?? 0;
      const location = response.headers.location;
      if (status >= 300 && status < 400 && location !== undefined) {
        response.resume();
        if (left === 0) {
          reject(new Error(`${url} redirects further than ${HOPS} hops.`));
          return;
        }
        hop(new URL(location, url).toString(), left - 1).then(resolve, reject);
        return;
      }
      const chunks: Buffer[] = [];
      response.on("data", (chunk: Buffer) => chunks.push(chunk));
      response.on("error", reject);
      response.on("end", () => resolve({ status, bytes: Buffer.concat(chunks) }));
    });
    request.on("error", reject);
  });
}
