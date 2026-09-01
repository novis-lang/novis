# Handoff

## State

**`Core\Cache` is closed for depth work**, `python tools/gaps.py` having ranked it thinnest at
`local` 4 and `Store` 4/4. Three `.nvst` cases landed, no Rust change and so no new refcount edge:

- **A key is the bytes it is** — a sweep of keys differing by one byte, by case, by a leading or
  trailing space, by a combining mark NFC would fold, and by their last character after 288 shared
  ones; each written with a value derived from itself and counted, so a store that normalised or
  truncated hands one key another's value and the count drops rather than a line looking wrong. Two
  spellings of equal bytes are one entry; a key one byte from a stored one is absent.
- **A rewrite replaces the entry rather than adding one** — thirteen writes to one key, each read
  back immediately and counted, alternating length in both directions and repeating a value; the
  shape flips string→int with it; and under a `1K` cap the key written before all of them is still
  there, which is `Local::order`'s slot-per-live-key. The playbook bullet above is how that cap was
  sized, and the probe is what makes the third line an assertion.
- **`local()` twice is one store** — a handle taken before any write reads all four entries, the
  first handle reads the second's write, and an entry outlives every handle that wrote it.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a complete Part II, which needs spec §§ 15-19 from goal 6. Not a regression and not closable
here. Nothing was missing from this session's pack.

## Next group

**`Core\Cli`'s three value types are `gaps.py`'s thinnest cluster that needs no Docker and no
network — `Cli\Style::of` 4, `Cli\Live::set` 4, `Cli\Color::index`/`::rgb` 5 — and they are one file
set: `crates/nvs-stdlib/src/cli.rs` and `tests/conformance/core/`. ADR 0086 is the spec for all
three, and the sink rule (§ 5's control-byte substitution) is what a depth case reaches for rather
than another styling row.**

- [ ] **A style is its seven axes, counted** — `of`'s seven arguments are independent, so a sweep
      setting one axis at a time and counting the renderings that differ from the unstyled one fails
      an axis silently dropped or folded into its neighbour, which no single-line case can see.
      `crates/nvs-stdlib/src/cli.rs:2707`.
- [ ] **`Cli\Color::index` names its last accepted value and its first refused one**, and `::rgb`
      does the same on each of its three channels — the bound on both sides, not below it.
      `crates/nvs-stdlib/src/cli.rs:2650`, `crates/nvs-stdlib/src/cli.rs:2680`.
- [ ] **A live region substitutes control bytes like every other terminal write** — ADR 0086 § 5's
      sink rule asked of `set`, where a caller most expects a cursor movement to pass through.
      `crates/nvs-stdlib/src/cli.rs:2899`.

## Backlog
- `Core\Env::all` 3 and the unasserted throw at `crates/nvs-stdlib/src/env.rs:200` (non-UTF-8 value).
- `Core\Cldr::pluralCategory` is 4/4 over a closed roster — `crates/nvs-stdlib/src/cldr.rs`.
- `Core\IO\File`'s `lock` 3, `truncate` 3, `flush` 4 — one file set, needs no Docker.
- `Core\Task::afterResponse` is the last differential gap (`fastcgi_finish_request`), `task.rs:561`.
- Stage 6's shared tier and stage 10's Part II gate both stay blocked — Docker, and goal 6's spec.
