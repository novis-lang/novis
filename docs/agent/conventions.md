# Conventions — the shapes this repository writes

Every answer here is the same in every session. They were being re-derived with a read of an existing
example each time, which is why this file exists.

**It holds skeletons and pointers, never a second copy of a rule.** Each section names a real file that
CI already runs, because a worked example that is also a passing test cannot go stale the way a pasted
snippet can. Where a shape is short enough to write out, it is written out; where it is not, the pointer
is the answer. If a skeleton here disagrees with the file it names, **the file is right** — fix this.

The rules themselves live elsewhere: [AGENTS.md](../../AGENTS.md) for what binds every agent, the
rulebook under [docs/rules/](../rules/) for a settled rule, the decision record a rule's `because` names
for its reasoning, the crate's module doc for how a subsystem works, [playbook.md](playbook.md) for traps.

## A commit message

```
<type>(<scope>): <the concrete change, named>, and <a second one>

The next most useful fact. Then, only if the diff does not say it, why this
shape rather than the obvious alternative. Name the rule (`rule:types/conversion`)
or the module doc that owns the rule, rather than restating it. No bullet lists
unless the content is genuinely a list.
```

`type` is `feat`, `fix`, `docs`, `refactor`, `perf` or `test`. `scope` is the subsystem, not the crate
path — `lang`, `types`, `ir`, `runtime`, `stdlib`, `codegen`, `syntax`, `test`, `adr`, `loop`, `agent`,
`abi-probe`. The subject is one line, lower case after the colon, no trailing period, and it reads as a
statement of what is now true rather than an instruction. Two clauses joined by `, and ` is the house
style when a commit does two things; one clause is fine when it does one.

**The first lines carry the information.** The subject names the concrete thing — the file, the goal by
slug, the request, the construct — so `git log --oneline` alone says what each commit did, and two
commits of one kind never share a subject. The body's first line is the next most useful fact. Why comes
after, and only what the diff and the owning module doc do not already say. The shortest message that
carries the same information is the right one, and a message a tool writes is held to the same bar: no
paragraph is repeated from one commit to the next.

**No trailers, ever.** A commit message documents the work and stops. No `Co-Authored-By`, no
`Signed-off-by`, no `Generated-with`, no tool or model attribution in any spelling. Who or what wrote a
commit is not part of the record this project keeps, and boilerplate at the foot of every message is
noise every future reader pays for. `git log` should read as a history of the language, not of its
authorship. This is enforced twice — `tools/session.py` strips a trailer out of any message it is handed,
and the `commit-msg` hook in `tools/git-hooks/` rejects one that arrives any other way.

Write the message to a file and use `git commit -F <file>` — never `-m` for anything multi-line, per
[commands.md](commands.md). `git log -1 --format=%B` is not needed to remember this; that is what this
section is for.

## A code comment

A comment says what the code does **now**. It is never a record of how the code got here, and this
repository has exactly one of those: `git log`. (`CHANGELOG.md` is a generated subset of it, written by
`tools/release.py` and edited by nobody.) The same rule that keeps a rule's fragment true in the present
tense — [doc-style.md](doc-style.md) § *Edit the rule, never overlay it* — is the rule for a `//!` header,
a `///` on a `Core` member, a `#` in a manifest and a `//` inside a `.nvst` case. Prose is prose.

The failure this exists to stop is cumulative, and it is the one an agent makes by default: a comment is
edited by writing the new truth **beside** the old one — "originally a `Vec`, now a slab", "this was
later widened to also take a closure" — and after enough sessions the comment is a diff of every session
that touched the file, which a reader has to replay in their head to learn what the function does today.
**So a comment is rewritten as a whole and never appended to.** When the code under it changes, write
the comment again from the code as it now stands and delete the sentence describing what it used to do;
rephrase the whole paragraph if that is what it takes. Reasoning is still worth writing — but in the
present tense, as what this shape buys rather than what the previous shape cost, citing the record that
owns it (`docs/decisions/0007.md § 3`, `rule:types/conversion`) instead of restating it.

These are therefore never in a comment:

- **A date.** When something was decided, changed or measured is not a property of the code. `git blame`
  answers it exactly; a comment answers it approximately and then rots. The apparent exception is not
  one: a date that is a **value the code handles** — a SQL literal, an epoch constant, the day a fixture
  is built on — stays, and belongs in backticks like every other literal in these comments. A bare date
  in a sentence is always either history or a value in the wrong clothes.
- **A count of anything that can change** — "the four subsystems", "the third of these", "all ten
  members". The next feature makes it wrong, and the only edit such a sentence ever gets is the one that
  bumps the digit, which is a session spent on nothing. Write "each subsystem", "the members below", and
  let the list be its own count. A number that is a fixed property of the code — a two-word header, a
  16-byte alignment, a limit the code enforces, an ABI offset — is a fact about the code and stays.
- **A measurement the code does not enforce** — "takes a minute or two", "cut it by 40%", "eight
  sessions paid it". A measured number lives with the guard test or the perf note that took it
  ([doc-style.md](doc-style.md)), and a comment quoting one is a copy that no run ever updates. State
  the shape of the cost instead — "the release relink dominates this step" — and cite the home when the
  figure itself is the argument.

None of this asks for a comment to be shorter or plainer. A dense paragraph explaining why a lock is
held across an await is exactly what belongs here; it just gets written as though it had always been
true.

## A `.nvst` test case

Canonical worked example, with every section that matters:
[tests/conformance/core/json-derive-encodes-declared-fields.nvst](../../tests/conformance/core/json-derive-encodes-declared-fields.nvst).
The format itself is `crates/nvs-test`'s module doc.

```
--TEST--
One sentence saying what is being pinned, ending with the `rule:` tokens it pins
--FILE--
<?nvs
// Comments cite the rule each block exists for. A case is top-level
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
// One or two plain sentences: what this shows, in a reader's words. No rule
// ids, no record numbers, no internal vocabulary — the audience has never
// seen this repository.

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
M4B — `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.

```
--TEST--
One sentence saying what is being pinned, ending with the `rule:` tokens it pins
--FILE--
<?nvs
class User { public string $name; }
var $u = new User();
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

`names` is the spec's signature column, one per positional slot and never the `$` (`rule:core-api/shape-rules` R2);
`every_registry_rows_names_are_the_specs_signature_column` holds the two together.

**2. The card** — `rule:core-api/reference-card`'s reference documentation, a `const` in the block of cards directly after the
class, in row order. `every_registry_row_carries_a_reference_card` fails `cargo test -p nvs-stdlib`
without it, and so does an enum without its `EnumDoc` or a constant with an empty `desc`:

```rust
/// `Core\Json::isValid`'s reference card — `rule:core-api/reference-card`.
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

**A rule is cited by its token, and the token is the whole citation.** `rule:types/conversion` is the
canonical spelling everywhere — a Rust doc comment, a `.nvst` header, a goal manifest, a commit message,
one rule citing another. It is flat text, not a link: `grep -rn 'rule:'` finds every citation in the
repository, `python tools/rules.py --check` reports one that resolves to nothing, and the generated
chapters linkify it on the way out. A rule id is its fragment's path — `types/conversion` is
`docs/rules/types/conversion.md` — so the token also says where the prose is.

**A decision record is linked, never cited as the rule.** `docs/decisions/NNNN.md` is frozen rationale:
link to it when the *reasoning* is the point — why this shape, what it cost, what was rejected — and
name the rule beside it when a reader needs what is currently true. A record's text was true on its
date and is not maintained afterwards, so a citation that means "the current rule" and points at a
record is stale from the first later decision that touches it. `python tools/brief.py --where
<keyword>` routes a topic to the rule that owns it.

**When a record is linked, the form depends on whether a renderer resolves it, and there are exactly
two.**

| Where you are writing | The form | Why |
|---|---|---|
| a `.md` file | relative to the file — `](../decisions/0067.md)` | GitHub and the website render it, and both resolve against the file's own location |
| a `.rs`, `.nvs` or `.nvst` file | absolute from the repository root — `](/docs/decisions/0067.md)` | nothing renders it, so the readers are people, agents and `grep` |

A source file's link had a `../` prefix until it was measured: 465 of 1,640 were dead — 442 with the
wrong number of `../`, 23 naming a filename the record no longer had. A prefix encodes the **citing**
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

## A rule fragment

A rule is **two files**, and a decision that creates or modifies one writes both in the same commit as
the record. The pair is `docs/rules/<topic>.json` — the structure, which carries no prose — and
`docs/rules/<topic>/<slug>.md` — the prose, which carries no structure. `docs/rules/<topic>.md` is
generated from the two by `python tools/rules.py --render` and **is never edited**, along with
[ground-rules.md](../ground-rules.md) and [divergences.md](../divergences.md).

The JSON entry, inserted into the topic's `rules` array at the position § *Where a rule sits in the
order* below gives it — never appended because it is the newest:

```json
{
  "id": "core-classes/schema-plan",
  "title": "Every plan step carries a grade and its complete SQL, and an unknown grade grades up",
  "status": "shipped",
  "because": ["0145", "0067"],
  "divergesFromPhp": "the sentence after “PHP …” — omit the field entirely when it does not",
  "seeAlso": ["core-classes/schema-converges"],
  "guardedBy": ["tests/conformance/core/db-schema-plans-every-difference.nvst"]
}
```

- **The `id` is the path.** `core-classes/schema-plan` is `docs/rules/core-classes/schema-plan.md`, and
  the citation token everywhere is `rule:core-classes/schema-plan`. The topic half namespaces it, so two
  topics cannot collide, and `python tools/peek.py rule:core-classes/schema-plan` reads it.
- **The `title` is the rule as one statement of what is now true** — the same voice as a record's H1 and a
  commit subject, and the line `ground-rules.md` prints. Not a topic ("Plan grades"), not an instruction.
- **`status` is `shipped` or `designed`**, and `designed` is the honest answer for a rule the tree does
  not hold yet. `docs/novis.md` filters to `shipped` so every line in it runs; the rulebook carries both,
  and the pack marks a designed rule where it prints one.
- **`because` is decision records, first-created-then-amended**, and it is one half of a relation whose
  other half is those records' `changes:` blocks. Writing one without the other is what `rules.py --check`
  refuses.
- **`guardedBy` is what holds the rule** — a test path, not a description. It is how a reader gets from
  the rule to the thing that fails when it is broken, and the pack samples it rather than printing it all.

The fragment is ordinary markdown with **no heading and no front matter**: the title lives in the JSON,
so the file opens on the first sentence of the rule. Its **opening sentence is cut verbatim** into
`ground-rules.md`, so write a sentence that stands alone. Body length is the rule's own business — one
paragraph where one is enough, four where the rule has a table of cases in it.

**Then run `python tools/rules.py --render`**, which rewrites the three generated files. `--check` and
`--render --check` are CI's `docs` job, and `session.py --wrap` runs both for any session that has
touched `docs/rules/` — so a fragment edited without a render, or a rule renamed under its citations,
refuses the wrap rather than reaching CI.

## Where a rule sits in the order

**Both arrays read from the ground up — the order a language is built in, never the order the decisions
arrived in.** `git log` keeps the arrival order, so nothing is lost by not writing it twice, and a
reader who opens a chapter at the top meets the thing being declared before anything that constrains it.

[`docs/rules/_index.json`](../rules/_index.json) is where a chapter's place is written, and its `order`
values sit ten apart so a new topic slots in without renumbering its neighbours. The five bands, in the
order the generated pages walk them:

| Band | Chapters |
|---|---|
| The language itself | `programs`, `types`, `expressions`, `statements`, `classes`, `enums`, `iteration`, `attributes`, `errors` |
| What checks it before it runs | `tooling`, `ide`, `testing` |
| The runtime it runs on | `security`, `concurrency`, `core-api`, `core-classes`, `observability` |
| The server on top of that | `http-server`, `routing` |
| What surrounds the language | `config`, `packaging`, `php-migration` |

Inside a chapter there is no `order` field — **position in the `rules` array is the order**, and the
same principle repeats one level down:

- The rule that says a thing **exists** comes before every rule that narrows it, and a refusal sits with
  the thing it refuses rather than in a block of refusals at the end.
- The surface a **program writes** comes before the machinery underneath it — the queue's members before
  the parking contract, a directive before how a reload applies it.
- A rule that cannot be understood without another one goes **after** that one.
- A run that is already grouped — the `fmt-` rules, the `db-` rules, the capability rules — stays one
  run: a new member joins it rather than starting a second group elsewhere in the array.

Nothing checks this, because no tool can read whether one rule explains another. It is the reason a
reorder is cheap: moving an entry changes no content, and `--render` rewrites the chapters from it.

## A decision record

A record is the reasoning behind a rule, frozen on acceptance at `docs/decisions/NNNN.md`. **The rule
is not in it** — the rule is a fragment under [docs/rules/](../rules/), written to the shape in
§ *A rule fragment* above, and the record is what its `because` list names. Writing one without the
other is half a decision, and the two are one commit. There is no scaffolder: a new record is written by hand from the shape below and
claims the next free number, which is one more than the highest file in `docs/decisions/`. Newest
worked example: [0104](../decisions/0104.md).

```markdown
---
status: accepted            # accepted | retired
changes:
  creates:
    - types/nullable-conversion
  modifies:
    - types/conversion
    - core-api/no-try-prefix
---
# ADR NNNN — the decision, as a statement of what is now true

- **Scope:** what this decides, and what it deliberately leaves to another record.
- **Depends on:** the records this one builds on, as links.
- **Validated by:** the guard test or the measurement that holds it.

> **In short:** the decision in one paragraph. A reader who only needs the rule
> stops here.

## Context
## Investigation
## Options considered
## Decision
## Diagnostics
## Consequences
## Alternatives rejected
## Revisiting
## Verification
```

**What the shape means, so none of it is a matter of care:**

- **The YAML block is the record's only machine-read field set** — `status` and `changes`.
  `changes.creates` is the rule ids this decision brings into the rulebook; `changes.modifies` is the
  rule ids whose fragment it edits. Both name rules by id (`types/conversion`), never a record. A
  record touching no rule is not a decision — put the paragraph in the module doc or the plan instead.
- **The `changes:` block and the rules' `because` lists are one relation, written twice, in one
  commit.** Every rule under `creates` gets a new topic-JSON entry whose `because` opens with this
  record's number; every rule under `modifies` gets this number appended to its `because`. The freeze
  derived `changes:` from `because` in exactly that way — first entry created, the rest amended — so a
  record and the rulebook must keep agreeing. `python tools/rules.py --check` refuses a rule whose
  `because` is empty or names something that is not a record id; `python tools/records.py --check` audits
  the record; `python tools/records.py --graph NNNN` prints what the record created and modified and which
  records share a rule with it, derived from every `changes:` block.
- **The H1 is `# ADR NNNN — <the decision as a statement>`**, and it is the title every index derives
  from. `Scope`, `Depends on` and `Validated by` are the three bullets that may follow it, each present
  only when the record has one. Nothing else goes above *In short* — there is no `Status:` line, no
  `Amends:`, no `Amended by:`, no `Relates to:`; all of that is either in the YAML block or derived
  from it.
- **The heading set is closed and ordered**: `Context`, `Investigation`, `Options considered`,
  `Decision`, `Diagnostics`, `Consequences`, `Alternatives rejected`, `Revisiting`, `Verification`.
  `tools/records.py`'s `CANONICAL` list is that set, `--check` refuses any other `##`, and anything else is
  a `###` subsection under `Decision`. Use only the sections that have content; `Revisiting` is for a
  decision with a real trigger to reconsider it.
- **Sections are numbered `### N.` and never renumbered.** `0007 § 3` is cited from `crates/`, from
  `docs/spec/` and from the goal manifests; a new section between two others is `§ 3a`.
- **A record is frozen on acceptance.** Its body states what was decided when it was written and is not
  edited when a later decision moves the rule: that decision is a new record whose `changes.modifies`
  names the rule, and **the rule's fragment is where the current text lives.** A body therefore carries
  no history and no maintenance — no "this previously said", no withdrawn-section tombstone, no running
  total of anything — and the reader who wants to know what is true now reads the rule, never the
  record.
- **A record carries no date, and neither does its body.** When it was accepted, and when anything in it
  was measured or superseded, is `git log`'s answer to give; a stamp in the file cannot be checked
  against anything and tells a reader years later only that time has passed. The same rule the code
  comments follow — § *A code comment*, *A date* — applies to every file under `docs/`.

Retiring a decision is `status: retired` in its own YAML block, in the same commit that edits or
removes the rules it created; there is no `Superseded` status and nothing to move in an index, because
every index is derived.

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

## A playbook bullet

```
- **<The trap, as one claim a reader can match against their own symptom.>** <Why it happens, one
  or two sentences.> <What to do instead, one sentence naming the file, flag, command or rule.>
  [until: <kind> <arg>]
```

Three sentences and about 400 bytes; `session.py --wrap` refuses a new bullet past 700. The first
sentence is the bullet's selector — a goal manifest fetches it by the opening words of that bold text — so it
states the trap and not the story. The trailer is required, and `tools/playbook.py`'s module doc is the
only home of its five kinds: prefer the mechanical ones (`test`, `exists`, `gone`, `rule`) over
`reviewed`, because those are what let the wrap delete the bullet for you the day it stops being true.

What does **not** go in: the session's narrative (which stage, which check, what was tried first —
`git log` holds it), a measured number, a rule that already has a home (a `rule:` token, a module doc,
`AGENTS.md`), or a trap whose whole subject is a stale comment in `loop-goal.toml` or a wrong claim in a
handoff — fix the comment instead. `python tools/playbook.py --match <path>` before writing says whether
the trap is already there.
