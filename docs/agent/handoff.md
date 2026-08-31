# Handoff

## State

**Goal 4, stage 0 — ADR 0056's regex tiering — is two thirds landed.** §§ 1 and 2 are now asserted
where the rule lives: `crates/nvs-stdlib/src/regex.rs`'s `build` routes on *which* refusal the linear
engine gave, and three named tests hold it. Stage 0's remaining item is the compile-time half, which is
entirely in `crates/nvs-types`.

**The routing rule is the substantive change and it is a behaviour change.** A pattern moves to the
backtracking tier only when `regex`'s **parser** refuses a construct; every other refusal — today
`CompiledTooBig`, and the enum is `non_exhaustive` — is thrown rather than re-tiered, because a pattern
whose exposure to backtracking depends on how large its automaton happens to be is the silent,
size-dependent move ADR 0056 exists to prevent. `build`'s own doc comment is the home of that reasoning;
nothing in the ADR needed changing, and its § *Verification* already reads correctly.

Everything else this goal needs is already built and is not to be re-invented: goal 2's parking stream,
goal 2's blocking pool, goal 2's graph copy, goal 3's capability gate.

## Next group

**ADR 0056 § 3, the compile-time half** — stage 0's last item, and the whole of it is in
`crates/nvs-types`. The fold that reads a literal pattern already exists and already refuses a malformed
one; what it does not do is record *which tier* prepared it. The sink half is a separate axis in the same
crate.

Two files: `crates/nvs-types/src/intrinsics.rs` and `crates/nvs-types/tests/intrinsics.rs`, plus
`crates/nvs-types/src/expr/quals.rs` for the second item.

- [ ] **A literal pattern's tier is settled while checking.** ADR 0056 § 3. The fold at
      `crates/nvs-types/src/intrinsics.rs:204` calls `nvs_stdlib::regex::validate`, which throws the
      automaton away — so make it report the tier instead of a bare `Result<(), String>` and record it.
      The acceptance name is `a_literal_patterns_tier_is_settled_while_checking`, `-p nvs-types`;
      `crates/nvs-types/tests/intrinsics.rs:152` holds the existing prepared-while-checking case and is
      where it goes. `crates/nvs-stdlib/src/regex.rs:@validate` is the function to widen, and its doc
      already says why the flags are not read.
- [ ] **The pattern argument refuses a `tainted` operand.** ADR 0056 § 4 over ADR 0024 § 3. The registry
      row already carries `Qual::Sink` (`crates/nvs-stdlib/src/regex.rs:108`), and
      `crates/nvs-types/src/expr/quals.rs:227` is where a sink parameter's refusal is decided; the census
      at `crates/nvs-types/src/core_lib.rs:598` counts the sinks. Check whether the refusal already
      fires before writing anything — the acceptance name is
      `a_regex_pattern_argument_refuses_a_tainted_operand` and it may be a test over landed behaviour.
      If it does fire, `regex.rs`'s module-doc gap 1 is stale in its first half and owes an edit.

## Backlog

- ADR 0056 § 3's `[regex] backtracking = "allow" | "warn" | "deny"` — no acceptance check names it; it
  needs M6's configuration, which goal 3 landed. `docs/adr/0056-regex-engine-policy.md` § *Verification*.
- `regex.rs` module-doc gap 2: `BACKTRACK_BUDGET` is a constant, not an `nvs.toml` directive, and M6's
  configuration subsystem is now on disk.
- `regex.rs` module-doc gap 3: `matchAll` converts each offset over the subject's prefix, so it is O(n·k).
- Stage 8 is open: conformance 1093, differential 210 of 210, migration 37% over its 36% floor.
