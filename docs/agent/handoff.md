# Handoff

## State

**Goal 4, M8.** The driver's one failing acceptance check is still the last gate the goal has open:
`differential`'s `min_passing = 250` (`docs/agent/loop-goal.toml:2630`). It is **not a regression** —
nothing fails, the count is the item.

**The suite is now 246 passing, 0 failing — 4 short.** This session took all four slices of the
previous group, two cases each. `Core\Uri::resolve` is measured against RFC 3986 § 5.2's transform
written out in PHP over the RFC's own Appendix B splitter, and § 5.3's verbatim recomposition is
asked beside `compareTo`'s normal form so that which member normalizes is one question with one
answer. `Core\Task::afterResponse`'s twin is **`register_shutdown_function`, not
`fastcgi_finish_request`** — the member with the name takes no closure and exists under no SAPI a
case can run.

Three of the eight are `--ORACLE-DIVERGES--` findings: `Core\Validate::isPrintable` asks Unicode's
`Cc` question where `ctype_print` asks the C locale's byte question, so `café`, a non-breaking space
and a bidi override are printable here and the empty string is too; `Core\Csv::format` quotes the
four bytes that change a parse where `fputcsv` also quotes a space and a tab; and the deferred queue
is sealed by its own drain where PHP's shutdown list may be appended to while it runs.

**`python tools/gaps.py`'s *differential gap* list is now empty** — every member with a PHP twin has
an oracle case. The remaining four are therefore **depth**: a second question of a pair that already
has one, and the group below is the class where that is cheapest.

## Next group

**All four are new files under `tests/differential/core/`**, sharing one module —
`crates/nvs-stdlib/src/hash.rs` — and spec § 11's first table. Every twin here is exact and
deterministic, so each is an `--ORACLE--` case with nothing to freeze by hand. Run one with
`target/debug/nvs.exe test <file>`; the runner prints both sides aligned, which is also how a
divergence is found rather than predicted.

- [ ] **`Core\Hash::equals` against `hash_equals`** (~1 case). The one member of the class with no
      oracle file: both are constant-time comparisons that answer `bool`, so the corpus is equal
      digests, digests differing in the first byte and in the last, and two of different lengths.
      `crates/nvs-stdlib/src/hash.rs:856`, `crates/nvs-stdlib/src/hash.rs:328`.
- [ ] **`Core\Hash::hmac` over the key-length boundary against `hash_hmac`** (~1 case). The landed
      `hash-hmac-matches-hash_hmac-over-every-strong-digest` sweeps the algorithms with one key;
      what no case asks is RFC 2104's own boundary — a key shorter than the block size, one exactly
      at it and one past it, where the key is hashed instead of padded (64 bytes for the SHA-2
      family, 128 for SHA-512 and its truncations). `crates/nvs-stdlib/src/hash.rs:818`.
- [ ] **`Core\Hash::of` over the empty subject and a multi-block one against `hash()`** (~1 case).
      `hash-of-matches-the-hash-family` asks every algorithm one subject; the block boundary is
      where a digest implementation's padding is, so the sweep is the empty string, one byte under
      a block, exactly a block and one over, for every `Core\Hash\Algorithm` case.
      `crates/nvs-stdlib/src/hash.rs:802`, `crates/nvs-stdlib/src/hash.rs:183`.
- [ ] **`Core\Hash\Stream` fed one byte at a time agrees with a single `hash_update`** (~1 case).
      Chunking invariance: the landed stream case feeds two chunks, and what matters is that *no*
      chunking changes the digest — one byte at a time, one whole subject, and an empty `update`
      between two real ones. `crates/nvs-stdlib/src/hash.rs:928`,
      `crates/nvs-stdlib/src/hash.rs:450`.

## Backlog

- `Core\Env`, `Core\Random` and `Core\Uuid` have PHP twins that `gaps.py` does not count (no
  deterministic expectation); a differential case for `Core\Env::get` against `getenv` would need
  the runner to set an environment — docs/agent/commands.md owns whether it can.
- `[context] modules` in `docs/agent/loop-goal.toml` carries no `nvs-runtime/src/deferred.rs`
  pattern; the `afterResponse` slice needed its sealing rule. Add it if a `Core\Task` slice returns.
- `Core\IO\Metadata` is the thinnest class at 1 case per member (`python tools/gaps.py`), which is a
  conformance gap rather than a differential one — spec § 14 owns the roster.
- The conformance suite stands at 1380 and its gate is met; nothing there is blocking.
