- **HTTP Garden's harness cannot run from Windows or WSL here: it reaches origins by container IP, and
  WSL's `docker` has no daemon.** Docker Desktop also refuses to bind-mount from drive E:. Bake the
  Garden's `tools/` into a `python:3.13-slim` image and run it on `http-garden_default` with
  `--label com.docker.compose.service=runner` and the Docker socket mounted, beside the WSL leg
  `docs/agent/commands.md` describes. `until: gone docs/agent/commands.md:use WSL`
