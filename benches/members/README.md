# The per-feature bench tree — one measured figure each

Every feature Novis ships owes a measured performance figure
([ADR 0134](../../docs/decisions/0134.md)), and this is where the programs
that produce it live. **Every number here is Novis against itself.** There is no PHP column and
there never will be — [`benches/userland/`](../userland/README.md) owns the cross-engine comparison,
and [`benches/abi-probe/`](../abi-probe/) owns the guards that fail a build. This tree exists to
answer one question: *we changed something — what did it cost?*

`python tools/dossier.py --record-perf` measures them and appends to
[`docs/perf/members.ndjson`](../../docs/perf/members.ndjson); `--perf-report` renders
[`docs/perf/members.md`](../../docs/perf/members.md) from it. This file owns what a bench **is**.

## Where a bench goes

`benches/members/<the feature's path>.nvs` — one file, not a directory, because a feature has one
figure: `benches/members/core/Str/length.nvs`. `python tools/dossier.py --id '<feature>'` prints the
path.

## What a bench is

```nvs
<?nvs
// Counting the graphemes of a short label -- what every truncation and column width does.
// bench: iterations 400000

class Bench {
    public static function run(int $rounds): int {
        var $labels = ["order", "customer", "shipping address", "ünïcödé"];
        var $total = 0;
        var $i = 0;
        while ($i < $rounds) {
            $total = $total + Core\Str::length($labels[$total % 4]);
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(400000), "\n";
```

Four rules, and the third is the one that is not obvious:

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
- **Size it to run in well under a second.** The sweep runs one program per feature and there are
  hundreds; a bench that takes ten seconds costs an hour across the tree. Raise `iterations` until
  the reading is stable, not until it is long.

## What the numbers mean

Two programs under `_calibration/` are measured in the same sweep as everything else:

- **`baseline.nvs`** — the empty program. Its time is `nvs run` starting, compiling and exiting, and
  it is **subtracted** from every reading, so a figure is the work rather than the CLI.
- **`unit.nvs`** — a fixed arithmetic loop that will not change. Every figure is also divided by it,
  and *that* ratio is the `units` column.

`ns/op` is honest on the machine that took it and meaningless on another one, which is
[ADR 0026](../../docs/decisions/0026.md)'s whole finding — so every
record carries a machine fingerprint and the report refuses to print a delta across two of them.
`units` divides out the clock speed and travels, to about a tenth. Neither is a gate: nothing here
fails a build, and a regression is a row in the report with a `Δ` on it.

**Do not edit `_calibration/unit.nvs`.** Every `units` figure in the ledger's history is relative to
it, and changing it silently re-bases all of them. If it ever must change, that is a new unit and a
new ledger.

## When a figure is re-measured

Only when the implementation moves. Every record carries the commit that last touched the file
implementing that feature, and `python tools/dossier.py --gate` accepts a figure while that commit
still matches. Change `crates/nvs-stdlib/src/str.rs` and every `Core\Str` figure goes stale at once;
change something else and nothing is re-measured. That is what keeps a sweep over hundreds of
features affordable enough to actually run.
