# Handoff

## State

Goal `lang-testing` is live, and four of the seven features in `lang:testing` carry every feature
proof: `a-test-is-a-method-marked-test`, `assertions`,
`core-test-fixture-built-once-injected-by-type` and `core-test-testwith-data-rows`. The three that
still owe every proof are the next group below. The previous handoff called the fixture feature the
goal's last one, which was wrong — `python tools/dossier.py --verify --group lang:testing` is what
says how many the group holds.

Two findings stay recorded rather than fixed, each needing a decision no dossier goal may take. An
`abstract` `#[Test]` compiles and `nvs test` then raises the internal-error fault
(`crates/nvs-types/src/testing.rs`, `check_method_shape` § *Known gaps*), and the assertion ledger is
appended to under `nvs run` as well, where nothing reads it
(`crates/nvs-runtime/src/ctx/error.rs`, `record_assertion` § *Known gaps*).

## Next group

**Stage 2, the dossier: one slice is one feature with all its feature proofs** — one file set:
`docs/examples/lang/testing/<slug>/`, `tests/hostile/lang/testing/<slug>/`,
`benches/members/lang/testing/<slug>.nvs`, `tests/conformance/lang/` and
`tests/conformance/reject/`. A language feature's tests are attributed only by a `// covers:` marker,
so each slice ends by marking the cases that already pin its rule; the playbook's bullet on a reject
case's line numbers applies to every one of those edits. An example and a bench run under `nvs run`,
where a marked method never runs, so each one prints from top-level code and the marked methods are
what the reader learns from.

- [ ] **`lang:testing/test-options`** — owes about, examples, hostile, perf, tests.
      `rule:testing/test-attribute` and `rule:testing/determinism-declared-on-the-test` specify it,
      and the chapter is `docs/reference/lang/95-testing.md:107`. Two cases already pin it:
      `tests/conformance/lang/a-retried-test-that-passes-is-reported-as-flaky.nvst` and
      `tests/conformance/reject/a-retry-states-the-reason-it-is-retried.nvst`.
- [ ] **`lang:testing/every-test-is-its-own-isolate-and-the-constructor-is-setup`** — owes about,
      examples, hostile, perf, tests. `rule:testing/isolate-per-test` and
      `rule:testing/constructor-is-setup` specify it, and the chapter is
      `docs/reference/lang/95-testing.md:62`. No `nvs run` program can show a second isolate, so the
      examples show the constructor as the setup each test gets.
- [ ] **`lang:testing/running-tests-nvs-test`** — owes about, examples, hostile, perf, tests.
      `rule:testing/runner-is-strict` and `rule:testing/report-formats` specify it, and the chapter is
      `docs/reference/lang/95-testing.md:343`. The three `a-test-run-*` cases under
      `tests/conformance/lang/` already pin the report formats.

## Backlog
- `docs/perf/members.md` lists three features while the ledger holds every figure this goal has
  measured; `python tools/dossier.py --perf-report` regenerates it, and nothing gates on it.
- The `abstract` `#[Test]` refusal needs a diagnostic nobody has decided — `crates/nvs-types/src/testing.rs` § *Known gaps*.
- The assertion ledger under `nvs run` has no reader — `crates/nvs-runtime/src/ctx/error.rs` § *Known gaps*.
