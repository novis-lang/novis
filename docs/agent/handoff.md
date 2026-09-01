# Handoff

## State

**All three `Core\Cli` value-type slices landed**, as four `.nvst` cases over `Color`, `Style` and
`Progress`. No Rust change, no member change, so no new refcount edge and no valgrind run behind
them. `python tools/gaps.py` no longer lists `Core\Cli\Color` or `Core\Cli\Progress` among the
thinnest classes at all, and `Core\Cli\Style` has left the 3.0 floor.

- **`Color`'s four bounded arguments are one bound reached four times** — the palette index and the
  three channels, swept rather than asked a line at a time: 0 and 255 accepted 8 of 8, 256 refused
  4 of 4, and each refusal naming its own subject 4 of 4. That last count is the one an `rgb` that
  read `$red`'s value three times fails while still saying `red` correctly on every line. The low
  end is the *type's* — a `uint` has no first refused value below zero — and the case says so.
- **A `Style`'s seven options land in seven slots**, compared as whole `Core\Debug::render`
  outputs: five attributes pairwise distinct 10 of 10, each one missed from the whole 5 of 5, and
  the two colour slots asked as a swap. Dropping and aliasing are the two failures, and each of
  them still reads correctly on a line of its own. Why the dump rather than the rendered row is a
  playbook bullet now.
- **`Progress::advance` is bounded at neither end of its total, and only the row clamps** — the
  completing step, the one past it and one far past it are all ordinary (`bar()` draws a finished
  bar for a loop that miscounted), a step of zero is a repaint that still takes the label, and a
  negative step is `E0401` a phase before any arithmetic runs, which is a second file because the
  program has to fail to compile.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**`Core\RateLimit` is the thinnest class left that needs no external service for the half a case can
reach, and it is one file set: `crates/nvs-stdlib/src/ratelimit.rs` and `tests/conformance/core/`.
ADR 0075 is the home of why the limiter bounds what only the application knows, and § 5 of why an
unreachable store throws rather than deciding *allowed*. Keep `consume` out of these three: it is
the `net.connect` half over Redis, and the Docker daemon the plan's `Blocking` field names is what
it waits on. `shed` and the `Decision` readers are this core's own memory, and their bodies are
inside 70 lines of each other.**

- [ ] **`Core\RateLimit::shed`'s window is bounded on both sides** — the last request admitted and
      the first one shed under one limit, named together and counted, so a limiter that spent its
      budget one request early prints plausibly against either half alone.
      `crates/nvs-stdlib/src/ratelimit.rs:758`.
- [ ] **A `Decision`'s three readers agree with the sweep that produced them** — `limit`,
      `remaining` and `retryAfter` asked after every request of a whole window and asserted to
      *agree* (remaining counts down to zero exactly once, `limit` never moves, `retryAfter` is
      answered only once shed), so a reader that grew its own arithmetic fails here while each of
      its own lines still reads right. `crates/nvs-stdlib/src/ratelimit.rs:806`,
      `crates/nvs-stdlib/src/ratelimit.rs:813`, `crates/nvs-stdlib/src/ratelimit.rs:821`.
- [ ] **`shed` keys its own memory** — two keys under one limit do not share a budget and a third
      key is untouched by either, counted over the sweep, which is ADR 0059 § 1's "a map in the
      calling core's own thread" asked as behaviour. `crates/nvs-stdlib/src/ratelimit.rs:758`.

## Backlog

- `Core\Http\Response::status` and `::text` sit at 3 cases each — `gaps.py`, ADR 0074.
- `Core\Cache\Store::get`/`::put` and `Core\RateLimit::consume` need the Docker Redis — plan
  § *Blocking*.
- `Core\Mail::send` is at 3 cases and needs an operator-named endpoint — ADR 0082 § 2.
- `Core\Decimal::divRound`/`::divExact` and `Core\Secret::reveal`/`::revealBytes` are the two
  service-free classes left under 4.0 — `gaps.py`.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case —
  `gaps.py`, `crates/nvs-stdlib/src/task.rs:561`.
- `orient.py`'s module-pattern matcher misses two live files — see `## State`.
