# Handoff

## State

**`Core\Validate`'s conformance floor is 5**, up from 4. Three cases landed, one per member `python
tools/gaps.py` named as the class's thinnest — `isMac`, `isPrintable`, `isIp` — and the class no
longer appears in that tool's thin list. Item 10 counts the per-class floor, not the corpus total,
so this closes `Core\Validate` as a worklist entry.

**What the three assert.** `isMac` is one predicate over a *value*, not three over three notations:
nine spellings of each value (three separators × three casings), counted, over 137 values, so a
member that grew its own reading of one notation fails on the count. `isPrintable`'s line is Unicode
general category `Cc` and nothing else — both refused runs are found by sweeping `U+0000`..`U+20FF`
rather than named, and the invisible characters just outside the category are accepted, which is the
"not a spoofing check" promise `validate.rs`'s module doc makes. `isIp`'s `{version}` is a
*narrowing*: over a 24-subject table the option-free answer is exactly the disjunction of the two
families, `{}` is the absent option, and no subject is both.

**The corpus is at 915** against the driver's floor of 950. Item 10's check still fails and is still
an item, not a regression.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have
playbook bullets under *Writing a test case*.

**Orientation gaps.** `[context] modules` still owes `crates/nvs-stdlib/src/validate.rs`,
`src/str.rs` and `src/test.rs` — the map printed neither this session's file set nor the next one's,
and both were read by hand. Also still owed, none added yet: `crates/nvs-types/src/links.rs` and
`src/routes.rs` in `modules`, `docs/spec/02-php-migration.md` and `tools/check-migration.py` in
`docs`, the stage-7 comment header's per-goal floor table, a selector that prints the *failing*
check's own `cases` block, and a `[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`. The
`shapes` selector printing conventions.md's *four shapes a depth case takes* was exactly right for
this group and should stay while the goal is conformance depth.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/test.rs` and `tests/conformance/core/`. `gaps.py` now
ranks `Core\Test` first — depth **4.0**, floor **3** — naming `assertThrows 3, assertCount 4,
assertNull 4`. **All three already have a case naming their bound**, so read
`ls tests/conformance/core/ | grep assert` first and take the shape that is missing: in this class
that is *agreement*, because these members share one ledger and one failure renderer.
`assertCount`'s two `Fault::fatal` sites are owed no case — the playbook says why.

- [ ] **`assertThrows` and `assertDoesNotThrow` are one predicate over a body** (`test.rs:206`,
      `test.rs:217`) — the *agreement* shape: a table of bodies, each asked both ways, counting that
      exactly one of the pair passes for every row.
      `test-assert-throws-is-bounded-on-both-sides-of-the-class-it-names.nvst` is that bound named
      at one pair; this is the invariant over it, and it is the floor member.
- [ ] **`assertThrows`'s class argument over the whole exception tree** (`test.rs:206`) — the same
      member again, from the *sweep* side: for each root of `nvs_hir::errors::TREE`, an ancestor
      passes and a sibling fails, counted rather than read off two lines.
- [ ] **`assertNull` and `assertSame(x, null)` agree over every value a case can build**
      (`test.rs:188`, `test.rs:160`) — `test-assert-null-refuses-every-value-that-is-not-null.nvst`
      pins the refusal; what is unasserted is that the two members answer alike, including for the
      `?T` absence a `Core` member returns.

## Backlog

- The corpus is 915 against 950 — item 10, `docs/agent/loop-goal.toml`.
- `Core\Math` floor **2** (`atan2`) and `Core\Regex` floor **2** (`quote`) are the two lowest floors
  left after this session — `python tools/gaps.py`.
- Item 12's 10 `UNCLASSIFIED` members, `crates/nvs-stdlib/src/registry.rs:1526` — ADR 0088 § 2.
- A `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking rather than diagnosing
  — both have playbook bullets under *Writing a test case*.
- `Core\Test` depth 4.0 is the thinnest class by median, not just by floor — `gaps.py` ranks it
  first for a reason the next group only half spends.
