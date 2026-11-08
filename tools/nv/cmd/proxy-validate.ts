// `bun nv proxy-validate`: an nginx configuration, a Caddyfile or a Dockerfile, checked by the tool that
// reads it, in its official image pinned by digest.
//
//     bun nv proxy-validate [--plugin <module>]... <file>...
//
// A file is judged by its name: `Caddyfile` or `*.caddyfile` is a Caddyfile, `Dockerfile` or
// `*.dockerfile` is a Dockerfile, and anything else is nginx. The `code` and `apps` parts of
// `bun nv site --check` call `validate` with a page block's text or an app's `deploy/` file.
//
// - **nginx**: `nginx -t` in `NGINX_IMAGE`. The text is a file nginx reads inside its `http` block, as
//   one under `/etc/nginx/conf.d/` is, and replaces the image's own `default.conf`. A text with an
//   `http { }` block of its own is a whole `nginx.conf` instead. Each certificate and key a
//   `ssl_certificate`, `ssl_certificate_key`, `ssl_trusted_certificate` or `ssl_client_certificate`
//   line names gets a self-signed stub at that path first, so the check needs no real certificate.
// - **Caddyfile**: `caddy validate --adapter caddyfile` in `CADDY_IMAGE`. A Caddyfile that needs a
//   plugin names it with `--plugin` (a page block: `plugins="<module> ..."` in its fence). The plugin
//   must be in `CADDY_PLUGINS`, which pins its version, and Caddy is built with every plugin a run
//   names by `xcaddy` in `CADDY_BUILDER_IMAGE`. The build is a local image tagged by what it holds,
//   so it runs once per machine.
// - **Dockerfile**: `docker build --check` over an empty context, which runs Docker's build checks and
//   builds nothing.
//
// The text reaches the container on standard input, so nothing is mounted and nothing is written in
// the tree. A new pin is taken from `docker buildx imagetools inspect <image>:<version>`, and each pull
// runs once per process, without the hang watch, because it waits on the network on purpose.
//
// Where Docker runs no Linux containers, nothing is checked, and the reason is returned as `skipped`
// (`dockerAbsent` in `proofs/run.ts`, the one probe), as `bun nv pdf-validate` does. CI runs these
// checks on the Linux leg. The command exits 0 when every file passes or Docker cannot run them, 1
// naming each file that fails with the tool's own message, and 2 on a usage error.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { ROOT, rel } from "../lib/paths.ts";
import { failLine, run as runProc } from "../lib/proc.ts";
import { dockerAbsent } from "../proofs/run.ts";

export const summary = "nginx, Caddyfile and Dockerfile files checked by nginx -t, caddy validate and docker build --check: nv proxy-validate [--plugin <module>]... <file>...";

const USAGE = "usage: bun nv proxy-validate [--plugin <module>]... <file>...";

/** nginx's image, `nginx:1.31.8`, by digest. */
export const NGINX_IMAGE = "nginx@sha256:165ddde8751e75a1c907d24ef10d9f6deb08b57e0fe56caf67512486aff76d82";
/** Caddy's image, `caddy:2.11.9`, by digest. */
export const CADDY_IMAGE = "caddy@sha256:0c2243efcf437ab7f7a978f1027fcf0d4f9be8a75df27b1b3fbc5dbe0ffa8cf0";
/** Caddy's builder image, `caddy:2.11.9-builder`, by digest: it has `xcaddy`. */
export const CADDY_BUILDER_IMAGE = "caddy@sha256:3fddbec729442a56b942d4c9eaef1df80893dbf2cf5b86701bfae6aad1408eeb";
/** Each Caddy plugin a Caddyfile may name, and the version it is built at. */
export const CADDY_PLUGINS: Record<string, string> = { "github.com/mholt/caddy-ratelimit": "v0.1.0" };

export type ProxyKind = "nginx" | "caddyfile" | "dockerfile";

/** The problems the tool found, none when the text passes, or why nothing was checked. */
export type ProxyVerdict = { problems: string[] } | { skipped: string };

/** How long one check may run. A Caddy build with plugins downloads its modules first. */
const CHECK_MS = 120_000;
const BUILD_MS = 900_000;

/** The kind a file is, by its name. */
export function kindOf(path: string): ProxyKind {
  const name = basename(path).toLowerCase();
  if (name === "caddyfile" || name.endsWith(".caddyfile")) return "caddyfile";
  if (name === "dockerfile" || name.endsWith(".dockerfile")) return "dockerfile";
  return "nginx";
}

/** Each image's pull, once per process: null once it is here, or the failure. */
const pulled = new Map<string, Promise<string | null>>();

function pull(image: string): Promise<string | null> {
  let p = pulled.get(image);
  if (!p) {
    p = runProc(["docker", "pull", "--quiet", image], { cwd: ROOT, timeoutMs: 600_000, hang: false }).then((r) =>
      r.code === 0 ? null : failLine(r.stderr + r.stdout, r.code, `docker pull ${image}`),
    );
    pulled.set(image, p);
  }
  return p;
}

/** A word for a `sh -c` script, in single quotes. */
const quote = (s: string): string => `'${s.replace(/'/g, `'\\''`)}'`;

/** The certificate and key paths an nginx text names, without any that holds a variable. */
export function nginxCertPaths(text: string): { certs: string[]; keys: string[] } {
  const certs = new Set<string>();
  const keys = new Set<string>();
  for (const m of text.matchAll(/(?<=^|[;{])\s*(ssl_certificate_key|ssl_certificate|ssl_trusted_certificate|ssl_client_certificate)\s+("[^"]+"|'[^']+'|[^\s;]+)\s*;/gm)) {
    const path = m[2]!.replace(/^["']|["']$/g, "");
    if (path.includes("$") || path.startsWith("data:")) continue;
    (m[1] === "ssl_certificate_key" ? keys : certs).add(path);
  }
  return { certs: [...certs], keys: [...keys] };
}

/** The shell script that stubs the certificates, puts the text where nginx reads it, and runs `nginx -t`. */
export function nginxScript(text: string): string {
  const { certs, keys } = nginxCertPaths(text);
  const whole = /^\s*http\s*\{/m.test(text);
  const lines = ["set -e"];
  if (certs.length || keys.length) {
    lines.push("openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -subj /CN=stub -days 1 -keyout /tmp/stub.key -out /tmp/stub.crt 2>/dev/null");
    for (const c of certs) lines.push(`mkdir -p "$(dirname ${quote(c)})" && cp /tmp/stub.crt ${quote(c)}`);
    for (const k of keys) lines.push(`mkdir -p "$(dirname ${quote(k)})" && cp /tmp/stub.key ${quote(k)}`);
  }
  if (whole) lines.push("cat > /etc/nginx/nginx.conf");
  else lines.push("rm -f /etc/nginx/conf.d/*.conf", "cat > /etc/nginx/conf.d/site.conf");
  lines.push("nginx -t -q");
  return lines.join("\n");
}

/** The tool's own message, without the lines that only say the test passed or failed, and without
 * nginx's timestamped copy of each error. */
const toolMessage = (out: string): string =>
  out
    .split(/\r?\n/)
    .filter((l) => l.trim() && !/configuration file .* test (failed|is successful)/.test(l) && !/^\d{4}\/\d\d\/\d\d [\d:]+ \[/.test(l))
    .join("\n");

/** The local Caddy image with each plugin built in, built once per machine: its tag, or the failure. */
const built = new Map<string, Promise<{ tag: string } | { error: string }>>();

function caddyWith(plugins: string[]): Promise<{ tag: string } | { error: string }> {
  const sorted = [...new Set(plugins)].sort();
  const withs = sorted.map((p) => `--with ${p}@${CADDY_PLUGINS[p]}`).join(" ");
  const dockerfile = `FROM ${CADDY_BUILDER_IMAGE} AS build\nRUN xcaddy build ${withs}\nFROM ${CADDY_IMAGE}\nCOPY --from=build /usr/bin/caddy /usr/bin/caddy\n`;
  const tag = `novis-caddy-check:${createHash("sha256").update(dockerfile).digest("hex").slice(0, 16)}`;
  let p = built.get(tag);
  if (!p) {
    p = (async () => {
      const have = await runProc(["docker", "image", "inspect", "--format", "{{.Id}}", tag], { cwd: ROOT, timeoutMs: 60_000 });
      if (have.code === 0) return { tag };
      for (const image of [CADDY_BUILDER_IMAGE, CADDY_IMAGE]) {
        const failed = await pull(image);
        if (failed) return { error: failed };
      }
      const empty = join(ROOT, ".agent-tmp", `proxy-validate-${process.pid}-context`);
      mkdirSync(empty, { recursive: true });
      try {
        const r = await runProc(["docker", "build", "--quiet", "-t", tag, "-f", "-", empty], { cwd: ROOT, input: dockerfile, timeoutMs: BUILD_MS, hang: false });
        return r.code === 0 ? { tag } : { error: failLine(r.stderr + r.stdout, r.code, `docker build of Caddy with ${sorted.join(", ")}`) };
      } finally {
        rmSync(empty, { recursive: true, force: true });
      }
    })();
    built.set(tag, p);
  }
  return p;
}

/**
 * Checks one text of `kind` with its tool. `plugins` are the Caddy plugins a Caddyfile needs, each of
 * them in `CADDY_PLUGINS`.
 */
export async function validate(kind: ProxyKind, text: string, plugins: string[] = []): Promise<ProxyVerdict> {
  const unknown = plugins.filter((p) => !(p in CADDY_PLUGINS));
  if (unknown.length) return { problems: [`no pinned version for the Caddy plugin ${unknown.map((p) => `\`${p}\``).join(", ")}: add it to \`CADDY_PLUGINS\` in tools/nv/cmd/proxy-validate.ts`] };
  if (plugins.length && kind !== "caddyfile") return { problems: ["only a Caddyfile takes plugins"] };
  const absent = await dockerAbsent();
  if (absent !== null) return { skipped: absent };
  const input = text.endsWith("\n") ? text : `${text}\n`;

  if (kind === "dockerfile") {
    const empty = join(ROOT, ".agent-tmp", `proxy-validate-${process.pid}-context`);
    mkdirSync(empty, { recursive: true });
    try {
      const r = await runProc(["docker", "build", "--check", "-f", "-", empty], { cwd: ROOT, input, timeoutMs: CHECK_MS, hang: false });
      return { problems: r.code === 0 ? [] : [`\`docker build --check\` fails:\n${toolMessage(r.stderr + r.stdout)}`] };
    } finally {
      rmSync(empty, { recursive: true, force: true });
    }
  }

  let image = kind === "nginx" ? NGINX_IMAGE : CADDY_IMAGE;
  if (plugins.length) {
    const b = await caddyWith(plugins);
    if ("error" in b) return { problems: [b.error] };
    image = b.tag;
  } else {
    const failed = await pull(image);
    if (failed) return { problems: [failed] };
  }
  const argv =
    kind === "nginx"
      ? ["docker", "run", "-i", "--rm", "--network", "none", image, "sh", "-c", nginxScript(input)]
      : ["docker", "run", "-i", "--rm", "--network", "none", image, "sh", "-c", "cat > /tmp/Caddyfile && caddy validate --config /tmp/Caddyfile --adapter caddyfile 2>&1"];
  const r = await runProc(argv, { cwd: ROOT, input, timeoutMs: CHECK_MS });
  if (r.code === 0) return { problems: [] };
  const what = kind === "nginx" ? "`nginx -t`" : "`caddy validate`";
  const message = kind === "nginx" ? toolMessage(r.stderr + r.stdout) : caddyMessage(r.stderr + r.stdout);
  return { problems: [`${what} fails:\n${message}`] };
}

/** Caddy's error: the `msg` of each `error` line of its JSON log, or its last lines when it has none. */
export function caddyMessage(out: string): string {
  const lines = out.split(/\r?\n/).filter((l) => l.trim());
  const errors: string[] = [];
  for (const l of lines) {
    if (/^Error: /.test(l)) errors.push(l);
    else if (l.startsWith("{")) {
      try {
        const entry = JSON.parse(l) as { level?: string; msg?: string; error?: string };
        if (entry.level === "error" || entry.level === "fatal") errors.push([entry.msg, entry.error].filter(Boolean).join(": "));
      } catch {
        // Not a log line: the last lines below show it.
      }
    }
  }
  return errors.length ? errors.join("\n") : lines.slice(-5).join("\n");
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(USAGE);
    return 0;
  }
  const plugins: string[] = [];
  const files: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--plugin") {
      const v = args[++i];
      if (v === undefined) {
        console.error(`nv proxy-validate: --plugin needs a module path\n${USAGE}`);
        return 2;
      }
      plugins.push(v);
    } else if (a.startsWith("-")) {
      console.error(`nv proxy-validate: unknown argument ${a}\n${USAGE}`);
      return 2;
    } else files.push(a);
  }
  if (files.length === 0) {
    console.error(`nv proxy-validate: name at least one file\n${USAGE}`);
    return 2;
  }
  let bad = 0;
  for (const f of files) {
    const path = resolve(f);
    if (!existsSync(path)) {
      console.error(`nv proxy-validate: no file ${f}`);
      return 2;
    }
    const kind = kindOf(f);
    const v = await validate(kind, readFileSync(path, "utf8"), kind === "caddyfile" ? plugins : []);
    if ("skipped" in v) {
      console.log(`nv proxy-validate: nothing checked: ${v.skipped}`);
      return 0;
    }
    if (v.problems.length) {
      bad++;
      console.error(`nv proxy-validate: ${rel(path)}: ${v.problems.join("\n")}`);
    } else console.log(`${rel(path)}: ok (${kind})`);
  }
  return bad ? 1 : 0;
}
