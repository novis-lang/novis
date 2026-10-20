---
looper: 1
title: "A route method's return value reaches the response through the program's own `respond`"
summary: "Core\\Router\\Responder extends Core\\Router\\Gate with respond(), Core\\Router::dispatch hands a route method's returned value to it when the gate implements it, nvs check warns at a dispatch call whose gate cannot be a Responder while a route method returns a value, Core\\Router\\Responder has its feature proofs, the two routing rules say so, and the checks below pass."
context: [crates/nvs-stdlib/src/router.rs, docs/rules/routing/a-gate-answers-around-every-handler.md]
stages:
  - id: 1
    title: "`Core\\Router\\Responder` is a contract, and `dispatch` hands a returned value to `respond`"
    status: done
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-types/src/core_lib.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-ir/src/ir.rs, crates/nvs-codegen/src/lib.rs, docs/rules/routing/matching-is-not-dispatching.md, tests/conformance/core/router-dispatch-discards-a-handlers-return-value.nvst]
    extra:
      context:
        rules: [routing/a-gate-answers-around-every-handler, routing/matching-is-not-dispatching, core-api/reference-card]
  - id: 2
    title: "`nvs check` warns at a `dispatch` whose gate cannot be a `Responder`"
    status: done
    context: [crates/nvs-types/src/routes.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/check.rs, crates/nvs-diagnostics/src/code/warnings.rs, crates/nvs-types/tests/routes.rs]
    extra:
      context:
        rules: [routing/a-gate-answers-around-every-handler, routing/api-document-is-generated-from-the-route-table]
  - id: 3
    title: "`Core\\Router\\Responder` has its feature proofs, and the pages say what `dispatch` does with a value"
    status: open
    context: [docs/examples/types/Router-Gate/about.md, docs/examples/core/Router/dispatch/about.md, website/src/content/docs/syntax/attributes/routes.mdx, docs/rules/testing/feature-proofs.md, data/rules/routing/a-gate-answers-around-every-handler.json]
    extra:
      context:
        rules: [testing/feature-proofs, testing/hostile-case-contract, routing/a-gate-answers-around-every-handler]
checks:
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, router-responder]
    reach: true
    floor: false
---

# A route method's return value reaches the response through the program's own `respond`

## Why

A route method may declare any return type, and `Core\Router::dispatch` discards the value
(`rule:routing/a-gate-answers-around-every-handler`, `crates/nvs-stdlib/src/router.rs:2717`). So a page
written as a function of its markup is silently empty:

```nvs
#[Core\Route(path: "/", method: Core\Http\Method::Get, name: "home")]
#[Core\Access(allow: Core\Audience::Public)]
public function render(): Core\Html\Markup { return html`<p>home</p>`; }
```

Under `dispatch` this answers `200` with an empty body, and `nvs check` says nothing. The return type
already feeds `nvs build --openapi` (`rule:routing/api-document-is-generated-from-the-route-table`), so
the type a program declares and what it sends disagree with no diagnostic. This goal lets the program's
own gate turn the value into the response, and warns where a value has nowhere to go.

## Standing decisions

The user settled the design. Each paragraph is an instruction, not a question.

**The interface.** `Core\Router\Responder extends Core\Router\Gate` with one member,
`respond(Core\Router\Match $route, mixed $value): void`. It is a second `CoreContract` beside
`crate::router::GATE` (`crates/nvs-stdlib/src/router.rs:909`), declared in `router.rs` with its class card
and its member card, and added to `CONTRACTS` (`crates/nvs-stdlib/src/registry.rs:2757`).

**`CoreContract` has no `extends` today, and gets one.** The struct is a name, a card and members
(`crates/nvs-stdlib/src/registry.rs:2730`). Add `parents: &'static [&'static str]`, empty for `GATE` and
`&[GATE_NAME]` for `RESPONDER`. Three readers take it: `nvs_types::core_lib` seeds the contract's class
and then one interface edge per parent, through the edge a program's `interface A extends B` gets
(`crates/nvs-types/src/core_lib.rs:179`, `crates/nvs-types/src/signatures.rs:821`), so conformance requires
`allows`, `refused`, `failed` and `respond` of an implementing class and a `Responder` is accepted where a
`Gate` is; `nvs meta`'s `contract_json` writes the parents (`crates/nvs-cli/src/meta.rs:298`); the
registry's shape tests over `CONTRACTS` check each parent names a contract
(`crates/nvs-stdlib/src/registry.rs:5921`). `Responder`'s own `members` list only `respond`. Safe
default if the edge cannot be seeded that way: repeat the three `Gate` members in `RESPONDER.members`,
and say so in the contract's doc.

**`dispatch` stays one function.** After the route method returns, and only when its declared return
type is neither `void` nor `never` and the gate implements `Responder`, `dispatch` calls
`$gate->respond($route, $value)` and then releases the value. A method declared `?T` that returns `null`
still reaches `respond` with `null`. The gate writes the response, for example `Core\Response::html` for
a `Core\Html\Markup`. An error `respond` throws is thrown by `dispatch`, as for `refused` and `failed`.
A route method that throws goes to `failed`, and `respond` is not called. `allows` still runs first, and
a refused request never reaches the route method or `respond`. A gate that is not a `Responder` drops
the value as today.

**How `dispatch` knows both facts at run time.** The route row carries a `returns_value` bit beside
`static_handler`: the checker's row (`crates/nvs-types/src/routes.rs:373`) sets it from the method's
resolved return type, the crossing copies it (`crates/nvs-cli/src/main.rs:1403`), and
`nvs_runtime::routes::Route` holds it (`crates/nvs-runtime/src/routes.rs:357`). Whether the gate is a
`Responder` is a bit on its class descriptor, carried the way `is_callable` is
(`crates/nvs-ir/src/ir.rs:410`, `crates/nvs-ir/src/lower/mod.rs:966`, `crates/nvs-codegen/src/lib.rs:1639`,
`crates/nvs-runtime/src/object.rs:1451`), set when the class's `conforms` names
`Core\Router\Responder`. `ClassDesc::conforms_to_name` cannot answer it, because a `Core` contract has no
descriptor and codegen skips its label (`crates/nvs-codegen/src/lib.rs:1621`). A class that declares a
`respond` method without implementing `Responder` is not called: the name alone never decides it.

**The warning.** At a `dispatch` call whose gate cannot be a `Responder`, while the program has a route
method whose declared return type is neither `void` nor `never`, `nvs check` gives a warning (the next
free `W` code in `crates/nvs-diagnostics/src/code/warnings.rs`, after `W1024` at line 248). It names up to
three such route methods in load order, then how many more, and its help names `Core\Router\Responder`.
It is a warning, never an error. The fix it names is to implement `Responder`. A program that wants
the value dropped writes a `respond` that does nothing, or declares the method `void`.

**"Cannot be a `Responder`" when the gate's static type is not exact.** The warning fires only when no
class in the program is both admitted by the argument's static type and a `Responder`. For a class type
that is the class and its subclasses; for `Core\Router\Gate` or another interface, every implementing
class; for `mixed` or an erased type, every class. So a program that has any `Responder` the argument
could hold gets no warning, and a warning is never wrong about the program as written. This costs a
missed warning in a program that has a `Responder` and passes a plain gate through a wide type, which
is the safe side for an advisory.

**Order in the checker.** A `dispatch` call can be checked before every route row is collected. So the
call site records its span and that one bit on the `RouteTable` (`crates/nvs-types/src/expr/calls.rs:640`,
`crates/nvs-types/src/routes.rs:669`), and the warning is raised once the table is complete, beside
`check_table` (`crates/nvs-types/src/check.rs:240`). The class graph is complete before bodies are
checked, so the bit is computed at the call.

**The OpenAPI document is unchanged.** Return types keep feeding `nvs build --openapi`
(`rule:routing/api-document-is-generated-from-the-route-table`). No code in `crates/nvs-cli/src/openapi.rs`
changes.

**The rules.** `rule:routing/a-gate-answers-around-every-handler`'s sentence "A handler may declare any
return type, and `dispatch` discards the value"
(`docs/rules/routing/a-gate-answers-around-every-handler.md:15`) and its Declined entry "A
return-value-to-response rule and constructor injection" (line 81) are rewritten: a return value reaches
the response only through the program's own `respond`, so no framework opinion is built in, and
constructor injection stays declined. `rule:routing/matching-is-not-dispatching`'s Declined "A
return-value-to-response rule" (`docs/rules/routing/matching-is-not-dispatching.md:45`) gets the same
rewrite. A rule fragment is always currently true, so the `dispatch` half lands with stage 1 and the
warning's sentence with stage 2. `bun nv rules --render` rewrites `docs/rules/routing.md`; never edit it.

**Trade-offs, recorded in the rule.** Latency: one extra method call, only for a request whose route
method returns a value and whose gate is a `Responder`; a `void` method or a plain gate pays one bit test.
Memory: none per request, and one bit per route row and per class descriptor per compiled unit.
Usability: a page is a function that returns its markup, and a test calls it as one. Simplicity: one
more interface and one warning. Security: unchanged, because `allows` still runs before the method.

**The feature id is `Core\Router\Responder`.** `bun nv proofs --id 'Core\Router\Responder'` names
nothing until stage 1 registers the contract; from then on it is the judge of what stage 3 owes. Its
proofs live where `Core\Router\Gate`'s do: `docs/examples/types/Router-Responder/`,
`benches/members/types/Router-Responder.nvs` and `tests/hostile/types/Router-Responder/`. If it names
other paths, its paths win.

**Lanes.** One lane, in order. Stage 2 needs stage 1's contract in the type table, and stage 3's
examples print what stages 1 and 2 do. Stages 1 and 2 may run as one group if the context allows.

## Stages

### 1. `Core\Router\Responder` is a contract, and `dispatch` hands a returned value to `respond`

**Does:** a gate that implements `Responder` receives the value of every route method that returns
one, and its `respond` writes the response. The motivating page answers with its markup.

- [x] `CoreContract` gains `parents`; `GATE` sets it empty (`rule:routing/a-gate-answers-around-every-handler`). `crates/nvs-stdlib/src/registry.rs:2730`, `crates/nvs-stdlib/src/router.rs:909`
- [x] `RESPONDER` with `respond`, its class card and member card in the plain voice, added to `CONTRACTS`, and the registry's contract tests cover a parent (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/router.rs:945`, `crates/nvs-stdlib/src/registry.rs:2757`, `crates/nvs-stdlib/src/registry.rs:5921`
- [x] `core_lib` seeds each parent edge, and `nvs meta` writes the parents. `crates/nvs-types/src/core_lib.rs:179`, `crates/nvs-types/src/signatures.rs:821`, `crates/nvs-cli/src/meta.rs:298`
- [x] The route row's `returns_value` bit, from the checker's row through the crossing to the runtime row. `crates/nvs-types/src/routes.rs:373`, `crates/nvs-cli/src/main.rs:1403`, `crates/nvs-runtime/src/routes.rs:477`
- [x] The descriptor's `Responder` bit, carried as `is_callable` is. `crates/nvs-ir/src/ir.rs:410`, `crates/nvs-ir/src/lower/mod.rs:966`, `crates/nvs-codegen/src/lib.rs:1639`, `crates/nvs-runtime/src/object.rs:2179`
- [x] `call_handler` hands the value back for a `returns_value` row, and `dispatch_matched` calls `respond` when the gate's descriptor has the bit, then releases the value; `dispatch`'s doc says the new step in its order. `crates/nvs-stdlib/src/router.rs:2717`, `crates/nvs-stdlib/src/router.rs:2601`, `crates/nvs-stdlib/src/router.rs:2483`
- [x] `DISPATCH_DOC`'s `ret` says the value goes to `respond` when the gate is a `Responder`. `crates/nvs-stdlib/src/router.rs:522`
- [x] Rust tests beside the router's: a `Responder` gets the value, a `void` method does not call `respond`, a throwing method calls `failed` and not `respond`, a throwing `respond` is thrown by `dispatch`, a plain gate drops the value. `crates/nvs-stdlib/src/router.rs:4100`
- [x] A conformance case: a returned `Core\Html\Markup` reaches the body through `respond`, with the status `200`. `tests/conformance/core/router-dispatch-hands-a-returned-value-to-respond.nvst:1`
- [x] The discard case stays the plain-gate half; if its expectation pins check output, it gains stage 2's warning there. `tests/conformance/core/router-dispatch-discards-a-handlers-return-value.nvst:1`
- [x] The two rule passages rewritten, with the trade-offs, and the new case in `guardedBy`. `docs/rules/routing/a-gate-answers-around-every-handler.md:15`, `docs/rules/routing/a-gate-answers-around-every-handler.md:81`, `docs/rules/routing/matching-is-not-dispatching.md:45`, `data/rules/routing/a-gate-answers-around-every-handler.json:13`

### 2. `nvs check` warns at a `dispatch` whose gate cannot be a `Responder`

**Does:** the motivating program, with a plain gate, gets one warning at its `dispatch` call that names
`render` and `Core\Router\Responder`. A program with a `Responder`, or with only `void` route methods,
gets none.

**Split:** an `Explore` subagent first lists every program under `docs/examples/`, `tests/`,
`benches/members/` and `website/snippets/` that calls `Core\Router::dispatch` with a plain gate and has
a route method that returns a value, as `file:line` one each. The session fixes each listed program
after the warning lands: a method that writes its own response is declared `void`, and one whose value
is the response gets a `Responder` gate.

- [x] The `RouteTable` records each `dispatch` site's span and whether its gate can be a `Responder`. `crates/nvs-types/src/routes.rs:655`, `crates/nvs-types/src/routes.rs:669`, `crates/nvs-types/src/expr/calls.rs:640`
- [x] The warning's code and card in the plain voice, raised beside `check_table`, naming up to three route methods. `crates/nvs-diagnostics/src/code/warnings.rs:248`, `crates/nvs-types/src/check.rs:240`
- [x] Rust tests: a plain gate warns, a `Responder` does not, `void` methods do not, a `Core\Router\Gate`-typed argument with a `Responder` in the program does not. `crates/nvs-types/tests/routes.rs:1`
- [x] The warning on the motivating program, pinned through the binary by `nvs check`. A conformance case pins stderr only for a run that fails, so this is a CLI test, as `W1023`'s is. `crates/nvs-cli/tests/check.rs:257`
- [x] Every program the `Explore` list names is fixed, so no published program prints the warning. The bench's route methods are `void`; the discard case keeps its plain gate, because it pins that half. `benches/members/core/Router/dispatch.nvs:9`
- [x] The rule's sentence on the warning, and the warning's test in `guardedBy`. `docs/rules/routing/a-gate-answers-around-every-handler.md:29`, `data/rules/routing/a-gate-answers-around-every-handler.json:13`

### 3. `Core\Router\Responder` has its feature proofs, and the pages say what `dispatch` does with a value

**Does:** `bun nv proofs --id 'Core\Router\Responder'` prints nothing owed, and every page that says
`dispatch` drops a returned value says what it does now.

**Split:** after stages 1 and 2 land, writing subagents write, one after another, the three examples
with their `.out`, then the bench, then the attack. The session checks each and writes the `about.md`
files and the website page itself.

- [ ] `about.md` for the feature, held to AGENTS.md § *Text an end user reads*. `docs/examples/types/Router-Responder/about.md:1`
- [ ] Example 01: a page method returns its markup, and `respond` sends it with `Core\Response::html`. `docs/examples/types/Router-Responder/01-a-page-returns-its-markup.nvs:1`
- [ ] Example 02: an API method returns an object, and `respond` sends it with `Core\Response::json`. `docs/examples/types/Router-Responder/02-an-api-method-returns-an-object.nvs:1`
- [ ] Example 03: one `respond` picks the response by the value's type, and a `void` method still writes its own. `docs/examples/types/Router-Responder/03-one-respond-for-pages-and-data.nvs:1`
- [ ] The bench: pages answered through `respond`, with its growth over the body size. `benches/members/types/Router-Responder.nvs:1`
- [ ] The attack: a denied request, a method that throws after it built its markup, and a `respond` that throws; none of the value reaches the body, and the last error is thrown by `dispatch` (`rule:testing/hostile-case-contract`). `tests/hostile/types/Router-Responder/a-value-never-skips-allows-or-failed.nvs:1`
- [ ] The `dispatch` and `Gate` pages say a `Responder` receives the value. `docs/examples/core/Router/dispatch/about.md:12`, `docs/examples/types/Router-Gate/about.md:1`
- [ ] The website's routes page says the same. `website/src/content/docs/syntax/attributes/routes.mdx:34`
- [ ] `bun nv proofs --id 'Core\Router\Responder'` prints nothing owed, and its Rust tests are the ones stage 1 wrote. `crates/nvs-stdlib/src/router.rs:4100`

## Handoff

### State

Stages 1 and 2 are done. `W1025` (`W_DISPATCH_IGNORES_RETURNED_VALUE`,
`crates/nvs-diagnostics/src/code/warnings.rs`) is raised by `crate::routes::check_responders` beside
`check_table` (`crates/nvs-types/src/check.rs:241`). Each `dispatch` call records a `DispatchSite` on
the `RouteTable`: the gate argument's span, and `gate_can_respond`'s answer. That answer is
`nvs_hir::implementors` of `Core\Router\Responder` over the class graph, filtered by the argument's
static type; a type naming no class of the program (`Core\Router\Gate`, `mixed`) admits every one.
Pinned by four tests in `crates/nvs-types/tests/routes.rs` (after
`a_call_to_dispatch_marks_the_unit…`) and by `crates/nvs-cli/tests/check.rs`'s
`a_dispatch_whose_gate_ignores_a_returned_value_warns`. No conformance case: the harness pins stderr
only for a run that fails (`crates/nvs-test/src/run.rs:332`). The discard case still warns under
`nvs check` and still runs green. `website/` was rendered again for stage 1's contract, and the 14
`Core\Router` and `Core\Router\Match` perf records were recorded again on `router.rs`.

### Next

**Stage 3**, the feature proofs. Follow the stage's `**Split:**` line: writing subagents, one after
another, write the three examples with their `.out`, then the bench, then the attack.

- [ ] Proofs mirror `Core\Router\Gate`'s, `docs/examples/types/Router-Gate/about.md:1`. Run
  `bun nv proofs --id 'Core\Router\Responder'` first. Its paths win over the stage's.
- [ ] Three examples, none printing `W1025`, `crates/nvs-cli/tests/fixtures/check/dispatch-ignores-a-returned-value.nvs:1`
  being the program that does. Run `nvs check` on each.
- [ ] The bench and the attack. `benches/members/types/Router-Responder.nvs:1`,
  `tests/hostile/types/Router-Responder/a-value-never-skips-allows-or-failed.nvs:1`
- [ ] `about.md` and the pages: `docs/examples/core/Router/dispatch/about.md:12`,
  `docs/examples/types/Router-Gate/about.md:1`, `website/src/content/docs/syntax/attributes/routes.mdx:34`.
  Each says that `nvs check` warns when a gate ignores a value.

### Backlog

- Stage 3's Rust tests are the five `dispatching` cases in `crates/nvs-stdlib/src/router.rs`
  (`a_responder_is_handed_the_value_a_route_method_returns` onward).
- Before `DONE`: `bun nv verify --doc` and `bun nv owners --closes router-responder`.
