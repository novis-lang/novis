#!/usr/bin/env bash
# Smoke test of one image built from `docker/Dockerfile`:
#
#     bash tests/docker/smoke.sh <image> [port]
#
# It runs the image the ways `docs/docker.md` tells a user to: each CLI subcommand on `app/`, the
# default command serving `app/main.nvs` under the hardened flags of the guide's Compose file, and
# a `docker stop` while a request is still running, which must finish that request and exit 0. The
# image is built by the caller -- `release.yml` before it pushes, `ci.yml`'s `docker` job on a change
# to the image -- so this file only runs it. Exits non-zero if any check fails.
set -u
image="${1:?usage: smoke.sh <image> [port]}"
port="${2:-18000}"
here="$(cd "$(dirname "$0")" && pwd)"
app="$here/app"
# Git Bash rewrites `/tmp`-like arguments and needs a Windows path for a bind mount; Linux needs
# neither.
if command -v cygpath >/dev/null 2>&1; then app="$(cygpath -w "$app")"; export MSYS_NO_PATHCONV=1; fi
name="novis-smoke-$$"
data="novis-smoke-data-$$"
pass=0
fail=0
check() { if [ "$1" = 0 ]; then echo "PASS  $2"; pass=$((pass + 1)); else echo "FAIL  $2"; fail=$((fail + 1)); fi; }
has() { case "$1" in *"$2"*) return 0 ;; *) return 1 ;; esac; }
cleanup() { docker rm -f "$name" >/dev/null 2>&1; docker volume rm -f "$data" >/dev/null 2>&1; }
trap cleanup EXIT

echo "== $image"
docker image inspect "$image" --format 'user={{.Config.User}} stop={{.Config.StopSignal}} workdir={{.Config.WorkingDir}} cmd={{.Config.Cmd}}'
[ "$(docker image inspect "$image" --format '{{.Config.User}}')" = "nonroot:nonroot" ]; check $? "runs as nonroot"
[ "$(docker image inspect "$image" --format '{{.Config.StopSignal}}')" = "SIGTERM" ]; check $? "stops with SIGTERM"

echo "== the CLI"
out="$(docker run --rm "$image" --version 2>&1)"; has "$out" "nvs "; check $? "--version: $out"
docker run --rm "$image" info >/dev/null 2>&1; check $? "info"
out="$(docker run --rm -v "$app:/app:ro" "$image" run cli.nvs -- a b 2>&1)"
has "$out" "arguments: 2"; check $? "run passes the arguments after --"
has "$out" "zone: ok"; check $? "run finds the zone files"
has "$out" "data folder: ok"; check $? "run writes to the data folder as nonroot"
[ "$fail" = 0 ] || echo "$out"
docker run --rm -v "$app:/app:ro" "$image" test tests/ >/dev/null 2>&1; check $? "test tests/"
docker run --rm -v "$app:/app:ro" "$image" check main.nvs >/dev/null 2>&1; check $? "check main.nvs"

echo "== serve, with the Compose file's flags"
docker run -d --name "$name" -p "127.0.0.1:$port:8000" -v "$app:/app:ro" -v "$data:/usr/local/bin/.nvsdata" \
  --read-only --cap-drop ALL --security-opt no-new-privileges:true "$image" >/dev/null
for _ in $(seq 1 50); do curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null 2>&1 && break; sleep 0.2; done
body="$(curl -fsS "http://127.0.0.1:$port/" 2>&1)"
[ "$body" = "hello from docker" ]; check $? "the default command serves /app/main.nvs: $body"
code="$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$port/healthz")"
[ "$code" = 200 ]; check $? "health_path returns 200 while accepting"

echo "== docker stop while a request runs"
slow="$(mktemp)"
curl -sS "http://127.0.0.1:$port/slow" >"$slow" 2>&1 &
client=$!
sleep 1
docker stop -t 15 "$name" >/dev/null
wait "$client"
[ "$(cat "$slow")" = "slow done" ]; check $? "the running request finished: $(cat "$slow")"
rm -f "$slow"
code="$(docker inspect "$name" --format '{{.State.ExitCode}}')"
[ "$code" = 0 ]; check $? "nvs exited with 0 after SIGTERM"
[ "$fail" = 0 ] || docker logs "$name" 2>&1 | tail -20

echo "== $pass passed, $fail failed"
[ "$fail" = 0 ]
