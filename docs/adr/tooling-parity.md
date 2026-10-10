# The PHP tool ecosystem, tool by tool — where each job landed

PHP outsources testing, static analysis, style, coverage and debugging to userland packages; Novis's
stance is that writing clean, tested code requires none of them. This page is the register of that
claim, indexed the way a PHP developer asks it — by the tool they would otherwise install — with the
owner of each job. Verified against the ADRs.

**The linked ADR is the rule; this page is only the index.** A row never states a mechanism, a
diagnostic or a migration path — those live in the ADR. The reader-facing rendering is the *tools
you do not install* section of
[docs/reference/tools/30-php-differences.md](../reference/tools/30-php-differences.md), which is
limited to what ships; this page also carries what is only planned, which that chapter must not
([docs/reference/README.md](../reference/README.md) § *Who reads this*).

| PHP tool | The job | The Novis answer | Owner | Lands |
|---|---|---|---|---|
| PHPUnit | declaring, running and asserting tests | `#[Test]` + `Core\Test` + `nvs test` | `rule:testing/test-attribute`, `rule:testing/assertions-are-typed`, `rule:testing/report-formats` | M4S tail — on disk |
| Pest | a nicer test-declaration syntax | nothing — one surface, no base class to improve on | `rule:testing/test-attribute` | — |
| Paratest | parallel test execution | every test is its own isolate; parallel is the default, not a flag | `rule:testing/isolate-per-test` | M5 |
| Mockery / Prophecy / PHPUnit mocks | test doubles | a shape literal of closures, structurally checked against the interface | `rule:testing/doubles`, `rule:testing/interaction-after-the-fact` | M4S or M5 |
| DAMA DoctrineTestBundle / dbunit | database-test isolation | `#[Test(db:)]` is a transaction the runner rolls back | `rule:testing/db-transaction` | M8 |
| WebTestCase / BrowserKit | HTTP-level tests | in-process dispatch through the compiled route table | `rule:testing/in-process-request` | M8 |
| phpunit-snapshot-assertions | snapshot tests | inline snapshots; the updater writes into the test | `rule:testing/inline-snapshots` | M8 |
| Eris | property-based testing | generators derived from the declared parameter types | `rule:testing/property-testing` | M4S or M5 |
| Infection | mutation testing | `nvs test --mutate` — type-valid mutants, run against the tests coverage says touch them | `rule:testing/mutation-testing` | M10 |
| PHPBench | benchmarking | `#[Bench]` reports the deterministic counter stream, so CI can gate on it | `rule:testing/bench-counters` | M10 |
| PHPStan / Psalm / Phan | static analysis | `nvs check` — the compiler is the analyzer; see below | `rule:types/declaration` and the section below | on disk |
| Psalm `--taint-analysis` | taint analysis | `tainted` and `secret` are type qualifiers, enforced as hard errors | `rule:security/tainted-qualifier`, `rule:security/secret-qualifier` | on disk |
| PHP CS Fixer / PHP_CodeSniffer | layout and style | `nvs fmt`, canonical and unconfigurable — and the bug-adjacent rules are compile errors already | `rule:tooling/fmt-is-one-canonical-style`; `rule:core-api/identifier-casing`, [0094](../decisions/0094.md) | errors on disk; `nvs fmt` M10 |
| Rector | automated codebase rewrites | `nvs convert` for PHP→Novis; Novis→Novis declined, with a recorded reopen trigger | [0089](../decisions/0089.md); `rule:tooling/fmt-is-never-a-diagnostic` | M11 |
| Xdebug — step debugging | breakpoints, stepping | `nvs dap`, on the safepoints codegen already emits | `rule:ide/the-debug-adapter-does-not-wait-for-an-editor` | M10 |
| Xdebug — coverage, trace, profile | observing a run | `Core\Debug` over always-compiled probe sites; Clover/lcov/Callgrind out | `rule:testing/debug-probes`, `rule:testing/debug-surface` | probes on disk (M3); surface M10 |
| phpDocumentor / Doctum | API docs from source | none — the one tool category with no answer; see the gaps below | — | open |
| Deptrac / phpat / PHPStan custom rules | project-specific structural rules | a `#[Test]` walking `Core\Ast` / `Core\Reflect` / `Core\Program` | `rule:tooling/reflection-and-source-parsing-are-core-features`, `rule:testing/test-attribute` | blocked; see the gaps below |
| Faker | realistic fake data | out of scope — data, not tooling; `rule:testing/property-testing`'s generators cover the structural half | `rule:testing/property-testing` | — |
| PHPMD / phpmetrics | complexity and "mess" metrics | none, on purpose — the type system and `--mutate` keep the part that found bugs | `rule:types/declaration`, `rule:testing/mutation-testing` | — |
| Composer, for the dev-tools above | installing all of it | nothing to install — which is this whole page; packages proper are [0081](../decisions/0081.md) | — | — |

## Why testing needed no package

The testing rules (`rule:testing/test-attribute` and the rules beside it) carry the argument in full: PHPUnit's own
mechanism cannot port — mock generation is `eval` on generated source, and
Novis has no `eval` (`rule:security/no-eval`) — and everything it fakes at run time with reflection,
the compiler does at compile time with types. Those rules absorb the whole testing column above:
the framework, the parallel runner, the doubles, the data providers, the snapshots, the property
testing, the benchmark harness and the mutation tester are rules of that chapter, not ports of the
packages.

## Why static analysis needed no package

PHPStan and Psalm exist because PHP's own compiler checks almost nothing. Novis's checker *is* the
strict configuration: every binding typed (`rule:types/declaration`),
definite property initialization (`rule:classes/definite-property-initialization`), `lateinit`
read-before-write (`rule:classes/lateinit-read-before-write`), member existence and
every-path-returns at compile time, literal regex/format/URI validation
(`rule:expressions/intrinsic-constant-arguments`), the route table checked while compiling
(`rule:routing/routes-are-compiled-not-registered`), and taint/secret flow as type qualifiers
(`rule:security/tainted-qualifier`,
`rule:security/secret-qualifier`). Levels and baseline files have no
analogue because they exist to bolt gradual strictness onto untyped code, and there is no untyped
code. CI reads `nvs check --json`
([0108](../decisions/0108.md)
§ 6, M10).

## Why style needed no package

One axis, two ends, both decided: what can hide a bug is a hard compile error with no suppression —
casing (`rule:core-api/identifier-casing`), written visibility
([0094](../decisions/0094.md)), one spelling per construct
throughout `docs/divergences.md` — and what is purely layout is `nvs fmt`
(`rule:tooling/fmt-is-one-canonical-style`): canonical, zero-configuration, never wired into the
compiler. The entire configure-a-ruleset category (`.php-cs-fixer.php`, sniff selection) is designed
out rather than replaced.

## The three gaps, and what was decided about each

1. **No API documentation generator.** phpDocumentor's job has no owner anywhere in the tree.
   Decided: a note in [docs/future-ideas.md](../future-ideas.md), no milestone; it gets an ADR when
   it is scheduled.
2. **Architecture rules cannot point yet.** The `#[Test]`-over-`Core\Ast` story is real, but a node
   carries no span and no text, so a failing rule cannot name a `file:line` —
   [crates/nvs-stdlib/src/ast.rs](../../crates/nvs-stdlib/src/ast.rs)'s known gap 2, which is that
   fact's one home and now names this use case as what it blocks.
3. **No `nvs fix` for Novis-to-Novis upgrades.** Declined in
   `rule:tooling/fmt-is-never-a-diagnostic` while nobody needs it. Decided: the reopen trigger
   is recorded in [docs/future-ideas.md](../future-ideas.md) — the first post-1.0 breaking surface
   change ships with an automated rewrite, as part of that change's cost.
