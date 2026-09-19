# Handoff

## State

Goal `lang-concurrency` is reached. All ten features of the concurrency chapter carry their
feature proofs, and the carried floor check that failed the last DONE sweep —
`native examples/queue-purge.nvs` — is green because the defect under it is fixed rather than
retried.

That failure was not a flake. Every wire driver filed `connect`'s handshake deadline on the
socket and never lifted it, so a caller that files no statement deadline of its own held a
connection whose clock was already spent and lost it on the first read that had to *wait* —
invisible on an idle machine, certain under load. `crates/nvs-db/src/pg.rs:666` carries the
reasoning and the other three drivers point at it; `rule:core-classes/db-statement-members` is
the contract it restores.

`python tools/verify.py` is 14 of 14, `--doc` is green, `python tools/db-matrix.py --all` is 8
of 8 legs, and `owners.py --closes lang-concurrency` and `playbook.py --closes lang-concurrency`
each own nothing. Nothing is blocked.

## Next group

The chain's next goal is `lang-attributes`, whose own handoff `goal-switch.py` installs, so this
group is only what to take if the switch has not happened yet.

**Stage 2: the dossier** — one file set: `docs/reference/lang/90-attributes.md`,
`docs/examples/lang/attributes/`, `tests/hostile/lang/attributes/`,
`benches/members/lang/attributes/` and `tests/conformance/`.

- [ ] **`lang:attributes/an-attribute-is-a-shape-literal-attached-to-a-declaration`** — owes
      examples, hostile, perf, tests (`rule:testing/feature-proofs`).
      `docs/reference/lang/90-attributes.md:9`
- [ ] **`lang:attributes/reading-attributes-back-core-attributes-get-and-all`** — owes the same
      four proofs, and shares the reading half with the item above.
      `docs/reference/lang/90-attributes.md:93`
- [ ] **`lang:attributes/the-names-the-compiler-acts-on`** — owes the same four proofs.
      `docs/reference/lang/90-attributes.md:116`

## Backlog

- `nvs schema apply` and `nvs queue migrate` open through `crate::schema::opened`, which files a
  10s handshake budget and no statement deadline; with the leak fixed their statements are
  unbounded, which is right for a migration a person is watching and is stated nowhere —
  `crates/nvs-cli/src/schema.rs:71`.
- `worker.rs:1525` cites `crate::queue`'s `open_and_apply`, which that module no longer has; the
  macro it describes now lives beside `schema::opened`.
- A task's `echo` is held until its group returns and is then printed one task at a time; the
  chapter never says so — `docs/reference/lang/80-concurrency.md` § *`Core\Task::all`*.
- Tasks do not start in the order their fields are written, which no chapter or rule states —
  `docs/reference/lang/80-concurrency.md` § *`Core\Task::all`*.
- `Core\Arr` has no `push`, so a trace across tasks is built by string concatenation — a gap only
  if `docs/reference/core/` means to offer one.
