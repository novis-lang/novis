---
looper: 1
title: "a program writes HTML attributes from a map, safely, and merges sets of them"
summary: "Reached when Core\\Html::attributes writes a map of attributes as Markup with every value escaped and double-quoted, true as the bare name, false and null left out, class and style lists joined, an unsafe URL left out unless the call sets allowAnyUrl: true, and an invalid name, class, id or style entry thrown at run time or refused at compile time when it is a literal; Core\\Html::mergeAttributes merges sets with the later set winning and class and style joined; a tainted map under allowAnyUrl does not compile; sanitize and attributes share one scheme list; and both members have their rule, proofs, help and pages."
context: [crates/nvs-stdlib/src/html.rs, docs/rules/core-classes/html-template.md, docs/rules/core-classes/html-auto-escape.md]
stages:
  - id: 1
    title: the rule says how attributes are written, checked and merged
    status: open
    context: [docs/agent/doc-style.md, docs/rules/core-classes/html-sanitize.md]
  - id: 2
    title: "Core\\Html::attributes writes a map of attributes, and sanitize shares its scheme list"
    status: open
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/markdown.rs]
  - id: 3
    title: "Core\\Html::mergeAttributes merges sets of attributes"
    status: open
  - id: 4
    title: "the checker refuses a literal that would throw, and a tainted map under allowAnyUrl"
    status: open
  - id: 5
    title: the feature proofs of both members
    status: open
    context: [docs/agent/testing.md, docs/agent/prose.md]
  - id: 6
    title: the help, the reference and the website say how to write attributes
    status: open
    context: [docs/agent/prose.md, website/src/data/changes.json]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: attributes-written
    stage: 2
    name: "Core\\Html::attributes escapes and quotes every value, writes true bare, leaves false, null and an unsafe URL out, and joins class and style lists"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/html-attributes-are-written-escaped-and-quoted.nvst]
    timeout: 1800
  - id: attributes-checked
    stage: 2
    name: "an invalid name, class, id or style entry throws, and an on* or srcdoc name throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/html-attributes-an-invalid-name-or-value-throws.nvst]
    timeout: 1800
  - id: attributes-merged
    stage: 3
    name: "Core\\Html::mergeAttributes keeps the later value and joins class and style"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/html-merge-attributes-the-later-set-wins.nvst]
    timeout: 1800
  - id: attributes-compile
    stage: 4
    name: "a literal that would throw does not compile, and neither does a tainted map under allowAnyUrl"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/html-attributes-a-literal-that-would-throw-does-not-compile.nvst]
    timeout: 1800
  - id: proofs
    stage: 5
    name: "every feature the goal's work reaches has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, html-attributes]
    timeout: 600
  - id: close
    stage: 7
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, html-attributes]
    timeout: 900
---

# a program writes HTML attributes from a map, safely, and merges sets of them

## Why

A web page writes attributes all the time, and Novis has no API for them. A developer writes each one
by hand inside an html template or a page. A quoted value is already safe: `{$x}` and `<?= $x ?>`
escape `"` and `'` (`crates/nvs-stdlib/src/html.rs:2816`). Everything else is left to the developer,
and `rule:core-classes/html-template` names it as the sink's blind spot that no rule closes yet
(`docs/rules/core-classes/html-template.md:59`):

- `href="{$url}"` writes a `javascript:` URL. `Core\Html::sanitize` removes one, but only from HTML it
  rebuilds (`html.rs:2216`).
- An unquoted attribute, an `on*` handler and an attribute name taken from data are all accepted.
- The everyday parts are written by hand with a ternary each: a boolean attribute (`disabled`,
  `checked`), an attribute left out when its value is `null`, a class list, and `data-*` or `aria-*`
  from a map. Nothing checks that a class or an `id` is valid HTML.

## Standing decisions

**Two new members on `Core\Html` (the user's call: an attributes API, a merge helper, an explicit
allow for `javascript:` URLs, `style` as a string or a list, and basic validity checks).**

```nvs
Core\Html::attributes(array<string, Value> $attributes, allowAnyUrl: bool = false): Core\Html\Markup
Core\Html::mergeAttributes(array<string, Value> ...$sets): array<string, Value>
```

`Value` stands for `string|int|uint|float|bool|null|array<?string>`, written out in the registry row
as the union. `allowAnyUrl` is a `CoreOption`, as `LATER_OPTIONS` declares `later`'s
(`html.rs:431`). `mergeAttributes` takes a `CoreTy::Variadic` (`registry.rs:857`). Stage 2 runs
`bun nv names --check`, and if the naming rule wants other names, it renames both and records the
names here.

**How `attributes` writes (my calls, inside the user's ones).**

- The output is ` name="value"` for each attribute, with one leading space, in the map's order. So
  ``html`<input{$attrs}>` `` and `<input<?= Core\Html::attributes([...]) ?>>` both work. An empty map
  gives an empty `Markup`.
- Every value is escaped by the same code as `Core\Html::escape`, and always written in double
  quotes. An `int`, `uint` or `float` is written as `echo` writes it.
- `true` writes the bare name. `false` and `null` leave the attribute out. For a name that starts
  with `aria-`, and for `contenteditable`, `draggable` and `spellcheck`, `true` and `false` write
  `"true"` and `"false"`, because those attributes need the word.
- A list is joined with one space, `null` entries are left out, and a repeated entry is written once.
  `class` is the common case, and `rel` and `aria-describedby` work the same way. An empty list leaves
  the attribute out.
- **`style` (the user's call):** a string, or a list where each entry is one CSS declaration, such as
  `"color: red"`. The entries are joined with `; `. A `null` entry is left out, and a trailing `;` on
  an entry is removed.
- A `Markup` value is not accepted. Its bytes can contain `"` and end the attribute.

**What `attributes` checks (the user's call: basic checks for what is invalid in HTML and CSS).**
The checks are the grammar of HTML and of a CSS declaration, never a list of known names, so a
framework's own attributes (`x-data`, `@click`, `:class`, `hx-get`) keep working.

- **A name** is not empty and has no ASCII whitespace, control character, `"`, `'`, `<`, `>`, `/`,
  `=` or `` ` ``. An invalid name throws `LogicError`, because a name is always the program's own.
- **`on*` and `srcdoc`** throw `LogicError`, compared without case. An `on*` value is JavaScript and
  a `srcdoc` value is a whole HTML document, and escaping makes neither safe. A framework attribute
  that runs code (`@click`) is that framework's sink, and the rule says so.
- **A class** (each token of a `class` string, or each list entry) is not empty and has no ASCII
  whitespace or control character. A list entry with a space throws, because a list entry is one
  class. It is not held to CSS identifier rules: `md:flex`, `w-1/2` and `2xl:p-4` are valid HTML
  classes, and a stricter check would break every Tailwind page.
- **An `id`** is not empty and has no ASCII whitespace or control character, as HTML requires.
- **A `style` declaration** (each list entry, or each part of a string split on `;` outside quotes
  and parentheses) is a property name, a `:` and a value that is not empty. The property name is a
  CSS identifier or a custom property (`--gap`). A `{` or `}` outside quotes throws.
- An invalid class, `id` or `style` value throws `RuntimeError`, because a value may come from data.
  The message names the attribute and the first character that is wrong. This is the split
  `rule:core-classes/ldap-error` already uses.

**URL attributes.** `href`, `src`, `action`, `formaction`, `cite`, `poster`, `background`, `data`,
`codebase`, `manifest`, `icon`, `ping`, `xlink:href` and `srcset` hold URLs. `ping` and `srcset` hold
several, and each one is checked. A URL whose scheme is not on the safe list leaves the attribute out,
as `sanitize` does, so bad input from a visitor never fails the request. The check is the scheme part
of `addressable` (`html.rs:2216`): tabs and line breaks are removed first, and a colon after `/`, `?`
or `#` is not a scheme. A scheme-relative `//host/x` is written, because `https://host/x` is written
too. Only `sanitize` keeps its host check.

**One scheme list.** `SCHEMES` (`html.rs:2188`) stays the one list, used by `sanitize`, by
`Core\Markdown` (`markdown.rs:631`) and by `attributes`. `tel` joins it, because it opens the phone's
dialer and runs nothing. That changes `sanitize` and `Core\Markdown::toHtml` too, so both get a
`changed` entry.

**`allowAnyUrl: true` (the user's call: an explicit way to allow `javascript:` URLs).** It writes
every URL attribute as given, still escaped. It covers every scheme (`javascript:`, `data:`, `blob:`,
an app's own `myapp:`), because they carry the same risk and one switch is simpler than one per
scheme. My calls, so that it stays explicit:

- The option takes a literal `true` or `false`. A computed value does not compile, so a reader always
  sees it at the call.
- A map whose type has a `tainted` element does not compile with `allowAnyUrl: true`. A visitor's
  `javascript:` URL is exactly the attack the default stops. Element qualifiers are part of the array
  type (`array<tainted string>`, `docs/examples/core/Html/parse/03-every-link-on-a-page.nvs:13`), so
  the checker reads them from the argument's type.

**How `mergeAttributes` merges (my calls).** The sets are read in order, and the result keeps the
order in which each name first appears. For `class` and `style`, the values join: a string and a list
both become list entries, and `attributes` writes them as above. For every other name, the later
set's value replaces the earlier one, `false` and `null` included, so a later `"disabled" => false`
removes an earlier `true`. A later `"class" => null` adds nothing and removes nothing. `mergeAttributes`
checks nothing. `attributes` checks the result, so each check has one home.

**What it costs** (`rule:programs/memory-priority`). `attributes` is one pass over the map that
writes the output bytes. `mergeAttributes` makes one new array per call. Both are per request and
freed with it. The checks are one scan of each name and value. Nothing is cached, and nothing grows
with traffic.

**Declined, for the rule's Declined list:**
- **A context-aware template that checks `href="{$url}"` itself.** `rule:core-classes/html-template`
  declines it already: it would need the compiler to track HTML positions.
- **CSS identifier rules for a class or an `id`.** They reject valid HTML that real pages use.
- **A list of known attribute names.** It breaks custom elements and every framework's attributes.
- **Checking CSS values**, such as `url(...)` inside `style`. Browsers run no script from CSS, and
  checking values is a CSS parser.
- **Throwing on an unsafe URL.** A visitor's input would fail the whole request.
- **A `Core\Html\Attributes` class.** A map is what a program already builds, and the class would
  need its own sink row next to `Markup`.
- **`Markup` as a value.**

**Lanes.** The stages run in order. Stages 2 and 3 share `html.rs`. Stage 4 is in the checker and
needs stage 2's rows. Stage 5's proof files are disjoint and go to writing subagents.

**What stops it.** One attempt per item and two sessions per stage. A stage that does not close in two
sessions writes what holds it in the handoff, and the next session splits it.

## Stages

### 1. the rule says how attributes are written, checked and merged

- [ ] `docs/rules/core-classes/html-attributes.md:1`: the rule, written the way
  `docs/agent/doc-style.md` says, from this file's standing decisions: the two signatures, how values
  are written, the checks and their errors, the URL attributes and the shared scheme list,
  `allowAnyUrl` and its two compile-time limits, how merging works, and what it costs. Its Declined
  list is the one above.
- [ ] `docs/rules/core-classes/html-template.md:55`: the paragraph about the sink's blind spot points
  to the new rule as the way to write an attribute from data, and is rewritten as a whole.
- [ ] `docs/rules/core-classes/html-sanitize.md`: the scheme list gains `tel`, and the rule names
  `SCHEMES` as the list `attributes` shares.
- [ ] `crates/nvs-stdlib/src/html.rs:1`: the module doc points to the new rule, rewritten as a whole
  (conventions.md § *A comment*).
- [ ] `bun nv rules --render`, then `bun nv rules --check` is green.

### 2. Core\Html::attributes writes a map of attributes, and sanitize shares its scheme list

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/html.rs:2188`: `tel` joins `SCHEMES`. `addressable` is split, so the
  scheme check is a function of its own that `attributes` calls, and `addressable` calls it beside its
  host check.
- [ ] `crates/nvs-stdlib/src/html.rs:197`: the `attributes` row after `escape`, its `allowAnyUrl`
  option, its reference card (`rule:core-api/reference-card`) and the runtime function. If the writer
  and its checks pass about 400 lines, they go into `crates/nvs-stdlib/src/html/attributes.rs`.
- [ ] Rust tests: each value kind, the `aria-*` words, list joining and repeats, `style` from a string
  and a list, each URL attribute with `javascript:`, `JaVaScRiPt:`, `java\tscript:`, `data:` and
  `//host`, `srcset` with one unsafe candidate, `allowAnyUrl`, and each check's error with its
  message. `tel:` and `mailto:` are written.
- [ ] `crates/nvs-stdlib/src/html.rs`: a `sanitize` test that keeps a `tel:` link, and a
  `markdown.rs` test that keeps one too.
- [ ] `tests/conformance/core/html-attributes-are-written-escaped-and-quoted.nvst:1` and
  `tests/conformance/core/html-attributes-an-invalid-name-or-value-throws.nvst:1`: the checks
  `attributes-written` and `attributes-checked` run them.
- [ ] `bun nv names --check` is green for the new member.

### 3. Core\Html::mergeAttributes merges sets of attributes

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/html.rs`: the `mergeAttributes` row, its card and the runtime function.
- [ ] Rust tests: the later value wins for a string, `true`, `false` and `null`; `class` and `style`
  join from strings and lists; the order of first appearance; no sets and one set.
- [ ] `tests/conformance/core/html-merge-attributes-the-later-set-wins.nvst:1`: a default set for a
  button merged with a caller's set, then written through `attributes`. The check
  `attributes-merged` runs it.

### 4. the checker refuses a literal that would throw, and a tainted map under allowAnyUrl

**Split:** an `Explore` subagent finds where the checker reads a `Core` call's literal arguments (the
check `toSource` or `Core\Config::set` uses for a literal is the model) and the diagnostic table's next
free error codes. Its answer is `file.rs:NN` and one line each.

- [ ] The checker: a literal key or a literal value in an `attributes` call that would throw at run
  time is a compile error, with the same message the runtime gives. A computed one is checked at run
  time only.
- [ ] The checker: `allowAnyUrl` takes a literal `bool`, and a map whose type has a `tainted` element
  under `allowAnyUrl: true` is a compile error that names the option.
- [ ] The two new error codes, each with its help in the binary and its row in the diagnostic table.
- [ ] `tests/conformance/core/html-attributes-a-literal-that-would-throw-does-not-compile.nvst:1`:
  both errors and a computed value that compiles. The check `attributes-compile` runs it.
- [ ] `docs/rules/core-classes/html-attributes.md`: the two codes are named in the rule.

### 5. the feature proofs of both members

**Split:** one writing subagent per member, side by side, and none of them builds. Each gets the
member's signature and card from stages 2 and 3, § *Text an end user reads* in AGENTS.md, and
`bun nv proofs --id 'Core\Html::escape'` as the model. The session builds and runs
`bun nv proofs --comments` over every new or changed `.nvs` once they all end.

- [ ] `Core\Html::attributes`: `docs/examples/core/Html/attributes/` (about.md, three examples: a form
  input with `required` and `disabled`, a link whose URL comes from a visitor, `data-*` and `aria-*`
  from a map), `benches/members/core/Html/attributes.nvs` and its `.scale.nvs` with growth by
  attribute count, and `tests/hostile/core/Html/attributes/01-a-visitor-chooses-the-link.nvs` (a
  `javascript:` URL, an `on*` name and a class with a quote, from a request).
- [ ] `Core\Html::mergeAttributes`: `docs/examples/core/Html/mergeAttributes/` (about.md, three
  examples: a component's default classes plus the caller's, a caller who turns off `disabled`, a
  `style` built from two lists), its bench, and
  `tests/hostile/core/Html/mergeAttributes/01-a-request-adds-a-handler.nvs` (a merged set from a
  request that adds `onclick`, which `attributes` then throws on).
- [ ] `Core\Html::sanitize` and `Core\Markdown::toHtml`: each `about.md` says that `tel:` links are
  kept.
- [ ] `bun nv proofs --owed --goal html-attributes` names nothing, and the check `proofs` is green.
  Each feature it names is paid here, new or changed, listed above or not.

### 6. the help, the reference and the website say how to write attributes

- [ ] `crates/nvs-stdlib/src/html.rs`: the `Core\Html` class card names `attributes` and
  `mergeAttributes`.
- [ ] `docs/examples/core/Html/escape/about.md` says that `Core\Html::attributes` writes a whole
  attribute, URL attributes included.
- [ ] `website/src/data/changes.json`: `new` `Core\Html::attributes`, `new`
  `Core\Html::mergeAttributes`, `changed` `Core\Html::sanitize` (`tel:` links), `changed`
  `Core\Markdown::toHtml` (`tel:` links).
- [ ] Every handwritten website page that writes an attribute from a variable, or explains escaping,
  is reread and stamped (`bun nv site --stamp`), and its attribute code uses `Core\Html::attributes`
  where a URL or a boolean attribute comes from data. An `Explore` subagent lists them first.
- [ ] `bun nv site --check stale` and `changes` are green.

### 7. the goal is verified once, and every failure is fixed

- [ ] The newcomer review, [prose.md](../agent/prose.md) § *A newcomer reads it*, over every page,
  `about.md` and card this goal wrote, and `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the session that discussed the design with the user. Nothing is built yet. Every design
choice is above, under *Standing decisions*. The ones marked as the user's call are not reopened.

### Next

Stage 1 from its first item. The checks name cases and proofs that do not exist yet, and they fail
until stages 2 to 5 write them.

### Backlog

- A call to `attributes` whose arguments are all literals could be computed at compile time, as a
  template with no holes is. Taken once a bench shows `attributes` on a page's hot path.
