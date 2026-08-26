# Handoff

## State

**Conformance is at 538 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*). The tree is clean.

**`Core\Math` is 38 members over 31 cases, and only one specified slice is left in it** — the
`format`/`round` agreement below. The two added here take the family's *composition* frontier. The
six hyperbolic members are held to their exponential definitions over 18 rows out to ±5: `sinh` is
the half difference of `exp($x)` and `exp(0.0 - $x)` and `cosh` the half sum, `cosh² - sinh²` is 1
to a step taken relative to `cosh²` (the magnitude the subtraction happens at, not that of its
answer), `tanh` is the ratio and has no pole to be near, `asinh` and `acosh` undo their partners —
`acosh` up to sign, `cosh` being even — and `atanh` is round-tripped the *other* way as
`tanh(atanh($y))`, because in its own direction it amplifies by 5500× at `$x = 5`. Past the table
the claims are properties: `tanh(20.0)` is exactly 1 while `cosh(20.0)` is still finite, and at 1000
both growing members answer `INFINITY`. The three inverse circular members are then pinned to their
*branch*: `asin(sin($x))` reflects about the nearer quarter turn outside `[-PI/2, PI/2]`,
`acos(cos($x))` folds onto `[0, PI]` and turns around again at `TAU - |$x|`, and `atan(tan($x))`
is the one that shifts by whole `PI`s rather than reflecting — which is why `atan2` exists. Each of
the three answers inside its own branch on all 20 rows, and `sin(asin(sin($x)))` recovers `sin($x)`
unconditionally, which is what makes the three counts a statement about the branch and not about
the members disagreeing. Every row of both cases was measured on the WSL leg as well as the native
one, through `php` rather than a cross-build; the two new playbook bullets name the well-conditioned
direction and the exact-on-both-legs rows so the next float session does not re-measure either.

## Next group

The first slice closes `Core\Math` and reads `crates/mwl-stdlib/src/math.rs`; the other two are
`Core\Path` and both read `crates/mwl-stdlib/src/path.rs`. All three add a new file under
`tests/conformance/core/` and all three are the *agreement* shape from conventions.md.
`docs/spec/01-core-library.md` §§ 3 and 7 own the rules. The tolerance spelling, the well-conditioned
direction and the exact-on-both-legs rows are all playbook bullets under *Writing a test case* — do
not re-derive any of them.

- [ ] **`format` and `round` agree wherever both name the same precision** — `Core\Math::format($n,
      {decimals: $d})` renders what `Core\Math::round($n, {precision: $d})` answers, on every row of
      a table, and parts from it only in the options `round` has no opinion about (the separators,
      whose defaults are MWL's and not `number_format`'s). `crates/mwl-stdlib/src/math.rs:110`
      (`round`), `:313` (`format`), `:449` (`FORMAT_OPTIONS`, where the empty group separator is
      decided).
- [ ] **`split` and `join` are inverses through `normalize`'s normal form** — `join` of what `split`
      returned is `normalize($p)` on every row of a table, `split` never yields an empty segment
      however many separators were repeated, and the round trip is asserted separator-free or
      through `Core\Str::replace($p, Core\Path::SEPARATOR, "/")`, per the goal's *Path and the two
      legs* decision. `crates/mwl-stdlib/src/path.rs:104` (`join`), `:111` (`split`), `:118`
      (`normalize`).
- [ ] **`relativeTo` and `join` undo each other, and the boundary is where they stop** —
      `join($base, relativeTo($p, $base))` normalizes back to `$p` over a table, and the pair parts
      exactly where no relative path exists (a different drive, `isAbsolute` disagreeing between the
      two arguments). Both sides of that bound named together. `crates/mwl-stdlib/src/path.rs:104`
      (`join`), `:118` (`normalize`), `:125` (`isAbsolute`), `:132` (`relativeTo`).

## Backlog

- `basename`/`dirname`/`extension`/`withExtension` agree on where the name ends — read
  `path-decomposes-a-path-without-touching-the-disk.mwlt` first; it may already own the claim.
- `Core\Csv` is 2 members over 5 cases and `Core\Validate` 5 cases — the two thinnest areas after
  `Core\Path`; `python tools/gaps.py` is the worklist (`docs/implementation-plan.md`, *Open now*).
- `Core\Json::decodeAs<T>`'s decoder reads scalar-fielded classes only — ADR 0071,
  `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
