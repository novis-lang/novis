# Conventions — the shapes this repository writes

Every answer here is the same in every session. They were being re-derived with a read of an existing
example each time, which is why this file exists.

**It holds skeletons and pointers, never a second copy of a rule.** Each section names a real file that
CI already runs, because a worked example that is also a passing test cannot go stale the way a pasted
snippet can. Where a shape is short enough to write out, it is written out; where it is not, the pointer
is the answer. If a skeleton here disagrees with the file it names, **the file is right** — fix this.

The rules themselves live elsewhere: [AGENTS.md](../../AGENTS.md) for what binds every agent, the ADR for
a decision and its reasoning, the crate's module doc for how a subsystem works,
[playbook.md](playbook.md) for traps.

## A commit message

```
<type>(<scope>): <lowercase clause>, and <a second clause>

Prose paragraphs. What changed and why this shape rather than the obvious
alternative. Name the ADR section or the module doc that owns the rule, rather
than restating it. No bullet lists unless the content is genuinely a list.
```

`type` is `feat`, `fix`, `docs`, `refactor`, `perf` or `test`. `scope` is the subsystem, not the crate
path — `lang`, `types`, `ir`, `runtime`, `stdlib`, `codegen`, `syntax`, `test`, `adr`, `loop`, `agent`,
`abi-probe`. The subject is one line, lower case after the colon, no trailing period, and it reads as a
statement of what is now true rather than an instruction. Two clauses joined by `, and ` is the house
style when a commit does two things; one clause is fine when it does one.

Write the message to a file and use `git commit -F <file>` — never `-m` for anything multi-line, per
[commands.md](commands.md). `git log -1 --format=%B` is not needed to remember this; that is what this
section is for.

## A `.mwlt` test case

Canonical worked example, with every section that matters:
[tests/conformance/core/json-derive-encodes-declared-fields.mwlt](../../tests/conformance/core/json-derive-encodes-declared-fields.mwlt).
The format itself is `crates/mwl-test`'s module doc.

```
--TEST--
One sentence saying what is being pinned, ending with the ADR §§ it comes from
--FILE--
<?mwl
// Comments cite the ADR section each block exists for. A case is top-level
// statements: no Main::main.
echo "…", "\n";
--EXPECT--
the exact stdout, byte for byte
```

- `--EXPECT--` is exact. `--EXPECTF-ERROR--` instead when the case must *fail* to compile, and it has to
  reproduce the diagnostic's own indentation, which widens with the line number.
- **Never `--ORACLE--` in `tests/conformance/`** — the WSL leg has no PHP, so the runner skips the whole
  case there. Oracle cases go in `tests/differential/`. The playbook says why this costs more than it
  looks.
- A new file under `tests/conformance/` is picked up with no registration.

## A `Core` member — the four edits

All four in the module that owns the class; `python tools/brief.py`'s *anchors* block resolves each
spelling to a file and line. Worked example: `crates/mwl-stdlib/src/json.rs`, which is small enough to
read whole.

**1. The row**, in that module's `pub const CLASS: CoreClass`:

```rust
CoreMethod {
    name: "isValid",
    params: &[CoreTy::Str],
    defaults: &[],
    return_ty: CoreTy::Bool,
    symbol: "mwl_core_json_is_valid",
},
```

**2. The body**, via the macro:

```rust
mwl_runtime::mwl_helper! {
    /// `Core\Json::isValid(string $json): bool` — replacing `json_validate`.
    ///
    /// Why this shape rather than the obvious one, if that is not obvious.
    fn mwl_core_json_is_valid(_ctx, args: [1]) {
        let text = text_of(&args[0], "isValid")?;
        // …
    }
}
```

**3. The `address()` arm** — the one that bites, because a miss is a *runtime* panic naming the symbol
rather than a link error:

```rust
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_json_is_valid" => (mwl_core_json_is_valid as *const ()).cast(),
        _ => return None,
    })
}
```

**4. A `.mwlt` case that calls it.** `crates/mwl-stdlib/tests/conformance_coverage.rs` fails
`cargo test -p mwl-stdlib` without one. An instance member needs a case writing `->name(`.

`args: [N]` must equal the row's arity, where an options bag flattens to one argument per option and an
instance receiver is slot 0 and absent from `params`. The playbook's *Adding a `Core` member* section has
the arithmetic and what a mismatch looks like.

## An ADR

Next free number: `python tools/brief.py` prints it, and re-check it immediately before creating the file
— another agent derives the same answer from the same directory. Newest worked example:
[0078](../adr/0078-config-reload-and-control-socket.md).

```markdown
# ADR NNNN — <the decision as a statement, not a topic>

- **Status:** Accepted
- **Date:** YYYY-MM-DD
- **Scope:** what this decides, then explicitly what it does *not* — with the file that owns each
  excluded thing.
- **Amends:** (only if it does) the ADR whose text this changes, and what changed there.

## Context
## Decision
## Consequences
## Alternatives rejected
## Verification
```

`## Revisiting` is optional and only for a decision with a real trigger to reconsider it. **Fold, never
overlay:** amending an ADR means editing that ADR's body so it reads as currently true, plus a one-line
cross-link — never a new paragraph elsewhere describing the change. Then add the row to
[docs/adr/README.md](../adr/README.md)'s index table *and* its *Where to look* routing table, and a
one-sentence bullet to [docs/adr/ground-rules.md](../adr/ground-rules.md).

## A diagnostic

Next free code per band: `python tools/brief.py`. Bands are by compiler phase and the legend is
`crates/mwl-diagnostics/src/lib.rs`'s own table. Declare it there as a `Code::new` constant next to its
siblings — that file is the whole registry, so a code declared anywhere else does not exist.

Never reuse a retired number; the next free one is the band's highest plus one, deliberately not the
lowest hole.

## An edit the Edit tool cannot express

```
python tools/splice.py <target> --patch <patch-file>
python tools/splice.py <target> --patch <patch-file> --dry-run
```

The patch file holds both blocks, written with the **Write tool** — never a heredoc, which eats exactly
the backslashes this repository's Rust is full of:

```
<<<<<<< OLD
the exact text to find
=======
the text to put there instead
>>>>>>> NEW
```

Several blocks in one file are applied in order, and if any one fails to match the target is left
untouched. A failed match reports the line where the anchor stopped matching and what the file has
there instead.

## A status-block field

```
python tools/plan.py                                  # field names and sizes
python tools/plan.py --get "Open now"                 # its current text
python tools/plan.py --set "Open now" --from <file>   # replace it
```

The field set is fixed and `plan.py` refuses a name that is not already there. Write the new text with
the Write tool; `--set` re-wraps that one field and leaves every other byte of the file alone.
