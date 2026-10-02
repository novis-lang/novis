#!/usr/bin/env bash
# ThreadSanitizer over the two crates that hold the concurrency — `nvs-host`,
# which owns every thread this runtime starts, and `nvs-runtime`, which owns the
# state a task reaches across one. This is the whole of the `tsan:` job in
# `.github/workflows/ci.yml`: the job runs this file, so what CI checks and what
# a contributor can run are the same command rather than two that drift.
#
#   wsl.exe -- bash tools/tsan.sh
#
# It prints `tsan: clean` and nothing else on success. A race is a report on
# stderr and a non-zero status, because the sanitizer's own exit code passes
# through `cargo test`.
#
# What is instrumented, and why each flag is here:
#
#   `-Zbuild-std` rebuilds `std` under the sanitizer too. Without it every
#   `Mutex`, `Condvar` and channel in `std` is a futex call TSAN cannot see
#   through, and the run reports a race at the first thing a worker thread hands
#   to another — false positives that no amount of annotation in this tree would
#   quiet, because the missing happens-before edge is inside `std`.
#
#   `--target` is load-bearing for the same reason it is in the ASAN job: build
#   scripts and proc macros run on the *host*, and instrumenting them means the
#   build dies before a test starts.
#
#   `--features nvs-host/tsan` compiles the fiber annotations in. They are the
#   whole reason this leg can be clean at all — a stackful coroutine moves a
#   thread onto a stack the sanitizer has not seen it on — and they call symbols
#   only the sanitizer runtime exports, so nothing but this script turns them on.
#
#   `--features nvs-runtime/sanitizer` swaps `counting_alloc`'s size-class cache
#   back out for the platform heap. That cache never returns a block, so the
#   sanitizer's shadow for a recycled allocation is never reset and two tasks
#   reusing one look like two threads sharing it. `counting_alloc`'s module doc
#   is the home of that reasoning; the ASAN job carries the same flag for the
#   neighbouring reason.
#
#   `--tests`, not the default target set: a doctest binary is linked under
#   `RUSTDOCFLAGS` and so resolves no `__tsan_*` symbol against an instrumented
#   rlib, which is a link error on every doctest whatever the code under it does.
#
# `crates/nvs-host/src/tsan.rs` is the other half of this leg and owns why a
# stack switch has to be announced at all.
set -u

# The repo root, derived from this script's own path rather than named: the checkout is wherever the
# machine put it, and docs/setup.md promises nothing here hardcodes one.
cd "$(dirname "$0")/.." || exit 1
# Its own target directory, and not the one `tools/leak-check.sh` uses: `RUSTFLAGS` is part of a
# fingerprint, so sharing one would rebuild the whole tree on every alternation between the two legs.
# /var/tmp rather than /tmp for the reason that script gives — systemd clears /tmp at every WSL boot.
# Overridable because CI has the opposite requirement: a runner's cache action
# only sees a directory inside the checkout, and a fresh runner has nothing in
# /var/tmp to keep warm anyway.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/tmp/nvs-tsan}"
export PATH="$HOME/.cargo/bin:$PATH"
. tools/target-checkout.sh

# `halt_on_error=0` so one race does not hide the next: the run still exits
# non-zero, and a leg that reports everything it found is worth more than one
# that stops at the first. `second_deadlock_stack` makes a lock-order report name
# both acquisitions rather than one.
export TSAN_OPTIONS="halt_on_error=0:second_deadlock_stack=1"
export RUSTFLAGS="-Zsanitizer=thread"

cargo +nightly test \
    -Zbuild-std \
    --target x86_64-unknown-linux-gnu \
    --features nvs-host/tsan,nvs-runtime/sanitizer \
    --tests \
    -p nvs-host \
    -p nvs-runtime || exit 1

echo "tsan: clean"
