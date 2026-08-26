# Handoff

## State

**Conformance is at 540 of 600, and it is the only frontier left.** Verify is green (1597 cargo tests,
74 suites, 540 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt` trees itself,
so after a green `verify.py` there is nothing else to run (playbook, *Running things*).

**The differential gate is met at 159**, and `gaps.py --differential` still names 9 candidates — the six
`Core\Time` members and three `Core\Encoding` ones — of which `now`, `monotonic` and `sleep` have no
oracle a case could freeze. Candidates, not work.

This session added no library code. It landed the first two of the three conformance slices the previous
handoff named, both the *agreement* shape from conventions.md:
`math-format-and-round-agree-wherever-both-name-the-same-precision.mwlt` (154 pairs over a table of
floats × precisions 0–6; the six `RoundMode`s scored against `format`'s own rendering on an exact-tie
table, where a tie alone cannot tell `HalfUp` from `Up`) and
`path-split-and-join-are-inverses-through-the-normal-form.mwlt` (20 rows, the round trip through
`normalize`, exact on an already-normal path, and no element ever empty — which is the divergence from
`explode`). The third, `relativeTo`, was not taken and heads the next group.

`orient.py`'s pack was complete for this work; nothing was fetched outside it beyond the two source
files the slices are about.

## Next group

Three slices that finish `Core\Path` — all of them over one file set: `crates/mwl-stdlib/src/path.rs`
for the rules and a new file each under `tests/conformance/core/`. `docs/spec/01-core-library.md` § 7
owns the rules. Every literal in these cases is written with `/` and no drive letter, and anything
printed goes through `Core\Str::replace($p, Core\Path::SEPARATOR, "/")`; the three new playbook bullets
under *Writing a test case* own the fold, the leg-safe counter and what `split` does not resolve — do
not re-derive any of them.

- [ ] **`relativeTo` and `join` undo each other, and the boundary is where they stop** —
      `join($base, relativeTo($p, $base))` normalizes back to `$p` over a table, and the pair parts
      exactly where no relative path exists (`isAbsolute` disagreeing between the two arguments).
      Both sides of that bound named together, and the `null` return asserted as the refusal it is.
      `crates/mwl-stdlib/src/path.rs:104` (`join`), `:118` (`normalize`), `:125` (`isAbsolute`),
      `:132` (`relativeTo`).
- [ ] **`dirname` and `basename` partition a path, and `join` puts it back** — `join(dirname($p),
      basename($p))` normalizes to `$p` on every row, `basename` never contains a separator, and the
      two `{levels:}`/`{withoutExtension:}` options are the only thing that moves the split.
      `crates/mwl-stdlib/src/path.rs:76` (`basename`), `:83` (`dirname`), `:147`
      (`BASENAME_OPTIONS`).
- [ ] **`withExtension` and `extension` are inverses wherever the name has one** —
      `extension(withExtension($p, $e)) == $e` over a table, `withExtension($p, null)` removes it,
      and a dotfile and a trailing dot are where the pair stops agreeing. `crates/mwl-stdlib/src/path.rs:90`
      (`extension`), `:97` (`withExtension`).

## Backlog

- `Core\Time\Instant` is the thinnest class at 0.00 cases/member and no case calls `compareTo`
  (`time.rs:1819`), `minus` (`:1793`) or `in` (`:2450`) — `python tools/gaps.py --coverage`.
- `Core\Time\DateTime` at 0.06: no case calls `date` (`time.rs:2366`), `dayOfYear` (`:2407`),
  `difference` (`:2330`).
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `gaps.py --errors` still names 58 unasserted `Fault::` sites, 57 of them `fatal` and so unreachable
  by any case.
