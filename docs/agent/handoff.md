# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `python tools/owners.py --closes
decided-closures` still names **5**: `nvs-diagnostics/src/embedded.rs:30`,
`nvs-stdlib/src/command.rs:74`, `nvs-syntax/src/lib.rs:96`, `nvs-types/src/defaults.rs:58`,
`nvs-types/src/response.rs:29`. The other two stage-6 gates stay green.

**`command.rs` gap 1's blocker is gone — the signature the help page needs is now on the row it
already reaches.** `nvs_runtime::MethodRow::param_types` carries each declared parameter's
*spelling* beside `param_names` (`crates/nvs-runtime/src/object.rs:654`), filled by
`nvs_types::layout`'s declaration walk through `nvs_ir::ir::Class` and both of `nvs-codegen`'s
binders — no new seam, and `crates/nvs-codegen/tests/objects.rs:428` pins `uint` and `?int` arriving
at a descriptor, which is exactly what a `Tag` nibble cannot say. The roster's shape is stated once
as `nvs_types::layout::MethodEntry` and re-exported through `nvs_types` and `nvs_ir`; the playbook
bullet under *Writing Novis itself* owns why it is a name and not a written-out tuple.

**What the render still has to do, and everything it needs is local to `command.rs` now.**
`Command::handler` is the `Class::method` label (`crates/nvs-runtime/src/commands.rs:235`),
`ctx.class_desc(name) -> Option<*const ClassDesc>` (`crates/nvs-runtime/src/ctx/error.rs:277`) is the
lookup, and `reflect.rs:2089` is the `#[expect(unsafe_code, reason = …)]` deref idiom to copy. A
`CommandArg::param` finds its own spelling by its position in `MethodRow::param_names`. Gap 1's last
sentence — that the declared default is withheld because a page naming one of the two reads as
though the other were absent — is what makes rendering **both** the type and
`CommandArg::default` the close rather than half of it.

## Next group

**Stage 4: the help page names its parameters' types** — one file set:
`crates/nvs-stdlib/src/command.rs` alone, plus the `.nvst` cases beside it.

- [ ] **Render the type and the default, and delete gap 1 — `crates/nvs-stdlib/src/command.rs:759`.**
      `page_for` reaches the handler's row through `ctx` and `block`
      (`crates/nvs-stdlib/src/command.rs:739`) grows the column; keep `usage_line`
      (`crates/nvs-stdlib/src/command.rs:720`) as it is, since the usage line is the shape of a
      command line and not a declaration. `page_for` has three call sites with no `ctx` — the unit
      tests at `crates/nvs-stdlib/src/command.rs:1145` — so the resolved row arrives as a parameter
      those pass `None` for. The module doc's § *What a page looks like* is the layout's one home
      (`crates/nvs-stdlib/src/command.rs:11`) and § *What a completion script completes*
      (`crates/nvs-stdlib/src/command.rs:47`) cites "gap 1 below", so both are rewritten in this
      slice. `rule:tooling/commands-are-compiled` is what the page owes.
- [ ] **Three `.nvst` cases, each a different question — `crates/nvs-stdlib/src/command.rs:216`.**
      A page naming a `uint` flag's type and its default; a positional whose declared type is a
      class or a nullable, so the spelling is one no `Tag` can reach; and the agreement case — every
      argument the table lists is named with a type on the page, asserted by counting rather than
      read off a line. `tests/conformance/core/` is where they land, never an `--ORACLE--` section.
- [ ] **Widen `Core\Reflect\MethodInfo::parameters` to carry the type, or state it as a bound —
      `crates/nvs-stdlib/src/reflect.rs:1643`.** That walk builds one `PARAMETER_INFO` per
      `param_names` entry and the type is now beside it; whichever way it goes is a `Core` surface
      decision, so it is the five edits of `docs/agent/conventions.md` § *A `Core` member* or a
      sentence in the module doc — not a silent omission.

## Backlog

- `nvs-diagnostics/src/embedded.rs:30`, `nvs-syntax/src/lib.rs:96`, `nvs-types/src/defaults.rs:58`,
  `nvs-types/src/response.rs:29` — the four stage-4 gaps this group does not touch, each with its
  own `Decided:` sentence in its module doc.
- `crates/nvs-runtime/src/object.rs:654` — `param_types` is empty for every native `Core` row, on
  `param_names`' terms; `nvs_stdlib::registry::CoreMethod::params` is where a spelling for one
  would have to come from if a reader ever needs it.
