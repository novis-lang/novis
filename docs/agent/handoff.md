# Handoff

## State

**M4 — language completeness.** Item 19 is **closed outright**: the lowering it still owed —
a closure capturing an enclosing `&$x` parameter — is one `InstKind::RefLoad` at the literal,
at the pointee type, plus the retain the capture loop already emitted, which is ADR 0031 § 2's
capture-by-value and is what makes the closure safe to outlive the caller-staged cell. An object
literal writing one field name twice is **E0494** at the checker rather than a panic in `mwl-ir`.
`python tools/holes.py` is at **27 sites, 7 items**.

`verify.py` 6 of 6 green — conformance **593**, differential **166**, 1630 unit tests.
`tools/leak-check.sh` green over a fixture that returns the closure and invokes it after the
staging frame is gone (the capture's retain is the one new refcount edge).

Facts recorded where they belong rather than here: `mwl-ir`'s gap 9 (`crates/mwl-ir/src/lib.rs`)
now says both halves of `&$x` are answered; E0494's reasoning is its own `Code::new` doc comment
in `mwl-diagnostics`; the shape-fields-are-a-set rule is
`mwl_types::expr::literals::check_object_literal`'s doc comment; and `lower_object_literal`'s
assert says in its own `# Panics` that it is an internal-consistency check now.

`[context] adrs` gained `0036 §2` and `0036 §4` this session — both slices rest on them and the
pack printed neither.

## Next group

**The last three refusals in `crates/mwl-ir/src/lower/expr.rs`**, which `holes.py` files under
item 17 because they share the file. One file set: `crates/mwl-ir/src/lower/expr.rs`, with
`crates/mwl-types/src/expr/members.rs` and `crates/mwl-diagnostics/src/lib.rs` (next free code is
**E0495**) for the first slice.

- [ ] **A property access through a `mixed` receiver** — `crates/mwl-ir/src/lower/expr.rs:3351`,
      the `_ =>` arm of `lower_property_access`'s `ExprInfo` match (`:3342`). **Measured this
      session: the assert's own message is stale.** A plain `object` receiver
      (`object $o = new Box(); echo $o->name;`) and a shape receiver naming a field it does not
      have (`var $s = {a: 1}; echo $s->b;` — an uncaught throw, not a panic) both lower and run
      today through ADR 0036 § 4's name-keyed fetch. The one route that still reaches the panic is
      `mixed $m = new Box(); echo $m->name;`. Decide between reusing that § 4 fetch behind an
      untag — a `mixed` is `Ty::Tagged` — and refusing with E0495 naming ADR 0007 § 2's "`mixed` is
      the one unchecked position"; PHP accepts the read, which argues for the lowering.
- [ ] **An `instanceof` whose right-hand side resolved to no class** —
      `crates/mwl-ir/src/lower/expr.rs:3872`. Same shape of question: find the source spelling that
      reaches it before deciding whether it is a lowering or a diagnostic.
- [ ] **An `as` whose target `closed_literal_set` cannot build** —
      `crates/mwl-ir/src/lower/expr.rs:4119`, ADR 0047's literal atoms. This one still says "does
      not lower", so it is a real hole rather than a stale message.

## Backlog

- Item 7's five sites in `crates/mwl-ir/src/lower/stmt.rs` (a nullsafe property assignment target,
  an unset base that erased) — `python tools/holes.py --item 7`.
- Item 1's 11 sites are catch-alls reachable only by a pair no widening exists for — plan, *Open now*.
- Two unattributed sites in `crates/mwl-codegen/src/ty.rs` that no item anchors — `holes.py`.
- 17 of the 32 named `.mwlt` cases are still to write — `python tools/loop.py --list`.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
