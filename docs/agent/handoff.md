# Handoff

## State

**M4's Stage 8, depth.** The tree is at **810 conformance plus 189 differential**. Nothing
is blocked.

The `Core\Arr` half of the padding questions is **done**. Both new cases are agreement
sweeps over the one shared helper rather than more rows: `append_copies` is reached by
`Core\Arr::fill`, `::padStart` and `::padEnd` alike, so what is now pinned is that the
three answer the *same* run for the same count, that the shortfall is a function of the
subject's own count alone (a map's keys never enter it), that at or below that count both
padding members are the identity **on the values** though not on the keys, and that the
added entries are the one `$value` rather than a copy — asserted by mutating an object
through the original handle and by ADR 0090 § 4's object `==`, which is identity.
`Core\Arr::fill`'s degenerate counts (`0` empty rather than a throw, `1` a single entry)
and the claim in its own doc comment that `::fillKeys` is what replaces PHP's dropped
`$start_index` are pinned in the same case.

Two spellings worth knowing were confirmed on scratch runs and are not traps: an
`array<array<string>>` literal iterates with `foreach (... as array<string> $subject)`,
and a `Core` member answering `array<T>` may be passed straight to another one, so a
mirror invariant (`reverse(padStart(a)) == padEnd(reverse(a))`) is writable with members
alone and needs no string surgery.

The gap four handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two
`Uri`s is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Encoding`, the thinnest class `gaps.py` now ranks** (depth 3.0, floor 2) — one file
set: `crates/nvs-stdlib/src/encoding.rs` and `tests/conformance/core/`. Nothing here needs
`arr.rs` open.

- [ ] **The three encoders agree with their decoders over a sweep**
      (`encoding.rs:821` `toBase64`, `encoding.rs:859` `toBase64Url`, `encoding.rs:890`
      `toBase32`) — one case. Round-tripping every buffer in a table through each encoder
      and its own decoder, asserted by **counting** agreements rather than reading rows off
      lines; the two base64 spellings differ only in the alphabet, so the case should also
      pin that `toBase64Url`'s output is `toBase64`'s with the two substitutions applied
      and never padded differently.
- [ ] **A decoder's refusals are named on both sides of their bound**
      (`encoding.rs:907` `fromBase32`, and the `from`-half beside each encoder above) —
      one case. The last accepted input and the first refused one, together: a length that
      is not a whole group, a character outside the alphabet, and padding in a position the
      alphabet does not allow. Check each site's `Fault::` constructor first — a `thrown`
      is catchable and renders inline, a `fatal` reaches no handler.
- [ ] **`Core\Encoding`'s degenerate subject** — one case, the empty buffer through every
      member in both directions, plus the one-byte buffer, which is where base32's group
      padding is widest.

## Backlog

- Whether any other `built_fallibly` caller checks a size it does not then allocate
  (`crates/nvs-stdlib/src/str.rs:707`) — the audit slice this session did not reach; a
  different file set from the group above.
- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists
  while `$a < $b` over two `Uri`s is `E0411` — ADR 0013, and a session of its own.
- `Core\Csv::format`'s "column N is not a `string`" is unreachable from source until ADR
  0007 § 2's `array<T> as array<U>` row lowers — playbook, *Writing a test case*.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` would run —
  `docs/agent/guard-name-debt.md`.
- `Core\Test`, `Core\Random` and `Core\Debug` are the next thinnest after `Core\Encoding`
  — `python tools/gaps.py`.
