# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

## Landed this session: ADR 0049 — `<?php` and `die` rejected, `<?mwl`/`exit` are the only spellings kept

A small, self-contained `mwl-syntax` slice, unrelated to M2's HIR/type-checker line of work below — picked
up from a design discussion, not from the milestone queue.

- **`die` is rejected; `exit` is the only process-termination keyword.** `parse_exit` (which both keywords
  still dispatch to) now reports `E_DIE_UNSUPPORTED` (`E0228`) when the consumed keyword was `die`, naming
  `exit` as the replacement, and returns `ExprKind::Error` instead of a live node. `ExprKind::ExitOrDie` is
  renamed to `ExprKind::Exit` since `die` can no longer reach it — updated across `mwl-types::expr`,
  `mwl-hir::{members,requires}`, and `mwl-syntax::casing`.
- **`<?php` is rejected; `<?mwl` is the only code-mode open tag.** The lexer still recognizes `<?php` and
  switches to code mode on it (unchanged), but `parser.rs`'s `parse_statement_inner` tag-reentry loop now
  reports `E_PHP_OPEN_TAG_UNSUPPORTED` (`E0229`) each time it consumes an `OpenTagPhp` token — at file start
  or any mid-file reopen — naming `<?mwl` as the replacement, then continues parsing the following code
  normally (not misread as inline HTML).
- Both diagnoses were chosen over keeping either as a permanent alias: neither ever differed in behavior
  from the spelling kept (confirmed by a cross-language survey — no other language surveyed keeps a bare,
  behaviorally-identical synonym for "terminate the process" or "open code mode"), and PHP source already
  needs a `mwl convert` pass regardless, so the extra rename costs that tool nothing new.
- **`docs/implementation-plan.md`'s M1 paragraph corrected**: `corpus_parse.rs`'s "parses clean" claim
  predates this ADR — every real `php-src` file opens with `<?php`, so every corpus file now also trips
  exactly one `E0229`. Noted as expected, not re-measured (no corpus is checked into this repo to re-run
  against).

Verified this session: `cargo build`/`cargo test`/`cargo clippy --all-targets -- -D warnings`/
`cargo fmt --check` all clean across the full workspace (`mwl-syntax` 191 tests — up from 189, two new
ADR 0049 fixtures — all other crates unaffected: `mwl-hir` 68, `mwl-types` 222, `mwl-ir` 102).

New doc: [ADR 0049](docs/adr/0049-single-open-tag-and-single-exit-keyword.md). Also touched: `docs/adr/
0034-legacy-cast-syntax-rejected.md` (its *Consequences* section had named `<?php` as a still-kept PHP
spelling — corrected to point at ADR 0049 instead), `docs/adr/README.md`'s index, `docs/spec/00-overview.md`
§ 1's tag table, and `CLAUDE.md`'s routing table + ground-rules bullet list.

## Pick up next: ADR 0043's `by`-delegation resolution — the one substantial M2 item still open

This was queued before this session and is untouched by it:

- **`by`-delegation resolution**: checking that `$field`'s declared type actually satisfies the delegated
  interface (`E_DELEGATE_TYPE_MISMATCH` on a mismatch), and synthesizing/checking the one-line forwarding
  methods [ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) § 4
  describes. `mwl-syntax`'s `ImplementsClause.by_field` has parsed since the M1 slice but is still
  completely unread by `mwl-hir`/`mwl-types` — grep for `by_field` to find the one unused site.
- **`E_INTERFACE_MEMBER_CONFLICT`**: today `resolve_method_rec`'s `find_map` over `extends.iter().chain
  (implements.iter())` silently returns whichever ancestor it reaches first when a method name is reachable
  from more than one default/delegated source with no class override — there is no collision check at all.
  This is a structural, per-class-declaration check (not a per-call-site one), closer in shape to
  `hierarchy::detect_cycles` than to anything in `expr.rs`. Needs `E_DELEGATE_TYPE_MISMATCH`'s `by`-delegation
  resolution to exist first, since a delegated interface is one of the three conflict sources ADR 0043 § 5
  names (the other two, "two different implemented interfaces' defaults," are already checkable without it —
  consider whether that narrower slice is worth landing on its own first).

Give it the same care ADR 0043 §§ 4-5 already spell out rather than re-deriving the rules here.

## `mwl-ir`: still the main line of work once the above is closed out

**Last landed: `break`/`continue` for `while` loops (level 1 only)**, via a `Lowering::LoopFrame` pushed
around `lower_while`'s body, recording one `(BlockId, Env)` edge per `break`/`continue` reached at any
nesting depth, folded into the header phi-patch (for `continue`) or a new `merge_envs` join at `after_block`
(for `break`, which previously always exited with exactly `header_env`). No new `Terminator`/`InstKind` was
needed. 102 tests. Nothing in `mwl-ir` changed this session or the one before it.

**Still explicitly out of scope:** `break N`/`continue N` for `N > 1` or a non-literal level (panics naming
`Lowering::loop_exit_level`); either keyword inside `for`/`switch` (neither lowers yet); a `break`/`continue`
outside any loop reaching this crate unrejected (a `mwl-types` gap, defensively caught here).

**Recommended next picks** (the workspace builds clean, so nothing here is blocked):

- **`for` loops** — reuses `LoopFrame` verbatim; the natural next pick per the plan's own milestone text.
- **`switch`** — needs PHP's fallthrough-by-default case semantics decided, plus a `break`-exits-the-switch
  frame kind distinct from `LoopFrame` (not a drop-in reuse — scope it deliberately).
- **The `mixed` runtime type-tag representation** (`mwl-ir`'s own known-gaps list, item 5) — the biggest
  remaining unblock, real design work rather than a mechanical slice. Pre-authorized to design and proceed.
- **A `...spread`/`&value` array-literal element** (same list, item 4) — its own design each (array-merge
  semantics; a reference-value representation).
- Skip known-gap item 6 ("Virtual dispatch") until M3 starts — deliberately architectural/speculative and
  deferred by the user's own standing instruction.

Once control flow, calls, and property/array access all lower, M2's *Verify* bullet ("IR snapshot tests; no
program in the corpus produces an `Unknown` type") is worth a real corpus-driven snapshot suite, not just
hand-written fixtures — at that point M2 should be closeable and M3 (baseline Cranelift backend, `Hello
World`) can start.

**Also still open, independent and low priority:** a `set`-hooked property is exempted from ADR 0022's
constructor check entirely rather than verified against the hook's body; the identical question applies to
whether a `lateinit` + hooked property should discharge on the hook's first commit (ADR 0038's own
*Revisiting* names this, deferred to `docs/spec/`).

## Housekeeping

`python .claude/brief.py`'s "WHERE THE PLAN STANDS" section is still hitting its 4000-byte budget and
truncating (the M2 paragraph in `docs/implementation-plan.md` is the largest single contributor) — this is
exactly the signal `DOC_CLEANUP_PROMPT.md` describes as "overdue for a trim pass." The user runs that pass
manually; flagging it again since it's now a recurring truncation, not a one-off.

## Other PHP output/echo-family stdlib functions raised in the same discussion, not yet actioned

Also discussed this session but deliberately not implemented (M2 has no `Core` stdlib classes yet — that's
M7/M8): `sprintf`/`printf`/`print_r`/`var_dump`/`var_export` should become `Core` methods when the stdlib
work starts, with `printf` as sugar over `echo sprintf(...)` (reuses ADR 0024/0033's sink rules for free)
and `print_r`/`var_export`'s "return vs. print" boolean flag split into two distinct calls rather than
carried over from PHP. `vprintf`/`vsprintf`/`fprintf`/`debug_zval_refcount` were recommended to skip
(depend on undecided argument-unpacking/stream-resource features, or have no MWL equivalent at all). No ADR
was deemed necessary — ordinary stdlib scoping under [ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md).
Revisit this note when `Core` stdlib design actually starts; it may be stale by then.
