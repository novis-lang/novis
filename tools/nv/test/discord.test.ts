// `bun nv discord` over a fake Discord: every request is answered here from a table of routes, so no
// network and no token are needed, and each test reads back what was sent.

import { describe, expect, test } from "bun:test";
import { relative, resolve } from "node:path";
import { API, diff, discord, permissionBits, permissionNames, type Seams } from "../cmd/discord.ts";

const TOKEN = "secret-token-value";
const GUILD = "100000000000000001";
const CHANNEL = "100000000000000002";
const BOT = "100000000000000003";
const ROOT = resolve("/repo");

interface Answer {
  status?: number;
  body?: unknown;
  headers?: Record<string, string>;
}

interface Sent {
  method: string;
  path: string;
  headers: Record<string, string>;
  body?: unknown;
}

/** A Discord that answers `routes["METHOD /path?query"]`, a list of answers given in turn (the last repeats). */
function fake(routes: Record<string, Answer | Answer[]>, files: Record<string, string> = {}) {
  const sent: Sent[] = [];
  const out: string[] = [];
  const err: string[] = [];
  const slept: number[] = [];
  const written: Record<string, string> = {};
  const seen = new Map<string, number>();
  const seams: Partial<Seams> = {
    fetch: async (url, init) => {
      const path = url.slice(API.length);
      const key = `${init.method} ${path}`;
      sent.push({ method: init.method!, path, headers: init.headers as Record<string, string>, body: init.body === undefined ? undefined : JSON.parse(init.body as string) });
      const r = routes[key];
      if (r === undefined) return new Response(JSON.stringify({ message: "404: Not Found", code: 0 }), { status: 404 });
      const list = Array.isArray(r) ? r : [r];
      const n = seen.get(key) ?? 0;
      seen.set(key, n + 1);
      const a = list[Math.min(n, list.length - 1)]!;
      const status = a.status ?? 200;
      return new Response(status === 204 ? null : JSON.stringify(a.body ?? {}), { status, headers: a.headers ?? {} });
    },
    sleep: async (ms) => void slept.push(ms),
    say: (l) => void out.push(l),
    warn: (l) => void err.push(l),
    read: (p) => {
      const f = { "/tok": `${TOKEN}\n`, ...files }[p];
      if (f === undefined) throw new Error(`ENOENT ${p}`);
      return f;
    },
    write: (p, t) => void (written[relative(ROOT, p).replaceAll("\\", "/")] = t),
    root: ROOT,
  };
  return { sent, out, err, slept, written, seams };
}

const T = ["--token-file", "/tok"];

describe("bun nv discord api", () => {
  test("sends the bot token, the user agent, the JSON body and the URL-encoded audit reason", async () => {
    const f = fake({ [`PATCH /channels/${CHANNEL}`]: { body: { id: CHANNEL, topic: "Releases" } } });
    const code = await discord(["api", ...T, "PATCH", `/channels/${CHANNEL}`, '{"topic":"Releases"}', "--reason", "topic für #releases"], f.seams);
    expect(code).toBe(0);
    expect(f.sent).toHaveLength(1);
    const s = f.sent[0]!;
    expect(s.headers.Authorization).toBe(`Bot ${TOKEN}`);
    expect(s.headers["User-Agent"]).toStartWith("DiscordBot (");
    expect(s.headers["Content-Type"]).toBe("application/json");
    expect(s.headers["X-Audit-Log-Reason"]).toBe("topic%20f%C3%BCr%20%23releases");
    expect(s.body).toEqual({ topic: "Releases" });
    expect(f.out.join("\n")).toContain('"topic": "Releases"');
  });

  test("the token never appears in anything printed, even when Discord refuses the call", async () => {
    const f = fake({ "DELETE /channels/1": { status: 403, body: { message: "Missing Permissions", code: 50013 } } });
    const code = await discord(["api", ...T, "DELETE", "/channels/1"], f.seams);
    expect(code).toBe(1);
    expect(f.err.join("\n")).toContain("DELETE /channels/1 failed: 403 Missing Permissions (code 50013)");
    expect([...f.out, ...f.err].join("\n")).not.toContain(TOKEN);
  });

  test("a 429 is retried after its retry_after, and an emptied bucket is waited out before the next call", async () => {
    const f = fake({
      "GET /users/@me": [
        { status: 429, body: { message: "You are being rate limited.", retry_after: 1.25, global: false } },
        { body: { id: BOT }, headers: { "X-RateLimit-Remaining": "0", "X-RateLimit-Reset-After": "0.5" } },
      ],
      [`GET /channels/${CHANNEL}`]: { body: { id: CHANNEL, guild_id: GUILD } },
      [`GET /guilds/${GUILD}/members/${BOT}`]: { body: { roles: [] } },
    });
    const code = await discord(["api", ...T, "--channel", CHANNEL, "GET", "/guilds/{guild}/members/{me}"], f.seams);
    expect(code).toBe(0);
    expect(f.slept).toEqual([1250, 500]);
    expect(f.sent.map((s) => s.path)).toEqual([`/channels/${CHANNEL}`, "/users/@me", "/users/@me", `/guilds/${GUILD}/members/${BOT}`]);
  });

  test("a GET is retried after a 5xx and a write is not, so a write is never sent twice", async () => {
    const f = fake({
      "GET /guilds/9": [{ status: 502, body: "bad gateway" }, { body: { id: "9" } }],
      "POST /channels/9/messages": { status: 502, body: "bad gateway" },
    });
    expect(await discord(["api", ...T, "GET", "guilds/9"], f.seams)).toBe(0);
    expect(await discord(["api", ...T, "POST", "/channels/9/messages", '{"content":"hi"}'], f.seams)).toBe(1);
    expect(f.sent.filter((s) => s.method === "POST")).toHaveLength(1);
  });

  test("{channel:<name>} and {role:<name>} become ids in the path and the body", async () => {
    const f = fake({
      [`GET /guilds/${GUILD}/channels`]: { body: [{ id: "201", name: "releases" }, { id: "202", name: "general" }] },
      [`GET /guilds/${GUILD}/roles`]: { body: [{ id: GUILD, name: "@everyone" }, { id: "301", name: "Mods" }] },
      "PUT /channels/201/permissions/301": { status: 204 },
    });
    const body = '{"type":0,"allow":"8192","id":"{role:@Mods}"}';
    const code = await discord(["api", ...T, "--guild", GUILD, "PUT", "/channels/{channel:#releases}/permissions/{role:Mods}", body], f.seams);
    expect(code).toBe(0);
    expect(f.sent.at(-1)!.path).toBe("/channels/201/permissions/301");
    expect(f.sent.at(-1)!.body).toEqual({ type: 0, allow: "8192", id: "301" });
    expect(f.out.at(-1)).toBe("PUT /channels/201/permissions/301: done");
  });

  test("a name two channels share is an error that names both ids, and nothing is sent", async () => {
    const f = fake({ [`GET /guilds/${GUILD}/channels`]: { body: [{ id: "201", name: "chat" }, { id: "202", name: "chat" }] } });
    const code = await discord(["api", ...T, "--guild", GUILD, "DELETE", "/channels/{channel:chat}"], f.seams);
    expect(code).toBe(1);
    expect(f.err.join("\n")).toContain("2 channels are named chat (201, 202)");
    expect(f.sent.every((s) => s.method === "GET")).toBe(true);
  });

  test("--dry-run resolves the call and prints it without sending it", async () => {
    const f = fake({ [`GET /guilds/${GUILD}/roles`]: { body: [{ id: "301", name: "Mods" }] } });
    const code = await discord(["api", ...T, "--guild", GUILD, "--dry-run", "--reason", "cleanup", "DELETE", "/guilds/{guild}/roles/{role:Mods}"], f.seams);
    expect(code).toBe(0);
    expect(f.out[0]).toBe(`dry run, nothing sent: DELETE ${API}/guilds/${GUILD}/roles/301`);
    expect(f.sent.every((s) => s.method === "GET")).toBe(true);
  });

  test("--all pages a channel's messages backwards with before, and members forwards with after and limit 1000", async () => {
    const f = fake({
      "GET /channels/5/messages?limit=2": { body: [{ id: "30" }, { id: "29" }] },
      "GET /channels/5/messages?limit=2&before=29": { body: [{ id: "28" }] },
      "GET /guilds/6/members?limit=1000": { body: [{ user: { id: "11" } }, { user: { id: "12" } }] },
    });
    expect(await discord(["api", ...T, "--all", "--out", ".agent-tmp/m.json", "GET", "/channels/5/messages?limit=2"], f.seams)).toBe(0);
    expect(JSON.parse(f.written[".agent-tmp/m.json"]!)).toEqual([{ id: "30" }, { id: "29" }, { id: "28" }]);
    expect(f.out.at(-1)).toContain("3 items written to");
    expect(await discord(["api", ...T, "--all", "GET", "/guilds/6/members"], f.seams)).toBe(0);
    expect(f.sent.at(-1)!.path).toBe("/guilds/6/members?limit=1000");
  });

  test("--all stops at --max and says the list may go on", async () => {
    const f = fake({ "GET /guilds/6/bans?limit=1000": { body: [{ user: { id: "1" } }, { user: { id: "2" } }, { user: { id: "3" } }] } });
    expect(await discord(["api", ...T, "--all", "--max", "2", "GET", "/guilds/6/bans"], f.seams)).toBe(0);
    expect(JSON.parse(f.out.at(-1)!)).toHaveLength(2);
    expect(f.err.join("\n")).toContain("stopped at --max 2");
  });

  test("usage errors: a token argument, --out outside .agent-tmp, a body that is not JSON", async () => {
    const f = fake({});
    expect(await discord(["api", "--token", TOKEN, "GET", "/users/@me"], f.seams)).toBe(2);
    expect(await discord(["api", ...T, "--out", "docs/server.json", "GET", "/users/@me"], f.seams)).toBe(2);
    expect(await discord(["api", ...T, "--out", ".agent-tmp", "GET", "/users/@me"], f.seams)).toBe(2);
    expect(await discord(["api", ...T, "POST", "/channels/1/messages", "{content: hi}"], f.seams)).toBe(2);
    expect(await discord(["api", "GET", "/users/@me"], f.seams)).toBe(2);
    expect(await discord(["api", ...T, "GET", "C:/Program Files/Git/users/@me"], f.seams)).toBe(2);
    expect(f.err.at(-1)).toContain("Git Bash rewrote a leading /");
    expect(f.sent).toHaveLength(0);
    expect(f.err.join("\n")).toContain("server data never goes into the tree");
    expect(f.err.join("\n")).not.toContain(TOKEN);
  });
});

describe("bun nv discord snapshot and diff", () => {
  const routes = (topic: string): Record<string, Answer> => ({
    [`GET /guilds/${GUILD}`]: { body: { id: GUILD, name: "Novis", roles: [{ id: "x" }], approximate_member_count: 40 } },
    [`GET /guilds/${GUILD}/channels`]: {
      body: [
        { id: "202", type: 0, name: "general", position: 0, parent_id: "200", last_message_id: "999", topic, permission_overwrites: [] },
        { id: "200", type: 4, name: "Community", position: 1, permission_overwrites: [] },
        { id: "201", type: 0, name: "rules", position: 0, permission_overwrites: [{ id: GUILD, type: 0, allow: "0", deny: "1024" }] },
      ],
    },
    [`GET /guilds/${GUILD}/roles`]: { body: [{ id: GUILD, name: "@everyone", position: 0, permissions: "1024" }, { id: "301", name: "Mods", position: 1, permissions: "8" }] },
    [`GET /guilds/${GUILD}/webhooks`]: { body: [{ id: "401", name: "CI", token: "hook-secret", url: "https://discord.com/api/webhooks/401/hook-secret" }] },
    [`GET /guilds/${GUILD}/onboarding`]: { status: 403, body: { message: "Missing Access", code: 50001 } },
  });

  test("snapshot names permissions, channel kinds and categories, drops webhook tokens and traffic counters", async () => {
    const f = fake(routes("Say hi"));
    expect(await discord(["snapshot", ...T, "--guild", GUILD, "--out", ".agent-tmp/before.json"], f.seams)).toBe(0);
    const snap = JSON.parse(f.written[".agent-tmp/before.json"]!);
    expect(snap.guild).toEqual({ id: GUILD, name: "Novis" });
    expect(snap.roles.map((r: { name: string; permissions: string[] }) => [r.name, r.permissions])).toEqual([["Mods", ["ADMINISTRATOR"]], ["@everyone", ["VIEW_CHANNEL"]]]);
    expect(snap.channels.map((c: { name: string }) => c.name)).toEqual(["rules", "Community", "general"]);
    expect(snap.channels[0].permission_overwrites[0]).toEqual({ id: GUILD, type: 0, allow: [], deny: ["VIEW_CHANNEL"], target: "@everyone" });
    expect(snap.channels[2]).toMatchObject({ kind: "text", category: "Community" });
    expect(snap.channels[2].last_message_id).toBeUndefined();
    expect(snap.webhooks).toEqual([{ id: "401", name: "CI" }]);
    expect(snap.onboarding).toEqual({ unavailable: "403 Missing Access (code 50001)" });
    expect(f.written[".agent-tmp/before.json"]).not.toContain("hook-secret");
    expect(f.out.at(-1)).toBe("snapshot of Novis: 3 channels, 2 roles, 0 AutoMod rules, written to .agent-tmp/before.json");
  });

  test("diff of two snapshots names each change by the channel or role it is in", async () => {
    const before = fake(routes("Say hi"));
    await discord(["snapshot", ...T, "--guild", GUILD, "--out", ".agent-tmp/a.json"], before.seams);
    const after = fake(routes("Be nice"));
    await discord(["snapshot", ...T, "--guild", GUILD, "--out", ".agent-tmp/b.json"], after.seams);
    const a = JSON.parse(before.written[".agent-tmp/a.json"]!);
    const b = JSON.parse(after.written[".agent-tmp/b.json"]!);
    b.roles.push({ id: "302", name: "Helpers", position: 2, permissions: [] });
    b.channels = b.channels.filter((c: { name: string }) => c.name !== "rules");
    expect(diff(a, b)).toEqual(["- channels[rules]", '~ channels[general].topic: "Say hi" -> "Be nice"', '+ roles[Helpers] {"id":"302","name":"Helpers","position":2,"permissions":[]}']);
    expect(diff(a, a)).toEqual([]);
  });
});

describe("bun nv discord check", () => {
  const ok: Record<string, Answer> = {
    "GET /users/@me": { body: { id: BOT, username: "Novis Admin" } },
    "GET /applications/@me": { body: { flags: (1 << 15) | (1 << 19), bot_public: false } },
    [`GET /channels/${CHANNEL}`]: { body: { id: CHANNEL, name: "general", guild_id: GUILD } },
    [`GET /guilds/${GUILD}`]: {
      body: {
        name: "Novis",
        mfa_level: 0,
        roles: [
          { id: GUILD, name: "@everyone", position: 0, permissions: "0" },
          { id: "301", name: "Novis Admin", position: 2, permissions: "8" },
          { id: "302", name: "Mods", position: 1, permissions: "0" },
        ],
      },
    },
    [`GET /guilds/${GUILD}/members/${BOT}`]: { body: { roles: ["301"] } },
  };

  test("a complete setup is all GREEN", async () => {
    const f = fake(ok);
    expect(await discord(["check", ...T, "--channel", CHANNEL], f.seams)).toBe(0);
    expect(f.out.every((l) => l.startsWith("GREEN"))).toBe(true);
    expect(f.out).toHaveLength(7);
  });

  test("each missing step is a RED line that says how to fix it", async () => {
    const f = fake({
      ...ok,
      "GET /applications/@me": { body: { flags: 0, bot_public: true } },
      [`GET /guilds/${GUILD}/members/${BOT}`]: { body: { roles: ["302"] } },
      [`GET /guilds/${GUILD}`]: { body: { ...(ok[`GET /guilds/${GUILD}`]!.body as object), mfa_level: 1 } },
    });
    expect(await discord(["check", ...T, "--channel", CHANNEL], f.seams)).toBe(1);
    const red = f.out.filter((l) => l.startsWith("RED"));
    expect(red.map((l) => l.split(" ").slice(1, 3).join(" "))).toEqual(["Server Members", "Message Content", "Public Bot", "the bot", "roles above"]);
    expect(red.at(-1)).toContain("Novis Admin");
    expect(f.out.at(-1)).toStartWith("NOTE the server requires 2FA");
  });

  test("a rejected token and an unseen channel stop the check with what to do", async () => {
    const bad = fake({ "GET /users/@me": { status: 401, body: { message: "401: Unauthorized", code: 0 } } });
    expect(await discord(["check", ...T, "--channel", CHANNEL], bad.seams)).toBe(1);
    expect(bad.out).toEqual(["RED the token is not accepted: reset it under the application's Bot tab and use the new one"]);
    const unseen = fake({ ...ok, [`GET /channels/${CHANNEL}`]: { status: 403, body: { message: "Missing Access", code: 50001 } } });
    expect(await discord(["check", ...T, "--channel", CHANNEL], unseen.seams)).toBe(1);
    expect(unseen.out.at(-1)).toContain("the bot cannot see channel");
  });
});

describe("bun nv discord bits", () => {
  test("names turn into a bitfield and a bitfield into names, in either spelling", async () => {
    expect(permissionBits(["VIEW_CHANNEL", "SendMessages"])).toBe("3072");
    expect(permissionNames("3072")).toEqual(["VIEW_CHANNEL", "SEND_MESSAGES"]);
    expect(permissionNames(String(1n << 52n | 1n << 47n))).toEqual(["BIT_47", "BYPASS_SLOWMODE"]);
    const f = fake({});
    expect(await discord(["bits", "PinMessages"], f.seams)).toBe(0);
    expect(f.out).toEqual([String(1n << 51n)]);
    expect(await discord(["bits", "NoSuchFlag"], f.seams)).toBe(2);
  });
});
