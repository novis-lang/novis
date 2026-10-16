// Bun's side of every case: one process per CPU of the budget, each serving the case's handler on port
// 8080 with `reusePort`, so the kernel spreads connections over them.
//
// The case's `server.ts` is mounted at `/app` and exports `default { fetch }`. `WORKERS` is the budget.

const child = process.env.COMPARE_CHILD !== undefined;

if (child) {
  const app = (await import("/app/server.ts")).default;
  Bun.serve({ hostname: "0.0.0.0", port: 8080, reusePort: true, fetch: app.fetch });
} else {
  const workers = Math.max(1, Number(process.env.WORKERS ?? "1"));
  const procs = Array.from({ length: workers }, () =>
    Bun.spawn(["bun", import.meta.path], { env: { ...process.env, COMPARE_CHILD: "1" }, stdout: "inherit", stderr: "inherit" }),
  );
  // One process ending stops the container, so a crash shows as a failed stack and never as a slower one.
  await Promise.race(procs.map((p) => p.exited));
  for (const p of procs) p.kill();
  process.exit(1);
}
