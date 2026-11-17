---
looper: 1
title: "a method takes a route and its parameters, checked like a link"
summary: "Reached when a call that passes a written method reference to a #[Core\\Router\\Target] parameter is checked against the route table as Core\\Router::url is, with its #[Core\\Router\\Params] array, the reference keeps its visibility check, the editor completes and counts it, and the feature has its proofs and pages."
context: [crates/nvs-types/src/links.rs, docs/rules/routing/link-target-and-params-are-checked.md, docs/rules/routing/a-route-target-parameter-is-checked-like-a-link.md]
stages:
  - id: 1
    title: the two attributes are read into the signature, and one written where it cannot be read does not compile
    status: open
    context: [crates/nvs-types/src/derive.rs, crates/nvs-types/src/paths.rs, crates/nvs-types/src/attributes.rs, crates/nvs-types/src/signatures.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-diagnostics/src/code/mod.rs, crates/nvs-diagnostics/src/code/types_e08.rs, docs/rules/expressions/method-reference.md]
  - id: 2
    title: a written route at a Target argument is checked against the finished route table, and the call stays an ordinary call
    status: open
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/args.rs, crates/nvs-types/src/check.rs, tests/conformance/reject/a-route-link-refuses-an-unknown-name-and-an-uncovered-capture.nvst, tests/conformance/lang/a-test-links-a-method-reference-passed-as-a-value.nvst, docs/rules/routing/a-link-reaches-another-module.md, docs/rules/core-classes/router-signed-url.md]
  - id: 3
    title: the editor completes the keys, hovers the reference and counts the call as a use of the route method
    status: open
    context: [crates/nvs-lsp/src/arguments.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/reach.rs, crates/nvs-lsp/tests/completion_edits.rs, crates/nvs-cli/src/code.rs, crates/nvs-cli/tests/code.rs, tests/lsp/references/a-routed-method-answers-every-link-to-it.lspt, docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md]
  - id: 4
    title: the feature's proofs and its pages
    status: open
    context: [docs/examples/lang/attributes/chapter.md, docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md, docs/agent/prose.md, docs/rules/testing/feature-proofs.md, website/src/content/docs/syntax/attributes/routes.mdx, benches/members/lang/attributes/core-route-and-core-access-the-route-table.nvs, tests/hostile/lang/attributes/core-route-and-core-access-the-route-table/01-capture-values-that-try-to-leave-their-route.nvs]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: attribute-sites
    stage: 1
    name: "a Target on a parameter that is not a callable, a Params on one that is not an array, a Params with no Target, a second Target in one method and either attribute on a method do not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/a-route-target-on-a-parameter-that-is-not-callable-does-not-compile.nvst, --case, tests/conformance/reject/route-params-on-a-parameter-that-is-not-an-array-does-not-compile.nvst, --case, tests/conformance/reject/route-params-without-a-route-target-does-not-compile.nvst, --case, tests/conformance/reject/a-second-route-target-in-one-method-does-not-compile.nvst, --case, tests/conformance/reject/a-route-target-on-a-method-does-not-compile.nvst]
    timeout: 1800
    tests:
      - tests/conformance/reject/a-route-target-on-a-parameter-that-is-not-callable-does-not-compile.nvst
      - tests/conformance/reject/route-params-on-a-parameter-that-is-not-an-array-does-not-compile.nvst
      - tests/conformance/reject/route-params-without-a-route-target-does-not-compile.nvst
      - tests/conformance/reject/a-second-route-target-in-one-method-does-not-compile.nvst
      - tests/conformance/reject/a-route-target-on-a-method-does-not-compile.nvst
  - id: call-checked
    stage: 2
    name: "at a call of a method with a Target, a written reference with no route, a missing capture, a stray key and a value its capture refuses do not compile, a named argument is read like a positional one, a private method of another class does not compile, a private method passed from its own class and a forwarded Target compile and run, and a callable parameter with no Target takes any method as before"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/a-route-target-argument-names-a-method-with-no-route.nvst, --case, tests/conformance/reject/a-route-params-argument-misses-a-capture.nvst, --case, tests/conformance/reject/a-route-params-argument-has-a-key-that-is-neither-a-capture-nor-a-query-parameter.nvst, --case, tests/conformance/reject/a-route-params-value-its-capture-cannot-convert.nvst, --case, tests/conformance/reject/a-route-params-value-outside-its-closed-set.nvst, --case, tests/conformance/reject/a-route-target-written-by-name-is-checked-like-a-positional-one.nvst, --case, tests/conformance/reject/a-route-target-names-a-private-method-of-another-class.nvst, --case, tests/conformance/lang/a-route-target-takes-a-private-method-from-inside-its-class.nvst, --case, tests/conformance/lang/a-forwarded-route-target-is-resolved-when-the-call-runs.nvst, --case, tests/conformance/lang/a-callable-parameter-without-route-target-takes-any-method.nvst]
    timeout: 1800
    tests:
      - tests/conformance/reject/a-route-target-argument-names-a-method-with-no-route.nvst
      - tests/conformance/reject/a-route-params-argument-misses-a-capture.nvst
      - tests/conformance/reject/a-route-params-argument-has-a-key-that-is-neither-a-capture-nor-a-query-parameter.nvst
      - tests/conformance/reject/a-route-params-value-its-capture-cannot-convert.nvst
      - tests/conformance/reject/a-route-params-value-outside-its-closed-set.nvst
      - tests/conformance/reject/a-route-target-written-by-name-is-checked-like-a-positional-one.nvst
      - tests/conformance/reject/a-route-target-names-a-private-method-of-another-class.nvst
      - tests/conformance/lang/a-route-target-takes-a-private-method-from-inside-its-class.nvst
      - tests/conformance/lang/a-forwarded-route-target-is-resolved-when-the-call-runs.nvst
      - tests/conformance/lang/a-callable-parameter-without-route-target-takes-any-method.nvst
  - id: call-checked-rust
    stage: 2
    name: "a Target site is checked against the table after the walk, records no RouteLink, reports a links-root class as E0897, checks a Target with no Params for its route alone, and a method with neither attribute has no route slots"
    run: [cargo, test, --lib, --, a_route_target_is_checked_against_a_route_declared_in_a_later_file, a_route_target_call_stays_an_ordinary_call, a_route_target_naming_a_links_root_class_does_not_compile, a_route_target_without_params_checks_only_the_route, a_method_without_route_attributes_has_no_route_slots]
    timeout: 1800
    tests:
      - a_route_target_is_checked_against_a_route_declared_in_a_later_file
      - a_route_target_call_stays_an_ordinary_call
      - a_route_target_naming_a_links_root_class_does_not_compile
      - a_route_target_without_params_checks_only_the_route
      - a_method_without_route_attributes_has_no_route_slots
  - id: editor-rust
    stage: 3
    name: "a Params key and a Target argument are completed as a link's are, and nvs code refs lists a Target argument as a use of the route method"
    run: [cargo, test, --test, completion_edits, --test, code, --, a_route_params_key_is_offered_the_routes_parameters, a_route_target_argument_is_offered_the_routed_methods_first, refs_lists_a_route_target_argument_as_a_use_of_the_route_method]
    timeout: 1800
    tests:
      - a_route_params_key_is_offered_the_routes_parameters
      - a_route_target_argument_is_offered_the_routed_methods_first
      - refs_lists_a_route_target_argument_as_a_use_of_the_route_method
  - id: editor
    stage: 3
    name: "the editor hovers a Target reference as it hovers a link's, finds it among the route method's references, and the used-by lens counts the call"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/hover/a-route-target-reference-hovers-like-a-links.lspt, --case, tests/lsp/references/a-routed-method-answers-a-route-target-argument.lspt, --case, tests/lsp/lens/a-test-that-passes-a-route-method-to-a-route-target-counts.lspt]
    timeout: 1800
    tests:
      - tests/lsp/hover/a-route-target-reference-hovers-like-a-links.lspt
      - tests/lsp/references/a-routed-method-answers-a-route-target-argument.lspt
      - tests/lsp/lens/a-test-that-passes-a-route-method-to-a-route-target-counts.lspt
  - id: feature-proofs
    stage: 4
    name: "the route parameters feature owes no proof"
    run: [bun, nv, proofs, --id, "lang:attributes/core-router-target-and-core-router-params-a-method-that-takes-a-route"]
    timeout: 900
---

# a method takes a route and its parameters, checked like a link

## Why

A framework often wraps `Core\Router::urlSigned` to fill in its keys and expiry, and every link in its
modules goes through that wrapper. The `restrict` goal is the setup that makes this the only way: it
forbids `urlSigned` outside the framework. But the wrapper loses the compile-time checks. A link
written as `Core\Router::url(Shop::show, ['id' => 3])` is checked while compiling: a method with no
route, a missing capture, a stray key and a value the capture cannot convert are all errors at the
call. The same link written as `Links::signed(Shop::show, ['id' => 3])` takes a plain `callable` and
an array, so each of those mistakes is found only when the line runs.

This goal lets a method say which of its parameters takes a route and which takes that route's
parameters. Every call of that method is then checked exactly as a call of `Core\Router::url` is:

```nvs
public static function signed(
    #[Core\Router\Target] callable $route,
    #[Core\Router\Params] array<mixed> $params,
): string
```

The person decided the feature, its two attributes, which checks a call gets, that a reference passed
as a value is still resolved at run time, the editor support and the full proofs. They are in
§ *Standing decisions* below with the choices this goal's author made, and none is reopened.

## Standing decisions

**The two attributes (the person's call).** `Core\Router\Target` goes on a `callable` parameter and
`Core\Router\Params` on an array parameter. A method has at most one of each. `Params` needs a
`Target` in the same method and pairs with it, and `Target` may stand alone, as
`Core\Router\Match::is` takes a route with no parameters. Neither takes a payload.
Declined, in the rule: an `of:` argument naming the paired parameter by a string. It is a new
mechanism, and a method that takes two routes is rare.

**Where each may be written (my call; safe default: the narrower set).** On a parameter of a method,
static or instance, and of a constructor. `derive::sites` gives both `Site::Param` alone, so a
promoted constructor parameter, a method, a class and a property get the existing `E0874`. A
top-level function's parameter is `E0874` too: its calls do not reach the call paths this goal changes.
A `Target` parameter's type is `callable`, a `callable(...)` type, or either one nullable. A `Params`
parameter's type is `array` or `array<...>`, nullable or not. A variadic parameter takes neither,
because many arguments fill it.

**The codes (my call).** `E0899` for an attribute on a parameter whose type or shape cannot take it,
in `crates/nvs-diagnostics/src/code/types_e08.rs` after `E0898`, the last free number of that range.
`E1000` for a pairing mistake (a `Params` with no `Target`, a second `Target` or a second `Params`), in
a new `crates/nvs-diagnostics/src/code/types_e10.rs`, because `E08xx` is then full and
`crates/nvs-diagnostics/src/code/mod.rs:23` names `E10xx` as the next range. Its table row reads
"types, continued: the `E08xx` range filled at `E0899`". If another goal takes `E0899` first, this goal
takes `E1000` and `E1001` in the new file and fixes this paragraph. Each code has its card in end-user
prose.

**What a call checks (the person's call).** At a call whose `Target` argument is a **written** method
reference, the same four questions `crates/nvs-types/src/links.rs:443` `resolve` asks for
`Core\Router::url`: the method has a `#[Route]` (`E0754`), a `$params` array literal covers every
capture (`E0755`), each of its keys is a capture or a `#[Query]` parameter (`E0759`), and a literal
value is one the capture's conversion and closed set accept (`E0772`). A method with no route, and a
reference that names no method, are reported as they are at a link. A named argument is read where it
was written, as `route:` is (links.rs module doc, lines 53-59). A computed `$params` is not checked. A
call that leaves the `Params` argument out is checked as `[]` when the parameter's default is `[]`,
and is not checked for any other default. Each message names the method the call wrote, not
`Core\Router::url`.

**The call stays an ordinary call (the person's call).** A `Target` site is recorded as
`crates/nvs-types/src/links.rs:212` `record_site` records a link, and checked after the walk in
`resolve`. `resolve` records nothing over it: no `ExprInfo::RouteLink`, so the `ExprInfo::Call`
stands, `nvs_ir` lowers an ordinary call, and the argument is a real callable value. A reference
passed as a value is not recorded, so a variable and a `Target` parameter forwarded to another such
method are resolved at run time, as today. `Core\Router::url` and its siblings keep `is_link` and
their prepared-path fold unchanged. A `Core` row never carries either attribute.

**The reference keeps its visibility check (my call, from the priority ordering).** A link to
`Core\Router::url` skips the visibility check because Core never calls the reference (links.rs:47-51,
`rule:routing/link-target-and-params-are-checked`'s "a link calls nothing"). A user method receives
a real callable, and it can call it. So `crates/nvs-types/src/expr/calls.rs:1566` `note_link_target`
does not mark a `Target` argument, and `check_method_visibility` (`calls.rs:604`) runs on it. A
`private` route method is passed only from inside its own class, which compiles today. **Security
and request isolation** outranks the convenience: skipping the check would let any class take another
class's private method and call it. The rule records it as a Declined entry, "Skipping the visibility
check for a `Target` argument". The same reasoning keeps the receiver rule. A written reference to an
instance method at a `Target` argument is an ordinary method reference
(`rule:expressions/method-reference`), and needs the receiver that rule asks for: inside an instance
method of its own class it forwards `$this` (`calls.rs:694`), so a page passes its own instance route
method. From anywhere else the method is `static`, or the link is written with `Core\Router::url`
itself. A conformance case of stage 2 passes an instance route method from its own class's instance
method.

**Which table a call checks against (the person's call, narrowed by my call).** The program's own route
table. A reference to a class under an `autoload links` root stays `E0897`
(`rule:routing/a-link-reaches-another-module`). That class is not compiled into the program, so no
callable value of it can exist, and `nvs_stdlib::router` resolves a value against this program's
table alone (`crates/nvs-stdlib/src/router.rs:56`). A link to another module is written with
`Core\Router::url` directly. Accepting it here would need a new run-time value, which the person ruled
out ("no change in the runtime"). The person asked for the links-root routes too. If they still want
them, that is a separate goal that adds a route-reference value. `urlSigned`'s refusal of another
module's route (`E0898`) stays inside the wrapper, at run time, because the call site cannot know
which `Core` member the method ends in.

**Overrides (my call; safe default: the same).** The marks are read from the signature the call
resolves to, as `#[Core\Path]`'s are (`MethodSig::param_text`). An override neither inherits nor needs
them. A call through an override with no marks is not checked at compile time, and still throws at
run time for a method with no route. That is lost checking, not lost safety: the visibility check
holds on every path.

**The representation (my call).** `MethodSig` (`crates/nvs-types/src/signatures.rs:57`) gains
`route_slots: Option<RouteSlots>`, where `RouteSlots { target: usize, params: Option<usize> }` holds
parameter indexes. It is `None` for every signature that marks nothing, so no existing signature
changes. `ResolvedCall` (`crates/nvs-types/src/expr_table.rs:99`) copies it, so the editor reads the
slots off the call it already holds. `LinkSite` (links.rs:104) gains one field saying whether the site
is a `Core` link or a `Target` call, and `record_site` takes the two parameter indexes in place of its
fixed `0` and `1`.

**The rule's home (my call).** A sibling fragment,
`docs/rules/routing/a-route-target-parameter-is-checked-like-a-link.md`. The existing rule is about
`Core\Router::url`, and its "a link calls nothing" is false for a user method. Two rules with different
visibility answers each keep their own Declined list. The existing rule gains one sentence that points
to the new one, and the new one cites the old for the four checks rather than restating them.

**The editor (the person's call).** `$params` key completion and routed-method completion at the
`Target` argument, as at `Core\Router::url`. A hover on the reference shows what a hover on a link's
reference shows. `nvs code refs` and the "used by" lens count the call as a use of the route method.
A written reference is already an occurrence in the reference index wherever it is written, so the
lens and `nvs code refs` are expected to need no code: stage 3's tests prove it. The lens keeps
"at least" for a method taken as a callable value, as
`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` already says, because the wrapper
may call it.

**What it costs.** Compile time only. One recorded site per call of a marked method whose `Target`
argument is a written reference, held until `resolve` runs and freed with the check. One
`Option<RouteSlots>` per signature and per resolved call, held once per compiled program. Nothing at
run time and nothing per request. The language gains two attributes and two codes.

**Lanes.** Stages 1 and 2 run in order: both change `crates/nvs-types`, and stage 2 reads the slots
stage 1 builds. After stage 2, stage 3 (`crates/nvs-lsp`, `crates/nvs-cli/tests/code.rs`,
`tests/lsp/`) and stage 4 (`docs/examples/`, `tests/hostile/`, `benches/members/`, `website/`) touch
disjoint files and may run as side goals at once. Stage 4's pages quote stage 2's error text.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the two attributes are read into the signature, and one written where it cannot be read does not compile

**Does:** writes the rule, declares the two attributes with their cards, reads them into
`MethodSig::route_slots`, and reports every attribute written where it cannot be read.

**Split:** once the codes land, a writing subagent writes the five `.nvst` cases the
`attribute-sites` check names, copying the shape of
`tests/conformance/reject/a-route-link-refuses-an-unknown-name-and-an-uncovered-capture.nvst`. The
session runs that check once.

- [ ] `docs/rules/routing/a-route-target-parameter-is-checked-like-a-link.md:1`: the new rule from
  § *Standing decisions*: the two attributes and where each goes, what a call checks, the call staying
  an ordinary call, the visibility and receiver checks, the table, overrides, and the cost line. Its
  Declined list: the `of:` argument, skipping the visibility check for a `Target` argument, a links-root
  route at a `Target` argument. One line in `docs/ground-rules.md` beside `:1049`. One sentence in
  `docs/rules/routing/link-target-and-params-are-checked.md:18` pointing to it. `bun nv rules
  --render`, then `bun nv rules --check` is green.
- [ ] `crates/nvs-types/src/derive.rs:796`: `ROUTE_TARGET` (`Core\Router\Target`) and `ROUTE_PARAMS`
  (`Core\Router\Params`) beside `QUERY`, each with a doc comment naming the rule. `:147` `sites` gives
  both `Site::Param`. `:254` `ATTRIBUTE_DOCS` gains a card for each (`rule:core-api/reference-card`),
  in end-user prose. `crates/nvs-types/src/attributes.rs:420` lists both among the names that take no
  options.
- [ ] `crates/nvs-types/src/route_params.rs:1`: a new module, the sibling of `crates/nvs-types/src/paths.rs`.
  `declared_slots` reads the two marks off a method's parameters, as `paths.rs:103` `declared_text`
  reads `#[Core\Path]`. `check_marker_sites` reports `E0899` (wrong type, variadic) and `E1000`
  (pairing), each at the attribute, as `paths.rs:460` `check_method` does. It is called beside
  `crate::paths::check_marker_sites` at `crates/nvs-types/src/attributes.rs:65`.
- [ ] `crates/nvs-types/src/signatures.rs:57`: `MethodSig::route_slots`, filled at `:1442` beside
  `param_text`, and `None` everywhere else a signature is built. `crates/nvs-types/src/expr_table.rs:99`:
  `ResolvedCall::route_slots`, copied where `param_text` is.
- [ ] `E0899` in `crates/nvs-diagnostics/src/code/types_e08.rs:1249` after `E0898`, and `E1000` in the new
  `crates/nvs-diagnostics/src/code/types_e10.rs:1`, its `AREA` added to `AREAS` at
  `crates/nvs-diagnostics/src/code/mod.rs:76` and its row to the table at `:23`. Each has its card.
- [ ] The five `.nvst` cases the `attribute-sites` check names, and the check run once.

### 2. a written route at a Target argument is checked against the finished route table, and the call stays an ordinary call

**Does:** records a site for each call of a marked method whose `Target` argument is a written
reference, and runs `resolve`'s four checks on it after the walk. It records no `RouteLink`, so
lowering and run time do not change.

**Split:** once the code lands, a writing subagent writes the ten `.nvst` cases the `call-checked`
check names, copying the shape of
`tests/conformance/reject/a-route-link-refuses-an-unknown-name-and-an-uncovered-capture.nvst` and,
for the run cases, `tests/conformance/lang/a-test-links-a-method-reference-passed-as-a-value.nvst`.
Each case uses the neutral names `Shop`, `Blog` and `Links`. The session writes the five Rust tests and
runs both checks once.

- [ ] `crates/nvs-types/src/links.rs:104`: `LinkSite` gains its kind (a `Core` link or a `Target`
  call, with the called method's name for messages). `:212` `record_site` takes the target and params
  parameter indexes and finds both through `argument_filling` (`crates/nvs-types/src/expr/args.rs:461`),
  so a named argument is read where it was written. A `Target` site whose reference names a links-root
  class records nothing; `E0897` is already reported there. An omitted `Params` argument is `[]` only
  when the parameter's default is `[]`.
- [ ] `crates/nvs-types/src/expr/calls.rs:180` (instance), `:963` (static) and `:1222` (`new`): where
  the resolved signature has `route_slots`, the site is recorded after `check_args_typed`, as the
  link branch at `:963` records one. `:1566` `note_link_target` is unchanged, and its doc says why a
  `Target` argument is not marked: it keeps the visibility check.
- [ ] `crates/nvs-types/src/links.rs:443` `resolve`: a `Target` site runs the same four checks and
  records nothing. The messages at `:621` (`E0754`), `:710` (`E0759`), `:757` (`E0755`), `:839` and
  `:900` (`E0772`) name the called method for a `Target` site, in end-user prose. The module doc
  (`:1-60`) gains a section *A method that takes a route* saying the call stays an ordinary call and
  why the visibility check stays.
- [ ] `crates/nvs-types/src/links.rs:941`: the five Rust tests the `call-checked-rust` check names, in
  the existing `mod tests` beside `a_link_to_a_links_root_route_is_checked_like_a_local_one` (`:1050`),
  which already builds a links root. `a_route_target_call_stays_an_ordinary_call` checks that the
  call's span looks up `ExprInfo::Call`, never `RouteLink`.
- [ ] The ten `.nvst` cases the `call-checked` check names, and both checks run once.

### 3. the editor completes the keys, hovers the reference and counts the call as a use of the route method

**Does:** the editor treats a `Target` call like a link: key completion, routed-method completion,
hover, references and the "used by" lens. It reads the slots off `ResolvedCall`, never a parameter
name.

**Split:** once the code lands, a writing subagent writes the three `.lspt` cases the `editor` check
names, copying the shape of `tests/lsp/references/a-routed-method-answers-every-link-to-it.lspt` and
`docs/agent/conventions.md` § *An `.lspt` case*. The session writes the three Rust tests and runs both
checks once.

- [ ] `crates/nvs-lsp/src/arguments.rs:152` `route_key_at`: a resolved call with `route_slots`
  answers like a link, with the array at the `params` slot and the reference at the `target` slot.
  `:214` `route_argument_at`: the argument at the `target` slot is a route argument. Both keep the
  `is_link` and `is_match_is` paths as they are.
- [ ] `crates/nvs-lsp/src/completion.rs:778` and `:1256` `route_keys`: expected to need no change once
  `route_key_at` answers. The Rust tests prove it.
- [ ] `crates/nvs-lsp/src/hover.rs:177`: a hover on a `Target` reference answers what a hover on a
  link's local reference answers. Expected to need no code; the `.lspt` case proves it.
- [ ] `crates/nvs-lsp/src/reach.rs:51` and `crates/nvs-cli/src/code.rs:497`: a written reference at a
  `Target` argument is an occurrence of the route method, so `nvs code refs` lists it and the lens
  counts the test that writes it, with "at least" as the rule says. Expected to need no code. If the
  index does not record it, the occurrence is added where the index records a method reference, and
  the rule fragment says so.
- [ ] `crates/nvs-lsp/tests/completion_edits.rs:188`: `a_route_params_key_is_offered_the_routes_parameters`
  and `a_route_target_argument_is_offered_the_routed_methods_first`, beside the link's two tests.
  `crates/nvs-cli/tests/code.rs:92`: `refs_lists_a_route_target_argument_as_a_use_of_the_route_method`.
- [ ] The three `.lspt` cases the `editor` check names, and both checks run once.

### 4. the feature's proofs and its pages

**Does:** the feature `lang:attributes/core-router-target-and-core-router-params-a-method-that-takes-a-route`
gets every proof `rule:testing/feature-proofs` lists, and the routes page explains it. Every file here
is text an end user reads (`AGENTS.md` § *Text an end user reads*), and each `.nvs` comment passes
`bun nv proofs --comments`.

**Split:** three writing subagents, whose files are disjoint, run side by side once the session has
written `about.md`: one writes the three examples and their `.out` files, one the attacks, one the
bench. Each is told the `about.md` path and stage 2's error text. The session writes the chapter
heading, `about.md`, `details.md` and the website section, and runs the `feature-proofs` check once.

- [ ] `docs/examples/lang/attributes/chapter.md:13`: the heading
  ``# `#[Core\Router\Target]` and `#[Core\Router\Params]`: a method that takes a route`` after the
  route table's, which makes the feature and its help (`nvs agent find` reaches it). The keywords at
  `:5` gain `Router\Target`, `Router\Params`, `Target`, `Params`. If `bun nv proofs` spells the id
  differently, the `feature-proofs` check takes its spelling.
- [ ] `docs/examples/lang/attributes/core-router-target-and-core-router-params-a-method-that-takes-a-route/about.md:1`
  and `details.md:1`: what the two attributes do, what a call checks, that a reference passed as a
  variable is checked when the call runs, that a `private` route method is passed only from its own
  class, and that a link to another module is written with `Core\Router::url`.
- [ ] Three examples in the same folder, each with its `.out`: `01-a-wrapper-that-builds-a-link`
  (`Links::url` calls `Core\Router::url` with both arguments), `02-a-wrapper-that-signs-a-link`
  (`Links::signed` calls `Core\Router::urlSigned` and fills in the keys and expiry), and
  `03-a-menu-built-from-route-methods` (an array of references forwarded through the `Target`
  parameter and resolved when the call runs).
- [ ] Attacks in `tests/hostile/lang/attributes/core-router-target-and-core-router-params-a-method-that-takes-a-route/`,
  laid out as the route table's attacks are: `01-take-another-classes-private-method`, marked
  `// hostile: expect-refusal` (a class passes another class's `private` route method to its own
  `Target` parameter, to call it), and `02-a-capture-value-that-leaves-its-segment`, which runs to its
  last line (a `$params` value with `/` and `..` passed through the wrapper stays in one encoded
  segment).
- [ ] `benches/members/lang/attributes/core-router-target-and-core-router-params-a-method-that-takes-a-route.nvs:1`:
  one link built through a wrapper with a `Target`, which a page does for every link it renders.
  `// bench: complexity constant`.
- [ ] `website/src/content/docs/syntax/attributes/routes.mdx:55`: a `FeatureSection` for the feature
  after *Link to another module*, and its id in the page's `covers` list at `:3`.
- [ ] The `feature-proofs` check run once, and every proof it names written.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-17 from a design conversation with the person. Nothing is built yet. The person's
calls are marked in § *Standing decisions*. Every other choice there is the authoring session's call
under the priority ordering, and stands until the person picks another. The `nvs test` fix for a
method reference passed as a value (`crates/nvs-cli/src/runner.rs`, case
`tests/conformance/lang/a-test-links-a-method-reference-passed-as-a-value.nvst`) lands before this
goal starts, and stage 2's run cases rely on it.

### Next

Stage 1 from its first item: the rule fragment, then `derive.rs`'s two names and cards,
`crates/nvs-types/src/route_params.rs`, `MethodSig::route_slots` at
`crates/nvs-types/src/signatures.rs:1442`, and the two codes. The `attribute-sites` cases do not
exist yet and are expected to fail until the stage writes them.

### Backlog

- `website/src/content/docs/guides/how-to-use/requests.mdx:51` and `:57` say a link is built from a
  route's `name`. `Core\Router::url` takes the route method. Not this goal's page; it belongs to
  whichever goal next edits that guide.
