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
    status: done
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

**The parser decides a bare reference by casing alone** (stage 2, decided by the session that built
it). A name after `::` that starts with a lowercase letter is a method, so the parser builds the same
`StaticCall { args: MethodRef }` node `(...)` built, and every later pass reads one shape. The
checker fallback the first decision names was not needed: the casing rules leave no lowercase
constant, enum case or nested type. The two new codes are parser codes, `E0136` for `(...)` on a
class and `E0137` for a dropped reference, because both are decided from the tree alone.
`Core\Test::assertCalled`'s reference still folds to the method's name and keeps `E0829`; it is marked
in its own set, `Env::method_name_args`, apart from a retrieval target. Why: one node is the simplest
change, and nothing downstream of the parser needed a new arm.

**A link's `$route` is a `callable`, and a written reference is marked for the link** (stage 3,
decided by the session that built it). `CoreTy::MethodRef` was not reused: it folds to the method's
name and refuses a value, and the design needs a value to work. So the three link members declare
`CoreTy::Callable`, and `nvs_types::expr::calls::note_link_target` marks a written `Class::method` in
`Env::link_target_args`, which skips the visibility check. A written reference is never lowered:
`links.rs` resolves it by `Route::handler` and `nvs-ir` passes the prepared path in its place, so an
instance method needs no object. `urlSigned` keeps `$params` required, because its settings after it
are required and a default cannot sit in front of a required parameter. `E0754` keeps its number and
now means a method with no route (`E_LINK_METHOD_HAS_NO_ROUTE`), because it is the same check at the
same argument. Why: the editor and the checker read an ordinary method reference, and a value reaches
the runtime as the callable it is.

**Two paths on one method are `E0891`, and a value-form reference carries its target as a class
constant** (stage 3, decided by the session that built the table half). The `E07xx` range is full at
`E0799`, so the new code is the next in `types_e08.rs`, `E_ROUTE_PATHS_ON_ONE_METHOD`, reported at the
second attribute with the first as a secondary label. `E0749` is retired with a comment. For the value
form, the method-reference thunk class (`anon_fn::thunk_class`) gets one private class constant named
`fn#target` whose value is the `Class::method` `Route::handler` writes, and the unfolded router
helpers read it from the callable's descriptor (`ClassDesc::constants`). An anonymous function's
class has no such constant, so it throws `no_such_route`. Why: one row per reference site per compiled
unit and nothing per request, where an object field would cost 16 bytes per evaluation of every method
reference, including the many that are never links. `#` cannot start a declared name, so nothing
collides.

**A value-form link makes the written link's checks at run time, and an instance-method reference
needs an object unless it is folded** (stage 3, decided by the session that built the value form).
The unfolded helpers throw for a `$params` key that is no capture and no `#[Query]` parameter, and
for a capture value its conversion does not match. The prepared helpers leave a computed value to
the router's `404`. Here the row is already in hand, so the check is one comparison per key. The
link is built from the runtime row's own parsed segments, so § 2's grammar keeps one runtime
reader. A reference such as `callable $f = Shop::show;` to an instance method with no `$this` is
`E0778` with a reference wording, because the callable has no object to store. Only a reference the
checker folds (a link target, an attribute retrieval, `assertCalled`'s method) is exempt. So a menu
of instance-method handlers writes `$shop->show(...)`, and a static handler writes `Shop::home`.
Why: the old exemption let every method reference through and `nvs-ir` panicked on the value form.

**`Match::is` folds a written reference to its method's label, not to a row index** (stage 4,
decided by the session that built it). One method with two verbs has two rows, and `is(Shop::show)`
must be `true` for both, so a single row index is the wrong identity. `nvs_types::links` records an
`is` site beside the link sites, asks the link's first question (a method with no route is
`E0754`, worded for `is`), and records `ExprInfo::RouteIs { handler }` over the call. `nvs-ir`
passes the receiver and the `Class::method` label to `nvs_core_router_match_is_route`, which
compares it with the matched row's handler, read through the match's row index from the unit's
table. A value goes to `nvs_core_router_match_is`, which finds its route as `value_route` does and
throws `no_such_route` for a method with none. The `name` slot leaves `Core\Router\Match`. Why:
one string comparison per call and nothing per request; the label never crosses into the program,
so the match stays not invocable.

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

- [x] `ClassConstAccess` whose name is `camelCase` and resolves to a method of the class is a method reference everywhere, recorded as `ExprInfo::CallableRef` as the `(...)` form is now; the `Core\Test` special case in `note_method_ref_args` becomes the general rule. `crates/nvs-types/src/expr/members.rs:77`, `crates/nvs-types/src/expr/calls.rs:240`, `crates/nvs-types/src/expr/calls.rs:1498`
- [x] The member walk resolves a bare method name with `MemberKind::Method` when no constant has it. `crates/nvs-hir/src/members.rs:1286`, `crates/nvs-hir/src/members.rs:1469`
- [x] `Class::method(...)`, `self::method(...)` and `static::method(...)` are refused with a new code and a fix that drops the `(...)`; `$obj->method(...)` is unchanged. `crates/nvs-syntax/src/parser/expr.rs:1132`, `crates/nvs-types/src/expr/calls.rs:240`, `crates/nvs-diagnostics/src/code/types_e07.rs:1`
- [x] A method reference as an expression statement is refused with a new code and the fix `Class::method()`; a reference where a non-callable type is expected adds the same hint to its message. `crates/nvs-types/src/expr/calls.rs:1`
- [x] Conformance cases: a bare reference called through a variable, `self::` and `static::` references, the refused `(...)` form, and the dropped reference. `tests/conformance/core/a-bare-method-reference-is-a-callable.nvst:1`, `tests/conformance/reject/a-static-method-reference-with-dots-is-refused.nvst:1`, `tests/conformance/reject/a-dropped-method-reference-is-refused.nvst:1`
- [x] The corpus rewrite in **Split**. `docs/examples/lang/expressions/calls/04-every-way-to-write-a-call.nvs:1`
  - [x] every `.nvs`, `.nvst` and Rust test source, and every diagnostic's help, writes the bare form
  - [x] every `.md` under `docs/` that shows a reference writes the bare form; a call with elided arguments keeps `(...)`
  - [x] every website page under `website/src/content/docs/` does too, and the generated `website/src/data/core.json` and `website/public/llms-full.txt` are written again

### 3. `#[Route]` has no `name`, and a link names its route by a method reference

**Does:** the route table has no names, two paths on one method do not compile, and `url`,
`urlAbsolute` and `urlSigned` take a method reference that is checked while compiling.

- [ ] `name` leaves the `OPTIONS` roster and `struct Route`; a written `name:` is an unknown field. `crates/nvs-types/src/routes.rs:174`, `crates/nvs-types/src/routes.rs:390`, `crates/nvs-types/src/routes.rs:2246`
- [x] `check_table`: two `#[Route]`s on one method with different paths are a new code with both sites and the `{x?}` or second-method hint; `E0749` is retired in `types_e07.rs`. `crates/nvs-types/src/routes.rs:3398`, `crates/nvs-diagnostics/src/code/types_e07.rs:963`
- [x] The first parameter of `url`, `urlAbsolute` and `urlSigned` is `$route`, a `CoreTy::Callable` (see § *Standing decisions*), and `$params` is optional with `[]` on `url` and `urlAbsolute`. `crates/nvs-stdlib/src/router.rs:314`, `crates/nvs-stdlib/src/router.rs:323`, `crates/nvs-stdlib/src/router.rs:332`
- [x] `links.rs` resolves a written reference to its route row: a method with no `#[Route]` replaces `E0754`'s unknown name, and the key and `E0772` checks read the row as before. Inside a link the reference needs no object and skips the visibility check. `crates/nvs-types/src/links.rs:156`, `crates/nvs-types/src/links.rs:343`, `crates/nvs-types/src/links.rs:491`
- [x] The runtime link functions take the row a written reference folds to, and resolve a reference passed as a value by its method, throwing `no_such_route` when it has none. `crates/nvs-stdlib/src/router.rs:1535`, `crates/nvs-stdlib/src/router.rs:1555`, `crates/nvs-stdlib/src/router.rs:1694`, `crates/nvs-stdlib/src/router.rs:1968`
  - [x] a written reference reaches the prepared-path helpers as before; an omitted `$params` lowers to an empty `ArrayNew` (`lower_route_link`)
  - [x] the method-reference thunk carries its target `Class::method` where the runtime can read it (`lower_method_ref`, `crates/nvs-ir/src/lower/expr.rs:3313`)
  - [x] the unfolded helpers `nvs_core_router_url`/`_absolute`/`_signed` find the row by `ctx.routes()` and `Route::handler`, build the link as the prepared helpers do, and throw `no_such_route` only for a method with no route
- [x] Tests in `routes.rs` rewritten from names to references, plus: two paths on one method, a method with no route, an instance method linked without an object, a private method linked from another class, and a reference passed as a value that throws. `crates/nvs-types/tests/routes.rs:525`, `crates/nvs-types/tests/routes.rs:864`, `crates/nvs-types/tests/routes.rs:909`
  - [x] the link tests use references; a method with no route, an instance method without an object, a private method from outside, a value left to run time, an omitted `$params` (`a_link_to_a_method_with_no_route_is_a_diagnostic` and the three after it)
  - [x] two paths on one method, with the `check_table` item (`one_method_with_two_paths_does_not_compile`)
  - [x] a reference passed as a value that throws, as a run-time test once the thunk carries its target (`router-url-finds-the-route-of-a-method-reference-passed-as-a-value.nvst`)
- [x] The help text of the three members (`URL_DOC`, `URL_ABSOLUTE_DOC`, `URL_SIGNED_DOC`) and the class card, held to § *Text an end user reads*. `crates/nvs-stdlib/src/router.rs:288`, `crates/nvs-stdlib/src/router.rs:435`

### 4. `Match::is`, signed links, the `operationId` and the metrics label read the method or the path

**Does:** nothing reads a route name any more.

- [x] `Match::name` is replaced by `Match::is(<method reference>): bool`: a written reference folds to the method's label (see § *Standing decisions*), a value compares by handler. `crates/nvs-stdlib/src/router.rs:746`, `crates/nvs-stdlib/src/router.rs:752`, `crates/nvs-stdlib/src/router.rs:2348`
  - [x] the registry row, its card and the two helpers; the `name` slot leaves the match
  - [x] `links.rs` records and resolves an `is` site, and `nvs-ir` lowers `ExprInfo::RouteIs`
  - [x] a conformance case and a checker test (`router-match-is-compares-the-route-with-a-method-reference.nvst`, `match_is_over_a_method_with_no_route_is_a_diagnostic`)
- [x] `urlSigned` and `signedRoute` sign and verify the path pattern and the values. `crates/nvs-stdlib/src/router.rs:1694`, `crates/nvs-stdlib/src/router.rs:1889`
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

Stages 1 and 2 are done; stage 3 lacks only the `name` removal. The value form landed
(`03ac235e7`): a forwarding reference's class carries the private constant `fn#target`, and the
unfolded router helpers find the row by it (`value_route`, `check_value_params`, `value_template`).
`Core\Reflect` skips the constant. An instance-method reference made into a callable with no `$this`
is now `E0778` instead of an `nvs-ir` panic; only folded references are exempt
(`crates/nvs-types/src/expr/calls.rs:633`). A corpus file that does this now fails, and stage 7
names any. The sweep's Miri and `env-secret-proofs` reds were stale (fixed in `cea10a7cd`).

`Route::name`, `RouteTable::named` and the `name:` roster row still exist, and `row.name` is
still read by `crates/nvs-cli/src/openapi.rs:425`, `crates/nvs-cli/src/main.rs:1562`,
`crates/nvs-lsp/src/completion.rs:1239`/`1334` and `Match::name` (`router.rs:858`, `:2500`).

Expected red until stage 6 and 7: every `.nvs`, `.nvst`, `.lspt` and Rust test source that calls
`Core\Router::url`/`urlAbsolute`/`urlSigned` with a quoted route name, including the router tests in
`crates/nvs-stdlib/src/router.rs` (`// covers: Core\Router::url` near line 3801), the conformance
cases under `tests/conformance/`, the LSP route-name completion tests and the examples. Also the Rust
test files the last handoff named (`crates/nvs-types/tests/attributes.rs`, `isolates.rs`,
`core_members.rs`, `paths.rs`, `crates/nvs-ir/tests/callable_names.rs`, `crates/nvs-lsp/tests/index.rs`,
`navigation.rs`, `crates/nvs-cli/tests/spawn_entry.rs`), which were not run. `crates/nvs-types/tests/routes.rs`
passes whole.

### Next

**Stage 4's readers first, then stage 3's last item.**

- [ ] `Match::is(<method reference>): bool` replaces `Match::name` (stage 4's first item). A written reference folds to the row index, like a link target: mark it in `Env::link_target_args` (or a sibling set) so `E0778` and visibility skip it (`crates/nvs-types/src/expr/calls.rs:572`). A value compares by handler, through `nvs_runtime::callable_target`, as `value_route` does (`crates/nvs-stdlib/src/router.rs`). `crates/nvs-stdlib/src/router.rs:858`, `crates/nvs-stdlib/src/router.rs:2500`
- [ ] The `operationId` item: `base_id` becomes `Class.method`, with the verb suffix for several verbs, and `#[Api]` gets an optional `operationId`. `crates/nvs-cli/src/openapi.rs:425`, `crates/nvs-cli/src/main.rs:1562`
- [ ] Then `name` leaves `OPTIONS`, `struct Route`, `RouteTable::named` and the runtime row (`Route::new`'s `name` argument, `crates/nvs-runtime/src/routes.rs:410`). `completion.rs:1239`/`1334` move in stage 5, or read the handler in the meantime. `crates/nvs-types/src/routes.rs:174`, `crates/nvs-types/src/routes.rs:390`

### Backlog
