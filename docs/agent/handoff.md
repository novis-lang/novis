# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 2 is landing `Core\IO`'s member
list.** `exists`, `size`, `remove`, `removeDir` and `temporaryDir` now have rows, reference cards,
bodies, `address` arms and `CAPABILITIES` entries; the class had two members and has seven.

**Five new doors in `crates/nvs-runtime/src/capability.rs`** — `metadata`, `exists`, `remove_file`,
`remove_dir` and `temp_dir` — because ADR 0118 § 2 puts every effect behind one and
`nvs_stdlib_reaches_the_os_only_through_the_gate` refuses a `std::fs` spelling in `nvs-stdlib`. Each
door's own doc comment owns its reasoning; the two decisions worth knowing are that removal is
`fs.write` rather than a capability of its own, and that `temporaryDir` **chooses its name first and
asks about it second**, so an operator grants the temporary root (or `true`) exactly as they would
any other path — the member creates a directory the program never named and is still not exempt.

**One decision this session deferred, and the `within` slice owes it.** Every `Core\IO` path
parameter is `Qual::Neutral`, including the five added here, matching the `read`/`write` rows that
were already on disk — but spec § 14's opening sentence calls every member a **path sink**, which is
`Qual::Sink`. Flipping half the class is worse than either end state, so the whole class flips
together in the slice that adds `within`, which is the launderer that makes the flip usable.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]` block
naming `examples/logging/handler.nvs`; and every fixture that reaches the world needs its `fs.*` /
`process.exec` / `net.connect` grants in `nvs.toml` — `examples/files.nvs` now needs `fs.write` to
cover the **temporary root**, not just the working directory.

`orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs` and
`crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**What `examples/files.nvs` still names, then the fixture's own configuration** — one file set
throughout: `crates/nvs-stdlib/src/io.rs` (the rows at `:41`, the cards after them, the `address`
arm), `crates/nvs-runtime/src/capability.rs:98`'s doors and
`crates/nvs-stdlib/src/registry.rs:1107`'s `CAPABILITIES`. Every member owes **three** `.nvst` cases,
not one — the playbook's *Writing a test case* now carries why.

- [ ] **`within`, the path-traversal launderer, and the class's `Qual` flip with it.** Spec § 14
      *Resolution* over ADR 0024 § 3: it resolves and *then* proves containment, so its `$path` is
      the qualifier-removing parameter and its return type is the plain `string`. Row at
      `crates/nvs-stdlib/src/io.rs:44`, arm at `crates/nvs-stdlib/src/io.rs:296`, grant at
      `crates/nvs-stdlib/src/registry.rs:1107`, and a `canonicalize` door beside
      `crates/nvs-runtime/src/capability.rs:98` — `nvs_config::resolve::Files::canonical` already
      resolves the deepest existing ancestor, which is what a `within` over a path that does not
      exist yet needs. Acceptance names `within_resolves_and_then_proves_containment` and
      `within_refuses_a_path_that_escapes_through_dotdot_or_a_symlink`, `-p nvs-stdlib`, beside
      `crates/nvs-stdlib/tests/capability.rs:43`'s existing path cases. Flip every `Qual::Neutral`
      path parameter in `crates/nvs-stdlib/src/io.rs` to `Qual::Sink` in the same slice, with one
      case proving a `tainted` path is refused and `within`'s result is not.
- [ ] **The text half: `readText` and `lines`.** Spec § 14 *Whole-file*. `readText` decodes where
      `read` hands back bytes — `crates/nvs-stdlib/src/encoding.rs` owns the `bytes`→`string`
      boundary and the `{charset?}` option is a trailing shape; `lines` answers an `Iterable<string>`
      and must not hold the file, so it is a cursor over the handle rather than a `Vec` —
      `crates/nvs-stdlib/src/cursor.rs:1` is the shape the other collections use. Rows at
      `crates/nvs-stdlib/src/io.rs:44`, bodies after `crates/nvs-stdlib/src/io.rs:296`.
- [ ] **`examples/files.nvs`'s own `nvs.toml`, and stage 2's acceptance line.** The fixture opens on
      `Core\IO::temporaryDir()` and then writes inside what it got, so the grant has to cover the
      temporary root and not only `.`; `tests/conformance/core/io-a-temporary-directory-is-a-granted-path-like-any-other.nvst:1`
      is the case that pins why. Then check the seven frozen lines in `docs/agent/loop-goal.toml`
      against what the binary actually prints — `examples/files.nvs:22` is the first line that runs.

## Backlog

- `Core\IO`'s remaining § 14 members — `append`, `writeStream`, `isFile`, `isDir`, `copy`, `move`,
  `makeDir`, `list`, `walk`, `canonicalize`, the `open`/`File` handle half (docs/spec/01-core-library.md § 14).
- `docs/agent/loop-goal.toml`'s `[context] modules` names two paths that match no module.
- The `Core\IO` reference chapter has no example for the new members (docs/reference/core/IO.md).
