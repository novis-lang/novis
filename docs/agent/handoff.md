# Handoff

## State

**Stage 0 is closed.** All seventeen items of [loop-goal.md](loop-goal.md) § *Stage 0* are done, and
every one of the 37 tests `loop-goal.toml`'s `stage = "0 catch-up"` block names exists and passes
(checked by name, this session). `loop.py` no longer short-circuits there, so **the loop is on
Stage 3**, whose frontier is `examples/collect.mwl` — the plan's `Open now` and `Blocking` hold the
detail.

**Item 15's last named test landed with the ABI it waited on.**
`mwl_array_get_index(array, i64, out)` and `mwl_array_set_index(array, i64, value) -> *mut
ArrayHeader` now sit beside the key-taking pair, answering from `Shape::Packed` with no decimal
rendered and nothing allocated, and synthesizing a key only where the shape is already `Hashed`.
`crates/mwl-runtime/src/array.rs`'s module doc owns the decision and what it spends; nothing about it
is restated here.

**One half of item 15's measured claim is still unbanked, and it is not a Stage 0 item.** `$a[] = $v`
and `foreach` reach the packed form from compiled code today; `$a[$i]` does not, because
`mwl_ir::lower::Lowering::lower_array_key` normalizes an `int` subscript to a decimal string through
`Helper::IntToString` before `InstKind::ArrayGet`/`ArraySet` ever reaches codegen. The key's
representation at the emit site is therefore already `Ty::Str` and the allocation has already
happened, so routing it is an `mwl-ir` change, not a codegen-local one. That is the next group.

Verify is green (**1560** tests, 74 suites, clippy and fmt clean) — +2 over last session, the new
guard and one for the allocator counter it needed. Conformance **435**, differential **90**,
untouched: no `.mwlt` case was added or edited, so `mwl test tests/` is unmoved at 519 passed /
6 failed (the PHP-on-Windows oracle set). No valgrind run: the new primitives are not reachable from
compiled code yet, and their refcount protocol is `mwl_array_set`'s unchanged.

**A second writer is editing this tree.** `docs/adr/0051-standard-library-tiers.md`,
`docs/adr/ground-rules.md` and `docs/implementation-plan.md`'s M10 body arrived modified mid-session
and are **not** this session's work; the M10 paragraph rode along in the commit that carries the
plan's status fields, which that message says.

## Next group — routing `$a[$i]` through the new pair

Slices 1 to 3 are one file set: `crates/mwl-ir/src/lower/expr.rs`, `crates/mwl-ir/src/lower/stmt.rs`,
`crates/mwl-ir/src/ir.rs`, `crates/mwl-codegen/src/emit.rs`, `crates/mwl-codegen/src/lib.rs`. Slice 4
shares none of it — take it only on its own. ADR 0007 § 5 is the semantics they must not move; the
shape of the change is stated in `crates/mwl-runtime/src/array.rs`'s module doc § *the ABI was the
part that expired*.

- [ ] **Let `InstKind::ArrayGet`/`ArraySet` carry a `Ty::Int` key.** No new variant: the key value's
      own representation is the discriminant, which is why `mwl_array_get_index` takes an `i64`
      rather than a second instruction. Anchors: `crates/mwl-ir/src/ir.rs:913` (`ArrayGet`), `:953`
      (`ArraySet`) — both doc comments state the "every key is normalized to `Ty::Str`" rule that is
      being widened. **`Ty::Uint` stays on the string path**: a `uint` above `i64::MAX` renders a
      decimal `integer_key` refuses, so it is an ordinary string key today and wrapping it to a
      negative index would move semantics.
- [ ] **Stop rendering an `int` subscript in `mwl-ir`.** `lower_array_key` is
      `crates/mwl-ir/src/lower/expr.rs:1876`; its four callers are `:3273` (an array literal's
      explicit `key =>`, which reaches `ArraySet`), `:3339` (the `Index` read),
      `crates/mwl-ir/src/lower/stmt.rs:563` and `:624`. Check which of those four is `unset`'s —
      `mwl_array_unset` has **no** index-taking form, so that one must keep converting. An
      unconverted key is not refcounted, so the "release the fresh key right after the read" the
      `Index` arm does must go with it, and `Self::aliasing_read`'s second return value is
      meaningless for one.
- [ ] **Dispatch on the key's representation in codegen.** `emit_array_get` is
      `crates/mwl-codegen/src/emit.rs:1837` and `emit_array_write` `:1861` — both already take the
      `Ty` back from `self.value(key)` and discard it. New `RuntimeSig::ArrayGetIndex`/`ArraySetIndex`
      at `:2345`, their `runtime_ref` arms at `:2290`, and the two `Signature`s beside
      `crates/mwl-codegen/src/lib.rs:608`, built next to `array_value_at`'s at `:969` (`ptr, I64,
      ptr` and `ptr, I64, ptr -> ptr`). Symbols are already in the table
      (`crates/mwl-runtime/src/helpers.rs:1198`). Snapshots move: `cargo insta test --accept -p
      mwl-ir`, and read the diff rather than accepting it blind — the `int_to_string` call and its
      release vanishing is the whole proof.
- [ ] **The first `docs/perf/history.ndjson` entry**, with a `php_ratio`, per
      [ADR 0026](../adr/0026-performance-measurement-methodology.md) §§ at `:100` (schema and where
      it is appended) and `:190` (what M3 owes it). Only `docs/perf/userland.ndjson` exists today.
      Do this **after** the three above, or the entry records the pre-wiring `$a[$i]` number.

## Backlog

- `Core\Out::capture` is the one remaining key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`, and its `Sink` return type waits on ADR 0088's sink work — `docs/spec/01-core-library.md:888`.
- `examples/collect.mwl:47` is Stage 3's last unfrozen fixture and stops on exactly that member — plan `Blocking`.
- `Core\Json::decodeAs<T>` — spec § 6's one gap, `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — plan `Open now`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
