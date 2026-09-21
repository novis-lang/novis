# Handoff

## State

Goal `core-cli-progress-and-6-more`: eight of its sixteen features are done — `Core\Cli\Progress::advance`,
`Core\Cli\Style::of`, all three of `Core\Cli\Text` and all three of `Core\Command` each carry an
`about.md`, three examples with blessed `.out` files, one attack, one bench with a judged declaration,
and a Rust `#[test]` with its `covers:` marker. `python tools/dossier.py --group 'Core\Command'` shows
every column filled.

`Core\Command::completions`'s attack found two shell-injection paths in the scripts it writes, and both
are fixed in this group rather than recorded (`rule:testing/a-failing-proof-is-fixed-or-recorded`): a
command name's colon ended a `zsh` `_describe` entry early, and `bash` expands every word of a
`compgen -W` list, so a name spelled `run$(id)` was a command the user's own shell ran on Tab. One
`.nvst` case pins each.

Nothing is blocked. The remaining eight items are `Core\Compress`'s four members and its two stream
classes, all in `compress.rs`.

## Next group

**`Core\Compress`'s four members** — one file set, none of it opened yet:
`crates/nvs-stdlib/src/compress.rs`, `docs/examples/core/Compress/`, `tests/hostile/core/Compress/`,
`benches/members/core/Compress/`. All four are `rule:testing/feature-proofs`, and `Core\Command`'s
three landed features are the model for what each proof looks like. The four share one fixture: a
program that compresses a buffer under a named codec and reads it back.

- [ ] **`Core\Compress::compress`** — owes about, examples, hostile, tests.
      `crates/nvs-stdlib/src/compress.rs:197`. It is the whole-buffer half every other member is
      compared against, so its examples are what the other three build on.
- [ ] **`Core\Compress::decompress`** — owes about, examples, hostile, tests.
      `crates/nvs-stdlib/src/compress.rs:206`. Its attack's subject is
      `rule:core-classes/decompression-bound`: the bound on what a stream may expand to cannot be
      switched off, so a compression bomb is what the file should try.
- [ ] **`Core\Compress::compressor`** — owes about, examples, hostile, tests.
      `crates/nvs-stdlib/src/compress.rs:223`. The streaming half, which answers a
      `Core\Compress\Compressor` rather than a buffer.
- [ ] **`Core\Compress::decompressor`** — owes about, examples, hostile, tests.
      `crates/nvs-stdlib/src/compress.rs:232`.

## Backlog

- `Core\Compress`'s two stream classes are the rest of this goal's roster, after the four members above.
- `fish` re-expands the argument of `complete -a`, and whether a command name reaches that expansion is
  **not checked** — `crates/nvs-stdlib/src/command.rs` § *What a completion script completes*.
- A bench for `Core\Command::run` measures the usage-error path only, because nothing gives a bench
  program a command line — `benches/members/README.md` names no directive for one.
