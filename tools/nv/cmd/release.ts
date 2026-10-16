// `bun nv release`: the release cycle, as one command per step.
//
// `.github/workflows/release.yml` is the schedule, and this file is every decision it makes. The
// workflow holds no version arithmetic, no changelog rules and no packaging logic, for the reason
// `nv verify` holds the verification steps: YAML that computes things can only be tested by pushing
// it, and a release is the one pipeline you cannot afford to debug in production.
//
// Everything here runs locally against a clone. `--preview` prints the exact notes and the exact
// version a dispatch would produce, and writes nothing:
//
//     bun nv release --preview major
//
// The four steps, in the order the workflow runs them:
//
//     bun nv release --plan major --github-output    # 1. the version math, no writes
//     bun nv release --notes 0.1.0 --out notes.md    # 2. render the changelog section
//     bun nv release --package ... --out dist/       # 3. archive a built binary
//     bun nv release --apply 0.1.0 --notes-file notes.md  # 4. rewrite the manifests, prepend
//
// `--check` is the gate: the workspace version, every `[workspace.dependencies]` path dependency's
// version pin and the newest tag agree, and `CHANGELOG.md` has a section for a released version.
// `--docker-tags` prints the registry tags one image variant claims.
//
// **The version scheme is not plain SemVer, and this is not the place that decides it.**
// `rule:packaging/below-1-0-the-breaking-slot-moves-left` owns it: one number for the whole workspace,
// and before 1.0 the breaking slot moves left by one, so `--plan major` on 0.0.1 gives 0.1.0 and below
// 1.0 `minor` and `patch` are the same increment. `--plan` will not reach 0.1.0 without
// `--allow-contract`, because `rule:packaging/the-version-contract-starts-at-0-1-0` makes that release
// the one that throws the switch to the version contract, and that is a person's decision.
//
// **What a version number lives in.** `[workspace.package]` carries the version every crate inherits,
// and `[workspace.dependencies]` restates it on each path dependency, because a path dependency needs a
// version to be publishable. `--apply` finds the pins by reading the manifest as TOML, so it bumps
// exactly the set `--check` checks, and edits only the lines that hold them. It then reads the result
// back and refuses a manifest where any pin still differs, and `cargo metadata` must read the workspace
// or the manifest is put back. `cargo update --workspace --offline` rewrites `Cargo.lock` for workspace
// members only, so a release cannot carry a dependency bump — that pass is a person's, per
// `docs/agent/dependency-update.md`.
//
// **The notes are compacted.** `git log` here is written by an unattended loop and is largely
// bookkeeping. So `feat`, `fix` and `perf` are listed in full and grouped by scope, anything breaking
// is lifted to the top, a subject that is not a conventional commit is listed under *Other*, and every
// other type collapses into one *Internal* line with per-type counts. The counts add up to the commit
// total, and the compare link at the foot reaches the rest.
//
// **This never publishes.** `--apply` writes the tree and stops. Tagging, pushing and the GitHub draft
// are the workflow's, because they need credentials, and this command must run for anyone with a clone
// and no secrets. No code path here reads a token.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync, appendFileSync } from "node:fs";
import { join, resolve as resolvePath } from "node:path";
import { deflateRawSync, gzipSync } from "node:zlib";
import { parse as parseToml } from "smol-toml";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs, pyInt } from "../lib/py.ts";

export const summary = "the release cycle: nv release --plan|--preview|--notes|--apply|--package|--check|--docker-tags";

const USAGE =
  "usage: nv release [-h] (--plan {patch,minor,major} | --preview {patch,minor,major} | --notes VERSION | " +
  "--apply VERSION | --package | --check | --docker-tags VERSION) [options]";

class Fatal extends Error {}

const die = (message: string): never => {
  throw new Fatal(`nv release: ${message}`);
};

// Shipped beside the binary in every archive. THIRD-PARTY-LICENSES.txt is not optional:
// `rule:packaging/the-third-party-notice-is-generated-never-written-by-hand` makes it the notice this
// project owes at distribution, and an archive is distribution. A missing one fails the packaging step.
export const ARCHIVE_EXTRAS = ["README.md", "LICENSE", "THIRD-PARTY-LICENSES.txt", "CHANGELOG.md"];

export const CHANGELOG_HEADER = `# Changelog

Every released version of Novis, newest first. Generated from the commit log by
\`bun nv release\` and prepended by the release workflow -- edit a section only to correct it,
never to add one by hand.

What a version number promises is \`rule:packaging/the-versioned-surface-is-enumerated\` and \`rule:packaging/below-1-0-the-breaking-slot-moves-left\`
: it covers the language, the \`Core\` library, \`nvs.toml\`, the CLI, diagnostic identity, the
extension ABI and \`serialize()\` output -- and explicitly not the Rust APIs of the \`nvs-*\` crates.
Before 1.0 the breaking slot moves left: \`0.MINOR\` carries breaking changes.

A release is cut by [the release workflow](.github/workflows/release.yml), fired by hand from the
Actions tab; [docs/release.md](docs/release.md) is the procedure and what it needs configured.
`;

// `feat(scope)!: subject` / `fix: subject`. The `!` and the scope are both optional. A subject that
// does not match is listed under *Other*.
const CONVENTIONAL = /^(?<type>[a-z]+)(?:\((?<scope>[^)]*)\))?(?<bang>!)?: (?<subject>.+)$/;

// Listed in full, in this order. Every other type collapses into the Internal count.
const HEADLINE_SECTIONS: [string, string][] = [
  ["feat", "Features"],
  ["fix", "Fixes"],
  ["perf", "Performance"],
];

// The most entries one section lists before the rest turn into a count. A GitHub release body is
// capped at 125,000 characters, and this repository's whole history rendered unbounded is past it,
// so the first release would be the one that failed. `--max-per-section` raises it.
export const SECTION_CAP = 100;

// Suffix per runtime base. The default image has none, so `novis:0.4.1` is the one most people type.
// The variant appends rather than prefixes, so `latest-debian` sorts beside `latest` in a registry.
export const DOCKER_VARIANTS: Record<string, string> = { distroless: "", debian: "-debian" };

const BUMPS = ["patch", "minor", "major"];

// ---------------------------------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------------------------------

export type Version = [number, number, number];

export function parseVersion(version: string): Version {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(version.trim());
  if (!m) die(`'${version}' is not MAJOR.MINOR.PATCH`);
  return [Number(m![1]), Number(m![2]), Number(m![3])];
}

export function compareVersions(a: Version, b: Version): number {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i]! - b[i]!;
  return 0;
}

/** `rule:packaging/below-1-0-the-breaking-slot-moves-left`'s scheme, which is SemVer only from 1.0. */
export function nextVersion(current: string, bump: string): string {
  const [major, minor, patch] = parseVersion(current);
  // Below 1.0 the breaking slot is MINOR, so no slot is left for a compatible feature to differ from
  // a fix in, and `minor` and `patch` give the same number.
  if (major === 0) return bump === "major" ? `0.${minor + 1}.0` : `0.${minor}.${patch + 1}`;
  if (bump === "major") return `${major + 1}.0.0`;
  if (bump === "minor") return `${major}.${minor + 1}.0`;
  return `${major}.${minor}.${patch + 1}`;
}

export interface Tag {
  version: Version;
  name: string;
}

/** Every `vX.Y.Z` in `git tag --list` output, newest version first. Any other tag is not a release. */
export function releaseTags(listing: string): Tag[] {
  const found: Tag[] = [];
  for (const raw of listing.split("\n")) {
    const name = raw.trim();
    if (/^v\d+\.\d+\.\d+$/.test(name)) found.push({ version: parseVersion(name.slice(1)), name });
  }
  return found.sort((a, b) => compareVersions(b.version, a.version));
}

// ---------------------------------------------------------------------------------------------------
// The manifest
// ---------------------------------------------------------------------------------------------------

type Table = Record<string, unknown>;

function table(value: unknown): Table {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? (value as Table) : {};
}

/** The version under `[workspace.package]`, the one every crate inherits. */
export function workspaceVersion(manifest: string): string {
  const version = table(table(table(parseToml(manifest)).workspace).package).version;
  if (typeof version !== "string") die("Cargo.toml has no version under [workspace.package]");
  return version as string;
}

/**
 * Every `[workspace.dependencies]` entry that has both a `path` and a `version`, with that version.
 * A path under the workspace's `exclude` is a package with its own version, so it is not a pin.
 */
export function versionPins(manifest: string): [string, string][] {
  const workspace = table(table(parseToml(manifest)).workspace);
  const deps = table(workspace.dependencies);
  const excluded = Array.isArray(workspace.exclude) ? workspace.exclude.filter((e) => typeof e === "string") : [];
  const outside = (path: string) => excluded.some((e) => path === e || path.startsWith(`${e}/`));
  const pins: [string, string][] = [];
  for (const [name, spec] of Object.entries(deps)) {
    const t = table(spec);
    if (typeof t.path === "string" && typeof t.version === "string" && !outside(t.path)) pins.push([name, t.version]);
  }
  return pins;
}

/** What disagrees with the workspace version: each pin that differs, one line each. */
export function pinProblems(manifest: string): string[] {
  const version = workspaceVersion(manifest);
  return versionPins(manifest)
    .filter(([, pinned]) => pinned !== version)
    .map(([name, pinned]) => `${name} is pinned at ${pinned}, workspace is ${version}`);
}

const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * The manifest with the workspace version and every pin `versionPins` finds set to `version`, and the
 * number of fields that changed hands. Each edit is scoped to one line: the `version` line under
 * `[workspace.package]`, and the line that opens each pinned dependency. The result is read back, and
 * a pin this edit could not reach, such as one written as a table of its own, is an error rather than
 * a release with a stale pin in it.
 */
export function bumpManifest(manifest: string, version: string): { text: string; changed: number } {
  parseVersion(version);
  const pinned = new Set(versionPins(manifest).map(([name]) => name));
  const lines = manifest.split("\n");
  let section = "";
  let changed = 0;
  let packageDone = false;
  const setVersion = (line: string) => line.replace(/(\bversion\s*=\s*)"[^"]*"/, `$1"${version}"`);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    const header = /^\s*\[([^\]]+)\]\s*(#.*)?\r?$/.exec(line);
    if (header) {
      section = header[1]!.trim();
      continue;
    }
    if (section === "workspace.package" && !packageDone && /^\s*version\s*=/.test(line)) {
      lines[i] = setVersion(line);
      packageDone = true;
      changed++;
      continue;
    }
    if (section !== "workspace.dependencies") continue;
    const key = /^\s*"?([A-Za-z0-9_-]+)"?\s*=/.exec(line)?.[1];
    if (key && pinned.has(key) && new RegExp(`^\\s*"?${escapeRe(key)}"?\\s*=\\s*\\{.*\\bversion\\s*=`).test(line)) {
      lines[i] = setVersion(line);
      pinned.delete(key);
      changed++;
    }
  }
  if (!packageDone) die("could not find the version under [workspace.package] to rewrite");
  const text = lines.join("\n");
  const stale = pinProblems(text);
  if (workspaceVersion(text) !== version || stale.length > 0 || pinned.size > 0) {
    die(`the rewritten manifest still disagrees with ${version}; its shape changed:\n  ${[...stale, ...[...pinned].map((n) => `${n} is not an inline table on one line`)].join("\n  ")}`);
  }
  return { text, changed };
}

/** Insert `section` above the newest release, or start the file with the header on a first release. */
export function prependChangelog(existing: string | null, section: string): string {
  if (existing === null) return `${CHANGELOG_HEADER}\n${section}`;
  const marker = /^## \[/m.exec(existing);
  if (marker) return existing.slice(0, marker.index) + section + "\n" + existing.slice(marker.index);
  return existing.replace(/\n+$/, "") + "\n\n" + section;
}

// ---------------------------------------------------------------------------------------------------
// Container image tags
// ---------------------------------------------------------------------------------------------------

/**
 * The registry tags one variant of one release claims, the variant's own version tag first.
 *
 * Here rather than in `docker/metadata-action`, because that action's `type=semver` is SemVer, and
 * below 1.0 `0.MINOR` carries a breaking change. A bare `0` tag would follow 0.0 to 0.1 across that
 * break, which a floating tag must never do. So the compatible line is always MAJOR.MINOR, and MAJOR
 * alone is a tag only from 1.0.
 *
 * `floating` adds the tags that move. The release run pushes only the fixed ones, and the promote
 * workflow adds these when a person publishes the draft, so `latest` never names a version whose notes
 * nobody can read yet. A version older than the newest tag gets no moving tags, so publishing an old
 * draft late cannot move `latest` backwards; `behind` names that tag so the caller can say so.
 */
export function dockerTags(
  version: string,
  image: string,
  variant: string,
  floating: boolean,
  short: string,
  known: Tag[],
): { tags: string[]; behind: string | null } {
  if (!(variant in DOCKER_VARIANTS)) die(`'${variant}' is not a known image variant (${Object.keys(DOCKER_VARIANTS).sort().join(", ")})`);
  const [major, minor] = parseVersion(version);
  const suffix = DOCKER_VARIANTS[variant]!;
  const chosen = [`${version}${suffix}`, `sha-${short}${suffix}`];
  let behind: string | null = null;
  if (floating) {
    if (known.length > 0 && compareVersions(parseVersion(version), known[0]!.version) < 0) behind = known[0]!.name;
    else {
      chosen.push(`${major}.${minor}${suffix}`);
      if (major >= 1) chosen.push(`${major}${suffix}`);
      chosen.push(`latest${suffix}`);
    }
  }
  return { tags: chosen.map((tag) => `${image}:${tag}`), behind };
}

// ---------------------------------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------------------------------

export interface Commit {
  sha: string;
  subject: string;
  body: string;
}

export interface Entry extends Commit {
  type: string | null;
  scope: string | null;
  clean: string;
  breaking: boolean;
}

/** The records of `git log --format=%H%x1f%s%x1f%b%x1e`, oldest first as git gave them. The unit and
 *  record separators are used because a body contains newlines and pipes, and the body is what
 *  carries `BREAKING CHANGE:`. */
export function parseLog(raw: string): Commit[] {
  const out: Commit[] = [];
  for (let record of raw.split("\x1e")) {
    record = record.replace(/^\n+|\n+$/g, "");
    if (record.trim() === "") continue;
    const [sha = "", subject = "", body = ""] = record.split("\x1f");
    out.push({ sha: sha.trim(), subject: subject.trim(), body });
  }
  return out;
}

export function classify(commit: Commit): Entry {
  const m = CONVENTIONAL.exec(commit.subject);
  const breaking = commit.body.includes("BREAKING CHANGE:") || commit.body.includes("BREAKING-CHANGE:");
  if (!m) return { ...commit, type: null, scope: null, clean: commit.subject, breaking };
  const g = m.groups!;
  return { ...commit, type: g.type!, scope: (g.scope ?? "").trim() || null, clean: g.subject!, breaking: breaking || g.bang !== undefined };
}

function bullet(entry: Entry, url: string): string {
  const short = entry.sha.slice(0, 8);
  const link = url ? ` ([\`${short}\`](${url}/commit/${entry.sha}))` : ` (\`${short}\`)`;
  const scope = entry.scope ? `**${entry.scope}**: ` : "";
  return `- ${scope}${entry.clean}${link}`;
}

const byCodePoint = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

/** One version's changelog section, dated `today`. */
export function renderNotes(
  version: string,
  previousTag: string | null,
  entries: Entry[],
  url: string,
  today: string,
  cap: number = SECTION_CAP,
): string {
  const lines = [`## [${version}] - ${today}`, ""];
  if (entries.length === 0) return [...lines, "No commits since the previous release.", ""].join("\n");

  const listing = (rows: Entry[]): string[] => {
    const ordered = [...rows].sort((a, b) => byCodePoint(a.scope ?? "", b.scope ?? "") || byCodePoint(a.clean, b.clean));
    const out = ordered.slice(0, cap).map((row) => bullet(row, url));
    if (ordered.length > cap) out.push(`- …and ${ordered.length - cap} more, in the full changes below.`);
    return out;
  };
  const section = (title: string, rows: Entry[]) => {
    if (rows.length === 0) return;
    lines.push(`### ${title}`, "", ...listing(rows), "");
  };

  const listed = new Set<Entry>();
  const take = (pick: (e: Entry) => boolean): Entry[] => {
    const rows = entries.filter((e) => !listed.has(e) && pick(e));
    for (const row of rows) listed.add(row);
    return rows;
  };

  const breaking = take((e) => e.breaking);
  if (breaking.length > 0) {
    lines.push(
      "### Breaking changes",
      "",
      "What a break may and may not be is `rule:packaging/the-versioned-surface-is-enumerated`, `rule:packaging/below-1-0-the-breaking-slot-moves-left` and `rule:packaging/who-can-see-it-decides-the-release-slot`.",
      "",
      ...listing(breaking),
      "",
    );
  }
  for (const [kind, title] of HEADLINE_SECTIONS) section(title, take((e) => e.type === kind));
  section("Other", take((e) => e.type === null));

  // Everything left is bookkeeping: counted, never listed.
  const rest = take(() => true);
  if (rest.length > 0) {
    const counts = new Map<string, number>();
    for (const e of rest) counts.set(e.type!, (counts.get(e.type!) ?? 0) + 1);
    const tally = [...counts].sort((a, b) => b[1] - a[1]).map(([kind, n]) => `${kind} (${n})`).join(", ");
    lines.push("### Internal", "", `${rest.length} further commits: ${tally}.`, "");
  }

  if (url) {
    const span = previousTag ? `${previousTag}...v${version}` : `v${version}`;
    lines.push(`[Full changes](${url}/compare/${span}) — ${entries.length} commits.`, "");
  }
  return lines.join("\n");
}

// ---------------------------------------------------------------------------------------------------
// Archives
// ---------------------------------------------------------------------------------------------------

export interface ArchiveFile {
  /** The path inside the archive. */
  name: string;
  data: Uint8Array;
  mode: number;
  /** Seconds since the epoch. */
  mtime: number;
}

function octal(value: number, width: number): string {
  return value.toString(8).padStart(width - 1, "0") + "\0";
}

function tarHeader(name: string, mode: number, size: number, mtime: number, dir: boolean): Uint8Array {
  if (name.length > 100) die(`${name} is longer than a tar header's 100-byte name field`);
  const h = new Uint8Array(512);
  const put = (text: string, at: number) => h.set(new TextEncoder().encode(text), at);
  put(name, 0);
  put(octal(mode, 8), 100);
  put(octal(0, 8), 108);
  put(octal(0, 8), 116);
  put(octal(size, 12), 124);
  put(octal(mtime, 12), 136);
  put("        ", 148);
  put(dir ? "5" : "0", 156);
  put("ustar\0" + "00", 257);
  let sum = 0;
  for (const byte of h) sum += byte;
  put(sum.toString(8).padStart(6, "0") + "\0 ", 148);
  return h;
}

/**
 * A gzipped ustar archive of `files` under the directory `stem`. Ownership is zeroed and each mode is
 * the one given, so an archive built on Windows, which has no mode bit to copy, still carries an
 * executable binary.
 */
export function tarGz(stem: string, files: ArchiveFile[], dirMtime: number): Uint8Array {
  const parts: Uint8Array[] = [tarHeader(`${stem}/`, 0o755, 0, dirMtime, true)];
  for (const file of files) {
    parts.push(tarHeader(`${stem}/${file.name}`, file.mode, file.data.length, file.mtime, false), file.data);
    const pad = (512 - (file.data.length % 512)) % 512;
    if (pad > 0) parts.push(new Uint8Array(pad));
  }
  parts.push(new Uint8Array(1024));
  // A tar stream is a whole number of 10240-byte records.
  const length = parts.reduce((n, p) => n + p.length, 0);
  const tail = (10240 - (length % 10240)) % 10240;
  if (tail > 0) parts.push(new Uint8Array(tail));
  return gzipSync(Buffer.concat(parts), { level: 9 });
}

function dosDateTime(seconds: number): [number, number] {
  const d = new Date(seconds * 1000);
  const time = (d.getHours() << 11) | (d.getMinutes() << 5) | Math.floor(d.getSeconds() / 2);
  const date = ((Math.max(d.getFullYear(), 1980) - 1980) << 9) | ((d.getMonth() + 1) << 5) | d.getDate();
  return [time, date];
}

/**
 * A deflated zip of `files` under the directory `stem`. Each entry says it was made on Unix and carries
 * its mode, so an extractor that honours modes marks the binary executable.
 */
export function zip(stem: string, files: ArchiveFile[]): Uint8Array {
  const local: Buffer[] = [];
  const central: Buffer[] = [];
  let offset = 0;
  for (const file of files) {
    const name = Buffer.from(`${stem}/${file.name}`, "utf8");
    const packed = deflateRawSync(file.data, { level: 9 });
    const crc = Bun.hash.crc32(file.data) >>> 0;
    const [time, date] = dosDateTime(file.mtime);
    const head = Buffer.alloc(30);
    head.writeUInt32LE(0x04034b50, 0);
    head.writeUInt16LE(20, 4);
    head.writeUInt16LE(0, 6);
    head.writeUInt16LE(8, 8);
    head.writeUInt16LE(time, 10);
    head.writeUInt16LE(date, 12);
    head.writeUInt32LE(crc, 14);
    head.writeUInt32LE(packed.length, 18);
    head.writeUInt32LE(file.data.length, 22);
    head.writeUInt16LE(name.length, 26);
    head.writeUInt16LE(0, 28);
    const entry = Buffer.alloc(46);
    entry.writeUInt32LE(0x02014b50, 0);
    entry.writeUInt16LE((3 << 8) | 20, 4);
    entry.writeUInt16LE(20, 6);
    entry.writeUInt16LE(0, 8);
    entry.writeUInt16LE(8, 10);
    entry.writeUInt16LE(time, 12);
    entry.writeUInt16LE(date, 14);
    entry.writeUInt32LE(crc, 16);
    entry.writeUInt32LE(packed.length, 20);
    entry.writeUInt32LE(file.data.length, 24);
    entry.writeUInt16LE(name.length, 28);
    entry.writeUInt32LE(((0o100000 | file.mode) << 16) >>> 0, 38);
    entry.writeUInt32LE(offset, 42);
    local.push(head, name, packed);
    central.push(entry, name);
    offset += head.length + name.length + packed.length;
  }
  const directory = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(files.length, 8);
  end.writeUInt16LE(files.length, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...local, directory, end]);
}

/**
 * Archive one built binary with the notices it must ship beside, and write its `.sha256` beside it.
 * The binary is not stripped: `[profile.release]` keeps `debug = "line-tables-only"` so a production
 * backtrace names file and line. A Linux binary's debug sections are zlib-compressed instead, which
 * keeps every byte of that and shrinks the binary by more than half: inlining under `lto` and
 * `codegen-units = 1` makes the inlined-call records most of its size. The standard library's
 * symbolizer reads compressed sections, and pays for it when a backtrace is first symbolized: the
 * sections are decompressed into memory once per process, never per request. Windows keeps its
 * debug info in a PDB that is not shipped, so its binary has no sections to compress.
 */
async function packageBinary(root: string, version: string, target: string, name: string, kind: string, outDir: string): Promise<string> {
  const exe = target.includes("windows") ? "nvs.exe" : "nvs";
  const built = join(root, "target", target, "release", exe);
  if (!existsSync(built) || !statSync(built).isFile()) die(`${built} does not exist; build before packaging`);
  const read = (path: string, mode: number, as: string): ArchiveFile => ({
    name: as,
    data: readFileSync(path),
    mode,
    mtime: Math.floor(statSync(path).mtimeMs / 1000),
  });
  mkdirSync(outDir, { recursive: true });
  let binary = read(built, 0o755, exe);
  if (target.includes("linux")) {
    const compressed = join(outDir, `.${exe}-compressed`);
    const r = await runProc(["objcopy", "--compress-debug-sections=zlib", built, compressed], { timeoutMs: 600_000 });
    if (r.code !== 0) die(`objcopy --compress-debug-sections=zlib ${built} failed: ${r.stderr.trim()}`);
    binary = { ...read(compressed, 0o755, exe), mtime: binary.mtime };
    rmSync(compressed);
  }
  const files = [binary];
  for (const extra of ARCHIVE_EXTRAS) {
    const source = join(root, extra);
    if (!existsSync(source)) die(`${extra} is missing; every archive ships it (\`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand\`)`);
    files.push(read(source, 0o644, extra));
  }
  files.sort((a, b) => byCodePoint(a.name, b.name));

  const stem = `nvs-${version}-${name}`;
  const archiveName = kind === "zip" ? `${stem}.zip` : `${stem}.tar.gz`;
  const bytes = kind === "zip" ? zip(stem, files) : tarGz(stem, files, Math.floor(Date.now() / 1000));
  writeFileSync(join(outDir, archiveName), bytes);
  const digest = createHash("sha256").update(bytes).digest("hex");
  writeFileSync(join(outDir, `${archiveName}.sha256`), `${digest}  ${archiveName}\n`);
  return `${archiveName}  ${(bytes.length / 1_048_576).toFixed(1)} MiB  ${digest}`;
}

// ---------------------------------------------------------------------------------------------------
// The repository
// ---------------------------------------------------------------------------------------------------

async function git(args: string[]): Promise<string> {
  const r = await runProc(["git", ...args], { timeoutMs: 120_000 });
  if (r.code !== 0) die(`git ${args.join(" ")} failed: ${r.stderr.trim()}`);
  return r.stdout;
}

async function tags(): Promise<Tag[]> {
  return releaseTags(await git(["tag", "--list", "v*"]));
}

const cargoToml = () => join(ROOT, "Cargo.toml");
const changelogPath = () => join(ROOT, "CHANGELOG.md");
const manifest = () => readFileSync(cargoToml(), "utf8");

/**
 * Where a commit link points. In CI the runner knows, and locally `[workspace.package]`'s `repository`
 * is the committed answer. Neither is guessed from `origin`, which on a contributor's clone is a fork.
 */
function repoUrl(): string {
  const server = process.env.GITHUB_SERVER_URL;
  const slug = process.env.GITHUB_REPOSITORY;
  if (server && slug) return `${server.replace(/\/+$/, "")}/${slug}`;
  const repo = table(table(table(parseToml(manifest())).workspace).package).repository;
  return typeof repo === "string" ? repo.replace(/\/+$/, "") : "";
}

async function notesFor(version: string, previousTag: string | null, cap: number): Promise<string> {
  // Never through a shell: a subject in this history may contain backticks, `$(...)` or a quote.
  const span = previousTag ? `${previousTag}..HEAD` : "HEAD";
  const raw = await git(["log", "--reverse", "--no-merges", "--format=%H%x1f%s%x1f%b%x1e", span]);
  const today = new Date().toISOString().slice(0, 10);
  return renderNotes(version, previousTag, parseLog(raw).map(classify), repoUrl(), today, cap);
}

/** (current, next, previous tag): every guard a release clears before it writes. */
async function resolveNext(bump: string, exact: string, allowContract: boolean): Promise<[string, string, string | null]> {
  const current = workspaceVersion(manifest());
  const known = await tags();
  const previous = known[0]?.name ?? null;
  if (known.length > 0 && compareVersions(known[0]!.version, parseVersion(current)) > 0) {
    die(`tag ${previous} is ahead of Cargo.toml's ${current}. The tree is behind a release that already happened -- reconcile before releasing again.`);
  }
  const target = exact.trim() ? exact.trim() : nextVersion(current, bump);
  const want = parseVersion(target);
  if (compareVersions(want, parseVersion(current)) <= 0) die(`${target} does not advance ${current}`);
  if (known.some((t) => compareVersions(t.version, want) === 0)) die(`v${target} is already tagged`);
  if (compareVersions(want, [0, 1, 0]) >= 0 && compareVersions(parseVersion(current), [0, 1, 0]) < 0 && !allowContract) {
    die(
      `${current} -> ${target} crosses into the version contract.\n` +
        "  `rule:packaging/the-version-contract-starts-at-0-1-0`: 0.1.0 is the release that declares the language complete enough to\n" +
        "  write against, and the switch is thrown once, in the commit that tags it. That is a\n" +
        "  decision a person takes, not a dropdown.\n" +
        "  Re-run the workflow with 'I understand this declares the version contract' ticked,\n" +
        "  or pass --allow-contract locally.",
    );
  }
  return [current, target, previous];
}

async function check(): Promise<number> {
  const text = manifest();
  const version = workspaceVersion(text);
  const problems = pinProblems(text);
  const known = await tags();
  if (known.length > 0) {
    const newest = known[0]!;
    const order = compareVersions(newest.version, parseVersion(version));
    if (order > 0) problems.push(`${newest.name} is ahead of Cargo.toml's ${version}`);
    if (order === 0 && existsSync(changelogPath()) && !readFileSync(changelogPath(), "utf8").includes(`## [${version}]`)) {
      problems.push(`${newest.name} is released but CHANGELOG.md has no [${version}] section`);
    }
  }
  for (const problem of problems) console.error(`nv release: ${problem}`);
  if (problems.length > 0) return 1;
  console.log(`nv release: version ${version} is consistent across the manifest, the tags and the changelog.`);
  return 0;
}

async function apply(version: string, notesFile: string | undefined, manifestsOnly: boolean): Promise<number> {
  parseVersion(version);
  let section: string | null = null;
  if (!manifestsOnly) {
    if (!notesFile || !existsSync(notesFile)) die("--apply needs --notes-file (render it with --notes first)");
    section = readFileSync(notesFile!, "utf8").replace(/\n+$/, "") + "\n";
  }
  const before = manifest();
  const { text, changed } = bumpManifest(before, version);
  writeFileSync(cargoToml(), text);
  for (const argv of [
    ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"],
    ["cargo", "update", "--workspace", "--offline"],
  ]) {
    const r = await runProc(argv);
    if (r.code !== 0) {
      writeFileSync(cargoToml(), before);
      die(`${argv.join(" ")} failed, so Cargo.toml was put back:\n${r.stdout}\n${r.stderr}`);
    }
  }
  if (section !== null) {
    writeFileSync(changelogPath(), prependChangelog(existsSync(changelogPath()) ? readFileSync(changelogPath(), "utf8") : null, section));
  }
  console.log(`nv release: ${changed} version fields at ${version}; Cargo.lock updated.`);
  return 0;
}

// ---------------------------------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------------------------------

function emitOutput(toGithub: boolean, values: [string, string][]): void {
  for (const [key, value] of values) console.log(`${key}=${value}`);
  const path = process.env.GITHUB_OUTPUT;
  if (toGithub && path) appendFileSync(path, values.map(([k, v]) => `${k}=${v}\n`).join(""));
}

/**
 * A multi-line step output, in the heredoc form `$GITHUB_OUTPUT` needs: `key=a\nb` would be read as
 * `key=a` and a malformed line. The delimiter is fixed, because the only value written through here is
 * a tag list built from a version and a short hash, and neither can contain it.
 */
function emitLines(toGithub: boolean, key: string, lines: string[]): void {
  for (const line of lines) console.log(line);
  const path = process.env.GITHUB_OUTPUT;
  if (toGithub && path) appendFileSync(path, `${key}<<NVS_RELEASE_EOF\n${lines.map((l) => `${l}\n`).join("")}NVS_RELEASE_EOF\n`);
}

function help(): string {
  return [
    USAGE,
    "",
    "The release cycle, as one command per step. The workflow is the schedule; this is every decision it makes.",
    "",
    "modes (exactly one):",
    "  --plan {patch,minor,major}     compute the next version; write nothing",
    "  --preview {patch,minor,major}  --plan and the notes, to stdout",
    "  --notes VERSION                render that version's changelog section",
    "  --apply VERSION                rewrite the manifests and prepend the changelog",
    "  --package                      archive a built binary",
    "  --check                        the manifests, the tags and the changelog agree",
    "  --docker-tags VERSION          the registry tags one image variant claims",
    "",
    "options:",
    "  -h, --help             show this help message and exit",
    "  --version VERSION      exact version, overriding --plan/--preview's arithmetic",
    "  --allow-contract       permit crossing into 0.1.0 (rule:packaging/the-version-contract-starts-at-0-1-0)",
    "  --out PATH             output file (--notes) or directory (--package)",
    "  --notes-file PATH      the rendered section --apply prepends",
    "  --since TAG            previous tag, when it is not the newest one",
    "  --manifests-only       --apply without touching CHANGELOG.md",
    "  --github-output        also append to $GITHUB_OUTPUT",
    "  --target TRIPLE        --package: the Rust target triple that was built",
    "  --name NAME            --package: the platform name used in the archive filename",
    "  --archive {tar.gz,zip} --package: format (default tar.gz)",
    "  --image REPO           --docker-tags: the registry repository, e.g. ghcr.io/novis-lang/novis",
    "  --variant {debian,distroless}  --docker-tags: the runtime base (default distroless)",
    "  --floating             --docker-tags: also the tags that move (latest, the MAJOR.MINOR line)",
    `  --max-per-section N    entries a section lists before it counts (default ${SECTION_CAP})`,
  ].join("\n");
}

const MODES = ["--plan", "--preview", "--notes", "--apply", "--package", "--check", "--docker-tags"];

function usageError(message: string): number {
  console.error(`${USAGE}\nnv release: error: ${message}`);
  return 2;
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  let values: Map<string, string>;
  try {
    ({ flags, values } = parseArgs(args, {
      flags: ["--package", "--check", "--allow-contract", "--manifests-only", "--github-output", "--floating"],
      valued: [
        "--plan", "--preview", "--notes", "--apply", "--docker-tags", "--version", "--out", "--notes-file", "--since",
        "--target", "--name", "--archive", "--image", "--variant", "--max-per-section",
      ],
    }));
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    return usageError(e.message);
  }
  if (flags.has("--help")) {
    console.log(help());
    return 0;
  }
  const modes = MODES.filter((m) => flags.has(m) || values.has(m));
  if (modes.length !== 1) return usageError(modes.length === 0 ? `one of the arguments ${MODES.join(" ")} is required` : `${modes.join(" and ")} cannot be used together`);
  const choice = (option: string, allowed: string[], fallback?: string): string | null => {
    const value = values.get(option) ?? fallback;
    if (value === undefined || allowed.includes(value)) return value ?? null;
    usageError(`argument ${option}: invalid choice: '${value}' (choose from ${allowed.join(", ")})`);
    return null;
  };
  const archive = choice("--archive", ["tar.gz", "zip"], "tar.gz");
  const variant = choice("--variant", Object.keys(DOCKER_VARIANTS).sort(), "distroless");
  const planning = modes[0] === "--plan" || modes[0] === "--preview";
  const bump = planning ? choice(modes[0]!, BUMPS) : null;
  if (!archive || !variant || (planning && !bump)) return 2;
  const cap = pyInt(values.get("--max-per-section") ?? String(SECTION_CAP));
  if (cap === null) return usageError(`argument --max-per-section: invalid int value: '${values.get("--max-per-section")}'`);
  const toGithub = flags.has("--github-output");

  try {
    switch (modes[0]) {
      case "--check":
        return await check();

      case "--docker-tags": {
        const version = values.get("--docker-tags")!;
        parseVersion(version);
        const image = values.get("--image");
        if (!image) die("--docker-tags needs --image (e.g. ghcr.io/novis-lang/novis)");
        const short = (await git(["rev-parse", "--short=7", "HEAD"])).trim();
        // Lower-cased, because a registry reference must be and `GITHUB_REPOSITORY` keeps the case
        // it was typed in: `Novis-Lang/Novis` would be refused by the registry after the whole build.
        const out = dockerTags(version, image!.trim().toLowerCase(), variant, flags.has("--floating"), short, await tags());
        if (out.behind) console.error(`nv release: ${version} is behind ${out.behind}; not moving the floating tags.`);
        emitLines(toGithub, "tags", out.tags);
        return 0;
      }

      case "--package": {
        const [version, target, name, out] = ["--version", "--target", "--name", "--out"].map((o) => values.get(o) ?? "");
        if (!(version && target && name && out)) die("--package needs --version, --target, --name and --out");
        console.log(await packageBinary(ROOT, version!, target!, name!, archive, resolvePath(out!)));
        return 0;
      }

      case "--plan":
      case "--preview": {
        const [current, target, previous] = await resolveNext(bump!, values.get("--version") ?? "", flags.has("--allow-contract"));
        if (modes[0] === "--plan") {
          emitOutput(toGithub, [["current", current], ["version", target], ["tag", `v${target}`], ["previous_tag", previous ?? ""]]);
        } else {
          console.log(`# ${current} --${bump}--> ${target}   (previous tag: ${previous ?? "none"})\n`);
          console.log(await notesFor(target, previous, cap));
        }
        return 0;
      }

      case "--notes": {
        const version = values.get("--notes")!;
        parseVersion(version);
        const previous = values.get("--since") || ((await tags())[0]?.name ?? null);
        const rendered = await notesFor(version, previous, cap);
        const out = values.get("--out");
        if (out) {
          writeFileSync(out, rendered);
          console.log(`nv release: wrote ${out}`);
        } else console.log(rendered);
        return 0;
      }

      case "--apply":
        return await apply(values.get("--apply")!, values.get("--notes-file"), flags.has("--manifests-only"));
    }
    return 0;
  } catch (e) {
    if (!(e instanceof Fatal)) throw e;
    console.error(e.message);
    return 1;
  }
}
