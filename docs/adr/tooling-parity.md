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
| PHPUnit | declaring, running and asserting tests | `#[Test]` + `Core\Test` + `nvs test` | [0079](../decisions/0079.md) §§ 1, 4, 22 | M4S tail — on disk |
| Pest | a nicer test-declaration syntax | nothing — one surface, no base class to improve on | [0079](../decisions/0079.md) § 1 | — |
| Paratest | parallel test execution | every test is its own isolate; parallel is the default, not a flag | [0079](../decisions/0079.md) § 2 | M5 |
| Mockery / Prophecy / PHPUnit mocks | test doubles | a shape literal of closures, structurally checked against the interface | [0079](../decisions/0079.md) §§ 10–11 | M4S or M5 (§ 24) |
| DAMA DoctrineTestBundle / dbunit | database-test isolation | `#[Test(db:)]` is a transaction the runner rolls back | [0079](../decisions/0079.md) § 17 | M8 |
| WebTestCase / BrowserKit | HTTP-level tests | in-process dispatch through the compiled route table | [0079](../decisions/0079.md) § 18 | M8 |
| phpunit-snapshot-assertions | snapshot tests | inline snapshots; the updater writes into the test | [0079](../decisions/0079.md) § 14 | M8 |
| Eris | property-based testing | generators derived from the declared parameter types | [0079](../decisions/0079.md) § 13 | M4S or M5 (§ 24) |
| Infection | mutation testing | `nvs test --mutate` — type-valid mutants, run against the tests coverage says touch them | [0079](../decisions/0079.md) § 21 | M10 |
| PHPBench | benchmarking | `#[Bench]` reports the deterministic counter stream, so CI can gate on it | [0079](../decisions/0079.md) § 15 | M10 |
| PHPStan / Psalm / Phan | static analysis | `nvs check` — the compiler is the analyzer; see below | [0007](../decisions/0007.md) and the section below | on disk |
| Psalm `--taint-analysis` | taint analysis | `tainted` and `secret` are type qualifiers, enforced as hard errors | [0024](../decisions/0024.md), [0033](../decisions/0033.md) | on disk |
| PHP CS Fixer / PHP_CodeSniffer | layout and style | `nvs fmt`, canonical and unconfigurable — and the bug-adjacent rules are compile errors already | [0039](../decisions/0039.md); [0029](../decisions/0029.md), [0094](../decisions/0094.md) | errors on disk; `nvs fmt` M10 |
| Rector | automated codebase rewrites | `nvs convert` for PHP→Novis; Novis→Novis declined, with a recorded reopen trigger | [0089](../decisions/0089.md); [0039](../decisions/0039.md) § 9 | M11 |
| Xdebug — step debugging | breakpoints, stepping | `nvs dap`, on the safepoints codegen already emits | [0016](../decisions/0016.md) | M10 |
| Xdebug — coverage, trace, profile | observing a run | `Core\Debug` over always-compiled probe sites; Clover/lcov/Callgrind out | [0018](../decisions/0018.md) | probes on disk (M3); surface M10 |
| phpDocumentor / Doctum | API docs from source | none — the one tool category with no answer; see the gaps below | — | open |
| Deptrac / phpat / PHPStan custom rules | project-specific structural rules | a `#[Test]` walking `Core\Ast` / `Core\Reflect` / `Core\Program` | [0019](../decisions/0019.md), [0079](../decisions/0079.md) | blocked; see the gaps below |
| Faker | realistic fake data | out of scope — data, not tooling; § 13's generators cover the structural half | [0079](../decisions/0079.md) § 13 | — |
| PHPMD / phpmetrics | complexity and "mess" metrics | none, on purpose — the type system and `--mutate` keep the part that found bugs | [0007](../decisions/0007.md), [0079](../decisions/0079.md) § 21 | — |
| Composer, for the dev-tools above | installing all of it | nothing to install — which is this whole page; packages proper are [0081](../decisions/0081.md) | — | — |

## Why testing needed no package

[0079](../decisions/0079.md)'s *Context* is the argument in full: PHPUnit's own
mechanism cannot port — mock generation is `eval` on generated source, and
[0052](../decisions/0052.md) closed `eval` — and everything it fakes at run time with reflection,
the compiler does at compile time with types. That one ADR absorbs the whole testing column above:
the framework, the parallel runner, the doubles, the data providers, the snapshots, the property
testing, the benchmark harness and the mutation tester are sections of it, not ports of the
packages.

## Why static analysis needed no package

PHPStan and Psalm exist because PHP's own compiler checks almost nothing. Novis's checker *is* the
strict configuration: every binding typed with real generics ([0007](../decisions/0007.md)),
definite property initialization ([0022](../decisions/0022.md)), `lateinit`
read-before-write ([0038](../decisions/0038.md)), member existence and
every-path-returns at compile time, literal regex/format/URI validation
([0057](../decisions/0057.md)), the route table checked while compiling
([0077](../decisions/0077.md)), and taint/secret flow as type qualifiers
([0024](../decisions/0024.md),
[0033](../decisions/0033.md)). Levels and baseline files have no
analogue because they exist to bolt gradual strictness onto untyped code, and there is no untyped
code. CI reads `nvs check --json`
([0108](../decisions/0108.md)
§ 6, M10).

## Why style needed no package

One axis, two ends, both decided: what can hide a bug is a hard compile error with no suppression —
casing ([0029](../decisions/0029.md)), written visibility
([0094](../decisions/0094.md)), one spelling per construct
throughout `docs/divergences.md` — and what is purely layout is `nvs fmt`
([0039](../decisions/0039.md)): canonical, zero-configuration, never wired into the
compiler. The entire configure-a-ruleset category (`.php-cs-fixer.php`, sniff selection) is designed
out rather than replaced, which is that ADR's § 3.

## The three gaps, and what was decided about each

1. **No API documentation generator.** phpDocumentor's job has no owner anywhere in the tree.
   Decided: a note in [docs/future-ideas.md](../future-ideas.md), no milestone; it gets an ADR when
   it is scheduled.
2. **Architecture rules cannot point yet.** The `#[Test]`-over-`Core\Ast` story is real, but a node
   carries no span and no text, so a failing rule cannot name a `file:line` —
   [crates/nvs-stdlib/src/ast.rs](../../crates/nvs-stdlib/src/ast.rs)'s known gap 2, which is that
   fact's one home and now names this use case as what it blocks.
3. **No `nvs fix` for Novis-to-Novis upgrades.** Declined in
   [0039](../decisions/0039.md) § 9 while nobody needs it. Decided: the reopen trigger
   is recorded in [docs/future-ideas.md](../future-ideas.md) — the first post-1.0 breaking surface
   change ships with an automated rewrite, as part of that change's cost.
