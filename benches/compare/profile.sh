#!/bin/bash
# The profile `bun nv bench-compare --profile <case>` takes, run inside a privileged container of the
# Novis bench image. The tool's header says how it is started; this file is what runs inside.
#
#   profile.sh CASE METHOD PATH [BODY-FILE]
#
# `/cases` is `benches/compare/cases`, read-only. `/profile` holds `oha`, copied out of its pinned
# image, and the request body when there is one. It exits 3 when `perf` cannot record here.
set -euo pipefail

case_id=$1 method=$2 path=$3 body=${4:-}
port=18080
url="http://127.0.0.1:$port$path"

echo "== installing linux-perf and strace"
apt-get update -qq >/dev/null
apt-get install -y -qq --no-install-recommends linux-perf strace >/dev/null 2>&1

# One core serves and eight others load, as the hand-taken profile did, so the server's core is its own.
cores=$(nproc)
if [ "$cores" -ge 16 ]; then serve_core=3 load_cores=8-15
elif [ "$cores" -ge 4 ]; then serve_core=1 load_cores="2-$((cores - 1))"
else echo "profile: $cores cpu(s) visible, and the profile needs 4 to keep the server's core its own" >&2; exit 3
fi

oha() { taskset -c "$load_cores" /profile/oha --no-tui "$@"; }
load_flags=(-m "$method")
[ -n "$body" ] && load_flags+=(-T application/json -D "/profile/$body")

# The server gets a cgroup of its own, so `memory.peak` is its alone. A cgroup that has processes cannot
# give its children a controller, so every process here moves to `init` first.
cg=/sys/fs/cgroup
mkdir -p "$cg/init" "$cg/serve"
for p in $(cat "$cg/cgroup.procs"); do echo "$p" >"$cg/init/cgroup.procs" 2>/dev/null || true; done
echo +memory >"$cg/cgroup.subtree_control"

mkdir -m 700 /root/data
# The server's log goes to a file: a line it writes to the terminal would be a `write` in the counts.
bash -c "echo \$\$ >$cg/serve/cgroup.procs && exec taskset -c $serve_core nvs serve /cases/$case_id/main.nvs --listen 127.0.0.1:$port --no-init --data /root/data" >/tmp/serve.log 2>&1 &
pid=$!
for _ in $(seq 100); do
  (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null && break
  sleep 0.1
done
idle=$(cat "$cg/serve/memory.current")
echo "== serving /cases/$case_id/main.nvs as pid $pid on core $serve_core, loaded from cores $load_cores"

if [ "$case_id" = wait ]; then
  # Each pass holds 1000 requests in flight. The second pass shows whether the server's pools keep growing.
  for pass in 1 2; do
    oha -z 30s -c 1000 "${load_flags[@]}" "$url" >/dev/null
    peak=$(cat "$cg/serve/memory.peak")
    echo "memory pass $pass: idle $((idle / 1024)) KiB, peak $((peak / 1024)) KiB, $(((peak - idle) / 1000)) bytes per request in flight"
  done
fi

oha -n 2000 -c 64 "${load_flags[@]}" "$url" >/dev/null

echo "== 10 s under -c 64, with nothing attached"
oha -z 10s -c 64 "${load_flags[@]}" "$url" |
  awk '/Requests\/sec:/ { printf "%.0f requests/sec on one core, %.1f us per request\n", $2, 1e6 / $2 }'

echo "== perf record -F 1999 for 15 s under -c 64"
oha -z 20s -c 64 "${load_flags[@]}" "$url" >/dev/null &
load=$!
sleep 2
if ! perf record -q -F 1999 -p "$pid" -o /tmp/perf.data -- sleep 15 >/dev/null 2>/tmp/perf.err; then
  cat /tmp/perf.err >&2
  exit 3
fi
wait "$load"
perf report -i /tmp/perf.data --no-children --sort symbol --stdio --percent-limit 0.5 2>/dev/null |
  grep -v '^#' | grep -v '^$' | sed -E 's/[[:space:]]+-[[:space:]]+-[[:space:]]*$//' | awk 'NR <= 40'

# The same load with call graphs, so a row is the time spent in a function and everything it called. A
# cost spread over many small symbols, such as an isolate's start and finish, shows up whole only here.
# The release build has no frame pointers, so perf unwinds from a copy of each sample's stack, 32 KiB
# deep: a request's frames sit within that of its task's entry. `--no-inline` keeps perf from asking
# `addr2line` about every inlined frame, which fails on this build. Each row is the time with callees,
# then the time in the function alone.
echo "== perf record --call-graph dwarf -F 999 for 10 s under -c 64, each function with its callees"
oha -z 15s -c 64 "${load_flags[@]}" "$url" >/dev/null &
load=$!
sleep 2
perf record -q -F 999 --call-graph dwarf,32768 -p "$pid" -o /tmp/perf-graph.data -- sleep 10 >/dev/null 2>/tmp/perf.err ||
  cat /tmp/perf.err >&2
wait "$load"
perf report -i /tmp/perf-graph.data --children --no-inline --sort symbol --stdio -g none --percent-limit 0.3 2>/dev/null |
  grep -v '^#' | grep -v '^$' | sed -E 's/[[:space:]]+-[[:space:]]+-?[[:space:]]*$//' | cut -c1-200 | awk 'NR <= 120' || true

# `strace -f` stops the serving thread at every call. Under 64 connections that stall is long enough
# for the watchdog to shed requests with 503, and a shed request is not the request being profiled.
echo "== strace -c -f over 20000 requests under -c 8"
before=$(grep -c '"status":5' /tmp/serve.log || true)
strace -c -f -p "$pid" -o /tmp/strace.txt &
tracer=$!
sleep 1
oha -n 20000 -c 8 "${load_flags[@]}" "$url" >/dev/null
kill -INT "$tracer"
wait "$tracer" || true
# Every thread is counted, so the watchdog's and the timers' calls are in the table too. Their count
# does not grow with the requests, and the per-request column shows which calls do.
awk '
  $1 ~ /^[0-9.]+$/ && NF >= 5 {
    errors = NF == 6 ? $5 : 0
    printf "%-16s %9d calls %9d errors   %6.2f per request\n", $NF, $4, errors, $4 / 20000
  }' /tmp/strace.txt
failed=$(($(grep -c '"status":5' /tmp/serve.log || true) - before))
echo "responses with a 5xx status under strace: $failed of 20000"

kill "$pid"
