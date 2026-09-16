# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 11 (CLDR) is closed** and stages 0, 2–10 before it; stages 12–15
(`Core\Metrics`, `Core\Process::spawn`, the verification remainder, the rulebook) are what is left.
`crates/nvs-stdlib/src/cldr.rs`'s `# Known gaps` now holds gap 1 alone, owned by `unowned-closures`.
Nothing is blocked.

`ORDINALS` is every language CLDR publishes a marked ordinal rule for that the cardinal roster
carries — `ga`, `kw`, `lo`, `mo`, `ms`, `ro` and `vi` went in, `kw` as the one new arm
(`OrdinalSet::Cornish`). The module doc's ordinal section is the home of that closure: a language
answering `Other` is now one CLDR marks nothing for, and one CLDR marks that `RULES` does not carry
is refused by both members rather than answered, so the only way the table falls behind is the
cardinal roster growing without it.

`python tools/verify.py --doc` was red on three `///` links to `#[cfg(test)]` tests that predate this
goal (`nvs-syntax::walk`, `nvs-stdlib::cli`, `nvs-stdlib::registry`); they are backticks now and it
is green. The new playbook bullet is why a DONE claim is the first thing that sees them.

## Next group

**Stage 12: `Core\Metrics`** — one file set: the class lands in `nvs-stdlib` over the registry
`nvs-server` holds today, so `crates/nvs-server/src/metrics.rs` and
`crates/nvs-stdlib/src/registry.rs` are open together, and the call-site checks follow in
`crates/nvs-types/src/intrinsics.rs`.

- [ ] **The class and its three members** — `crates/nvs-server/src/metrics.rs:94` is gap 2, which
      says the members have no row yet and that they are three calls onto `Registry`;
      `crates/nvs-server/src/metrics.rs:176` is where a name is fixed to one kind, which is the
      throw the second test names; `crates/nvs-stdlib/src/registry.rs:1439` is `CLASSES`, where the
      row goes. `rule:observability/metrics-three-members` is the spelling and
      `rule:observability/a-registry-is-per-core-and-nothing-reads-it` is why nothing reads a value
      back. The goal's § *Standing decisions* settles the crate split: `nvs-stdlib` cannot depend on
      `nvs-server`, so the registry type moves to where a `Ctx` reaches it and the exporter stays
      feature-gated where it is, recorded in both module docs. Closes
      `core_metrics_is_a_registered_class`,
      `a_metrics_name_used_as_a_gauge_then_incremented_throws_naming_both_sites` and
      `core_metrics_accumulates_with_the_exporter_feature_off`.
- [ ] **A literal name and label value are checked where they are written** —
      `crates/nvs-types/src/intrinsics.rs:360` is `check_call`, the compile-time reader of a `Core`
      member's literal arguments, and `:476` and `:536` are the two checkers already written against
      it to copy the shape from. Closes `a_literal_metrics_name_outside_the_grammar_is_a_compile_error`
      and `a_tainted_metrics_label_value_is_a_compile_time_diagnostic`.
- [ ] **The three verbs accumulate, as a case** —
      `tests/conformance/core/metrics-increment-observe-and-gauge-accumulate.nvst`, the one
      `docs/agent/loop-goal.toml:11081` names. No `--ORACLE--`, and nothing reads a metric back, so
      what it prints is the program's own output around the calls.

## Backlog

- Stage 13 is `Core\Process::spawn` (`docs/agent/loop-goal.toml:11093`) — five `nvs-stdlib` tests, a
  release-profile `nvs-abi-probe` guard and two cases; a different file set again.
- Stage 14 is M8's verification remainder over `nvs-stdlib`, `nvs-codegen` and `nvs-types`
  (`docs/agent/loop-goal.toml:11128`), and stage 15 marks three rules shipped.
- `crates/nvs-stdlib/src/cldr.rs` gap 1, the compile-time prepared pattern, is `unowned-closures`'.
- A language CLDR marks that the cardinal `RULES` roster does not carry is refused by both members
  rather than answered; widening that roster is its own question, in the module doc's plural section.
- A pattern's offset is written to the minute wherever an offset appears, including the localized GMT
  format, so a zone with a second-level historical offset renders truncated — `X` and `x` have always
  done this and the newer letters agree with them.
