`benches/userland/` carries a twin per engine beside each case — `.nvs`, `.php`, `.py`, and `.ts`
for Bun — and `bun nv bench` runs whichever engines a case has twins for
(`rule:testing/userland-benchmarks`). **The engine list is data, not a count.** `evaluate`, the
`00-baseline` subtraction, the table, `explain` and the NDJSON record all iterate the list, and
`--engines nvs,php` narrows it; adding an engine is one `Engine` entry plus a suffix. Bun is kept
because it is the strongest engine in the suite: measuring only engines Novis beats is how a
benchmark suite stops being evidence.

**`total` is the headline for a CLI claim and `work` for a language claim**, and the suite refuses to
pick one. `total` on `00-baseline` is the cold-start figure; quoting either without saying which is
the misuse `rule:tooling/python-claims` forbids.

**Byte-identical output is still the gate:** a case whose twins disagree reports `DIFF` and no time.
The fairness rule — each case in its own language's idiom, never transliterated — gains exactly one
exception, of one kind: **arithmetic that must agree.** Python floors `%` where the other three
truncate, so two cases spell the truncated remainder out; a TypeScript `number` is a float64, so the
cases whose seeded LCG passes 2^53 run that one line in `BigInt`. Each is confined to the cases that
need it and carries a comment saying why.

**Neither Python nor Bun is a toolchain dependency.** A missing twin skips that engine with a warning
and an uninstalled engine is narrowed away with `--engines`, so neither can turn a build, a test or a
loop session red — nothing outside this suite reads any of it.
