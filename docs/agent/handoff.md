# Handoff

## State

**Stage 9's acceptance check is closed** — all four `-p nvs-stdlib` tests exist and pass, and
`Core\Cldr` answers where its own gap notes 2–4 said it refused:

- **The plural roster names no absence.** Gap 3's twenty languages are carried, as fifteen new
  `RuleSet` arms, with `da`, `fil`, `tl` and `ceb` beside them — a roster that answers for `af` and
  refuses Danish is a hole rather than a boundary.
- **The eight letters format and parse** — `G`, `Q`, `w`, `W`, `L`, `c`, `F`, `u`. Two readings are
  written into the module doc and should not be re-derived. **`y` stays the signed proleptic year**:
  CLDR's `y` is the year *of era*, and rendering that instead made `yyyy-MM-dd` print `10000-01-01`
  for year −9999 and broke five landed cases that (rightly) treat that pattern as sortable and
  reversible over the whole `Core\Time\Date` range. So `u` renders the same year and differs only in
  reading a leading `-` back, and `G` is a rendering of the sign that parses read-and-discarded.
  **`w` and `W` are ISO 8601's week rule**, because CLDR states that rule per *territory* — the
  locale data this module refuses — so `W` answers `0` for a month opening on a short partial week,
  as ICU's does.
- **`Core\Cldr::ordinalCategory` is a second member over a second table**, answering the same six
  categories. **Its absence answers `Other` where the cardinal one throws**: CLDR files every
  language marking no ordinal form into one bucket, so that `Other` is published rather than
  unknown. The module doc's ordinal section is the home of that argument.

Six cases and one `-p nvs-types` test had used `G` or `Q` as their example of a refused letter and
are corrected as source to `Y`. **Stage 7's group was not taken** — the acceptance failure outranked
it, for the second session running. Nothing is blocked on a decision.

## Next group

**§ 3's lease, made real end to end — the store gains the operation and the binary joins the two.**
One file set: `crates/nvs-stdlib/src/cache.rs`, `crates/nvs-cli/src/serve.rs`,
`crates/nvs-server/src/schedule.rs`.

- [ ] **A set-if-absent with an expiry on the shared tier, and not a `Core` member.** The goal's
      standing decision 13 says so out loud, so this is Rust-visible only: a `SET key NX PX` over the
      same connection `store_put` already holds at `crates/nvs-stdlib/src/cache.rs:722`, beside
      `crates/nvs-stdlib/src/cache.rs:567`'s thread-local. No registry row, no card, no `.nvst` case
      — the roster is unchanged, which is the whole point of keeping it off `Core\Cache`'s surface.
      Its own test is that two callers over one store get two different answers.
- [ ] **`nvs serve` implements `nvs_server::Leases` over it.** `crates/nvs-cli/src/serve.rs:472` is
      the boot that holds the shared store and the ticker in one place; `arm` already takes
      `Option<&dyn Leases>` and a `None` leaves every `fleet` entry unarmed with a note
      (`crates/nvs-server/src/schedule.rs:322`).
- [ ] **A fleet fire, driven through the ticker rather than through the gate.** The stage's three
      `-p nvs-server` tests are about *which host runs the fire*, so they drive
      `crates/nvs-server/src/schedule.rs:345`'s arm with a scripted `Leases` and never a real store.

## Backlog

- `python tools/verify.py` cannot reach its clippy leg on this box right now: `-p nvs-cli --bin nvs`
  fails the known cranelift `NegOverflow` flake (three playbook bullets own it) and the gate stops
  there. Run `cargo clippy --all-targets -- -D warnings` and the two `.nvst` trees by hand when it
  does, as this session did — all three are green.
- `docs/novis.md` is generated reference text carrying `Core\Cldr`'s member list; it still shows one
  member. Regenerate it when whatever tool owns it next runs.
- ADR 0082 § 2's table row now says *two* members, and still points at the module doc as the home of
  both decisions.
- `Y`, `e`, `U`, `r`, `B`, `b`, `A`, `g` and the four zone spellings are still refused pattern
  letters — gap 2 names why, and `Y` is the one with a caller waiting.
