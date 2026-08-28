# Handoff

## State

**M4's Stage 8, depth.** The tree is at **853 conformance plus 189 differential**, all green.
Nothing is blocked.

Three cases landed, all `Core\Validate`, over one file set — the class was `gaps.py`'s
smallest at depth 4.0 / floor 3 and is now depth 4.5 / floor 4, so it is off the top of that
table. What each adds, beyond the eight cases already there:

- **`isIp` names its numbers.** The existing case asks what `{version}` is *for*; this one asks
  the grammar: `255` beside `256` in the last octet and the first, four octets beside three and
  five, eight groups beside seven and nine, four hex digits beside five, and — the bound worth
  the case on its own — `::` elides *at least one* group, so seven written groups plus `::` is
  an address and eight is not. Three counted sweeps carry the same three bounds without naming
  a number: eleven last octets accept six, six dotted widths accept one, twelve group widths
  accept one.
- **`isAscii` and `isDomain` were each given the *compositional* question**, which is what the
  corpus had no case for in either class: the whole is the **conjunction over the parts**,
  counted over a table rather than read off a line. `isAscii($s)` equals the conjunction over
  `Core\Str::codePoints`, and equally `Core\Bytes::length($s as bytes) == Core\Str::length($s)`
  — two independent statements of one property, 16 subjects each, plus two runs of a hundred
  that grow a subject a character at a time. `isDomain($d)` equals the conjunction over
  `Core\Str::split($d, ".")`, which is what makes every dot-shaped refusal in the corpus a
  *prediction* of an empty label rather than a rule of its own.
- **Depth in a domain is bounded by nothing but the 253 bytes.** A name of `n` single-byte
  labels is `2n − 1` bytes, so 127 labels is accepted and 128 refused, and a 63-byte label
  followed by 95 short ones is accepted at the identical total with a third of the depth.

That compositional move is the general one and it cost no new spellings: where the corpus
already pins a member's line on a single element, the unasked question is the sweep over whole
subjects that must agree with it.

`orient.py`'s pack was otherwise complete, with one gap: **`[context] modules` names no
`nvs-stdlib/src/*.rs` pattern**, so `validate.rs`'s module doc — the home of every line these
three cases pin, and the first thing any depth case in a `Core` class needs — cost a `peek.py`
of its own.

## Next group

**`Core\Uri`** — 19 cases over 30 members, `gaps.py`'s thinnest class now (depth 4.0, floor 3).
One shared file set: `crates/nvs-stdlib/src/uri.rs` plus `tests/conformance/core/`. Read the
existing `--TEST--` lines first — that is what found two of this session's three questions.

- [ ] **`Core\Uri::compareTo`** (3 cases) — row `uri.rs:506`, helper `uri.rs:1689`.
      ADR 0090 § 4 is the constraint and the playbook's `loop-goal.toml` bullet is about this
      exact member: `==` on two objects is identity, there is no `Equatable`, and
      `$a->compareTo($b) == 0` is *the* content-equality spelling. The compositional shape above
      transfers — an ordering must agree with itself over a pool of URIs (antisymmetry,
      transitivity, and agreement with the normalized rendering), counted over the pairs.
- [ ] **`Core\Uri::path`** (3 cases) — row `uri.rs:433`, helper `uri.rs:1507`.
- [ ] **`Core\Uri::decodeFormValue`** (3 cases) — row `uri.rs:382`, helper `uri.rs:1772`.

## Backlog

- A set returned by `union`/`intersect`/`diff` has no writable type, so every derived
  `Core\ObjectSet` is a chain — playbook, *Writing a test case*.
- `docs/agent/guard-name-debt.md`: 54 of the 156 guard tests `loop-goal.toml` names match
  nothing `cargo test` runs.
- 65 unasserted `Fault::fatal` paths (`python tools/gaps.py --errors`); `csv.rs:512`'s `thrown`
  is unreachable from source and is owed no case (playbook).
- Next thinnest after `Core\Uri`: `Core\Bytes`, `Core\Test`, `Core\Random`, `Core\Time`, all at
  depth 4.0.
