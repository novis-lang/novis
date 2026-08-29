# Handoff

## State

**Spec § 11's `Core\Digest` roster is fifteen cases, up from six, and `StrongDigest` is ten,
up from three.** That is goal 1 item 16, and it closes the Stage 8 acceptance check that had
been failing since the goal started. The roster's home is
`docs/spec/01-core-library.md` § 11's new case table — octets, `StrongDigest` membership and
one note per case — and `crates/nvs-stdlib/src/hash.rs` is the four places it is spelled in
Rust (`DIGEST`, `DigestKind`, `kind_of`, `digest_of`/`hmac_of`), which
`the_strong_subset_and_its_dispatch_agree` already ties together.

**Added:** `Sha224`, `Sha512_224`, `Sha512_256` (free — `sha2` was already pinned), all four
of FIPS 202's SHA-3 through the new `sha3` crate, `Crc32c` through `crc32c`, and `Blake3`,
which was declared in `Cargo.toml` for ADR 0042's artifact cache and had no consumer until
now. `blake3` is `default-features = false, features = ["std", "pure"]` so it stays pure Rust
rather than building its assembly through `cc`. Ordinals are appended at 6..14 and are ABI —
`Hash::stream` reads one back out of a slot — so the list is append-only and `STRONG` is a
list, never a range.

**Outside `STRONG` and staying there:** `Blake3` (HMAC-BLAKE3 is a construction nobody uses;
BLAKE3 is keyed natively, so it waits for a keyed member) and `Crc32c` (a checksum, for
`Crc32`'s reason). Both are argued in `hash.rs`'s `STRONG` doc comment and in § 11.

**Dependencies.** Three new workspace entries — `sha3 = "0.10"`, `crc32c = "0.6"` and the
rewritten `blake3` — each with the comment ADR 0051 § 4 owes. `python tools/gen-attribution.py`
has been run. `cargo deny check` could **not** be: the subcommand is not installed here (new
playbook bullet). The six crates it would have judged — `sha3`, `keccak`, `crc32c`, `blake3`,
`arrayvec`, `constant_time_eq` — all resolve to a license on `deny.toml`'s `allow` list,
checked by hand against the regenerated `THIRD-PARTY-LICENSES.txt`.

**The corpus is at 918.** Three cases landed and one was amended: widening the union changed
the `E0401` text `hash-hmac-refuses-a-weak-digest.nvst` had frozen.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have
playbook bullets under *Writing a test case*.

**Orientation gaps.** `[context] modules` owes `crates/nvs-stdlib/src/hash.rs` and
`src/test.rs`; neither this session's file set nor the next one's was in the map, and both
were found by hand. `[context] docs` owes `docs/spec/01-core-library.md`, which is the roster's
home and was read blind. Still owed from before: `crates/nvs-types/src/links.rs`,
`src/routes.rs`, `crates/nvs-stdlib/src/validate.rs`, `src/str.rs`,
`docs/spec/02-php-migration.md`, `tools/check-migration.py`, the stage-7 comment header's
per-goal floor table, a selector printing the *failing* check's own `cases` block, and a
`[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/test.rs` and `tests/conformance/core/`. This is
the group the previous handoff named and item 16 pre-empted; `gaps.py` ranks `Core\Test`
first — depth **4.0**, floor **3**. All three members below already have a case naming their
bound, so the shape that is missing is *agreement*: they share one ledger and one failure
renderer. `assertCount`'s two `Fault::fatal` sites are owed no case — the playbook says why.

- [ ] **`assertThrows` and `assertDoesNotThrow` are one predicate over a body**
      (`crates/nvs-stdlib/src/test.rs:467`, `test.rs:546`) — a table of bodies, each asked
      both ways, counting that exactly one of the pair passes for every row.
      `test-assert-throws-is-bounded-on-both-sides-of-the-class-it-names.nvst` is that bound
      named at one pair; this is the invariant over it, and it is the floor member.
- [ ] **`assertThrows`'s class argument over the whole exception tree**
      (`crates/nvs-stdlib/src/test.rs:467`) — the same table against each root of
      `nvs_hir::errors::TREE`, counting that a throw is caught by its own class and by every
      ancestor and by nothing else.
- [ ] **`assertNull` and `assertSame($x, null)` agree over every value a case can build**
      (`crates/nvs-stdlib/src/test.rs:370`, `test.rs:265`) — one question of two members that
      share a rule, counted rather than read off a line.

## Backlog

- Item 12's 10 `UNCLASSIFIED` registry members — `docs/agent/loop-goal.md` § 12.
- A `bytes` array key ICEs in `nvs-ir` — playbook, *Writing a test case*.
- `Core\Hash::stream` over the nine new cases has no `.nvst` of its own; the existing stream
  cases walk the original six — `docs/spec/01-core-library.md` § 11.
- A keyed `Blake3` member, which is what `STRONG` is waiting for — `hash.rs`'s `STRONG` doc.
- `cargo deny check` unavailable locally — see the new Tooling bullet.
