# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 2 is landing `Core\IO`'s member
list.** The class has eight members: `read`, `write`, `exists`, `size`, `remove`, `removeDir`,
`temporaryDir` and now `within`. `examples/files.nvs` still names `readText` and `lines`, which is
what the driver's acceptance check reports; that is an item still open, not a regression.

**Every path parameter in `crates/nvs-stdlib/src/io.rs` is now `Qual::Sink`, and the deferred
decision the last session recorded is closed.** ADR 0088 § 1's table classifies a filesystem path
component as an instruction — `..` and the separators direct the resolver — so spec § 14's opening
sentence is that predicate applied rather than a second rule. The whole class flipped in one slice
because half of it marked teaches the wrong rule. `Core\Path` stays `Qual::Contagious`; its own
module doc owns why, and the difference is that nothing in § 8 resolves anything.

**`within` is the class's one `Qual::Launder` row** — `$base` a sink, `$path` the laundered half,
answering the plain `string` ADR 0024 § 3 requires. It resolves through
`nvs_runtime::capability::canonicalize`, a sixth door behind `fs.read`, and then proves containment
with `Path::starts_with` in `io.rs` itself, which is the whole of what the body owns. The door
reuses `nvs_config::capability::resolved` — made `pub` for exactly this — so containment is proved
about the same resolution the grant was compared against; a second canonicalizer is how a `..` gets
through. The refusal names the base and the caller's own argument and never where the argument led.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]`
block naming `examples/logging/handler.nvs`; and every fixture that reaches the world needs its
`fs.*` / `process.exec` / `net.connect` grants in `nvs.toml` — `examples/files.nvs` needs `fs.write`
covering the **temporary root**, not just the working directory.

`orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs` and
`crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**What `examples/files.nvs` still names, then the fixture's own configuration** — one file set
throughout: `crates/nvs-stdlib/src/io.rs` (rows at `:53`, cards after them, the `address` arm and
the bodies), `crates/nvs-runtime/src/capability.rs:100`'s doors and
`crates/nvs-stdlib/src/registry.rs:1117`'s `CAPABILITIES`. Every member owes **three** `.nvst`
cases — the playbook's *Writing a test case* carries why — and, now, a refusal message whose literal
head is backslash-free, per the *Tooling* bullet this session added.

- [ ] **`readText`, the decoding half of the whole-file read.** Spec § 14 *Whole-file*:
      `readText(string $path, {charset?}): string`, which is `read` plus the `bytes`→`string`
      conversion `Core\Encoding` already owns — `crates/nvs-stdlib/src/encoding.rs:336` is the
      charset enum and the conversion beside it. Row beside `within` at
      `crates/nvs-stdlib/src/io.rs:104`, card after `crates/nvs-stdlib/src/io.rs:305`, `address` arm
      at `crates/nvs-stdlib/src/io.rs:359`, body after `crates/nvs-stdlib/src/io.rs:492`, grant
      (`fs.read`) at `crates/nvs-stdlib/src/registry.rs:1117`. It needs **no new door** —
      `nvs_runtime::capability::open_read` at `crates/nvs-runtime/src/capability.rs:101` is the one
      `read` already uses. The options bag is one trailing shape (ADR 0063 R3); `CoreTy::Options` at
      `crates/nvs-stdlib/src/regex.rs:109` is the row shape to copy.
- [ ] **`lines`, and the `Iterable<string>` it answers.** Spec § 14 *Whole-file*:
      `lines(string $path): Iterable<string>` — the file is **not** held, which is the whole reason
      it is not `array<string>`. The shape to read first is
      `crates/nvs-stdlib/src/cursor.rs:1` — ADR 0053 § 1's `Iterator<T>` half, over a snapshot list —
      and `CoreTy::Instance`'s value side at `crates/nvs-stdlib/src/instance.rs:1`. Budget this as a
      full slice on its own: it is the first `Core` member answering a lazy sequence over an open
      handle, and `crates/nvs-runtime/src/capability.rs:101`'s `open_read` hands back the `File` it
      needs.
- [ ] **`examples/files.nvs`'s own `nvs.toml`, and stage 2's acceptance line.** The fixture opens on
      `Core\IO::temporaryDir()`, so the grant has to cover the **temporary root** and not only the
      working directory — `crates/nvs-runtime/src/capability.rs`'s `temp_dir` doc owns why the name
      is chosen first and asked about second. The seven lines it prints are frozen in
      `docs/agent/loop-goal.toml` (stage 2); check them against the fixture's own `echo`s at
      `examples/files.nvs:28`.

## Backlog

- `Core\IO`'s `canonicalize` member — spec § 14 *Resolution*'s other half; the door exists already.
- `Core\Path`'s `Qual::Contagious` rows are correct today, but § 8's module doc at
  `crates/nvs-stdlib/src/path.rs:70` should point at ADR 0088 § 1's table rather than re-argue it.
- Spec § 14's *Metadata* bullet still owes `isFile`, `isDir`, `isReadable`, `isWritable`,
  `modifiedAt` and `stat`; `metadata` is one door and one `stat` for all of them.
- `orient.py`'s two dead `[context] modules` selectors — `docs/agent/loop-goal.toml`.
