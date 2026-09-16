# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 20`
(was 31), `goal-owned: 65`, `past-milestone: 8`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0` and
`sections outside Known gaps: 0`; the stage's other check, `python tools/owners.py --deferrals`, is
green. The stage wants the first at 0, and § *UNOWNED* is the roster of the 20 left.

**`nvs-ir` now has no unowned gap.** All eleven were bounds rather than owed work, and the tree said
so: `python tools/holes.py` counts 0 refusal sites, so every "…panics" claim among them was stale, and
`target/debug/nvs.exe` run over a scratch file answered `<=>` on an `int`, a mixed numeric and a
`decimal` pair, `**`, `**=`, and a ternary joining an `int` and a `float` arm. The eleven are now
prose under `crates/nvs-ir/src/lib.rs` § *What each area lowers, and the limit it holds within*;
`# Known gaps` moved to the foot of that doc and holds 7 (M12), 14, 18 and the new 21, its preamble
rewritten because it claimed every item panics.

**One live bug found while reading, filed as gap 21 rather than fixed.** A conversion whose target is
a *union* carrying `null` never reaches the membership chain: `"z" as ?"a"|"b"` answers `"z"` where
`rule:expressions/nullable-conversion` answers `null`, because the annotation parses as a
`TypeKind::Union` whose first member carries the `?` and `nullable_target` reads the annotation's own
kind. `$s as ?"a"` is unaffected. Nothing is blocked.

## Next group

**Stage 6: gap 21, the one live bug the register now names** — one file set:
`crates/nvs-ir/src/lower/convert.rs` and one new `tests/conformance/lang/` case.

- [ ] **Read the annotation through its union in `nullable_target`** —
      `crates/nvs-ir/src/lower/convert.rs:2047` is the three-arm match that answers `None` for
      `Union[Nullable("a"), "b"]`, `crates/nvs-ir/src/lower/convert.rs:700` is the `as ?T` branch it
      gates (whose `class_ref_base`, array-restamp and property-key rows all read the *whole*
      annotation already), and `crates/nvs-ir/src/lower/convert.rs:1580`'s `nullable_target_atoms`
      already drops `null` out of a union of any width. `rule:expressions/nullable-conversion`.
- [ ] **Pin both spellings with one conformance case** — `?"a"|"b"` and `"a"|"b"|null`, a hit and a
      miss each, against `crates/nvs-ir/src/lower/convert.rs:1692`'s
      `lower_nullable_membership`; `rule:types/literal-types` for the accepted set's rendering.
- [ ] **Then take the next unowned cluster, `crates/nvs-cli/src/openapi.rs:32`** (gaps 1–5, the
      largest one left) — a different file set, so only if the two above leave room.

## Backlog

- `crates/nvs-runtime/src/graph.rs:61`, `:77` and `ctx/mod.rs:65` — three unowned (owners.py § UNOWNED).
- `crates/nvs-host/src/group.rs:90`, `placed.rs:33`, `worker.rs:98` — three unowned, one file set.
- The eight `past-milestone` items owners.py lists: each needs a goal or a bound, not an older `M`.
- `docs/agent/carried-gaps.md:53` — `Core\Process::spawn`'s owner goal `m8-stdlib-depth` is retired
  (`python tools/playbook.py --check` names it).
- `crates/nvs-ir/src/lower/convert.rs:406` — `convert_or_null`'s `# Panics` section still names
  `$m as ?array<T>`, which `lower_array_restamp` has answered since; the code below it says so.
- `crates/nvs-server/src/schedule.rs:1596` — `a_fleet_lease_is_renewed_while_its_run_is_in_flight`
  failed beside the other test binaries and passed alone in the next run, which is the isolation
  defect `tools/verify.py` § *Why `test` runs its binaries side by side* describes.
