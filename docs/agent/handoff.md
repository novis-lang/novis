# Handoff

## State

**Stage 0's catch-up list is closed and the frontier is Stage 4's own counts** — conformance **439**
of 600, differential **90** of 150. Both of Stage 4's named guards pass, so what is short is
behavioural depth per member, not coverage: every registered member already has a case. Verify is
green: **1596** tests, 74 suites, clippy and fmt clean.

**A codegen hole is closed, and it was blocking the case work rather than merely near it.**
Cranelift's `enable_probestack` was on but its *strategy* defaulted to `outline`, which emits a
call to `__cranelift_probestack` — a symbol with its own register convention that `Jit::new`'s
`builder.symbol` loop never supplied. So a frame over 4 KiB did not get a probe; it panicked
`cranelift-jit` with `can't resolve libcall __cranelift_probestack`. That is roughly **fifty
statements at a script's file scope**, which any `.mwlt` case of ordinary size reaches. The
strategy is `inline` now (`crates/mwl-codegen/src/lib.rs:674`, with the reasoning beside the flag)
and `crates/mwl-codegen/tests/backend_policy.rs:36` runs a 200-statement script rather than
grepping the source for a flag, because this one is observable.

**`encoding` gained two cases and is no longer a thin section.** They pin what the four existing
ones did not: every padding length as a function of the operand's length, the empty operand in both
directions, a symbol neither base64 alphabet admits, base32's five block lengths and their padding
counts, `fromHex` refusing a space/`0x`/newline/non-ASCII, and — the row that needed `fromHex` to
write at all — a buffer of `00 ff 80 fe` round-tripping through all three pairs, which no
`"…" as bytes` can produce (ADR 0009 § 1).

## Next group

Three `Core\Hash` slices, in this order. Shared file set: `crates/mwl-stdlib/src/hash.rs`,
`tests/conformance/core/hash-*.mwlt`, and `docs/spec/01-core-library.md` § 11. All three live inside
the playbook's `bytes` trap — `"…" as bytes` and `Core\Encoding::fromHex(…)` are the only two
spellings and `toHex` is the only assertion — and inside the `Core`-instance trap for the third,
since `Hash\Stream` accumulates into a slot rather than holding a native context.

- [ ] **`Core\Hash::of` per algorithm** — `hash.rs:452`. One published test vector per `Core\Digest`
      case, the empty input for each, and a `bytes` no `string` could hold. Today
      `hash-computes-every-digest-and-authenticates-with-hmac.mwlt` walks the roster once with one
      input.
- [ ] **`Core\Hash::hmac` and `::equals` edge rows** — `hash.rs:468`, `hash.rs:501`. RFC 4231's key
      shapes (empty, shorter than the block, longer than it and therefore hashed first) and
      `equals` over operands of unequal length, which is the row a constant-time compare must still
      answer. `hash-hmac-refuses-a-weak-digest.mwlt` is the only case touching either.
- [ ] **`Core\Hash\Stream` chunk boundaries** — `hash.rs:549` `stream`, `:573` `update`, `:601`
      `finish`. Zero updates, a value split across two updates matching `of` on the whole, and what
      a second `finish` does. `hash-streams-a-digest-in-chunks.mwlt` pins one chunking today.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `crates/mwl-stdlib/src/json.rs` gap 2, ADR 0071.
- ADR 0088's registry-wide qualifier classification on `mwl-stdlib`'s member rows — M8.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- The next-thinnest sections after `hash`: `csv`, `out` and `validate` at one case each.
- The differential corpus is 90 of 150 — `tests/differential/`, and it needs PHP on the leg.
