# Handoff

## State

**M4 — item 28 is closed at both ends: a `Core`-owned class renders wherever
ADR 0028 § 1 says it does, including behind an operand whose static type names
no class.** `mixed $m = Core\Uri::parse(…); echo $m;` answers what
`$uri->toString()` answers, and so do `"$m"`, `"" . $m` and `$m as string`, for
each of the three classes the spec gives a `toString`. `mwl-ir`'s known gap 12
is gone from that list rather than reworded.

- **What the two crates share is the descriptor, not a roster.**
  `mwl-runtime` cannot read `mwl_stdlib::registry`, so `mwl_stdlib::instance`
  (`crates/mwl-stdlib/src/instance.rs:158`) puts the class's own registered
  `toString` address on its `ClassDesc` through `ClassTable::set_render`
  (`crates/mwl-runtime/src/object.rs:697`), and
  `mwl_runtime::dispatch::call_render` (`crates/mwl-runtime/src/dispatch.rs:101`)
  is what `stringify` (`crates/mwl-runtime/src/helpers.rs:1006`) asks before
  the method table. It is derived from `registry::render_symbol`, the member
  half of `class_renders`, so the runtime cannot name a member the *checker*
  did not see.
- **It is not a method-table row, and the reason is the calling convention.**
  That table holds compiled MWL functions, which release their parameters; a
  native `Core` member is an ADR 0002 helper and borrows argument 0. Which
  descriptor field an address came out of is what says which reference the
  caller owes — so `call_render` neither retains nor releases, where
  `call_method` does both. Valgrind-clean over a fixture that renders a
  borrowed and a fresh operand two hundred times each.
- **The sink carriers needed nothing.** A carrier has no `toString` member, so
  both lookups answer `None` and `value_to_string`'s ADR 0088 § 5 branch
  renders its slot — behind a `mixed` exactly as through its own type. Only
  `Core\Cli\Text` is reachable from source; `Core\Html\Markup` arrives at M7.

## Next group

**`mwl-ir` gap 8 / item 16: a named argument and a spread argument lower.** The
file set: `crates/mwl-ir/src/lower/call.rs`, `crates/mwl-types/src/expr/args.rs`,
`tests/conformance/lang/`, `tests/differential/lang/`. The checker half is
pre-authorized to land first (loop-goal.md § *Standing decisions*); confirm what
`args.rs` already resolves before writing a lowering with nothing to lower
against.

- [ ] **A named argument lowers at a resolved call or `new`** —
      `crates/mwl-ir/src/lower/call.rs:85` is the panic, inside
      `lower_call_args` (`:74`), which today refuses anything but
      `CallArgs::List`. Every parameter still has to receive exactly one value
      in declaration order, which is what the default-materializing pass just
      below it already does.
- [ ] **A named or spread argument through a `callable`** —
      `crates/mwl-ir/src/lower/call.rs:652` and `:667`, the same two refusals on
      the closure path, where there is no declared signature to name a
      parameter by. Decide and record whether that makes it a diagnostic rather
      than a lowering (ADR 0031 § 1's `callable` carries the parameter *types*,
      not their names).
- [ ] **The two named cases item 16 still owes** —
      `tests/conformance/lang/a-reference-argument-is-written-back-before-the-next-read.mwlt`
      and `tests/differential/lang/a-reference-argument-matches-phps.mwlt`, over
      the `&$x` staging that same file already does. `python tools/loop.py
      --list` is the roster; 14 named cases are still unwritten across stage 8.

## Backlog

- ADR 0024 § 5's `string as Core\Html\Markup` is the one target left in
  `Lowering::convert`'s catch-all, and waits on `Core\Html` at M7 — plan
  § *Open now*.
- `$m->toString()` on a `mixed` receiver still panics rather than diagnosing
  (`crates/mwl-ir/src/lower/expr.rs:3731`); `E0477` is named there for the
  erased case — `mwl-ir` known gap for item 25.
- `Core\ObjMap` cannot be `new`ed from source at all: codegen refuses a class
  reference the unit declares no descriptor for — found while looking for a
  non-rendering `Core` class with instances.
- ADR 0007 § 4's promotion table has 9 refusal sites still standing, the largest
  single item left (`python tools/holes.py --item 1`).
- `object` as a declared type has 2 representation arms left —
  `python tools/holes.py --item 25`.
- ADR 0013's `Comparable` is still satisfied by member rather than by
  declaration, so `$a < $b` over two `Core\Time\Duration`s is refused —
  `crates/mwl-stdlib/src/time.rs:80`.
