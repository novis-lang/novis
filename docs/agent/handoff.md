# Handoff

## State

Goal `types-enum-2-2` — types:enum (2/2) — is met: all 17 enums carry a description, three examples
and an attributed test, and the goal's own dossier check has been green since session 0005.

**Three DONE claims before this one each fell to a different `[1 floor]` check, and both of the two
still standing were bugs in `tools/bench.py`.** A full `python tools/loop.py --goal-only` sweep this
session found exactly those two red over 217 checks and nothing else.

`--serve-vs-fpm` died on a traceback: `--reps` grew a `None` default so the warm-start leg could
take its own count, `main` resolves it, and the serve leg still read `args.reps`. It records again
— `nvs serve` 33,559 req/s, 2.48x `php-cgi` at concurrency 1.

The warm-start guard abstained above four times its *budget* — 24 ms of start floor, five times
what a quiet box reads — so it saw the 77.5 ms box session 0006 wrote it for and not the 6.1 ms one
an acceptance sweep leaves behind it. That run read 7.1 ms of work against a 6.0 ms budget thirty
seconds after the sweep finished, where the same binary idle reads 4.5 and 5.2–5.5; both asks were
red, so `COST_SETTLE` called the shadow a regression. The threshold is now a floor *level*,
`QUIET_FLOOR_MS`, because only the floor moves with the machine.

**The headroom is thin and it is a real question, not a flake:** idle work is 5.2–5.5 ms against a
6.0 ms budget written when the floor was 6.4 ms and the work 4.6 ms. A box that reads a quieter
floor than the one the budget was written on should read *less* work, not more.

No Rust changed; one Python file did.

## Next group

**Goal `types-enum-2-2` is met, so the driver's goal switch installs goal `types-exception`'s own
generated handoff over this one.** Its next three exceptions, in the order `TREE` declares them —
one file set: `crates/nvs-hir/src/errors.rs`, `docs/examples/types/<name>/` and
`tests/hostile/types/<name>/`, with `rule:testing/four-proofs` naming what each owes and
`crates/nvs-codegen/tests/arithmetic.rs:40-57` the spelling a `catch` clause and a `->message` read
take.

- [ ] **`Core\Cli\NotInteractive`** — page, three examples, an attack and a `covers:` marker.
      `crates/nvs-hir/src/errors.rs:106`
- [ ] **`Core\Db\DbError`** — the same four; a `RuntimeError` subclass, so a `catch (RuntimeError …)`.
      `crates/nvs-hir/src/errors.rs:107`
- [ ] **`Core\Db\RolledBack`** — the same four, and the neighbour in that file.
      `crates/nvs-hir/src/errors.rs:108`

A prompt reads the controlling terminal and never standard input, and
`crates/nvs-runtime/src/terminal.rs:487`'s `is_interactive` asks the tty profile rather than trying
the open — so an example or a `.nvst` case, which runs with its output piped and its input closed,
takes the `default` or throws `Core\Cli\NotInteractive` without blocking. That is what makes these
three writable as ordinary deterministic proofs.

## Backlog

- The 6 ms warm-start budget has under a millisecond of headroom on this box and the work figure has
  grown against a faster floor — `tools/bench.py`'s `warm_start` owns the reasoning, and finding the
  start-work regression is nobody's slice yet.
- `tools/loop.py`'s `COST_SETTLE` is 30 s and the sweep's shadow outlasts it; the warm-start leg now
  abstains instead, and `benches/abi-probe`'s guards still rely on the settle alone.
