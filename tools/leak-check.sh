#!/usr/bin/env bash
# `valgrind --leak-check=full` over the fixtures named on the command line — the
# narrow counterpart of `python tools/loop.py --leg-only`'s whole-suite memory leg.
#
# Run one of these for **any** new refcount edge, before it reaches a session's
# last commit: the whole-suite leg only covers `examples/`, and this repository's
# one real leak so far (a refcounted local declared inside a loop body, fixed in
# `nvs_ir::lower::end_iteration`) went unnoticed until a fixture happened to
# declare one.
#
#   wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh target/mine.nvs examples/report.nvs
#   wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh --test target/tests.nvs
#
# `--test` runs the fixtures through `nvs test` instead of `nvs run`, which is a
# different program: the entry file's own statements do not run and the
# `#[Test]` methods and ADR 0079 § 8's fixtures do, so a refcount edge on that
# path is unreachable from `run` at all.
#
# A `wsl.exe -- bash -lc "…"` one-liner mangles under two layers of shell
# quoting, so this is a file passed by path instead. AGENTS.md says why.
set -u

cd /mnt/<drive>/<repo> || exit 1
# /var/tmp, not /tmp: systemd clears /tmp at every WSL boot, and WSL boots again
# after every idle gap -- a target directory there costs a cold build every run.
export CARGO_TARGET_DIR=/var/tmp/nvs-linux
export PATH="$HOME/.cargo/bin:$PATH"

cargo build --quiet -p nvs-cli || exit 1
BIN=/var/tmp/nvs-linux/debug/nvs

# 97 rather than 1, because the fixture's *own* exit status passes straight
# through valgrind: `examples/uncaught.nvs` ends in an uncaught throw and so
# exits 1 by design, which under `--error-exitcode=1` is indistinguishable from
# a definite leak. 97 is a status no Novis program produces.
VG_ERROR=97

SUB=run
if [ "${1:-}" = "--test" ]; then
    SUB=test
    shift
fi

fails=0
for f in "$@"; do
    echo "== $f"
    valgrind --error-exitcode=$VG_ERROR --errors-for-leak-kinds=definite \
        --leak-check=full "$BIN" "$SUB" "$f" >/tmp/leak-out 2>/tmp/leak-err
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
