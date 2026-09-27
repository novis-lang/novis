# The per-feature bench tree — one measured record each

Every feature Novis ships owes a measured performance record
([ADR 0134](../../docs/decisions/0134.md)), and this is where the programs
that produce it live. **Every number here is Novis against itself.** There is no PHP column and
there never will be — [`benches/userland/`](../userland/README.md) owns the cross-engine comparison,
and [`benches/abi-probe/`](../abi-probe/) owns the guards that fail a build. This tree exists to
answer two questions: *we changed something — what did it cost?*, and *is this member doing more
than it should?* — the second on the very first run, before there is anything to compare against.

`bun nv proofs --record-perf` measures them and appends to
[`docs/perf/members.ndjson`](../../docs/perf/members.ndjson); `--perf-report` renders
[`docs/perf/members.md`](../../docs/perf/members.md) from it. This file owns what a bench **is**.

## Where a bench goes

`benches/members/<the feature's path>.nvs` — one file, not a directory, because a feature has one
figure: `benches/members/core/Str/length.nvs`. `bun nv proofs --id '<feature>'` prints the path.

## What a bench is

```nvs
<?nvs
// Counts the characters of a short label, as a program does before it shortens or pads one.
// bench: iterations 400000
// bench: allocations 0
// bench: complexity constant

class Bench {
    public static function run(uint $rounds): uint {
        array<string> $labels = ["order", "customer", "shipping address", "ünïcödé"];
        uint $total = 0;
        uint $i = 0;
        while ($i < $rounds) {
            $total = $total + Core\Str::length($labels[($total % 4) as int]);
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(400000), "\n";
```

The rules, of which the chained input and the `int` subscript are the ones that are not obvious:

- **The top comment is one plain sentence**: what is measured, and where somebody meets it in real
  code. It is read by people looking the feature up, so it follows
  [`AGENTS.md`](../../AGENTS.md) § *Text an end user reads* — no
  optimiser, no lowering, no allocator in it. Why the loop is shaped the way it is belongs here in
  this file, not in the bench.
- **`// bench: iterations N` is required**, and N is the number of times the measured operation
  happens. It is what turns a wall-clock reading into a per-operation figure, and a bench without it
  is refused rather than guessed at.
- **Measure the feature and as little else.** The loop, the counter and the array read are charged
  to every bench equally and the calibration below subtracts the constant part; anything *else* in
  the loop is being measured too.
- **Chain the inputs.** Iteration N's input must depend on iteration N−1's result — above, the
  subscript is `$total % 4`. A loop whose input never changes is a loop an optimiser may delete, and
  a bench that measures a deleted loop reads as a triumph. This is `benches/userland/README.md`'s
  rule for the same reason, and the symptom is the same: a figure indistinguishable from the empty
  program's.
- **Index with an `int`.** A `uint` subscript renders a decimal string key, on purpose — a `uint`
  past `i64::MAX` has no `i64` spelling naming the same element (`nvs_ir::lower::expr`'s
  `lower_array_key`) — and that render is two allocations per read, which a bench declaring
  `allocations 0` then fails on. Above, `($total % 4) as int` is what keeps the read free, and the
  count columns are what made this visible.
- **A bench reads nothing on standard input** unless a sibling `<name>.in` is there, which every
  timed and counted run is then sent from its first byte
  ([docs/examples/README.md](../../docs/examples/README.md) § *A file for standard input*). Input
  can be read once per run, so a bench of `Core\IO::stdin` declares `iterations 1`.
- **A bench answers no request** unless a sibling `<name>.nvsr` describes one, which every timed
  and counted run then answers ([docs/examples/README.md](../../docs/examples/README.md) § *A request
  for the program*). A `Core\Request` member throws without one.
- **Size it to run in well under a second.** The sweep runs one program per feature and there are
  hundreds; a bench that takes ten seconds costs an hour across the tree. Raise `iterations` until
  the reading is stable, not until it is long.

## What a bench declares, so its first run can be judged

A bench with no history still has a yardstick: what its author knew before measuring. Each of
these is optional, checked by `--record-perf` on every run including the first, and a bench that
misses one is a failing proof — no record is written, and
[`rule:testing/a-failing-proof-is-fixed-or-recorded`](../../docs/rules/testing.md#testing-a-failing-proof-is-fixed-or-recorded)
names the two answers, the second being a `// proof: gap` marker on the bench.

- **`// bench: allocations 0`**, and likewise `calls`, `statements`, `bytes` — what one operation
  should count, in the four counts below. Met to within a hundredth per operation, so the one
  set-up allocation of the `$labels` array rounds away and an allocation per call does not. A
  member that returns a scalar declares `allocations 0`; a member bench declares `calls 0`, because
  a `Core` member is a helper and not a compiled call site, and a bench over a language feature
  that calls a method declares the calls it makes.
- **`// bench: complexity constant`** (or `linear`), paired with a sibling **`<name>.scale.nvs`**
  that runs the same operation over an input `// bench: scale K` times larger and declares its own
  `iterations`. Both are timed in the same sweep, and the ratio of their per-operation figures may
  not exceed three times what the complexity predicts — one for constant, K for linear. An upper
  bound only: a linear member over a short input is dominated by its fixed per-call cost and looks
  nearly constant, which is not a bug, while growing faster than declared is. This is the one
  wall-clock check that holds on any machine, because both numbers came from the same one seconds
  apart, and it is the only one that sees inside a Rust member.
- **`// requires: unix`** — the bench opens a Unix-domain socket, which this build has only on
  Unix, so `--record-perf` leaves it out on Windows. The ledger is measured on Windows, so such a
  member's perf proof is a `skip` entry in `data/proofs/policy.json` naming that reason, and the
  bench on disk is what a Unix sweep measures.

## What the numbers mean

A record is **four counts and one clock**, all per operation.

The counts — `statements`, `calls`, `allocations`, `bytes` — come from `nvs run --count`, which
reads the probe sites every compiled unit carries
([`rule:testing/debug-probes`](../../docs/rules/testing.md#testing-debug-probes)) in the counting
mode of [`rule:testing/bench-counters`](../../docs/rules/testing.md#testing-bench-counters), plus
the allocator's own per-thread totals. They are **the same on every machine and every day** for the
same program and binary, so the report diffs them against the previous record wherever it was
taken. They count what the program asked for, not what it cost: a `Core` member is one helper call
however much it does inside, and only the allocator sees into it. A member that got slower without
allocating is invisible to every count and visible only to the clock on one machine. That gap is
accepted, since an extra allocation or copy is the common regression.

The clock is measured against two programs under `_calibration/`, in the same sweep as everything
else:

- **`baseline.nvs`** — the empty program. Its time and its counts are `nvs run` starting, compiling
  and exiting, and both are **subtracted** from every reading, so a figure is the work rather than
  the CLI.
- **`unit.nvs`** — a fixed arithmetic loop that will not change. Every clock figure is also divided
  by it, and *that* ratio is the `units` column.

`ns/op` is the fastest of the reps and honest on the machine that took it and meaningless on
another one, which is [ADR 0026](../../docs/decisions/0026.md)'s whole finding — so every record
carries a machine fingerprint and the report refuses to print a clock delta across two of them. The
`median` rides beside it so a delta can be read against the spread it was taken in: a delta inside
that spread is the machine, not the code. `units` divides out the clock speed and travels, to about
a tenth. None of it is a gate: nothing here fails a build except a bench missing what it declared,
and a regression is a row in the report with a `Δ` on it.

**Do not edit `_calibration/unit.nvs`.** Every `units` figure in the ledger's history is relative to
it, and changing it silently re-bases all of them. If it ever must change, that is a new unit and a
new ledger.

## When a figure is re-measured

Only when the implementation's code moves. Every record carries `bun nv proofs --impl-hash` of the
implementing file, and `bun nv proofs --gate` accepts a figure while that hash still matches. Change
a member's body in `crates/nvs-stdlib/src/str.rs` and every `Core\Str` figure goes stale at once;
add a test or a comment to the same file and nothing is re-measured. What the hash reads, and why
the code rather than the commit, is `rule:testing/member-perf-ledger`.
