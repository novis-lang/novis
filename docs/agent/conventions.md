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

## A `.nvst` test case

Canonical worked example, with every section that matters:
[tests/conformance/core/json-derive-encodes-declared-fields.nvst](../../tests/conformance/core/json-derive-encodes-declared-fields.nvst).
The format itself is `crates/nvs-test`'s module doc.

```
--TEST--
One sentence saying what is being pinned, ending with the ADR §§ it comes from
--FILE--
<?nvs
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

## A feature's four proofs — an example, an attack, a bench, a `covers:` marker

`rule:testing/four-proofs` makes these owed. **Each tree's own
README owns the rules** — [docs/examples/](../examples/README.md), [tests/hostile/](../../tests/hostile/README.md),
[benches/members/](../../benches/members/README.md) — and this section is only the four skeletons, so
nothing is copied out of an existing file to get the shape right. `python tools/dossier.py --id
'<feature>'` prints the three paths for any feature; all four trees share one path per feature
(`core/Str/length`, `lang/expressions/precedence-and-associativity`, `types/Throwable`, …).

**An example** — `docs/examples/<path>/03-slug.nvs`, three per member, each a *different* use, plus a
`.out` created with `python tools/dossier.py --bless <file>`:

```nvs
<?nvs
// One or two plain sentences: what this shows, in a reader's words. No ADR
// numbers, no internal vocabulary — the audience has never seen this repository.

array<string> $labels = ["Order #1042", "Café Größenwahn"];
foreach ($labels as string $label) {
    echo Core\Str::length($label), "\n";
}
```

**An attack** — `tests/hostile/<path>/01-slug.nvs`, no expected output at all. It passes when the
*runtime* survives: a throw, a limit, a clean fatal and a clean run are all passes; a panic, a hang,
a crash-shaped exit, a definite leak — **and a compile diagnostic** — are not.

```nvs
<?nvs
// Attack: what this tries to break, in one line.
// hostile: timeout-ms 20000     (optional; 10s otherwise)
// hostile: expect-refusal       (only when being refused IS the assertion)
```

**A bench** — `benches/members/<path>.nvs`, one file, iterations declared, inputs chained so no
optimiser can delete the loop:

```nvs
<?nvs
// What real work this stands for, in one line.
// bench: iterations 400000

class Bench {
    public static function run(uint $rounds): uint {
        array<string> $labels = ["order", "customer", "shipping address"];
        uint $total = 0;
        uint $i = 0;
        while ($i < $rounds) {
            $total = $total + Core\Str::length($labels[$total % 3]);   // chained: $total is last round's
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(400000), "\n";
```

**A `covers:` marker** — the one thing that attributes a *test* to a feature, since a case lives
where its suite wants it. In the `--FILE--` block of a `.nvst` case, or immediately above a Rust
`#[test]`, under any doc comment:

```rust
    /// The doc comment says what the test pins, as always.
    // covers: Core\Str::length
    #[test]
    fn length_counts_characters_not_bytes_or_code_points() {
```

A `Core` member is also credited by a case that plainly calls it (`Core\Str::length(`), so an
existing case needs no marker; an **instance** member and every language, tool or directive feature
needs one, and adding it to a case that already exists is the whole edit.

## An `.lspt` case

An LSP answer, frozen the way the section above freezes stdout. Sibling of `.nvst` and deliberately a
separate suite: `nvs test`'s `N passed` is a number the loop gates on, and it must keep meaning one thing.
The format is `crates/nvs-lsp`'s module doc; the section lexer is the same one `.nvst` uses. Lands at
M4B — [ADR 0099](../adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 5.

```
--TEST--
One sentence saying what is being pinned, ending with the ADR §§ it comes from
--FILE--
<?nvs
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
- **`--EXPECT--` is exact and frozen**, on the same terms as `.nvst`'s. The rendering it compares against
  is canonical and lives in `nvs_lsp::render` — a case never invents a spelling, and a case that seems to
  need one has found a gap in that module.
- `--FILE <relative/path>--` works exactly as it does for `.nvst`, which is how a go-to-definition case
  reaches across a `require`.
- **The document usually does not parse, and that is the point.** A case that only ever asks about valid
  code is not testing what the resilient tree exists for.
- A new file under `tests/lsp/` is picked up with no registration, and its coverage is **inferred** from
  the node the cursor resolved to — `nvs lsp-test --coverage` prints the matrix and
  `every_request_answers_every_construct` fails naming each empty cell.

## A `Core` member — the five edits

All five in the module that owns the class; `python tools/brief.py`'s *anchors* block resolves each
spelling to a file and line. Worked example: `crates/nvs-stdlib/src/json.rs`, which is small enough to
read whole.

**1. The row**, in that module's `pub const CLASS: CoreClass`:

```rust
CoreMethod {
    name: "isValid",
    names: &["json"],
    params: &[CoreTy::Text(Qual::Neutral)],
    defaults: &[],
    return_ty: CoreTy::Bool,
    symbol: "nvs_core_json_is_valid",
    doc: Some(&IS_VALID_DOC),
},
```

`names` is the spec's signature column, one per positional slot and never the `$` (ADR 0063 R2);
`every_registry_rows_names_are_the_specs_signature_column` holds the two together.

**2. The card** — ADR 0117's reference documentation, a `const` in the block of cards directly after the
class, in row order. `every_registry_row_carries_a_reference_card` fails `cargo test -p nvs-stdlib`
without it, and so does an enum without its `EnumDoc` or a constant with an empty `desc`:

```rust
/// `Core\Json::isValid`'s reference card — ADR 0117.
const IS_VALID_DOC: MethodDoc = MethodDoc {
    short: "Reports whether `$json` is a well-formed JSON document, as `json_validate` does, \
            without building the value.",
    params: &[ParamDoc {
        name: "json",
        desc: "The text to check.",
        shape: &[],
    }],
    ret: "`true` for a document `decode` would accept, `false` otherwise.",
    errors: &[],
};
```

The field docs on `MethodDoc`, `EnumDoc` and `CoreConst::desc` in `registry.rs` are the rule; the
three things they do not say are these. `params` is the row's `names` in order, then one entry per
option of a trailing options bag under the option's own name
(`a_documented_rows_param_docs_agree_with_its_names`), and `shape` is filled only for a fixed-key
shape parameter. `errors` is **what the body throws** — every `Fault::thrown_as(ThrownClass::…)` the
helper reaches, by the class's `catch` name from the spec's § 10 tree (`LogicError`, `ParseError`), one
entry per class with its conditions in the `desc`, and never an error the body does not raise. Two
readings that cost the backfill time: a bare `Fault::thrown(…)` is `RuntimeError` (`abi.rs` says so),
and a `Fault::fatal` is not a throw at all — no `catch` sees it, so it is never an entry, though `ret`
may say so where a caller would otherwise expect one. And the card is a condensed reference: one or two sentences a
field, no examples, no tips — an essay belongs in the website's page, which renders the card above it.

**3. The body**, via the macro:

```rust
nvs_runtime::nvs_helper! {
    /// `Core\Json::isValid(string $json): bool` — replacing `json_validate`.
    ///
    /// Why this shape rather than the obvious one, if that is not obvious.
    fn nvs_core_json_is_valid(_ctx, args: [1]) {
        let text = text_of(&args[0], "isValid")?;
        // …
    }
}
```

**4. The `address()` arm** — the one that bites, because a miss is a *runtime* panic naming the symbol
rather than a link error:

```rust
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_json_is_valid" => (nvs_core_json_is_valid as *const ()).cast(),
        _ => return None,
    })
}
```

**5. Three `.nvst` cases that call it** — one is what makes the member reachable, and
`crates/nvs-stdlib/tests/conformance_coverage.rs`'s floor is three, each asking a *different* question
(a second case asking the same one does not count). `cargo test -p nvs-stdlib` fails below either
number. An instance member needs a case writing `->name(`.

`args: [N]` must equal the row's arity, where an options bag flattens to one argument per option and an
instance receiver is slot 0 and absent from `params`. The playbook's *Adding a `Core` member* section has
the arithmetic and what a mismatch looks like.

## Citing a document

**The form depends on whether a renderer resolves the link, and there are exactly two.**

| Where you are writing | The form | Why |
|---|---|---|
| a `.md` file | relative to the file — `](../adr/0067-core-db.md)` | GitHub and the website render it, and both resolve against the file's own location |
| a `.rs`, `.nvs` or `.nvst` file | absolute from the repository root — `](/docs/adr/0067-core-db.md)` | nothing renders it, so the readers are people, agents and `grep` |

A source file's link had a `../` prefix until it was measured: 465 of 1,640 were dead — 442 with the
wrong number of `../`, 23 naming a filename its ADR no longer had. A prefix encodes the **citing**
file's depth, so every split, rename and new directory level silently invalidated every link in the
half that moved, and nothing looked. The root-absolute form has one spelling per target and survives
the move, which is the whole reason it is worth two rules instead of one.

`python tools/check-links.py` is the gate for both, and CI's `docs` job runs it. Two shapes in a
source file are not paths and it skips them: a rustdoc intra-doc link naming an item
(`[the store](Cache::store)`) has no `/`, and a link to a rustdoc page (`../nvs_ir/ids/index.html`)
is deliberately relative to the rendered HTML. `.py` is outside the gate — `tools/` emits markdown,
so a link in a string literal there is relative to the *generated* file.

Neither form is what rustdoc follows: a relative link in a doc comment resolves against the generated
HTML page, where `../../../docs/` has never existed, so `cargo doc` was never a check on any of this.

## An ADR

**Do not assemble one by hand.** `python tools/adr.py --draft > .agent-tmp/adr.md` prints the skeleton,
and `python tools/adr.py --new .agent-tmp/adr.md` claims the next free number, derives the
filename, dates it, folds the `Amended by:` back-link into every ADR you amend, adds the routing row and
the ground-rules bullet, regenerates the index table and re-audits — restoring every byte if the tree
gained a finding. Write the prose; the rest of README.md § *Adding a decision* is a form, and that is the
call that fills it. Newest worked example: [0104](../adr/0104-an-application-is-an-entry-file-path.md).

**The skeleton is not copied here.** Run `--draft` and read what it prints: the copy that used to sit
here had drifted from the tool that generates and checks it, which is what a second home costs. Below
is what a *finished* ADR carries — a different question, and the one that survives you.

**Six rules the tool enforces, so none of them is a matter of care:**

- **The field set is closed** — `Status:`, `Date:`, `Scope:`, `Depends on:`, `Amends:`, `Amended by:`,
  `Validated by:`, and nothing else; `tools/adr.py:115` is that list. The draft offers you neither
  `Date:` nor `Amended by:`, because both are the tool's to maintain. There is no `Relates to:`; the
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
cross-link — never a new paragraph elsewhere describing the change. The cross-link's two halves and the
three index rows are what `--new` writes for you; **the body edit is the half no tool can do**, and both
`--new` and `--fold` end by naming the file that still owes it.

For an ADR that already exists, `python tools/adr.py --fold 0110 --into "<a markdown link to 0033> § 4 —
what changed there"` writes both halves of the amendment, `--set-status 0033 Superseded` moves the index cell
with the status, and `--next-section 0007` says where a new `### N.` goes without renumbering one.

## A diagnostic

Next free code per band: `python tools/brief.py`. Bands are by compiler phase and the legend is
`crates/nvs-diagnostics/src/lib.rs`'s own table. Declare it there as a `Code::new` constant next to its
siblings — that file is the whole registry, so a code declared anywhere else does not exist.

Never reuse a retired number; the next free one is the band's highest plus one, deliberately not the
lowest hole.

## An edit the Edit tool cannot express, or a run of three or more edits

```
python tools/splice.py --patch <patch-file>
python tools/splice.py --patch <patch-file> --dry-run
```

The patch file names each file it edits with a `--- <path>` line and holds that file's blocks under it,
written with the **Write tool** — never a heredoc, which eats exactly the backslashes this repository's
Rust is full of:

```
--- crates/nvs-ir/src/lower/expr.rs
<<<<<<< OLD
the exact text to find
=======
the text to put there instead
>>>>>>> NEW
```

A second `--- <path>` starts the next file's blocks. Blocks apply in order and **the whole patch applies
or none of it does**, across every file it names — a stale anchor in the last file cannot leave the first
one half-edited. A failed match reports the line where the anchor stopped matching and what the file has
there instead.

`splice.py <target> --patch <f>` is the older one-file form, whose blocks carry no `--- <path>` line;
naming a target *and* using headers is refused rather than guessed at. Reach for a patch from three edits
up — one or two are cheaper as plain `Edit` calls, and [commands.md](commands.md) § *One shell call runs
one command* has the measurement. **A patch cannot carry a patch**: a block whose text contains the
markers themselves ends at the first `=======` inside it, so an edit to prose *about* this format is one
the `Edit` tool has to make.

## A status-block field

```
python tools/plan.py                                  # field names and sizes
python tools/plan.py --get "Open now"                 # its current text
python tools/plan.py --set "Open now" --from <file>   # replace it
```

The field set is fixed and `plan.py` refuses a name that is not already there. Write the new text with
the Write tool; `--set` re-wraps that one field and leaves every other byte of the file alone.
