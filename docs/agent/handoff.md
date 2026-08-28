# Handoff

## State

**M4's Stage 6 is the frontier, and the `mixed` receiver's whole deferral is now
closed — convention, descriptor, checker, lowering, dispatch and case.** A call
through a `mixed` receiver runs; `docs/adr/README.md` § *Decisions taken at
project start* has the paragraph that specifies it and is not re-opened.

- **`nvs_ir::Helper::CallErasedMethod` is the call**, that variant's own doc
  comment being the home of its shape: receiver tagged, member name an
  immortal `ConstStr`, every argument in one array. `nvs_ir::lower::call`'s
  `lower_erased_method_call` is the one emitter.
- **`nvs_runtime::dispatch::call_erased_method` is the answer**, and its own
  `# Errors` section is the roster of what it refuses and why each is
  catchable. `nvs_call_erased_method` is the exported symbol.
- **`check_param_tags` takes a word now, not a closure object** — that is what
  makes it the one implementation ADR 0007 § 2's widening lives in for both
  erased paths; `closure_param_tags` reads the closure half.
- **A `Core` receiver is refused twice over**, and the second is worded off the
  reserved namespace rather than off a row, because a `Core` class carries no
  compiled method table for the first to read.
- `lower_method_call`'s panic roster is empty: every receiver naming no class is
  now either `E0477` at the checker or dispatched here.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and
  0069; 0014 and 0046 have theirs.

## Next group

**ADR 0023's *Verification* section, which M4's acceptance has named since it
was written and which no session has opened.** One file set:
`docs/adr/0023-clone-serialize-and-cross-boundary-copy.md`, plus whatever
`tests/conformance/class/` cases the section turns out to owe. Nothing here
reaches `nvs-ir` or `nvs-runtime` again, so it shares no files with the group
just closed — take it as a fresh window.

- [ ] **Read what ADR 0023 already decides and what the tree already pins** —
      the four rules are at
      `docs/adr/0023-clone-serialize-and-cross-boundary-copy.md:50` (`clone`),
      `:81` (the graph copy), `:121` (`unserialize`) and `:156` (why neither
      depth reopens ADR 0022); *Consequences* is `:173` and the file ends at
      `:239` with no *Verification* section at all. `python tools/gaps.py` and
      `python tools/loop.py --list` name the `.nvst` cases the stage still
      owes. Write the section over the cases that exist rather than inventing
      rules.
- [ ] **Write the section**, stating the things no case can assert, inserted
      before *Alternatives rejected* at
      `docs/adr/0023-clone-serialize-and-cross-boundary-copy.md:202` — the
      shape ADR 0046 § *Verification* took, which is the worked example, over
      four cases, two running and two refusing.
- [ ] **Land whatever case the section names and the tree does not have**, one
      `.nvst` under `tests/conformance/class/`.

## Backlog

- ADR 0028's and ADR 0069's *Verification* sections, the other two M4's
  acceptance names — `docs/plan/m4.md` § *Verify*.
- A `require` whose path is not a string literal runs nothing at all, silently,
  in both forms — `nvs_hir::requires`' own known gap.
- A shape literal's field carries no `secret` bit, its type being inferred —
  `nvs_stdlib::debug`'s known gap 1, ADR 0033's unmodelled container axis.
- Virtual dispatch by slot rather than by name, which is the other half of
  `nvs-ir`'s item 8 — a lookup cost, M12.
- `nvs_types::signatures`' own known gap: a class constant's declared type is
  unmodelled, so `Class::TOKEN` infers `mixed` at every expression site.
