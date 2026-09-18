# Handoff

## State

Goal `types-enum-1-2` is met: all 17 enums it names carry `about.md`, three examples with blessed
`.out` files and one test attributed by a `covers:` marker, and
`python tools/dossier.py --verify --only …` reports nothing owed and no failure. The last four landed
this session — `Core\Db\Isolation`, `Core\Db\Plan\Grade`, `Core\Db\Tls` and `Core\Digest`. The
previous handoff's group named only the first three; `Core\Digest` was owed as well and named in no
group, which is why the goal did not close on the slice count the group predicted. Nothing is blocked.

Two facts the `Core\Db` enums were written around. **An example never opens a database**
(`docs/examples/README.md` § *What an example is*), so a `Core\Db` enum's examples use its cases as
values. **A `.nvst` case still can**: a `--FILE nvs.toml--` section granting `[capabilities.db]
connect` with a `[db.main]` block of `driver = "sqlite"` and `path = ":memory:"` gives the case a real
engine with no container. `Core\Digest` needs neither — hashing computes, so its examples print real
digests.

## Next group

**Stage 2: the dossier, one enum per slice** — goal `types-enum-2-2`'s first three, one file set only
in the sense that all three are enum pages under `docs/examples/types/`; their declarations sit in
three different crates' modules, so each costs its own read. Each owes `about.md`, three examples and
one attributed test (`rule:testing/four-proofs`); `python tools/dossier.py --id '<name>'` prints the
directory and `--bless <file>` writes the `.out`. Grep `tests/conformance/` for the enum's name first —
a case that already exercises it needs the `covers:` marker and nothing else.

- [ ] **`Core\Env\Mode`** — owes examples, tests. `crates/nvs-stdlib/src/env.rs:277`
- [ ] **`Core\Http\Method`** — owes examples, tests. `crates/nvs-stdlib/src/router.rs:86`
- [ ] **`Core\IO\FileMode`** — owes examples, tests. `crates/nvs-stdlib/src/io.rs:1236`

## Backlog

- A dossier goal's seeded group can be shorter than what the goal's check demands; run the goal's own
  `--verify --only …` line before believing the group is the whole list — `docs/agent/loop-goal.toml`.
