# Handoff

## State

**M8 goal 5, stage 10.** The driver's failing acceptance check was the valgrind sweep — thirteen
fixtures red — and it was two unrelated causes stacked. `ring`'s AEAD assembly reports uninitialised
values on every TLS connection a fixture's queue worker opens, and `--error-exitcode=1` made
`examples/limits.nvs`'s by-design FATAL exit read as a leak. `tools/valgrind.supp` and the sweep's
move to exit code 97 close both; twelve of the thirteen are verified green under the real flags, and
`limits.nvs` exits 1, not 97. The playbook bullet is the whole diagnosis; `docs/agent/commands.md`
owns what the file hides.

**One real leak is now visible and is not fixed.** 292 bytes (112 direct, 180 indirect), *definitely*
lost, seen once in `examples/transaction.nvs` during a five-wide sweep and not reproducible on a
standalone run of the same fixture. The stack is JIT → `nvs_array_set` → `NvsArray::set` →
`make_unique`, so it is a copy-on-write clone nobody released. Do not chase it with four copies of
that fixture at once: they race to `create table accounts` and die on `pg_type_typname_nsp_index`,
which is the harness, not the bug.

**`Core\Taint::assertTrusted` earns a registry row**, and the decision plus its four arguments are in
`Qual`'s own doc comment — the mark is `Launder` with no sixth variant, the exception is held on the
roster rather than spelled in the mark, and it answers a plain `string` under ADR 0133 § 1. Nothing
of the member itself is written yet.

**Orientation gap, carried:** `[context]` still has no field that can name a `docs/spec/` file.

## Next group

**One file set: `crates/nvs-stdlib/src/registry.rs` with a new `taint.rs` beside it, widening to
`crates/nvs-types/src/core_lib.rs` on the second and to `crates/nvs-runtime/src/array.rs` on the
third.**

- [ ] **Write `Core\Taint::assertTrusted`'s five edits.** ADR 0024 § 3 spells the signature
      `assertTrusted(tainted string, string $reason): string`; the mark and the reasoning are decided
      at `crates/nvs-stdlib/src/registry.rs:186`. `crates/nvs-stdlib/src/secret.rs:41` is the model to
      copy whole, `$reason` included, and the row joins `crates/nvs-stdlib/src/registry.rs:1212`. The
      diagnostic that already advises the call is `crates/nvs-types/src/expr/args.rs:731`, so a
      conformance case can assert it now resolves.
- [ ] **Hold the roster the way the `secret` axis is held.**
      `crates/nvs-types/src/core_lib.rs:994` is
      `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`; its `tainted` twin asserts
      that `Core\Taint` is the only class whose `Launder` row names no one sink.
- [ ] **Chase the copy-on-write leak.** `crates/nvs-runtime/src/array.rs:1032` is `make_unique` and
      `crates/nvs-runtime/src/array.rs:1482` is `nvs_array_set`. Reproduce with the *whole* sweep
      (`printf '%s\n' db transaction pool queue | xargs -P 5 …`), never with one fixture repeated, and
      read `tools/valgrind.supp`'s header first so the two suppressed contexts are not re-diagnosed.

## Backlog

- The `[context]` manifest needs a `docs = [...]` selector — `docs/agent/loop-goal.toml`.
- A hand-run `valgrind examples/http.nvs` needs `tools/origin.py` listening on :8099, or it fails on
  connection-refused rather than on memory — not a finding, and the driver starts it itself.
- `examples/limits.nvs` stays *in* the sweep rather than joining `[valgrind] skip`: exit code 97 is
  what distinguishes it, and skipping would lose its coverage — `docs/agent/loop-goal.toml`.
- Whether the sweep should also fail on a fixture that *crashes* under valgrind, now that only 97 is a
  verdict — `tools/loop.py`'s `valgrind()`.
