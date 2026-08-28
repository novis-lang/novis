# Handoff

## State

**M4's Stage 8, depth.** The tree is at **799 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Time` left the frontier this
session — all three of its floor-2 members (`monotonic`, `sleep`, `fromEpoch`) gained a depth case,
so the class no longer ranks thin. Nothing is blocked.

The three cases landed are the invariance, the both-sided bound and the agreement shapes over one
class's own members:

- **`time-monotonic-is-only-ever-compared-with-itself.nvst`** — twelve readings taken in one array
  literal and asserted over all 66 *ordered pairs* rather than the 11 consecutive ones the older
  case walks: each later reading at or after the earlier one, each difference non-negative, each
  difference closing the gap, and each difference antisymmetric. A reading is then shown to hold
  nothing but a nanosecond count (rebuilding it from `Duration::nanoseconds` compares equal) and to
  be zone-free: the same reading renders identically in four zones where `Core\Time::now()->in(...)`
  renders three of the four differently, which is the positive form of "a `Duration` has no `in`".
- **`time-sleep-is-bounded-at-zero-and-waits-in-whatever-unit-it-is-given.nvst`** — the bound at
  zero on both sides. Seven non-positive durations out to `nanoseconds(-9223372036854775807)` all
  return, and the whole sweep costs under 40 ms; `1ns` is above the bound and is taken as a wait;
  four positive waits are each at least as long as requested (4/4); and one millisecond spelled four
  ways waits the same millisecond (4/4), which is the four PHP members `sleep` replaces.
- **`time-from-epoch-bounds-its-second-on-both-sides-and-its-nanos-at-one-second.nvst`** — three
  bounds, each with its last accepted and first refused value. The epoch second is
  `-377705023201 ..= 253402207200`, swept ten values around each end and counted 6/4 both times,
  with the two accepted ends printed as the instants they are and read back exactly; `nanos` is
  bounded at one *second* and not at the end of its `uint`, 999999999 accepted and 1000000000
  refused, swept 4/3. Only the `nanos` message is pinned — the range message is `jiff`'s wording and
  is deliberately not asserted.

## Next group

**`Core\Uri`'s own floor** — `gaps.py --coverage` ranks it at median 3.0 with a floor of 2, the
thinnest class on the board, and its three thinnest members share one file set:
`crates/nvs-stdlib/src/uri.rs` plus `tests/conformance/core/`. Seventeen `uri-*` cases already exist,
so check what each member is *already* asked before writing: `ls tests/conformance/core/ | grep uri`
and `sed -n '2p'` over the hits.

- [ ] **`Core\Uri::buildQuery` is asserted by invariance over a swept table** (`uri.rs:1851`) — one
      case. Two cases already name it (the bracket convention, and the round trip the wire cannot
      make); what neither does is count a property over a whole table — every built string parses
      back to the same map, ordering is the map's, and an empty map builds the empty string.
- [ ] **`Core\Uri::compareTo`'s bound and its refusals** (`uri.rs:1689`) — one case. The total-order
      case asserts the ordering; the edges are what is missing — a URI compared with itself, the
      component whose absence sorts before its empty spelling at *every* level, and what the member
      does with two URIs equal under normalization but different byte for byte.
- [ ] **`Core\Uri::decodeComponent`'s edges** (`uri.rs:1733`) — one case. `a%FFb` is refused (the
      crate's own unit test says so at `uri.rs:2138`); the case owes the boundary sweep — a truncated
      escape at end of input, `%` alone, a lower/upper-case hex pair agreeing, and the byte sweep's
      two ends.

## Backlog

- `Core\Bytes` (`at`, `endsWith`, `repeat` at 2 each) and `Core\Encoding` (`toBase32`, `toBase64`,
  `toBase64Url` at 2) are the next two floors after `Core\Uri` — `python tools/gaps.py --coverage`.
- `Core\Str::fold`/`graphemes`/`indexOf` sit at **1** case each, the lowest floor in the tree, but
  the Windows `php` has no `mbstring` (playbook) so their Unicode rows need the UCD table cited.
- `orient.py` warned twice that `[context] playbook` names two bullets under *Writing a test case*
  that no longer lead with those words (`'nvs-codegen has'`, `"emitbinop's ordering"`) —
  `docs/agent/loop-goal.toml` owes those two selectors a fix.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
