# Handoff

## State

**Stage 0 item 8a is done; 8b and 8c are what is left of item 8, and item 8 is all that stands between
the loop and `Core` breadth.** ADR 0061 § 1's two file-scope forms now parse: `autoload 'Prefix' from 'a',
'b';` and `autoload discover 'glob';` reach the AST as `StmtKind::AutoloadDecl`, carrying spans (not
cooked values) exactly as `ExprKind::Str` does, so `mwl_hir`'s existing `cook_quoted` decodes them when
8b builds the map. `autoload` is a reserved word; `discover` and `from` are **contextual**, so no member
named `discover` stops compiling. A path that is not a plain string literal — an interpolated `"$dir"`, a
concatenation, a variable — is `E0123` (`E_AUTOLOAD_PATH_NOT_LITERAL`), reported **once**: the parser
swallows a malformed declaration through its `;` and yields `StmtKind::Error` rather than also failing on
the token it stopped at.

Two doc contradictions this slice had to settle, both recorded where they belong: ADR 0061 § 1's example
wrote `'Framework\'`, which a single-quoted string cannot spell at all (the `\'` escapes the quote), and
the spec's grammar comment called a prefix "backslash-terminated" while its own examples were not. **A
prefix is written without a trailing separator**; matching appends it. ADR 0061 § 1 and
`docs/spec/00-overview.md` § 2 both say so now.

Cost: nothing at run time — the declaration produces no IR (`mwl-ir`'s script-statement walk skips it with
the other file-scope forms). `python tools/verify.py` is green (1374 tests).

## Next group — item 8, ADR 0061's remaining two slices

**Shared file set:** `crates/mwl-hir/src/requires.rs`, `crates/mwl-hir/src/resolve.rs`,
`crates/mwl-diagnostics/src/lib.rs` and `tests/conformance/lang/`. The rule is
[`loop-goal.md`](loop-goal.md) § *Stage 0* item 8; the semantics are
[ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) §§ 1, 2 and 5.

- [ ] **8b — name-to-file resolution as a fixpoint** over the require-graph worklist. The graph is
      [requires.rs:99](../../crates/mwl-hir/src/requires.rs#L99)'s `resolve_program`; the walk that
      harvests literals is [requires.rs:305](../../crates/mwl-hir/src/requires.rs#L305)'s
      `find_require_literals` (add an `AutoloadDecl` arm to its `walk_stmt` at
      [:311](../../crates/mwl-hir/src/requires.rs#L311)), the decoder is
      [requires.rs:653](../../crates/mwl-hir/src/requires.rs#L653)'s `cook_quoted`, and § 1's exact-name
      comparison is already [requires.rs:240](../../crates/mwl-hir/src/requires.rs#L240)'s
      `check_path_case`. The map is the union of every declaration in the `require` chain, longest prefix
      wins, roots probed in declaration order. New E03xx codes (next free **E0315**):
      `E_DUPLICATE_AUTOLOAD_PREFIX` (§ 1), `E_AUTOLOAD_IN_AUTOLOADED_FILE` (§ 1) and
      `E_AUTOLOAD_FILE_SHAPE` (§ 2's one-declaration-per-file rule). § 5's probe trace — the ordered list
      of paths tried, misses included — can be recorded now or left to the cache slice; say which.
- [ ] **8c — the `.mwlt` cases**, in `tests/conformance/lang/`. A class reached only through `autoload`,
      a `discover` glob whose non-`PascalCase` matches are skipped rather than diagnosed, a vendor root
      shadowed by an earlier one, and one `--EXPECTF-ERROR--` case per new diagnostic. A case needing more
      than one file has no precedent in that directory — check `crates/mwl-test`'s module doc for whether
      the format has a second-file section before assuming it does.

## Backlog

- `mwl-ir` gap 12's whole remainder: a `Core`-owned class is exempt from `require_stringable`, so
  `echo $someCoreObject` panics. Saying which `Core` classes stringify is `mwl_stdlib::registry`'s answer.
- `mwl-ir` gap 21: a binding declared at the opaque `object` top has no representation arm, so
  `object $o = $obj;` panics — that gap's text says what a session landing it owes.
- ADR 0094's one uncovered shape: a promoted constructor parameter is no table's property, so nothing
  resolves it to check — `mwl-types`' `signatures` known gaps. `private(set)` is § 3's write half and is
  not modeled at all.
- ADR 0043 § 4's `by $field` delegation exempts a whole class from conformance — `mwl-types`'
  `conformance` module doc.
- No override-compatibility check exists, so ADR 0013 § 1's `compareTo(self)` variance rule is unenforced
  — `mwl-types`' `signatures`.
- `mwl-ir` gap 19: `$n + $f` and `$n < $f` type-check and still fail in codegen.

## Orientation gaps

`[context]` has **no `spec` field**, so nothing selects a `docs/spec/` section — item 8a's own text sent
this session to `docs/spec/00-overview.md` § 2 for the grammar it had to implement verbatim, and that read
was unbudgeted. Every later spec-driven item (all of `Core` breadth) will pay the same. `modules` gained
`crates/mwl-syntax/src/ast.rs` this session; the `adrs` list already names ADR 0061 §§ 1, 2, 4, 5 and was
right for this item.
