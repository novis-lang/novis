# Handoff

## State

**Goal `unowned-closures`. The register is unchanged at `unowned: 15`, and goal-owned is down to
53.** The 15 unowned are the scheduling questions, none of which is one of this goal's own gaps, so
`python tools/owners.py`'s `unowned: 0` still turns on answers only the user can give — the position
the previous session left it in, unmoved by this one. `--deferrals` is green.

**`crates/nvs-types/src/intrinsics.rs` gap 1 is struck, built rather than deferred.** A refusal about
a template placeholder now underlines the placeholder instead of the whole literal.
`nvs_syntax::string_lit::cook_string_literal_positions` is the decoder's second mode — the map from
each decoded byte to the file offset it was written at — and it runs only once a diagnostic is being
built, which is what the gap's `Decided:` sentence chose. `nvs_stdlib::format::Written` is the other
half: the grammar locates every placeholder it reads and every refusal it makes.

**What is left of it is a bound, in that module's own prose rather than a gap.** The SQL, regex, CLDR
and metric-name validators answer with a message and no position, so their refusals still underline
the whole pattern; a heredoc falls back for a different reason, its indentation strip having moved
every byte off the offset it was written at.

**Gap numbering in that file is deliberately not contiguous — do not renumber it.** The block opens
at gap 2, which is the slot `docs/agent/loop-goal.md` § *Standing decisions* names as this goal's one
ADR. `crates/nvs-ir/src/lib.rs` is the same shape (gaps 14 and 18), so this is the tree's practice
and not a leftover.

**The manifest's one dead pattern is gone.** `[context] modules` named
`crates/nvs-types/src/string_lit.rs`, which no file matches: that path is a `pub use` of
`nvs_syntax::string_lit` at `crates/nvs-types/src/lib.rs:203`, and the escape grammar itself lives in
`nvs-syntax`. `orient.py` reported it dead every session and will not now.

## Next group

**Stage 3: the checker's intrinsic pass, the two gaps left in it** — one file set:
`crates/nvs-types/src/intrinsics.rs`, `crates/nvs-ir/src/lower/`, and
`crates/nvs-types/tests/intrinsics.rs`.

- [ ] **Open this goal's one ADR for the prepared-pattern channel, and build it** —
      `crates/nvs-types/src/intrinsics.rs:94` (gap 2) is the slot, and its `Decided:` sentence is
      "build the checker-to-IR channel; store prepared patterns in the artifact". The roster row
      already names what preparation produces (`rule:expressions/intrinsic-list-is-closed`), and
      `crates/nvs-types/src/expr_table.rs:@ExprTypeTable` is the existing checker-to-IR channel to
      widen rather than a second one to invent — `record_regex_tier` at
      `crates/nvs-types/src/intrinsics.rs:519` is a prepared fact already travelling that way.
- [ ] **Refuse an unterminated string literal in a literal query, and amend the doc that declines
      it** — `crates/nvs-types/src/intrinsics.rs:101` (gap 3), whose `Decided:` sentence settles the
      disagreement in this direction. The other half is `crates/nvs-db/src/sql.rs:53`, whose
      "malformed SQL is the server's diagnosis, not ours" paragraph is what has to be rewritten in
      the same slice (`rule:core-classes/db-literal-query-checking`).

## Backlog

- The four other grammars report a message with no offset, so their refusals underline the whole
  pattern — stated as a bound on `crates/nvs-types/src/intrinsics.rs`, not owed work.
- 53 gaps still carry `owner: unowned-closures`; `python tools/owners.py` lists them by file.
