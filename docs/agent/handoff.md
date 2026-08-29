# Handoff

## State

**`Core\Test`'s conformance floor is closed.** Three depth cases landed over
`crates/nvs-stdlib/src/test.rs`'s assertion members; `python tools/gaps.py` now puts the
class at depth **5.0**, floor **4** (it was 4.0 / 3, and was the ranked-first gap). The
corpus is at **921**, all passing.

**The Stage 8 acceptance check still fails at 921 of 950 wanted.** That is a growth floor,
not a regression — no case fails and no check names a test that disappeared. It closes as
the suite grows; the fastest route is the floor-2 classes in `## Backlog`.

**What the three cases pin**, all in `tests/conformance/core/`:

- `...-partition-a-table-of-bodies.nvst` — `assertThrows` at `Throwable::class` and
  `assertDoesNotThrow` are one predicate and its negation over ten bodies (five throwing,
  five returning, including a throw crossing a closure frame and a body that catches its
  own). Its last count is the other half: named a class *inside* the tree, the two are no
  longer complements, because only one of them has a bound.
- `...-agrees-with-the-exception-tree-at-every-pair.nvst` — all 81 pairs of
  `nvs_hir::errors::TREE`'s nine classes, with the expected verdict *derived* from a parent
  map keyed by `::class` rather than restated. 21 of 81 hold. A name the tree does not carry
  bounds nothing, so it refuses all nine.
- `...-are-one-predicate-over-every-value-kind.nvst` — `assertNull`, `assertSame` against
  `null` in both orders and `assertEquals` against `null` agree over an object, a closure,
  `bytes`, a container of containers, a shape, a `Core\Time\Duration`, an enum case and a
  `?T` holding a value, plus the three spellings of `null`. `assertSame($x, $x)` is the
  control that stops the counts being satisfied by a member that refuses everything.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at
`crates/nvs-stdlib/src/registry.rs:1526`), a `bytes` array key ICE in `nvs-ir`, and
`catch (Core\Error $e)` panicking — the last two have playbook bullets under
*Writing a test case*.

**Orientation gaps.** `[context] modules` owes `crates/nvs-stdlib/src/math.rs` (the next
group's file, absent from the map) and still owes `crates/nvs-stdlib/src/hash.rs` and
`src/test.rs`. Still owed from before: `crates/nvs-types/src/links.rs`, `src/routes.rs`,
`crates/nvs-stdlib/src/validate.rs`, `src/str.rs`, `docs/spec/01-core-library.md`,
`docs/spec/02-php-migration.md`, `tools/check-migration.py`, the stage-7 comment header's
per-goal floor table, a selector printing the *failing* check's own `cases` block, and a
`[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/math.rs` and `tests/conformance/core/`.
`gaps.py` ranks `Core\Math` first among the floor-2 classes — depth **5.0**, floor **2**,
44 cases over 38 members — and its three thinnest are one file and one domain. The
differential gap is empty for all three, so these are conformance-depth cases, never
`--ORACLE--` ones (conventions.md).

- [ ] **`Core\Math::atan2` is one angle recovered over all four quadrants**
      (`crates/nvs-stdlib/src/math.rs:999`) — a table of angles, each turned into
      `(sin, cos)` and handed back, counting the rows that come out where they went in, plus
      the four signed-zero axes where the sign of the argument is the whole answer.
- [ ] **`ceil`, `floor` and `round` agree on an ordering and part only at the half**
      (`math.rs:649`, `math.rs:655`, `math.rs:901`) — one table of floats asked all three
      ways, counting `floor <= round <= ceil` and that the three coincide exactly on the
      integers. `round`'s third argument is its precision and is the bound to name.
- [ ] **`toBase` and `fromBase` round-trip over every base the pair accepts**
      (`math.rs:1072`, `math.rs:1105`) — bases 2..36 by a table, counting the values that
      survive the round trip, with the first refused base on each side named together.

## Backlog

- The 950-case Stage 8 floor: the floor-2 classes are `Core\Regex` (`quote` 2),
  `Core\Debug` (`dump` 3, `render` 7), `Core\Csv`, `Core\Hash\Stream` and
  `Core\Time\TimeOfDay` (`compareTo` 2) — `python tools/gaps.py`.
- Item 12's 10 `UNCLASSIFIED` members — `crates/nvs-stdlib/src/registry.rs:1526`, ADR 0088 § 2.
- `catch (Core\Error $e)` is an ICE rather than a diagnostic — playbook, *Writing a test case*.
- A `bytes` array key ICE in `nvs-ir` — playbook, *Writing a test case*.
- `gaps.py`'s 65 unasserted `Fault::fatal` sites, most unreachable from source — judge before writing.
- `docs/spec/01-core-library.md` § 11's roster is fifteen `Core\Digest` cases — landed, nothing owed.
