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
loop session red — nothing outside this suite reads any of it. The cost is that a default run is
slower, because CPython is far behind the other engines on these loops and pays that on every rep;
`--engines nvs,php` and the case patterns are the escape. Every twin is one more file to keep in
step, and the byte-identical gate is what stops one rotting silently.

**Declined:**
- **A Python column transliterated from the PHP cases.** A transliteration measures how badly one
  language writes another's idiom, and the arithmetic exception is stated as one so it is not read
  as licence for more.
- **PyPy as well as CPython, or instead of it.** CPython is what a tool is actually run under, and a
  second Python engine doubles the twins to sharpen a comparison the command-line audience never
  meets. `--engines` makes adding one later a flag.
- **Leaving Bun out because the command-line audience is not switching from it.** Bun is the
  closest engine to what Novis is trying to be, a fast JIT behind a scripting surface with the
  batteries in the binary, so its numbers are the ones that test this project's own claims.
