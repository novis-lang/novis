# Handoff

## State

**Goal 6, M7. ADR 0102 § 5's narrowing is now a property of the table for every type the matcher can
reach.** A `Core\Uuid` capture converts and refuses what is not RFC 9562 § 4's canonical form, and
the placement question `nvs_runtime::routes`' gap 2 stated is **decided the first way: the parse
moved down.** `crates/nvs-runtime/src/uuid.rs` is its home, beside `crate::decimal` and for the same
reason — a capture is accepted or refused in the crate the walk is in — and `Core\Uuid::parse` reads
through it, so a segment a route admits and a string a program parses are one grammar. That module's
doc is the home of the decision. `nvs_runtime::routes` now has three known gaps, renumbered: the
linear scan, the row that does not cross, and percent-decoding.

**`Core\Router\Match` answers the five forms.** `CAPTURE` in `crates/nvs-stdlib/src/router.rs` was
still `tainted string|int|uint` after `decimal` landed last session; it is now one member per
conversion the matcher performs, and the two `.nvst` expectations, `nvs-types`' tainted-answer roster
and the generated `docs/novis.md` moved with it. `bool` and `enum` are what `CaptureConv::Unconverted`
still covers, and that variant's own doc says so.

**The driver's failed acceptance check was a file lock, not a regression** — "failed to remove file
`target\debug\nvs.exe`". `cargo build -p nvs-cli` succeeded at this commit, twice, with nothing in the
tree changed to make it.

**Unchanged and still triaged**: neither half of
`the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again` exists, for the reason
the Backlog names. Six paths were dirty in the working tree when this session started — `nvs-cli`,
`nvs-server` and two ADRs — and none of them is this session's; they were left untouched.

## Next group

**`crate::commands`' gap 1, minus its placement half — the two conversions whose readers this crate
now holds.** § 6's argument matcher is the route matcher one table along, and the file set is
`crates/nvs-types/src/commands.rs`, `crates/nvs-runtime/src/commands.rs`, `crates/nvs-cli/src/main.rs`
and `crates/nvs-stdlib/src/command.rs`.

- [ ] **A `decimal` command argument converts** (ADR 0086 § 6) — the choice is
      `crates/nvs-types/src/commands.rs:185`, the enum it crosses as is
      `crates/nvs-runtime/src/commands.rs:57`, the crossing itself is
      `crates/nvs-cli/src/main.rs:784`, and the arm that turns text into a value is
      `crates/nvs-stdlib/src/command.rs:501`. The refusal that narrows with it is
      `crates/nvs-stdlib/src/command.rs:281`, which today rejects the whole command at run time.
      `crates/nvs-runtime/src/routes.rs:348` is the shape to copy.
- [ ] **A `Core\Uuid` command argument converts** — the same four anchors, and no longer a placement:
      the parse is `crates/nvs-runtime/src/uuid.rs:46` and `nvs-stdlib` may call it directly.
      `crates/nvs-types/src/commands.rs:694` is where the type is already admitted.
- [ ] **A `.nvst` case per conversion, both sides named together** — the value a good argument becomes
      and the refusal a bad one gets, against `crates/nvs-stdlib/src/command.rs:501`'s arms and
      `crates/nvs-stdlib/src/command.rs:528`'s message.

## Backlog

- Split `the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again` — its check is
  `docs/agent/loop-goal.toml:3455`; ADR 0102 § 8's CSRF half needs `nvs_stdlib::crypto` and the `route`
  label half is ADR 0076 § 1's, and `nvs-server` depends on neither.
- An `enum` capture's segment spelling is `Core\Router::match`'s to decide and is out of this goal's
  scope — `docs/agent/loop-goal.md` § *Standing decisions*.
- `nvs_runtime::routes` gap 1: the walk is a linear scan rather than ADR 0077 § 2's trie.
- `nvs_runtime::routes` gap 3: a capture's value is still percent-encoded; the decoder is
  `nvs_stdlib::uri`, and this is the placement question just answered, asked again one type along.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by
  `docs/plan/m7.md`.
