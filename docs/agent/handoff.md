# Handoff

## State

**Goal 4, M8.** The driver's failing acceptance check is the last gate the goal has open:
`differential`'s `min_passing = 250` (`docs/agent/loop-goal.toml:2630`). It is **not a regression** —
nothing fails, the count is the item, and the work is writing cases.

**The suite is now 238 passing, 0 failing — 12 short.** This session took the previous group's first
two slices: `Core\Json::encode` against `json_encode` (four files) and `Core\Path`'s lexical
decomposition against `pathinfo` (three). Four of the seven are `--ORACLE-DIVERGES--` findings the
sweeps turned up rather than predicted, each now pinned in its own file: `json_encode` on PHP 8.5
drops a whole-numbered float's point (`1.0` is `1`) and keeps a mantissa fraction under an exponent
(`1.0e+100` against `1e+100`); `/`-escaping and `JSON_PRETTY_PRINT`'s four-space indent have no Novis
spelling at all, against two bare spaces; `pathinfo` reads a dotfile's whole name and a trailing dot's
empty string as extensions; and `dirname('')` is `.` here against PHP's `''`.

`python tools/gaps.py`'s *differential gap* list still holds one member with a PHP twin and no oracle
file at all — `Core\Task::afterResponse` against `fastcgi_finish_request` — and it is the last slice
of the group below.

## Next group

**All four slices are new files under `tests/differential/core/`**, sharing the same two reads apiece:
the spec section holding the member's *Replaces* column, and its owning `nvs-stdlib` module for the
option names. `--ORACLE--`, never `--EXPECT--`, and never in `tests/conformance/`
(`docs/agent/conventions.md`). Write the whole sweep as one oracle case and run it with
`target/debug/nvs.exe test <file>` — the runner prints both sides aligned and tells you which rows are
a divergence case instead.

- [ ] **`Core\Uri::resolve` and `$uri->compareTo` against the hand-written PHP** (~2 cases). Neither
      has a `parse_url` twin, so the oracle is RFC 3986 § 5.3's merge written out in PHP beside the
      normalization `uri-compare-to-matches-a-hand-written-rfc-3986-normalization` already uses.
      `crates/nvs-stdlib/src/uri.rs:2076`, `crates/nvs-stdlib/src/uri.rs:2155`.
- [ ] **`Core\Validate::isAscii` and `::isPrintable` against `ctype_print` and a byte-band probe**
      (~2 cases). § 12's prose roster; the band each accepts is the case, asserted on both sides of
      its bound. `crates/nvs-stdlib/src/validate.rs:586`,
      `crates/nvs-stdlib/src/validate.rs:601`.
- [ ] **`Core\Csv::format`'s quoting against `fputcsv`** (~2 cases). The landed
      `csv-format-writes-the-records-fputcsv-writes` asks the ordinary rows; what is untouched is
      which fields get quoted — leading and trailing whitespace, a bare quote, an embedded newline,
      the empty field. `crates/nvs-stdlib/src/csv.rs:558`.
- [ ] **`Core\Task::afterResponse` against `fastcgi_finish_request`** (~2 cases). The one member
      `gaps.py` still lists with a PHP twin and no oracle file; the ordering of the hook against the
      response is the claim, and ADR 0127 owns whether it runs before or after `onExit`.
      `crates/nvs-stdlib/src/task.rs:561`.

## Backlog

- `Core\Json::decodeAs`'s issue paths have no differential twin — ADR 0071 § 5 owns the shape.
- `Core\Path::relativeTo` and `::join` have no PHP twin at all; a hand-written oracle is the shape.
- `Core\Uri::query` round-trips are pinned but its `resolve` of a scheme-relative reference is not.
- Spec § 9's collections still have the least differential depth per member — `docs/spec/01-core-library.md`.
- `Core\Encoding`'s error paths are pinned; its `isValidText` band over the C1 range is not.
- ADR 0129's bcrypt roster has conformance cases and no differential one against `password_verify`.
