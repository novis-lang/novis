# The userland benchmark suite

Twenty pieces of ordinary web-and-CLI PHP, each written twice: once as `NN-slug.php` and once as
`NN-slug.mwl`. They are not micro-benchmarks of the runtime — [ADR 0026](../../docs/adr/0026-performance-measurement-methodology.md)
owns *that* measurement, counted in instructions so it compares across machines. These are the loops a
person actually writes: build a string, tally an array, sort a table, match a regex, encode a payload.

Run them with one call:

```sh
python tools/bench.py                    # every case, side by side
python tools/bench.py 05 regex           # only the cases whose name contains these
python tools/bench.py --check            # do the two agree? (no timing)
python tools/bench.py --php-mode default # PHP as installed, not with opcache+JIT
```

`python tools/bench.py --help` is the rest. That script owns *how* a case is measured; this file owns
what a case **is**.

## What a case is

Two files with the same stem, one per language, and nothing else — no manifest, no registration:

- **They print the same bytes.** Every case ends in one `echo` of a checksum over the work it did. The
  harness refuses to report a time for a case whose two halves disagree, because a benchmark that has
  quietly stopped doing the same work in both languages is worse than none.
- **They describe themselves in their first `//` line**, which is what the harness prints. One sentence,
  naming the *userland* thing it stands for — "the placeholder fill every mailer does", not "str_replace".
- **They are written the way a person writes them**, in each language's own idiom: `explode`/`implode`
  against `Core\Str::split`/`join`, `usort` with a comparator against `Arr::sort` with a `by` key. A case
  is a fair fight between two languages' normal spellings, not a transliteration of one into the other.
- **Each iteration's input depends on the previous iteration's result.** This is the one rule that is not
  obvious, and it is not stylistic — see below.

Add a case by writing the pair. `tools/bench.py` finds it; nothing else needs editing.

## Why the inputs are chained

A loop whose input never changes is a loop an optimiser may delete. PHP 8.5's tracing JIT does exactly
that: `06-string-split-join` over a fixed line ran in **43 ms**, which was the empty program's time to the
tenth of a millisecond, against 124 ms with the JIT off. The 200,000 `explode` calls had not got faster —
they had stopped happening.

So every case keeps a running `$total` and indexes its input table by `$total % 16`. Iteration N's input
is not known until iteration N−1 has finished, which no optimiser on either side can hoist, and it costs
one array read. The same discipline is why `10-array-sort` writes one element of its source between
rounds rather than sorting the same array twenty times.

If you add a case, chain it. If a new case ever measures within a millisecond of `00-baseline`, that is
the symptom.

## Reading the numbers

- `mwl ms` / `php ms` are whole-process wall clock, **min** of N reps after a discarded warm-up. Minimum,
  because every source of noise on a desktop adds time and none subtracts it; the median is checked
  against it and the harness says so out loud when the two diverge.
- `mwl work` / `php work` subtract `00-baseline`, the empty program. Both engines pay to start a process
  and produce code before any userland line runs, and over a 40 ms case that cost *is* the measurement.
  `total` answers "what does this script cost me at the prompt"; `work` answers "how fast is the
  language".
- `php/mwl` is the ratio of the two `work` figures: **above 1.00 means MWL is faster.**

Two things the table cannot say for itself:

- **A ratio is one machine, one moment.** Wall clock is not comparable across machines, which is the whole
  reason ADR 0026 counts instructions instead. Quote a ratio with the host it came from, or don't quote it.
- **A ratio can depend on the size.** `03-string-concat` is the standing example: MWL's `.=` is currently
  quadratic (10k rows 66 ms, 20k 333 ms, 40k 4.1 s) where PHP's is flat, so that case's ratio is a
  statement about 20,000 rows and nothing else. When a case's cost is not linear in its own size, say so
  here rather than letting the number imply it is a constant.

## PHP's configuration is chosen, not inherited

`--php-mode jit` (the default) runs PHP with `opcache.enable_cli=1` and the tracing JIT, because the
project's own target is stated against PHP *with* JIT and a comparison against an engine asked to run
slower proves nothing. `--php-mode default` runs it exactly as installed, which is what a CLI user
actually gets. The mode is recorded in every JSON record so a history file cannot silently mix the two.

MWL is always the **release** binary: `tools/bench.py` refuses a `target/debug/` build by name, because a
debug build measures its own assertions. Nothing in this suite builds anything — the binary on disk is the
binary that runs, and the harness warns when it is older than the newest file under `crates/`.
