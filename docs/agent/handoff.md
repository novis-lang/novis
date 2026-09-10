# Handoff

## State

**Goal `unowned-sweep`, stage 2.** ADR 0147's mechanism is landed end to end and `Core\Uri::with` is
the member spending it; `rule:core-classes/uri-removable-components` stays `designed` for its second
level, the `queryParameter`/`withQueryParameter` pair, which is `crates/nvs-stdlib/src/uri.rs`'s
known gap 1.

`Core\Queue::push`'s `$args` refuses a `secret` now, and queue.rs's gap 2 is gone with it. The
refusal is **not** a registry spelling and could not have been one: `mixed` admits every qualifier
there is, so it is a call-site rule beside its four siblings —
`nvs_types::expr::quals::reject_secret_enqueued_argument` — reporting the serialiser sink's own code,
because `payload_of` encodes the payload with `Core\Json::encode`'s encoder. ADR 0033 § 4's
serialiser bullet carries the queue spelling and `rule:security/secret-sinks-refuse` says it.

`examples/uri-without-fragment.nvs` exists, which is what the driver's acceptance check was stopping
on: a fixture named in `files` and missing returns early in `loop.py`'s `begin`, so *no* floor check
ran. The `-p nvs-stdlib` check that named `a_secret_argument_to_push_is_refused_where_the_call_is_written`
could not host it — that crate cannot see a diagnostic — so the name moved to a `-p nvs-types` check
and `docs/agent/goals/31-unowned-sweep.toml` is byte-identical to the live goal again. Nothing is
blocked.

## Next group

**Stage 2: the options bag, continued** — one file set: `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/db/registry.rs`.

- [ ] **`Core\Queue`'s `limits` and `grants` are declared** — `crates/nvs-stdlib/src/queue.rs:64` is
      gap 1 and `crates/nvs-stdlib/src/queue.rs:984` the bag with nowhere to write them.
      `rule:core-api/shape-parameter`: a shape is only ever a whole parameter, so what is actually
      being decided is whether a `CoreOption`'s type may be one — `crates/nvs-stdlib/src/registry.rs:720`
      is `CoreTy::Shape`, and the option's `ty` field is the cell that would have to accept it.
- [ ] **The `Core\Db::open` half agrees on the spelling** — `crates/nvs-stdlib/src/db/registry.rs:41`
      is `SETTINGS`, whose literal already *is* a whole parameter, so it left this blocker first.
      `rule:core-api/one-checked-shape-type`: the check
      `the_core_db_open_half_of_the_same_blocker_and_this_one_agree_on_the_spelling` wants the two
      named together rather than each looking right alone.

## Backlog

- `rule:core-classes/uri-removable-components`'s second level, `queryParameter`/`withQueryParameter`
  — `crates/nvs-stdlib/src/uri.rs` gap 1; three named checks already wait on it in `loop-goal.toml`.
- Stage 3: `array<T>` element covariance, the user's decision recorded not re-argued
  (`docs/agent/loop-goal.md` § *Standing decisions*).
- Stage 4: the doc-comment pass that changes no answer (`docs/agent/loop-goal.toml`).
- `examples/uri-without-fragment.nvs` is exercised only by `files` existence; freezing its output
  would be a `command` check nobody has written.
