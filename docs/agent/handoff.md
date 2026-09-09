# Handoff

## State

**Goal 23 — `nvs serve` takes every core. Stage 2's first two slices are on disk and green.** Goal
22's whole list is this goal's Stage 1 floor and is green.

**What landed:** a compiled unit can now cross a core boundary. `nvs_codegen::Unit`'s two shared
fields are `std::sync::Arc`s (`crates/nvs-codegen/src/lib.rs:328`, `:355`), as are the two
signatures that take them — `Ctx::install_statics` (`crates/nvs-runtime/src/ctx/isolate.rs:164`) and
`ErrorClass` (`crates/nvs-runtime/src/ctx/error.rs:73`). `ClassTable` and `Unit` each carry an
`unsafe impl Send + Sync` whose argument is in its own doc comment, and each is pinned by a
compile-time crossing test — `a_class_table_crosses_a_core_boundary_behind_an_arc` and
`a_unit_crosses_a_core_boundary_behind_an_arc`. `nvs_codegen::Placed` gained a `Send + Sync`
supertrait so the loader's half is bounded rather than vouched for; `nvs-cli`'s `Loaded` satisfies
it unchanged.

**The stage-2 blocker the previous handoff named is closed.** `Arc<Unit>` is `Send + Sync`, so
`Compiled` can hold one and the cache can be reached from a second core. Nothing is blocked on a
decision.

**Single-flighting is still a later slice, and still on purpose.** With one accepting core
`CompileState::Compiling` and the "a fresher revalidation has not won" compare are unreachable, so
they land with the fan-out rather than before it.

## Next group

**Stage 2: the cache publishes one unit to every core** — one file set:
`crates/nvs-cli/src/script.rs`, `crates/nvs-cli/src/serve.rs`.

- [ ] **`Compiled` holds `Arc`s and the cache's `Ready` entry publishes one** —
      `crates/nvs-cli/src/script.rs:126` (`Compiled::unit`, today an `Rc<nvs_codegen::Unit>`),
      `crates/nvs-cli/src/script.rs:171` (`CompileState::Ready`), `crates/nvs-cli/src/script.rs:414`
      (`Compiler::compile`) and `crates/nvs-cli/src/script.rs:534` (`program_over`). The unit side
      is done: `Arc<nvs_codegen::Unit>` is `Send + Sync` and
      `crates/nvs-codegen/src/lib.rs:404`'s doc comment is the home of why.
      `rule:config/an-edit-reaches-the-next-request-without-a-restart` is the cache's shape, and the
      goal's § *Standing decisions* fixes the publisher in `script.rs`.
- [ ] **`a_compiled_unit_is_read_by_every_core_through_one_arc`, in `-p nvs-cli`** —
      `crates/nvs-cli/src/script.rs:774` is the shape a multi-reader test already takes here (one
      `Compiler`, N readers, a counted answer). The other four names in the same `[[check]]` block
      are the specification for the rest of the stage; take only this one until the fan-out exists,
      since `the_compile_counter_counts_compiles_and_not_cores` needs more than one core to mean
      anything.
- [ ] **`Compiler` is shared by `Arc`, not built per core** — `crates/nvs-cli/src/serve.rs:68`
      imports both `Rc` and `Arc` already, and `crates/nvs-cli/src/script.rs:783`'s comment says the
      current arrangement is one `Rc<Compiler>` per core, which is what stage 2 replaces.

## Backlog

- Single-flighting (`CompileState::Compiling`, the fresher-revalidation compare) — lands with the
  fan-out, not before it; `crates/nvs-cli/src/script.rs`'s module doc § *Known gaps*.
- `nvs-cli/src/main.rs:1584` still builds `Rc<nvs_codegen::Unit>` for the one-shot run path; it is
  correct as it stands and only needs revisiting if that path ever shares a unit.
- `nvs-cli/src/runner.rs` holds `Rc<nvs_codegen::Unit>` throughout (`:2101` counts strong handles);
  the test runner is single-core by design and is not stage 2's business.
