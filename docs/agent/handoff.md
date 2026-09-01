# Handoff

## State

**`Core\Cli`'s value types are closed for depth.** Two counted sweeps over ADR 0086 § 1's whole
control range landed — every C0, `DEL` and every C1 — one through `Core\Cli\Live::set` and one
through `Core\Cli\Text::styled`. Both read a carrier back with `as string` rather than echoing it,
so the sink is out of the path and what is asserted is what the constructor stored (ADR 0088 § 5).
No Rust change in `nvs-stdlib`, so no new refcount edge and no valgrind run.

**Two of the previous group's three items were already on disk**, and the handoff had proposed them
anyway: `cli-a-styles-seven-slots-are-independent.nvst` (33199eaa) is item 1 verbatim and
`cli-every-colour-argument-shares-one-bound-counted.nvst` (74a1bb66) is item 2. The playbook bullet
above is why that was not visible from `gaps.py`'s ranking. Item 3 — the region as a sink — was
genuinely open and is what landed, with the `styled` sweep beside it as the same question asked of
the constructor that emits control bytes on purpose.

**`tools/try.py` decoded a subprocess with the console code page**, so any case echoing one of § 1's
Control Pictures (`␛` is `E2 90 9B`) crashed it under Windows cp1252 instead of failing a
comparison. It reads UTF-8 with `errors="replace"` now.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a complete Part II, which needs spec §§ 15-19 from goal 6. Not a regression and not closable
here. Nothing was missing from this session's pack.

## Next group

**`Core\IO\File` is `gaps.py`'s thinnest cluster that needs no Docker and no network — `lock` 3,
`truncate` 3, `flush` 4 — and the three are one file set: `crates/nvs-stdlib/src/io.rs` and
`tests/conformance/core/`. `docs/spec/01-core-library.md` § 14's handle roster is the spec for all
three, and ADR 0118 §§ 2-3's doors are what each of them sits behind.** Read the three bodies
before fixing the claims below: this session did not open `io.rs`, so each item names the shape and
the anchor and leaves the boundary to the session that reads it.

- [ ] **`truncate`'s edges, and where the cursor lands** — a length past the end, a length of zero
      and a length equal to the current one are three different answers, and a handle's position
      after each is the part a single-line case does not ask about.
      `crates/nvs-stdlib/src/io.rs:1681`.
- [ ] **`flush` is an agreement, not an answer** — what a second reader sees before and after it,
      asked of the same handle, so a `flush` that only satisfied its own buffer fails here.
      `crates/nvs-stdlib/src/io.rs:1721`.
- [ ] **`lock`'s bound on both sides** — the state it refuses and the state it grants, named
      together, plus what a released lock leaves behind. `crates/nvs-stdlib/src/io.rs:1784`.

## Backlog

- `Core\Cldr::pluralCategory` at 4 and one member — `crates/nvs-stdlib/src/cldr.rs`, spec § 4.
- `Core\Cli` itself is thinner than its value types now: `arguments`, `displayWidth`, `height` at 3.
- `Core\Cli\Progress::advance` at 5 — the bar is unobservable without a terminal, so only the
  member's answers can be asserted; `crates/nvs-stdlib/src/cli.rs`'s `bar` is the arithmetic.
- `Core\Test` at `advance`/`assertContains`/`scriptAnswers` 3 — ADR 0079 § 4.
- Stage 10's `every_part_two_spec_member_is_registered` waits on spec §§ 15-19 (goal 6) —
  `docs/agent/loop-goal.toml`.
- `Core\Http\Response` and `Core\Mail::send` rank thinner still, and both need the outside world.
