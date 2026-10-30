// `bun nv discord`: the project's Discord server, administered from a session when the user asks for a
// change, and the release and dev build messages CI posts to it.
//
//     bun nv discord check    --token-file <path> (--guild <id> | --channel <id>)
//     bun nv discord api      --token-file <path> <METHOD> <api-path> [<json> | --body-file <path>] [--reason <text>]
//                             [--guild <id> | --channel <id>] [--all] [--max <n>] [--out <path>] [--dry-run]
//     bun nv discord snapshot --token-file <path> (--guild <id> | --channel <id>) --out <path>
//     bun nv discord diff     <before.json> <after.json>
//     bun nv discord bits     <NAME>... | <bitfield>
//     bun nv discord announce dev --sha <commit> --run-url <url> [--since <commit>] [--dry-run]
//     bun nv discord announce release --tag <vX.Y.Z> --url <url> --notes-file <path> [--dry-run]
//
// Every subcommand but `announce` runs only in a session, on the user's request. The bot's token is read
// from `--token-file` alone, the path a session's secret prompt gives, and only ever goes into the
// `Authorization` header: no argument takes it, and nothing prints or stores it. A token in an
// environment variable or a file the tree keeps would hand the unattended loop sessions an administrator
// of the server.
//
// `api` is one REST call against Discord's API v10, so every endpoint the docs list is reachable without a
// command of its own; a task that comes back gets its own subcommand. The API path's leading `/` is
// optional, because Git Bash rewrites an argument that starts with one into a Windows path. It waits out a
// bucket the last response emptied and retries a 429 after the `retry_after` Discord sends (a GET is also retried on a
// 5xx, a write never is, so a write is never sent twice). `--reason` becomes the `X-Audit-Log-Reason` the
// server's audit log shows. `--all` follows a list endpoint page by page: `before` for a channel's messages,
// which come newest first, `after` for everything else, up to `--max` items. In the path and the body,
// `{guild}`, `{me}`, `{channel:<name>}` and `{role:<name>}` become ids, looked up in the server that
// `--guild` names or that `--channel` is in; a name two channels share is an error naming both ids.
// `--dry-run` resolves the call and prints it without sending it.
//
// `snapshot` writes the server's settings, channels, roles, AutoMod rules, onboarding, webhooks, invites,
// emojis, stickers, events and active threads to one JSON file, with permission bitfields as flag names,
// each channel's kind and category, and each overwrite's role named. Values that change with traffic
// (`last_message_id`, counts) and webhook tokens are left out, so `diff` of a snapshot before a task and
// one after it shows what the task changed and nothing else. A section the bot cannot read is recorded as
// `{ "unavailable": <why> }`. `--out` of `api` and `snapshot` must be under `.agent-tmp/`: server data
// never goes into the tree, and whoever writes it deletes it.
//
// `check` verifies the setup a session needs and names each missing step: the token, the bot's privileged
// intents, that it is not public, that it is in the server and sees the channel, that it holds
// Administrator, and that no role sits above its own. A server's id given as `--channel` is named as one.
// `bits` turns flag names into a bitfield for a write, or a bitfield into names.
//
// `announce` is the one subcommand CI runs (`.github/workflows/discord.yml`). It posts one embed to the
// webhook URL in `DISCORD_WEBHOOK_URL`, which can post to one channel and do nothing else, and never
// prints that URL. `dev` lists the commits from `--since` (the previous dev build) to `--sha`, grouped
// the way `nv release` groups a changelog: features, fixes and performance by name, every other commit
// counted. With no `--since`, or one git does not know, it names the one commit. `release` shows the
// release's notes, cut at a line to fit an embed, with the install page and the image to pull. Neither
// mentions anyone, whatever a commit subject or the notes contain. `--dry-run` prints the message.
//
// Exits 0 on success, 1 when Discord, git or a check fails, and 2 on a usage error.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { classify, HEADLINE_SECTIONS, parseLog } from "./release.ts";

export const summary = "administer the project's Discord server on request, and post CI's messages: nv discord check|api|snapshot|diff|bits|announce";

const USAGE = `usage:
  bun nv discord check    --token-file <path> (--guild <id> | --channel <id>)
  bun nv discord api      --token-file <path> <METHOD> <api-path> [<json> | --body-file <path>] [--reason <text>]
                          [--guild <id> | --channel <id>] [--all] [--max <n>] [--out <path>] [--dry-run]
  bun nv discord snapshot --token-file <path> (--guild <id> | --channel <id>) --out <path>
  bun nv discord diff     <before.json> <after.json>
  bun nv discord bits     <NAME>... | <bitfield>
  bun nv discord announce dev --sha <commit> --run-url <url> [--since <commit>] [--dry-run]
  bun nv discord announce release --tag <vX.Y.Z> --url <url> --notes-file <path> [--dry-run]`;

export const API = "https://discord.com/api/v10";
const USER_AGENT = "DiscordBot (https://github.com/novis-lang/novis, 1)";
/** How many times one call is retried after a 429, or a GET after a 5xx. */
const RETRIES = 5;
const REASON_MAX = 512;
const DEFAULT_MAX = 10_000;
const METHODS = new Set(["GET", "POST", "PUT", "PATCH", "DELETE"]);
const SNOWFLAKE = /^\d{17,20}$/;

/** Discord's permission flags by bit, named as its docs name them. A bit not here prints as `BIT_<n>`. */
export const PERMISSIONS: readonly (readonly [number, string])[] = [
  [0, "CREATE_INSTANT_INVITE"], [1, "KICK_MEMBERS"], [2, "BAN_MEMBERS"], [3, "ADMINISTRATOR"],
  [4, "MANAGE_CHANNELS"], [5, "MANAGE_GUILD"], [6, "ADD_REACTIONS"], [7, "VIEW_AUDIT_LOG"],
  [8, "PRIORITY_SPEAKER"], [9, "STREAM"], [10, "VIEW_CHANNEL"], [11, "SEND_MESSAGES"],
  [12, "SEND_TTS_MESSAGES"], [13, "MANAGE_MESSAGES"], [14, "EMBED_LINKS"], [15, "ATTACH_FILES"],
  [16, "READ_MESSAGE_HISTORY"], [17, "MENTION_EVERYONE"], [18, "USE_EXTERNAL_EMOJIS"], [19, "VIEW_GUILD_INSIGHTS"],
  [20, "CONNECT"], [21, "SPEAK"], [22, "MUTE_MEMBERS"], [23, "DEAFEN_MEMBERS"],
  [24, "MOVE_MEMBERS"], [25, "USE_VAD"], [26, "CHANGE_NICKNAME"], [27, "MANAGE_NICKNAMES"],
  [28, "MANAGE_ROLES"], [29, "MANAGE_WEBHOOKS"], [30, "MANAGE_GUILD_EXPRESSIONS"], [31, "USE_APPLICATION_COMMANDS"],
  [32, "REQUEST_TO_SPEAK"], [33, "MANAGE_EVENTS"], [34, "MANAGE_THREADS"], [35, "CREATE_PUBLIC_THREADS"],
  [36, "CREATE_PRIVATE_THREADS"], [37, "USE_EXTERNAL_STICKERS"], [38, "SEND_MESSAGES_IN_THREADS"], [39, "USE_EMBEDDED_ACTIVITIES"],
  [40, "MODERATE_MEMBERS"], [41, "VIEW_CREATOR_MONETIZATION_ANALYTICS"], [42, "USE_SOUNDBOARD"], [43, "CREATE_GUILD_EXPRESSIONS"],
  [44, "CREATE_EVENTS"], [45, "USE_EXTERNAL_SOUNDS"], [46, "SEND_VOICE_MESSAGES"], [48, "SET_VOICE_CHANNEL_STATUS"],
  [49, "SEND_POLLS"], [50, "USE_EXTERNAL_APPS"], [51, "PIN_MESSAGES"], [52, "BYPASS_SLOWMODE"],
];
const BY_BIT = new Map(PERMISSIONS);
const BY_NAME = new Map(PERMISSIONS.map(([bit, name]) => [name, bit]));

/** Application flags: each privileged intent, for a bot in 100 servers or more and one in fewer. */
const MEMBERS_INTENT = (1 << 14) | (1 << 15);
const CONTENT_INTENT = (1 << 18) | (1 << 19);

const CHANNEL_KINDS: Record<number, string> = {
  0: "text", 2: "voice", 4: "category", 5: "announcement", 10: "announcement thread", 11: "public thread",
  12: "private thread", 13: "stage", 14: "directory", 15: "forum", 16: "media",
};

/** Keys whose values move with traffic, not with a setting, so a snapshot leaves them out. */
const VOLATILE = new Set(["last_message_id", "message_count", "total_message_sent", "member_count", "approximate_member_count", "approximate_presence_count"]);

/** Endpoints whose `limit` defaults below their maximum, so `--all` asks for full pages. */
const PAGE_LIMITS: readonly (readonly [RegExp, number])[] = [
  [/\/channels\/\d+\/messages$/, 100],
  [/\/guilds\/\d+\/members$/, 1000],
  [/\/guilds\/\d+\/bans$/, 1000],
];

/** A wrong invocation: exit 2 with the usage. */
class Usage extends Error {}
/** A failure that is not Discord's answer: exit 1. */
class Fail extends Error {}

/** A non-2xx answer from Discord, with its error body. */
export class DiscordError extends Error {
  constructor(
    readonly status: number,
    readonly method: string,
    readonly path: string,
    readonly body: unknown,
  ) {
    super(`${method} ${path} failed: ${describe(status, body)}`);
  }
}

function describe(status: number, body: unknown): string {
  if (body !== null && typeof body === "object") {
    const b = body as { message?: string; code?: number; errors?: unknown };
    let s = `${status} ${b.message ?? ""}`.trim();
    if (b.code !== undefined) s += ` (code ${b.code})`;
    if (b.errors !== undefined) s += ` ${JSON.stringify(b.errors)}`;
    return s;
  }
  return `${status} ${String(body ?? "").slice(0, 300)}`.trim();
}

/** Everything the command reaches outside itself, replaced under test. Each one left out is the real one. */
export interface Seams {
  fetch(url: string, init: RequestInit): Promise<Response>;
  sleep(ms: number): Promise<void>;
  say(line: string): void;
  warn(line: string): void;
  read(path: string): string;
  write(path: string, text: string): void;
  /** `git <args>` in the root; its standard output, or a throw when it fails. */
  git(args: string[]): string;
  env: Record<string, string | undefined>;
  now(): number;
  root: string;
}

const REAL: Seams = {
  fetch: (url, init) => fetch(url, init),
  sleep: (ms) => Bun.sleep(ms),
  say: (l) => console.log(l),
  warn: (l) => console.error(l),
  read: (p) => readFileSync(p, "utf8"),
  write: (p, t) => {
    mkdirSync(dirname(p), { recursive: true });
    writeFileSync(p, t);
  },
  git: (args) => {
    const r = Bun.spawnSync(["git", ...args], { cwd: ROOT });
    if (r.exitCode !== 0) throw new Fail(`git ${args[0]} failed: ${r.stderr.toString().trim()}`);
    return r.stdout.toString();
  },
  env: process.env,
  now: () => Date.now(),
  root: ROOT,
};

/** One bot's REST calls, sequential, waiting out the rate limits Discord reports. */
export class Client {
  /** How long to wait before the next request: the bucket the last answer emptied, or a 429's `retry_after`. */
  private waitMs = 0;

  constructor(
    private readonly token: string,
    private readonly s: Seams,
  ) {}

  async call(method: string, path: string, c: { body?: unknown; reason?: string | undefined } = {}): Promise<unknown> {
    const headers: Record<string, string> = { Authorization: `Bot ${this.token}`, "User-Agent": USER_AGENT };
    if (c.body !== undefined) headers["Content-Type"] = "application/json";
    if (c.reason !== undefined) headers["X-Audit-Log-Reason"] = encodeURIComponent(c.reason);
    const init: RequestInit = { method, headers, body: c.body === undefined ? undefined : JSON.stringify(c.body) };
    for (let attempt = 0; ; attempt++) {
      if (this.waitMs > 0) {
        await this.s.sleep(this.waitMs);
        this.waitMs = 0;
      }
      let res: Response;
      try {
        res = await this.s.fetch(`${API}${path}`, init);
      } catch (e) {
        throw new Fail(`${method} ${path}: ${(e as Error).message}`);
      }
      const body = await parse(res);
      if (res.headers.get("X-RateLimit-Remaining") === "0") this.waitMs = seconds(res.headers.get("X-RateLimit-Reset-After"), 0);
      if (res.status === 429 && attempt < RETRIES) {
        const after = (body as { retry_after?: number } | null)?.retry_after;
        this.waitMs = typeof after === "number" ? Math.ceil(after * 1000) : seconds(res.headers.get("Retry-After"), 1000);
        continue;
      }
      if (res.status >= 500 && method === "GET" && attempt < RETRIES) {
        this.waitMs = 1000 * 2 ** attempt;
        continue;
      }
      if (!res.ok) throw new DiscordError(res.status, method, path, body);
      return body;
    }
  }
}

async function parse(res: Response): Promise<unknown> {
  const text = await res.text();
  if (text === "") return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function seconds(header: string | null, fallbackMs: number): number {
  const n = header === null ? Number.NaN : Number(header);
  return Number.isFinite(n) && n >= 0 ? Math.ceil(n * 1000) : fallbackMs;
}

/** The names of the flags a bitfield sets, lowest bit first. */
export function permissionNames(bits: string): string[] {
  const v = BigInt(bits);
  const out: string[] = [];
  for (let n = 0; v >> BigInt(n) !== 0n; n++) if ((v >> BigInt(n)) & 1n) out.push(BY_BIT.get(n) ?? `BIT_${n}`);
  return out;
}

/** The bitfield of flag names, written `SEND_MESSAGES`, `SendMessages` or `BIT_<n>`. */
export function permissionBits(names: string[]): string {
  let v = 0n;
  for (const raw of names) {
    const name = raw.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toUpperCase();
    const bit = BY_NAME.get(name) ?? (/^BIT_(\d+)$/.test(name) ? Number(name.slice(4)) : undefined);
    if (bit === undefined) throw new Usage(`no permission named ${raw}`);
    v |= 1n << BigInt(bit);
  }
  return String(v);
}

interface Named {
  id: string;
  name: string;
}

/** The server a call is about, and its channels and roles, each read once and only when a call needs it. */
class Scope {
  private guildId?: Promise<string>;
  private meId?: Promise<string>;
  private lists = new Map<string, Promise<Named[]>>();

  constructor(
    private readonly client: Client,
    private readonly guildArg?: string,
    private readonly channelArg?: string,
  ) {}

  guild(): Promise<string> {
    this.guildId ??= (async () => {
      if (this.guildArg) return this.guildArg;
      if (!this.channelArg) throw new Usage("name the server with --guild <id> or --channel <id>");
      const ch = (await this.client.call("GET", `/channels/${this.channelArg}`)) as { guild_id?: string };
      if (!ch.guild_id) throw new Fail(`channel ${this.channelArg} is not in a server`);
      return ch.guild_id;
    })();
    return this.guildId;
  }

  me(): Promise<string> {
    this.meId ??= this.client.call("GET", "/users/@me").then((u) => (u as { id: string }).id);
    return this.meId;
  }

  list(kind: "channels" | "roles"): Promise<Named[]> {
    let l = this.lists.get(kind);
    if (!l) {
      l = this.guild().then((g) => this.client.call("GET", `/guilds/${g}/${kind}`) as Promise<Named[]>);
      this.lists.set(kind, l);
    }
    return l;
  }
}

const HOLE = /\{(guild|me|channel:[^{}]+|role:[^{}]+)\}/g;

/** `text` with each `{guild}`, `{me}`, `{channel:<name>}` and `{role:<name>}` replaced by its id. */
async function fill(text: string, scope: Scope): Promise<string> {
  for (const hole of new Set(text.match(HOLE) ?? [])) text = text.split(hole).join(await resolveHole(hole.slice(1, -1), scope));
  return text;
}

async function resolveHole(hole: string, scope: Scope): Promise<string> {
  if (hole === "guild") return scope.guild();
  if (hole === "me") return scope.me();
  const colon = hole.indexOf(":");
  const kind = hole.slice(0, colon) as "channel" | "role";
  const name = hole.slice(colon + 1).trim();
  const bare = name.replace(/^[#@]/, "");
  const hits = (await scope.list(kind === "channel" ? "channels" : "roles")).filter((x) => x.name === name || x.name === bare);
  if (hits.length === 1) return hits[0]!.id;
  if (hits.length === 0) throw new Fail(`no ${kind} named ${name} in the server`);
  throw new Fail(`${hits.length} ${kind}s are named ${name} (${hits.map((h) => h.id).join(", ")}): write the id instead`);
}

function idOf(x: unknown): string | undefined {
  const o = x as { id?: string; user?: { id?: string } } | null;
  return o?.id ?? o?.user?.id;
}

/** Every item of a list endpoint, page by page, up to `max`. */
export async function paginate(client: Client, path: string, max: number, s: Seams): Promise<unknown[]> {
  const q = path.indexOf("?");
  const base = q < 0 ? path : path.slice(0, q);
  const params = new URLSearchParams(q < 0 ? "" : path.slice(q + 1));
  const backward = /\/messages$/.test(base);
  const preset = PAGE_LIMITS.find(([re]) => re.test(base));
  if (preset && !params.has("limit")) params.set("limit", String(preset[1]));
  const limit = Number(params.get("limit") ?? 0);
  const items: unknown[] = [];
  for (;;) {
    const page = await client.call("GET", params.size > 0 ? `${base}?${params}` : base);
    if (!Array.isArray(page)) throw new Fail(`--all needs an endpoint that returns a list, and GET ${base} returned an object`);
    for (const x of page) {
      items.push(x);
      if (items.length >= max) {
        s.warn(`nv discord: stopped at --max ${max} items; the list may go on`);
        return items;
      }
    }
    if (page.length === 0 || (limit > 0 && page.length < limit)) return items;
    const ids = page.map(idOf).filter((id): id is string => id !== undefined).map(BigInt);
    if (ids.length === 0) throw new Fail(`--all needs items with an id, and GET ${base} returned none`);
    params.set(backward ? "before" : "after", String(ids.reduce((a, b) => ((backward ? b < a : b > a) ? b : a))));
  }
}

/** `abs` relative to the root, with `/` on every platform. */
function shown(abs: string, root: string): string {
  return relative(root, abs).replaceAll("\\", "/");
}

/** The absolute path of `path`, which must be a file under `.agent-tmp/`. */
function scratch(path: string, root: string): string {
  const abs = resolve(root, path);
  const rel = relative(join(root, ".agent-tmp"), abs);
  if (rel === "" || rel.startsWith("..") || isAbsolute(rel)) throw new Usage(`--out ${path} is not under .agent-tmp/: server data never goes into the tree`);
  return abs;
}

const SECTIONS: readonly (readonly [string, string])[] = [
  ["guild", "/guilds/{g}"],
  ["channels", "/guilds/{g}/channels"],
  ["roles", "/guilds/{g}/roles"],
  ["automod", "/guilds/{g}/auto-moderation/rules"],
  ["onboarding", "/guilds/{g}/onboarding"],
  ["welcomeScreen", "/guilds/{g}/welcome-screen"],
  ["webhooks", "/guilds/{g}/webhooks"],
  ["invites", "/guilds/{g}/invites"],
  ["emojis", "/guilds/{g}/emojis"],
  ["stickers", "/guilds/{g}/stickers"],
  ["events", "/guilds/{g}/scheduled-events"],
  ["threads", "/guilds/{g}/threads/active"],
];

type Json = Record<string, unknown>;

/** The server as `snapshot` writes it. */
export async function snapshot(client: Client, guild: string): Promise<Json> {
  const raw: Json = {};
  for (const [key, path] of SECTIONS) {
    try {
      raw[key] = await client.call("GET", path.replace("{g}", guild));
    } catch (e) {
      if (!(e instanceof DiscordError)) throw e;
      raw[key] = { unavailable: describe(e.status, e.body) };
    }
  }
  return shape(raw);
}

function shape(raw: Json): Json {
  const g = raw.guild as Json | undefined;
  if (g && !("unavailable" in g)) for (const k of ["roles", "emojis", "stickers"]) delete g[k];
  const roles = Array.isArray(raw.roles) ? (raw.roles as (Json & Named & { position: number })[]) : [];
  const roleName = new Map(roles.map((r) => [r.id, r.name]));
  for (const r of roles) r.permissions = permissionNames(String(r.permissions));
  roles.sort((a, b) => b.position - a.position);
  if (Array.isArray(raw.channels)) raw.channels = channels(raw.channels as Json[], roleName);
  // An incoming webhook's `token`, and the `url` that embeds it, post as that webhook.
  if (Array.isArray(raw.webhooks)) {
    for (const w of raw.webhooks as Json[]) {
      delete w.token;
      delete w.url;
    }
  }
  const threads = raw.threads as Json | undefined;
  if (threads && Array.isArray(threads.threads)) raw.threads = threads.threads;
  return strip(raw) as Json;
}

function channels(list: Json[], roleName: Map<string, string>): Json[] {
  const byId = new Map(list.map((c) => [c.id as string, c]));
  for (const c of list) {
    c.kind = CHANNEL_KINDS[c.type as number] ?? `type ${c.type}`;
    const parent = c.parent_id ? byId.get(c.parent_id as string) : undefined;
    if (parent) c.category = parent.name;
    for (const o of (c.permission_overwrites as Json[] | undefined) ?? []) {
      o.target = o.type === 0 ? (roleName.get(o.id as string) ?? `role ${o.id}`) : `member ${o.id}`;
      o.allow = permissionNames(String(o.allow));
      o.deny = permissionNames(String(o.deny));
    }
  }
  // Uncategorised channels first, then each category followed by its channels, as the client lists them.
  const rank = (c: Json): [number, number, number] => {
    const parent = c.parent_id ? byId.get(c.parent_id as string) : undefined;
    if (c.type === 4) return [c.position as number, 0, 0];
    return parent ? [parent.position as number, 1, c.position as number] : [-1, 1, c.position as number];
  };
  return [...list].sort((a, b) => {
    const [x, y] = [rank(a), rank(b)];
    return x[0] - y[0] || x[1] - y[1] || x[2] - y[2];
  });
}

function strip(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(strip);
  if (v === null || typeof v !== "object") return v;
  const out: Json = {};
  for (const [k, x] of Object.entries(v)) if (!VOLATILE.has(k) && !(k === "members" && Array.isArray(x))) out[k] = strip(x);
  return out;
}

/** One line per difference between two snapshots: `+` added, `-` removed, `~` changed. */
export function diff(a: unknown, b: unknown, at = ""): string[] {
  if (JSON.stringify(a) === JSON.stringify(b)) return [];
  if (Array.isArray(a) && Array.isArray(b) && [...a, ...b].every((x) => idOf(x) !== undefined)) {
    const label = (x: unknown) => `${at}[${(x as { name?: string }).name ?? idOf(x)}]`;
    const before = new Map(a.map((x) => [idOf(x)!, x]));
    const after = new Map(b.map((x) => [idOf(x)!, x]));
    const out: string[] = [];
    for (const [id, x] of before) if (!after.has(id)) out.push(`- ${label(x)}`);
    for (const [id, y] of after) out.push(...(before.has(id) ? diff(before.get(id), y, label(y)) : [`+ ${label(y)} ${short(y)}`]));
    return out;
  }
  if (isObject(a) && isObject(b)) {
    const out: string[] = [];
    for (const k of [...new Set([...Object.keys(a), ...Object.keys(b)])].sort()) {
      const path = at === "" ? k : `${at}.${k}`;
      if (!(k in b)) out.push(`- ${path}: ${short(a[k])}`);
      else if (!(k in a)) out.push(`+ ${path}: ${short(b[k])}`);
      else out.push(...diff(a[k], b[k], path));
    }
    return out;
  }
  return [`~ ${at}: ${short(a)} -> ${short(b)}`];
}

function isObject(v: unknown): v is Json {
  return v !== null && typeof v === "object" && !Array.isArray(v);
}

function short(v: unknown): string {
  const s = JSON.stringify(v) ?? "undefined";
  return s.length > 200 ? `${s.slice(0, 197)}...` : s;
}

/** The setup a session needs, one GREEN or RED line per step; returns the exit code. */
async function check(client: Client, where: { guild?: string | undefined; channel?: string | undefined }, s: Seams): Promise<number> {
  let red = 0;
  const say = (ok: boolean, line: string) => {
    if (!ok) red++;
    s.say(`${ok ? "GREEN" : "RED"} ${line}`);
  };
  let me: { id: string; username: string };
  try {
    me = (await client.call("GET", "/users/@me")) as typeof me;
  } catch (e) {
    if (e instanceof DiscordError && e.status === 401) {
      say(false, "the token is not accepted: reset it under the application's Bot tab and use the new one");
      return 1;
    }
    throw e;
  }
  say(true, `the token works: bot ${me.username} (${me.id})`);
  const app = (await client.call("GET", "/applications/@me")) as { flags?: number; bot_public?: boolean };
  const flags = app.flags ?? 0;
  say((flags & MEMBERS_INTENT) !== 0, "Server Members Intent is on (Bot tab, Privileged Gateway Intents)");
  say((flags & CONTENT_INTENT) !== 0, "Message Content Intent is on (Bot tab, Privileged Gateway Intents)");
  say(app.bot_public === false, "Public Bot is off (Bot tab), so only you can add the bot to a server");
  // The servers the bot is in: an id given as a channel may be one of them, and is then named as a server.
  const servers = (await client.call("GET", "/users/@me/guilds")) as Named[];
  const notIn = (id: string) => `the bot is not in server ${id}: add it with the OAuth2 URL Generator (scope bot, permission Administrator)`;
  let guildId = where.guild;
  let seen = "";
  if (where.channel !== undefined) {
    const server = servers.find((g) => g.id === where.channel);
    if (server) {
      say(false, `${where.channel} is the id of the server ${server.name}, not of a channel: run check with --guild ${where.channel}`);
      return 1;
    }
    let ch: { name?: string; guild_id?: string };
    try {
      ch = (await client.call("GET", `/channels/${where.channel}`)) as typeof ch;
    } catch (e) {
      if (!(e instanceof DiscordError) || (e.status !== 403 && e.status !== 404)) throw e;
      say(false, `the bot cannot see channel ${where.channel}: ${servers.length === 0 ? notIn("of that channel") : "it is in no server the bot is in, or Administrator is missing"}`);
      return 1;
    }
    if (!ch.guild_id) {
      say(false, `channel ${where.channel} is not in a server`);
      return 1;
    }
    guildId = ch.guild_id;
    seen = `#${ch.name} in `;
  }
  if (!servers.some((g) => g.id === guildId)) {
    say(false, notIn(guildId!));
    return 1;
  }
  const guild = (await client.call("GET", `/guilds/${guildId}`)) as { name: string; mfa_level: number; roles: (Named & { position: number; permissions: string; managed?: boolean })[] };
  say(true, `the bot sees ${seen}${guild.name} (server ${guildId})`);
  const member = (await client.call("GET", `/guilds/${guildId}/members/${me.id}`)) as { roles: string[] };
  const mine = guild.roles.filter((r) => member.roles.includes(r.id) || r.id === guildId);
  say(mine.some((r) => permissionNames(r.permissions).includes("ADMINISTRATOR")), "the bot has Administrator");
  const top = Math.max(...mine.map((r) => r.position));
  const above = guild.roles.filter((r) => r.position > top);
  say(above.length === 0, above.length === 0 ? "no role is above the bot's own" : `roles above the bot's own, which it cannot change: ${above.map((r) => r.name).join(", ")}. Drag its role to the top under Server Settings, Roles`);
  if (guild.mfa_level === 1) s.say("NOTE the server requires 2FA for moderation, so the bot's owner needs 2FA on their Discord account");
  return red === 0 ? 0 : 1;
}

const REPO_URL = "https://github.com/novis-lang/novis";
const IMAGE = "ghcr.io/novis-lang/novis";
const INSTALL_URL = "https://novis-lang.org/install/";
/** The website's blue. */
const BRAND = 0x11a7d7;
/** How many commits a dev build lists by name before it counts the rest. */
const DEV_LINES = 15;
const SUBJECT_MAX = 160;
/** An embed's description holds 4096 characters, and the notes leave room for the link after them. */
const NOTES_MAX = 3800;
const WEBHOOK = /^https:\/\/(?:canary\.|ptb\.)?discord(?:app)?\.com\/api\/webhooks\/\d+\/[\w-]+$/;
const COMMIT = /^[0-9a-f]{40}$/;

/** One Discord embed, as `announce` posts it. */
export interface Embed {
  title: string;
  url: string;
  description: string;
  color: number;
  fields: { name: string; value: string }[];
  footer?: { text: string };
  timestamp: string;
}

function clip(text: string, max: number): string {
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
}

function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

/** `a`, `a and b`, `a, b and c`. */
function listed(words: string[]): string {
  return words.length < 2 ? (words[0] ?? "") : `${words.slice(0, -1).join(", ")} and ${words.at(-1)}`;
}

/** The message of a dev build: the commits since the dev build before it, from `git log`'s records. */
export function devEmbed(o: { sha: string; since: string | null; version: string; runUrl: string; log: string; now: number }): Embed {
  const entries = parseLog(o.log).map(classify);
  const lines: string[] = [];
  let named = 0;
  let unnamed = 0;
  for (const [type, title] of HEADLINE_SECTIONS) {
    const rows = entries.filter((e) => e.type === type);
    if (rows.length === 0) continue;
    const room = Math.max(0, DEV_LINES - named);
    named += Math.min(room, rows.length);
    unnamed += rows.length - Math.min(room, rows.length);
    if (room === 0) continue;
    if (lines.length > 0) lines.push("");
    lines.push(`**${title}**`, ...rows.slice(0, room).map((e) => `- ${e.scope ? `**${e.scope}**: ` : ""}${clip(e.clean, SUBJECT_MAX)}`));
  }
  if (unnamed > 0) lines.push(`- and ${unnamed} more`);
  const headline = new Set(HEADLINE_SECTIONS.map(([type]) => type));
  const rest = entries.filter((e) => e.type === null || !headline.has(e.type));
  if (rest.length > 0) {
    const kinds = [...new Set(rest.map((e) => (e.type === "docs" ? "documentation" : e.type === "test" ? "tests" : "internal work")))];
    if (lines.length > 0) lines.push("");
    lines.push(`${plural(rest.length, named + unnamed > 0 ? "other change" : "change")}: ${listed(kinds)}.`);
  }
  if (entries.length === 0) lines.push("No new commits since the last dev build.");
  lines.push("", o.since ? `[See all ${plural(entries.length, "commit")}](${REPO_URL}/compare/${o.since}...${o.sha})` : `[See the commit](${REPO_URL}/commit/${o.sha})`);
  return {
    title: `Dev build ${o.version}+dev`,
    url: `${REPO_URL}/commit/${o.sha}`,
    description: lines.join("\n"),
    color: BRAND,
    fields: [
      { name: "Docker", value: `\`\`\`\ndocker pull ${IMAGE}:dev\n\`\`\`` },
      { name: "Binaries", value: `[Windows, Linux and macOS](${o.runUrl}). The files are kept for 30 days.` },
    ],
    footer: { text: `${o.sha.slice(0, 7)} · A dev build is not a release. It can change or break at any time.` },
    timestamp: new Date(o.now).toISOString(),
  };
}

/** Release notes as an embed shows them: the version heading dropped, headings bold, cut at a line. */
export function notesText(notes: string, url: string): string {
  const lines = notes
    .replace(/<!--[\s\S]*?-->/g, "")
    .split("\n")
    .filter((l) => !/^##\s+\[/.test(l))
    .map((l) => l.replace(/^#{1,6}\s+(.*)$/, "**$1**"));
  const text = lines.join("\n").replace(/\n{3,}/g, "\n\n").trim();
  if (text.length <= NOTES_MAX) return text;
  const cut = text.lastIndexOf("\n", NOTES_MAX);
  return `${text.slice(0, cut > 0 ? cut : NOTES_MAX)}\n\n[Read all release notes](${url})`;
}

/** The message of a published release. */
export function releaseEmbed(o: { tag: string; url: string; notes: string; now: number }): Embed {
  const version = o.tag.replace(/^v/, "");
  return {
    title: `Novis ${version} is released`,
    url: o.url,
    description: notesText(o.notes, o.url) || `[Release notes](${o.url})`,
    color: BRAND,
    fields: [
      { name: "Install", value: `[Install page](${INSTALL_URL}) · [Release on GitHub](${o.url})` },
      { name: "Docker", value: `\`\`\`\ndocker pull ${IMAGE}:${version}\n\`\`\`` },
    ],
    timestamp: new Date(o.now).toISOString(),
  };
}

/** The workspace version, from the root `Cargo.toml`'s `[workspace.package]`. */
function workspaceVersion(toml: string): string {
  const m = /^\[workspace\.package\][^[]*?^version\s*=\s*"([^"]+)"/m.exec(toml);
  if (!m) throw new Fail("Cargo.toml has no [workspace.package] version");
  return m[1]!;
}

/** Posts one embed through the webhook. No error names the URL, which holds the webhook's token. */
async function post(url: string, embed: Embed, s: Seams): Promise<void> {
  const init: RequestInit = {
    method: "POST",
    headers: { "Content-Type": "application/json", "User-Agent": USER_AGENT },
    body: JSON.stringify({ embeds: [embed], allowed_mentions: { parse: [] } }),
  };
  for (let attempt = 0; ; attempt++) {
    let res: Response;
    try {
      res = await s.fetch(`${url}?wait=true`, init);
    } catch (e) {
      throw new Fail(`posting to the webhook failed: ${(e as { code?: string }).code ?? "network error"}`);
    }
    const answer = await parse(res);
    if (res.status === 429 && attempt < RETRIES) {
      const after = (answer as { retry_after?: number } | null)?.retry_after;
      await s.sleep(typeof after === "number" ? Math.ceil(after * 1000) : 1000);
      continue;
    }
    if (!res.ok) throw new Fail(`the webhook did not take the message: ${describe(res.status, answer)}`);
    return;
  }
}

async function announce(o: Opts, s: Seams): Promise<number> {
  const [kind, extra] = o.positional;
  if (extra !== undefined) throw new Usage(`unexpected argument ${extra}`);
  let embed: Embed;
  if (kind === "dev") {
    if (!o.sha || !o.runUrl) throw new Usage("announce dev needs --sha <commit> and --run-url <url>");
    for (const c of [o.sha, o.since]) if (c !== undefined && !COMMIT.test(c)) throw new Usage(`${c} is not a full commit hash`);
    const format = "--format=%H%x1f%s%x1f%b%x1e";
    let since = o.since ?? null;
    let log: string;
    try {
      log = s.git(["log", "--no-merges", format, since ? `${since}..${o.sha}` : `${o.sha}^!`]);
    } catch (e) {
      if (since === null) throw e;
      s.warn(`nv discord: git does not know ${since}, so the message names the one commit`);
      since = null;
      log = s.git(["log", "--no-merges", format, `${o.sha}^!`]);
    }
    const version = workspaceVersion(s.read(join(s.root, "Cargo.toml")));
    embed = devEmbed({ sha: o.sha, since, version, runUrl: o.runUrl, log, now: s.now() });
  } else if (kind === "release") {
    if (!o.tag || !o.url || !o.notesFile) throw new Usage("announce release needs --tag <vX.Y.Z>, --url <url> and --notes-file <path>");
    if (!/^v\d+\.\d+\.\d+/.test(o.tag)) throw new Usage(`${o.tag} is not a release tag, as in v1.2.3`);
    embed = releaseEmbed({ tag: o.tag, url: o.url, notes: s.read(o.notesFile), now: s.now() });
  } else throw new Usage("announce needs dev or release");
  if (o.dryRun) {
    s.say(JSON.stringify({ embeds: [embed] }, null, 2));
    return 0;
  }
  const url = s.env.DISCORD_WEBHOOK_URL;
  if (!url || !WEBHOOK.test(url)) throw new Fail("DISCORD_WEBHOOK_URL is not set to a Discord webhook URL");
  await post(url, embed, s);
  s.say(`nv discord: posted the ${kind} message`);
  return 0;
}

interface Opts {
  positional: string[];
  tokenFile?: string;
  channel?: string;
  guild?: string;
  reason?: string;
  bodyFile?: string;
  out?: string;
  sha?: string;
  since?: string;
  runUrl?: string;
  tag?: string;
  url?: string;
  notesFile?: string;
  all: boolean;
  max: number;
  dryRun: boolean;
}

const VALUED = {
  "--token-file": "tokenFile",
  "--channel": "channel",
  "--guild": "guild",
  "--reason": "reason",
  "--body-file": "bodyFile",
  "--out": "out",
  "--sha": "sha",
  "--since": "since",
  "--run-url": "runUrl",
  "--tag": "tag",
  "--url": "url",
  "--notes-file": "notesFile",
} as const;

function parseArgs(args: string[]): Opts {
  const o: Opts = { positional: [], all: false, max: DEFAULT_MAX, dryRun: false };
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--token") throw new Usage("the token is read from a file only: pass --token-file <path>");
    if (a === "--all") o.all = true;
    else if (a === "--dry-run") o.dryRun = true;
    else if (a === "--max") {
      const n = Number(args[++i]);
      if (!Number.isInteger(n) || n < 1) throw new Usage("--max needs a positive whole number");
      o.max = n;
    } else if (a in VALUED) {
      const v = args[++i];
      if (v === undefined) throw new Usage(`${a} needs a value`);
      o[VALUED[a as keyof typeof VALUED]] = v;
    } else if (a.startsWith("--")) throw new Usage(`unexpected option ${a}`);
    else o.positional.push(a);
  }
  for (const k of ["channel", "guild"] as const) if (o[k] !== undefined && !SNOWFLAKE.test(o[k]!)) throw new Usage(`--${k} needs a numeric id, not ${o[k]}`);
  if (o.guild && o.channel) throw new Usage("name the server once: --guild or --channel");
  return o;
}

function readToken(o: Opts, s: Seams): string {
  if (!o.tokenFile) throw new Usage("--token-file is required: the path the secret prompt gave");
  let text: string;
  try {
    text = s.read(o.tokenFile);
  } catch {
    throw new Fail(`--token-file ${o.tokenFile} cannot be read`);
  }
  const token = text.trim().replace(/^Bot\s+/i, "");
  if (token === "") throw new Fail(`--token-file ${o.tokenFile} is empty`);
  return token;
}

async function api(o: Opts, s: Seams): Promise<number> {
  const [method, given, inline, extra] = o.positional;
  if (method === undefined || given === undefined) throw new Usage("api needs a method and a path");
  if (extra !== undefined) throw new Usage(`unexpected argument ${extra}`);
  if (!METHODS.has(method)) throw new Usage(`no method ${method}: GET, POST, PUT, PATCH or DELETE`);
  // Git Bash rewrites an argument that starts with `/` into a Windows path, so the `/` is optional.
  if (/^[A-Za-z]:[\\/]/.test(given)) throw new Usage(`${given} is a file path: Git Bash rewrote a leading /, so write the path without it, as in channels/{id}`);
  const path = given.startsWith("/") ? given : `/${given}`;
  if (inline !== undefined && o.bodyFile) throw new Usage("give the body once: inline or --body-file");
  if (o.reason !== undefined && (o.reason.length === 0 || o.reason.length > REASON_MAX)) throw new Usage(`--reason is 1 to ${REASON_MAX} characters`);
  if (o.all && method !== "GET") throw new Usage("--all follows the pages of a GET");
  const out = o.out === undefined ? undefined : scratch(o.out, s.root);
  const client = new Client(readToken(o, s), s);
  const scope = new Scope(client, o.guild, o.channel);
  const filled = await fill(path, scope);
  const text = inline ?? (o.bodyFile === undefined ? undefined : s.read(o.bodyFile));
  let body: unknown;
  if (text !== undefined) {
    try {
      body = JSON.parse(await fill(text, scope));
    } catch (e) {
      if (e instanceof SyntaxError) throw new Usage(`the body is not JSON: ${e.message}`);
      throw e;
    }
  }
  if (o.dryRun) {
    s.say(`dry run, nothing sent: ${method} ${API}${filled}`);
    if (o.reason !== undefined) s.say(`reason: ${o.reason}`);
    if (body !== undefined) s.say(JSON.stringify(body, null, 2));
    return 0;
  }
  const data = o.all ? await paginate(client, filled, o.max, s) : await client.call(method, filled, { body, reason: o.reason });
  if (out !== undefined) {
    s.write(out, `${JSON.stringify(data, null, 2)}\n`);
    s.say(`${method} ${filled}: ${Array.isArray(data) ? `${data.length} items` : "the answer"} written to ${shown(out, s.root)}`);
  } else s.say(data === null ? `${method} ${filled}: done` : JSON.stringify(data, null, 2));
  return 0;
}

/** Parses, runs one subcommand and prints; returns the exit code. */
export async function discord(args: string[], over: Partial<Seams> = {}): Promise<number> {
  const s: Seams = { ...REAL, ...over };
  const [sub, ...rest] = args;
  try {
    const o = parseArgs(rest);
    switch (sub) {
      case "api":
        return await api(o, s);
      case "check": {
        if ((!o.channel && !o.guild) || o.positional.length > 0) throw new Usage("check needs --guild <id> or --channel <id>, and nothing else");
        return await check(new Client(readToken(o, s), s), { guild: o.guild, channel: o.channel }, s);
      }
      case "snapshot": {
        if (o.out === undefined || o.positional.length > 0) throw new Usage("snapshot needs --out <path> and nothing else");
        const out = scratch(o.out, s.root);
        const client = new Client(readToken(o, s), s);
        const snap = await snapshot(client, await new Scope(client, o.guild, o.channel).guild());
        s.write(out, `${JSON.stringify(snap, null, 2)}\n`);
        const n = (k: string) => (Array.isArray(snap[k]) ? (snap[k] as unknown[]).length : 0);
        s.say(`snapshot of ${(snap.guild as { name?: string }).name ?? "the server"}: ${n("channels")} channels, ${n("roles")} roles, ${n("automod")} AutoMod rules, written to ${shown(out, s.root)}`);
        return 0;
      }
      case "diff": {
        const [a, b, extra] = o.positional;
        if (a === undefined || b === undefined || extra !== undefined) throw new Usage("diff needs two snapshot files");
        const lines = diff(JSON.parse(s.read(a)), JSON.parse(s.read(b)));
        for (const l of lines) s.say(l);
        if (lines.length === 0) s.say("no difference");
        return 0;
      }
      case "bits": {
        if (o.positional.length === 0) throw new Usage("bits needs flag names or a bitfield");
        const [first] = o.positional;
        s.say(o.positional.length === 1 && /^\d+$/.test(first!) ? permissionNames(first!).join(" ") : permissionBits(o.positional));
        return 0;
      }
      case "announce":
        return await announce(o, s);
      default:
        throw new Usage(sub === undefined ? "name a subcommand" : `no subcommand ${sub}`);
    }
  } catch (e) {
    if (e instanceof Usage) {
      s.warn(`${USAGE}\nnv discord: error: ${e.message}`);
      return 2;
    }
    if (e instanceof DiscordError || e instanceof Fail) {
      s.warn(`nv discord: ${e.message}`);
      return 1;
    }
    throw e;
  }
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return discord(args);
}
