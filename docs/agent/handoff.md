# Handoff

## State

**All three `Core\Csrf` slices landed**, and the class has left the floor `python tools/gaps.py`
ranks: six cases over two members, and it no longer appears among the thinnest. Three `.nvst`
cases, no Rust change, no member change, so no new refcount edge and no valgrind run behind them.

- **A token is not a function of its arguments** — twelve issues of one session under one key are
  twelve strings, all verifying, all pairwise different (66 of 66) and all the same width, so a
  member that went deterministic, or that folded in a counter or a timestamp, fails by count. The
  same twelve refuse another session and another key, which is what keeps an always-`true` `verify`
  from passing the sweep.
- **Every single-character mutation is refused and none of them throws** — at every one of the
  token's 79 positions, three ways: substituted inside base64's alphabet (the tag check's
  question), substituted outside it (the decode's), and deleted. One answer to all three, and the
  unmutated token is asked last so the sweep is not vacuous in the other direction.
- **The 32-octet bound is named on both sides and on both members** — 31 and 33 throw a
  `LogicError` naming the member and `$key`, 32 issues and verifies, and eight wrong widths refuse
  16 of 16 across `issue` and `verify` alike. A *rotated* 32-octet key is `false` and not a throw,
  which is the card's own distinction: a forgery is the ordinary answer, only a program bug throws.

**Nothing echoes a key.** `wrong_key_length`'s sentence says so itself and the cases only ever echo
that message, a count, or a fixed word.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**The `Core\Cli` value types are three of the six classes now sitting at the 3.0 floor, and they
are one file set: `crates/nvs-stdlib/src/cli.rs` and `tests/conformance/core/`. ADR 0086 is the
home of why the terminal is a sink and why styling is a value type rather than an escape string, so
each case asks what the *type* refuses rather than what it renders. The four member bodies are
inside 400 lines of each other.**

- [ ] **`Core\Cli\Color::index` and `::rgb` bound their channels on both sides** — the last accepted
      value and the first refused one for the palette index and for each of the three channels,
      named together and counted over the three channels so a member that checked one of them
      passes no line. `crates/nvs-stdlib/src/cli.rs:2650` and `crates/nvs-stdlib/src/cli.rs:2680`.
- [ ] **`Core\Cli\Style::of`'s seven slots are independent** — every option asked alone and in
      combination, counted, so a bag that dropped or aliased one fails by count while each single
      option still renders correctly. `crates/nvs-stdlib/src/cli.rs:2707`.
- [ ] **`Core\Cli\Progress::advance` is bounded at both ends of its own total** — the step that
      completes it and the one past it, and what a zero or negative advance does.
      `crates/nvs-stdlib/src/cli.rs:3055`.

## Backlog

- `Core\RateLimit` at 3.0 (`consume` 3, `shed` 3) — ADR 0075; `consume` needs the Docker daemon the
  plan's *Blocking* field names, `shed` does not.
- `Core\Http\Response` at 3.0 (`status` 3, `text` 3) — ADR 0074.
- `Core\Cache` 3.5 and `Core\Cache\Store` 4.0 — ADR 0059, same Docker dependency for the shared tier.
- `Core\Secret` at 3.5 (`revealBytes` 3, `reveal` 4) — ADR 0033 § 3.
- `Core\Task::afterResponse` is the one member with a PHP twin (`fastcgi_finish_request`) and no
  oracle case, `crates/nvs-stdlib/src/task.rs:561` — it belongs in `tests/differential/`.
- The two `[context] modules` patterns above that match no module.
