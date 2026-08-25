# Handoff

## State

**Spec § 5 owes only `replaceWith`.** `Core\Regex\Pattern` is registered — two slots, the pattern as
written and a flag bitmask — `Core\Regex::compile` builds one, and the five matching rows now take the
`Pattern|string` the spec writes, so a handle's flags reach the engine. The flags are spliced as one
inline group rather than through either builder (`fancy-regex` has no `swap_greed` setting), and the
per-core cache is keyed on (text, flags); `crates/mwl-stdlib/src/regex.rs`'s module doc owns both, and
its gap 1 is now `replaceWith` alone. `Pattern` is the registry's first **handle** — slots read by
another class's members, no member of its own — which `registry.rs`'s
`a_class_with_slots_has_instance_members_and_the_reverse` names as a carve-out.

The ratchet (`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is down to **5 keys**: `§2 from`,
`§5 replaceWith`, `§11 Hash::stream`, `§11 Random::bytes`, `§12 Out::capture`. Conformance is 413 of 600
and differential 89 of 150. `examples/collect.mwl`'s frontier is still `Core\Out::capture` at
`collect.mwl:47`, which lands with M4S.

Two open temporary-lifetime gaps of one family are named where they live: `mwl_ir::lower::Lowering`'s
`owned_temporaries` field doc holds the transferred-argument case, and `landing_block`'s *Known gap* holds
the producers that still release inline. `mwl-ir`'s crate doc gap 2 is the index of both.

## Next group — the ratchet's three registerable rows

**These do not share a file**, which is the honest statement: each remaining key is its own module plus a
`.mwlt` case, and a session taking a second slice pays for a second file's context. [2] and [3] are the
pair worth combining — both are § 11 and both are `bytes`-valued. Take [1] alone if you take it first.

- [ ] **`Core\Regex::replaceWith`** — spec row at `docs/spec/01-core-library.md:545`,
      `replaceWith(string $subject, Pattern|string $pattern, callable $fn, {limit?: uint}): string`.
      All in `crates/mwl-stdlib/src/regex.rs`: the row goes beside `replace` (`regex.rs:144`),
      the helper beside `mwl_core_regex_replace`, the `address()` arm at `regex.rs:402`. It is `replace`
      with a closure where the template is — `pattern_of` (`regex.rs:611`) already decodes the pattern,
      and `mwl_runtime::call_closure` carries the `Match` built by `built_match`. Strike `§5 replaceWith`
      in the same commit; § 5 is then whole.
- [ ] **`Core\Random::bytes`** — spec row at `docs/spec/01-core-library.md:739`,
      `bytes(uint $count): bytes`. `crates/mwl-stdlib/src/random.rs`: row in `CLASS` (`random.rs:81`),
      `address()` arm at `random.rs:141`, helper beside the others from `random.rs:251`. `Tag::Bytes`
      exists, so this is a `MwlStr` of raw octets — `Core\Encoding::toHex` is how the case asserts it,
      and the case must not assert a *value*, only a length and that two calls differ.
- [ ] **`Core\Hash::stream`** — spec row at `docs/spec/01-core-library.md:760`,
      `stream(Digest $digest): Hash\Stream`, and the rows the spec writes on `Hash\Stream` just below it.
      `crates/mwl-stdlib/src/hash.rs`: `CLASS` at `hash.rs:167`, `address()` at `hash.rs:199`,
      `DIGEST` enum at `hash.rs:119`. A **mutable** `Core` instance, so `crate::instance::set_slot` is
      the write half; `objmap.rs` is the shape to copy, and its slot must be a value MWL can hold.

## Backlog

- ADR 0056 § 4's sink is unenforced — no registry row can state a qualifier at all (`regex.rs` gap 2).
- `§2 from` waits on an `Iterable`/`Iterator` argument (`docs/implementation-plan.md` *Open now*).
- `§12 Out::capture` is the last ratchet key and `collect.mwl`'s frontier (plan *Open now*).
- `matchAll` reports offsets in O(n·k); the fix is a cursor across cluster boundaries (`regex.rs` gap 4).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` crate doc).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
