# Handoff

## State

**Stage 10's C-dependency ledger is on disk and green.**
`python tools/gen-attribution.py --check-c-deps` enumerates the default binary's C
dependencies out of the resolved graph — a `links` declaration, or a build dependency on
`cc`/`cmake`/`pkg-config`/`bindgen`/`nasm-rs`/`meson` — and holds each against
`C_DEPENDENCIES`, the ledger in that same script. Four candidates, one of them C: `ring` is
`verified` under ADR 0051 § 4's second question (`Cargo.toml`'s own `rustls` comment is the
home of that decision), and `blake3`, `defmt` and `wasm-bindgen-shared` are `no-native-code`.
The check fails in both directions, and a `no-native-code` verdict resting on a feature names
it — `blake3`'s `pure` — so the C cannot be switched back on silently. The gate lives beside
the license one because both read the same resolved graph; the script's own docstring says so.

**§§ 16 and 17 are no longer unguarded.** `every_part_two_spec_class_is_registered` in
`crates/nvs-stdlib/tests/spec_registry_coverage.rs` walks their `| Class | Surface | … |`
tables — the members are English but the class column is a code span — and asks
`registry::CLASSES` for an **exact** row per class, over the 9-key ratchet
`tests/spec-classes-part-two-outstanding.txt`. It is a weaker claim than the member walk by
construction and the test's own doc says which failure it does and does not catch. The item
named § 16; § 17 has the same table shape and the same walk reads it, so both are in.

**Stage 10's remaining open check is its last one, and it is the goal's own measure**:
`check-migration --min 74` reads 37%. The conformance floor of 1250 passes at 1346, and all
six `cargo-named` tests are green — nothing else in the stage is a session's work.

Nothing was missing from this session's pack.

## Next group

**Register what the two ratchets still owe. One file set: `crates/nvs-stdlib/src/` and
`crates/nvs-stdlib/tests/`.**

- [ ] **Strike ratchet keys by registering § 14's directory and metadata half.** 59 member keys
      remain and § 14's need no external dependency — `append`, `canonicalize`, the directory
      members. `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:18`,
      `crates/nvs-stdlib/src/io.rs:1`.
- [ ] **Register `Core\Os` and strike the first class key.** Spec § 16's row is `pid`,
      `hostname`, `cpuCount`, `memoryUsage`, `loadAverage` — ADR 0051 § 3's "`Core\Os`
      (`posix`, minus fork)", host facts with no capability door of their own. Five rows, five
      cards, five bodies, the `address()` arm and three `.nvst` cases each.
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:19`,
      `crates/nvs-stdlib/src/registry.rs:1079`.

## Backlog

- §§ 16-17 still have no *member*-level gate; `part_two_members`'s doc owns why and what it costs.
- The 59-key member ratchet, `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`.
- `check-migration --min 74` at 37% — stage 10's terminal check, `docs/agent/loop-goal.toml`.
- `Core\Net`, `Core\Signal` have no owning slice; the class ratchet's own comment says so.
- Goals 4 and 5 still need a reachable Docker daemon — the plan's `Blocking` field.
