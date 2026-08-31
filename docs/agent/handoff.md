# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 0 is closed and stage 2 has opened.**
The regex tiering work landed over the previous two sessions; this one added the assertion that
`Core\Regex`'s pattern parameter refuses a `tainted` operand while its subject does not, which the
registry rows already enforced (`compile`'s pattern is `Qual::Sink`, every other member takes the
pattern through a union carrying no mark) and no test had asked.

**The acceptance list is measurable again.** The `0 regex tiering` cargo check listed
`examples/limits.nvs` — a program leg — among its `cargo test -p nvs-stdlib` test names, so it could
never pass and the driver stopped every iteration after four checks. Removed from
`docs/agent/loop-goal.toml` and its copy under `docs/agent/goals/`; the playbook's Tooling section
now carries the trap.

**`Core\File` is gone: the class is `Core\IO`, as spec § 14 names it.** The module moved with it
(`crates/nvs-stdlib/src/io.rs`) and so did its symbols (`nvs_core_io_read`/`_write`); the ADRs, the
reference chapter (`docs/reference/core/IO.md`), the conformance cases and `examples/capability.nvs`
all follow. The class still has only `read` and `write` — the member list below is the rest.

Three fixtures still owe configuration their stage must write, unchanged from last session:
`examples/http.nvs` names `http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs`
needs an `[[app]]` block naming `examples/logging/handler.nvs` as the error handler; and every fixture
that reaches the world needs its `fs.*` / `process.exec` / `net.connect` grants in `nvs.toml`.

`orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs` and
`crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**`Core\IO`'s member list, in the order `examples/files.nvs` needs it** — one file set throughout:
`crates/nvs-stdlib/src/io.rs` (the rows at `:41`, the cards after them, the `address` arm at `:132`)
and `crates/nvs-stdlib/src/registry.rs:1107`'s `CAPABILITIES`, which is where each new member declares
the grant its door asks for. Spec § 14 is the member list and is authoritative; the conventions'
*A `Core` member* is the five edits each one owes.

- [ ] **The metadata and removal members: `exists`, `size`, `remove`, `removeDir`.** Spec § 14
      *Metadata* and *Manipulation*. Four rows in `crates/nvs-stdlib/src/io.rs:41`, four cards, four
      bodies behind `nvs_runtime::capability`'s existing doors, four arms at
      `crates/nvs-stdlib/src/io.rs:132`, and four `CAPABILITIES` entries at
      `crates/nvs-stdlib/src/registry.rs:1107` (`exists` and `size` are `fs.read`, the two removals
      are `fs.write`). One `.nvst` case covers the group.
- [ ] **The text half: `readText` and `lines`.** Spec § 14 *Whole-file*. `readText` decodes where
      `read` hands back bytes; `lines` answers an `Iterable<string>` and must not hold the file, so it
      is a cursor over the handle rather than a `Vec` — `crates/nvs-stdlib/src/cursor.rs` is the
      shape the other collections use. Rows at `crates/nvs-stdlib/src/io.rs:41`.
- [ ] **`temporaryDir`.** Spec § 14 *Manipulation*, and the one member here whose grant is a
      decision rather than a lookup: it creates a directory the program did not name, so what
      `fs.write` means for it belongs in the row's card and in
      `crates/nvs-stdlib/src/registry.rs:1107`. `examples/files.nvs` opens on it.
- [ ] **`within`, the path-traversal launderer.** Spec § 14 *Resolution* over ADR 0024 § 3 — it
      resolves and *then* proves containment, which is the check `Core\Path::normalize` structurally
      cannot make, so its return type is the plain `string` and its `$path` parameter takes
      `tainted`. Row at `crates/nvs-stdlib/src/io.rs:41`, arm at `crates/nvs-stdlib/src/io.rs:132`,
      grant at `crates/nvs-stdlib/src/registry.rs:1107`. Acceptance names `within_resolves_and_then_proves_containment` and
      `within_refuses_a_path_that_escapes_through_dotdot_or_a_symlink`, `-p nvs-stdlib`, beside
      `crates/nvs-stdlib/tests/capability.rs`'s existing path cases.

## Backlog

- Stage 2's handle half — `open`, the `FileMode` enum, `Core\IO\File` and its nine instance members;
  spec § 14 *Handles*, acceptance `an_open_file_is_an_object_and_never_a_resource` and
  `a_file_mode_is_an_enum_and_never_a_string`.
- `writeStream` — ADR 0105 § 4, acceptance `write_stream_defaults_to_no_overwrite` and
  `a_write_stream_that_fails_midway_removes_the_partial_file`; goal 6's uploads are its second caller.
- `no_member_dispatches_on_a_uri_scheme` — ADR 0052, a source scan over `nvs-stdlib` rather than a
  blocklist, in the shape of `nvs_stdlib_reaches_the_os_only_through_the_gate`.
- The three fixtures owing configuration, named in `## State`; `docs/agent/loop-goal.toml` stages 5
  and 7 own them.
- `orient.py`'s two dead `[context] modules` selectors, in `docs/agent/loop-goal.toml`.
