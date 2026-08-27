# Handoff

## State

**ADR 0107's surface is in the tree.** `inout` is a reserved word that parses
before the type in all three binding positions and again at a call site, `&`
in a by-reference position is `E0237`, and the checker enforces the call-site
marker in both directions (`E0713`/`E0714`). Stage 0a items 44 and 45 are
done; the corpus rewrite that items 44–45 forced is done with it. Both trees
are green at 674 conformance and 174 differential — the **unchanged** expected
output of the 12 rewritten cases is ADR 0107's own proof that it changed no
semantics.

- **What is left of Stage 0a is item 46's prose and item 47's new case.** Every
  read of a renamed AST field already moved (that is what made the workspace
  compile), so item 46 is now purely `by_ref` → `inout` in *identifiers and doc
  comments*: `MethodSig::by_ref`/`is_by_ref`/`has_by_ref`
  (`crates/mwl-types/src/signatures.rs:80`), `ResolvedCall::by_ref`
  (`crates/mwl-types/src/expr_table.rs:98`), `check_by_ref_arg`
  (`crates/mwl-types/src/expr/args.rs:558`), `check_foreach_by_ref`
  (`crates/mwl-types/src/expr/iteration.rs:279`), `by_ref_elements` and
  `collect_by_ref_holders` (`crates/mwl-ir/src/lower/control.rs:1158`, `:2296`),
  and the `&$x` still written in about 30 doc comments. `grep -rn by_ref
  --include=*.rs crates/` is the whole worklist. **The user-facing message
  strings are already done** — nothing a program can print still says `&`.
- **`at_intersection_amp` survives, and ADR 0107 § 3 now says so.** The ADR
  asked for it to be deleted; deleting it means the type parser eats the `&` in
  `int &$x` as an intersection continuation and the site above has nothing left
  to name, so `E0237` would degrade to "expected a type". The correction is
  folded into § 3's body and into the function's own doc comment
  (`crates/mwl-syntax/src/parser/ty.rs:147`), which is the pair of homes.
- **The two returning forms are refused, not renamed.** `MethodMember::by_ref`
  and `PropertyHook::by_ref` were parsed and then silently dropped by every
  later pass; they are gone from the AST and `E0237` names them where they are
  written.
- Three codes are declared and each is used: `E0237`
  (`crates/mwl-diagnostics/src/lib.rs:340`), `E0713` and `E0714` (`:1145`).
  Next free is now `E0238` and `E0715`.

## Next group

**Stage 0a items 46 and 47 — the rename and the corpus's own new case. One
session: 46 is mechanical and 47 is one case plus a doc sweep, and neither
compiles against a half-renamed tree.** The file set:
`crates/mwl-types/src/{signatures.rs,expr_table.rs,expr/}`,
`crates/mwl-ir/src/lower/`, `crates/mwl-stdlib/src/`, then `tests/` and `docs/`.

- [ ] **Item 46 — the rename.** `by_ref` → `inout` through the identifiers and
      doc comments listed under *State*; ADR 0107 § *Consequences* calls it
      semantically empty and it is. `Ty::Ref`, `pending_refs`
      (`crates/mwl-ir/src/lower/mod.rs:1182`) and `write_through_element` change
      only in their prose. Start from `grep -rn by_ref --include=*.rs crates/`.
- [ ] **Item 47a — the new case.**
      `tests/conformance/lang/a-by-reference-argument-is-written-inout-at-both-ends.mwlt`,
      pinning the accepted shapes and all three diagnostics — ADR 0107
      § *Verification* names it. The three diagnostics are already reachable;
      `tests/conformance/lang/compound-assignment-writes-through-a-by-reference-parameter.mwlt`
      is the nearest worked shape.
- [ ] **Item 47b — the 15 doc files.** Every `&$x` left in `docs/`, plus
      `docs/plan/m4.md`, then `docs/agent/loop-goal.md` § *Stage 0a* deleted.
      `grep -rn '&\$' docs/` is the worklist.

## Backlog
- ADR 0089's rule table owes ADR 0107's convert rule at tier D, with the tier N
  fallback for an unresolvable callee — `docs/adr/0107-…md` § *Verification* (M11).
- `check_generic_args` returns before `check_by_ref_arg` runs, so a generic
  method's `inout` argument skips both obligations — `crates/mwl-types/src/expr/args.rs:72`.
- `cargo install cargo-insta` is not done in this environment; see the playbook
  bullet under *Tooling*.
