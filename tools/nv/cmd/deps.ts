// `bun nv deps`: every pin in the tree beside its latest stable release, read from the registry that
// publishes it. The dependency sweep starts from this list (`docs/agent/dependency-update.md`).
//
//     bun nv deps           every pin, one line each, the ones behind first
//     bun nv deps --check   only what fails, and exit 1 when anything does
//
// The pins it reads, and where each one's latest comes from:
//
// - `crate`      a versioned row of a dependency table in every tracked `Cargo.toml`,
//                and each `cargo install <crate> --version <v>` in a workflow. The crates.io sparse index.
// - `npm`        `dependencies` and `devDependencies` of every tracked `package.json`, and `engines.bun`
//                as the `bun` package. The npm registry's `latest` tag.
// - `action`     each `uses: owner/repo@<sha>` in a workflow. The repository's latest GitHub release, or
//                the head of the branch the line's comment names (`# nightly branch`).
// - `image`      `FROM`, `ARG *_BASE=` and compose `image:` lines. The registry's tag list: the newest
//                tag of the same shape (`8.5.11-fpm` is compared with `8.5.12-fpm`, never `8.5.12-cli`).
//                A tag with no number in it, such as `trixie-slim`, is `floating` and not compared. A
//                digest pin is compared with the digest the registry gives for `latest`.
// - `toolchain`  `rust-toolchain.toml`'s channel. rust-lang/rust's latest release.
// - `wasi-sdk`   CI's `WASI_SDK_VERSION` and the libwebp build's `SOURCE.json`. WebAssembly/wasi-sdk's
//                latest release.
//
// A crate's or package's range is current when the latest release satisfies it; anything else is behind.
// A hold is the line `# HOLD (revisit <date>): <reason>` in the comment block directly above the pin, or
// in a `package.json`, which has no comments, the same text as the value of `holds.<package>`. A held pin
// that is behind is `held` until its date passes and `expired` after. The command reads, and never edits
// a file. `--check` fails on `behind`, `expired` and `unknown` (a registry that did not answer).
// GitHub's API is called with `GH_TOKEN`, `GITHUB_TOKEN` or `gh auth token`, whichever comes first.
//
// No file under `vendor/` or `tools/looper/` gives a pin. A vendored crate's manifest is upstream's,
// and a patch to it is judged as a fork. The looper kit is written by `looper sync` and never edited
// here, so neither a bump nor a hold can land in it: looper's own sweep moves its pins, and the next
// sync brings them.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { tracked } from "../lib/git.ts";
import { abs } from "../lib/paths.ts";

export const summary = "every pin beside its latest stable release; --check fails on one behind with no hold: nv deps [--check]";

export type Kind = "crate" | "npm" | "action" | "image" | "toolchain" | "wasi-sdk";

export interface Hold {
  revisit: string;
  reason: string;
}

export interface Pin {
  kind: Kind;
  /** What the registry knows it as: a crate, a package, `owner/repo`, an image reference without its tag. */
  name: string;
  /** The text the file pins: a range, a tag, a SHA. */
  pinned: string;
  /** Repo-relative, with the 1-based line of the pin. */
  file: string;
  line: number;
  hold?: Hold | undefined;
  /** An action's comment: the release or branch the SHA was taken from. */
  note?: string | undefined;
}

export type Status = "behind" | "expired" | "unknown" | "held" | "current" | "floating";

export interface Row extends Pin {
  status: Status;
  latest: string;
  /** Why the status is what it is, when the pinned text and the latest do not say. */
  why?: string | undefined;
}

const HOLD_RE = /^#?\s*HOLD \(revisit (\d{4}-\d{2}-\d{2})\):\s*(.*)$/;

/** The hold in the comment block directly above line `i` (0-based), if one is there. */
export function holdAbove(lines: string[], i: number): Hold | undefined {
  for (let j = i - 1; j >= 0; j--) {
    const t = lines[j]!.trim();
    if (!t.startsWith("#")) return undefined;
    const m = HOLD_RE.exec(t);
    if (m) return { revisit: m[1]!, reason: m[2]! };
  }
  return undefined;
}

/** A `package.json` hold's value, or undefined when it is not in the hold form. */
export function parseHold(text: string): Hold | undefined {
  const m = HOLD_RE.exec(text.trim());
  return m ? { revisit: m[1]!, reason: m[2]! } : undefined;
}

/** `line` without a trailing `#` comment, ignoring a `#` inside a double-quoted string. */
function uncomment(line: string): string {
  let quoted = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === '"' && line[i - 1] !== "\\") quoted = !quoted;
    else if (c === "#" && !quoted) return line.slice(0, i);
  }
  return line;
}

const DEP_SECTION = /(^|\.)(dev-|build-)?dependencies$/;
const DEP_TABLE = /(^|\.)(dev-|build-)?dependencies\.([A-Za-z0-9_-]+)$/;

/** The registry crate a dependency value names, or undefined when it is a path, git or workspace row. */
function crateOf(key: string, value: string): { name: string; req: string } | undefined {
  const bare = /^"([^"]+)"$/.exec(value.trim());
  if (bare) return { name: key, req: bare[1]! };
  if (/\b(path|git|workspace)\s*=/.test(value)) return undefined;
  const version = /\bversion\s*=\s*"([^"]+)"/.exec(value);
  if (!version) return undefined;
  const pkg = /\bpackage\s*=\s*"([^"]+)"/.exec(value);
  return { name: pkg ? pkg[1]! : key, req: version[1]! };
}

/** Every versioned registry dependency a `Cargo.toml` declares, with the hold above it. */
export function cargoPins(text: string, file: string): Pin[] {
  const lines = text.split(/\r?\n/);
  const pins: Pin[] = [];
  let section = "";
  for (let i = 0; i < lines.length; i++) {
    const t = uncomment(lines[i]!).trim();
    const header = /^\[\[?([^\]]+)\]\]?$/.exec(t);
    if (header) {
      section = header[1]!.trim();
      const table = DEP_TABLE.exec(section);
      if (table) {
        let body = "";
        for (let j = i + 1; j < lines.length && !/^\s*\[/.test(lines[j]!); j++) body += ` ${uncomment(lines[j]!)}`;
        const dep = crateOf(table[3]!, body);
        if (dep) pins.push({ kind: "crate", name: dep.name, pinned: dep.req, file, line: i + 1, hold: holdAbove(lines, i) });
      }
      continue;
    }
    if (!DEP_SECTION.test(section)) continue;
    const row = /^"?([A-Za-z0-9_-]+)"?\s*=\s*(.*)$/.exec(t);
    if (!row) continue;
    let value = row[2]!;
    let j = i;
    const depth = (s: string) => (s.match(/\{/g)?.length ?? 0) - (s.match(/\}/g)?.length ?? 0);
    while (depth(value) > 0 && j + 1 < lines.length) value += ` ${uncomment(lines[++j]!).trim()}`;
    const dep = crateOf(row[1]!, value);
    if (dep) pins.push({ kind: "crate", name: dep.name, pinned: dep.req, file, line: i + 1, hold: holdAbove(lines, i) });
    i = j;
  }
  return pins;
}

/** A Cargo requirement as the npm range `Bun.semver` reads: a bare version is a caret one in Cargo. */
export function cargoRange(req: string): string {
  return req
    .split(",")
    .map((p) => p.trim())
    .map((p) => (/^\d/.test(p) ? `^${p}` : p))
    .join(" ");
}

/** The 1-based line of the first `"key":` at or after `from` (0-based), or `from + 1`. */
function jsonLine(lines: string[], key: string, from = 0): number {
  const needle = `${JSON.stringify(key)}:`;
  for (let i = from; i < lines.length; i++) if (lines[i]!.includes(needle)) return i + 1;
  return from + 1;
}

/** Every registry package a `package.json` names, `engines.bun` as `bun`, with its `holds` entry. */
export function npmPins(text: string, file: string): Pin[] {
  const json = JSON.parse(text) as Record<string, unknown>;
  const lines = text.split(/\r?\n/);
  const holds = (json.holds ?? {}) as Record<string, string>;
  const pins: Pin[] = [];
  const add = (section: string, name: string, range: string) => {
    if (!/^[\^~<>=\d*xX]/.test(range)) return;
    const hold = holds[name] === undefined ? undefined : parseHold(holds[name]!);
    pins.push({ kind: "npm", name, pinned: range, file, line: jsonLine(lines, name, jsonLine(lines, section) - 1), hold });
  };
  for (const section of ["dependencies", "devDependencies"]) {
    for (const [name, range] of Object.entries((json[section] ?? {}) as Record<string, string>)) add(section, name, range);
  }
  const bun = ((json.engines ?? {}) as Record<string, string>).bun;
  if (bun) add("engines", "bun", bun);
  return pins;
}

/** Every `uses: owner/repo@ref` in a workflow, with the comment after it. */
export function actionPins(text: string, file: string): Pin[] {
  const lines = text.split(/\r?\n/);
  const pins: Pin[] = [];
  lines.forEach((l, i) => {
    const m = /^\s*(?:-\s*)?uses:\s*([^@\s]+)@(\S+)(?:\s+#\s*(.*))?$/.exec(l);
    if (!m || m[1]!.startsWith("./") || m[1]!.startsWith("docker://")) return;
    const repo = m[1]!.split("/").slice(0, 2).join("/");
    pins.push({ kind: "action", name: repo, pinned: m[2]!, file, line: i + 1, hold: holdAbove(lines, i), note: m[3]?.trim() });
  });
  return pins;
}

/** Each `cargo install <crate> --version <v>` in a workflow, as an exact crate pin. */
export function cargoInstallPins(text: string, file: string): Pin[] {
  const lines = text.split(/\r?\n/);
  const pins: Pin[] = [];
  lines.forEach((l, i) => {
    const m = /cargo install\s+([A-Za-z0-9_-]+)\s+--version\s+([0-9][^\s]*)/.exec(l);
    if (m) pins.push({ kind: "crate", name: m[1]!, pinned: `=${m[2]}`, file, line: i + 1, hold: holdAbove(lines, i) });
  });
  return pins;
}

/** Every image a Dockerfile or compose file pulls; build stages, `scratch`, `${…}` and `:local` are not pulled. */
export function imagePins(text: string, file: string): Pin[] {
  const lines = text.split(/\r?\n/);
  const stages = new Set<string>();
  const pins: Pin[] = [];
  lines.forEach((l, i) => {
    const from = /^\s*FROM\s+(?:--platform=\S+\s+)?(\S+)(?:\s+AS\s+(\S+))?/i.exec(l);
    const arg = /^\s*ARG\s+\w+_(?:BASE|IMAGE)=(\S+)/.exec(l);
    const compose = /^\s*image:\s*["']?([^"'\s]+)/.exec(l);
    const ref = from?.[1] ?? arg?.[1] ?? compose?.[1];
    if (from?.[2]) stages.add(from[2].toLowerCase());
    if (!ref || ref === "scratch" || ref.includes("$") || ref.endsWith(":local") || stages.has(ref.toLowerCase())) return;
    pins.push({ kind: "image", name: ref, pinned: ref, file, line: i + 1, hold: holdAbove(lines, i) });
  });
  return pins;
}

/** An image reference split into the registry host, the repository path, the tag and the digest. */
export function parseImage(ref: string): { registry: string; repo: string; tag: string | undefined; digest: string | undefined } {
  const [rest, digest] = ref.split("@") as [string, string | undefined];
  const slash = rest.indexOf("/");
  const first = slash < 0 ? "" : rest.slice(0, slash);
  const hosted = first.includes(".") || first.includes(":") || first === "localhost";
  const path = hosted ? rest.slice(slash + 1) : rest;
  const colon = path.lastIndexOf(":");
  const repo = colon < 0 ? path : path.slice(0, colon);
  const tag = colon < 0 ? undefined : path.slice(colon + 1);
  if (hosted) return { registry: first, repo, tag, digest };
  return { registry: "registry-1.docker.io", repo: repo.includes("/") ? repo : `library/${repo}`, tag, digest };
}

function numbers(s: string): number[] {
  return (s.match(/\d+/g) ?? []).map(Number);
}

function compareNumbers(a: number[], b: number[]): number {
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const d = (a[i] ?? -1) - (b[i] ?? -1);
    if (d !== 0) return d;
  }
  return 0;
}

/** The newest of `tags` with the same shape as `tag` (each run of digits a number, the rest equal), or
 * undefined when `tag` has no number to compare. */
export function newestTag(tag: string, tags: string[]): string | undefined {
  const shape = tag.replace(/\d+/g, "#");
  if (!shape.includes("#")) return undefined;
  let best = tag;
  for (const t of tags) if (t.replace(/\d+/g, "#") === shape && compareNumbers(numbers(t), numbers(best)) > 0) best = t;
  return best;
}

/** The highest stable version among `versions`: no pre-release part, in Cargo's or npm's sense. */
export function newestStable(versions: string[]): string | undefined {
  const stable = versions.filter((v) => /^\d+\.\d+\.\d+(\+.*)?$/.test(v));
  return stable.sort((a, b) => Bun.semver.order(a, b)).at(-1);
}

/** The status of a pin given its latest, before any hold is applied. A version's build metadata
 * (`1.1.8+spec-1.1.0`) plays no part in a range, and `Bun.semver` matches no range with it present. */
export function compare(pin: Pin, latest: string): "current" | "behind" {
  const release = latest.replace(/\+.*$/, "");
  switch (pin.kind) {
    case "crate":
      return Bun.semver.satisfies(release, cargoRange(pin.pinned)) ? "current" : "behind";
    case "npm":
      return Bun.semver.satisfies(release, pin.pinned) ? "current" : "behind";
    case "wasi-sdk":
      return numbers(pin.pinned)[0] === numbers(latest)[0] ? "current" : "behind";
    default:
      return pin.pinned === latest ? "current" : "behind";
  }
}

/** A behind pin with a hold is `held` until the hold's date passes, then `expired`. */
export function judge(pin: Pin, base: "current" | "behind", today: string): Status {
  if (base === "current") return "current";
  if (!pin.hold) return "behind";
  return pin.hold.revisit < today ? "expired" : "held";
}

// --- registries -------------------------------------------------------------------------------------

/** At most `n` requests in flight, so a sweep of a few hundred pins does not open a few hundred sockets. */
function gate(n: number): <T>(f: () => Promise<T>) => Promise<T> {
  let active = 0;
  const waiting: (() => void)[] = [];
  return async (f) => {
    if (active >= n) await new Promise<void>((r) => waiting.push(r));
    active++;
    try {
      return await f();
    } finally {
      active--;
      waiting.shift()?.();
    }
  };
}

const limit = gate(12);
const memo = new Map<string, Promise<unknown>>();

function once<T>(key: string, f: () => Promise<T>): Promise<T> {
  if (!memo.has(key)) memo.set(key, f());
  return memo.get(key) as Promise<T>;
}

async function get(url: string, headers: Record<string, string> = {}, method = "GET"): Promise<Response> {
  return limit(() => fetch(url, { method, headers: { "user-agent": "novis-nv-deps", ...headers } }));
}

function indexPath(name: string): string {
  const n = name.toLowerCase();
  if (n.length <= 2) return `${n.length}/${n}`;
  if (n.length === 3) return `3/${n[0]}/${n}`;
  return `${n.slice(0, 2)}/${n.slice(2, 4)}/${n}`;
}

function latestCrate(name: string): Promise<string> {
  return once(`crate:${name}`, async () => {
    const r = await get(`https://index.crates.io/${indexPath(name)}`);
    if (!r.ok) throw new Error(`crates.io index: ${r.status}`);
    const versions = (await r.text())
      .split("\n")
      .filter(Boolean)
      .map((l) => JSON.parse(l) as { vers: string; yanked: boolean })
      .filter((v) => !v.yanked)
      .map((v) => v.vers);
    const latest = newestStable(versions);
    if (!latest) throw new Error("no stable release on crates.io");
    return latest;
  });
}

function latestNpm(name: string): Promise<string> {
  return once(`npm:${name}`, async () => {
    const r = await get(`https://registry.npmjs.org/${name.replace("/", "%2f")}`, { accept: "application/vnd.npm.install-v1+json" });
    if (!r.ok) throw new Error(`npm registry: ${r.status}`);
    const latest = ((await r.json()) as { "dist-tags"?: { latest?: string } })["dist-tags"]?.latest;
    if (!latest) throw new Error("no latest tag on npm");
    return latest;
  });
}

function githubToken(): string | undefined {
  const env = process.env.GH_TOKEN ?? process.env.GITHUB_TOKEN;
  if (env) return env;
  const gh = spawnSync("gh", ["auth", "token"], { encoding: "utf8" });
  return gh.status === 0 ? gh.stdout.trim() || undefined : undefined;
}

let token: string | undefined | null = null;

async function github<T>(path: string): Promise<T | undefined> {
  return once(`gh:${path}`, async () => {
    if (token === null) token = githubToken();
    const headers: Record<string, string> = { accept: "application/vnd.github+json" };
    if (token) headers.authorization = `Bearer ${token}`;
    const r = await get(`https://api.github.com/${path}`, headers);
    if (r.status === 404) return undefined;
    if (!r.ok) throw new Error(`GitHub ${path}: ${r.status}`);
    return (await r.json()) as T;
  });
}

/** A repository's latest release tag; a repository with no releases gives its highest version tag. */
async function latestRelease(repo: string): Promise<string> {
  const release = await github<{ tag_name: string }>(`repos/${repo}/releases/latest`);
  if (release) return release.tag_name;
  const tags = (await github<{ name: string }[]>(`repos/${repo}/tags?per_page=100`)) ?? [];
  const versions = tags.map((t) => t.name).filter((t) => /^v?\d+(\.\d+)*$/.test(t));
  const best = versions.sort((a, b) => compareNumbers(numbers(a), numbers(b))).at(-1);
  if (!best) throw new Error(`${repo} has no release and no version tag`);
  return best;
}

async function commitOf(repo: string, ref: string): Promise<string> {
  const commit = await github<{ sha: string }>(`repos/${repo}/commits/${encodeURIComponent(ref)}`);
  if (!commit) throw new Error(`${repo} has no ref ${ref}`);
  return commit.sha;
}

/** What an action pin is compared with: the SHA of the latest release, or of the branch its comment names. */
async function latestAction(pin: Pin): Promise<{ latest: string; sha: string }> {
  const branch = /^(\S+) branch$/.exec(pin.note ?? "")?.[1];
  if (branch) return { latest: `${branch} @ ${(await commitOf(pin.name, branch)).slice(0, 8)}`, sha: await commitOf(pin.name, branch) };
  const tag = await latestRelease(pin.name);
  if (!/^[0-9a-f]{40}$/.test(pin.pinned)) return { latest: tag, sha: tag };
  const sha = await commitOf(pin.name, tag);
  return { latest: `${tag} @ ${sha.slice(0, 8)}`, sha };
}

const MANIFESTS = [
  "application/vnd.oci.image.index.v1+json",
  "application/vnd.docker.distribution.manifest.list.v2+json",
  "application/vnd.oci.image.manifest.v1+json",
  "application/vnd.docker.distribution.manifest.v2+json",
].join(", ");

const registryTokens = new Map<string, string>();

/** A registry request, answering the anonymous bearer challenge a registry sends on the first one. */
async function registry(host: string, repo: string, path: string, accept = "application/json", method = "GET"): Promise<Response> {
  const url = path.startsWith("http") ? path : `https://${host}${path}`;
  const key = `${host}/${repo}`;
  const send = () => {
    const headers: Record<string, string> = { accept };
    const t = registryTokens.get(key);
    if (t) headers.authorization = `Bearer ${t}`;
    return get(url, headers, method);
  };
  let r = await send();
  const challenge = r.headers.get("www-authenticate");
  if (r.status === 401 && challenge?.startsWith("Bearer ")) {
    const params = Object.fromEntries([...challenge.matchAll(/(\w+)="([^"]*)"/g)].map((m) => [m[1], m[2]]));
    const q = new URLSearchParams({ service: params.service ?? "", scope: params.scope ?? `repository:${repo}:pull` });
    const t = await get(`${params.realm}?${q}`);
    if (!t.ok) throw new Error(`${host} token: ${t.status}`);
    const body = (await t.json()) as { token?: string; access_token?: string };
    registryTokens.set(key, body.token ?? body.access_token ?? "");
    r = await send();
  }
  return r;
}

function imageTags(host: string, repo: string): Promise<string[]> {
  return once(`tags:${host}/${repo}`, async () => {
    const tags: string[] = [];
    let next: string | undefined = `/v2/${repo}/tags/list?n=1000`;
    for (let page = 0; next && page < 50; page++) {
      const r = await registry(host, repo, next);
      if (!r.ok) throw new Error(`${host} tags of ${repo}: ${r.status}`);
      tags.push(...(((await r.json()) as { tags?: string[] }).tags ?? []));
      next = /<([^>]+)>;\s*rel="next"/.exec(r.headers.get("link") ?? "")?.[1];
    }
    return tags;
  });
}

async function latestImage(pin: Pin): Promise<{ latest: string; status?: "floating" }> {
  const ref = parseImage(pin.pinned);
  if (ref.digest) {
    const r = await registry(ref.registry, ref.repo, `/v2/${ref.repo}/manifests/latest`, MANIFESTS, "HEAD");
    const digest = r.headers.get("docker-content-digest");
    if (!r.ok || !digest) throw new Error(`${ref.registry} has no digest for ${ref.repo}:latest (${r.status})`);
    return { latest: `${pin.pinned.split("@")[0]}@${digest}` };
  }
  const tag = ref.tag ?? "latest";
  const newest = newestTag(tag, await imageTags(ref.registry, ref.repo));
  if (!newest) return { latest: pin.pinned, status: "floating" };
  const name = pin.pinned.slice(0, pin.pinned.length - tag.length);
  return { latest: `${name}${newest}` };
}

/** One pin, looked up and judged. A lookup that fails is `unknown`, with the error as the reason. */
async function resolve(pin: Pin, today: string): Promise<Row> {
  try {
    let latest: string;
    let base: "current" | "behind";
    switch (pin.kind) {
      case "crate":
        latest = await latestCrate(pin.name);
        base = compare(pin, latest);
        break;
      case "npm":
        latest = await latestNpm(pin.name);
        base = compare(pin, latest);
        break;
      case "action": {
        const a = await latestAction(pin);
        latest = a.latest;
        base = pin.pinned === a.sha ? "current" : "behind";
        break;
      }
      case "image": {
        const i = await latestImage(pin);
        if (i.status) return { ...pin, status: i.status, latest: i.latest, why: "the tag has no version to compare" };
        latest = i.latest;
        base = compare(pin, latest);
        break;
      }
      case "toolchain":
        latest = await latestRelease("rust-lang/rust");
        base = compare(pin, latest);
        break;
      case "wasi-sdk":
        latest = (await latestRelease("WebAssembly/wasi-sdk")).replace(/^wasi-sdk-/, "");
        base = compare(pin, latest);
        break;
    }
    const status = judge(pin, base, today);
    const why = base === "current" && pin.hold ? "the hold above it is no longer needed" : undefined;
    return { ...pin, status, latest, why };
  } catch (e) {
    return { ...pin, status: "unknown", latest: "?", why: e instanceof Error ? e.message : String(e) };
  }
}

// --- the tree ---------------------------------------------------------------------------------------

/** The trees whose files give no pin, as the head comment says. */
export const SKIPPED = ["vendor/", "tools/looper/"];

/** Every pin in the tracked files, in file order. */
export async function pins(): Promise<Pin[]> {
  const read = (f: string) => readFileSync(abs(f), "utf8");
  return (await tracked()).flatMap((f) => filePins(f, read));
}

/** The pins one tracked file gives. `read` is called only for a file that can give one. */
export function filePins(f: string, read: (f: string) => string): Pin[] {
  const out: Pin[] = [];
  if (SKIPPED.some((p) => f.startsWith(p))) return out;
  const base = f.split("/").at(-1)!;
  if (base === "Cargo.toml") out.push(...cargoPins(read(f), f));
  else if (base === "package.json" && !f.startsWith("data/")) out.push(...npmPins(read(f), f));
  else if (f.startsWith(".github/workflows/") && /\.ya?ml$/.test(f)) {
    const text = read(f);
    out.push(...actionPins(text, f), ...cargoInstallPins(text, f));
    const lines = text.split(/\r?\n/);
    lines.forEach((l, i) => {
      const m = /^\s*WASI_SDK_VERSION:\s*['"]?(\d+)/.exec(l);
      if (m) out.push({ kind: "wasi-sdk", name: "WebAssembly/wasi-sdk", pinned: m[1]!, file: f, line: i + 1, hold: holdAbove(lines, i) });
    });
  } else if (/^Dockerfile(\.|$)/.test(base) && !base.endsWith(".dockerignore")) out.push(...imagePins(read(f), f));
  else if (/^compose.*\.ya?ml$/.test(base)) out.push(...imagePins(read(f), f));
  else if (f === "rust-toolchain.toml") {
    const lines = read(f).split(/\r?\n/);
    const i = lines.findIndex((l) => /^\s*channel\s*=/.test(l));
    const m = /"([^"]+)"/.exec(lines[i] ?? "");
    if (m) out.push({ kind: "toolchain", name: "rust", pinned: m[1]!, file: f, line: i + 1, hold: holdAbove(lines, i) });
  } else if (base === "SOURCE.json") {
    const text = read(f);
    const version = (JSON.parse(text) as { toolchain?: { version?: string } }).toolchain?.version;
    const m = /^wasi-sdk (\d+(?:\.\d+)*)$/.exec(version ?? "");
    if (m) out.push({ kind: "wasi-sdk", name: "WebAssembly/wasi-sdk", pinned: m[1]!, file: f, line: jsonLine(text.split(/\r?\n/), "toolchain") });
  }
  return out;
}

const ORDER: Status[] = ["behind", "expired", "unknown", "held", "floating", "current"];
const FAILS = new Set<Status>(["behind", "expired", "unknown"]);

function table(rows: Row[]): string[] {
  const cells = rows.map((r) => [
    r.status,
    r.kind,
    r.name === r.pinned ? "" : r.name,
    r.kind === "action" && r.note ? `${r.note} @ ${r.pinned.slice(0, 8)}` : r.pinned,
    r.latest === r.pinned ? "" : `-> ${r.latest}`,
    `${r.file}:${r.line}`,
    r.why ?? (r.hold ? `hold to ${r.hold.revisit}: ${r.hold.reason}` : ""),
  ]);
  const widths = cells[0]?.map((_, c) => (c === 6 ? 0 : Math.max(...cells.map((row) => row[c]!.length)))) ?? [];
  return cells.map((row) => row.map((cell, c) => cell.padEnd(widths[c]!)).join("  ").trimEnd());
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const unknown = args.filter((a) => a !== "--check");
  if (unknown.length) {
    console.error(`nv deps: unknown argument ${unknown[0]}\n${summary}`);
    return 2;
  }
  const check = args.includes("--check");
  const today = new Date().toISOString().slice(0, 10);
  const rows = await Promise.all((await pins()).map((p) => resolve(p, today)));
  rows.sort((a, b) => ORDER.indexOf(a.status) - ORDER.indexOf(b.status) || a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name));
  const shown = check ? rows.filter((r) => FAILS.has(r.status)) : rows;
  for (const line of table(shown)) console.log(line);
  const counts = ORDER.map((s) => [s, rows.filter((r) => r.status === s).length] as const).filter(([, n]) => n > 0);
  console.log(`\n${rows.length} pins: ${counts.map(([s, n]) => `${n} ${s}`).join(", ")}`);
  return rows.some((r) => FAILS.has(r.status)) ? 1 : 0;
}
