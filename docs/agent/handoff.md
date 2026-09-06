# Handoff

## State

**Goal 8's stage 3 is on disk, and it was the last stage with a check.**
`Core\Program::id()` is a registry row, a card, a body and an `address()` arm in
`crates/nvs-stdlib/src/program.rs:78`; it reads `Ctx::program_id` and hashes, caches, memoizes and
truncates nothing, because `nvs_config::cache::program_id` did the combine once at program
resolution. ADR 0061 gained § 6 as the folded amendment the goal pre-authorized — the formula, the
plain-`string` decision, and the circularity that forbids folding the id into a unit — and no new
ADR number was taken. `examples/program-id.nvs` prints the two frozen lines; three `.nvst` cases pin
the shape, the constancy within a run, and the agreement between two units of one program.

**Only the CLI run path writes an id.** `crates/nvs-cli/src/serve.rs:588` hands each request an
isolate over a `Ctx` nobody wrote one onto, so the member throws there. That refusal is asserted by
`program::tests::id_refuses_a_context_no_host_wrote_an_identity_onto` rather than by a case: a
`.nvst` runs under `nvs run`, which always writes the id, so no case can reach the path — which is
the `no case can reach this` declaration `every_error_path_is_asserted_or_declared_unreachable`
requires. The "recomputed at the hot-reload swap" half of the goal's standing decision still needs a
swap `nvs-cli`'s `Compiler` does not perform.

Nothing is blocked. Two `[context]` gaps in `docs/agent/loop-goal.toml`: `modules` still does not
print `nvs-cli`'s `src/main.rs` or `src/cache.rs` (the previous session asked for both and the
manifest is unchanged), and nothing under `shapes` names the `--FILE <path>--` auxiliary-file section
of a `.nvst` case, which the third case here needed — I read an existing case for the spelling.

## Next group

**The served half of the id** — one file set, `crates/nvs-cli/src/serve.rs` and
`crates/nvs-cli/src/main.rs`, and the first slice decides the other two.

- [ ] **A served request's context carries the program id** — `crates/nvs-cli/src/serve.rs:588`
      (`isolate`, where a request's `Ctx` is wired before the isolate runs) and
      `crates/nvs-cli/src/main.rs:1321` (the `nvs run` write, which is the shape to copy verbatim).
      ADR 0061 § 6 computes the id at program resolution, and `serve` resolves at boot — so compute
      it once there and write it onto every request's context, never per request.
- [ ] **A `-p nvs-cli` test that a served request answers the same id as `nvs run`** —
      `crates/nvs-cli/src/serve.rs:588` for the seam and `crates/nvs-cli/src/script.rs:602`
      (`run_serving`) for the twenty-line fixture shape a test in this crate needs. Agreement between
      the two hosts is the claim; either one's 64 characters on its own proves nothing.
- [ ] **Say what "recomputed at the swap" means while nothing revalidates** —
      `docs/adr/0061-compile-time-autoload-and-program-discovery.md:200` (§ 6's third paragraph) and
      `crates/nvs-cli/src/serve.rs:588`. ADR 0017's swap is the other moment the answer changes, and
      `nvs-cli`'s `Compiler` compiles once per written path and never revalidates, so § 6 currently
      states a rule the tree cannot exercise. One paragraph, in the ADR or in `serve.rs`'s module doc.

## Backlog

- `[context] modules` needs `nvs-cli/src/main.rs` and `nvs-cli/src/cache.rs` — `docs/agent/loop-goal.toml`.
- `[context] shapes` has no entry for a `.nvst` `--FILE <path>--` section — `docs/agent/conventions.md` owns the shape.
- Every chapter under `docs/reference/core/` carries exactly one example, so a new member extends the existing one — `docs/agent/doc-style.md`.
