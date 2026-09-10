# Handoff

## State

**Goal `unowned-sweep`, stage 2.** ADR 0147's mechanism is landed end to end and `Core\Uri::with` is
the member spending it; `rule:core-classes/uri-removable-components` stays `designed` for its second
level, the `queryParameter`/`withQueryParameter` pair, which is `crates/nvs-stdlib/src/uri.rs`'s
known gap 1.

`Core\Queue`'s gap 1 is closed as a **decision rather than a declaration**, and queue.rs's module doc
is its home. The spelling was never the blocker: an option's type is never a shape
(`rule:core-api/shape-parameter`) and `rule:concurrency/queue-four-members` puts both knobs inside the
one trailing bag, so `grants` is a list of capability names and `limits` its sub-caps one option each.
What they wait on is enforcement — `nvs_types::expr::isolate`'s spawn check reports
`E_SPAWN_OPTION_UNSUPPORTED` for `limits:` and `grants:` — and declaring them ahead of that is
`rule:concurrency/an-upgrades-options-are-spawn-scripts`'s accepted-and-dropped narrowing. The stage's
check name and the goal's prose item 3 moved to match, in both copies of each.

**The stage-2 `Core\Uri::with` check names eight `-p nvs-stdlib` tests and none of the eight exists**,
which is why it is the earliest-stage red line; the conformance cases that guard the rule do exist.
That is the next group. Nothing is blocked.

## Next group

**Stage 2: `with`'s three states, asserted where the check looks for them** — one file set:
`crates/nvs-stdlib/src/uri.rs`, `crates/nvs-stdlib/src/registry.rs`.

- [ ] **Three tests over the landed `with`** — `crates/nvs-stdlib/src/uri.rs:2111` is the helper and
      `crates/nvs-stdlib/src/uri.rs:1388` its three-state read, so drive it the way the playbook's
      `nvs_runtime::call` bullet drives another module's member.
      `rule:core-api/omission-is-not-a-written-null` is the rule. The names the check waits on are
      `a_written_null_removes_a_component_and_an_omitted_key_leaves_it_alone`,
      `an_empty_query_stays_distinct_from_an_absent_one` and
      `there_is_no_empty_string_means_remove_rule_anywhere_in_with`.
- [ ] **Two more read off the rows rather than off a call** —
      `crates/nvs-stdlib/src/registry.rs:3352` is the pairing guard the two sit beside;
      `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`. The names are
      `a_nullable_option_omits_as_unset_and_a_non_nullable_one_omits_as_null` and
      `a_null_written_into_a_non_nullable_option_is_still_refused`.
- [ ] **`queryParameter` and `withQueryParameter`** — `crates/nvs-stdlib/src/uri.rs:2111` is the
      member they compose with; `rule:core-classes/uri-removable-components`'s second level, and the
      goal's own § *Stage 2* item 2 says why the spec rows, the registry entries, the conformance
      cases and `docs/reference/core/Uri.md` are **one edit**. The check's last three names are here.

## Backlog

- `Core\Queue` gap 2: the `existing` arm reads the table inside the statement that writes it, racy at
  `read committed` — `crates/nvs-stdlib/src/queue.rs`'s own `# Known gaps`.
- `limits`/`grants` become two ordinary options the day `spawn script` stops answering
  `E_SPAWN_OPTION_UNSUPPORTED` — same gap list, item 1.
- Stage 3, the `array<T>` covariant read the user already took —
  `crates/nvs-types/src/expr/assign.rs`.
- Stage 4, the panic hook and `[limits] max_output` — `crates/nvs-runtime/src/lib.rs`,
  `crates/nvs-stdlib/src/process.rs`.
- `Core\Db::open`'s half of the lifted blocker needs no edit of its own; the agreement is asserted
  from `crates/nvs-stdlib/src/queue.rs` over every registered row.
