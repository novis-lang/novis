# Handoff

## State

**M4's Stage 8, and `Core\Test`'s equality members now have their boundaries.** The tree is
at **765 conformance plus 189 differential**. Three cases landed, all over
`crates/nvs-stdlib/src/test.rs`, and the whole named group is done.

- **`assertEqualsDeep`'s depth cap counts every level the walk descends, the `null` leaf
  included.** A chain of **63** objects compares and **64** refuses — `MAX_DEPTH = 64` at
  `test.rs:706`, checked at `:691` before the tag match, so the 64th call is the descent into
  the innermost object's `next`. The path in the message carries 64 `->next` segments however
  much graph is left below it.
- **Two distinct allocations are needed to reach the cap at all.** `object_difference`'s
  `std::ptr::eq` shortcut (`test.rs:800`) answers `None` for one allocation compared against
  itself, so a 96-deep chain compared against *itself* passes — which is also why a sub-object
  shared by both graphs does not spend the budget twice.
- **A shape difference names the container, a leaf difference names the leaf.** A length, a
  key or a class mismatch is reported at the container's own path (`$actual["1"]`), because
  the first differing element is a position the two subjects do not share.
- **`assertSame` and `assertEqualsDeep` part at exactly one thing: a second allocation.** Over
  a 13-row sweep, 10 agree, 3 part (a twin, a `clone`, a twin one level down inside an array)
  and 0 part the other way — identity passing where the structural walk fails is impossible by
  construction, and that is what the counted `sameOnly=0` pins.

## Next group

**`Core\Test`'s failure *rendering*** — the file set is `crates/nvs-stdlib/src/test.rs` plus
`tests/conformance/core/`. Verified new this session: no case under `tests/conformance/`
asserts `Core\Test`'s own quoting bound, and the one secret row on disk is a secret *leaf*.

- [ ] **A quoted `string` in a failure message stops at 64 characters** (`test.rs:872` `shown`,
      `:902` `quoted`, `:912` `SHOWN_CHARS = 64`) — the *bound asserted on both sides* shape: a
      64-character subject is quoted whole and a 65-character one ends in `…`. The reason is in
      `shown`'s own doc comment (a message goes to a build log), so cite it rather than restate.
- [ ] **A container is named by its size and an object by its class, never quoted**
      (`test.rs:884`-`:892` — `bytes`, `array`, `object`, and `closure`/`resource` beside them)
      — the *agreement* shape over every tag `shown` answers: one failing comparison per tag,
      asserting that none of them puts contents in the message.
- [ ] **A `secret` property redacts a difference found *below* it, and reports the secret
      field's own path rather than the deeper one** (`test.rs:826`-`:838`, `REDACTED` at
      `:844`) — `object_difference` replaces both the path and both sides when the slot is
      secret, so a secret holding an object is a row the existing leaf case cannot see.

## Backlog

- `Core\Uri` reads as the thinnest class in `gaps.py` but its obvious boundaries are already on
  disk — `tryParse`'s agreement, the port bound and `with` naming the first component that
  moved each have a case. Rank it by what is *left*, not by depth (`docs/agent/loop-goal.md`).
- The next thinnest classes with room: `Core\Time\DateTime` (17 members), `Core\ObjectMap`
  (`objmap.rs`, 5 case files over 9 members), `Core\Random` (`random.rs`).
- `csv.rs:512` is the one remaining unasserted `thrown` and is owed no case (playbook).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
