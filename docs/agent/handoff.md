# Handoff

## State

**Stage 4's doors are on disk and the gate now guards something.**
[ADR 0118](../adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md) §§ 2-3 are
implemented, not just decided: `nvs_runtime::capability::open_read`/`write` and
`nvs_runtime::script::resolve` each ask `require` before they name a spelling that performs the
effect, and `nvs_stdlib::registry::CAPABILITIES` declares the two `Core\File` members. The ADR's
body is the rule — do not re-derive it from this paragraph.

New on disk and green: `crates/nvs-stdlib/src/file.rs` (`Core\File::read`/`::write`, the five edits
of `conventions.md` § *A `Core` member*), three `.nvst` cases under `tests/conformance/core/`
named `file-*`, and `nvs_runtime::capability::io_failure` — the one author of the `IOError` a door
reports when the OS refuses something the capability allowed. `require` split into itself plus a
`pub(crate) refusal` returning § 5's message as data, because `script::resolve` owns an error type
of its own.

**Two consequences worth knowing before touching anything nearby.**

1. **Deny-by-default now reaches every fixture that spawns.** A `.nvst` case that writes no
   `nvs.toml` grants nothing, so four cases gained a `--FILE nvs.toml--` section — the playbook
   bullet under *Writing a test case* owns the mechanism and the failure mode. `nvs.toml`'s
   repository-wide `[[app]]` block grants `script.spawn` for the examples, and a narrower
   `entry = "examples/capability.nvs"` block takes it away again with `spawn = false`.
2. **`nvs_config::Snapshot` now derives `Default`** — the configuration of a host with no file
   anywhere, which ADR 0103 § 1 step 3 makes valid. It grants nothing, so it cannot be the shape a
   permission leaks through; its doc comment is the one home for that.

`examples/capability.nvs` prints `docs/agent/loop-goal.toml:1813`'s three frozen lines exactly. Its
source changed twice: `await` is an expression, so `await $child;` is not a statement, and the three
clauses now catch `RuntimeError` rather than `Throwable` now that § 5 has decided the class.

**Still open and not this group's**: `native examples/limits.nvs [4 capabilities]`, the driver's
failing check, is item 11's memory cap and is unwritten. It is not a regression — the fixture's own
header says it exits 0 in a tree not enforcing the cap.

`orient.py`'s pack was accurate. Its one warning is real: `[context] modules` names
`crates/nvs-host/src/budget.rs`, which does not exist — item 11's accounting is unwritten, so that
pattern should be `crates/nvs-host/src/watchdog.rs` until it does.

## Next group

**Close the driver's failing check — item 11's safepoint-driven limit enforcement.** File set:
`crates/nvs-host/src/watchdog.rs:166` (`Watchdog`, the deadline half already on disk),
`crates/nvs-runtime/src/counting_alloc.rs:88` (`allocated_bytes`, the only byte counter that
exists and `#[cfg(test)]` today), `crates/nvs-runtime/src/abi.rs:146` (`Fault::fatal`, what a
breach becomes), and `crates/nvs-config/src/tree.rs:156` (`Limits`, where `memory` and `cpu_time`
are already parsed).

- [ ] **A per-request byte counter the runtime keeps outside `#[cfg(test)]`** — ADR 0020 § 1. The
      allocation path already routes through `crates/nvs-runtime/src/alloc.rs`; what is missing is
      an accumulator on `Ctx` and a ceiling read off the snapshot's `[limits] memory`.
- [ ] **The safepoint that reads it and raises `Fault::fatal`** — ADR 0020, and
      `crates/nvs-codegen/src/lib.rs:1270` is where a safepoint is emitted. A breach is a `FATAL`
      and never reaches a `catch`; the goal's § *Standing decisions* says so and a fixture that
      wants to catch one has found the rule.
- [ ] **`examples/limits.nvs` exits non-zero with `FATAL` and `memory` on stderr**
      (`docs/agent/loop-goal.toml:1806`), keeping `before the cap` on stdout. This is the check the
      driver reports every iteration.

## Backlog

- Item 13's `Core\Fatal::onLimit` registration and `fatal_reserve_memory` — `docs/agent/loop-goal.md` item 13.
- Item 12's `max_script_depth` and per-tree accounting across concurrent isolates — same file, item 12.
- Item 18's `Core\Secret::reveal()` is not in the registry, though `Qual::Reveal` decides its row — `crates/nvs-stdlib/src/registry.rs`.
- `Live::admit`'s same-class check is asked of the answer, not the argument — `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- Item 22's `Core\Script::args`/`valueOrThrow` are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `Core\File` has no spec § of its own; § 8 is `Core\Path`'s lexical half — `docs/spec/01-core-library.md`.
