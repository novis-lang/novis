#!/usr/bin/env bash
# `valgrind --leak-check=full` over the fixtures named on the command line — the
# narrow counterpart of the valgrind sweep `bun nv loop` runs over every fixture.
#
# Run one of these for **any** new refcount edge, before it reaches a session's
# last commit: the whole-suite leg only covers `examples/`, and this repository's
# one real leak so far (a refcounted local declared inside a loop body, fixed in
# `nvs_ir::lower::end_iteration`) went unnoticed until a fixture happened to
# declare one.
#
#   wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh target/mine.nvs examples/report.nvs
#   wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh --test target/tests.nvs
#   wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh --request examples/json-body.nvsr examples/json-body.nvs
#
# `--test` runs the fixtures through `nvs test` instead of `nvs run`, which is a
# different program: the entry file's own statements do not run and the
# `#[Test]` methods and ADR 0079 § 8's fixtures do, so a refcount edge on that
# path is unreachable from `run` at all.
#
# `--request` answers the request that file describes, for the same reason: a
# body reader's edges — the octets a request holds and the document
# `Core\Request::json` leaves beside them — are unreachable without one, because
# every `Core\Request` member refuses outright where no request arrived. The
# whole-suite sweep in `loop.py` runs a fixture with no arguments at all, so
# this is the only way an edge that only a served request opens gets looked at.
#
# A `wsl.exe -- bash -lc "…"` one-liner mangles under two layers of shell
# quoting, so this is a file passed by path instead. AGENTS.md says why.
set -u

# The repo root, derived from this script's own path rather than named: the checkout is wherever the
# machine put it, and docs/setup.md promises nothing here hardcodes one.
cd "$(dirname "$0")/.." || exit 1
# /var/tmp, not /tmp: systemd clears /tmp at every WSL boot, and WSL boots again
# after every idle gap -- a target directory there costs a cold build every run.
export CARGO_TARGET_DIR=/var/tmp/nvs-linux
export PATH="$HOME/.cargo/bin:$PATH"
. tools/target-checkout.sh

cargo build --quiet -p nvs-cli || exit 1
BIN=/var/tmp/nvs-linux/debug/nvs

# 97 rather than 1, because the fixture's *own* exit status passes straight
# through valgrind: `examples/uncaught.nvs` ends in an uncaught throw and so
# exits 1 by design, which under `--error-exitcode=1` is indistinguishable from
# a definite leak. 97 is a status no Novis program produces.
VG_ERROR=97

SUB=run
REQUEST=()
if [ "${1:-}" = "--test" ]; then
    SUB=test
    shift
fi
if [ "${1:-}" = "--request" ]; then
    REQUEST=(--request "${2:?--request names the file describing the request}")
    shift 2
fi

fails=0
for f in "$@"; do
    echo "== $f"
    valgrind --error-exitcode=$VG_ERROR --errors-for-leak-kinds=definite \
        --suppressions=tools/valgrind.supp \
        --leak-check=full "$BIN" "$SUB" ${REQUEST[@]+"${REQUEST[@]}"} "$f" \
        >/tmp/leak-out 2>/tmp/leak-err
    code=$?
    echo "   exit $code"
    if [ "$code" -eq "$VG_ERROR" ]; then
        fails=$((fails + 1))
        grep -E "definitely lost|nvs_stdlib|nvs_ir|nvs_runtime::" /tmp/leak-err | head -12
    fi
done

echo
echo "leak check: $fails failure(s)"
[ "$fails" -eq 0 ]
