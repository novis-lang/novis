// Which release archive this machine needs, and which release it comes from.
//
// `rule:ide/the-extension-guides-an-install-and-never-bundles-one`: the `.vsix` carries no `nvs` on
// any platform, so a machine without one is offered the archive the release workflow already built
// for it. This file is the two decisions that offer needs — the machine's archive, and the release
// to take it from — and nothing else. It reads no configuration, touches no disk and reaches no
// network: a caller that has decided to fetch something asks these functions what to fetch.
//
// Nothing here imports `vscode`, for the reason `version.ts` gives: both halves are checkable in
// plain Node, and one of them is checkable from outside the extension entirely.
// `crates/nvs-lsp/tests/extension_release.rs` reads `TARGETS` below as *text* and holds it against
// `.github/workflows/release.yml`'s build matrix, because the two lists drifting apart is a 404 the
// user meets only after asking for an install. **That test parses one row per line with the fields
// in the order `Target` declares them** — a row wrapped across lines compiles and fails the pin.

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
 * `tools/release.py --package` builds the name, and this is the same stem read from the other end:
 * `nvs-0.1.0-linux-x86_64.tar.gz`, `.zip` where the workflow says `zip`.
 */
export function archiveName(version: string, target: Target): string {
  return `nvs-${version}-${target.name}.${target.archive}`;
}

/**
 * The release number a tag names — `v0.1.0` and `0.1.0` both give `0.1.0`, and anything else gives
 * `undefined`.
 *
 * The releases carry a `v` (`tools/release.py`'s `vX.Y.Z`) and the archives inside them do not, so
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
