# The userland benchmark suite

Twenty-odd pieces of ordinary web-and-CLI code, each written once per engine: `NN-slug.mwl`,
`NN-slug.php` and `NN-slug.py`. They are not micro-benchmarks of the runtime — [ADR 0026](../../docs/adr/0026-performance-measurement-methodology.md)
owns *that* measurement, counted in instructions so it compares across machines. These are the loops a
person actually writes: build a string, tally an array, sort a table, match a regex, encode a payload.

**Python is here because MWL's CLI claim is made against Python**, and this project does not publish an
unmeasured claim — [ADR 0100](../../docs/adr/0100-against-python-mwl-claims-the-tool-that-gets-handed-over.md)
§ 5 is the decision and `--engines` is how you narrow it.

Run them with one call:

```sh
python tools/bench.py                    # every case, every engine, side by side
python tools/bench.py 05 regex           # only the cases whose name contains these
python tools/bench.py --check            # do all three agree? (no timing)
python tools/bench.py --engines mwl,php  # narrow the roster; mwl is always in it
python tools/bench.py --php-mode default # PHP as installed, not with opcache+JIT
```

`python tools/bench.py --help` is the rest. That script owns *how* a case is measured; this file owns
what a case **is**.

## What a case is

Files with the same stem, one per engine, and nothing else — no manifest, no registration. The `.mwl` file
is what defines a case; a missing `.py` or `.php` twin skips that engine with a warning rather than
silently reading as agreement:

- **They print the same bytes.** Every case ends in one `echo` of a checksum over the work it did. The
  harness refuses to report a time for a case whose halves disagree, because a benchmark that has
  quietly stopped doing the same work in one language is worse than none.
- **They describe themselves in their first `//` line**, which is what the harness prints. One sentence,
  naming the *userland* thing it stands for — "the placeholder fill every mailer does", not "str_replace".
- **They are written the way a person writes them**, in each language's own idiom: `explode`/`implode`
  against `Core\Str::split`/`join`, `usort` with a comparator against `Arr::sort` with a `by` key, PHP's
  `.=` against Python's list-and-`join`, `array_map` against a comprehension. A case is a fair fight
  between each language's normal spelling, not a transliteration of one into the others.
- **The one exception is arithmetic that has to agree.** PHP and MWL truncate `%` toward zero where Python
  floors it, so a case whose running total goes negative writes the truncated remainder explicitly, in a
  named helper with a comment saying why. Two cases need it — `01-arith-loop` and `19-object-property` —
  and it is a requirement of the byte-identical gate, not licence to transliterate anything else
  ([ADR 0100](../../docs/adr/0100-against-python-mwl-claims-the-tool-that-gets-handed-over.md) § 5).
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
- `<engine> work` subtracts `00-baseline`, the empty program. Every engine pays to start a process and
  produce code before any userland line runs, and over a 40 ms case that cost *is* the measurement.
  `total` answers "what does this script cost me at the prompt"; `work` answers "how fast is the
  language". **For a CLI claim the headline is `total` and for a language claim it is `work`**, and
  quoting either without saying which is the misuse
  [ADR 0100](../../docs/adr/0100-against-python-mwl-claims-the-tool-that-gets-handed-over.md) § 2 forbids.
- `php/mwl` and `py/mwl` are ratios of the `work` figures: **above 1.00 means MWL is faster.**

Two things the table cannot say for itself:

- **A ratio is one machine, one moment.** Wall clock is not comparable across machines, which is the whole
  reason ADR 0026 counts instructions instead. Quote a ratio with the host it came from, or don't quote it.
- **A ratio can depend on the size.** `03-string-concat` was the standing example — MWL's `.=` was
  quadratic where PHP's was flat, so that case's ratio was a statement about 20,000 rows and nothing
  else. It no longer is: the string gained capacity and n-ary concatenation, and
  [docs/perf/userland-gap.md](../../docs/perf/userland-gap.md) owns what that closed and what is left.
  The rule outlives the example. When a case's cost is not linear in its own size, say so here rather
  than letting the number imply it is a constant.

## PHP's configuration is chosen, not inherited

`--php-mode jit` (the default) runs PHP with `opcache.enable_cli=1` and the tracing JIT, because the
project's own target is stated against PHP *with* JIT and a comparison against an engine asked to run
slower proves nothing. `--php-mode default` runs it exactly as installed, which is what a CLI user
actually gets. The mode is recorded in every JSON record so a history file cannot silently mix the two.

MWL is always the **release** binary: `tools/bench.py` refuses a `target/debug/` build by name, because a
debug build measures its own assertions. Nothing in this suite builds anything — the binary on disk is the
binary that runs, and the harness warns when it is older than the newest file under `crates/`.

**Python is the interpreter running `bench.py`** unless `--python` names another, and it is passed no
flags: there is no second CPython mode the way there is a second PHP one, and CPython is what a tool is
actually run under — a PyPy column was considered and rejected in ADR 0100's *Alternatives rejected*.
Expect Python to win the cases where its loop is really a call into C (`sorted`, `sum`, a comprehension,
`in` over a list) and to lose the ones that are a real interpreted loop. That split is the honest shape of
the comparison and neither half should be quoted without the other.
