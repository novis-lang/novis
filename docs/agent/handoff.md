# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 2 is landing `Core\IO`'s member
list.** Nine members now: `read`, `readText`, `write`, `exists`, `size`, `remove`, `removeDir`,
`temporaryDir` and `within`. `examples/files.nvs` still names `lines`, which is what the driver's
acceptance check now reports; that is an item still open, not a regression.

**`readText` is `read` plus spec § 7's exact decode, and deliberately not a second reader.** The
door, the buffer and the ceiling are `read`'s — both bodies go through the new `slurp` in
`crates/nvs-stdlib/src/io.rs` — and the conversion is `Core\Encoding`'s own, reached through
`crate::encoding::decode_argument`. `{charset?}` is ADR 0063 R3's one trailing shape, defaulting to
`Core\Charset::Utf8`, because that decode is the identity on a file already written the way Novis
spells text. The class's module doc owns the rest of why.

**The decode refusal was reworded, and both classes share the one sentence.** Its literal head is
now the rule (`a conversion is exact or it throws: …`) with the member in the interpolated tail, so
`conformance_coverage.rs` can read a stem off the site at all — the playbook's *Writing a test
case* bullets own both halves of that. `Core\Encoding::decodeText` produces the same sentence, and
its case's five expectation lines plus one substring sweep moved with it.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]`
block naming `examples/logging/handler.nvs`; and every fixture that reaches the world needs its
`fs.*` / `process.exec` / `net.connect` grants in `nvs.toml` — `examples/files.nvs` needs `fs.write`
covering the **temporary root**, not just the working directory.

`orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs` and
`crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**The last member `examples/files.nvs` names, then the fixture's own configuration** — one file set
throughout: `crates/nvs-stdlib/src/io.rs`, `crates/nvs-stdlib/src/cursor.rs`,
`crates/nvs-stdlib/src/registry.rs` and `examples/files.nvs` beside the `nvs.toml` it still owes.
Every member owes **three** `.nvst` cases, and a refusal message whose literal head is backslash-free
and not an interpolation.

- [ ] **`lines`, and the `Iterable<string>` it answers.** Spec § 14 *Whole-file*:
      `lines(string $path): Iterable<string>`, replacing `file` and `fgets`. Row beside `readText`
      at `crates/nvs-stdlib/src/io.rs:155`, card after `crates/nvs-stdlib/src/io.rs:417`, `address`
      arm at `crates/nvs-stdlib/src/io.rs:455`, body beside `slurp` at
      `crates/nvs-stdlib/src/io.rs:526`, grant (`fs.read`) at
      `crates/nvs-stdlib/src/registry.rs:1117`. **No new door** — `slurp` is already the whole of
      the read. The decision the slice owes: `crates/nvs-stdlib/src/cursor.rs:102`'s `over(NvsArray)`
      is a **snapshot** cursor, so answering it means holding every line at once, and
      `examples/files.nvs:36`'s comment ("the file is not held") is a fixture comment and not a
      spec — decide and say which, in the member's doc comment. `Iterable<T>`'s element type is
      declared at `crates/nvs-stdlib/src/registry.rs:1402`'s `ITERABLES`.
- [ ] **`examples/files.nvs`'s own `nvs.toml`, and stage 2's acceptance line.** The fixture opens on
      `Core\IO::temporaryDir()`, so its `fs.write` grant has to cover the **system temporary root**
      and not just `.`; `crates/nvs-stdlib/src/io.rs:526`'s door refuses against the canonical
      spelling. The seven expected lines are frozen in `docs/agent/loop-goal.toml` (stage 2) and
      `examples/files.nvs:19` says so.

## Backlog
- Two dead `[context] modules` selectors in `docs/agent/loop-goal.toml` (`nvs-host/src/pool.rs`, `stream.rs`).
- Spec § 14's unbuilt half: `append`, `writeStream`, `isFile`/`isDir`/`stat`, `copy`/`move`/`makeDir`/`list`/`walk` — `docs/spec/01-core-library.md:1002`.
- `examples/http.nvs` needs stage 5's pinned origin — `docs/agent/loop-goal.toml`.
- `examples/logging.nvs` needs an `[[app]]` block naming its handler — `docs/agent/loop-goal.toml`.
