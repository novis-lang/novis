# Handoff

## State

**ADR 0086's `Core\Cli` surface gained the two depth cases `gaps.py` named as its thinnest** —
`displayWidth`'s composition and `arguments`' word-as-value. Two new `.nvst` cases and two doc
comments; no behaviour changed anywhere, so no new refcount edge and no valgrind run. Conformance
1344.

**`Core\Cli::displayWidth` is not a sum over classes of codepoint, which is what the item predicted.**
It is a fold over ADR 0009 § 2's grapheme clusters that asks each *whole* cluster for its width
(`nvs_runtime::terminal::advance`, over `unicode-width`), so a ZWJ emoji sequence is **2** columns
where a per-codepoint sum says 4, and a ZWJ couple carrying a variation selector is 2 where the sum
says 5. What landed is the law that is actually true and stronger: both measures are cluster-shaped,
so `displayWidth` is `Core\Str::length` plus exactly the wide clusters — pinned over a five-sample
sweep by counting, against the codepoint sum that gets three of five right.

**A leading `--` never reaches the program**, and that is the launcher's rule rather than the
member's: `nvs run`'s `arguments` is `trailing_var_arg`, so clap spends one `--` as its escape token
in that position and nothing later. Both homes now say so — `crates/nvs-cli/src/main.rs`'s
`arguments` field and `nvs_test::case::Case::args` — and the playbook carries the case author's half,
including that `--ARGS--` cannot express an empty word at all (the item asked for one).

**The acceptance check still names `every_part_two_spec_member_is_registered`**, and no test by that
name is on disk in any crate. It is stage 10's gate over a complete Part II and needs spec §§ 15-19
from goal 6. Not a regression and not closable here. Nothing was missing from this session's pack.

## Next group

**`Core\Env` is `gaps.py`'s thinnest class that needs neither a fixture nor a capability — one file,
`crates/nvs-stdlib/src/env.rs`, plus `tests/conformance/core/`.** `--ENV--` is honoured, and its
value is the whole of the line past the first `=`, which is what makes both of these writable. The
three cases on disk pin the ordering, the taint and that an absent name is absent from both answers;
neither question below is one of those.

- [ ] **`Core\Env::get` answers a value whole, and a value is not a syntax** — a value holding `=`,
      one holding a `;`, one wrapped in quotes and one holding leading spaces all arrive as
      themselves, since nothing between the process's environment and this member has an opinion
      about what a value means. The shape is this session's `arguments` case: assert by counting a
      pairwise match against the literals, so a member that split at the second `=` or stripped a
      quote fails on a count rather than on a line. `crates/nvs-stdlib/src/env.rs:188`.
- [ ] **An empty value is a value and not an absence** — `NVS_CASE_EMPTY=` makes `get` answer `""`
      rather than `null`, the name appears in `all` beside the ones holding text, and the `??` an
      absent name falls through does not fire. That is ADR 0063's "absence is `?T`" asserted on both
      sides of its bound, and it is the one state of a variable no case on disk asks about.
      `crates/nvs-stdlib/src/env.rs:215`, `crates/nvs-stdlib/src/env.rs:188`.

## Backlog

- `Core\Cldr::pluralCategory` is four cases over one member — `crates/nvs-stdlib/src/cldr.rs`.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case, and an oracle
  case goes in `tests/differential/` — `crates/nvs-stdlib/src/task.rs:561`, `python tools/gaps.py`.
- `Core\Http\Response`'s `status`/`text` are the thinnest pair on disk but need a response to hold.
- `crates/nvs-stdlib/src/env.rs:200`'s thrown path (a value that is not UTF-8) is unreachable from
  `--ENV--`, which writes text; it needs a launcher-side fixture or nothing.
- Stage 10's `every_part_two_spec_member_is_registered` waits on goal 6's spec §§ 15-19.
