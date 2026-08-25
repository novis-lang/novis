# Handoff

## State

**Spec § 4 is whole.** `$d->withTime(TimeOfDay $t): DateTime` is registered, implemented and covered;
`crates/mwl-stdlib/src/time.rs`'s gap 1 now says only that there is no `Core\Month` and never will be,
which is the fact `docs/implementation-plan.md` cross-references by that number. The helper is `with`'s
body reached from a component view — `at.with().time(t).build()` — so all four clock fields go at once
and a wall clock the zone skips resolves forward by the gap under `jiff`'s compatible disambiguation,
which the conformance case pins on Berlin's 2024-03-31 02:30.

The ratchet (`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is down to **6 keys**: `§2 from`,
`§5 compile`, `§5 replaceWith`, `§11 Hash::stream`, `§11 Random::bytes`, `§12 Out::capture`. Conformance
is unchanged at 412 of 600 (the new rows went into an existing case) and differential at 89 of 150.
`examples/collect.mwl`'s frontier is still `Core\Out::capture` at `collect.mwl:47`, which lands with M4S.

Two open temporary-lifetime gaps of one family are named where they live: `mwl_ir::lower::Lowering`'s
`owned_temporaries` field doc holds the transferred-argument case, and `landing_block`'s *Known gap* holds
the producers that still release inline. `mwl-ir`'s crate doc gap 2 is the index of both.

## Next group — `Core\Regex\Pattern`, in three slices

All three are `crates/mwl-stdlib/src/regex.rs` plus a `.mwlt` case; nothing else is touched. `regex.rs:63`
states the whole design debt as gap 1, and ADR 0056 §§ 1, 2, 4 are the sections that specify it. Do them in
order — [2] and [3] are meaningless before [1].

- [ ] **`Core\Regex\Pattern` and `Core\Regex::compile`** — spec row at `docs/spec/01-core-library.md:540`,
      `compile(string $pattern, {caseInsensitive?, multiline?, dotAll?, ungreedy?}): Pattern`. A new
      `CoreClass` beside `MATCH` (`regex.rs:193` is the shape, slots and all), registered in
      `registry::CLASSES`, with an `address()` arm at `regex.rs:294`. The four flags must reach
      `compiled`'s cache key: `regex.rs:361` keys on the pattern text alone today, so the key becomes
      (text, flags) — `Compiled` at `regex.rs:333`, `CACHE_CAPACITY` at `regex.rs:325`. Both engines take
      the same four as builder options, so the tiering at `regex.rs:375` does not change shape. Strike
      `§5 compile` in the same commit.
- [ ] **The six registered rows widen to `Pattern|string`** — spec rows at
      `docs/spec/01-core-library.md:541`-`:546`; the rows themselves start at `regex.rs:116`
      (`matches`) and `regex.rs:137` (`replace`). `CoreTy::Union` is legal in either direction — its own
      doc says so, and the only registry test that restricts a union is `a_union_option_excludes_null`,
      which is about an *option*. Each helper then accepts an instance or a string in the pattern slot; a
      program written against the narrow spelling keeps compiling, which is what `regex.rs:63` promises.
- [ ] **`replaceWith`** — spec row at `docs/spec/01-core-library.md:545`,
      `replaceWith(string $subject, Pattern|string $pattern, callable $fn, {limit?: uint}): string`. The
      callback half of `replace` (`regex.rs:137`); it hands `$fn` a `Match` built the way `matchAll`
      already builds one, through `mwl_runtime::call_closure`. Strike `§5 replaceWith`; § 5 is then whole
      but for gaps 2-4, which are M6/ADR-0088 work rather than member work.

## Backlog

- ADR 0056 § 4's sink is unenforced — no registry row can state a qualifier at all (`regex.rs` gap 2).
- `§2 from` waits on an `Iterable`/`Iterator` argument (`docs/implementation-plan.md` *Open now*).
- `§11 Random::bytes` and `§11 Hash::stream` are unwritten, not blocked (`mwl_stdlib::hash` module doc).
- `matchAll` reports offsets in O(n·k); the fix is a cursor across cluster boundaries (`regex.rs` gap 4).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` crate doc).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
