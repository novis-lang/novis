#!/usr/bin/env bash
# The Linux leg of `docs/agent/loop-goal.md`'s acceptance list, run by hand from a
# session that has just made the Windows leg green.
#
# `docs/agent/loop-goal.toml` is the authority on what must pass — `python
# tools/loop.py --goal-only` runs all of it, both legs included, and this script
# is the hand-run shortcut for just the Linux half from inside WSL. It runs the
# same `mwl run` commands against a Linux build, then the same
# valgrind sweep, so a calling-convention divergence in the JIT or a leak in the
# refcount protocol shows up where it hides — which is the whole reason the
# second leg exists (see `docs/agent/loop-goal.md`).
#
# A `wsl.exe -- bash -lc "…"` one-liner mangles under two layers of shell
# quoting, so this is a file passed by path instead. AGENTS.md says why.
set -u

cd /mnt/<drive>/<repo> || exit 1
export CARGO_TARGET_DIR=/tmp/mwl-linux
export PATH="$HOME/.cargo/bin:$PATH"

fails=0
mwl() { cargo run --quiet -p mwl-cli -- run "$@"; }

# `$(…)` strips trailing newlines from both sides, so a multi-line $want literal
# compares exactly against multi-line output with no normalization needed.
expect_stdout() {
    local want="$1"; shift
    local got code
    got="$(mwl "$@" 2>/tmp/mwl-err)"
    code=$?
    if [ "$code" -ne 0 ]; then
        echo "FAIL $* : exit $code"
        head -3 /tmp/mwl-err
        fails=$((fails + 1))
        return
    fi
    if [ "$got" != "$want" ]; then
        echo "FAIL $* : stdout was"
        printf '%s\n' "$got" | sed 's/^/       got  /'
        printf '%s\n' "$want" | sed 's/^/       want /'
        fails=$((fails + 1))
        return
    fi
    echo "ok   $*"
}

# ---- stage 1: the object and array gate -------------------------------------

expect_stdout 'Hello, World!' examples/hello.mwl
expect_stdout 'quadruple(5) = 20' examples/calls.mwl
expect_stdout 'sum = 998000' examples/arith.mwl
expect_stdout 'caught: boom' examples/throw.mwl

expect_stdout 'cat has 4 legs
rex has 4 legs, and barks
Hello, rex!
dog is an animal
dog greets
leaf
leaf is a mid' examples/objects.mwl

expect_stdout 'alpha=1;beta=2;gamma=3;
alpha=1;gamma=3;
alpha=1;gamma=3;beta=20;
alpha=1;gamma=3;beta=20;
alpha=99;gamma=3;beta=20;
count=3
total=10' examples/arrays.mwl

expect_stdout 'checked host
value-of-host
checked bad
config: bad key
checked missing
logic: no such key: missing
backtrace present' examples/errors.mwl

expect_stdout '6
20
Counter(10)
n=6' examples/hooks.mwl

expect_stdout 'ana outranks bo
gold=2
ana,clone' examples/enums.mwl

# trace.mwl: `#0 Deep::inner()` then `#1 Deep::outer()`, in that order.
out="$(mwl examples/trace.mwl)"
if printf '%s' "$out" | grep -q '^#0 Deep::inner()' &&
   printf '%s' "$out" | grep -q '^#1 Deep::outer()'; then
    echo "ok   examples/trace.mwl"
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

# ---- stage 2: the rest of M4's language surface -----------------------------

expect_stdout 'generator=15
iterable=10' examples/iterate.mwl

# ---- stage 3: Core Part I is real -------------------------------------------

expect_stdout 'evens=5
PEAR|APPLE|FIG|BANANA
fig,pear,Apple,banana
a+b+c
007
Mwl runs' examples/core.mwl

expect_stdout 'the...3
quick.1
brown.1
fox...2
lazy..1
dog...1
distinct=6' examples/report.mwl

# ---- stage 4: the two suites ------------------------------------------------

for suite in conformance differential; do
    if cargo run --quiet -p mwl-cli -- test "tests/$suite/" > "/tmp/mwl-$suite" 2>&1; then
        echo "ok   mwl test tests/$suite/ -- $(grep -o '[0-9]* passed, [0-9]* failed' "/tmp/mwl-$suite" | tail -1)"
    else
        echo "FAIL mwl test tests/$suite/"
        tail -5 "/tmp/mwl-$suite"
        fails=$((fails + 1))
    fi
done

# ---- stage 6: memory --------------------------------------------------------
#
# Cycles are out of scope by decision (see loop-goal.md), so only *definite*
# losses fail. fatal.mwl and uncaught.mwl are skipped: both exit non-zero by
# design, which valgrind's --error-exitcode cannot be distinguished from.

cargo build --quiet -p mwl-cli || { echo "FAIL valgrind: linux build failed"; exit 1; }

for f in examples/hello.mwl examples/calls.mwl examples/arith.mwl examples/throw.mwl \
         examples/trace.mwl examples/objects.mwl examples/arrays.mwl examples/errors.mwl \
         examples/hooks.mwl examples/enums.mwl examples/iterate.mwl examples/core.mwl \
         examples/report.mwl; do
    if valgrind --error-exitcode=1 --leak-check=full --errors-for-leak-kinds=definite -q \
                "$CARGO_TARGET_DIR/debug/mwl" run "$f" > /dev/null 2>/tmp/mwl-vg; then
        echo "ok   valgrind $f"
    else
        echo "FAIL valgrind $f"
        head -12 /tmp/mwl-vg
        fails=$((fails + 1))
    fi
done

echo
if [ "$fails" -eq 0 ]; then
    echo "Linux leg: everything passes"
else
    echo "Linux leg: $fails failure(s)"
fi
exit "$fails"
