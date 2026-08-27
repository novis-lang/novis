# Handoff

## State

**M4 — language completeness.** Two of the three shapes that reached `mwl-ir`'s statement
catch-all are gone, and they went out opposite doors.

**Inline HTML lowers.** A run of text between `?>` and the next `<?mwl` is
`Lowering::lower_inline_html` (`crates/mwl-ir/src/lower/expr.rs:374`) — the same
`Helper::EchoStr` write `lower_echo` emits, over the span's raw source bytes, with the
`ConstStr` owned as a temporary so both edges release it. Nothing had to be cooked or
escaped, and nothing is file-scope-specific: the lexer already swallows the one newline
after `?>` and pushes no token for an empty run, and a run inside a loop body lowers into
that body. Byte-identical to PHP over the same file, checked by hand; `tools/leak-check.sh`
clean.

**A `class`, `interface` or `enum` declared anywhere but file scope is `E0233`** — a decision
this session took and recorded in `docs/adr/README.md` § *Decisions taken at project start*:
a name whose existence depends on control flow has no reading MWL's static class table can
give it. It is reported by `mwl_types::locals`' per-body walk
(`crates/mwl-types/src/locals.rs:940`, `nested_type_declaration`), which is reached *only*
from inside a body — `check::check_stmts` matches all three at file scope itself and never
forwards one — so arriving is the nesting test and no flag is threaded. Covers a method body,
a property hook, a closure body and a block at file scope.

`mwl-ir`'s known-gaps list gained a rule it was already following by accident: **a number
there is a stable identifier and a closed gap leaves a hole**, never a renumber, because
`loop-goal.md` and `playbook.md` cite them. Gap 13 is retired.

Two conformance cases under `tests/conformance/lang/`; `verify.py` green — conformance 628,
differential 173. `python tools/holes.py` is at 25 sites.

**Gap in the pack:** `orient.py` printed no map line for `crates/mwl-types/src/check.rs`,
which is what proves the `locals.rs` walk is nested-only. `[context] modules` wants an
`mwl-types/src/check.rs` pattern on any item about where a statement is checked.

## Next group

**What still reaches a catch-all in the statement slice, all in one file.** The file set:
`crates/mwl-ir/src/lower/stmt.rs`, `crates/mwl-types/src/locals.rs`,
`tests/conformance/lang/`. A scratch `.mwl` under `.agent-tmp/` reproduces each in one call.

- [ ] **ADR 0050's `[$a, $b] = $pair` destructuring lowers** — the *last* shape reaching the
      dispatch catch-all at `crates/mwl-ir/src/lower/stmt.rs:230`. The checker half is already
      there: `mwl_types::locals::walk_destructure_target`, `crates/mwl-types/src/locals.rs:934`.
      The largest of the three.
- [ ] **The reassignment catch-all** at `crates/mwl-ir/src/lower/stmt.rs:1170` — `holes.py`
      item 7's remaining sites. Each shape it names is either a lowering to write or a
      checker refusal to move up, the way the increment's three were.
- [ ] **An `unset` target whose base is not an array** at
      `crates/mwl-ir/src/lower/stmt.rs:1243` — the message already names the two ways in
      (erased to `mixed`, or a `?T` never narrowed), so it is a refusal-or-lower judgement.

## Backlog

- `mwl-ir` known-gaps entries 1 (`do`/`while`) and 16 (`$x++`/`--$x`) have stale headlines —
  both lower now. `crates/mwl-ir/src/lib.rs:150`, `:380`.
- Two refusal sites no item anchors: `crates/mwl-codegen/src/ty.rs:116` and `:121` —
  `python tools/holes.py`.
- `docs/agent/loop-goal.md:203` cites `mwl-ir` gap 13, now retired.
- ADR 0007 § 4's promotion table — `holes.py` item 1, 11 sites, the largest left.
- 14 named `.mwlt` cases still owed — `python tools/holes.py --cases`.
