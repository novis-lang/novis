# Handoff

## State

**Spec § 14's three free members are closed for depth** — `Core\IO::within`, the `lines`/`readText`
pair, and `writeStream`'s `overwrite`. Three new `.nvst` cases, no Rust change in `nvs-stdlib`, so no
new refcount edge and no valgrind run. Conformance 1342.

**The `overwrite` item as handed over named the wrong member.** The option is `writeStream`'s
(`WRITE_STREAM_OPTIONS`, `crates/nvs-stdlib/src/io.rs:279`); `Core\IO::write` has no options at all
and replaces without asking. Both sides of the bound were already pinned by
`io-write-stream-lands-its-chunks-in-order-and-refuses-to-replace.nvst`, so what landed instead is
the gap that was open: the bound is about a name being *taken* rather than about what is in the file
— an empty file refuses as an occupied one does — and with `overwrite: true` the member answers what
`Core\IO::write` answers on every state of the path, which is the whole difference between the two
whole-file writers.

**`Core\IO::lines` is `read`'s reading and not `readText`'s**, which its own doc comment says and
nothing asserted. The existing agreement case checks `lines` against `Core\Str::lines`, the *same*
splitter, so a splitter wrong about a trailing terminator agrees with itself; `readText` answers the
file undivided and is the independent oracle the new case rebuilds against.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a complete Part II, which needs spec §§ 15-19 from goal 6. Not a regression and not closable here.
Nothing was missing from this session's pack.

## Next group

**ADR 0086's terminal surface is one file — `crates/nvs-stdlib/src/cli.rs` and
`tests/conformance/core/` — and `python tools/gaps.py` names these two members as `Core\Cli`'s
thinnest.** Both are pure questions about a value or about the process, so neither needs a terminal,
a capability or a fixture.

- [ ] **`Core\Cli::displayWidth` is a sum over classes of codepoint, not over characters** — the
      three existing cases ask about a tab, a newline and a table's alignment; what none asks is the
      width of a zero-width combining mark, of a wide CJK codepoint and of a ZWJ emoji sequence,
      counted against `Core\Str::length`'s cluster count so that the two disagree by exactly the
      wide columns. `crates/nvs-stdlib/src/cli.rs:1345`.
- [ ] **`Core\Cli::arguments` answers words, and a word is a value** — a word that looks like an
      option, an empty word and a word holding a separator all survive as themselves, which the
      three existing cases (the words, the empty program, the taint) do not reach.
      `crates/nvs-stdlib/src/cli.rs:1146`.

## Backlog

- An empty environment value is a value and not an absence — the boundary PHP's `getenv` conflates,
  unpinned by any of the four `env-*` cases. `crates/nvs-stdlib/src/env.rs:188`.
- `Core\Env::get` refuses a value that is not UTF-8 (`crates/nvs-stdlib/src/env.rs:200`) — check
  first whether `--ENV--` can carry a non-UTF-8 byte at all; `crates/nvs-test`'s module doc owns that.
- `Core\Csv::format`'s throw for a column that is not a `string` is unasserted —
  `crates/nvs-stdlib/src/csv.rs:610`, from `python tools/gaps.py --errors`.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case —
  `crates/nvs-stdlib/src/task.rs:561`, and it belongs in `tests/differential/`.
- `Core\Http\Response` (`status`, `text`) and `Core\Mail::send` are `gaps.py`'s thinnest classes;
  both need the transport's test hook rather than a network.
