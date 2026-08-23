#!/usr/bin/env bash
# `valgrind --leak-check=full` over the fixtures named on the command line — the
# narrow counterpart of `.claude/wsl-acceptance.sh`'s whole-suite memory leg.
#
# Run one of these for **any** new refcount edge, before it reaches a session's
# last commit: the whole-suite leg only covers `examples/`, and this repository's
# one real leak so far (a refcounted local declared inside a loop body, fixed in
# `mwl_ir::lower::end_iteration`) went unnoticed until a fixture happened to
# declare one.
#
#   wsl.exe -- bash /mnt/<drive>/<repo>/.claude/leak-check.sh target/mine.mwl examples/report.mwl
#
# A `wsl.exe -- bash -lc "…"` one-liner mangles under two layers of shell
# quoting, so this is a file passed by path instead. CLAUDE.md says why.
set -u

cd /mnt/<drive>/<repo> || exit 1
export CARGO_TARGET_DIR=/tmp/mwl-linux
export PATH="$HOME/.cargo/bin:$PATH"

cargo build --quiet -p mwl-cli || exit 1
BIN=/tmp/mwl-linux/debug/mwl

fails=0
for f in "$@"; do
    echo "== $f"
    valgrind --error-exitcode=1 --errors-for-leak-kinds=definite \
        --leak-check=full "$BIN" run "$f" >/tmp/leak-out 2>/tmp/leak-err
    code=$?
    echo "   exit $code"
    if [ "$code" -ne 0 ]; then
        fails=$((fails + 1))
        grep -E "definitely lost|mwl_stdlib|mwl_ir|mwl_runtime::" /tmp/leak-err | head -12
    fi
done

echo
echo "leak check: $fails failure(s)"
[ "$fails" -eq 0 ]
