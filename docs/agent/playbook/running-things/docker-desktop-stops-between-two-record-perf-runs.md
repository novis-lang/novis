- **Docker Desktop can stop again a few minutes after `docker desktop start`, so a `--record-perf`
  of a group that needs a service fails as "this host's Docker is not answering".** `Core\Session`
  needs `redis`, and a background script that started it after other groups found the engine gone.
  Start Docker and record that group in one call: `docker desktop start --timeout 300 && docker info
  && bun nv proofs --record-perf --group '<class>'`. See `tools/nv/cmd/proofs.ts`.
