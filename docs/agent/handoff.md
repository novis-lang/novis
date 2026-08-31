# Handoff

## State

**ADR 0086 § 1 has landed: terminal output is a sink.** `echo` neutralizes every control byte
before a byte reaches the stream — `nvs_echo_str` at `crates/nvs-runtime/src/helpers.rs:2422` is
the site and its doc comment owns the reasoning, including why the table's idempotence is what
makes `echo`ing a captured `Core\Cli\Text` correct and why an ill-formed byte goes through
`from_utf8_lossy`. The table itself was already on disk as `nvs_render::text::substitute`
(`crates/nvs-render/src/text.rs:56`), written for ADR 0092 § 5; it is **called, not restated**, so
the sink, `Core\Cli::escape` and a diagnostic record cannot come to disagree. It now answers a
`Cow`, so ordinary text — nearly every write — costs one scan and no allocation.

**`Core\Cli` is five members**: `escape`, `isTty`, `width`, `height`, `colorDepth`. `escape` is
ADR 0024 § 3's named launderer for this sink, a `Qual::Launder` row, and exists for the program
that wants the neutralized form *as a value*; the module doc's gap list is down to `write`,
`displayWidth` and the rest of § 13.

**What `[3 process and terminal]`'s `Core\Cli` check still owes**: `styling_is_a_value_type_and_never_a_grammar`,
`a_prompt_reads_the_controlling_terminal_and_not_stdin`, `no_prompt_blocks_without_a_deadline` and
`a_live_region_is_scoped_and_restores_the_terminal_on_a_panic`. Stage 2 still owes `truncate`,
`lock` and `Core\IO::stdin`/`stdout`/`stderr`, which is where the previous handoff pointed and
which the driver's failing check outranked.

**§ 1's second mechanism is not built**: an `ESC` in a source literal is still not a compile
error, so `"\e"` compiles. The three new cases build their control bytes with
`Core\Str::fromCodePoint` for that reason and stay correct once it lands.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field. § 15's `Core\Cli` bullet (`docs/spec/01-core-library.md:1079`) is where this
member's signature came from, and this session paid a call for it as the last three did.

## Next group

**ADR 0086 § 2's styling half, over `crates/nvs-stdlib/src/cli.rs` and
`crates/nvs-runtime/src/helpers.rs` — the same two files this session held.**

- [ ] **`Cli\Text::plain` and `Cli\Text::styled`, and the raw path the sink must not neutralize**
      — ADR 0086 § 1's *"there is exactly one raw path, `Cli\Text`"* and § 2. **Decide this
      first, because it is the one thing the landed sink constrains**: a `Text` built by
      `styled` holds real `ESC` bytes, and `echo` of it lowers through
      `value_to_string`'s carrier arm (`crates/nvs-runtime/src/helpers.rs:1649`) to a
      `Tag::Str` that `nvs_echo_str` (`crates/nvs-runtime/src/helpers.rs:2422`) would then
      substitute. Nothing is broken today — the only producer is `Core\Out::capture`, whose
      bytes are already neutralized — so the choice is between echoing a carrier without going
      through `EchoStr` and marking the value. Rows and cards beside `escape`'s at
      `crates/nvs-stdlib/src/cli.rs:101` and `crates/nvs-stdlib/src/cli.rs:156`, the class at
      `crates/nvs-stdlib/src/cli.rs:502`, the `address()` arm at
      `crates/nvs-stdlib/src/cli.rs:346`, the builder at `crates/nvs-stdlib/src/cli.rs:512`.
- [ ] **`Cli\Style` and `Cli\Color`, and `styling_is_a_value_type_and_never_a_grammar`** — ADR
      0086 § 2, whose whole content is that neither is a grammar, so ADR 0063 R11's count stays
      at four. `Style::of({color, bold, …})` is one trailing options shape. Enums beside
      `COLOR_DEPTH` at `crates/nvs-stdlib/src/cli.rs:265`, registered in
      `crates/nvs-stdlib/src/registry.rs:1596`; the test goes beside
      `terminal_output_substitutes_a_control_sequence_visibly` at
      `crates/nvs-stdlib/src/cli.rs:517`.
- [ ] **`Core\Cli::write(Cli\Stream $stream, …)`** — spec § 15's roster, the second spelling of
      the sink `echo` already is, so it owes a row and a stream argument rather than a rule.
      Row and card beside `escape`'s at `crates/nvs-stdlib/src/cli.rs:101` and
      `crates/nvs-stdlib/src/cli.rs:156`, `address()` arm at
      `crates/nvs-stdlib/src/cli.rs:346`.

## Backlog

- `Core\IO\File::truncate` and `::lock`, and `Core\IO::stdin`/`stdout`/`stderr` — spec § 14's
  roster, `crates/nvs-stdlib/src/io.rs:792`.
- ADR 0086 § 4's prompts (`ask`, `confirm`, `select<T>`, `secret`) and § 5's `live<T>` — three
  more of `[3 process and terminal]`'s named tests.
- ADR 0086 § 1's second mechanism: an `ESC` in a source literal is a compile error, a new
  `E02xx` code — `crates/nvs-types/src/string_lit.rs:255`.
- Stage 8's reflective property write — `docs/plan/m8.md`.
- `docs/spec/01-core-library.md` is in no `[context]` field of `docs/agent/loop-goal.toml`.
