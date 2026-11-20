// Node.js's side of every case: a `node:cluster` primary that forks one worker per CPU of the budget,
// each serving the case's handler with `node:http` on port 8080. The primary hands the connections out.
//
// The case's `server.mjs` is in `APP_DIR`, which is `/app` unless a row serves a link in it, and exports
// a request listener as its default. `WORKERS` is the budget.

import cluster from "node:cluster";
import { createServer } from "node:http";

if (cluster.isPrimary) {
  const workers = Math.max(1, Number(process.env.WORKERS ?? "1"));
  for (let i = 0; i < workers; i++) cluster.fork();
  // One worker ending stops the container, and its restart policy starts it again, as `bun/serve.ts`
  // does. A crash then shows in the container's restart count, which the feature rows read.
  cluster.once("exit", () => {
    for (const w of Object.values(cluster.workers ?? {})) w?.kill();
    process.exit(1);
  });
} else {
  const handler = (await import(`${process.env.APP_DIR ?? "/app"}/server.mjs`)).default;
  const server = createServer(handler);
  // Longer than nginx's upstream keep-alive, so nginx never reuses a connection Node has just closed.
  // Node's documentation gives this advice for a server behind a load balancer.
  server.keepAliveTimeout = 65_000;
  server.headersTimeout = 66_000;
  server.listen(8080, "0.0.0.0");
}
