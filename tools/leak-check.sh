#!/usr/bin/env bash
# `valgrind --leak-check=full` over the fixtures named on the command line — the
# narrow counterpart of `python tools/loop.py --leg-only`'s whole-suite memory leg.
#
# Run one of these for **any** new refcount edge, before it reaches a session's
# last commit: the whole-suite leg only covers `examples/`, and this repository's
# one real leak so far (a refcounted local declared inside a loop body, fixed in
# `mwl_ir::lower::end_iteration`) went unnoticed until a fixture happened to
# declare one.
#
#   wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh target/mine.mwl examples/report.mwl
#
# A `wsl.exe -- bash -lc "…"` one-liner mangles under two layers of shell
# quoting, so this is a file passed by path instead. AGENTS.md says why.
set -u

cd /mnt/<drive>/<repo> || exit 1
# /var/tmp, not /tmp: systemd clears /tmp at every WSL boot, and WSL boots again
# after every idle gap -- a target directory there costs a cold build every run.
export CARGO_TARGET_DIR=/var/tmp/mwl-linux
export PATH="$HOME/.cargo/bin:$PATH"

cargo build --quiet -p mwl-cli || exit 1
BIN=/var/tmp/mwl-linux/debug/mwl

# 97 rather than 1, because the fixture's *own* exit status passes straight
# through valgrind: `examples/uncaught.mwl` ends in an uncaught throw and so
# exits 1 by design, which under `--error-exitcode=1` is indistinguishable from
# a definite leak. 97 is a status no MWL program produces.
VG_ERROR=97

fails=0
for f in "$@"; do
    echo "== $f"
    valgrind --error-exitcode=$VG_ERROR --errors-for-leak-kinds=definite \
        --leak-check=full "$BIN" run "$f" >/tmp/leak-out 2>/tmp/leak-err
    code=$?
    echo "   exit $code"
    if [ "$code" -eq "$VG_ERROR" ]; then
        fails=$((fails + 1))
        grep -E "definitely lost|mwl_stdlib|mwl_ir|mwl_runtime::" /tmp/leak-err | head -12
    fi
done

echo
echo "leak check: $fails failure(s)"
[ "$fails" -eq 0 ]
