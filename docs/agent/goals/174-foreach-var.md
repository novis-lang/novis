---
milestone: post-parity
position: last
---
# Loop goal 174 — `var` is a `foreach` binding's type wherever a local allows it

A `foreach` binding may write `var` where it writes a type today, and everything
`rule:types/var-inference` says of a `var` local is then true of it: `foreach ($users as var $u)`
gives `$u` the subject's element type, fixed for good, and `foreach ($stock as var $name => var
$qty)` gives `$name` a `string`. Once this goal is green the most common binding after a local no
longer repeats a type its subject already states, and `foreach` stops being the one loop header
`var` cannot appear in — `for (var $i = 0; …)` has parsed since `rule:iteration/for-init-clause`.

Nothing else moves. A written type is still accepted everywhere it is today, and is still what a
binding that *widens* writes — `Animal $a` over an `array<Dog>`.

## Why here

It needs nothing that is not built: the checker computes the subject's element type before it looks
at the binding (`crates/nvs-types/src/locals.rs`'s `StmtKind::Foreach` arm calls `foreach_source`
first), so `var` takes a type that is already in hand. Nothing after it waits on it.

It sits behind goal `plain-comments` because it writes new programs under `docs/examples/` and
`tests/hostile/`, and that goal's gate is what holds their comments to `AGENTS.md` § *Text an end
user reads* from the first write. It says `position: last` for the reason that goal does:
`--emit-goals` keeps the pinned tail behind whatever it appends, and a goal without the pin sitting
behind one with it is what `python tools/chain.py --check` refuses. In front of goal `ci-green`,
because that goal proves the tree the run ends on and this one still changes it.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/types/var-inference.md` — scoped to a local declaration; it gains the `foreach`
  binding, stated by the same four properties.
- `docs/rules/iteration/foreach-subjects.md` — says nothing of how a binding gets its type; it
  gains one sentence and a pointer to `rule:types/var-inference`.
- `docs/rules/types/declaration.md` — "every binding site carries a written type"; `var` in a
  loop header is a second place the type is taken rather than written, and the sentence that
  already makes room for the `var` local makes room for this one.
- `docs/reference/lang/40-statements.md` § *`foreach`* — "Every binding declares its type", its
  grammar line, and the refused-forms list further down that repeats it.
- `docs/reference/lang/60-iteration.md` § *What `foreach` walks* — "The binding is typed."
- `docs/reference/lang/20-types.md`'s `var` bullet — names a local only.
- `docs/reference/lang/10-programs.md`'s annotated program — its comment says every `foreach`
  binding declares its type.
- `docs/reference/tools/30-php-differences.md` — "Every binding declares a type once", twice.
- `docs/spec/00-overview.md` § 3.2 *`foreach` with typed bindings*.
- `docs/agent/playbook.md`'s bullet "A `foreach` binding with no type is a syntax error" — still
  true of `as $v`, and it says what to write instead, which now has two answers.
- `crates/nvs-syntax/src/parser/stmt.rs:@parse_foreach_binding` — the diagnostic's own text says
  every binding declares a type; it names `var` as the other thing that may stand there.
- `crates/nvs-syntax/src/ast.rs:@ForeachBinding` and `crates/nvs-ir/src/lower/mod.rs:@binding_ty`
  — both doc comments call the type mandatory and read `None` as error recovery.

`docs/novis.md` and `docs/ground-rules.md` are generated and are regenerated, never edited.
`docs/decisions/0037.md`'s scope line is **not** edited either: a record is frozen, and the new
record's `changes: modifies` is what says it was overtaken.

## Stage 1 — the floor

Goal `plain-comments`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the binding parses and checks, the keystone

One file set: `crates/nvs-syntax/src/parser/stmt.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-types/src/locals.rs`, `crates/nvs-types/src/expr/iteration.rs`.

- **The parser.** `parse_foreach_binding` accepts `var` where `can_start_type()` is tested today,
  for the key and for the value, after `inout` when there is one. `ForeachBinding` says which of
  three things was written — a type, `var`, or nothing — because `ty: None` already means "omitted,
  and the parser reported it", and `check_foreach_key` and `binding_ty` both read it that way. An
  untyped `as $v` is still the parse error it is.
- **The checker.** With `var`, the value binding's type is `ForeachSource::value_ty()` and the key
  binding's is `string`. A source with no element type — a `mixed` or `iterable` subject, or one
  already diagnosed — gives `mixed`, which is what a `var` local over a `mixed` initializer gets.
  The element type passes through `reject_void_or_never_binding` exactly as a `var` local's does.
  `check_foreach_inout`'s exact-element-type obligation holds by construction and is not skipped.
- **The refusal.** A subject that is a bare array literal under a `var` binding is
  `E_VAR_ARRAY_LITERAL_NEEDS_TYPE`, the code the local has, with a help line that names the two
  fixes: write the binding's type, or give the literal a typed local. `foreach (f([1, 2]) as var
  $x)` is fine, for the reason `var $x = f([1, 2]);` is.
- **A key over a cursor is still `E_FOREACH_KEY_ON_CURSOR`**, `var` or not.

Pinned by one accepting case and one refusing case under `tests/conformance/`, and by Rust tests in
`crates/nvs-syntax/src/parser/tests/stmt.rs` and `crates/nvs-types/tests/locals.rs`.

## Stage 3 — the loop lowers and the editor shows the type

One file set: `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/src/lower/control.rs`,
`crates/nvs-lsp/src/hints.rs`, and whichever of `nvs-lsp`'s `semantic.rs` and `definition.rs` and
`nvs-hir`'s `requires.rs` read `ForeachBinding::ty`.

- **`nvs-ir`.** `binding_ty` lowers the *written* type off the AST and panics on `None`. Under
  `var` the binding's representation is the checked element type erased — the checker records it
  against the binding's span and lowering reads it back, which is how `ForeachDrive` already
  crosses that boundary. A Rust test in `crates/nvs-ir/src/lower/tests.rs` pins that `as var $v`
  and the same loop with the type written lower to the same IR: that is this feature's whole
  performance claim, and it is why no bench is written.
- **`nvs-lsp`.** The inlay hint a `var` local carries — `: T` after the name — appears on a `var`
  `foreach` binding, key and value. Every other walker that reads `ForeachBinding::ty` is taught
  the third state rather than left to treat `var` as a missing type.
- **`nvs fmt`** prints a `var` binding as written. `python tools/verify.py`'s `nvs-fmt` step over
  the new examples is what proves it.

## Stage 4 — the feature proofs and the reference

No new reference heading, so no new feature on the roster: `var` in a `foreach` header is part of
`lang:statements/foreach` and `lang:iteration/what-foreach-walks`, and those two features' proofs
grow.

**What is new.**

- One new example under `docs/examples/lang/statements/foreach/` whose subject is a call, so the
  reader sees a type that is written nowhere in the file arrive on the binding.
- One new numbered step in `tests/hostile/lang/iteration/what-foreach-walks/`: a `var` binding
  over a `tainted` element stays `tainted`, so `var` is not a way to lose a qualifier.
- The `covers:` markers on Stage 2's two cases name both features.

**What is already on disk, and is brought up to date so the feature is documented where a reader
looks for it.** This is the user's instruction, and it is a stage of its own weight, not a tail.

- Every document Stage 0 lists, each rewritten whole. The reference chapters run their fenced
  programs, so each chapter that gains the sentence gains a `var` header that executes: § *`foreach`*
  shows the value form, the key-and-value form and `inout var`, and says in one sentence when a
  written type is still the right thing — a binding wider than the element.
- The `about.md` of `lang:statements/foreach`, `lang:iteration/what-foreach-walks` and
  `lang:types/every-binding-has-a-type`, each inside its own word band.
- The existing examples of those three features, and of the other features under
  `docs/examples/lang/iteration/`: in each feature at least one program is rewritten to a `var`
  header and at least one keeps a written type, so a reader who opens either feature meets both
  forms. A program's `.out` does not change, because `var` changes nothing a program prints; one
  that does is a finding, not a re-bless.
- `python tools/reference.py` regenerates `docs/novis.md`, and the rulebook's generated files are
  regenerated by the tool the wrap already runs.
- Last, one search closes the stage: `grep -rn -i "binding declares\|binding is typed\|declares
  its type\|declare a type" docs crates --include=*.md --include=*.rs`, read line by line. Every
  hit is either true as it stands, rewritten, or under `docs/decisions/`.

## Standing decisions

- **The rule is one sentence, and it is the user's: a `foreach` binding may write `var` wherever a
  local declaration may, and means what it means there.** Every question a session meets is
  answered by asking what a `var` local does. Where the two genuinely cannot agree, the local's
  behaviour wins and the difference is recorded in the rule fragment.
- **One new decision record and no other number.** It `modifies` `types/var-inference`,
  `iteration/foreach-subjects` and `types/declaration`, and states the tradeoffs: nothing in
  performance or memory, since `var` is gone before any IR exists; a shorter header for developers;
  one more position where a type is not visible in the source, which the inlay hint answers in an
  editor and nothing answers in a diff.
- **`catch`, destructuring, parameters, properties, constants, closures and returns are not
  reopened.** They keep their written types. A `catch` type is the filter, not an annotation, and
  the others have no single source expression to take a type from.
- **No new error code** unless the refusal's message cannot be made true of both sites; the local's
  code is reused with a help line that fits a loop header.
- **A `mixed` subject does not lower today** — `lower_foreach` has no `ForeachDrive` for one, and
  that is a known gap of `nvs-ir` this goal neither closes nor widens. `var` over one is a checker
  rule and is pinned by a checker test, not by a program that runs.
- **The example sweep is bounded, and it is not a house-style change.** The features Stage 4 names
  are rewritten; the several hundred other programs with a `foreach` in them keep their written
  types, because a program teaching `Core\Db` is not improved by also teaching `var`, and a written
  element type tells a beginner what the loop holds. Neither form is preferred in new programs. A
  session that thinks the whole tree should move puts that in the handoff's `## Backlog` for the
  user and does not start it.
- **No bench.** The IR-equality test of Stage 3 is the proof that there is nothing to measure.
- **Every comment in a new `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write**, and `python tools/dossier.py --comments <paths>` is run over
  them before the wrap.
