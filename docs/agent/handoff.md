# Handoff

## State

**M4's Stage 8 is where the work is, and `Core\Validate` is no longer the thinnest
class**: its three remaining character boundaries landed, so the tree is at **751
conformance plus 189 differential** and `python tools/gaps.py` now ranks `Core\Uri` (0.84)
thinnest, then `Core\Random` (0.86) and `Core\Time\DateTime` (0.88); `Core\Validate` reads
1.33 (8 cases over 6 members). The frontier is past the conformance floor of 750 now, so
what is left is depth alone; `python tools/loop.py --list` reports no named `.nvst` case
owed by any stage, and `python tools/gaps.py --differential` still ranks 0.

- The three landed cases are two of the four shapes. **A bound at both ends**:
  `validate-mac-takes-three-spellings-and-refuses-the-nearest-miss.nvst` names each
  separator form, group count, group width and hex digit `isMac` accepts beside the
  nearest it refuses, and counts that exactly one group count per form is taken.
  **Two edges**: `validate-email-draws-its-local-part-at-the-dot-and-the-atext.nvst` sorts
  all 128 ASCII code points into "accepted alone" (81) and "accepted between two atoms"
  (82), the one disagreement being the dot rule itself; and
  `validate-domain-labels-are-ldh-and-agree-with-the-email-domain.nvst` does the same over
  a label (64) and counts `isDomain($d)` agreeing with `isEmail("u@" . $d)` at all 128.
- Nothing in `crates/nvs-stdlib/src/validate.rs` was touched: every line each case names
  was already drawn, and PHP's `filter_var` agrees with all of `isMac`'s rows. The two
  places the module deliberately diverges — a single-label domain, and an all-numeric one
  — are named in the `isDomain` case rather than left to the module doc alone.

## Next group

**`Core\Uri`'s remaining edges**, the thinnest class `python tools/gaps.py` now ranks (16
cases over 19 members). The file set is `crates/nvs-stdlib/src/uri.rs` for the lines each
member draws and `tests/conformance/core/` for the cases; the standing twelve already own
the RFC 3986 § 5.4 resolve table, the two percent-coder inverse pairs over a byte sweep,
the bracket convention in both directions, the seven readers as one parse, and `with`'s
refusal of a component that moves — so what is left is the two members no case asks a
*boundary* of.

- [ ] **`parse` and `tryParse` are one line asked two ways** (`uri.rs:347`, `uri.rs:354`)
      — the *agreement* shape: every subject a sweep can build, asked of both, asserting
      that `tryParse` answers `null` exactly where `parse` throws rather than what either
      answered, counted.
- [ ] **`port`'s bound at both ends** (`uri.rs:426`, `with`'s own row at `uri.rs:474`) —
      `0` and `65535` beside `65536`, an empty `:` with no digits, a scheme's default port
      written out against the same port omitted, and the same four through `with`.
- [ ] **`scheme` and `host` are the two components case does not belong to**
      (`uri.rs:405`, `uri.rs:419`) — the *edge* shape: which of the two RFC 3986 § 6.2.2
      normalizes, asserted beside a path and a query that keep their case, and the first
      byte each stops accepting.

## Backlog

- `Core\Random` (0.86) and `Core\Time\DateTime` (0.88) are the next two thinnest —
  `python tools/gaps.py`.
- `crates/nvs-stdlib/src/csv.rs:512` is still the one `Fault::thrown` no program can
  reach — the playbook bullet under *Divergences and refusals already pinned* owns why.
- 65 `Fault::fatal` sites remain unasserted; most are ABI assertions no case can reach —
  `python tools/gaps.py --errors` judges one at a time.
