---
looper: 1
title: a link names a route of another module
summary: "Reached when Core\\Router::url and urlAbsolute link to a route a link-only autoload root declares, the server picks the owning mount at boot with canonical breaking a tie, a test and the editor give the same link, and the rules, reference, website and nvs agent show document it."
context: [docs/rules/programs/autoload.md, docs/rules/routing/link-target-and-params-are-checked.md, docs/rules/routing/link-carries-the-mount-prefix.md, docs/rules/http-server/a-mount-table-expands-at-boot.md]
stages:
  - id: 1
    title: autoload links and a prefixed discover build the map
    status: done
    context: [crates/nvs-syntax/src/parser/decl.rs, crates/nvs-syntax/src/ast.rs, crates/nvs-hir/src/autoload.rs, crates/nvs-cli/src/check.rs, editors/vscode/syntaxes/nvs.tmLanguage.json, website/config/novis.tmLanguage.json, docs/rules/classes/reserved-spellings-are-lower-case.md, data/rules/programs.json]
  - id: 2
    title: a link to a link-only root is checked and lowered
    status: done
    context: [crates/nvs-types/src/links.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-hir/src/members.rs, crates/nvs-hir/src/requires.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-diagnostics/src/code/types_e08.rs, docs/rules/routing/table-is-opt-in.md, docs/rules/core-classes/router-signed-url.md]
  - id: 3
    title: the server picks the mount a link goes through
    status: done
    context: [crates/nvs-config/src/tree.rs, crates/nvs-config/src/mount.rs, crates/nvs-cli/src/serve.rs, crates/nvs-cli/src/serve/mounts.rs, crates/nvs-cli/src/config.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-runtime/src/ctx/wiring.rs, docs/rules/routing/an-origin-is-per-mount-and-checked-at-boot.md, docs/rules/http-server/a-mount-carries-no-policy.md]
  - id: 4
    title: a test and the editor give the server's link
    status: done
    context: [crates/nvs-stdlib/src/test.rs, crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/src/completion.rs, docs/rules/testing/in-process-request.md]
  - id: 5
    title: two modules link under nvs serve, and the feature has its proofs
    status: done
    context: [crates/nvs-cli/tests/per_mount_config.rs, docs/examples/tools/server/mounts-which-file-answers-a-request/about.md, docs/examples/core/Router/url/about.md, docs/examples/lang/programs/autoload-find-a-class-by-its-namespace/about.md, tools/nv/proofs/roster.ts]
  - id: 6
    title: the rules, the reference, the website and nvs agent show say all of it
    status: done
    context: [docs/reference/tools/25-server.md, docs/reference/core/Router.md, website/src/content/docs/syntax/programs/namespaces.mdx, website/src/content/docs/syntax/attributes/routes.mdx, website/src/content/docs/guides/how-to-use/configuration.mdx, website/src/content/docs/guides/how-to-use/project-layout.mdx, crates/nvs-cli/src/agent.rs]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: links-roots-build-the-map
    stage: 1
    name: "autoload links parses in both forms, discover puts its prefix in front of each folder, and an ordinary root beats a links root at the same prefix"
    run: [cargo, test, --lib, --, a_links_declaration_parses_in_both_forms, discover_puts_its_prefix_in_front_of_each_folder, an_ordinary_root_beats_a_links_root_at_the_same_prefix, the_autoload_map_lists_links_roots_apart]
    timeout: 1800
    tests:
      - a_links_declaration_parses_in_both_forms
      - discover_puts_its_prefix_in_front_of_each_folder
      - an_ordinary_root_beats_a_links_root_at_the_same_prefix
      - the_autoload_map_lists_links_roots_apart
  - id: a-links-target-is-checked
    stage: 2
    name: "a link to a links-root route is checked like a local one, never joins the table, and any other use of a links-root class or a signed link to one does not compile"
    run: [cargo, test, --lib, --, a_link_to_a_links_root_route_is_checked_like_a_local_one, a_links_root_route_never_joins_the_table, a_links_root_class_used_for_anything_else_does_not_compile, a_signed_link_to_a_links_root_route_does_not_compile, a_links_target_file_joins_the_revalidation_set]
    timeout: 1800
    tests:
      - a_link_to_a_links_root_route_is_checked_like_a_local_one
      - a_links_root_route_never_joins_the_table
      - a_links_root_class_used_for_anything_else_does_not_compile
      - a_signed_link_to_a_links_root_route_does_not_compile
      - a_links_target_file_joins_the_revalidation_set
  - id: the-server-picks-the-mount
    stage: 3
    name: "a link takes the mount that served the request, then one on the same host, then one on another host with its origin; canonical breaks a tie, and an unbroken tie a link needs refuses the boot"
    run: [cargo, test, --bin, nvs, --, a_link_takes_the_mount_block_that_served_the_request, a_link_falls_back_to_a_mount_on_the_same_host, a_link_to_another_host_is_absolute_with_its_origin, canonical_breaks_a_tie, a_tie_a_link_needs_refuses_the_boot, a_tie_no_link_needs_still_boots, a_link_to_a_module_that_did_not_compile_throws_and_the_rest_serve]
    timeout: 1800
    tests:
      - a_link_takes_the_mount_block_that_served_the_request
      - a_link_falls_back_to_a_mount_on_the_same_host
      - a_link_to_another_host_is_absolute_with_its_origin
      - canonical_breaks_a_tie
      - a_tie_a_link_needs_refuses_the_boot
      - a_tie_no_link_needs_still_boots
      - a_link_to_a_module_that_did_not_compile_throws_and_the_rest_serve
  - id: canonical-is-checked
    stage: 3
    name: "canonical is a mount key, and nvs config check names an unbroken tie a link needs and a link no mount serves"
    run: [cargo, test, --test, mount, --, canonical_is_a_mount_key, two_canonical_mounts_for_one_entry_are_refused]
    timeout: 1800
    tests:
      - canonical_is_a_mount_key
      - two_canonical_mounts_for_one_entry_are_refused
  - id: a-test-and-the-editor-agree
    stage: 4
    name: "Core\\Test::request gives the link nvs serve gives, and the editor goes to a links target's definition"
    run: [cargo, test, --lib, --, a_test_request_gives_the_servers_link_to_another_module, a_links_target_goes_to_its_definition]
    timeout: 1800
    tests:
      - a_test_request_gives_the_servers_link_to_another_module
      - a_links_target_goes_to_its_definition
  - id: two-modules-link-under-nvs-serve
    stage: 5
    name: "under the built nvs serve, two scanned modules link to each other through the shared prefix, and an explicit second mount stays in itself"
    run: [cargo, test, --test, links_between_modules]
    timeout: 1800
    tests:
      - two_scanned_modules_link_through_the_shared_prefix
      - a_link_inside_a_second_mount_of_one_entry_stays_in_that_mount
      - a_link_to_a_module_on_another_host_carries_its_origin
  - id: links-proofs-complete
    stage: 5
    name: "the links-between-modules feature owes no feature proof"
    run: [bun, nv, proofs, --id, "tools:server/links-between-modules"]
    timeout: 600
  - id: router-url-proofs-complete
    stage: 5
    name: "Core\\Router::url still owes no feature proof"
    run: [bun, nv, proofs, --id, "Core\\Router::url"]
    timeout: 600
  - id: brief-finds-the-links-rule
    stage: 6
    name: "bun nv brief --where finds the new rules by link, module and canonical"
    run: [bun, nv, brief, --where, canonical]
    stdout_contains: ["rule:http-server/a-canonical-mount-breaks-a-tie", "rule:routing/a-link-reaches-another-module"]
    timeout: 300
  - id: the-server-reference-says-it
    stage: 6
    name: "the server reference chapter that nvs agent show and llms-full.txt print documents canonical and links between modules"
    file: docs/reference/tools/25-server.md
    output_contains: ["canonical = true", "autoload links", "Core\\Router::url"]
  - id: the-website-guide-says-it
    stage: 6
    name: "the website has a page for many modules on one server, with the mount table, autoload links and a link between modules"
    file: website/src/content/docs/guides/how-to-use/modules.mdx
    output_contains: ["[[server.mount]]", "autoload links", "Core\\Router::url", "canonical"]
  - id: website-renders
    stage: 6
    name: "every rendered website file, llms-full.txt included, is what its sources render"
    run: [bun, nv, render, --website, --check]
    timeout: 600
---

# a link names a route of another module

## Why

A framework is often split into many modules. Each module is its own program: one entry file,
`<Module>/public/index.nvs`, reached by one `scan` mount, declaring only the framework's root and
its own root. That isolation is wanted, and the language forces it: every module has a `Get /`, so a
program that declared two modules' roots would declare `/` twice (`E0748`,
`crates/nvs-types/src/routes.rs:3477`).

Pages link to other modules all the time, and today no such link is checked:

- `Core\Router::url` reverses only the current program's table. A link is recorded at
  `crates/nvs-types/src/links.rs:206`, resolved at `crates/nvs-types/src/links.rs:403`, and lowered
  to a prepared path (`crates/nvs-ir/src/lower/expr.rs:389`). At run time only the current request's
  prefix goes in front (`crates/nvs-stdlib/src/router.rs:1556`).
- `Core\Router::url(Shop\View\Index::render)` written in `Blog` is `E0303`, because `Blog` has no
  `Shop` root (`crates/nvs-hir/src/members.rs:1082`). Declaring that root pulls every `Shop` route into
  `Blog`'s table, because the table scans every root (`crates/nvs-hir/src/requires.rs:599`).
- So the link is a string, `'/shop/'`. Under `prefix = "/base/{1:lower}"` it misses `/base`, and
  nothing in `nvs check` or a test can see it.

The user decided the design in an interactive session on 2026-11-12: a link-only root, a prefixed
`discover`, the owning mount chosen at boot, and a `canonical` mount key. The user also asked that
every part of it be documented where users and agents look, which is stage 6.

## Standing decisions

**The syntax is `autoload links`.** `links` follows `autoload` the way `discover` does, and both forms
take it:

```nvs
autoload 'Framework\Core' from '../../Framework/src';
autoload 'Framework\{..}' from '../src';                 // this module's own root
autoload links discover 'Framework' from '../../*/src';  // every module, for links only
autoload links 'Framework\Shop' from '../../Shop/src';   // or one module by name
```

`links` is a contextual word like `discover`, listed beside it in
`rule:classes/reserved-spellings-are-lower-case`. Declined: a trailing `for links`, because the
modifier would sit far from the keyword. Declined: a separate `link` declaration, because it would be
a second keyword with the same path rules.

**`autoload discover` takes an optional fixed prefix, for every root and not only links ones.**
`autoload discover 'Framework' from '../../*/src';` maps `Shop/src` to `Framework\Shop`. Without the
prefix, `discover` works exactly as today. The prefix is held to `E_AUTOLOAD_PREFIX_SHAPE` and may
carry one braced segment, as an explicit prefix may. Declined: a `*` inside the prefix
(`'Framework\*'`), because `'App\*'` is the PSR-4 mistake `E_AUTOLOAD_PREFIX_SHAPE` names today.

**An ordinary root beats a links root at the same prefix.** The module's own root and the framework's
root are ordinary, so `links discover` over every module skips them, the same way an explicit prefix
already beats a `discover` glob (`crates/nvs-hir/src/autoload.rs:449`). Two links declarations with
the same prefix and different roots are `E_DUPLICATE_AUTOLOAD_PREFIX`. That is why a route of a shared
root, such as the framework's login page, always links inside the current module: it is in the
current program's own table.

**A links root is read for routes, and never compiled.** Its classes are not autoloaded, never join
the route table or `implementing<T>()`, and are not in the program enumeration's scan. The one place a
name under a links root may be written is the route argument of `Core\Router::url` and
`Core\Router::urlAbsolute`, and a `use` import of a class the program links to, so a link may write
the short name. An import names nothing a program can call, and every other use of the imported name
is still the error below. Declined: forcing the fully qualified name at each link, because a
namespaced file then cannot write a link the way it writes every other class name. Anywhere else it is a new `E08xx` code (the next free one in
`crates/nvs-diagnostics/src/code/types_e08.rs`) whose message says the root is declared for links
only. The check parses only the file that declares the target class, and reads the target method's
`#[Route]` and `#[Query]` attributes and its capture types. The method, the captures and the keys are
checked exactly as a local link is (`rule:routing/link-target-and-params-are-checked`). A capture
whose type is a class or enum of the links root is not value-checked, like a computed value. Its key
is still checked. Safe default if reading one file's attributes needs more of the hierarchy than the
parser gives: read the whole target class's declarations, still without compiling a body.

**`urlSigned` and `signedRoute` never take a links-root route.** That is a second new `E08xx` code.
A signature one program creates is checked with that program's key, so a program that could create a
signature another program trusts would cross the isolation boundary. This goes in
`rule:core-classes/router-signed-url`'s Declined list.

**A missing or broken target is a compile error in the linking program.** That is the dead link being
caught. It matches `rule:programs/autoload`'s rule that a hard cross-module reference needs that module
while compiling. A module that only some deployments ship must not be linked to from code every
deployment compiles. The target file joins the linking unit's revalidation set
(`crates/nvs-hir/src/autoload.rs:634`), so a changed or removed route recompiles the linker on its next
validation. With `validate = never`, which production runs, the deploy recompiles it. No new kind of
dependency is added.

**The compiler resolves which method, and the server resolves which mount.** A links-root site
lowers to the prepared mount-relative path plus the target's handler name, `Framework\Shop\View\Index::render`.
It does not lower to a finished path, so compiling never reads `nvs.toml`. That keeps the routing
rules' split: a path belongs to the source, and a mount belongs to the deployment.

**The server picks the mount at boot, in this order.** After every row is compiled, and again on every
reload that expands the table, each row whose unit has links-root links gets a table: handler to
prefix, plus an origin when the target is on another host. The candidates for a handler are the rows
whose own compiled table holds that handler as an ordinary route.
1. Rows from the same `[[server.mount]]` block as the row being linked from. A `scan` block counts as
   one block, so `/base/blog` links to `/base/shop`.
2. Otherwise, rows on the same host: the same resolved `host`, or neither row has one. The link is a
   path.
3. Otherwise, rows on another host that resolve an origin. The link is absolute with that row's
   origin, as `urlAbsolute` builds it.

`urlAbsolute` uses the same order and always writes an origin: the target row's origin, or the
served row's origin in steps 1 and 2. The table is built once per expansion and shared by every
request of that row through an `Arc`, the same way `at_mount_origin` gives the origin today
(`crates/nvs-cli/src/serve.rs:2566`). A link costs one hash lookup.
- **Memory:** O(rows × links-root handlers) per expansion. Nothing is held per request.

**A tie is broken by `canonical = true`, and only a tie a link needs is an error.** Two rows tie when
they are candidates at the same step. A row from a block with `canonical = true` wins. With none, or
with two, the boot fails naming both blocks and the handler, the same way an unreached origin fails it
(`crates/nvs-cli/src/serve.rs:575`). On reload, the linking row is left out and the reason is logged
(`crates/nvs-cli/src/serve/mounts.rs:428`). A tie that no link needs is not an error, so existing
tables with two mounts of one entry keep starting. `canonical` joins the closed key set of
`rule:http-server/a-mount-carries-no-policy`, because it says which door is the main one, which is
routing and not policy. It is `System` and reloadable, like `origin`.

**Two `canonical = true` blocks that mount one entry are `E0621`, whether or not a link needs
them.** `canonical` means "the one main door", so two of them say nothing a link could use. `nvs
config check` refuses two literal blocks naming one `entry`, and the expansion refuses a pair a
`scan` reaches; a later expansion keeps the first and leaves the second out. This is safer and
simpler than finding the contradiction only when a link happens to meet it.

**A target row that did not compile does not stop the server.** A link to it is logged at boot and
throws when it runs, naming the handler and the mount, until a pass compiles the target. This follows
`rule:http-server/a-mount-table-expands-at-boot`'s Declined boot that fails when any mount does not
compile.

**`nvs config check` runs the same resolution.** Today it checks the mount table's shape and compiles
nothing (`crates/nvs-config/src/mount.rs:106`). For this check it compiles each row's entry through the
compile cache, as the boot does, and reports an unbroken tie a link needs and a link no row serves.
`nvs config check --mounts` prints the expanded table: each row, its block, `canonical`, and the mount
each links-root handler resolves to. That is also what `rule:http-server/a-mount-table-expands-at-boot`
claims `nvs info --config` prints today, a command that does not exist. The rule is corrected to name
`nvs config check --mounts`.

**One resolution function, used by the boot, reload, `nvs config check` and `Core\Test::request`.**
It is pure: rows, each row's handler set and the served row in, a prefix and an optional origin out.
It lives beside `expand` in `crates/nvs-config/src/mount.rs:295`. `Core\Test::request` feeds it the
mount table of the configuration the test run reads, and its existing `mount` key picks the served row
by prefix. With no mount table, a links-root link throws naming the handler and saying that no mount
serves it.

**The user-facing name is "a link to another module".** Rule titles, the example folder, the website
page and the reference say *module*, and also say *program* once, so `bun nv brief --where` finds
them by `link`, `module`, `another program` and `canonical`. The feature's proof id is
`tools:server/links-between-modules`, a new example folder beside
`mounts-which-file-answers-a-request`.

**Lanes.** The stages run in order: stage 2 lowers what stage 1 parses, stage 3 serves what stage 2
lowers, and stages 4 to 6 document and prove it. Stage 3 changes `crates/nvs-cli/src/serve/mounts.rs`,
which `request-path-per-core` rewrites. Its gate is that goal's retirement: `docs/goals/request-path-per-core.md`
no longer exists. Inside stages 5 and 6 the writing is split across subagents, as their **Split:** says.

## Stages

### 1. autoload links and a prefixed discover build the map

**Does:** parse `links` and the discover prefix, build the map with links roots apart, and show them.

- [x] `AutoloadDecl` gets `links: bool`, and `AutoloadKind::Discover` gets `prefix: Option<..>`
  (`crates/nvs-syntax/src/ast.rs:1984`, `crates/nvs-syntax/src/ast.rs:2009`).
- [x] `parse_autoload_decl` reads `links` after `autoload`, and `discover 'Prefix' from 'glob'` beside
  today's `discover 'glob'` (`crates/nvs-syntax/src/parser/decl.rs:725`).
- [x] `AutoloadMap::build` keeps links roots in their own map, applies the discover prefix, and lets an
  ordinary root beat a links root at one prefix (`crates/nvs-hir/src/autoload.rs:367`,
  `crates/nvs-hir/src/autoload.rs:432`, `crates/nvs-hir/src/autoload.rs:449`,
  `crates/nvs-hir/src/autoload.rs:1120`).
- [x] `AutoloadMap::render` lists links roots in their own section for `nvs check --autoload-map`, and
  the flag's help says so (`crates/nvs-hir/src/autoload.rs:741`, `crates/nvs-cli/src/check.rs:73`).
- [x] Both grammars highlight `links` after `autoload` (`editors/vscode/syntaxes/nvs.tmLanguage.json:387`,
  `website/config/novis.tmLanguage.json:587`). The editor completes `links` and `discover` after
  `autoload` (`crates/nvs-lsp/src/completion.rs:814`).
- [x] Tests `a_links_declaration_parses_in_both_forms`, `discover_puts_its_prefix_in_front_of_each_folder`,
  `an_ordinary_root_beats_a_links_root_at_the_same_prefix` and `the_autoload_map_lists_links_roots_apart`
  beside the map's own tests (`crates/nvs-hir/src/autoload.rs:1`).
- [x] `rule:programs/autoload` gets the discover prefix and the links form, with the Declined
  spellings above. `rule:classes/reserved-spellings-are-lower-case` lists `links`
  (`docs/rules/programs/autoload.md:1`, `docs/rules/classes/reserved-spellings-are-lower-case.md:3`).

### 2. a link to a link-only root is checked and lowered

**Does:** resolve a written route argument through the links map, check it, refuse every other use,
and lower it to a path plus a handler.

- [x] A written route argument of `url` or `urlAbsolute` resolves its class through the links map when
  the ordinary map has no such class (`crates/nvs-types/src/expr/calls.rs:1559`,
  `crates/nvs-hir/src/members.rs:1082`).
- [x] The target file is parsed for its class's `#[Route]`, `#[Query]` and capture types alone. Its
  routes never reach `check_table` (`crates/nvs-types/src/routes.rs:1008`,
  `crates/nvs-types/src/routes.rs:3469`). The program scan skips links roots
  (`crates/nvs-hir/src/requires.rs:599`).
- [x] `record_site` and `resolve` check a links-root site against that route with the same four checks
  (`crates/nvs-types/src/links.rs:206`, `crates/nvs-types/src/links.rs:403`).
- [x] The two new `E08xx` codes with their cards: a links-root class used for anything else, and
  `urlSigned` or `signedRoute` on a links-root route (`crates/nvs-diagnostics/src/code/types_e08.rs:1`).
- [x] The target file joins the probe trace (`crates/nvs-hir/src/autoload.rs:634`).
- [x] `ExprInfo::RouteLink` carries the handler for a links-root site, and the lowering calls a new
  helper with the prepared path and the handler (`crates/nvs-ir/src/lower/expr.rs:389`,
  `crates/nvs-ir/src/lower/expr.rs:7929`). Until stage 3 the helper throws "no mount serves it".
- [x] The five tests of check `a-links-target-is-checked`, beside the link tests
  (`crates/nvs-types/src/links.rs:1`).
- [x] `rule:routing/table-is-opt-in` says a links root is never scanned and its routes never join the
  table (`docs/rules/routing/table-is-opt-in.md:25`). `rule:core-classes/router-signed-url` declines a
  signed link to another module (`docs/rules/core-classes/router-signed-url.md:1`).

### 3. the server picks the mount a link goes through

Gate: `docs/goals/request-path-per-core.md` no longer exists.

**Does:** add `canonical`, resolve each links-root handler per row at boot and reload, and give the
row's table to every request it serves.

- [x] `Mount` gets `canonical: Option<bool>`, and `Mounted` gets its block index and `canonical`
  (`crates/nvs-config/src/tree.rs:1417`, `crates/nvs-config/src/mount.rs:56`,
  `crates/nvs-config/src/mount.rs:532`).
- [x] The pure resolution function beside `expand`, with the three steps and the tie rule
  (`crates/nvs-config/src/mount.rs:440`).
- [x] The boot compile builds each linking row's table after every row has compiled, and an unbroken
  tie a link needs fails the boot like an unreached origin (`crates/nvs-cli/src/serve.rs:563`,
  `crates/nvs-cli/src/serve.rs:2484`, `crates/nvs-cli/src/serve.rs:2512`).
- [x] The reload builds the table again, and leaves out a linking row whose tie is unbroken
  (`crates/nvs-cli/src/serve/mounts.rs:401`).
- [x] Each request gets its row's table on the isolate beside its origin, and the new router helper
  reads it (`crates/nvs-cli/src/serve.rs:1381`, `crates/nvs-runtime/src/ctx/wiring.rs:439`,
  `crates/nvs-stdlib/src/router.rs:1556`).
- [x] `nvs config check` compiles the rows and reports a tie and an unserved link, and `--mounts`
  prints the table (`crates/nvs-cli/src/config.rs:1414`, `crates/nvs-cli/src/config.rs:60`).
- [x] The seven tests of check `the-server-picks-the-mount`, beside the origin tests
  (`crates/nvs-cli/src/serve.rs:5736`). The two of check `canonical-is-checked` in
  `crates/nvs-config/tests/mount.rs:1`.
- [x] New fragments `docs/rules/routing/a-link-reaches-another-module.md` and
  `docs/rules/http-server/a-canonical-mount-breaks-a-tie.md`, from the standing decisions, with their
  records, their slugs in `data/rules/routing.json` and `data/rules/http-server.json`, and
  `bun nv rules --render`.
- [x] `rule:http-server/a-mount-carries-no-policy` adds `canonical` to the key set, and
  `rule:http-server/a-mount-table-expands-at-boot` names `nvs config check --mounts` in place of
  `nvs info --config` (`docs/rules/http-server/a-mount-carries-no-policy.md:1`,
  `docs/rules/http-server/a-mount-table-expands-at-boot.md:26`).

### 4. a test and the editor give the server's link

**Does:** let `Core\Test::request` and the editor use stage 3's function and the links map.

- [x] `Core\Test::request` resolves a links-root link through the shared function, over the mount
  table of the configuration the test run reads, with the `mount` key picking the served row
  (`crates/nvs-stdlib/src/test.rs:2924`, `crates/nvs-stdlib/src/test.rs:2950`).
- [x] Go to definition on a links-root route argument opens the target method, and hover shows its
  route (`crates/nvs-lsp/src/definition.rs:899`).
- [x] Tests `a_test_request_gives_the_servers_link_to_another_module`
  (`crates/nvs-stdlib/src/test.rs:1`) and `a_links_target_goes_to_its_definition`
  (`crates/nvs-lsp/src/definition.rs:1`).
- [x] `rule:testing/in-process-request` says a test request gives the same link to another module
  (`docs/rules/testing/in-process-request.md:1`).

### 5. two modules link under nvs serve, and the feature has its proofs

**Does:** prove the whole path against the built binary, then write the feature proofs.

**Split:** once the three end-to-end tests pass, three writing subagents run side by side on disjoint
files: one writes the new example folder and its `about.md`, one writes the attack and the bench, and
one writes the three examples added to existing folders. Each is held to AGENTS.md § *Text an end user
reads* and runs `bun nv proofs --comments` on its own files. None of them builds.

- [x] New test file `crates/nvs-cli/tests/links_between_modules.rs`, on the harness of
  `crates/nvs-cli/tests/per_mount_config.rs:385`, with modules `Blog` and `Shop` under one scan
  `prefix = "/base/{1:lower}"`, an explicit second mount `/shop.second` of `Shop`'s entry, and a host
  mount on `shop.example.com` (`crates/nvs-cli/tests/links_between_modules.rs:1`).
- [x] Its three tests, named in check `two-modules-link-under-nvs-serve`.
- [x] New example folder `docs/examples/tools/server/links-between-modules/`: `about.md`, `nvs.toml`,
  two modules, and three numbered examples with `.nvsr` requests: a link between two scanned modules,
  a link to another host, and `canonical` choosing between two mounts
  (`docs/examples/tools/server/links-between-modules/about.md:1`).
- [x] Attacks `tests/hostile/tools/server/links-between-modules/01-call-a-method-of-another-module.nvs`,
  `02-create-an-object-of-another-module.nvs` and `03-sign-a-link-to-another-module.nvs`: a module
  tries to call, construct and sign through a class it may only link to, and each attempt does not
  compile. One file each, because an `expect-refusal` attack cannot name its code.
- [x] Bench `benches/members/tools/server/links-between-modules.nvs`: one link to another module per
  operation, with its growth (`benches/members/tools/server/links-between-modules.nvs:1`).
- [x] Example `05-link-to-another-module.nvs` in the autoload folder, showing `autoload links discover`
  (`docs/examples/lang/programs/autoload-find-a-class-by-its-namespace/about.md:1`). Example
  `04-a-link-to-another-module.nvs` in `docs/examples/core/Router/url/`
  (`docs/examples/core/Router/url/about.md:1`). Example `02` with `canonical` in
  `docs/examples/tools/server/mounts-which-file-answers-a-request/`
  (`docs/examples/tools/server/mounts-which-file-answers-a-request/about.md:1`). Each `about.md`
  gains one sentence for it.
- [x] The help text for `nvs config check --mounts` (`crates/nvs-cli/src/config.rs:60`).
- [x] `bun nv proofs --id tools:server/links-between-modules` and `bun nv proofs --id 'Core\Router::url'`
  each print `complete.` (`tools/nv/proofs/roster.ts:433`).

### 6. the rules, the reference, the website and nvs agent show say all of it

**Does:** make every place a user or an agent looks say what the feature does, and check that it does.

**Split:** three writing subagents run side by side on disjoint files: one writes the website pages,
one writes the reference chapter and the `Core\Router` cards, and one checks the rule fragments and
`brief --where`. Each is held to AGENTS.md § *Text an end user reads* where its files are read by
users. The session then runs `bun nv render --website` once.

- [x] New website page `website/src/content/docs/guides/how-to-use/modules.mdx`, "Many modules on
  one server": the `scan` mount, one entry file per module with its own root, `autoload links discover`,
  a link between modules, a link to another host, and `canonical`. It gets a sidebar entry if the
  sidebar is not generated, and `project-layout.mdx` and `configuration.mdx` link to it
  (`website/src/content/docs/guides/how-to-use/project-layout.mdx:30`,
  `website/src/content/docs/guides/how-to-use/configuration.mdx:37`).
- [x] `namespaces.mdx` documents `autoload links` and the discover prefix
  (`website/src/content/docs/syntax/programs/namespaces.mdx:14`). `routes.mdx` gets a section "Link to
  another module" (`website/src/content/docs/syntax/attributes/routes.mdx:100`).
- [x] The reference chapter's mount section documents `canonical`, the order a link picks its mount,
  `autoload links` and `nvs config check --mounts`. `nvs agent show` and `llms-full.txt` print this
  chapter (`docs/reference/tools/25-server.md:95`, `crates/nvs-cli/src/agent.rs:363`).
- [x] `docs/reference/core/Router.md` and the `url` and `urlAbsolute` cards say a link may name a route
  of another module, and the `urlSigned` card says it may not (`docs/reference/core/Router.md:1`,
  `crates/nvs-stdlib/src/router.rs:441`, `crates/nvs-stdlib/src/router.rs:459`,
  `crates/nvs-stdlib/src/router.rs:474`).
- [x] The language reference's autoload chapter that `bun nv render` writes into `docs/reference/lang/`
  carries the new forms after a render. If it is hand-written, the section is edited.
- [x] `bun nv brief --where` finds the new rules by `link`, `module`, `another program` and
  `canonical`. A rule's title is reworded until it does.
- [x] `bun nv render --website` writes `llms-full.txt`, and the three checks of stage 6 are green.

### 7. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and the goal is ready to retire. The first `looper verify` of the gate stage
named three failures, and the second run is green. The goal-end gates are green too:
`bun nv verify --doc`, `bun nv owners --closes links-between-modules` and `bun nv ci --check`.

- The server chapter's example block is formatted. An editor case names the stub line that the
  longer `url` card moved.
- `socket_max_duration_ends_an_endless_conversation` failed only under load. It now starts its
  lifetime once the peer's first tick is heard.
- The autoload example's `about.md` is back inside 200 words. Every perf figure this goal made
  stale is measured again, and the namespaces page is stamped.
- The sweep's `env-secret-proofs` names no check in the tree. The nightly's jobs were cancelled by a
  later push.

### Next

Nothing: the goal is reached. No test is expected red.

### Backlog
