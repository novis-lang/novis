# Handoff

## State

**Goal `decided-closures`, stage 2 — the runtime and the lowering.** Stage 1's floor is the closed
goal `cache-shared-dial`'s checks, and they pass. Both of stage 2's own checks are green; what keeps
the goal open is its **owner gate**, and one stage 2 item is still a gap naming it.

`crates/nvs-ir/src/lib.rs` gap 18 is **built and its item deleted** — a throw escaping an abandoned
generator's `finally` is reported rather than dropped. `nvs-runtime` is below the crate that runs
tier 3, so the ladder arrives as a function pointer: `crates/nvs-runtime/src/floor.rs`'s `Ladder`,
`install_ladder` and `escalate` are the seam, `crates/nvs-cli/src/main.rs:1198` installs
`nvs_host::ladder::escalate` into it beside the panic hook, and
`crates/nvs-runtime/src/ctx/error.rs:341` reports the escaped throw through tier 3 and then the floor
before putting the saved failure back. **Tier 2 is skipped on purpose** and that is the divergence
from PHP that remains: the throw reached no request root, so the program's own uncaught handler is
not user code to re-enter from inside a refcount-zero release. The restore is still a replace, so
nothing the report leaves behind can become the pending failure.

The two guards are in `error.rs`'s test module, serialised against each other because the installed
tier 3 is process-wide: one asserts the escaped throw reaches a handler and the failure in flight is
the one that comes back, the other that the floor writes it when no tier 3 answers. Nothing is
blocked.

## Next group

**Stage 3: the checker refuses a `secret` into a container** — one file set:
`crates/nvs-types/src/expr/`, with `crates/nvs-runtime/src/record.rs` for the gap text itself.

- [ ] **`crates/nvs-runtime/src/record.rs:48` gap 1 — a `secret` stored into an array element or a
      shape-literal field is refused where it is written** — `rule:security/secret-qualifier` and
      `rule:errors/record-transformations`, whose redaction row this closes at the one end neither
      existing refusal reaches. The refusal is `nvs-types`'s and belongs beside the one already
      there: `crates/nvs-types/src/expr/quals.rs:529` is `reject_secret_debug_argument`, called from
      `crates/nvs-types/src/expr/calls.rs:372`. The acceptance check the driver reports red names it
      — `nvs-types`'s `a_secret_stored_into_an_array_element_is_refused_at_compile_time` — so the
      test's name is fixed and its crate is too. The gap's own text says why neither end reaches a
      container today: the qualifier composes onto no element type, and a shape field's type is
      inferred rather than declared, so the refusal is at the **write**, not on a carried bit. A new
      diagnostic code comes from the `E08xx` band (next free `E0824`; re-derive it before claiming).
- [ ] **Delete gap 1 from `crates/nvs-runtime/src/record.rs:46`'s register** once the refusal lands,
      leaving gap 2's struck prose alone — the item is the last thing the owner gate reads for this
      file, and `python tools/owners.py --closes decided-closures` is what proves it.

## Backlog

- The goal's own owner gate is what remains after the two items above — `owners.py --closes` and
  `playbook.py --closes` name the rest (`docs/agent/loop-goal.md` § *Standing decisions*).
- `[context.stage.3]` in `docs/agent/loop-goal.toml:216` names `types/type-alias` and two others but
  not `rule:security/secret-qualifier` or `rule:errors/record-transformations`, which the item above
  is written against; `[context] modules` names no `crates/nvs-types/src/expr/` path either.
- Stage 4's prepared-pattern channel still holds this goal's one ADR slot (`docs/agent/loop-goal.md`).
