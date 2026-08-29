# Handoff

## State

**The conformance corpus is at 906 and the driver's floor is 950**, so the failing acceptance check
is 44 cases short and is not a regression — it is item 10's corpus floor, closing by depth cases at
about two a session. Item 12's roster is untouched this session: 10 members across five classes
remain `UNCLASSIFIED` at `crates/nvs-stdlib/src/registry.rs:1526` — `Core\Uuid` (2), `Core\Hash` (3),
`Core\Hash\Stream` (1), `Core\Router` (2), `Core\Csv` (2).

**`Core\Random`'s floor moved 3 → 4** — `float` 3 → 4 and `pick` 4 → 5. What the two new cases add
over the sweeps already there: the member's *bit width* rather than its interval (every draw is an
exact multiple of 2^-53, no draw is confined to the 2^-32 grid a narrower generator would leave it
on, and both parities of the last bit appear), and the two subjects where a draw has no freedom
left, where determinism is asserted a hundred draws at a time and `sample`'s two smallest refusals
name both of their numbers.

**The previous *Next group* was already on disk.** Its items 1 and 2 named shapes that landed in
c299254 and 3b89685; the playbook's new *Tooling* bullet has the one-call check that would have
caught it. Item 3 — `pick` over an empty and a one-entry array — is the second case landed here.

**Nothing consumes `Qual` yet** — it is declarative in `nvs-stdlib`, no `nvs-types` code reads
`classification()`, so a classification slice still cannot change what a program does today.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale, a `bytes` array
key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two have playbook bullets under
*Writing a test case*.

**Orientation gaps.** Still owed, none added yet: `docs/spec/02-php-migration.md` and
`tools/check-migration.py` in `[context] docs`, the stage-7 comment header's per-goal floor table, a
selector that prints the *failing* check's own `cases` block, `[context] modules` naming an
`nvs-stdlib` class module and `nvs-ir/src/lower/*`, a `[context] anchors` entry for `registry.rs`'s
`UNCLASSIFIED`, ADR 0088 § 1, 0063 R11, `0085 §§ 1-4`, `0071 §§ 2, 7`, and conventions.md's *four
shapes a depth case takes* whenever the group is conformance depth rather than a member.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/hash.rs` and `tests/conformance/core/hash-*.nvst`.
`gaps.py` ranks `Core\Hash\Stream` third-thinnest (depth 4.0, floor 4, 4 cases over 2 members) and
`Core\Hash` fourth (floor 4); eight sibling cases already cover *chunking agreement* from three
angles, so read their `--TEST--` lines first — the shape left is the stream's **lifecycle**, not its
arithmetic. The `.nvst` shapes are conventions.md's *four shapes a depth case takes*.

- [ ] **`Core\Hash\Stream::finish` is terminal** (`crates/nvs-stdlib/src/hash.rs:606`) — what a
      second `finish`, or an `update` after one, answers. The doc says a stream is "consumed by
      `finish`"; whether that is a throw, a diagnostic or a second digest is unasserted anywhere.
      *Edges*, and the message is the assertion.
- [ ] **A stream that was never updated, and an empty `update`**
      (`crates/nvs-stdlib/src/hash.rs:578`, `:554`) — both must equal `Core\Hash::of` over the empty
      input, which `hash-of-reproduces-each-digests-published-vectors` already pins for the direct
      member. *Agreement*, over every `Core\Digest` case.
- [ ] **`Core\Hash::hmac`'s key at exactly one block**
      (`crates/nvs-stdlib/src/hash.rs:468`) — `hash-hmac-takes-a-key-of-any-shape…` covers a key
      shorter than the block, a binary one and a longer one; the block length itself is the boundary
      between "used as given" and "hashed first", and it is the one RFC 2104 makes a special case of.
      *A bound asserted on both sides*: one byte under, exactly at, one over.

## Backlog

- `Core\Router::urlAbsolute` has 2 cases, the thinnest floor `gaps.py` prints — `router.rs:430`,
  and `::match` is out of scope per the goal's standing decisions.
- `Core\Regex::quote` has 2 cases (`regex.rs:1305`); `Core\Debug::dump` has 3 (`debug.rs:140`).
- Item 12's last 10 `UNCLASSIFIED` members — `registry.rs:1526`, ADR 0088 § 2.
- `router.rs:39`'s *Known gaps* 1 and 2 are stale — a docs slice, owned by that module doc.
- A `bytes` array key ICEs in `nvs-ir`; `catch (Core\Error $e)` panics — both have playbook bullets.
- `tools/check-migration.py` reads 34% classified against the plan's 100% — docs/plan/m4s.md.
