---
milestone: post-parity
---
# Loop goal 67 — `is` is the one type test, and `instanceof` is gone

Novis has one type test, `$x is T`, and it asks every question the two operators used to split:
`$x is Request` is the class test, `$x is $cls` is the dynamic class test against a `class<T>` value
and narrows to `T`, and `int $n; $n is Request` compiles and folds to `false` like every other test
the declaration settles. The spelling `instanceof` is refused where it is written, naming `is`; the
descriptor walk it lowered to keeps running under a name that is not the keyword. The decision, the
divergence from PHP, and the fact that **neither keyword is compared with PHP again** are one record
and one rule, written first — and at the end of the goal the word `instanceof` survives only in that
rule, in the refusal that names it, and in the frozen records.

## Why here

Directly after goal `decided-closures` and before goal `gap-zero`. It needs goal `core-class-tests`,
which laid the descriptor path a `Core` class is tested through — `is Core\Time\Date` runs on exactly
that path, so nothing here builds a second one. It sits before `gap-zero` because that goal's gate
declares the tree clean of owed work and the register closed, and a construct deleted *after* the
declaration would reopen a hundred sites the gate had just passed over. It sits before `dossier`
because that goal generates one goal per feature from the rulebook, and a rule naming `instanceof`
would generate a goal for a spelling that no longer exists. Goal `decided-closures` builds what the
register says and touches none of these files, so the two share nothing and the order between them
is only that this one's sweep is cheaper on a tree whose gaps are already closed.

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong, each with its home. None is edited ahead of the stage that
owns it; the list exists so a session does not rediscover one and treat it as a gap.

- `php-migration/is-takes-pattern-matchings-type-patterns` § *Why `$x is $cls` is refused* said a bare
  variable on the right of `is` is a top-level capture in PHP. The RFC as read for this goal — version
  0.9, *in discussion* — permits binding only inside an object or array pattern and refuses a bare
  variable as a whole pattern. The rule is deleted in stage 2 with the sentence in it.
- `rule:types/type-test`'s *three refusals* table, row `$x is $cls` (`E0812`), and its paragraph
  opening *That differs from `instanceof`*. Rewritten in stage 2.
- `rule:types/narrowing`'s first sentence counts five spellings; there are four. Stage 2.
- `rule:types/class-reference-sites`'s third row spells the site `$x instanceof $cls`. Stage 2.
- `rule:php-migration/let-and-is-are-reserved`'s last paragraph says *`is` and `instanceof` test*.
  Stage 2, where `instanceof` joins the paragraph as a refused spelling of the empty kind.
- `rule:php-migration/a-declared-type-answers-before-the-program-runs`: its title, its `instanceof`
  half and its `divergesFromPhp` sentence naming `1 instanceof Box` and `E0497`. Stage 2, reduced to
  the `->` half.
- `rule:php-migration/every-divergence-is-deliberate-and-listed`'s clause *`->` and `instanceof` before
  the program runs*. Stage 2.
- `crates/nvs-syntax/src/parser/ty.rs:@reject_value_in_type_test`'s help, which sends the reader to
  `instanceof` for the dynamic test. Deleted in stage 3 with the refusal.
- Goal `core-class-tests`'s descriptor work is on `main` and is what `is Core\Time\Date` runs on:
  `nvs_types::expr::members::testable_class_name`, the `emit.rs` guard that admits a `Core` class's
  process-wide descriptor, `nvs_stdlib::class_has_instances`, and the case
  `tests/conformance/lang/instanceof-answers-for-a-core-class.nvst`. Stage 3 folds the first into the
  type resolver's own answer for a `Core` class written as a type, stage 4 keeps the guard under the
  renamed emitter, and stage 6 rewrites the case to `is`.

## Stage 1 — the floor

Goal `decided-closures`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never
traded.

## Stage 2 — the record and the rulebook, first

**One slice, before any code changes**, so the tree states the decision from its first commit. The
record is written from the shape in [conventions.md](conventions.md) § *A decision record*, claims
the next free number, and its `changes:` block and the rules' `because` lists are written together.

- **The record** — `docs/decisions/NNNN.md`, *`is` is the one type test, `instanceof` is a refused
  spelling, and neither keyword is compared with PHP again*. Its `Context` freezes what was read once
  and is never read again: PHP's Pattern Matching RFC at version 0.9, in discussion, which spells the
  class test `$foo is Request` as an `instanceof` equivalent, refuses a bare variable as a whole pattern,
  spells pinning `^$var` with `===`, and does not address a dynamic class name; and PHP 8.6's passed
  deprecation of `is` as an identifier, whose stated motivation is that RFC. Its `Decision` is the
  four sentences of the target above. Its `Alternatives rejected` are the two this goal replaced: a
  `Core\Reflect::isInstance` call for the dynamic test, and keeping `instanceof` for that form alone.
  Its `Revisiting` names **one** trigger and no PHP one: Novis wanting pattern syntax of its own, which
  is a design question opened by a record of its own, decided on Novis's priorities.
- **The new rule** — `php-migration/one-type-test`, created by the record, `status: designed` until
  stage 7 flips it. It states, as the divergence's one home: `instanceof` is refused where it is
  written and the diagnostic names the rewrite; `$x is $cls` is the dynamic class test against a
  `class<T>` and narrows to `T`, where PHP's RFC refuses the spelling; a `string` or anything else on
  the right of `is` that is a value and not a `class<T>` is `E0496`, with help naming `as class<T>`;
  the converter (M11's, over `php-rs-parser`) rewrites `$x instanceof C` to `$x is C` and leaves
  PHP's string and object right-hand sides to that refusal; and **no session re-reads the RFC, tracks
  its vote, or compares either keyword with PHP again** — the shapes the deleted rule listed as
  reserved (object patterns, array patterns, comparison patterns, pinning, `match ... is {`) keep
  refusing as syntax Novis does not have, with no RFC named in the diagnostic, and any future pattern
  syntax is Novis's own design question. Its `divergesFromPhp` field is the one sentence
  `divergences.md` prints.
- **The rule that goes** — `php-migration/is-takes-pattern-matchings-type-patterns`: its fragment
  deleted, its `php-migration.json` entry removed, every `seeAlso` that names it swept. Record 0150
  stays `accepted`, since it also created `types/type-test`; if `changes:` has no key for a removal,
  § *Decision* of the new record states it in one sentence and the handoff says so.
- **The six the record modifies**, each fragment rewritten to the language this goal ships, each
  `because` gaining the record's number: `types/type-test` (the third refusal becomes the value form's
  acceptance and the `E0496` case; the *differs from `instanceof`* paragraph goes; the sentence
  *every other row is one tag comparison, or the descriptor walk* names the walk without the keyword),
  `types/narrowing` (four spellings; `$x is $cls` narrows to `T`), `types/class-reference-sites` (third
  row `$x is $cls`), `php-migration/let-and-is-are-reserved` (`instanceof` is a refused spelling with a
  living rewrite, beside `let`'s empty kind; the living-spellings sentence reads *`is` tests*),
  `php-migration/a-declared-type-answers-before-the-program-runs` (title and body reduced to `->`),
  `php-migration/every-divergence-is-deliberate-and-listed` (the `->` clause alone, plus one clause
  citing the new rule).
- `python tools/rules.py --render`, then `--check` and `--render --check`, then `python
  tools/records.py --check`.

## Stage 3 — the front end

One file set: `crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-syntax/src/parser/ty.rs`,
`crates/nvs-syntax/src/ast.rs`, `crates/nvs-syntax/src/token.rs`, `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-types/src/expr/type_test.rs`, `crates/nvs-types/src/locals.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- **The grammar.** `ExprKind::InstanceOf` is deleted. `ExprKind::TypeTest` carries
  `against: TestOperand`, with `TestOperand::Type(Type)` and `TestOperand::Value(Box<Expr>)`. After
  `is`, a `$variable` token starts the value arm, parsed at the `|>` level exactly as the old
  `instanceof` operand was, so `$x is $this->cls` and `$x is $cls` are both values; **every other
  token starts a type**, which keeps a DNF type's opening `(` a type. `parse_instanceof` becomes
  `parse_type_test`; `parse_type_test_operand` and `reject_value_in_type_test` go.
- **The refusal.** `Keyword::InstanceOf` stays a token so the spelling can be named. Met where a
  binary operator may stand, it reports a new code beside `E_RESERVED_FOR_FUTURE_USE` — *`instanceof`
  is not a Novis operator; the type test is `is`* — with help *write `$x is Request`; a class
  reference on the right is `$x is $cls`* — and consumes the right operand so the site costs one
  diagnostic. Nothing else about the token changes; it was never a name.
- **The checker.** `infer_type_test` gains the value arm: the operand must type as a `class<T>`, and
  anything else goes through `reject_dynamic_class_name` — `E0496`, the one report `new $v()` and
  `$v::f()` already share — so the site records `ExprInfo::ClassRefTest { base }`, `base` being `T`.
  A subject whose declared type can hold no object records `SettledTypeTest { answer: false }` and no
  diagnostic, as every other settled test does. `infer_instanceof`, `testable_class_name`,
  `testable_core_class`, `instanceof_residue`, `instanceof_test` and `ExprInfo::InstanceOf` are
  deleted; the `is` residue in `locals.rs` narrows the value form to `T` on the true edge. A `Core`
  class or an enum on the right of `is` is whatever the type resolver already says of that name as a
  written type — no `is`-specific refusal is added for either.
- **The codes.** `E0496`'s doc comment is rewritten to its three sites and nothing else. `E0497` and
  `E0812` are retired: their constants and doc comments deleted, their numbers never reassigned.
- **Guard tests**, named in the `.toml`: the parser refuses `instanceof` naming `is` and parses a
  variable after `is` as a value; the checker narrows the value form to its base, refuses a non-class
  reference value, and folds a class test over a subject that holds no object.

## Stage 4 — the lowering, the codegen and the runtime primitive

One file set: `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/lower/exception.rs`,
`crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/print.rs`, `crates/nvs-codegen/src/emit.rs`,
`crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/object.rs`, `crates/nvs-runtime/src/lib.rs`.

- `lower_instanceof` is deleted; `lower_type_test` gains the value arm, lowering the operand at
  `Ty::ClassDesc` and emitting `TestedClass::Descriptor`, with the subject's release rule unchanged.
- **The primitive is renamed so the keyword names nothing in `crates/`.** The instruction is what
  `is C`, `is $cls`, `is callable`, `is iterable` and the `catch` chain all emit, so it is named for
  the question it asks: `InstKind::InstanceOf` becomes `InstKind::ClassTest`, `RuntimeSig::InstanceOf`
  becomes `ClassTest`, `Sigs::instanceof` becomes `class_test`, `emit_instanceof` becomes
  `emit_class_test`, and `nvs_object_instanceof` / `nvs_value_instanceof` become `nvs_object_is_class`
  / `nvs_value_is_class`. `TestedClass` keeps its name. The printer's mnemonic follows. Every doc
  comment that explained itself by *the walk `instanceof` already emits* is rewritten as a whole to
  name the instruction.
- The unit tests in `nvs-ir` and `nvs-codegen` that spell the old names are renamed with them.

## Stage 5 — the library, the LSP and the formatter

One file set: `crates/nvs-stdlib/src/ast.rs`, `crates/nvs-stdlib/src/debug.rs`,
`crates/nvs-stdlib/src/reflect.rs`, `crates/nvs-stdlib/src/lib.rs`, `crates/nvs-lsp/src/definition.rs`,
`crates/nvs-lsp/src/semantic.rs`, `crates/nvs-lsp/src/hints.rs`, `crates/nvs-lsp/src/index.rs`,
`crates/nvs-fmt/`.

- **`Core\Ast`**: the node kind `InstanceOf` leaves the roster; the type-test node carries the value
  operand as a child. `docs/spec/01-core-library.md`'s roster line and
  `rule:core-classes/ast-is-inert` follow, the rule by its own process.
- **`Core\Debug`** and **`Core\Reflect`**: module docs that explained a union or a subclass by way of
  `instanceof` are rewritten to say the same of `is`.
- **The LSP**: the `ExprInfo::InstanceOf` arms in definition, semantic tokens, hints and the index go;
  go-to-definition on the type after `is` resolves through the type node it already resolves for a
  declaration, and on `$x is $cls` through the binding. The `.lspt` case is rewritten and renamed.
- **The formatter** formats `is $cls` as it formats `is T`; the fixture in
  `crates/nvs-fmt/tests/novis_constructs.rs` is rewritten.

## Stage 6 — the tests

Every `.nvst`, `.lspt` and Rust guard test that spells `instanceof` is rewritten to `is`, and the
assertions are kept — this stage changes spellings, never what is proved. A file whose **name** says
`instanceof` is renamed, and in the same slice every path that names it is patched:
`docs/agent/loop-goal.toml`, the `.toml` of every goal after this one, every `guardedBy` and
`covers:` marker. `git grep` of the old basename over the tree is the check, and it is run before the
slice is committed.

- **The refusal cases become answers.** `reject/an-is-test-against-a-variable-names-a-value-not-a-type.nvst`
  becomes `lang/an-is-test-against-a-class-reference-is-the-dynamic-class-test.nvst`: a `class<T>`
  answers for the class it holds and for an implementor, narrows the subject to `T`, and a subject
  that holds no object answers `false` without a diagnostic. `lang/instanceof-refuses-a-subject-that-can-hold-no-object.nvst`
  is deleted; its scalar-subject rows join `lang/an-is-test-whose-answer-the-declaration-settles-is-not-a-diagnostic.nvst`
  as the class row. `lang/instanceof-refuses-a-right-hand-side-that-is-not-a-class.nvst` becomes
  `reject/a-value-that-is-not-a-class-reference-on-the-right-of-is-is-refused.nvst`, pinning `E0496`
  for a `string` variable and keeping its `E0303` row; its enum row moves to the enum's own answer
  case, and its `Core\Str` row pins whatever the type resolver says of that name.
- **A new refusal case**: `reject/instanceof-is-spelled-is.nvst`, pinning the new code and its help.
- **The differential twins go**: `tests/differential/class/instanceof-matches-php.nvst` and
  `tests/differential/lang/instanceof-through-an-erased-subject-matches-phps.nvst` are deleted, because
  PHP cannot run `is` and this goal ends every comparison. What they proved is already in
  `class/instanceof-answers-for-every-supertype.nvst` and
  `class/instanceof-through-an-erased-subject-answers-every-tag.nvst`, which are renamed for `is`.
- `core/one-core-value-answers-the-same-from-is-instanceof-and-as.nvst` becomes the `is`-and-`as`
  case, and goal `core-class-tests`'s `lang/instanceof-answers-for-a-core-class.nvst` becomes
  `lang/an-is-test-against-a-core-class-answers-like-a-declared-one.nvst`.
- The Rust guard tests in `crates/nvs-types/tests/`, `crates/nvs-codegen/tests/` and
  `crates/nvs-ir/src/lower/tests.rs` are rewritten in place.

## Stage 7 — the sweep and the gate

- Every remaining home outside the code: `docs/reference/lang/20-types.md`, `30-expressions.md` and
  `50-classes.md`; `docs/spec/02-php-migration.md`; `docs/agent/playbook.md`'s bullets that name the
  keyword, each edited to what is now true or deleted when its trailer holds;
  `docs/agent/guard-name-debt.md`; goal `core-class-tests`'s row in `docs/agent/goals/README.md`.
  Not touched: `docs/decisions/`, `docs/adr/README.md` and any retired goal's `.md` — frozen history.
- `php-migration/one-type-test` flips to `status: shipped`, and the render is re-run.
- **The gate** is the absence check in the `.toml`: `git grep -i -w instanceof` over `crates/`,
  `tests/`, `docs/reference/`, `docs/spec/`, `docs/rules/` and `docs/agent/playbook.md` finds nothing,
  with exactly these homes excluded because each *has* to spell it — the token table and the parser
  site that refuse it, the diagnostics crate's doc for that code, the parser test and the conformance
  case that pin it, and the `php-migration` chapter that states the divergence.

## Standing decisions

- **The user's decision, settled with this goal, and its scope.** Novis diverges from PHP on `is` and
  `instanceof` for good. `is` is Novis's own operator, judged more understandable than what PHP does or
  plans with the word; `instanceof` is refused naming `is`; `$x is $cls` is the dynamic class test.
  **No session compares either keyword with PHP again**: the RFC is not re-read, its vote is not
  tracked, and a future pattern syntax is a Novis design question opened by its own record. A session
  that finds itself weighing what PHP would do here has left the goal.
- **Migration is mechanical and is M11's.** The converter rewrites `$x instanceof C` to `$x is C`;
  PHP's string and object right-hand sides are left to `E0496`'s help, which names `as class<T>`.
  Nothing in this goal builds the converter.
- **What is deleted is deleted, not deprecated.** No compatibility path, no alias, no `#[deprecated]`,
  no retained `ExprKind`. A retired diagnostic code's number is never reassigned. The primitive is
  renamed rather than kept under the keyword's name, because the gate is that the word names nothing
  in `crates/` but the refusal.
- **The value arm starts with `$`, and nothing else is a value.** A call or a constant on the right
  of `is` is a type and resolves or fails as one; a program that wants a computed class reference
  binds it to a local first. This is the whole grammar question, and it is not reopened.
- **`E0496` is the one report for a value that is not a `class<T>` at any of its three sites**, `is`
  included. No new code for the `is` site.
- **Narrowing the value form to `T` is sound and is taken**: a `class<T>` holds `T` or an implementor,
  so a subject that answers `true` is a `T`. False-edge narrowing stays untaken for every spelling, as
  `rule:types/narrowing` says.
- **A renamed test file is patched everywhere it is named in the same slice**, and the floor is not
  traded by it: a path that moves is the same check under a new name, and `git grep` of the old
  basename over the tree is what proves the rename is whole.
- **What it spends**: nothing. Same instruction, same call, same walk; `rule:programs/memory-priority`
  has no figure to record.
- **ADR slots**: one, the record stage 2 writes, and no other number.
- **Not this goal**: the converter (M11); pattern syntax of any kind; false-edge narrowing; any
  change to `as`.
