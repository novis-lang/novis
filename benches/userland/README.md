# The userland benchmark suite

Twenty-odd pieces of ordinary web-and-CLI code, each written once per engine: `NN-slug.nvs`,
`NN-slug.php`, `NN-slug.py` and `NN-slug.ts`. They are not micro-benchmarks of the runtime — [ADR 0026](../../docs/decisions/0026.md)
owns *that* measurement, counted in instructions so it compares across machines. These are the loops a
person actually writes: build a string, tally an array, sort a table, match a regex, encode a payload.

**Python is here because Novis's CLI claim is made against Python**, and this project does not publish an
unmeasured claim — [ADR 0100](../../docs/decisions/0100.md)
§ 5 is the decision. **Bun is here because it is the hardest engine to beat**, and a suite that only
measured engines Novis wins against would stop being evidence. `--engines` is how you narrow the roster,
and an engine you have not installed is narrowed away rather than left to fail every case.

Run them with one call:

```sh
bun nv bench                    # every case, every engine, side by side
bun nv bench 05 regex           # only the cases whose name contains these
bun nv bench --check            # do all four agree? (no timing)
bun nv bench --engines nvs,php  # narrow the roster; nvs is always in it
bun nv bench --php-mode default # PHP as installed, not with opcache+JIT
```

`bun nv bench --help` is the rest. `tools/nv/cmd/bench.ts` owns *how* a case is measured; this file
owns what a case **is**.

## What a case is

Files with the same stem, one per engine, and nothing else — no manifest, no registration. The `.nvs` file
is what defines a case; a missing `.py` or `.php` twin skips that engine with a warning rather than
silently reading as agreement:

- **They print the same bytes.** Every case ends in one `echo` of a checksum over the work it did. The
  harness refuses to report a time for a case whose halves disagree, because a benchmark that has
  quietly stopped doing the same work in one language is worse than none.
- **They describe themselves in their first `//` line**, which is what the harness prints. One sentence,
  naming the *userland* thing it stands for — "the placeholder fill every mailer does", not "str_replace".
- **They are written the way a person writes them**, in each language's own idiom: `explode`/`implode`
  against `Core\Str::split`/`join`, `usort` with a comparator against `Arr::sort` with a `by` key, PHP's
  `.=` against Python's list-and-`join` — but *not* against TypeScript's, where `+=` builds a rope and is
  what a JS author writes; `array_map` against a comprehension against a chained `.map().filter()`. A case
  is a fair fight between each language's normal spelling, not a transliteration of one into the others.
- **The only exceptions are arithmetic that has to agree**, because the byte-identical gate below is a
  correctness requirement and not a style choice. There are two kinds, each carrying a comment saying why
  ([ADR 0100](../../docs/decisions/0100.md) § 5):
  - **Python floors `%` where the other three truncate it toward zero.** `01-arith-loop` and
    `19-object-property` run a negative total, so their Python twins spell the truncated remainder out in
    a named helper. Their TypeScript twins need nothing — JavaScript truncates.
  - **A TypeScript `number` is a float64.** `10-array-sort` and `11-array-sort-by-field` seed themselves
    with an LCG whose product reaches ~2.4e18, past the 2^53 a float64 holds exactly, so those two run
    that one line in `BigInt` and convert back. Every other value in the suite fits.
- **Each iteration's input depends on the previous iteration's result.** This is the one rule that is not
  obvious, and it is not stylistic — see below.

Add a case by writing the pair. `bun nv bench` finds it; nothing else needs editing.

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

- `nvs ms` / `php ms` are whole-process wall clock, **min** of N reps after a discarded warm-up. Minimum,
  because every source of noise on a desktop adds time and none subtracts it; the median is checked
  against it and the harness says so out loud when the two diverge.
- `<engine> work` subtracts `00-baseline`, the empty program. Every engine pays to start a process and
  produce code before any userland line runs, and over a 40 ms case that cost *is* the measurement.
  `total` answers "what does this script cost me at the prompt"; `work` answers "how fast is the
  language". **For a CLI claim the headline is `total` and for a language claim it is `work`**, and
  quoting either without saying which is the misuse
  [ADR 0100](../../docs/decisions/0100.md) § 2 forbids.
- `php/nvs`, `py/nvs` and `bun/nvs` are ratios of the `work` figures: **above 1.00 means Novis is faster.**

Two things the table cannot say for itself:

- **A ratio is one machine, one moment.** Wall clock is not comparable across machines, which is the whole
  reason ADR 0026 counts instructions instead. Quote a ratio with the host it came from, or don't quote it.
- **A ratio can depend on the size.** `03-string-concat` was the standing example — Novis's `.=` was
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

Novis is always the **release** binary: `bun nv bench` refuses a `target/debug/` build by name, because a
debug build measures its own assertions. Nothing in this suite builds anything — the binary on disk is the
binary that runs, and the harness warns when it is older than the newest file under `crates/`.

**Python is the interpreter running each `.py` twin** unless `--python` names another, and it is passed
no flags: there is no second CPython mode the way there is a second PHP one, and CPython is what a Python
program is actually run under — a PyPy column was considered and rejected in ADR 0100's *Alternatives rejected*.
Expect Python to win the cases where its loop is really a call into C (`sorted`, `sum`, a comprehension,
`in` over a list) and to lose the ones that are a real interpreted loop. That split is the honest shape of
the comparison and neither half should be quoted without the other.

**Bun runs the `.ts` file directly**, unless `--bun` names another executable. It transpiles the
TypeScript on the way in, and that stays inside the measurement on purpose: it is part of what a Bun user
pays at start-up, and pre-compiling it away would measure an engine nobody runs. Expect Bun to be the
engine to beat — it is a mature JIT and the suite exists to be evidence, not encouragement.
