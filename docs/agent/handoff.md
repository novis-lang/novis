# Handoff

## State

**Stage 0 item 19 is closed and Stage 3 is still green.** `mwl-codegen`'s
`an_integer_subscript_reaches_the_packed_form_from_compiled_code`
(`crates/mwl-codegen/tests/arrays.rs:110`) holds the compiled half of the packed-array claim
`mwl_runtime::array`'s `an_integer_subscript_allocates_no_key` holds for the primitive: 404 passes over a
four-element list allocate exactly what 4 passes allocate, while the same accesses through a rendered key
allocate per access. It measures with a `#[cfg(debug_assertions)]` `#[global_allocator]` of its own, because
`mwl-runtime`'s `counting_alloc` is `#[cfg(test)]`-private to that crate; the playbook's two new bullets own
both halves of that shape.

`docs/perf/history.ndjson` exists and holds its first entry —
`{"workload":"userland_08_array_list_build","instructions":217254092,"php_ratio":0.93}`, measured in WSL
against PHP 8.5.9 on a release build taken *after* item 18's allocator landed. ADR 0026 § 4 now carries the
three-command recipe and the finding that a whole `mwl run` reproduces to six significant figures rather
than bit-for-bit, which narrows that ADR's § 2 claim (measured of `callgrind_spike`, a bare example binary).
Items 15 and 19 are struck in `loop-goal.md`. Verify is green: **1593** tests, 74 suites, clippy and fmt
clean.

**The first red acceptance check is now Stage 0's `mwl-runtime (string capacity)`** — item 20, three tests
that do not exist. That is the next group.

## Next group

Item 20's one layout revision, in the order its three tests are listed. Shared file set:
`crates/mwl-runtime/src/string.rs`, `crates/mwl-ir/src/ir.rs` + `crates/mwl-ir/src/lower/expr.rs`, and
`crates/mwl-codegen/src/emit.rs`. `loop-goal.md` item 20 argues all three; nothing here restates it.

- [ ] **`appending_to_a_uniquely_owned_string_does_not_reallocate`** — `StrHeader`
      (`crates/mwl-runtime/src/string.rs:127`) gains a capacity beside its refcount and length, and
      `mwl_str_append` (`string.rs:912`, which already exists) appends into it on `mwl_array_set`'s own
      *consume one reference, return one* protocol. `mwl_str_concat` is `string.rs:797`. Measure it the way
      the playbook's new bullet says — `counting_alloc::allocated_bytes` is in scope here, since this one is
      `-p mwl-runtime`'s own test.
- [ ] **`a_concat_of_many_pieces_allocates_once`** — an n-ary `InstKind::Concat` (`crates/mwl-ir/src/ir.rs:657`
      already carries the variant, and `lower/expr.rs:1849` already emits it) reaching
      `MwlStr::from_pieces` (`string.rs:263`), which exists and which codegen never calls.
- [ ] **`an_immortal_literal_is_never_retained_or_freed`** — `emit_const_str`
      (`crates/mwl-codegen/src/emit.rs:933`) stops allocating per evaluation. **The part to write down
      rather than re-derive**: an immortal literal lives in the compiled unit, the one thing a request
      shares with another, so `string.rs`'s "no `MwlStr` is ever reachable from two threads" reasoning
      behind the plain `Cell` refcount stops being true as stated. It stays sound — a pinned refcount is
      never written — and the module doc has to say so.

## Backlog

- Item 21: `Core\Arr::map`/`filter`/`reduce`/`sort` synthesize a key per element no callback asked for —
  `loop-goal.md` item 21.
- Item 22: `produced(&str)` copies twice and `text()` re-validates UTF-8, at 56 call sites each —
  `loop-goal.md` item 22.
- Stage 4 is the wall: conformance 437 of 600, differential 90 of 150 — `docs/implementation-plan.md`
  § *Open now*.
- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json`'s module doc.
- ADR 0088's registry-wide qualifier classification — `crates/mwl-stdlib/src/registry.rs`.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
