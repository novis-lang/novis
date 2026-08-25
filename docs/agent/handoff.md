# Handoff

## State

**The acceptance gate reaches its real frontier for the first time in this run.** Stage 0 had been
failing since session 0014 on one name: `an_autoload_declaration_resolves_a_name_to_its_file`, which
`loop-goal.toml:139` requires and which existed nowhere — ADR 0061 was built and pinned, but never by
a test with that name. It is now `mwl_hir::requires`' test module, asserting the fixpoint's *output*
(the `AutoloadMap` names the file, and the walk loaded it) rather than repeating the symbol-arrival
assertion beside it. Every other Stage 0 test already existed and passed.

**`python tools/loop.py --goal-only` now passes Stages 0, 1, 2 and six of Stage 3's seven fixtures,
and stops here:**

    NOT GREEN: native examples/collect.mwl [3 Core Part I]: exit 1 -- error[E0102]: expected an expression
    examples/collect.mwl:13:37   var $seen = new Core\ObjectSet<Tag>();

So the whole `Core` roster below `collect.mwl` runs, and **the one thing between the loop and Stage 4
is a parser hole, not a library one.** Beyond it, Stage 4's two counts are the far wall: conformance
is **369 of 600**, differential **86 of 150** (it has not moved at all this run), and
`every_part_one_spec_member_is_registered` still does not exist.

Spec § 12's `Core\Uri` is its percent-encoding half only; `uri.rs`' own docs own why `isValid` waits
on the RFC 3986 dependency pick. `Tag::Bytes` still blocks `Random::bytes`, `Core\Encoding`,
`Core\Bytes` and `Hash::*` — `collect.mwl:25-26` asserts a sha256 and a base64, so that is on the
acceptance path rather than in the backlog, and it is a **design call the loop is pre-authorized to
settle** (`loop-goal.md` § *Standing decisions*): `mwl_ir::Ty::Bytes` and `registry::CoreTy::Bytes`
both already exist, and what is missing is a runtime producer — `mwl_runtime::Tag` has no `Bytes`
row, so nothing can construct a fresh one (`value.rs:49` currently spends `Tag::Str` on both).

## Next group — `new Core\X<T>()`, the one expression `collect.mwl` stops on

**Shared file set:** `crates/mwl-syntax/src/parser/expr.rs` (`parse_new` at `expr.rs:1207`,
`parse_call_type_args` at `expr.rs:744` — the `<...>` list a *static call* already reads, and the
tie-break comment above it is the one this reuses), `crates/mwl-syntax/src/ast.rs`
(`ExprKind::New` at `ast.rs:1197`, and `StaticCall::type_args` at `ast.rs:741` is the field shape to
copy), `crates/mwl-types/src/expr/calls.rs` (`check_new_target` at `calls.rs:368`, reached from
`expr/mod.rs:307`), `crates/mwl-ir/src/lower/expr.rs` (`lower_new` at `expr.rs:2303`), and
`tests/conformance/lang/`.

- [ ] **`new Target<T, U>(...)` parses**, carrying its `type_args: Vec<Type>` on `ExprKind::New`
      exactly as `StaticCall` and `MethodCall` already do. `parse_call_type_args` is the parser and
      its ambiguity trade is already argued at `expr.rs:728-744`; a target that declares no type
      parameter must refuse a written list the way a `Core` member with no `written()` already does.
- [ ] **The checker binds them**, in `check_new_target`: a `Core`-owned generic class binds its
      variables from the written list, and `ObjectMap<K, V>`'s two arities are the case that proves
      the list is positional. Type variables stay compiler-owned — a *user* class with `<T>` is still
      refused, per `loop-goal.md` § *Standing decisions*.
- [ ] **`lower_new` erases them**, which is the whole run-time cost: ADR 0047 § 5's rule already
      applies — a type argument is a checker fact, so the instance lowered is the same
      `registry::CoreTy::Instance` shape `Core\Uuid` and `Core\Time\Instant` already lower to.
- [ ] **Conformance cases under `tests/conformance/lang/`**, in the same slice as the rows
      (`playbook.md` § *Writing a test case*): one that constructs each arity, one that refuses a
      written list on a class with no type parameter, and one that refuses a user-declared `<T>`.

## Backlog

Ordered by what the acceptance test blocks on, not by section number — the first three are all on
`collect.mwl`'s path:

- **Spec § 9's three collections** (`ObjectMap`, `ObjectSet`, `Heap` — 23 rows, spec
  `01-core-library.md:650-673`), over the `registry::CoreTy::Instance` shape `uuid.rs:115` models.
  Needs the group above first.
- **A `bytes` producer** — the `Tag::Bytes` decision above, then § 7's seven rows and `Hash::*`.
- **`Uri::parse`/`isValid`, `Csv`, `Validate`, `Out::capture`** — the rest of `collect.mwl`'s roster;
  `Uri`'s own gap 1 owns the dependency pick.
- **Stage 4's counts are their own work, not a side effect.** 231 conformance and 64 differential
  cases are owed, and this run has produced ~2 and 0 per session respectively. A case over an
  *already registered* member costs no new code and shares one file set, so these belong in
  dedicated groups of a dozen rather than two at a time behind a member slice.
- **`every_part_one_spec_member_is_registered`** (`loop-goal.toml:339`) — reads the member rows out
  of the spec's §§ 1-12 and fails naming every one with no registry entry. It is the loop's own
  definition of done and does not exist yet; `crates/mwl-stdlib/tests/conformance_coverage.rs` is
  the file it belongs in and the parsing model to copy.
- `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain
  (`tools/check-migration.py`).

`orient.py` printed everything this session needed.
