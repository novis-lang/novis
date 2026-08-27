# Handoff

## State

**ADR 0107 is written and the loop is pointed at it: a by-reference binding is
spelled `inout`, before the type and again at the call site, and `&` is retired
as a by-reference marker.** This session landed the decision and the docs
around it. **No code changed** — the tree still spells it `&`, and closing that
gap is Stage 0a, items 44–47.

- **The word is not `ref`, and the reason is mechanical.** MWL's `&$x`
  parameter is copy-in/copy-out — `Lowering::pending_refs`
  (`crates/mwl-ir/src/lower/mod.rs:1182`) stages a cell and copies back after
  the call, and `write_back_holder` (`:2123`) names the throwing-callee path
  that skips it — while a `foreach` binding is a write-through instead. Every
  true aliasing spelling is already refused (`E0701`, `E0492`, `E0493`). Across
  languages the word tracks the mechanism: `ref`/`&`/`var` alias, `inout`
  copies in and back. Hack reached the same conclusion from the same PHP
  starting point.
- **The call-site marker is the half that buys something**, and it is the half
  a rename alone would not. `Adder::bump($n)` is today indistinguishable from
  `Adder::sum($a, $b)` at the point of call.
- **`&` keeps bitwise AND and the intersection type and loses nothing else**,
  which is what lets `Parser::at_intersection_amp`
  (`crates/mwl-syntax/src/parser/ty.rs:147`) be deleted rather than reworded.
- **Three codes are named but not yet declared**: `E0237` (the parse-time
  refusal, in the rejected-PHP-constructs band), `E0713` and `E0714` (the two
  call-site mistakes). They are the next free in their bands as of this
  session; item 44 declares them in `crates/mwl-diagnostics/src/lib.rs`, which
  is the registry and the only place a code exists.
- **M11 is covered by the ADR's own Verification entry**, per
  `docs/plan/m11.md`'s rule that every construct MWL removed keeps its
  destination in the ADR that removed it. The rule is **tier D**, not E — a
  callee that writes and then throws diverges — and the call-site rewrite is a
  new obligation on the converter, since PHP call sites carry no marker and
  emitting one needs the callee's signature. An unresolvable callee is a tier N
  branch with a `TODO`, never a silent by-value call.
- `docs/spec/02-php-migration.md` needed nothing: it maps PHP *names*, and this
  is a syntax rule.

## Next group

**Stage 0a items 44, 45 and 46 — the front end, the checker and the rename.
They are one session and one green run, because the workspace does not compile
between them.** The file set: `crates/mwl-diagnostics/src/lib.rs`,
`crates/mwl-syntax/`, `crates/mwl-types/expr/`, `crates/mwl-ir/src/lower/`. Read
the ADR first; `docs/agent/loop-goal.md` § *Stage 0a* has the anchors.

- [ ] **Item 44 — `inout` parses, `&` is `E0237`.** `Keyword::Inout` at
      `crates/mwl-syntax/src/token.rs:368`; the modifier slot `parse_param`
      already runs at `crates/mwl-syntax/src/parser/expr.rs:1512`; the
      `foreach` binding at `crates/mwl-syntax/src/parser/stmt.rs:366` and the
      destructuring leaf at `:902`. The ten `TokenKind::Amp` sites are listed
      by `grep -n "TokenKind::Amp" crates/mwl-syntax/src/`; two of them
      (`parser/expr.rs:308`, `parser/ty.rs:148`) are the meanings that stay.
      `ast::Arg` gains the call-site marker; `at_intersection_amp` is deleted.
- [ ] **Item 45 — the call-site rule.** `E0713`/`E0714` from
      `crates/mwl-types/src/expr/args.rs:524` and
      `crates/mwl-types/src/expr/calls.rs:671`.
- [ ] **Item 46 — the rename.** `by_ref` → `inout` through
      `crates/mwl-ir/src/lower/`, `signatures.rs` and `core_lib.rs`. No
      semantics move; `Ty::Ref` and `pending_refs` keep every rule they have.

Item 47 (the corpus and the remaining docs) is the group after, and is
deliberately not in this one: it is 14 `.mwlt` rewrites over a file set that
shares nothing with the above, and its whole check is that the expected output
does not move.

## Backlog

- The three `.mwlt` cases the previous handoff named, now **after** Stage 0a so
  they are authored once:
  `tests/conformance/lang/a-mixed-value-answers-arithmetic-truth-and-a-subscript.mwlt`,
  `every-remaining-conversion-row-runs-or-throws.mwlt`, and
  `inline-html-at-file-scope-is-echoed-in-place.mwlt`
  (`crates/mwl-ir/src/lower/convert.rs:60`,
  `crates/mwl-ir/src/lower/expr.rs:3142`, `:467`).
- An abandoned generator's `finally` — goal item 13, pre-authorized in
  `docs/agent/loop-goal.md` § *Standing decisions*; two named cases owed, and
  `crates/mwl-ir/src/lower/generator.rs:99` is the anchor.
- First-class callable syntax (`Class::method(...)`) still panics — `mwl-ir` gap
  1, at `lower/call.rs:85` and `:652`.
- ADR 0007 § 4's promotion table has 9 refusal sites left (`python
  tools/holes.py --item 1`), the largest remaining group.
- `crates/mwl-codegen/src/ty.rs:116` and `:121` are the two refusal sites no
  item anchors (`holes.py`'s UNATTRIBUTED section).
- Item 25: `object` as a declared type has 2 refusal sites left.
- `tests/conformance/lang/a-required-file-runs-its-own-top-level-statements.mwlt`
  is owed but blocked on `mwl-ir` gap 22 (a `require`d file's own statements do
  not run).
