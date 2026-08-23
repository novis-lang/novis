#!/usr/bin/env bash
# The WSL leg of `.claude/loop-goal.md`'s acceptance list, run by hand from a
# session that has just made the Windows leg green.
#
# `Test-GoalLeg` in `.claude/loop.ps1` is the authority on what must pass; this
# script runs the same eight `mwl run` commands against a Linux build so a
# calling-convention divergence in the JIT shows up where it hides — which is
# the whole reason the second leg exists (see `.claude/loop-goal.md`).
#
# A `wsl.exe -- bash -lc "…"` one-liner mangles under two layers of shell
# quoting, so this is a file passed by path instead. CLAUDE.md says why.
set -u

cd /mnt/<drive>/<repo> || exit 1
export CARGO_TARGET_DIR=/tmp/mwl-linux
export PATH="$HOME/.cargo/bin:$PATH"

fails=0
mwl() { cargo run --quiet -p mwl-cli -- run "$@"; }

expect_stdout() {
    local want="$1"; shift
    local got
    got="$(mwl "$@" 2>/tmp/mwl-err)"
    local code=$?
    if [ "$code" -ne 0 ]; then
        echo "FAIL $* : exit $code"
        head -3 /tmp/mwl-err
        fails=$((fails + 1))
        return
    fi
    if [ "$got" != "$want" ]; then
        echo "FAIL $* : stdout '$got', wanted '$want'"
        fails=$((fails + 1))
        return
    fi
    echo "ok   $*"
}

expect_stdout 'Hello, World!' examples/hello.mwl
expect_stdout 'quadruple(5) = 20' examples/calls.mwl
expect_stdout 'caught: boom' examples/throw.mwl
expect_stdout 'sum = 998000' examples/arith.mwl

# trace.mwl: `#0 Deep::inner()` then `#1 Deep::outer()`, in that order.
out="$(mwl examples/trace.mwl)"
if printf '%s' "$out" | grep -q '^#0 Deep::inner()' &&
   printf '%s' "$out" | grep -q '^#1 Deep::outer()'; then
    echo "ok   examples/trace.mwl"
    printf '%s\n' "$out" | sed 's/^/       /'
else
    echo "FAIL examples/trace.mwl: $out"
    fails=$((fails + 1))
fi

# uncaught.mwl: non-zero exit, and an MWL backtrace on stderr.
err="$(mwl examples/uncaught.mwl 2>&1 >/dev/null)"
code=$?
if [ "$code" -eq 0 ]; then
    echo "FAIL examples/uncaught.mwl: exited 0"
    fails=$((fails + 1))
elif printf '%s' "$err" | grep -q 'Uncaught Exception: unhandled' &&
     printf '%s' "$err" | grep -q '#0 Boom::inner()' &&
     printf '%s' "$err" | grep -q '#1 Boom::outer()'; then
    echo "ok   examples/uncaught.mwl"
    printf '%s\n' "$err" | sed 's/^/       /'
else
    echo "FAIL examples/uncaught.mwl: $err"
    fails=$((fails + 1))
fi

# fatal.mwl: FATAL on stderr, non-zero exit, and the output written before the
# panic survives it.
out="$(mwl --fault-inject=helper-panic examples/fatal.mwl 2>/tmp/mwl-err)"
code=$?
err="$(cat /tmp/mwl-err)"
if [ "$code" -ne 0 ] &&
   printf '%s' "$err" | grep -q 'FATAL' &&
   printf '%s' "$out" | grep -q 'start'; then
    echo "ok   examples/fatal.mwl --fault-inject=helper-panic"
else
    echo "FAIL examples/fatal.mwl: exit $code, out '$out'"
    fails=$((fails + 1))
fi

# --dump-asm prints generated code instead of running it.
bytes="$(mwl --dump-asm examples/arith.mwl | wc -c)"
if [ "$bytes" -ge 200 ]; then
    echo "ok   --dump-asm examples/arith.mwl ($bytes bytes)"
else
    echo "FAIL --dump-asm examples/arith.mwl: only $bytes bytes"
    fails=$((fails + 1))
fi

echo
if [ "$fails" -eq 0 ]; then
    echo "WSL leg: all eight commands pass"
else
    echo "WSL leg: $fails failure(s)"
fi
exit "$fails"
