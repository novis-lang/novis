---
milestone: post-parity
position: last
---
# Loop goal 180 — `var` infers an array literal whose elements all have one type

`var $ids = [1, 2, 3];` declares an `array<int>`, where today it is `E_VAR_ARRAY_LITERAL_NEEDS_TYPE`
and the author writes `array<int> $ids = [1, 2, 3];`. The rule is one sentence: **an array literal
under `var` takes the type `array<T>` when every element has the same type `T`, and every other
literal is still refused, with a help line that names the type to write.** An element's type is
exactly the type `var $e = <element>;` gives, so `1` is `int`, a call is its return type, and a
`mixed` element is `mixed`. Once this goal is green the most common array declaration no longer
repeats a type its elements already state, and a literal whose elements disagree is still a type
somebody writes down.

```nvs
var $ids = [1, 2, 3];                  // array<int>
var $byKey = ["a" => 1, "b" => 2];     // array<int>: keys are always strings
var $grid = [[1, 2], [3]];             // array<array<int>>
var $all = [...$ids, ...$more];        // array<int>, when both are array<int>

var $mixed = [1, "a"];                 // E0414, help: `array<int|string> $mixed = [1, "a"];`
var $maybe = [1, null];                // E0414, help: `array<?int> $maybe = [1, null];`
var $people = [new User(), new Admin()];   // E0414, even when Admin extends User
var $empty = [];                       // E0414, help: `array<T> $empty = [];`
```

Nothing else moves. A literal in any other position with no target — an `echo` argument, a bare
expression statement — types exactly as it does today, and a written `array<T>` is accepted
everywhere it is today.

## Why here

It needs nothing that is not built. `check_array_literal` already checks every element, and the
`var` arm in `crates/nvs-types/src/locals.rs` already computes an initializer's type with no
expectation; what is missing is one function that turns the element types into `array<T>` or a
refusal.

It sits behind goal `foreach-var` because that goal gives a `foreach` binding `var` and copies this
refusal onto a bare array-literal subject, and its standing decision is that a `var` binding means
what a `var` local means. This goal changes what the local means, so it changes both sites at once,
and it can only do that once the loop header exists. It says `position: last` for the reason that
goal does, and sits in front of goal `ci-green` because that goal proves the tree the run ends on
and this one still changes it.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/types/var-inference.md` — its third paragraph refuses every bare array literal; it
  states the one-type rule and the refusal that remains.
- `docs/rules/types/arrays.md` — "Array literals are checked against the target type, never inferred
  and then compared", and "which is why `var` refuses a bare one". A literal still has a target
  everywhere a type is written; under `var` it has none and is inferred by the one-type rule.
- `docs/rules/ide/no-compile-path-calls-the-synthesis.md` — "No compile path calls it" is false
  once `var` calls it. The fragment is rewritten to say which one compile path does, and why the
  union answer it declines to attach to a binding is still declined: a union is shown in the help
  line and written by a person, never attached to a binding.
- `docs/rules/ide/narrow-an-annotation-to-its-literal.md` — read against the rewritten fragment
  above; it changes only if it restates the old sentence.
- `docs/rules/iteration/foreach-subjects.md` and `docs/rules/types/var-inference.md`'s `foreach`
  half, as goal `foreach-var` left them — a bare literal subject under a `var` binding follows the
  local.
- `docs/reference/lang/20-types.md`'s `var` bullet and `docs/reference/lang/30-expressions.md`'s
  array-literal text.
- `docs/reference/tools/30-php-differences.md`, where it names the `var` refusal.
- `crates/nvs-types/src/locals.rs`'s comment on the `var` arm, and
  `crates/nvs-diagnostics/src/lib.rs`'s entry for `E_VAR_ARRAY_LITERAL_NEEDS_TYPE`.
- The playbook bullets under `docs/agent/playbook/writing-a-test-case/` that name the refusal.

The same search closes the stage as it opened it:
`grep -rn "E_VAR_ARRAY_LITERAL\|E0414\|refuses a bare\|bare array literal" docs crates data`, read
line by line. Every hit is either true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, `docs/rules/*.md` chapters and `website/` are generated and
are regenerated, never edited. `docs/decisions/0037.md` is **not** edited: a record is frozen, and
the new record's `changes: modifies` says it was overtaken.

## Stage 1 — the floor

Goal `foreach-var`'s whole acceptance list, carried in by the goal switch. Never traded.

Two of its checks name the refusal this goal narrows —
`foreach_var_over_a_bare_array_literal_is_refused` and
`tests/conformance/reject/a-foreach-var-binding-refuses-a-bare-array-literal-subject.nvst`. Both
keep their names and stay green: the literal in each becomes one whose elements differ, which is
still refused with the same code. That edit is the new record's, not a trade.

## Stage 2 — the literal infers, the keystone

**Does:** Infers `array<T>` for a one-type array literal under `var`, and refuses every other literal
with a help line naming the type.

One file set: `crates/nvs-types/src/expr/literals.rs`, `crates/nvs-types/src/locals.rs`,
`crates/nvs-types/src/expr/iteration.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- **The decision record**, written first, from § *Standing decisions*. It `modifies`
  `types/var-inference`, `types/arrays` and `ide/no-compile-path-calls-the-synthesis`, and those
  fragments are rewritten whole with it.
- **The synthesis.** One function in `crates/nvs-types/src/expr/literals.rs`, a sibling of
  `check_array_literal`: given the literal, it returns `array<T>` when every element's type is the
  same interned type, or the reason it does not — two types (with their canonical union, for the
  help line), an empty literal, or an element it could not type. A nested literal is inferred by
  the same function first, so `[[1], [2, 3]]` is `array<array<int>>` and `[[1], ["a"]]` is two
  types. A spread element contributes its source's element type. Keys are checked as they are today
  and take no part in `T`.
- **The `var` arm.** `crates/nvs-types/src/locals.rs`'s `None` arm calls the synthesis for a bare
  literal. On success the literal is then checked with `array<T>` as its target, the same call a
  written `array<T> $x = [...]` makes, and the binding is declared at `array<T>`. On refusal it is
  `E_VAR_ARRAY_LITERAL_NEEDS_TYPE`, as now, with a message saying which case it is and a help line
  that names the written declaration: the union for two types, `?T` when the second type is
  `null`, and `array<T>` with the `T` left for the author when the literal is empty.
- **The `foreach` subject.** A bare literal subject under a `var` binding
  (`crates/nvs-types/src/expr/iteration.rs`) goes through the same function, so
  `foreach ([1, 2] as var $n)` gives `$n` an `int`.
- **Every other no-target position is untouched.** A guard test holds that `var`'s arm and the
  `foreach` subject are the only callers of the synthesis.

Pinned by `tests/conformance/lang/var-infers-an-array-literal-whose-elements-have-one-type.nvst`
(runs, prints the values) and
`tests/conformance/reject/var-refuses-an-array-literal-whose-elements-differ.nvst` (every refusing
shape above, one diagnostic each). `tests/conformance/lang/var-refuses-a-bare-array-literal.nvst`
is folded into the second and deleted, and `data/rules/types/var-inference.json` and
`data/rules/ide/no-compile-path-calls-the-synthesis.json` name the new cases. The Rust tests are in
`crates/nvs-types/tests/locals.rs`, where `var_rejects_a_bare_array_literal_initializer` is
rewritten into the named tests of this stage's check.

## Stage 3 — the later write, the editor and the lowering

**Does:** Points a failed write at the `var` line that fixed the element type, shows the inferred
type in the editor, and proves the lowering is the written type's.

One file set: the `nvs-types` module that reports a write against an array's element type (`bun nv
peek --locate` from the diagnostic a `$a[] = 2.5` on an `array<int>` reports today),
`crates/nvs-lsp/src/hints.rs`, `crates/nvs-ir/src/lower/tests.rs`.

- **The later write.** `var $prices = [10, 20]; $prices[] = 12.5;` is an error, as it would be with
  the type written. Its help line names the `var` line and the declaration that would accept the
  write — `array<int|float> $prices = [10, 20];`. If a `var $n = 1; $n = 2.5;` mismatch already
  carries a note pointing at the `var` line, this reuses it; otherwise both gain it here, since the
  cause is the same.
- **`nvs-lsp`.** The inlay hint a `var` local carries shows `: array<int>` on an inferred literal.
  Nothing new is built if the hint already reads the binding's checked type; the test pins it.
- **`nvs-ir`.** A Rust test pins that `var $x = [1, 2];` and `array<int> $x = [1, 2];` lower to the
  same IR. That is this feature's whole performance claim, and it is why no bench is written.

## Stage 4 — the feature proofs and the reference

**Does:** Adds the tests, examples and reference text for an inferred array literal.

No new reference heading, so no new feature on the roster: this is part of
`lang:types/every-binding-has-a-type` and `lang:types/array-t`, and those two features' proofs grow.

- One new example under `docs/examples/lang/types/array-t/` that declares three arrays with `var`
  and prints them, so the reader sees the element type come from the values.
- One new numbered step in the attack under `tests/hostile/lang/types/every-binding-has-a-type/`:
  a literal mixing a `tainted` string with a plain one is refused, so `var` cannot drop a
  qualifier by mixing elements.
- The `covers:` markers on Stage 2's two cases name both features.
- Every document Stage 0 lists, each rewritten whole. The reference chapter's fenced programs run,
  so § on `var` gains one that executes.
- The `about.md` of both features, each inside its own word band.
- The existing examples of those two features: in each, at least one program is rewritten to
  `var` over a literal, and at least one keeps a written `array<T>`, so a reader meets both forms.
  A program's `.out` does not change; one that does is a finding, not a re-bless.
- `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The rule is the user's: an array literal under `var` is `array<T>` when every element has the
  same type, and otherwise it is refused with the type to write.** Every question a session meets
  is answered from that sentence. No common supertype is ever computed: two classes, even parent
  and child, are two types.
- **An element's type is what `var` already gives that element alone.** No new widening rule is
  written; a literal type, an enum case and a call widen exactly as they do for a `var` scalar.
- **A qualifier is part of the type.** Elements that differ only in `tainted` or `secret` are two
  types and are refused, so no inference can drop or invent a qualifier.
- **An empty literal is refused wherever it stands under `var`**, nested included. `array<never>`
  is never inferred for a binding.
- **The code stays `E_VAR_ARRAY_LITERAL_NEEDS_TYPE` (`E0414`).** Its message and help change; no new
  code, unless a refusal case cannot be made true under that name.
- **One new decision record and no other number.** It states the tradeoffs: nothing in performance
  or memory, since the type is fixed at compile time and lowers as the written one does; less to
  write for the most common array declaration; one more place a type is not visible in the source,
  which the inlay hint answers in an editor and nothing answers in a diff; a later write of a new
  element type is an error that has to be read back to the `var` line, which Stage 3's help line
  answers.
- **The example sweep is bounded, and it is not a house-style change.** Stage 4's two features are
  rewritten; the other programs with `array<T> $x = [...]` keep it. Neither form is preferred in new
  programs. A session that thinks the whole tree should move puts that in the handoff's
  `## Backlog` for the user and does not start it.
- **No bench.** Stage 3's IR-equality test is the proof that there is nothing to measure.
- **Every comment in a new `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before
  the wrap.
