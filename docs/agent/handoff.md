# Handoff

## State

**Goal 23 — `nvs serve` takes every core. Stage 2's publishing half is on disk and green.** Goal
22's whole list is this goal's Stage 1 floor and is green.

**What landed:** the compiled-unit cache publishes rather than copies. `Compiled` holds an
`Arc<nvs_codegen::Unit>` and a `Ready` entry is an `Arc<Compiled>`
(`crates/nvs-cli/src/script.rs:131`, `:176`), so every reader of one content digest is handed a
clone of the same pointer. `Compiler` is `Send + Sync` with no annotation — nothing in it was
thread-affine once the unit stopped being — and `a_compiled_unit_is_read_by_every_core_through_one_arc`
(`crates/nvs-cli/src/script.rs:820`) is four real OS threads holding their programs at a barrier while
the published entry's reference count is read. `nvs serve` builds one cache before it binds anything
and hands it out by `Arc` (`crates/nvs-cli/src/serve.rs:276`).

**What is left in stage 2 is step 4's compare and the three tests over it.** `Compiler::advance`
still writes the path pointer unconditionally, which is the "publish only if nobody moved it since"
half of `rule:config/an-edit-reaches-the-next-request-without-a-restart`. Single-flighting stays a
later slice, with the fan-out, for the reason `script.rs`'s first known gap gives. Nothing is
blocked on a decision.

## Next group

**Stage 2: step 4's publish is conditional, and the stage's four remaining tests** — one file set:
`crates/nvs-cli/src/script.rs`.

- [ ] **`a_reader_holding_the_old_unit_keeps_answering_until_it_drops_it` and
      `the_compile_counter_counts_compiles_and_not_cores`** — both are tests over what already
      landed, and neither needs a code change. The first is the swap seen from a reader that
      resolved before it: `crates/nvs-cli/src/script.rs:989` is that fixture over one core, and
      `crates/nvs-cli/src/script.rs:820` is the threaded shape to copy. The second asserts the
      counter's own claim at `crates/nvs-cli/src/script.rs:245` — one per cache, not one per core.
      `rule:config/an-edit-reaches-the-next-request-without-a-restart` is what both state.
- [ ] **Step 4 publishes only if nobody moved the pointer since, and
      `a_stale_revalidation_does_not_overwrite_a_fresher_published_one`** —
      `crates/nvs-cli/src/script.rs:381` (`Compiler::advance`, today an unconditional `insert`) and
      `crates/nvs-cli/src/script.rs:399` (`Compiler::record`). The compare is against the
      `PathEntry` the resolve observed, so `advance` needs that entry as an argument;
      `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s step 4 is the sentence, and
      `script.rs`'s first known gap (`crates/nvs-cli/src/script.rs:63`) is what stops being a gap.
- [ ] **`a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile`** —
      `crates/nvs-cli/src/script.rs:381` for the publish and
      `crates/nvs-cli/src/script.rs:885` (`no_request_stalls_while_the_file_is_compiled`) for the
      never-blocks half, which is the same claim asserted across threads rather than resumes.

## Backlog

- Single-flighting a compile in flight — `crates/nvs-cli/src/script.rs:63`, lands with the fan-out.
- `nvs serve` accepts on one core; the per-core listeners are stage 3 — `crates/nvs-cli/src/serve.rs:296`.
- `Table` is still `Rc`-shared in `serve.rs`; the fan-out decides whether it crosses too.
