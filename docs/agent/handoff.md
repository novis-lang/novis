# Handoff

## State

**Goal `decided-closures`, stage 3 — the checker and the front end.** `python tools/owners.py
--closes decided-closures` now names **2**, both stage 3: `crates/nvs-diagnostics/src/embedded.rs:30`
(autoload roots into a bundle) and `crates/nvs-syntax/src/lib.rs:96` (a shape-typed local). Stage 4 —
the library — owns none. The other two stage-6 gates stay green.

**What a parameter default may be now**, and its one home is `crates/nvs-types/src/defaults.rs`'s
§ *What a default may be*: a literal, a `null` where the type admits one, an enum case and another
class's `const` — the same set a property default takes, folded by the same
`const_reference_default`. The call site emits it at the **parameter's own** IR type
(`crates/nvs-ir/src/lower/call.rs:615`'s `position`), so a folded case is a `Ty::Enum` value and not
a plain `int`; `crates/nvs-ir/tests/parameter_defaults.rs` is the only thing that can see that, for
the playbook's reason. A `#[Command]` row carries such a default as the **case name**
(`crates/nvs-types/src/commands.rs:752`), because `ArgConv::Enum` decided a command line spells a
case by name and the backing integer is a word it would refuse.

**The one-body-writer refusal now states its bound rather than carrying it as a gap.** A mount's
entry script is answered by § 3's default, in `crates/nvs-types/src/response.rs:29`'s own prose and
in `rule:security/response-body-is-one-typed-member`'s last paragraph, which this session amended to
say where the error is reported.

## Next group

**Stage 3: the shape-typed local's targeted error** — one file set:
`crates/nvs-syntax/src/parser/` plus the code table in `crates/nvs-diagnostics/src/lib.rs` and one
reject case.

- [ ] **Give `{x: int} $point;` the targeted error its `Decided:` names** — a new `E01xx` code
      (next free is `E0134`) reported where a statement-initial `{...}` is followed by a variable,
      pointing at `type Point = {x: int}; Point $p;`. The tell is half-built already:
      `crates/nvs-syntax/src/parser/expr.rs:1581` (`at_object_literal_in_block_position`) looks one
      token past the `{`, and what this needs is the token after the **matched** `}` — scanned from
      `crates/nvs-syntax/src/parser/stmt.rs:62`. `{ echo 1; } $x = 1;` is what the scan must not
      claim. Rule: `rule:types/shape-type`, and `rule:types/object-literal` for the collision the
      parser already resolves.
- [ ] **Delete the gap item and its two tags at `crates/nvs-syntax/src/lib.rs:96`**, rewriting the
      `# Known gaps` bullet as the module's own prose: the slot is narrower than the others, and the
      error names the alias form. One `tests/conformance/reject/` case with `--EXPECTF-ERROR--`
      pins the message, and a second asserts the block-then-assignment it must not claim.

## Backlog

- `crates/nvs-diagnostics/src/embedded.rs:30` gap 1 — resolve `autoload` roots into the bundle at
  build time (`Decided:` there); the file set is `crates/nvs-cli`'s bundler plus that module.
- `crates/nvs-types/src/defaults.rs`'s `decimal` paragraph — a `decimal` default is still refused;
  it carries no owner and is not this goal's.
- `python tools/owners.py --deferrals` is the proof any remaining gap needs if it goes to M9+.
