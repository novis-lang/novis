# Handoff

## State

**`Core\IO\File`'s three thinnest members are closed for depth** — `truncate`, `flush` and `lock`
each gained the case its existing ones could not distinguish from their own opposite. No Rust
change in `nvs-stdlib`, so no new refcount edge and no valgrind run.

**The three existing `truncate` cases only ever ask about a size below the position**, so a member
that repaired the position on a grow, or skipped the call when the size already matched, was green
on every line of all three. The new case is the counted sweep over all twelve relations, plus the
equal-length row spelled out.

**The existing `flush` cases write, flush, then read**, which passes whether or not the member does
anything — the module doc's claim (no buffer in front of the descriptor) is only observable from a
second handle reading *before* any flush, at sizes on both sides of a plausible buffer.

**The three existing `lock` cases all ask about one file**, so a lock that was one flag on the
program passed every one of them. The new case names the bound from outside: three files, three
locks at once, a second spelling of a held path refused, and a `close` that frees only what it
named.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a complete Part II, which needs spec §§ 15-19 from goal 6. Not a regression and not closable
here. Nothing was missing from this session's pack.

## Next group

**`Core\IO`'s free members are the same file set one level up — `crates/nvs-stdlib/src/io.rs` and
`tests/conformance/core/` — and the three below share `docs/spec/01-core-library.md` § 14's roster
with the handle members just closed.** `Core\IO::within` is the path-containment predicate every
other member's capability check is written against, `Core\IO::lines` is the streaming half of
`readText`, and `Core\IO::write`'s `overwrite` option is a bound with two sides and one case.

- [ ] **`Core\IO::within` is a containment question about paths, not about a disk** — what it
      answers for a path that escapes upward, for one that only looks like it does, and for a base
      that is not a prefix in the string sense but is one in the path sense.
      `crates/nvs-stdlib/src/io.rs:2124`.
- [ ] **`Core\IO::lines` and `readText` are one reading of the same bytes** — the agreement over a
      final terminator, a file with none, an empty file and a lone terminator, counted rather than
      read off four lines. `crates/nvs-stdlib/src/io.rs:1233`.
- [ ] **`overwrite` is a bound with two sides** — the state `Core\IO::write` refuses with it unset
      and the state it grants with it set, named together over the same path.
      `crates/nvs-stdlib/src/io.rs:1858`.

## Backlog
- `Core\Process`'s members, ranked next by `gaps.py` after `Core\IO` — needs no Docker.
- `Core\Storage`'s four rows over a `[storage.<name>]` disk — ADR 0082 § 2.
- Stage 10's `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which is goal 6's.
- `Core\Cache`'s shared tier needs a reachable Docker daemon — blocked, per the plan's Blocking.
