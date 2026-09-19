# Handoff

## State

Goal types:exception, 11 of its 13 features complete. `ParseError`, `IOError` and `TimeoutError`
landed this session, each with `about.md`, three examples with blessed output, one attack and one
`.nvst` case; exceptions owe no bench. `python tools/dossier.py --verify --group types:exception`
names the two that are left, and `--run all --only <id>` runs one feature's suites without the gate
in front of them.

Every `Core` member that raises an `IOError` or a `TimeoutError` reaches the filesystem or the
network, so it needs a capability grant and no example or `.nvst` case in the tree calls one. Those
two classes' proofs raise them from a small helper of their own instead, which is also how a reader
meets them. `ParseError` is the exception: `Core\Arr::shapeAs<{…}>`, `Core\Json::decode` and
`Core\Json::decodeAs<C>` all throw a real one with no grant at all, and a multi-field failure is
where its `issues` list is visible.

The spellings these proofs needed, none of which the shapes document: a `foreach` binding over
`issues` is the parenthesised shape `({path: string, message: string}) $issue`; an options argument is
written inline at the call, so a cause is `new IOError("…", {previous: $cause})`; `$e is TimeoutError`
is the class test.

## Next group

**Stage 2: the dossier — one file set: the exception tree and the three proof trees keyed off it** —
`crates/nvs-hir/src/errors.rs`, `docs/examples/types/`, `tests/hostile/types/`,
`tests/conformance/error/`. One slice is one feature with all four proofs:

- [ ] **`RecursionError`** — owes about, examples, hostile, tests. The one class on the branch the
      runtime raises by itself, with no member and no grant involved: a program that recurses past
      the ceiling gets it, which makes a real one free to raise in an example and in an attack.
      `rule:testing/four-proofs`. `crates/nvs-hir/src/errors.rs:103`, and the ceiling itself is
      `crates/nvs-runtime/src/floor.rs:115`'s neighbourhood — check what a caught one's message says
      before writing the `.out`.
- [ ] **`Core\Script\Finished`** — owes about, examples, hostile, tests. Not an error at all: it is
      the marker `Core\Script::finish()` raises to end a script, it has no parent, and `E0814`
      refuses a `catch` arm that names it — so its `.nvst` case is an `--EXPECTF-ERROR--` one and its
      attack must never write that arm, because a compile diagnostic fails the hostile contract
      (`rule:testing/hostile-case-contract`). `crates/nvs-hir/src/errors.rs:109`,
      `crates/nvs-stdlib/src/script.rs:165`, `crates/nvs-runtime/src/throwable.rs:271`.

## Backlog

- A `#[Core\Json\Derive]` class with **no constructor** passes `E0821`/`E0460` while compiling and
  then throws `LogicError` at every decode door: `crates/nvs-types/src/derive.rs:1878` stays quiet on
  the assumption `ctor_init` reported it, which is false when every property has a default. The fix
  is a refusal at the decode site (`nvs_types::derive::check_json_sites`), which needs a call on
  whether an encode-only derived class stays legal — `rule:core-classes/derive-attribute` says opting
  in is answered at the declaration.
- `Core\Test\Failure`, `Core\Cli\NotInteractive`, `Core\Db\DbError` and `Core\Db\RolledBack` landed
  before this goal's proof pass and are not re-swept; `docs/examples/README.md` § *How a comment is
  written* applies to them only when a slice touches them for another reason.
