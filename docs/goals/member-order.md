---
looper: 1
title: "class members have one order, which an editor action and nvs code order write"
summary: "nvs code order and the editor's Order class members action write the member order the rule gives, Core\\Json\\KeepKeyOrder keeps a derived class's JSON key order, the rules say so, and the checks below pass."
context: [docs/rules/tooling/fmt-never-reorders-members.md, docs/rules/core-classes/derive-property-list.md, crates/nvs-syntax/src/ast.rs, crates/nvs-types/src/derive.rs]
stages:
  - id: 1
    title: "the order is a rule, and Core\\Json\\KeepKeyOrder compiles only beside Core\\Json\\Derive"
    status: done
    context: [docs/rules/attributes/attach-sites-and-forms.md, docs/rules/core-classes/derive-attribute.md, crates/nvs-diagnostics/src/code/types_e08.rs, docs/reference/lang/90-attributes.md]
  - id: 2
    title: "nvs_lsp::order writes the member order, and the editor offers it as a source action"
    status: done
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/organize.rs, crates/nvs-lsp/src/capabilities.rs, crates/nvs-fmt/src/lib.rs, docs/rules/tooling/fmt-base-style-is-per.md]
  - id: 3
    title: "nvs code order writes, checks and prints the member order"
    status: done
    context: [crates/nvs-cli/src/code.rs, crates/nvs-cli/tests/code.rs, docs/rules/tooling/nvs-code-answers-about-the-project.md, crates/nvs-cli/src/agent.rs, docs/reference/tools/10-cli.md]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: member-order-tests
    stage: 2
    name: nvs_lsp::order puts every member where the rule says, keeps ties and frozen keys as written, and is offered only when asked for
    run: [cargo, test, --test, member_order]
    tests:
      - order_puts_groups_static_visibility_and_abstract_final_in_order
      - order_keeps_the_written_order_between_equal_members
      - order_puts_the_constructor_first_among_instance_methods
      - order_moves_comments_and_attributes_with_their_member
      - order_moves_a_joined_declaration_as_one
      - order_leaves_enum_cases_and_promoted_parameters_where_they_are
      - order_keeps_the_instance_properties_of_a_keep_key_order_class
      - order_leaves_a_keep_key_order_class_whose_promoted_field_comes_first
      - order_notes_a_derived_class_whose_key_order_changes
      - order_of_an_ordered_file_is_no_edit
      - order_formats_the_file_it_writes
      - order_offers_nothing_for_a_file_with_a_syntax_error
      - order_action_is_offered_under_source_and_never_under_fix_all
  - id: code-order-tests
    stage: 3
    name: nvs code order writes what the editor's action writes, checks without writing, and refuses a changed file
    run: [cargo, test, --test, code]
    tests:
      - code_order_writes_the_member_order_and_formats_each_file
      - code_order_check_lists_each_class_out_of_order_and_writes_nothing
      - code_order_dry_run_prints_each_class_it_would_move_and_writes_nothing
      - code_order_prints_a_note_for_a_derived_class_whose_key_order_changes
      - code_order_skips_a_file_with_a_syntax_error_and_names_it
      - code_order_refuses_a_file_that_changed_after_it_was_read
      - code_order_json_lists_every_class_and_note
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, member-order]
    reach: true
    floor: false
---

# class members have one order, which an editor action and nvs code order write

## Why

Two developers write the same class in two orders, and a reader of a long class does not know where
to look for its constructor, its static factory or its abstract methods. A project that wants one
order today writes it in its own style guide and checks it in review, because no tool knows it.

`rule:tooling/fmt-never-reorders-members` keeps `nvs fmt` out of this, and it stays that way. The
formatter runs on save and reads one file alone, and the order changes what a derived JSON codec
writes (`rule:core-classes/derive-property-list`). The same rule already allows the reordering "as a
deliberate, diff-visible code action". This goal builds that action, and gives Novis one member
order for it to write:

- **One order, in a rule.** Every project gets the same order, and nobody has to agree on one.
- **One function, two clients.** The editor's *Order class members* source action and
  `nvs code order` call the same `nvs-lsp` function, as `nvs code rename` and the editor's rename do
  (`rule:ide/one-server-two-thin-clients`).
- **One command for order and layout.** `nvs code order` leaves every file it writes formatted, so a
  project runs one command, not two. `--check` gives CI a red line for a class out of order.
- **An exception only where the order is visible outside the program.** A derived class whose JSON
  key order a consumer reads carries `#[Core\Json\KeepKeyOrder(reason: '…')]`, and its keys stay as
  written.

## Standing decisions

**The order.** It applies to the body of a class and an interface. Novis has no anonymous class:
`new class { … }` is `E0244` at the parse, so no file that parses has one. Top to bottom:

1. **Group:** `enum` and `type` declarations, then constants, then properties (hooked ones
   included), then methods.
2. **Inside a group:** every `static` member, then every instance member.
3. **Inside the static members, and again inside the instance members:** `public`, then
   `protected`, then `private`. The read visibility counts, so `public private(set)` is `public`.
4. **Inside one visibility:** `abstract`, then `final`, then the rest.

Steps 2 to 4 read only the modifiers the member writes. An interface method writes no `abstract`, so
it sorts with the rest. An `enum` or `type` declaration has none of these modifiers, so its group
keeps its written order.

**Equal members keep their written order.** The sort is stable. Two `public` instance methods with
no `abstract` or `final` stay in the order the author wrote them. This is what keeps a test class's
report order: every `#[Test]` method is `public`, so they keep their order among themselves.

**The constructor is the first instance method**, after every static method, whatever its
visibility and modifiers. A `private` constructor behind a `public static` factory is common, and a
reader looks for the constructor in one place.

**What does not move.** A promoted constructor parameter stays in the constructor. An enum's cases
keep their order, because `cases()` returns them in that order. Hooks stay inside their property.

**What moves with a member.** Its `///` run, its attributes, and every comment between it and the
member above it. That is the formatter's own reading: a comment between two members belongs to the
member below it (`rule:tooling/fmt-base-style-is-per`). A comment at the end of the member's last
line moves with it. A comment after the last member, above the class's `}`, stays last. A joined
declaration (`public int $a, $b;`) is parsed into one `ClassMember` per name, each spanning from the
declaration's first byte to its own name (`crates/nvs-syntax/src/parser/decl.rs:1548`). The members
that share a first byte are one unit, and it moves as one.

**A member moves as whole lines.** Its unit runs from the line after the member above it ends (or
after the body's `{`) to the end of its own last line. A body this cannot cut into whole lines is
left as written: two members on one line, a member on the `{` or `}` line, or a line that ends inside
a block comment. Such a body gets no edit and no note, since `nvs fmt` puts each member on its own
line and the next run orders it.

**`field_order` lists a skipped field.** `nvs_types::derive::field_order` takes no resolved names, so
a field with `skip: true` is in its list. A class whose only change is a skipped field's position
therefore gets the key-order note, or under `KeepKeyOrder` is left as written. That errs toward
naming a class, never toward a silent key change.

**`source` answers both source actions through `Kind::Source`.** `Kind` is one value per request, so
a request naming `source`, or naming both source kinds, gets `Kind::Source`, which answers organize
imports and then order members. A `.lspt` case cannot ask for `only=source`, because the case parser
admits only a kind in `CODE_ACTION_KINDS`; `order_action_is_offered_under_source_and_never_under_fix_all`
covers it from Rust.

**The output is formatted.** `nvs_lsp::order` moves each member's text, then runs
`nvs_fmt::format` (`crates/nvs-fmt/src/lib.rs:165`) over the whole result. The editor's action and
`nvs code order` therefore write a file that `nvs fmt --check` accepts. A file with a syntax error,
or one the formatter refuses, gets no action, and `nvs code order` names it and skips it. Safe
fallback: if a moved member's text cannot be formatted without changing a token, the function
returns no edit for the file, and `nvs code order` names the file and the class.

**A missing `reason` is `E0401`**, the code a missing field of any attribute's payload gets
(`docs/reference/lang/90-attributes.md`), and not a second new code. Stage 1 built it that way.

**`#[Core\Json\KeepKeyOrder(reason: string)]`.** A compiler-recognized attribute in
`ATTRIBUTES` (`crates/nvs-types/src/derive.rs:74`), allowed on a class only
(`crates/nvs-types/src/derive.rs:146`), with one required field, `reason`. It compiles only on a
class that also carries `#[Core\Json\Derive]`. Anywhere else it does nothing, so it is a new error
code at the attribute, the next free one in `crates/nvs-diagnostics/src/code/types_e08.rs`. The
compiler does nothing else with it: the codec reads the declaration order as it always has. An empty
`reason` compiles, and review judges it. It is named for JSON because `Core\Json\Derive` is the only
codec whose output order a consumer reads: `Core\Db\Derive` matches columns by name.

**What `KeepKeyOrder` freezes.** A derived class's field order is its instance properties in written
order, with the promoted constructor parameters at the constructor's position
(`crates/nvs-types/src/derive.rs:2397`). Under `KeepKeyOrder`, the instance properties keep their
written order, and everything else in the class is ordered as usual. The sorted order always puts the
constructor after every property. So if the written field order has a promoted parameter before a
declared property, no member order keeps it. The class is then left exactly as written. The action
skips it, `nvs code order` prints one note naming it, and `--check` does not report it.

**The field order has one home.** Stage 2 moves the walk at `crates/nvs-types/src/derive.rs:2402`
into one public function, `nvs_types::derive::field_order`, which returns the field names in encode
order. The codec check and `nvs_lsp::order` both call it, so the order the action protects is the
order the codec writes.

**A derived class whose key order changes is named.** A class with `Core\Json\Derive` and no
`KeepKeyOrder` is ordered like any other. When its field order before and after differs,
`nvs code order` prints one note: the class, and that its JSON key order changes. The editor's action
writes the same edit and shows no note, because the change is in the diff the developer is looking
at.

**The editor action is a source action.** A new `actions::Kind::OrderMembers`
(`crates/nvs-lsp/src/actions.rs:114`), with the wire kind `source.orderMembers.nvs`, declared in
`CODE_ACTION_KINDS` (`crates/nvs-lsp/src/capabilities.rs:93`). Its title is "Order class members". It
edits the whole file, wherever the cursor is, as organize imports does
(`crates/nvs-lsp/src/organize.rs:31`). It is answered when `only` names it or names `source`, and a
request for `source` answers both source actions. It is never answered under `quickfix` or
`source.fixAll.nvs`, so no save runs it. That holds the line
`rule:tooling/fmt-never-reorders-members` draws. An ordered file gets no action.

**`nvs code order [paths...] [--check] [--dry-run] [--json]`.** With no path it orders every `.nvs`
file in the project. A path is a file or a directory under the project root.
- It writes through `write_out` (`crates/nvs-cli/src/code.rs:1074`), the one write path `rename` and
  `move` use. It writes only inside the root, and only files whose bytes are unchanged since the call
  read them. It prints one line per changed file with the number of classes it moved, then the
  notes.
- `--check` writes nothing. It prints one line per class out of order (position and full name) and
  exits 1 if there is one. It checks the order only. `nvs fmt --check` checks the layout.
- `--dry-run` writes nothing and prints one line per class it would move, with the notes.
- `--json` gives the files, the classes, the notes and whether anything was written, in the shape the
  other verbs use.

**`nvs fmt` does not change, and gets no flag.** `fmt` reads one file and runs on save. The order
needs names resolved across the project (`Core\Json\Derive`, `KeepKeyOrder`), and it must never run
on save. A flag on `fmt` would be a second name for `nvs code order`.

**The rules.** Two new rules are written `designed` with `goal: member-order` in stage 1, and become
`shipped` in stage 3:
- `rule:tooling/class-members-have-one-order`: the order, the ties, the constructor, what moves and
  what does not, and the two clients. Its place in `data/rules/tooling.json` is right after
  `tooling/fmt-never-reorders-members` (`data/rules/tooling.json:44`). Declined: a setting for a
  project's own order (one order for every project is the point); grouping a private helper under the
  method that calls it (the order would depend on a call graph and change with every edit); sorting
  by name (it moves members on every rename).
- `rule:core-classes/keep-key-order-freezes-a-derived-classs-keys`: the attribute, its site, its
  error, and what it freezes. Declined: a generic `KeepOrder` (the compiler could not check where it
  means something); an `order` field on `Core\Json\Field` (it is a second way to write the order the
  declaration already gives); refusing an empty `reason` (review judges a reason, and a check
  cannot).

`rule:tooling/fmt-never-reorders-members` changes in stage 1. Its body names the action and the
command. Its Declined bullet "A canonical member order" becomes "**`nvs fmt` writing the member
order**", with the reason above. `rule:tooling/nvs-code-answers-about-the-project` gains `order` in
stage 3, and its title says `rename`, `move` and `order` write.

**Text an end user reads.** The help text of `order` and its flags, the `PROTOCOL` line
(`crates/nvs-cli/src/agent.rs:1629`), the new error code's card, the attribute's `AttributeDoc`
(`crates/nvs-types/src/derive.rs:253`), the reference sections, the `about.md` and `details.md`
changes and every new example follow AGENTS.md § *Text an end user reads*.

**Trade-offs.** Security: nothing on the request path. `nvs code order` writes only inside the
project root, and only files that did not change after it read them. Latency: one analysis per file
per call, as every `nvs code` verb costs. Memory: one index for one call, nothing after it, and
nothing per request. Simplicity: one order every project shares, one function for both clients, one
new attribute, and `nvs fmt` unchanged. Usability: the order is opt-in per run, and a derived class's
key change is named before anyone ships it.

**Lanes.** Stage 2 reads stage 1's attribute constant, and stage 3 calls stage 2's function, so the
stages run in order. The goal depends on no other chain goal, and runs after `code-rename-and-calls`,
whose `write_out` it reuses.

## Stages

### 1. the order is a rule, and Core\Json\KeepKeyOrder compiles only beside Core\Json\Derive

**Does:** the rulebook holds the member order and the attribute, and the compiler accepts
`KeepKeyOrder` beside `Core\Json\Derive` and nowhere else.

- [x] `rule:tooling/class-members-have-one-order`: record and fragment, `designed`, `goal: member-order`, inserted after `tooling/fmt-never-reorders-members`. `data/rules/tooling/class-members-have-one-order.json:1`, `docs/rules/tooling/class-members-have-one-order.md:1`, `data/rules/tooling.json:44`
- [x] `rule:core-classes/keep-key-order-freezes-a-derived-classs-keys`: record and fragment, `designed`, `goal: member-order`, placed after `core-classes/derive-property-list` in its topic's list. `data/rules/core-classes/keep-key-order-freezes-a-derived-classs-keys.json:1`, `docs/rules/core-classes/keep-key-order-freezes-a-derived-classs-keys.md:1`
- [x] `rule:tooling/fmt-never-reorders-members` names the action and the command, and its Declined bullet is rewritten. Then `bun nv rules --render`. `docs/rules/tooling/fmt-never-reorders-members.md:10`
- [x] `KEEP_KEY_ORDER` in `ATTRIBUTES` beside `DERIVE`, `Site::Class` in `sites`, and its `AttributeDoc`. `crates/nvs-types/src/derive.rs:74`, `crates/nvs-types/src/derive.rs:146`, `crates/nvs-types/src/derive.rs:253`, `crates/nvs-types/src/derive.rs:670`
- [x] The `reason` field is required, and the new error code fires when the class has no `Core\Json\Derive`. The check sits with the class's other derive checks. `crates/nvs-types/src/derive.rs:2311`, `crates/nvs-diagnostics/src/code/types_e08.rs:1031`
- [x] Conformance: one case that compiles and encodes in written order, one `reject/` case without `Core\Json\Derive`, and one `reject/` case without `reason`. Each runs once by name. `tests/conformance/core/json-keep-key-order-compiles-beside-json-derive.nvst:1`, `tests/conformance/reject/json-keep-key-order-needs-json-derive.nvst:1`, `tests/conformance/reject/json-keep-key-order-needs-a-reason.nvst:1`
- [x] The attributes reference gets a `Core\Json\KeepKeyOrder` paragraph under the JSON derive section, and the feature's `about.md` and `details.md` say what it keeps. `docs/reference/lang/90-attributes.md:684`, `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/about.md:1`, `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/details.md:1`
- [x] A seventh example: a class whose `type` key must come first. `bun nv proofs --comments` passes on it. `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/07-keep-the-json-key-order.nvs:1`

### 2. nvs_lsp::order writes the member order, and the editor offers it as a source action

**Does:** one function computes the ordered, formatted file, and the editor offers it under
`source.orderMembers.nvs`.

**Split:** the `.lspt` cases under `tests/lsp/actions/` are a batch of similar files. A writing
subagent writes them once the session has landed `nvs_lsp::order` and the kind. It builds nothing.

- [x] `nvs_types::derive::field_order`: the walk at the codec check becomes one public function, and the check calls it. `crates/nvs-types/src/derive.rs:2467`
- [x] `nvs_lsp::order`: the sort key the standing decision gives, the stable sort, the units that move, the `KeepKeyOrder` freeze and its skip, the derived-class notes, then `nvs_fmt::format`. It returns the new text, the classes it moved, and the notes, or nothing for an ordered file. `crates/nvs-lsp/src/order.rs:1`, `crates/nvs-lsp/src/lib.rs:145`
- [x] `Kind::OrderMembers`, its wire name, `asked_for` answering `source` with both source actions, and `CODE_ACTION_KINDS`. `crates/nvs-lsp/src/actions.rs:114`, `crates/nvs-lsp/src/actions.rs:145`, `crates/nvs-lsp/src/actions.rs:188`, `crates/nvs-lsp/src/capabilities.rs:93`
- [x] The handshake test that pins the capabilities expects the new kind. `crates/nvs-lsp/tests/handshake.rs:1`
- [x] The tests the `member-order-tests` check names. `crates/nvs-lsp/tests/member_order.rs:1`
- [x] `.lspt` cases: an unordered class, an ordered file with no action, and a `source.fixAll.nvs` request that does not list it. `tests/lsp/actions/order-members-orders-a-class.lspt:1`, `tests/lsp/actions/order-members-is-not-offered-for-an-ordered-file.lspt:1`, `tests/lsp/actions/order-members-is-never-a-fix-all.lspt:1`
- [x] The editor reference names the source action. `docs/reference/tools/40-editor.md:9`

### 3. nvs code order writes, checks and prints the member order

**Does:** `nvs code order` writes what the editor's action writes, and `--check` gives CI a red
line for a class out of order.

- [x] The `Order` verb with `[paths...]`, `--check`, `--dry-run` and `--json`, and its help text. It calls `nvs_lsp::order` per file and writes through `planned` and `committed`, the write path `write_out` wraps. `crates/nvs-cli/src/code.rs:136`, `crates/nvs-cli/src/code.rs:230`, `crates/nvs-cli/src/code.rs:1074`
- [x] The tests the `code-order-tests` check names. The changed-file test uses the test hook `rename` already uses, never a timer. `crates/nvs-cli/tests/code.rs:1`
- [x] `PROTOCOL` names `order` in one line, and the `tests/agent.rs` test that pins the text is updated. `crates/nvs-cli/src/agent.rs:1629`
- [x] The CLI reference and the `nvs code` feature describe `order`, with a second example: a class before and after. `bun nv proofs --id 'tools:cli/nvs-code'` owes nothing, and `bun nv proofs --comments` passes on the example. `docs/reference/tools/10-cli.md:1045`, `docs/examples/tools/cli/nvs-code/about.md:1`, `docs/examples/tools/cli/nvs-code/02-order-class-members.nvs:1`
- [x] `rule:tooling/nvs-code-answers-about-the-project` describes `order` and its title names the three writers. `docs/rules/tooling/nvs-code-answers-about-the-project.md:57`, `data/rules/tooling/nvs-code-answers-about-the-project.json:2`
- [x] Both new rules are `shipped`, with their tests and conformance cases as `guardedBy`. Then `bun nv rules --render` and `bun nv render --website`. `data/rules/tooling/class-members-have-one-order.json:1`, `data/rules/core-classes/keep-key-order-freezes-a-derived-classs-keys.json:1`
- [x] `bun nv owners --closes member-order` passes. `tools/nv/cmd/owners.ts:1`

### 4. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done. `nvs code order` (`crates/nvs-cli/src/code.rs`, `fn order` and
`fn ordered_files`) runs `nvs_lsp::order::of` over each file a path names, or every indexed file,
and writes each changed file as one whole-file `Rewrite` through `planned` and `committed`, the
write path `write_out` also wraps. `write_out` itself prints by edit, so `order` prints by class
instead. `--check` prints class lines only and exits 1; `--dry-run` prints class lines then notes;
a syntax error or a moved text that does not format goes to standard error and the file is skipped.
`--json` gives `files`, `notes`, `skipped` and `written`. The seven `code-order-tests` pass by name,
and so does the `tests/agent.rs` protocol pin.

`PROTOCOL`, the CLI reference (`docs/reference/tools/10-cli.md`, `# nvs code`), the feature's
`about.md` and its `02-order-class-members.nvs` example describe `order`; `bun nv proofs --id
'tools:cli/nvs-code'` owes nothing. Both new rules are `shipped`, the `nvs code` rule names three
writers, the member-order rule no longer names an anonymous class, the chapters and
`llms-full.txt` are rendered, and `bun nv owners --closes member-order` passes.

The sweep's `env-secret-proofs` log exits 0 and its nightly run (`40919832286`) is the one the
handoffs before read. Nothing new to fix there.

### Next

Stage 4, the gate.

- [ ] Start `looper verify` with `run_in_background`, end the turn, and group its failures by cause. `crates/nvs-cli/tests/code.rs:1`
- [ ] Fix every failure in one pass, then run it once more. `crates/nvs-lsp/tests/coverage.rs:1`
- [ ] The goal-end gates: `bun nv verify --doc`, `bun nv owners --closes member-order`, `bun nv ci --check`. `docs/goals/member-order.md:1`

Tests that may go red: a help or index snapshot that lists every `nvs code` verb (the new `order`
verb and its flags reach `nvs agent index`), `crates/nvs-lsp/tests/coverage.rs` if it wants the
three `order-members` `.lspt` cases in its matrix, and the website stale guard if a rendered page
quotes the CLI reference.

### Backlog
