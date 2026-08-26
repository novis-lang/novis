# Handoff

## State

**Stage 4's two counts are the frontier — conformance 486 of 600, differential 127 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **486 passed, 0 failed**, `mwl test tests/differential` is **127 passed,
0 failed** and `mwl test tests/` is **613 passed, 0 failed** — run those as well as `verify.py`, which
executes no `.mwlt` case at all (playbook, twice), and **rebuild `target/release/mwl.exe` first** if
anything under `crates/` is newer than it (playbook, *Running things*).

**Differential is the half that moves, and the reason is cost**: an `--ORACLE--` case has no frozen
output to derive, because PHP computes it. `python tools/gaps.py --differential` is the worklist —
**36 members** with a PHP twin and no oracle case, each with a `file:line` anchor — and `--errors` is the
same list for the unasserted `Fault::` sites on the conformance side. Do not re-derive either. PHP on
this machine is 8.5.9.

**Over `Core\Arr`, ask the key question before writing a twin, because it decides which kind of case you
are writing.** Almost every PHP array function renumbers the integer keys of its result and keeps the
string ones; ADR 0069 § 3 refuses exactly that, so an MWL member either renumbers all of them or none.
Over a list the two rules coincide and the case is an `--ORACLE--` one; over a map or a mixed-key
subject they part and the case is an `--ORACLE-DIVERGES--` one with MWL's own output frozen in
`--EXPECT--`. Seven pairs now exist to copy — the window (`arr-slice-*`), the padding pair, the
`reverse` pair, `replaceRange`, `arr-unique-*`, `arr-sort-by-key-*` and `arr-fill-*`. Which way a member
points is in its own doc comment in `crates/mwl-stdlib/src/arr.rs`; read the comment rather than
assuming the key rule bites. **A member answering a value rather than an array escapes the rule
entirely** — `min`/`max` and `reduce` all match PHP over a map — so for those the divergence, if there
is one, is in the comparison or the callback protocol instead.

**Three comparison facts are settled and pinned.** MWL orders by `mwl_stdlib::ordering::compare_values`,
one row per representation with nothing crossing except the two numeric ones, where PHP's `<` converts:
`min([0, "a"])` is `0` there and a throw here, and two numeral strings compare *numerically* there and
bytewise here, so `min(["1e2", "50"])` is `"50"` in PHP and `"1e2"` in MWL. The third is the same fact
one level up: ADR 0007 § 5 makes every stored *key* a `string`, so `sortByKey` compares keys bytewise
where `ksort` reads a numeral key as a number. That refusal is a `Fault::thrown` and a `catch
(Throwable $e)` does reach it, unlike the `Fault::fatal` an argument-shape guard raises (playbook,
*Writing a test case*, two adjacent bullets).

**MWL's `float` rendering is byte-identical to PHP's** — precision 14, trailing zeros trimmed:
`Core\Math::sqrt(2.0)` prints `1.4142135623731` and `0.1 + 0.2` prints `0.3` on both sides. So the
`Core\Math` group below can `echo` a float straight into an `--ORACLE--` block with no formatting, which
is the one thing that would otherwise have made that group expensive.

**Rendering is what makes a `?T` comparable**: absence is `none` on both sides and the guarded branch
returns `$found as string`. A `?bool` has no such shape — the truthy condition panics `mwl-ir` — so keep
a `bool`-valued subject out of a case about a `?T`-answering member (playbook, *Writing a test case*).

The spellings a case cannot use — a closure through the variable holding it, `Class::method(...)`,
`bool as int`, an array literal wherever an `array<T>` is expected, a bare `function` at file scope, an
`int` literal in an `array<float>`'s element position, `<` over two strings — are all in the playbook
under *Writing a test case* and *Writing MWL itself*; do not re-discover them.

## Next group

All three are differential, all three read a doc comment in `crates/mwl-stdlib/src/math.rs` and write
into `tests/differential/core/`, so a session that loads that file once can take two. `Core\Math` is the
largest remaining cluster in `gaps.py --differential` (13 of its 36), and none of it has a key rule to
diverge over — the questions are the zero and overflow boundaries, and whether MWL widens where PHP
converts.

- [ ] **`Core\Math::min`/`max` against PHP's `min`/`max`** — `crates/mwl-stdlib/src/math.rs:816` and
      `:825`, spec § 3. The same `ordering::compare_values` divergence `Core\Arr::min` already pins,
      asked of two loose values instead of a container: the agreeing half is one numeric domain, and
      the diverging half is PHP's loose `<` over an `int` against a `string`.
- [ ] **`Core\Math::intDiv` and `::mod` against `intdiv` and `fmod`** — `math.rs:875` and `:892`,
      spec § 3. Division by zero and `PHP_INT_MIN / -1` are the two boundaries; check the `Fault::`
      constructor at each site to know whether a `catch (Throwable $e)` reaches it.
- [ ] **`Core\Math::gcd`, `::lcm` and `::hypot` against `gmp_gcd`, `gmp_lcm` and `hypot`** —
      `math.rs:917`, `:932` and `:948`, spec § 3. `gmp_*` needs the `gmp` extension, so check
      `php -m` before writing those two rows; `hypot` is pure core and needs no extension.

## Backlog

- `Core\Arr::flattenDeep` against `iterator_to_array` — `crates/mwl-stdlib/src/arr.rs:2135`, spec § 2.
- The `Core\Str` cluster, 11 members — `compare`, `slice`, `before`, `after`, `chunk`, `codePoints`,
  `replaceAll`, `replaceRange`, `reverse`, `wrap`, `fromCodePoint`, all in `crates/mwl-stdlib/src/str.rs`.
  The Windows `php` has no `mbstring` (playbook), so the `mb_*` twins need the WSL leg or a cited UCD row.
- `python tools/gaps.py --errors` — the conformance side's worklist, every unasserted `Fault::` site.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
