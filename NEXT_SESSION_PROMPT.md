# Next session prompt

Continue MWL. The `tainted` qualifier's grammar (ADR 0024 § 1) just landed in `mwl-syntax`:
`Keyword::Tainted`, `TypeAtom::TaintedString`/`TaintedBytes`, the parser production restricting
`tainted` to `string`/`bytes` (else `E_TAINTED_NON_SCALAR`), and round-trip tests in every
declaration slot. `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check`
are all clean, and the corpus-parse test already passes against the local `php-src` checkout.

What's left to close out M1 is its own verification step (see the plan's M1 paragraph and *Verify*
line in `docs/implementation-plan.md`):

1. **`cargo fuzz` on the lexer and parser, 5 minutes each, finding no panic.** This needs WSL, not
   native Windows — CLAUDE.md's "Fuzzing on Windows: use WSL" section has the one-time setup and the
   exact commands (`cargo +nightly fuzz run lex -- -max_total_time=300`, then `parse` likewise, from
   `/mnt/d/swlang` via `wsl.exe -- bash -lc "<command>"`). If either finds a panic, fix it in
   `mwl-syntax` before moving on — a fuzz-found panic is real input `mwl-syntax` must not crash on,
   even if the resulting AST doesn't need to be *correct* yet (that's M2's job).
2. Once both are clean, update `docs/implementation-plan.md`'s status block: M1's own verification
   is then fully done (corpus-parse already passes, fuzzing now too), so record that and move the
   milestone to done / start scoping M2 (name resolution, the type checker, IR lowering — see the
   plan's M2 paragraph and `docs/adr/README.md`'s index for the ADRs it has to enforce: 0007's type
   table, 0010/0013/0014/0015/0022's checker-side rules, and 0024 §§ 2-3's tainted propagation and
   laundering).

Read `CLAUDE.md` first (it routes to the one file you need per topic), then run `sh .claude/brief.sh`
for the live status slice before doing anything else.
