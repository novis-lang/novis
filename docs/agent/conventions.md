# Conventions — the shapes this repository writes

Every answer here is the same in every session, so it is read here rather than re-derived from an
existing example.

**It holds skeletons and pointers, never a second copy of a rule.** Each section names a real file that
CI already runs, because a worked example that is also a passing test cannot go stale the way a pasted
snippet can. Where a shape is short enough to write out, it is written out; where it is not, the pointer
is the answer. If a skeleton here disagrees with the file it names, **the file is right** — fix this.

The rules themselves live elsewhere: [AGENTS.md](../../AGENTS.md) for what binds every agent, the
rulebook under [docs/rules/](../rules/) for a settled rule and the reasons behind it, the crate's module
doc for how a subsystem works, [playbook.md](playbook.md) for traps.

## A commit message

```
<type>(<scope>): <the concrete change, named>, and <a second one>

The next most useful fact. Then, only if the diff does not say it, why this
shape rather than the obvious alternative. Name the rule (`rule:types/conversion`)
or the module doc that owns the rule, rather than restating it. No bullet lists
unless the content is genuinely a list.
```

`type` is `feat`, `fix`, `docs`, `refactor`, `perf` or `test`. `scope` is the subsystem, not the crate
path — `lang`, `types`, `ir`, `runtime`, `stdlib`, `codegen`, `syntax`, `test`, `loop`, `agent`,
`abi-probe`. A commit that changes what `nvs fmt` writes takes the scope `fmt`, whatever its type, so
the release notes list it under *Formatter*. The subject is one line, lower case after the colon, no trailing period, and it reads as a
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
authorship. The `commit-msg` hook in `tools/git-hooks/` rejects a message that carries one, whoever
commits it.

The one line that is not attribution: the commit that fixes **a bug reported in a public issue** ends its
body with `Fixes #<n>`, which links the fix to the issue and closes it. Nothing else says a change came
from a report — no reporter's name, no quote ([user-report.md](user-report.md)).

Write the message to a file and use `git commit -F <file>` — never `-m` for anything multi-line, per
[commands.md](commands.md).

## Names

- **A test's name states the behaviour it checks,** as a sentence a reader can check against the
  body: `every_syntactic_form_is_reachable_through_a_heading` (`crates/nvs-cli/tests/agent.rs`), or
  `"a doc that still exists is not a finding"` (`tools/nv/test/rules.test.ts`). A `.nvst` case is
  named the same way, in kebab-case (§ *A `.nvst` test case*).
- **A doc and a goal slug are kebab-case,** and so is a TypeScript file ([bun.md](bun.md)).
- **Example data and every name that came out of a report are neutral,** as AGENTS.md § *The rules
  only this project has* says.

## A code comment

A comment says what the code does **now**. It is never a record of how the code got here, and this
repository has exactly one of those: `git log`. (`CHANGELOG.md` is a generated subset of it, written by
`bun nv release` and edited by nobody.) The same rule that keeps a rule's fragment true in the present
tense — [doc-style.md](doc-style.md) § *Edit the rule, never overlay it* — is the rule for a `//!` header,
a `///` on a `Core` member, a `#` in a manifest and a `//` inside a `.nvst` case. Prose is prose.

The failure this exists to stop is cumulative, and it is the one an agent makes by default: a comment is
edited by writing the new truth **beside** the old one — "originally a `Vec`, now a slab", "this was
later widened to also take a callable" — and after enough sessions the comment is a diff of every session
that touched the file, which a reader has to replay in their head to learn what the function does today.
**So a comment is rewritten as a whole and never appended to.** When the code under it changes, write
the comment again from the code as it now stands and delete the sentence describing what it used to do;
rephrase the whole paragraph if that is what it takes. Reasoning is still worth writing — but in the
present tense, as what this shape buys rather than what the previous shape cost, citing the rule that
owns it (`rule:types/conversion`) instead of restating it.

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
  sessions paid it". A measured number lives with the guard test or the ledger under `data/perf/`
  that took it ([doc-style.md](doc-style.md)), and a comment quoting one is a copy that no run ever
  updates. State the shape of the cost instead — "the release relink dominates this step" — and cite
  the home when the figure itself is the argument.

None of this asks for a comment to be shorter or plainer. A dense paragraph explaining why a lock is
held across an await is exactly what belongs here; it just gets written as though it had always been
true.

The one place a comment *is* asked to be plainer is a program an end user reads: the `.nvs` files
under `docs/examples/`, `tests/hostile/` and `benches/members/`. Their comments follow
[AGENTS.md](../../AGENTS.md) § *Text an end user reads*.

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
- The expectation is what the rules say Novis prints, never what another language prints. A section
  the runner does not know is an error, and that includes PHP comparison sections.
- A new file under `tests/conformance/` is picked up with no registration.

**The four shapes a depth case takes.** A new case reaches for one of these, and what it adds is the
boundary or the invariant the member is written around — never another row of the same shape.
`bun nv gaps` finds the candidates; the playbook owns the spellings that will not compile.

- *Edges* — the answer where the member stops accepting: an empty receiver, a name it cannot read, a
  padding only wide enough to be used. `bun nv gaps --errors` lists the boundaries no case asks about.
- *Invariance over a sweep* — a property that must hold across a whole table, asserted by **counting**
  rather than read off a line, so a member answering plausibly row by row still fails.
- *A bound asserted on both sides* — the last accepted value and the first refused one, named
  together. A member that stops one entry early prints plausibly against either half alone.
- *Agreement* — one question asked of every member that shares a rule, asserting that they **agree**
  rather than what each answered, so a member that grew its own comparison fails here while still
  looking right on its own line. The shape with the most room left.

## Feature proofs — an example, an attack, a bench, a `covers:` marker

`rule:testing/feature-proofs` makes these owed. **Each tree's own
README owns the rules** — [docs/examples/](../examples/README.md), [tests/hostile/](../../tests/hostile/README.md),
[benches/members/](../../benches/members/README.md) — and this section is only the skeletons, so
nothing is copied out of an existing file to get the shape right. `bun nv proofs --id '<feature>'`
prints the three paths for any feature; all four trees share one path per feature
(`core/Str/length`, `lang/expressions/precedence-and-associativity`, `types/Throwable`, …).

**An example** — `docs/examples/<path>/03-slug.nvs`, three per member, each a *different* use, plus a
`.out` created with `bun nv proofs --bless <file>`:

```nvs
<?nvs
// Shows how to count the characters of a text. `Core\Str::length` counts
// what a person sees, so "é" and "ö" are one character each.

array<string> $labels = ["Order #1042", "Café Größenwahn"];
// This prints 11 and 15.
foreach ($labels as string $label) {
    echo Core\Str::length($label), "\n";
}
```

**An attack** — `tests/hostile/<path>/01-slug.nvs`, no expected output at all. It passes when the
*runtime* survives: a throw, a limit, a clean fatal and a clean run are all passes; a panic, a hang,
a crash-shaped exit, a definite leak — **and a compile diagnostic, or an ending before the last
line** — are not. Every step that can be caught is caught; the one that cannot goes last.

```nvs
<?nvs
// Attack: a text of one billion characters is measured. The program should
// stop at its memory limit with an error, and must not crash.
// hostile: timeout-ms 20000     (optional; 10s otherwise)
// hostile: expect-refusal       (only when being refused IS the assertion)
// hostile: ends-early 3 FATAL   (only when the last step ends the program; it prints `step 3` first)
```

**A bench, with its growth** — `benches/members/<path>.nvs`, one file, iterations and complexity
declared, inputs chained so no optimiser can delete the loop. Its growth is checked by the ramp, and
a complexity other than `constant` also needs the `.scale.nvs` sibling its README describes:

```nvs
<?nvs
// Counts the characters of a short label, as a program does before it shortens or pads one.
// bench: iterations 400000
// bench: complexity constant

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

**Every comment in those three is a model, not a placeholder**, written to
[AGENTS.md](../../AGENTS.md) § *Text an end user reads*, which is the rule.

**`about.md` is a feature proof too**, the description the website shows above the examples. It has
no skeleton here: [docs/examples/README.md](../examples/README.md) § *The description* is its shape
and carries two models.

**Help in the binary is a feature proof too**, and it is not a file of its own: it is the card, the
reference section or the index entry `nvs agent find` reaches and `nvs agent show` prints, written in
the same slice as the feature (`rule:testing/feature-proofs`). Check it by running both commands with
the name a user would type.

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
The format is `crates/nvs-lsp`'s module doc; the section lexer is the same one `.nvst` uses
(`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`).

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

All five in the module that owns the class; `bun nv peek --locate <symbol>` resolves each spelling
to a file and line. Worked example: `crates/nvs-stdlib/src/json.rs`, which is small enough to
read whole.

A new *class* is a `CoreClass` const whose line above, past any attributes, is `// rank: N`: the
place it takes in `registry::CLASSES`, which `bun nv render` writes into `classes.rs`. Pick a rank
between its neighbours' — they are spaced by ten.

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

`names` is the published signature's parameter names, one per positional slot and never the `$`
(`rule:core-api/shape-rules` R2).

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

A **class** carries a card of its own on the same terms: a `ClassDoc` with one `short` saying what the
class is for, under the row's `doc` field, and `every_registry_row_carries_a_reference_card` fails a new
class without one. The field docs on `MethodDoc`, `EnumDoc`, `ClassDoc` and `CoreConst::desc`
in `registry.rs` are the rule; the three things they do not say are these. `params` is the row's `names` in order, then one entry per
option of a trailing options bag under the option's own name
(`a_documented_rows_param_docs_agree_with_its_names`), and `shape` is filled only for a fixed-key
shape parameter. `errors` is **what the body throws** — every `Fault::thrown_as(ThrownClass::…)` the
helper reaches, by the class's `catch` name from the `Core\Error` tree (`LogicError`, `ParseError`), one
entry per class with its conditions in the `desc`, and never an error the body does not raise. Two
readings are easy to miss: a bare `Fault::thrown(…)` is `RuntimeError` (`abi.rs` says so),
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
repository, `bun nv rules --check` reports one that resolves to nothing, and the generated
chapters linkify it on the way out. A rule id is its fragment's path — `types/conversion` is
`docs/rules/types/conversion.md` — so the token also says where the prose is.

**A reason is cited by the rule that holds it.** There are no decision records, so a line that needs a
reason cites the rule or states the reason in place, and `bun nv brief --where <keyword>` routes a topic
to the rule that owns it. A new decision becomes a goal, and when it lands, what it decided, why, what
it spends and what it declined go into its rule fragment.

**A section mark names its document in the same phrase, in one of three forms.**

| The document | The form |
|---|---|
| an RFC | `RFC 3986 § 5.4` |
| another standard | its name before the mark: `[MS-TDS] § 2.2.6`, `FIPS 180-4 § 6.2`, `WHATWG URL § 5` |
| a document in this repository | its heading in asterisks: `AGENTS.md` § *The priority ordering* |

A second mark of the same standard joined by `and`, `or` or a comma keeps the name: `RFC 9110 § 8.8
and § 13.1`. Any other `§ N` is bare, because a reader cannot tell what it cites, even where the file's
head comment names a standard: write `RFC 8446 § 3`, not `§ 3`. A section of this repository's
documents is cited by its heading, never its number, so a new section above it does not change the
citation. `bun nv rules --check` fails on each bare mark in a file under `crates/` and
`tests/conformance/`, and `bun nv rules --marks <paths>` lists them. In a conformance case it reads the
description and the comments only, so a `§` the case's program tests is not a citation. The standards it
recognises are `NAMED_STANDARD` in `tools/nv/cmd/rules.ts`; a new one is added there with a case.

**A link to another document depends on whether a renderer resolves it, and there are exactly two
forms.**

| Where you are writing | The form | Why |
|---|---|---|
| a `.md` file | relative to the file — `](../rules/types.md)` | GitHub and the website render it, and both resolve against the file's own location |
| a `.rs`, `.nvs` or `.nvst` file | absolute from the repository root — `](/docs/rules/types.md)` | nothing renders it, so the readers are people, agents and `grep` |

A `../` prefix encodes the **citing** file's depth, so every split, rename and new directory level
silently breaks every link in the half that moved. The root-absolute form has one spelling per target
and survives the move, which is the whole reason it is worth two rules instead of one.

`bun nv links` is the gate for both, and CI's `docs` job runs it. Two shapes in a
source file are not paths and it skips them: a rustdoc intra-doc link naming an item
(`[the store](Cache::store)`) has no `/`, and a link to a rustdoc page (`../nvs_ir/ids/index.html`)
is deliberately relative to the rendered HTML. A `.py` file is read only for the repository paths its
prose names, and a `.ts` file not at all: a tool emits markdown, so a link in one of its string
literals is relative to the *generated* file.

Neither form is what rustdoc follows: a relative link in a doc comment resolves against the generated
HTML page, where `../../../docs/` does not exist, so `cargo doc` is no check on any of this.

## A rule fragment

A rule is **two files**, and a change that creates or modifies one writes both in one commit. The pair is `data/rules/<topic>/<slug>.json` — the structure, which carries no prose — and
`docs/rules/<topic>/<slug>.md` — the prose, which carries no structure. The rule's place is its id in
the `rules` list of its topic's record, `data/rules/<topic>.json`, inserted at the position § *Where a
rule sits in the order* below gives it — never appended because it is the newest. `docs/rules/<topic>.md`
is generated from all of them by `bun nv rules --render` and **is never edited**, along with
[ground-rules.md](../ground-rules.md).

The record, `data/rules/core-classes/schema-plan.json`:

```json
{
  "title": "Every plan step carries a grade and its complete SQL, and an unknown grade grades up",
  "status": "shipped",
  "seeAlso": ["core-classes/schema-converges"],
  "guardedBy": ["tests/conformance/core/db-schema-plans-every-difference.nvst"]
}
```

- **The id is the path.** `core-classes/schema-plan` is `docs/rules/core-classes/schema-plan.md`, and
  the citation token everywhere is `rule:core-classes/schema-plan`. The topic half namespaces it, so two
  topics cannot collide, and `bun nv peek rule:core-classes/schema-plan` reads it.
- **The `title` is the rule as one statement of what is now true** — the same voice as a commit subject, and the line `ground-rules.md` prints. Not a topic ("Plan grades"), not an instruction.
- **`status` is `shipped` or `designed`**, and `designed` is the honest answer for a rule the tree does
  not hold yet. The rulebook carries both, and the pack marks a designed rule where it prints one.
- **A `designed` rule names the goal that ships it**, as a gap does: `goal`, the slug of any goal
  file under `docs/goals/`. A `shipped` rule names none. The chapter prints the owner beside the title, and
  `bun nv owners` lists every designed rule by its owner.
- **`guardedBy` is what holds the rule** — a test path, not a description. It is how a reader gets from
  the rule to the thing that fails when it is broken, and the pack samples it rather than printing it all.

The fragment is ordinary markdown with **no heading and no front matter**: the title lives in the JSON,
so the file opens on the first sentence of the rule. Its **opening sentence is cut verbatim** into
`ground-rules.md`, so write a sentence that stands alone. Body length is the rule's own business — one
paragraph where one is enough, four where the rule has a table of cases in it.

**A rule keeps its own reasons.** After the statement of what is true, the fragment says why it holds,
in the same present-tense voice: the bound it sets, the attack it closes, the cost it avoids. It never
says where a reason came from: no report, no date, no "the rule used to say". A
reason the code or a test contradicts is not written; a still-wanted change becomes a gap under
`data/gaps/`. Why a crate is built the way it is goes into the crate's module doc instead.

**What the rule spends is a `**Memory:**` bullet**, per request or per task, stated as what is held
and when: "**Memory:** each read of a match costs one string and one integer. A program that never
reads a match pays nothing."

**What was declined goes under a bold `**Declined:**` line at the end**, one bullet per alternative.
A bullet opens with the declined thing in bold and gives the reason in one or two sentences, citing
the rule that already says it where one does:

```markdown
**Declined:**
- **The server calls the matched method.** Calling it would be dispatch
  (`rule:routing/the-servers-match-dispatches-nothing`).
```

Something declined that no rule covers becomes a rule of its own whose title says it does not exist,
the way `rule:routing/no-wildcard-verb` does. `rule:routing/a-match-is-not-invocable` is a worked
example of all three.

**Then run `bun nv rules --render`**, which rewrites the three generated files. `--check` and
`--render --check` are CI's `docs` job, and a session that touched `docs/rules/` runs both before it
wraps, so a fragment edited without a render, or a rule renamed under its citations, is caught before
it reaches CI.

## Where a rule sits in the order

**Both lists read from the ground up — the order a language is built in, never the order the decisions
arrived in.** `git log` keeps the arrival order, so nothing is lost by not writing it twice, and a
reader who opens a chapter at the top meets the thing being declared before anything that constrains it.

A chapter's place is the `order` field of its topic record, `data/rules/<topic>.json`, and the
values sit ten apart so a new topic slots in without renumbering its neighbours. The five bands, in the
order the generated pages walk them:

| Band | Chapters |
|---|---|
| The language itself | `programs`, `types`, `expressions`, `statements`, `classes`, `enums`, `iteration`, `attributes`, `errors` |
| What checks it before it runs | `tooling`, `ide`, `testing` |
| The runtime it runs on | `security`, `concurrency`, `core-api`, `core-classes`, `observability` |
| The server on top of that | `http-server`, `routing` |
| What surrounds the language | `config`, `packaging`, `php-migration` |

Inside a chapter there is no `order` field — **position in the topic record's `rules` list is the
order**, and the
same principle repeats one level down:

- The rule that says a thing **exists** comes before every rule that narrows it, and a refusal sits with
  the thing it refuses rather than in a block of refusals at the end.
- The surface a **program writes** comes before the machinery underneath it — the queue's members before
  the parking contract, a directive before how a reload applies it.
- A rule that cannot be understood without another one goes **after** that one.
- A run that is already grouped — the `fmt-` rules, the `db-` rules, the capability rules — stays one
  run: a new member joins it rather than starting a second group elsewhere in the list.

Nothing checks this, because no tool can read whether one rule explains another. It is the reason a
reorder is cheap: moving an entry changes no content, and `--render` rewrites the chapters from it.

## A diagnostic

Each area of the compiler has its own range and its own file under `crates/nvs-diagnostics/src/code/`,
and that module's head comment is the table of both. Declare a code in its area's file as a `Code::new`
constant with its card, and take the highest number in that file plus one: never a retired number in
a hole. A test fails on a code outside its file's range and on a number declared twice. A full range
continues in a new range and a new file, which the table names.

## An `nvs` command

A command is its own module under `crates/nvs-cli/src/`, and that module owns the command's arguments
and how they are run. It declares a `#[derive(clap::Args)] pub(crate) struct Args` whose doc comment is
the command's help line, and an `Args::run(self, ...)` that takes only what the command reads from the
global flags. `main.rs` then gets two lines: the variant `Name(name::Args)` in `Command`, with no doc
comment, and the arm `Command::Name(args) => args.run(...)`. A command with subcommands gives its
`Args` one `#[command(subcommand)]` field holding a private `#[derive(clap::Subcommand)] enum Command`,
which `Args::run` matches; `crates/nvs-cli/src/schema.rs` is that shape. A hidden alias is the one
variant that keeps a doc comment, beside its `#[command(hide = true)]`. `crates/nvs-cli/src/ast.rs`'s
`Args` is the shape.
Moving a command keeps every help page byte for byte the same, so compare `nvs <command> --help` and
`-h` before and after.

## An edit the Edit tool cannot express, or a run of three or more edits

```
bun nv splice --patch <patch-file>
bun nv splice --patch <patch-file> --dry-run
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

`nv splice <target> --patch <f>` is the one-file form, whose blocks carry no `--- <path>` line;
naming a target *and* using headers is refused rather than guessed at. Reach for a patch from three edits
up — one or two are cheaper as plain `Edit` calls ([commands.md](commands.md) § *A shell never carries
file content into the tree*). **A patch cannot carry a patch**: a block whose text contains the
markers themselves ends at the first `=======` inside it, so an edit to prose *about* this format is one
the `Edit` tool has to make.

## The README and the status

**`README.md` is a short version of the website's home page, in its order and its words.** Its H1 is
the project's title, and the `nvs --help` banner and the extension's `displayName` are tested against
it. The feature list follows `website/src/content/docs/index.mdx`'s sections in their order. Each
item takes the component's sentence for the same point and links to its page on
`https://novis-lang.org/`, never to `docs/rules/` or into `website/src/`. A heading the website also
has (*Sponsoring*, the roadmap, the status) takes the website's text, shortened. The README copies no
measured number and names no version, so a recorded run or a release leaves it current. The whole
file is held to AGENTS.md § *Text an end user reads*: it is the first page most readers see. A change
to the home page's order or sentences changes the README in the same commit.

**The status is "Pre-Alpha", always with that capitalization.** `website/src/data/release.ts`'s
`status` is what every page says. Each place that names the status says that Novis is released and
what Pre-Alpha means, in one plain sentence: any release can change the language, and Novis is not
ready for production use. The README, `CONTRIBUTING.md` and `SECURITY.md` say the same. `SECURITY.md`
names the latest Pre-Alpha release and `main` as the ones that get security fixes.

## A playbook bullet

```
- **<The trap, as one claim a reader can match against their own symptom.>** <Why it happens, one
  or two sentences.> <What to do instead, one sentence naming the file, flag, command or rule.>
  `until: <kind> <arg>`
```

Three sentences and about 400 bytes; `looper wrap` refuses a new bullet past 700. The first sentence
states the trap and not the story, because it is what a session matches against its symptom. It is
also the bullet's file name: each bullet is `docs/agent/playbook/<section>/<lead-in words>.md`. The
trailer is required, and looper's `docs/goal-format.md` § *The playbook* is the only home of its kinds.
Each one is a condition the tree answers, so `looper playbook retire` deletes the bullet the day it
stops being true. A bullet names, in backticks, at least one file in the tree by its path from the
repository root, and the wrap refuses one that names none.

What does **not** go in: the session's narrative (which stage, which check, what was tried first —
`git log` holds it), a measured number, a rule that already has a home (a `rule:` token, a module doc,
`AGENTS.md`), or a trap whose whole subject is a stale line in a goal's record or prose or a wrong claim
in a handoff — fix the line instead. Grep `docs/agent/playbook/` for the path before writing, to find
the traps already filed against it.

## A goal

Every session that runs a goal reads it, so its author plans once what each session would otherwise
work out again. How a goal is written in full is [loop-authoring.md](loop-authoring.md).

- **Every stage fits one session.** A stage that needs more than the 120k slice budget is split into two
  stages before the run starts.
- **Every piece of a stage's work is an item.** The tests, feature proofs, rule flips and deleted gap
  records a stage owes are items too, each with its rule and its `path:line` anchor. A file the stage
  creates is anchored at `path:1`.
- **A set is listed, never described.** "The eleven `about.md` files" is written out in the goal, or its
  item says that an `Explore` subagent lists it first.
- **A stage names its split.** A `**Split:**` paragraph after the `**Does:**` one says what goes to
  subagents: the searches, and the batches of similar files (pages, examples, attacks, `.lspt` cases,
  fixtures) that writing subagents produce. Writing subagents whose files are disjoint run side by side
  in the tree, and none of them builds; the session builds once they all end. Ones that share a file
  run one after another. A stage with nothing to hand off has no such paragraph.
- **The goal names its lanes.** `## Standing decisions` says which stages touch disjoint files and can run
  as side goals, and which share a file and run in order.
- **A dependency is a gate.** A stage that needs another goal's work names the file or command that
  proves it landed. `### Next` starts only work whose gate holds.
- **Every choice is made before the run.** A standing decision states the choice and its safe default.
  That covers a new dependency, a port, a command's shape and where a test's fixtures come from. A goal
  never says "goes to the user": the session decides by the priority ordering and records why.
- **A goal is verified once, at its end.** Its last stage is the gate stage (`gate: true`), which
  `looper goal new` writes: it runs `bun nv affected --run` over the whole goal, fixes every failure in
  one pass, and runs it once more. Every other stage checks with a build. An item asks for a test or
  verify run of its own only where the stage cannot be finished without seeing it pass, and then the
  item names the exact command. A slice whose change breaks an existing test updates it as it goes.

## A handoff

The live goal's `## Handoff` in `docs/goals/<slug>.md`, overwritten whole every session and never
appended to, because every later session reads it whole. It aims at about 60 lines, and no session is
spent trimming it.

- **`### State`**: a few lines on what is on disk and what is blocked. A settled fact leaves it for a
  rule, a module doc or a commit message.
- **`### Next`**: the next group, under one line naming the stage, the file set its slices share and
  the gate it waits on. It lists every open item of the group, its tests and rule flips included, so
  finishing `### Next` finishes the group. Each item names the rule that specifies it and its anchors
  repo-rooted (`crates/nvs-runtime/src/ctx/isolate.rs:116`), because the item is where `looper orient`
  reads the code it inlines into the next pack.
- **`### Backlog`**: up to six one-line items, each with its owning doc.

The handoff is state. A fact that will still be true in ten sessions goes to the playbook, a rule's
fragment or a module doc, and the handoff points at that home.
