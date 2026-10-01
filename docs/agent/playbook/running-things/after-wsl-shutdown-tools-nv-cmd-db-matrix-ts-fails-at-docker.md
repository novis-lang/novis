- **After `wsl --shutdown`, `tools/nv/cmd/db-matrix.ts` fails at `docker compose up` with `500 Internal
  Server Error` on `dockerDesktopLinuxEngine/_ping`, and its message says only that the daemon may be
  unreachable.** Docker Desktop's processes stay up while its engine sits at `stopping`, so nothing
  restarts it on its own. Run `docker desktop status`, and if it says `stopping`, `docker desktop restart`
  brings the engine back in under a minute. [until: gone tools/nv/cmd/db-matrix.ts:unreachable]
