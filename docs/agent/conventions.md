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

**No trailers, ever.** A commit message documents the work and stops. No `Co-Authored-By`, no
`Signed-off-by`, no `Generated-with`, no tool or model attribution in any spelling. Who or what wrote a
commit is not part of the record this project keeps, and boilerplate at the foot of every message is
noise every future reader pays for. `git log` should read as a history of the language, not of its
authorship. This is enforced twice — `tools/session.py` strips a trailer out of any message it is handed,
and the `commit-msg` hook in `tools/git-hooks/` rejects one that arrives any other way.

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
- **Never `--ORACLE--` in `tests/conformance/`** — CI runs that suite on three hosted runners and none
  of them has PHP, so an oracle section makes the runner skip the whole case and subtract from the very
  count Stage 4 measures. Oracle cases go in `tests/differential/`, where the expectation is PHP's own
  output and nothing has to be frozen by hand. The playbook bullet owns the rest, including how to check
  against PHP while authoring; PHP is on `PATH` under Windows and inside the WSL distro alike, so the
  local legs run an oracle case rather than skipping it.
- A new file under `tests/conformance/` is picked up with no registration.

**The four shapes a depth case takes.** Every section of Part I has had a pass, so a new case reaches
for one of these rather than for a section, and what it adds is the boundary or the invariant the
member is written around — never another row of the same shape. `python tools/gaps.py` finds the
candidates; the playbook owns the spellings that will not compile.

- *Edges* — the answer where the member stops accepting: an empty receiver, a name it cannot read, a
  padding only wide enough to be used. `gaps.py --errors` lists the boundaries no case asks about.
- *Invariance over a sweep* — a property that must hold across a whole table, asserted by **counting**
  rather than read off a line, so a member answering plausibly row by row still fails.
- *A bound asserted on both sides* — the last accepted value and the first refused one, named
  together. A member that stops one entry early prints plausibly against either half alone.
- *Agreement* — one question asked of every member that shares a rule, asserting that they **agree**
  rather than what each answered, so a member that grew its own comparison fails here while still
  looking right on its own line. The shape with the most room left.

## An `.lspt` case

An LSP answer, frozen the way the section above freezes stdout. Sibling of `.mwlt` and deliberately a
separate suite: `mwl test`'s `N passed` is a number the loop gates on, and it must keep meaning one thing.
The format is `crates/mwl-lsp`'s module doc; the section lexer is the same one `.mwlt` uses. Lands at
M4B — [ADR 0099](../adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 5.

```
--TEST--
One sentence saying what is being pinned, ending with the ADR §§ it comes from
--FILE--
<?mwl
class User { public string $name; }
$u = new User();
$u-><|>
if (true) {
--REQUEST--
completion
--EXPECT--
name    property  string
```

- **`<|>` is the cursor**, exactly one per case, removed before analysis. A request that needs none
  (`diagnostics`, `semanticTokens`, `documentSymbol`) writes none.
- **`--REQUEST--`** is one line: the request name, then optional `key=value` arguments.
- **`--EXPECT--` is exact and frozen**, on the same terms as `.mwlt`'s. The rendering it compares against
  is canonical and lives in `mwl_lsp::render` — a case never invents a spelling, and a case that seems to
  need one has found a gap in that module.
- `--FILE <relative/path>--` works exactly as it does for `.mwlt`, which is how a go-to-definition case
  reaches across a `require`.
- **The document usually does not parse, and that is the point.** A case that only ever asks about valid
  code is not testing what the resilient tree exists for.
- A new file under `tests/lsp/` is picked up with no registration, and its coverage is **inferred** from
  the node the cursor resolved to — `mwl lsp-test --coverage` prints the matrix and
  `every_request_answers_every_construct` fails naming each empty cell.

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
[0104](../adr/0104-an-application-is-an-entry-file-path.md). **`python tools/adr.py` checks everything
below**, so write it to the shape and let the tool say whether you did.

```markdown
# ADR NNNN — <the decision as a statement, not a topic>

- **Status:** Accepted
- **Date:** YYYY-MM-DD
- **Scope:** what this decides, then explicitly what it does *not* — with the file that owns each
  excluded thing.
- **Depends on:** (only if it does) the ADR without which this one has nothing to decide.
- **Amends:** (only if it does) the ADR whose text this changes, and what changed there — one clause
  per target. Adding this obliges the same number in that ADR's `Amended by:`, in the same commit.
- **Amended by:** (maintained by whoever amends you) bare numbers, comma-separated, nothing else.
- **Validated by:** (only if a test holds a claim this ADR makes) the test, by path.

> **In short:** the whole decision, in one blockquote. A reader who needs only the rule stops here, so
> this paragraph is the ADR's front page and is worth more care than any section below it.

## Context
## Decision
## Consequences
## Alternatives rejected
## Verification
```

**Six rules the tool enforces, so none of them is a matter of care:**

- **The field set is closed** — the seven above and nothing else. There is no `Relates to:`; the
  citation graph is derived, and `python tools/adr.py --graph NNNN` prints it.
- **`Status:` is a bare value**, one of `Accepted`/`Proposed`/`Rejected`/`Superseded`/`Retired`. What
  has shipped of a decision belongs in its body or in the plan, never in the status field.
- **The heading set is closed and ordered**: `Context`, `Investigation`, `Options considered`,
  `Decision`, `Diagnostics`, `Consequences`, `Alternatives rejected`, `Revisiting`, `Verification`.
  Anything else is a `###` subsection under `Decision`.
- **Sections are numbered `### N.` and never renumbered.** `0007 § 3` is cited from `crates/`, from
  `docs/spec/` and from `docs/agent/loop-goal.toml`; a new section between two others is `§ 3a`.
- **An `Amends:` is bidirectional** — the amended ADR gains your number in its `Amended by:` and its
  body is edited to state the new rule, in the same commit.
- **A body carries no history.** No "this previously said", no withdrawn-section tombstone, no running
  total of anything. Git is the changelog, and a count kept in two files is wrong in one of them.

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
