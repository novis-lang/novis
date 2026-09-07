# Handoff

## State

**Goal 12 — the resilient tree — has stages 2, 3 and 4's in-process property landed and green.**
Recovery is explicit (`rule:ide/recovery-is-explicit`), the `SyntaxIndex` is the walk flattened, and
`crates/nvs-syntax/tests/prefixes.rs` now cuts every `examples/*.nvs` at every token boundary and asks
all three of stage 4's questions of each cut: 14,268 prefixes parse without panicking and invent no
span outside their own text, 14,192 of them answer a containment-chain lookup at the last byte they
contain, and all 8,716 that end inside an open bracket report a diagnostic. The whole sweep is 4s in a
debug build, so it stays an every-iteration test rather than a nightly one.

**What stage 4 still owes is the fuzz target**, which is CI's job rather than the loop's: M4B's
acceptance paragraph names "a fuzz target over truncated and mid-edit inputs finds no panic in five
minutes", and `fuzz/fuzz_targets/` has `lex`, `parse` and `uri` with only the first two in the
`fuzz-smoke` matrix.

**One residue, deliberate and unchanged:** `ExprKind::ClassConstAccess` carries a bare `Span` for its
name, so `Foo::` at the caret still carries a name nobody wrote. The `::` path routes `Missing` into
the same arm `Ident` takes rather than reporting an invented second diagnostic; the comment at that
site says so, and the Backlog carries the fix.

**Stage 0 is empty and stays empty**, unchanged: the four M4 language holes are closed and `bool as
int` is `E0708` under `rule:types/conversion`, not a hole.

## Next group

**The truncation fuzz target** — one new file plus two registrations, all of them outside `crates/`,
which is why this session stopped rather than taking it: `fuzz/fuzz_targets/prefix.rs` modelled on
`fuzz/fuzz_targets/parse.rs`, its `[[bin]]` beside the one at `fuzz/Cargo.toml:30`, and the matrix at
`.github/workflows/ci.yml:414`. The three slices share that file set.

- [ ] **A `prefix` fuzz target that truncates its own input** — take the arbitrary bytes as a Novis
      source, cut it at a byte the input itself chooses, and parse both halves through
      `crates/nvs-syntax/src/parser/mod.rs:771`, asserting nothing beyond "it returned".
      `rule:ide/the-tree-survives-a-syntax-error`; M4B's acceptance paragraph names the five minutes.
- [ ] **Register it as a bin and in the smoke matrix** — a `[[bin]]` beside `fuzz/Cargo.toml:30`, and
      `prefix` added to `target: [lex, parse]` at `.github/workflows/ci.yml:414`. Note while there
      whether `uri` belongs in that matrix too; it is a target no job runs.
- [ ] **A seed corpus under `fuzz/corpus/prefix/`** — a handful of `examples/*.nvs` bodies, so the
      five minutes start from real Novis rather than from random bytes — `examples/db.nvs:1` is the
      largest and the first worth seeding. The cache step that restores the corpus across runs is
      `.github/workflows/ci.yml:429` and needs no edit.

## Backlog

- `ExprKind::ClassConstAccess` should carry a `MemberName`, so `Foo::` reports one diagnostic and no
  invented name — `rule:ide/recovery-is-explicit`, `crates/nvs-syntax/src/ast.rs`.
- `fuzz/fuzz_targets/uri.rs` is a target the `fuzz-smoke` matrix never runs — `.github/workflows/ci.yml:414`.
- Stage 5 onward of goal 12 is untouched; `docs/agent/loop-goal.toml` is the roster.
