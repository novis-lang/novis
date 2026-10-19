// A stand-in for looper's service, for the tests of `project add` and `init`. `standInService(home)`
// starts it as `bun service-stand-in.ts <home>`. It serves the project endpoints on `127.0.0.1` and
// writes its port into `<home>/looper.port`, as `looper serve` does. It runs until it is killed.
//
// - `GET /api/projects` gives every project registered, in their order.
// - `POST /api/projects` with `{"root", "name"}` registers one and gives it as `{"id", "name", "root"}`.
//   A name taken already is refused with 400 and `{"error"}`, as looper's service refuses it.
//
// It holds the projects in memory. Its bound is its life: one test, which registers a handful.

import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { PORT_FILE } from "../jobs/slots.ts";

/** The longest the stand-in may take to write its port. */
const START_MS = 20_000;

/** A running stand-in: its port, and how to stop it. */
export interface StandIn {
  port: number;
  /** The projects it holds, as `GET /api/projects` gives them. */
  projects(): Promise<{ id: number; name: string; root: string }[]>;
  stop(): void;
}

/** Starts a stand-in service whose home is `home`, and waits for its port. */
export async function standInService(home: string): Promise<StandIn> {
  const child = Bun.spawn(["bun", import.meta.path, home], { stdin: "ignore", stdout: "ignore", stderr: "inherit" });
  const file = join(home, PORT_FILE);
  const start = Date.now();
  while (!existsSync(file)) {
    if (Date.now() - start > START_MS) {
      child.kill();
      throw new Error(`the stand-in service wrote no port in ${START_MS} ms`);
    }
    await Bun.sleep(50);
  }
  const port = Number(readFileSync(file, "utf8").trim());
  return {
    port,
    projects: async () => (await (await fetch(`http://127.0.0.1:${port}/api/projects`)).json()) as never,
    stop: () => child.kill(),
  };
}

function serve(home: string): void {
  const projects: { id: number; name: string; root: string }[] = [];
  const server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    fetch: async (req) => {
      const path = new URL(req.url).pathname;
      if (path !== "/api/projects") return Response.json({ error: `no route ${path}` }, { status: 404 });
      if (req.method === "GET") return Response.json(projects);
      const { root, name } = (await req.json()) as { root: string; name: string };
      if (projects.some((p) => p.name === name)) return Response.json({ error: `a project is named ${name} already` }, { status: 400 });
      const project = { id: projects.length + 1, name, root };
      projects.push(project);
      return Response.json(project);
    },
  });
  mkdirSync(home, { recursive: true });
  writeFileSync(`${join(home, PORT_FILE)}.tmp`, `${server.port}\n`);
  renameSync(`${join(home, PORT_FILE)}.tmp`, join(home, PORT_FILE));
}

if (import.meta.main) {
  const home = process.argv[2];
  if (home === undefined) throw new Error("usage: bun service-stand-in.ts <home>");
  serve(home);
}
