# Handoff

## State

**M4 — language completeness.** Both remaining edges of the closure tag check are pinned from MWL.
An **object** argument reaches it through `Core\Out::capture($body, {through: $fn})`, and the line
it draws is objecthood and nothing finer: every non-object representation refuses by name while
`object`, `mixed`, `?T` and *any class at all* accept. An **enum** parameter is its backing integer
(`crates/mwl-ir/src/lower/mod.rs:2734`), so an enum, that integer and any other enum over the same
backing are one representation, while `int` and `uint` are still told apart.

**A wrong-class closure parameter is a type confusion, and it is the session's real finding.**
Two `final` classes and one wrong `Core\Arr::filter` callback write an `int` over a `string` field
and the next read dereferences it — `misaligned pointer dereference: address must be a multiple of
0x8 but is 0x5`, from a program with no `unsafe` in it. This is the only way a named-class binding
comes to hold another class's instance, and `docs/adr/README.md` § *Decisions taken at project
start* owns the decision: the **closure's own entry** pays, not every property access. That
paragraph is the specification for the next group; `param_tag_nibble`'s doc comment points at it.

`verify.py` green — conformance 624, differential 173.

## Next group

**Close the type confusion at the closure's entry.** The file set:
`crates/mwl-ir/src/lower/closure.rs:59` (`param_tags_word`), `crates/mwl-ir/src/lower/mod.rs:2727`
(`param_tag_nibble`), `crates/mwl-runtime/src/closure.rs:325` (`check_param_tags`),
`crates/mwl-runtime/src/object.rs:1020` (`MwlObj::is_instance_of`), `tests/conformance/core/`.

- [ ] **A closure parameter naming a class checks the argument's class at entry.**
      `mwl_ir::lower::closure` knows the declared class and `lower_expr` already emits the
      `mwl_value_instanceof` call-and-branch for `$x instanceof C`; emit the same at the body's
      first block, one per class-declared parameter, throwing `LogicError` in the sentence shape
      `crates/mwl-runtime/src/closure.rs:402` already writes. The tag word gains no class channel.
      `docs/adr/README.md` § *Decisions taken at project start*, second new paragraph.
- [ ] **`out-a-callback-object-parameter-is-checked-by-representation-not-by-class.mwlt` flips and
      is renamed.** Its `Marker`/`?Marker` rows go `y`→`n` (a `?T` still accepts — it erases to
      `Ty::Tagged` before any class survives), and its "**the class half is a hole**" comment block
      becomes the statement of the check. A subclass and an interface both accepting is the new
      case's other half, `is_instance_of` being a flattened ancestry scan.
- [ ] **`reduce` counts positions, not roles** — carried over untaken. The carry is argument 1, the
      element 2 and the key 3, and the refusal names the position rather than the role.
      `crates/mwl-stdlib/src/arr.rs:2611`.

## Backlog

- A promoted constructor property is not readable — `public function constructor(public string $name)`
  then `$m->name` is `E0405`. ADR 0030 owns the spelling; playbook § *Writing MWL itself*.
- An integer literal in an array-literal element position keeps `int` against a declared
  `array<uint>`. ADR 0054 § 2; playbook § *Writing MWL itself*.
- `Core\Reflect::typeOf` does not exist, so a value's tag is observable only through a `callable`
  parameter today. ADR 0007 § 4 names it; `docs/spec/01-core-library.md` owns the member.
- The `[context]` manifest printed everything this item needed; no field was missing.
