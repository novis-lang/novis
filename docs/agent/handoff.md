# Handoff

## State

**Conformance is at 543 of 600, and it is the only frontier left.** Verify is green (1597 cargo tests,
74 suites, 543 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt` trees itself,
so after a green `verify.py` there is nothing else to run (playbook, *Running things*).

This session added no library code. It landed the last of the three `Core\Path` slices the previous
handoff named: `path-with-extension-and-extension-are-inverses.mwlt` — 120 cells asserting
`extension(withExtension($p, $e)) == $e`, 20 asserting `withExtension($p, extension($p))` is the path
back through the normal form, 720 asserting that setting twice is setting once, and the removal walk,
which peels **one** extension per call and reaches a fixed point that is exactly a name `extension`
answers `null` for. The three places the pair stops are named: an extension holding a dot (the setter
accepts `tar.gz`, the reader answers `gz`), a dotfile, and a trailing dot — the last two being
`stem_and_extension`'s single divergence from PHP, written here as what buys the inverse.

**`Core\Path` is off the frontier**: `gaps.py --coverage` now puts it at 1.22 cases per member, 15th of
27 classes. `Core\Time\Instant` (0.00, three members no case calls) and `Core\Time\DateTime` (0.06,
three more) are the thinnest, with `Core\Time\Date` (0.33) third — all in one file.

`orient.py`'s pack was complete for this work; nothing was fetched outside it beyond `path.rs`'s two
helper bodies and one sibling `Core\Path` case.

**A by-hand pass over `docs/adr/` is still in flight and is not loop work.** 103 modified ADRs plus
`ground-rules.md` have been uncommitted for two sessions now — a `Scope:` field added, `Supersedes:`
renamed to `Amends:`, heading levels moved. That is [doc-cleanup.md](doc-cleanup.md)'s pass, which
AGENTS.md says the user fires and the loop never does. **Do not stage it and do not `git commit -a`**:
stage your own paths, exactly as `session.py --wrap` already does.

## Next group

Three slices, **all on `crates/mwl-stdlib/src/time.rs`** and all writing a new file under
`tests/conformance/core/`, so a session taking two pays for the file set once.
`docs/spec/01-core-library.md` § 4 owns the `Time` rules. This is where the frontier actually is.

- [ ] **`Core\Time\Instant` orders and subtracts consistently, and `in` is the only zone question** —
      `compareTo` agrees with the sign of `minus` on every pair of a table, `minus` is antisymmetric,
      and `in` changes what the instant *renders* as without changing any ordering or difference, since
      an `Instant` is a point on the line and a zone is a reading of it.
      `crates/mwl-stdlib/src/time.rs:1819` (`compareTo`), `:1793` (`minus`), `:2450` (`in`).
- [ ] **`Core\Time\DateTime`'s parts agree with each other** — `date` is the civil date the
      part accessors already answer, `dayOfYear` agrees with walking the months of that year (and with
      the leap rule at both ends of February), and `difference` between two `DateTime`s agrees with the
      `Instant` subtraction underneath it. `crates/mwl-stdlib/src/time.rs:2366` (`date`),
      `:2407` (`dayOfYear`), `:2330` (`difference`).
- [ ] **`Core\Time\Date` is six members over one civil date** — 0.33 cases per member, third thinnest.
      Take it only after the two above, and only if the file is already loaded.

## Backlog

- `Core\Time` and `Core\Encoding` own all 9 members whose PHP twin has no oracle case
  (`python tools/gaps.py --differential`) — cheap to add while `time.rs` is open.
- `Core\Uri` at 0.53 cases per member, and its `parseQuery` bracket convention (loop-goal § *Standing
  decisions*).
- `Core\Json::decodeAs<T>` reads a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row
  (`mwl_stdlib::hash`'s module doc).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- `mwl-ir` gaps 1 and 18: `do`/`while`, a closure called through the variable holding it, ADR 0043's
  `by`-delegation.
