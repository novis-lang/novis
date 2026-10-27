---
looper: 1
title: "A route is named by its method, and a method reference needs no `(...)`"
summary: "Class::method without (...) is a method reference, #[Route] has no name field, every link, signed link, match check and operationId names the route by its method, the editor follows those links, every example and page is rewritten, the changed rules are shipped, and the checks below pass."
context: [docs/rules/routing.md, crates/nvs-types/src/routes.rs, crates/nvs-types/src/links.rs, crates/nvs-stdlib/src/router.rs]
stages:
  - id: 1
    title: the rules say a route is named by its method, and a method reference needs no `(...)`
    status: done
    context: [docs/rules/expressions/method-reference.md, docs/rules/routing/route-attribute.md, docs/rules/routing/link-target-and-params-are-checked.md, docs/rules/core-classes/router-signed-url.md, docs/rules/observability/route-label-is-the-path-pattern.md]
    extra:
      context:
        rules: [expressions/method-reference, routing/route-attribute, routing/routes-are-compiled-not-registered, routing/a-method-is-one-endpoint-everywhere, routing/matching-is-not-dispatching, routing/a-match-is-not-invocable, routing/link-target-and-params-are-checked, routing/link-carries-the-mount-prefix, routing/api-document-is-generated-from-the-route-table, core-classes/router-signed-url, observability/route-label-is-the-path-pattern, types/callable-values]
  - id: 2
    title: "`Class::method` without `(...)` is a method reference everywhere"
    status: open
    context: [crates/nvs-syntax/src/parser/expr.rs, crates/nvs-syntax/src/ast.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-hir/src/members.rs]
    extra:
      context:
        rules: [expressions/method-reference, types/callable-values]
  - id: 3
    title: "`#[Route]` has no `name`, and a link names its route by a method reference"
    status: open
    context: [crates/nvs-types/src/routes.rs, crates/nvs-types/src/links.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-types/tests/routes.rs]
    extra:
      context:
        rules: [routing/route-attribute, routing/link-target-and-params-are-checked, routing/routes-are-compiled-not-registered]
  - id: 4
    title: "`Match::is`, signed links, the `operationId` and the metrics label read the method or the path"
    status: open
    context: [crates/nvs-stdlib/src/router.rs, crates/nvs-cli/src/openapi.rs, crates/nvs-types/src/routes.rs]
    extra:
      context:
        rules: [routing/a-match-is-not-invocable, core-classes/router-signed-url, routing/api-document-is-generated-from-the-route-table, observability/route-label-is-the-path-pattern]
  - id: 5
    title: the editor follows a link to its method
    status: open
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/arguments.rs, crates/nvs-lsp/src/definition.rs]
    extra:
      context:
        rules: [ide/completion-offers-only-what-the-compiler-derived]
  - id: 6
    title: every example, page and proof writes routes and links the new way, and the rules are shipped
    status: open
    context: [docs/examples/core/Router, docs/examples/core/Router-Match, docs/examples/lang/attributes/core-route-and-core-access-the-route-table, website/snippets]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: router-proofs
    stage: 6
    name: the router features and method references owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "Core\\Router::url", "Core\\Router::urlAbsolute", "Core\\Router::urlSigned", "Core\\Router::signedRoute", "Core\\Router\\Match::is", "Core\\Request::route", "lang:attributes/core-route-and-core-access-the-route-table", "lang:expressions/calls"]
  - id: rules-check
    stage: 6
    name: no rule cites a removed rule or an unknown one
    run: [bun, nv, rules, --check]
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, routes-by-method]
    reach: true
    floor: false
---

# A route is named by its method, and a method reference needs no `(...)`

## Why

A route carries a `name` that copies its method: `name: 'App::show'` sits on `App::show`, in about 330
of the corpus's 365 `#[Route]`s. Every link goes through that string, so the editor cannot follow a link
to its handler, complete it, find its callers or rename it, and the reader learns a mapping the code
already has. The one argument the rules gave for the name (`rule:routing/route-attribute`'s Declined
"deriving `name` from the class and method") is about a *derived string*; a checked method reference
breaks on a rename as loudly as a mistyped name. So the route is named by its method. Writing that
reference as `App::show(...)` is boilerplate the casing rules make unneeded, so `Class::method` without
`(...)` becomes the method reference first. Novis has not shipped, so nothing old is kept.

## Standing decisions

The user settled the design in an interactive session on 2026-10-27. Each decision below is the
answer, not a question.

**A bare `Class::method` is a method reference.** `Shop::show`, `self::show` and `static::show`, with
no `(...)`, take a reference to the method. The casing rules make this unambiguous: a constant is
`SCREAMING_SNAKE_CASE` (`E0113`), a method `camelCase` (`E0111`), an enum case or a nested type
`PascalCase`, and a static property carries `$`. **`Class::method(...)` no longer compiles**, with a
fix that drops the `(...)`, so each receiver form has one way to write a reference. **`$obj->method(...)`
keeps its form**, because a property and a method may share a name and `$obj->name` is the property.
Why: the common form loses its boilerplate, and a reader sees the Java and Kotlin form. Fallback: if
the parser cannot tell a bare reference from a member access before the checker runs, the parser
produces one node and the checker decides by the casing and the class's members.

**A reference that is created and dropped is a compile error.** `Cache::flush;` as a statement is
refused with a fix to `Cache::flush()`. A reference where a `string` or another non-callable type is
expected is already a type error, and its message adds the same hint. Why: a forgotten call is the one
trap the bare form opens.

**`#[Route]` is `{path, method, slotted?}`, and a route is identified by its method.** The `name` field
is removed. Several `#[Route]`s on one method must share their `path` and differ by verb; two
different paths on one method do not compile, and the message suggests an optional segment `{x?}` or a
second method. `E0749` (duplicate name) and the rule that let one method's routes share a name
go away; the code is retired, not reused. Why: one identity per route, and no mapping to learn.

**A link names its route by a method reference.** `Router::url`, `urlAbsolute` and `urlSigned` take a
method reference as the first argument, and `$params` defaults to `[]`: `Router::url(App::show, ['id' =>
3])`. A reference written at the call is checked while compiling: a missing method, a method with no
`#[Route]`, a missing or unknown key and a value its conversion refuses (`E0772`) are errors at the
argument. A reference passed as a value, such as a menu entry, is resolved at runtime and throws when
the method has no route, as a computed name throws today. Inside a link an instance method needs no
object, and its visibility is not checked, because a link calls nothing. Why: every written link is
checked, and the editor sees a reference it already understands.

**`Match::name()` is replaced by `Match::is(<method reference>): bool`.** The compiler folds a written
reference to the row index; a reference passed as a value is compared by row at runtime. The match
still carries nothing invocable (`rule:routing/a-match-is-not-invocable`). Why: hand dispatch and
"current page" checks read the same identity a link writes.

**A signed link signs the route's path pattern and its values.** The pattern is the mount-relative
`#[Route]` `path`, such as `/unsubscribe/{reader}`. A remount and a rename of the method leave old links
valid; a changed path breaks them, which a changed URL does anyway. Rewrite
`rule:core-classes/router-signed-url` to say so.

**The OpenAPI `operationId` is `Class.method`**, with the lowercased verb appended when one method
serves several verbs, as the name rule does now. `#[Api]` gets an optional `operationId` field that
replaces it, for a public API whose clients must keep their function names. A method rename changes
the `operationId`, and `nvs api diff` reports it as breaking, which is its job.

**The metrics `route` label carries the route's path pattern**, such as `/users/{id}`
(`rule:observability/route-label-is-the-path-pattern`, renamed from the declared-name rule).
It is still a closed set known when the program is built, never the request's raw path, so it passes
`rule:security/metric-label-refuses-tainted` as the name did. Every matched route now has the label,
where an unnamed route had none. The verb stays in its own label if the series has one, and no label is
added. Why: a method rename keeps its dashboard series, and the pattern is what a reader of the
dashboard recognises.

**The editor follows a link.** Go-to-definition, find-references and rename work on the reference in
`url`, `urlAbsolute`, `urlSigned` and `is`, because it is an ordinary method reference. Completion in
those arguments offers the methods that carry a `#[Route]` first.

**Trade-offs.** Security: unchanged, and every written link is now checked while compiling, so the
runtime throw for a mistyped name goes away for written links. Latency: none; a reference folds to the
row index a name did. Simplicity: one route field and one mapping fewer, and the editor understands
links; the cost is a second way to write a method reference, one per receiver form. Memory: none.

**Rules.** Stage 1 rewrites the affected rule fragments to the design above and sets each to
`designed` with `"goal": "routes-by-method"`; the last code stage sets them `shipped` with the tests
that guard them. A removed rule's fragment and data file are deleted, and every citation of it is
rewritten (`bun nv rules --check`). The Declined entry on deriving a name is replaced by a Declined
entry on a `name` field, with the reasons above.

**Lanes.** Stages 1 to 4 run in order: the rules first, then the parser and checker, then the route
table and links, then the readers of the match. Stage 5 (editor) touches only `crates/nvs-lsp/` and
`tests/lsp/` and may run as a side goal once stage 4 lands. Stage 6's batches of `.nvs` files are
disjoint by folder and go to writing subagents side by side.

## Stages

### 1. the rules say a route is named by its method, and a method reference needs no `(...)`

**Does:** every rule this goal changes states the design in § *Standing decisions*, is `designed` with
`"goal": "routes-by-method"`, and no rule cites a removed one. No code changes.

- [x] `rule:expressions/method-reference`: a bare `Class::method`, `self::method` and `static::method` is the reference, `Class::method(...)` does not compile, `$obj->method(...)` keeps its form, and a dropped reference is an error; replace "the only spelling" with one form per receiver and say why the casing rules allow it. `docs/rules/expressions/method-reference.md:1`, `data/rules/expressions/method-reference.json:1`
- [x] `rule:types/callable-values` and every other fragment that writes `Class::method(...)` say the bare form; an `Explore` subagent lists them first (the grep `::[a-z]\w*\(\.\.\.\)` under `docs/rules/*/`). `docs/rules/types/callable-values.md:1`
- [x] `rule:routing/route-attribute`: `{path, method, slotted?}`; the Declined entry on deriving a name becomes one on a `name` field. `docs/rules/routing/route-attribute.md:1`
- [x] The rule that let one method's routes share a `name` is deleted, with its data file; `rule:routing/routes-are-compiled-not-registered` says several routes on one method share one path, and the two-paths error. `docs/rules/routing/routes-are-compiled-not-registered.md:1`
- [x] The shared-name endpoint rule is renamed `rule:routing/a-method-is-one-endpoint-everywhere`: one `url()` target, one `Match::is`, one `operationId` per verb. `docs/rules/routing/a-method-is-one-endpoint-everywhere.md:1`
- [x] The link rule is renamed `rule:routing/link-target-and-params-are-checked`: a written reference is checked while compiling, a reference passed as a value throws when the method has no route, `$params` defaults to `[]`, and visibility is not checked. `docs/rules/routing/link-target-and-params-are-checked.md:1`
- [x] `rule:routing/matching-is-not-dispatching` and `rule:routing/a-match-is-not-invocable`: `Match::is` replaces `Match::name`. `docs/rules/routing/matching-is-not-dispatching.md:1`, `docs/rules/routing/a-match-is-not-invocable.md:1`
- [x] `rule:core-classes/router-signed-url` signs the path pattern; `rule:routing/api-document-is-generated-from-the-route-table` derives the `operationId` and names the `#[Api]` override; the metrics-label rule is renamed `rule:observability/route-label-is-the-path-pattern`. `docs/rules/core-classes/router-signed-url.md:1`, `docs/rules/routing/api-document-is-generated-from-the-route-table.md:1`, `docs/rules/observability/route-label-is-the-path-pattern.md:1`
- [x] The other fragments that mention a route name are rewritten: `routing/link-carries-the-mount-prefix`, `routing/table-is-opt-in`, `routing/a-match-reads-an-attribute-as-data`, `routing/a-quick-fix-writes-a-derived-path`, `routing/a-security-scheme-is-declared-in-code`, `testing/in-process-request`, `security/metric-label-refuses-tainted`, `ide/completion-offers-only-what-the-compiler-derived`. `docs/rules/routing/link-carries-the-mount-prefix.md:1`
- [x] `bun nv rules --check` passes, and the chapter files and `docs/ground-rules.md` are regenerated as that command says. `docs/ground-rules.md:1`

### 2. `Class::method` without `(...)` is a method reference everywhere

**Does:** the parser and checker read `Shop::show` as a method reference wherever it is written, refuse
`Shop::show(...)` with a fix, and refuse a reference that is created and dropped.

**Split:** after the compiler change builds, an `Explore` subagent lists every `.nvs`, `.nvst`, `.lspt`
and `.md` file that writes `Class::method(...)`; the grep `::[a-z]\w*\(\.\.\.\)` found 57 `.nvs` files.
Writing subagents rewrite them by folder (`docs/examples/`, `tests/hostile/`, `tests/conformance/`,
`benches/`, `crates/*/tests/fixtures/`), side by side, none of them building.

- [ ] `ClassConstAccess` whose name is `camelCase` and resolves to a method of the class is a method reference everywhere, recorded as `ExprInfo::CallableRef` as the `(...)` form is now; the `Core\Test` special case in `note_method_ref_args` becomes the general rule. `crates/nvs-types/src/expr/members.rs:77`, `crates/nvs-types/src/expr/calls.rs:240`, `crates/nvs-types/src/expr/calls.rs:1498`
- [ ] The member walk resolves a bare method name with `MemberKind::Method` when no constant has it. `crates/nvs-hir/src/members.rs:1286`, `crates/nvs-hir/src/members.rs:1469`
- [ ] `Class::method(...)`, `self::method(...)` and `static::method(...)` are refused with a new code and a fix that drops the `(...)`; `$obj->method(...)` is unchanged. `crates/nvs-syntax/src/parser/expr.rs:1132`, `crates/nvs-types/src/expr/calls.rs:240`, `crates/nvs-diagnostics/src/code/types_e07.rs:1`
- [ ] A method reference as an expression statement is refused with a new code and the fix `Class::method()`; a reference where a non-callable type is expected adds the same hint to its message. `crates/nvs-types/src/expr/calls.rs:1`
- [ ] Conformance cases: a bare reference called through a variable, `self::` and `static::` references, the refused `(...)` form, and the dropped reference. `tests/conformance/core/a-bare-method-reference-is-a-callable.nvst:1`, `tests/conformance/reject/a-static-method-reference-with-dots-is-refused.nvst:1`, `tests/conformance/reject/a-dropped-method-reference-is-refused.nvst:1`
- [ ] The corpus rewrite in **Split**. `docs/examples/lang/expressions/calls/04-every-way-to-write-a-call.nvs:1`

### 3. `#[Route]` has no `name`, and a link names its route by a method reference

**Does:** the route table has no names, two paths on one method do not compile, and `url`,
`urlAbsolute` and `urlSigned` take a method reference that is checked while compiling.

- [ ] `name` leaves the `OPTIONS` roster and `struct Route`; a written `name:` is an unknown field. `crates/nvs-types/src/routes.rs:174`, `crates/nvs-types/src/routes.rs:390`, `crates/nvs-types/src/routes.rs:2246`
- [ ] `check_table`: two `#[Route]`s on one method with different paths are a new code with both sites and the `{x?}` or second-method hint; `E0749` is retired in `types_e07.rs`. `crates/nvs-types/src/routes.rs:3398`, `crates/nvs-diagnostics/src/code/types_e07.rs:963`
- [ ] The first parameter of `url`, `urlAbsolute` and `urlSigned` is `CoreTy::MethodRef`, and `$params` is optional with `[]`. `crates/nvs-stdlib/src/router.rs:314`, `crates/nvs-stdlib/src/router.rs:323`, `crates/nvs-stdlib/src/router.rs:332`
- [ ] `links.rs` resolves a written reference to its route row: a method with no `#[Route]` replaces `E0754`'s unknown name, and the key and `E0772` checks read the row as before. Inside a link the reference needs no object and skips the visibility check. `crates/nvs-types/src/links.rs:156`, `crates/nvs-types/src/links.rs:343`, `crates/nvs-types/src/links.rs:491`
- [ ] The runtime link functions take the row a written reference folds to, and resolve a reference passed as a value by its method, throwing `no_such_route` when it has none. `crates/nvs-stdlib/src/router.rs:1535`, `crates/nvs-stdlib/src/router.rs:1555`, `crates/nvs-stdlib/src/router.rs:1694`, `crates/nvs-stdlib/src/router.rs:1968`
- [ ] Tests in `routes.rs` rewritten from names to references, plus: two paths on one method, a method with no route, an instance method linked without an object, a private method linked from another class, and a reference passed as a value that throws. `crates/nvs-types/tests/routes.rs:525`, `crates/nvs-types/tests/routes.rs:864`, `crates/nvs-types/tests/routes.rs:909`
- [ ] The help text of the three members (`URL_DOC`, `URL_ABSOLUTE_DOC`, `URL_SIGNED_DOC`) and the class card, held to § *Text an end user reads*. `crates/nvs-stdlib/src/router.rs:288`, `crates/nvs-stdlib/src/router.rs:435`

### 4. `Match::is`, signed links, the `operationId` and the metrics label read the method or the path

**Does:** nothing reads a route name any more.

- [ ] `Match::name` is replaced by `Match::is(<method reference>): bool`: a written reference folds to the row index, a value compares by row. `crates/nvs-stdlib/src/router.rs:746`, `crates/nvs-stdlib/src/router.rs:752`, `crates/nvs-stdlib/src/router.rs:2348`
- [ ] `urlSigned` and `signedRoute` sign and verify the path pattern and the values. `crates/nvs-stdlib/src/router.rs:1694`, `crates/nvs-stdlib/src/router.rs:1889`
- [ ] `base_id` is `Class.method`, the verb suffix stays for a method with several verbs, and `#[Api]` gets an optional `operationId` row that replaces it. `crates/nvs-cli/src/openapi.rs:366`, `crates/nvs-cli/src/openapi.rs:424`, `crates/nvs-types/src/routes.rs:1218`
- [ ] The metrics `route` label carries the path pattern; an Explore subagent finds where the label is written (grep `route` label in `crates/nvs-runtime/` and `crates/nvs-stdlib/`). `crates/nvs-stdlib/src/router.rs:1`
- [ ] Tests: the stdlib test module's `Match::name` cases become `Match::is`, a signed link survives a method rename and a remount, the pinned OpenAPI document is re-blessed with the new ids, and an `#[Api]` override wins. `crates/nvs-stdlib/src/router.rs:3085`, `crates/nvs-cli/tests/openapi.rs:1`, `crates/nvs-cli/tests/fixtures/openapi/schema-3.1.json:1`

### 5. the editor follows a link to its method

**Does:** the editor treats the link's first argument as the method reference it is.

- [ ] Completion in the first argument of `url`, `urlAbsolute`, `urlSigned` and `Match::is` offers the methods that carry a `#[Route]`, replacing `route_names`; `route_keys` reads the route of the written reference. `crates/nvs-lsp/src/completion.rs:747`, `crates/nvs-lsp/src/completion.rs:1228`, `crates/nvs-lsp/src/completion.rs:1265`
- [ ] `route_name_at` and the `RouteLink` go-to-definition read the reference; find-references and rename on a route method include its links. `crates/nvs-lsp/src/arguments.rs:151`, `crates/nvs-lsp/src/definition.rs:899`
- [ ] Tests: the completion test and the `.lspt` case rewritten, plus a find-references case over a link. `crates/nvs-lsp/tests/completion_edits.rs:213`, `tests/lsp/definition/a-link-to-a-declared-route-answers-its-stub.lspt:1`

### 6. every example, page and proof writes routes and links the new way, and the rules are shipped

**Does:** no file in the tree writes a route `name:`, a link by name or `Match::name`, and every
feature this goal changed has its proofs.

**Split:** an `Explore` subagent lists every file that writes `name:` inside `#[Route(`,
`Router::url(` or `urlAbsolute(` or `urlSigned(` with a string, or `->name()` on a match. The grep found
about 330 route lines across `docs/examples/`, `tests/conformance/`, `tests/hostile/`, `benches/`,
`crates/nvs-cli/tests/fixtures/`, `website/snippets/`, `examples/` and `apps/`. Writing subagents take
one folder each, side by side, none of them building; each `.nvs` they touch is held to § *Text an end
user reads*. A link that loops over a list of names becomes a list of method references.

- [ ] The corpus rewrite in **Split**, including `apps/notes/src/App.nvs` and `apps/notes/src/Pages.nvs`. `apps/notes/src/App.nvs:18`, `apps/notes/src/Pages.nvs:24`
- [ ] The `Match::name` feature becomes `Match::is`: its examples, bench and attack move to `Router-Match/is/`, and the menu example marks the open page with `is`. `docs/examples/core/Router-Match/name/03-call-the-method-for-each-route.nvs:1`, `docs/examples/core/Request/route/03-marking-the-current-page-in-a-menu.nvs:1`, `benches/members/core/Router-Match/name.nvs:1`, `tests/hostile/core/Router-Match/name/01-names-read-from-many-matches.nvs:1`
- [ ] `about.md` of the route table, `url`, `urlAbsolute`, `urlSigned`, `signedRoute` and `lang:expressions/calls`, and the reference chapters that show a route or a `(...)` reference. `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md:1`, `docs/reference/lang/90-attributes.md:1`
- [ ] `bun nv proofs --id` owes nothing for each feature in the `router-proofs` check. `docs/examples/core/Router/url/03-the-links-of-a-menu.nvs:1`
- [ ] Every rule stage 1 set `designed` is `shipped` with the tests that guard it, and `bun nv owners --closes routes-by-method` passes. `data/rules/routing/route-attribute.json:1`

### 7. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stage 1 is done: the rules state the design, and the code still writes the old one. Every rule this
goal changes is `designed` with `"goal": "routes-by-method"` (30 records, `grep -l routes-by-method
data/rules/*/*.json`). Three rules were renamed, `a-method-is-one-endpoint-everywhere`,
`link-target-and-params-are-checked` and `route-label-is-the-path-pattern`, and every citation of the
old ids in code, tests and docs was rewritten by id. The shared-name rule was deleted, and its
citations in `crates/nvs-types/src/routes.rs` and `crates/nvs-types/tests/routes.rs` now point at
`rule:routing/routes-are-compiled-not-registered`. Their comments still describe a shared `name`, and
stage 3 rewrites them with the code. `bun nv rules --check` is clean, and the chapters are rendered.

Two choices the rules settle beyond the standing decisions: `#[Api]`'s `operationId` takes the verb
suffix like the derived id when a method serves several verbs, and a duplicate `operationId` is
`E0771` (`docs/rules/attributes/api-adds-and-cannot-contradict.md:20`). The spawn, upgrade, worker,
structural-retrieval, `static` and enum rules now write the bare `Class::method` form, so stage 2's
checker change covers their entry operands and `Core\Attributes::get<T>(Class::method)` too.
`parent::constructor(...)`, `Category::find(...)` and `$cls::f(...)` in the rules are calls with
elided arguments, not references, and stay.

No tests are expected red: the only code edits are rule-id citations in comments.

### Next

**Stage 2: the bare method reference**, files `crates/nvs-syntax/src/parser/expr.rs`,
`crates/nvs-types/src/expr/calls.rs`, `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-hir/src/members.rs`, `crates/nvs-diagnostics/src/code/types_e07.rs`.

- [ ] A `camelCase` `ClassConstAccess` that resolves to a method is a method reference, recorded as `ExprInfo::CallableRef`; `note_method_ref_args`' `Core\Test` case becomes the general rule. `crates/nvs-types/src/expr/members.rs:77`, `crates/nvs-types/src/expr/calls.rs:225`
- [ ] The member walk resolves a bare method name when no constant has it. `crates/nvs-hir/src/members.rs:1286`
- [ ] `Class::method(...)`, `self::method(...)` and `static::method(...)` are refused with a new code and a fix that drops `(...)`. `crates/nvs-types/src/expr/calls.rs:240`
- [ ] A dropped reference as a statement is refused with the fix `Class::method()`, and the non-callable type error adds the hint. `crates/nvs-types/src/expr/calls.rs:1`
- [ ] The three conformance cases, then the corpus rewrite in stage 2's **Split**, which also covers the `spawn script`, `Socket::upgrade`, `on worker` and `Core\Attributes::get` operands. `tests/conformance/reject/a-dropped-method-reference-is-refused.nvst:1`

### Backlog
