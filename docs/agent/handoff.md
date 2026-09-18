# Handoff

## State

Goal `types-enum-1-2` is met. Its 17 enums were already complete on disk; the check that named it
was **unsatisfiable rather than unmet** — `--verify`'s hostile leg printed a sentence for a suite
with no files, and an enum owes no hostile program, so an enum-only group could never produce the
second `0 failed` its generated `want` asks for. `tools/dossier.py` now reports the same counts, all
zero, for an empty suite, and the check passes.

Three of goal `types-enum-2-2`'s enums also landed: `Core\Env\Mode`, `Core\Http\Method` and
`Core\IO\FileMode` each carry `about.md`, three examples with blessed `.out` files and a `covers:`
marker on the conformance case that already exercised them. Nothing is blocked.

Two facts these three were written around. **An example gets no `nvs.toml`**, so `Core\IO::open` is
refused for want of `fs.write` and a `Core\IO\FileMode` example uses its cases as values, the way the
`Core\Db` enums do. **A route table is compiled in-process**, so `Core\Http\Method` is the exception:
`Core\Router::match` and `Core\Router::methodsFor` both run in a plain example with no server, and
`methodsFor` answers an `array<Core\Http\Method>`.

## Next group

**Stage 2: the dossier, one enum per slice** — goal `types-enum-2-2`'s next three. One file set only
in the sense that all three are enum pages under `docs/examples/types/`; the declarations sit in
three different modules, so each costs its own read. Each owes `about.md`, three examples and one
attributed test (`rule:testing/four-proofs`); `python tools/dossier.py --id '<name>'` prints the
directory and `--bless <file>` writes the `.out`. Grep `tests/conformance/` for the enum's name
first — a case that already exercises it needs the `covers:` marker and nothing else.

- [ ] **`Core\Log\Level`** — owes examples, tests. `crates/nvs-stdlib/src/log.rs:94`
- [ ] **`Core\Mime\Type`** — owes examples, tests. `crates/nvs-stdlib/src/mime.rs:81`
- [ ] **`Core\NormalForm`** — owes examples, tests. `crates/nvs-stdlib/src/str.rs:130`

## Backlog

- An expected type is not pushed into the arms of a `match` or a ternary: `return 3600;` satisfies a
  method declared `uint`, and `return $b ? 3600 : 0;` is refused as `int` (E0403). No rule decides
  it; `rule:types/arrays` states the opposite habit for array literals. Owner: nobody.
- `Core\Env\Mode`, `Core\Http\Method` and `Core\IO\FileMode` have no `about.md` check yet — goal
  `the-description-is-owed` is where that is switched on.
