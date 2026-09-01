# Handoff

## State

**Spec § 14 is one member from complete.** `Core\IO::modifiedAt`, `::stat`, `::isReadable` and
`::isWritable` are on disk over six new conformance cases, together with the class `stat` answers
with, and the ratchet `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is down from 51
keys to 47. `walk` is all that is left of § 14.

**`Core\IO\Metadata` is a new registry class, and its own doc comment owns every decision in it**: an
instance rather than an ADR 0036 shape because `CoreTy` has no spelling for a shape *return*; four
members — `size`, `modifiedAt`, `isFile`, `isDir` — that duplicate `Core\IO` statics without being
ADR 0063 R17's two spellings, because a static's subject is a path and a syscall while a member's is
a snapshot and a slot read; and no permission member, since `fileperms` has no portable content and
the question a program asks is `isReadable`/`isWritable`'s. `crate::time::instant_at_system_time` is
the seam that keeps `Instant`'s two-slot representation inside `time.rs`.

**The access pair is two gates and they answer differently on purpose.** The capability *refuses* a
path outside the grant where the operating system answers `false`; folding the first into the second
would hand any program a boolean to map its own configuration with.
`nvs_runtime::capability::readable`/`::writable` own that, and own the `access(2)`-on-unix /
read-only-attribute-on-Windows split. `isWritable` declares **`fs.write`**, which is this table's one
row decided by what a question is *about* rather than by what its member does.

**Stage 10's remaining open check is still the goal's own measure**: `check-migration --min 74` reads
37%, unmoved by these six.

Nothing was missing from this session's pack.

## Next group

**`walk` closes § 14, then § 15's `Core\Env` half opens. All four share
`crates/nvs-stdlib/src/registry.rs` and `tests/conformance/core/`; the first adds `io.rs` and
`cursor.rs`, the rest are one file, `env.rs`.**

- [ ] **Register `walk`, § 14's streaming listing and its last outstanding key.** The
      `Iterable<string>` half of `list`, over the same `capability::read_dir` door. `Core\IO\Lines`
      is the shape to copy — a slotted class with no members whose `iterate` is dispatched by name —
      and its own docs argue why it holds rather than streams, which is the decision `walk` has to
      make again over a directory rather than a file.
      `crates/nvs-stdlib/src/io.rs:1610`, `crates/nvs-stdlib/src/io.rs:3001`,
      `crates/nvs-stdlib/src/cursor.rs:102`.
- [ ] **Register `Core\Env`'s `EOL`, `OS` and `VERSION` constants.** That module's own gap 1 is the
      home of the decision each carries — `EOL` in particular, which is fixed by the machine that
      *compiled* the program in PHP and must not be here.
      `crates/nvs-stdlib/src/env.rs:96`, `crates/nvs-stdlib/src/env.rs:79`.
- [ ] **Register `Core\Env::mode`, the run mode.** The same module's gap 1 names what it reads;
      a `CoreEnum` beside it is the shape, since R11 forbids a mode string.
      `crates/nvs-stdlib/src/env.rs:96`.

## Backlog

- `§15 Cap::has` — the last non-request § 15 key; `docs/spec/01-core-library.md` § 15.
- § 15's `Request`, `Response` and `Session` rosters need a request: goal 6's, per the ratchet's own
  section comment.
- § 18's `Core\Db` needs a driver and a server: goal 5's, and blocked on a Docker daemon.
- `check-migration --min 74` reads 37% and is the goal's own remaining gate —
  `docs/agent/loop-goal.toml`.
- ADR 0063 R18's M4S check ("no static member's name collides with an instance method on a type its
  own class constructs") would flag `Core\IO::size` today; `crates/nvs-stdlib/src/io.rs`'s
  `METADATA` doc is the argument for why that wording is narrower than the ADR body's rule.
