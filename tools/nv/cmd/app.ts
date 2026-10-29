// `bun nv app`: serves the applications under `apps/` on this machine, one or several at a time.
//
//     bun nv app                          every application, with the address it listens on
//     bun nv app registry                 serve apps/registry until stopped
//     bun nv app registry telemetry       serve both, side by side, until stopped
//
// An application is a folder under `apps/` with `public/index.nvs`. Each runs as `nvs serve
// public/index.nvs --config nvs.toml` in its own folder, so its relative paths are its own. The address
// is its `nvs.toml`'s `[server] listen`, the one home of it: the registry's is `127.0.0.1:8140` and the
// telemetry collector's `127.0.0.1:8141`, so the two run at the same time. An application with no
// `listen` gets `nvs serve`'s default, `127.0.0.1:8000`.
//
// The `nvs` is the newest built one (`lib/nvs-binary.ts`), or `NVS_BIN` when it is set. A server waits
// on purpose, so its run has no hang watch and no timeout; Ctrl+C stops it.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { newestNvs } from "../lib/nvs-binary.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough } from "../lib/proc.ts";

export const summary = "serve an application under apps/, such as registry or telemetry: nv app [NAME...]";

const USAGE = "usage: bun nv app [NAME...]";

const APPS = join(ROOT, "apps");

/** The longest timeout `setTimeout` takes, about 24 days: a server runs until it is stopped. */
const FOREVER_MS = 2 ** 31 - 1;

/** The name of every folder under `apps/` that has `public/index.nvs`, sorted. */
export function apps(dir = APPS): string[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isDirectory() && existsSync(join(dir, e.name, "public", "index.nvs")))
    .map((e) => e.name)
    .sort();
}

/** The addresses `nvs.toml`'s `[server] listen` names, or `nvs serve`'s default when it names none. */
export function listens(toml: string): string[] {
  const server = (Bun.TOML.parse(toml) as { server?: { listen?: string[] } }).server;
  return server?.listen ?? ["127.0.0.1:8000"];
}

function listensOf(name: string): string[] {
  const path = join(APPS, name, "nvs.toml");
  return listens(existsSync(path) ? readFileSync(path, "utf8") : "");
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("-h") || args.includes("--help")) {
    console.log(`${summary}\n${USAGE}`);
    return 0;
  }
  const known = apps();
  if (args.length === 0) {
    for (const name of known) console.log(`  ${name.padEnd(12)} http://${listensOf(name)[0]}/`);
    return 0;
  }
  const unknown = args.filter((a) => !known.includes(a));
  if (unknown.length > 0) {
    console.error(`nv app: no application ${unknown.map((a) => `\`${a}\``).join(", ")} under apps/; there are ${known.join(", ")}`);
    return 2;
  }
  const nvs = process.env.NVS_BIN || newestNvs() || "nvs";
  const runs = args.map((name) => {
    console.log(`nv app: ${name} listens on http://${listensOf(name)[0]}/`);
    return passthrough([nvs, "serve", "public/index.nvs", "--config", "nvs.toml"], {
      cwd: join(APPS, name),
      timeoutMs: FOREVER_MS,
      hang: false,
    });
  });
  const codes = await Promise.all(runs);
  return Math.max(...codes);
}
