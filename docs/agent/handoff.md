# Handoff

## State

**Goal 1 Stage 2's route table has eight of its acceptance check's nine named tests on disk and green**
in `crates/nvs-types/tests/routes.rs`. The ninth,
`a_url_key_that_is_neither_a_capture_nor_a_query_parameter_is_a_diagnostic`, cannot be written before
`#[Query]` exists — `crates/nvs-types/src/links.rs`' gap 1 owns why — so the check stays red at exactly
one item, which is item 4.

**`common::check_program_table` is new**: a whole *program* written to a temp dir and walked by
`nvs_hir::resolve_program`, then checked. It is the only shape that can tell "the program's table" apart
from "this file's" — `check_src_table` resolves one file, so a class in a second is never found — and
the duplicate-route and unknown-name questions are asked over two files through it now.

**ADR 0102 § 6's runtime half is on disk.** A `$params` key that names no capture becomes the link's
query string, written by `crate::uri::build` — `Core\Uri::buildQuery`'s own pass, run over the same array
with the captures omitted — so a link's query is `http_build_query`'s spelling, nesting and escaping
rather than a second convention. The refusal half is slice 3 below. `substitute`'s doc comment in
`crates/nvs-stdlib/src/router.rs` is that split's home.

**Item 15 in `docs/agent/loop-goal.md` still owns M4's seventeen `nvs-ir` lowering refusals**, unchanged
and standing by design; the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

## Next group

**One file set: `crates/nvs-types/src/derive.rs`, `src/routes.rs`, `src/links.rs`, `src/commands.rs` and
`tests/routes.rs`.** All three slices are item 4's `#[Query]`, and slice 3 is what turns the route-table
acceptance check green.

- [ ] **`#[Query]` joins the roster and is recognized on a parameter.** ADR 0102 §§ 3 and 6. The name goes
      on `ATTRIBUTES` at `crates/nvs-types/src/derive.rs:81`, matched nominally like every other one; the
      pass that reads it is beside `check_captures` at `crates/nvs-types/src/routes.rs:445`, which already
      holds the method's parameter list and is where a capture is bound to its parameter.
- [ ] **A `#[Query]` parameter's type is held to § 3's closed list.** The roster is
      `crates/nvs-types/src/commands.rs:230`'s `converts_from_string`, read and never copied — § 3's list
      is one question asked by the capture check, the `#[Option]` check and this one. Extend
      `a_capture_or_query_parameter_outside_the_type_list_is_a_diagnostic` in
      `crates/nvs-types/tests/routes.rs` rather than writing a second test: its own comment says the query
      half joins it here, and the acceptance check names that one test for both halves.
- [ ] **A `$params` key that is neither a capture nor a declared `#[Query]` parameter is a compile error.**
      ADR 0102 § 6, next free code `E0759`. It is `crates/nvs-types/src/links.rs:165`'s `resolve` beside
      `covered` at `:215` — the site already holds the literal keys and the row, so what it lacks is the
      route's declared query parameters. Closes `links.rs`' gap 1, writes the acceptance check's last name
      `a_url_key_that_is_neither_a_capture_nor_a_query_parameter_is_a_diagnostic`, and retires the
      paragraph in `substitute`'s doc comment (`crates/nvs-stdlib/src/router.rs`) that says it cannot be
      done yet.

## Backlog

- `#[Access]` and ADR 0096's "a `#[Route]` without a sibling `#[Access]` does not compile" — item 4's
  other half, `docs/agent/loop-goal.md`.
- The command table's check names four tests and two exist: `a_command_table_is_built_from_the_program_enumeration`
  and `a_duplicate_command_name_is_a_diagnostic` are owed, and `check_program_table` is now the shape for
  the first — item 6.
- `crates/nvs-types/src/links.rs` gap 2: `Core\Router::url(name: "…")` is a named argument, is not folded,
  and throws as a computed name would.
- `crates/nvs-types/src/routes.rs` gap 2: only the first `#[Route]` on a method becomes a row, so ADR 0110
  § 1's shared-`name` exception has nothing yet to except.
- Item 5's launder claim: the spec's Q column marks `url`/`urlAbsolute` **launder**, and no case asserts a
  `tainted` value reaching one — the playbook's qualifier bullet is why it cannot be written yet.
- Item 7's intrinsic folding (ADR 0057) is untouched; three module docs mention it and no pass reads them.
