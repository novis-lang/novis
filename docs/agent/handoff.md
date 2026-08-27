# Handoff

## State

**M4 — a `Core`-owned class renders where the spec gives it a `toString`, and the
whole static half of ADR 0028 § 1 is closed.** `echo $uri`, `"$uri"`, `"" . $uri` and
`$uri as string` are one call for one value, and they agree with `$uri->toString()`
written out; a `Core` class with no `toString` was already `E0710` at the site.

- **One check records, one lowering chooses.** `require_stringable_object` no longer
  returns early for a `Core` class: whichever question it passed — `Stringable` for a
  declared class, `mwl_stdlib::registry::class_renders` for a `Core` one — it records
  the same resolved `toString` under the operand's span. `lower_to_string_call` then
  asks `mwl_types::core_symbol_of`, and a `Core` member takes the native
  `InstKind::CoreCall` its written-out spelling takes instead of a `CallVirtual` into a
  method table it has no entry in. Nothing dispatches on the runtime class there,
  because a `Core` class is final by construction.
- **The ownership inverts with the call and that is the new refcount edge.** A native
  member *borrows* argument 0, where a compiled one owns its parameters — so an
  aliasing receiver is no longer retained and a fresh one (`echo Core\Uri::parse(…)`)
  is the rendering site's to release, staged on the temporaries stack so a throwing
  edge drops it too. Valgrind clean over a fixture that renders in a loop.
- **ADR 0088 § 5's sink carrier is untouched and is why the two rows stay apart.** It
  is the one rendering class with no `toString` member, so `resolve_method` finds
  nothing, nothing is recorded, and it still renders its own bytes through
  `mwl_runtime::stringify`. The new case pins that by rendering an `echo` through
  `Core\Out::capture`, whose answer is a carrier.
- **What is left of `mwl-ir`'s known gap 12 is the erased operand.**
  `mixed $m = Core\Uri::parse(…); echo $m;` still throws "does not implement
  `Stringable`": the runtime dispatch reads a compiled method table, and `mwl-runtime`
  is below `mwl-stdlib` so it cannot ask the registry. That gap's own text names the
  two shapes that could close it.

## Next group

**The two rows `Lowering::convert`'s catch-all still names, then the erased half
above.** The file set: `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-runtime/src/helpers.rs`, `crates/mwl-stdlib/src/registry.rs`,
`tests/conformance/lang/`.

- [ ] **`$m as Plain` — a tagged operand converted to an object** — ADR 0007 § 6's
      checked way out of `mixed`. The design is already resolved and needs no new
      instruction, no new `Helper` row and no new `ExprInfo`: `InstKind::InstanceOf`
      already takes a `Ty::Tagged` subject and answers `false` for a non-object tag
      (`crates/mwl-codegen/src/emit.rs:2140` bakes the descriptor in), so the row is a
      test, a `Terminator::Throw` on the false edge and an `InstKind::Untag` on the
      true one. Write it at `crates/mwl-ir/src/lower/expr.rs:4746`
      (`lower_conversion`'s `None =>` arm), **not** inside `convert`
      (`crates/mwl-ir/src/lower/expr.rs:1032`), whose `cur` is by value and so cannot
      branch; `lower_literal_membership`
      (`crates/mwl-ir/src/lower/expr.rs:5240`) is the same test-then-throw shape with
      `cur: &mut BlockId` and is the one to copy, and
      `crates/mwl-ir/src/lower/closure.rs:385` is the synthesized `New` +
      `Terminator::Throw` pair. The class name needs no new table: it is
      `exprs.declared_ty(ty.span)` read back through `checked_types`, the route
      `lower_decl_type` (`crates/mwl-ir/src/lower/mod.rs:2402`) already takes. A
      `Helper` row is the wrong shape here — helper arguments are stored as `Value`s
      (`crates/mwl-codegen/src/emit.rs:1802`), so a `Ty::ClassDesc` cannot ride one.
- [ ] **`$xs as array<U>`** — the last row of that catch-all, ADR 0007 § 2's O(n)
      element walk, and the one the playbook says blocks three written cases
      (`Core\Csv::format`'s column refusal, a nested `array<mixed>` read). Catch-all at
      `crates/mwl-ir/src/lower/expr.rs:1321`; its `?` twin is `convert_or_null`
      (`crates/mwl-ir/src/lower/expr.rs:1380`), whose panic names the same row.
- [ ] **A `Core` object behind a `mixed` renders** — gap 12's residual, above. It is a
      `mwl-runtime`/`mwl-stdlib` boundary question, not a lowering one:
      `crates/mwl-runtime/src/helpers.rs:985` (`stringify`) and
      `crates/mwl-runtime/src/ctx.rs:179` (`is_carrier`, the roster shape that already
      crosses the boundary by name).

## Backlog

- `Lowering::convert`'s catch-all message still names both missing rows — rewrite it as
  each lands (`crates/mwl-ir/src/lower/expr.rs:1321`).
- The goal's `[context] playbook` selector prints neither of the two bullets about
  `wsl.exe` needing `MSYS_NO_PATHCONV=1` from the Bash tool, so a session that runs the
  documented `tools/leak-check.sh` invocation still pays for that call.
- ADR 0028 § 2's abandoned-generator `finally`, pre-authorized in
  `docs/agent/loop-goal.md` § *Standing decisions*.
