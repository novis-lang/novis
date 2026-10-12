---
looper: 1
title: "The language rules that are designed are built"
summary: "Every rule this goal owns is `shipped` with a guard test, and `bun nv owners --closes language-rule-debts` passes."
context: []
stages:
  - id: 1
    title: "an `ESC` in source text is a compile error"
    status: open
    context: [crates/nvs-syntax/src/string_lit.rs, crates/nvs-render/src/text.rs]
    extra:
      context:
        rules: [tooling/an-escape-in-source-text-is-a-compile-error, tooling/terminal-output-is-a-sink, tooling/text-is-the-one-raw-path]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 2
    title: "`let` and `is` name nothing"
    status: open
    context: [crates/nvs-syntax/src/parser/decl.rs, crates/nvs-syntax/src/parser/mod.rs]
    extra:
      context:
        rules: [statements/let-and-is-are-reserved, classes/reserved-spellings-are-lower-case]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 3
    title: the renderings match the rule
    status: open
    context: [crates/nvs-render/src/plain.rs, crates/nvs-stdlib/src/debug.rs, crates/nvs-stdlib/src/cli.rs]
    extra:
      context:
        rules: [errors/renderings, tooling/the-terminal-profile-resolves-once]
        shapes: [A rule fragment]
  - id: 4
    title: "a generator does not `clone`"
    status: open
    context: [crates/nvs-types/src/expr/mod.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-runtime/src/object.rs, crates/nvs-runtime/src/graph.rs]
    extra:
      context:
        rules: [iteration/generator-stays-in-one-isolate, iteration/generators, classes/clone-is-shallow]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 5
    title: "`?T $x = null` decodes an absent key"
    status: open
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-types/src/defaults.rs]
    extra:
      context:
        rules: [core-api/required-optional-and-nullable, core-classes/derive-field-list]
        shapes: ["A `.nvst` test case"]
  - id: 6
    title: "`nvs check` warns on a nullable condition"
    status: open
    context: [crates/nvs-types/src/expr/mod.rs, crates/nvs-diagnostics/src/lib.rs]
    extra:
      context:
        rules: [expressions/nullable-condition-lint, expressions/truthy-positions]
        shapes: [A diagnostic]
  - id: 7
    title: a member reached from two sources is a compile error
    status: open
    context: [crates/nvs-types/src/conformance.rs, crates/nvs-types/src/layout.rs, crates/nvs-diagnostics/src/lib.rs]
    extra:
      context:
        rules: [classes/member-conflict-is-an-error, classes/interface-default-methods, classes/no-traits]
        shapes: [A diagnostic]
checks:
  - id: esc-in-source
    stage: 1
    name: "a string literal holding `ESC`, written raw or as an escape, does not compile, and the error names `Core\\Cli\\Text`"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/an-esc-byte-in-a-string-literal-does-not-compile.nvst, --case, tests/conformance/lang/an-escape-that-writes-esc-does-not-compile.nvst]
    tests: [tests/conformance/lang/an-esc-byte-in-a-string-literal-does-not-compile.nvst, tests/conformance/lang/an-escape-that-writes-esc-does-not-compile.nvst]
  - id: let-and-is-name-nothing
    stage: 2
    name: "nvs-syntax (`let` and `is` do not name a class, an interface, an enum, a constant, a function, a method or a parameter)"
    run: [cargo, test, --lib, "parser::tests"]
    tests: [let_and_is_name_no_declaration, let_and_is_name_no_method_and_no_parameter]
  - id: let-and-is-renamed-case
    stage: 2
    name: "the mutual-recursion case still runs with its method renamed"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/mutual-recursion-across-two-classes.nvst]
    tests: [tests/conformance/lang/mutual-recursion-across-two-classes.nvst]
  - id: renderings-colour
    stage: 3
    name: "nvs-render (the plaintext rendering is coloured when the terminal profile has colour, and plain when it has none)"
    run: [cargo, test, --lib, "plain::"]
    tests: [the_plaintext_rendering_is_coloured_when_the_profile_has_colour, the_plaintext_rendering_has_no_colour_when_the_profile_has_none]
  - id: generator-clone
    stage: 4
    name: "a `clone` of a generator does not compile, and one reached through `mixed` throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/cloning-a-generator-does-not-compile.nvst, --case, tests/conformance/lang/cloning-a-generator-through-mixed-throws.nvst]
    tests: [tests/conformance/lang/cloning-a-generator-does-not-compile.nvst, tests/conformance/lang/cloning-a-generator-through-mixed-throws.nvst]
  - id: nullable-default-decodes
    stage: 5
    name: "a `?T $x = null` field decodes an absent key and a `null` one to `null`, and encodes as `null`"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/json-decode-fills-an-absent-key-defaulted-to-null.nvst]
    tests: [tests/conformance/core/json-decode-fills-an-absent-key-defaulted-to-null.nvst]
  - id: nullable-condition-lint
    stage: 6
    name: "nvs-cli (`nvs check` warns on a `?T` condition, names `!= null`, and is silent on a compared one)"
    run: [cargo, test, --test, check]
    tests: [a_nullable_condition_warns_naming_not_null, a_nullable_compared_with_null_does_not_warn]
  - id: member-conflict
    stage: 7
    name: "nvs-types (two sources of one method are a compile error naming both, and an override resolves it)"
    run: [cargo, test, --test, interface_defaults]
    tests: [two_defaults_for_one_method_are_an_error_naming_both, a_default_and_a_delegate_for_one_method_are_an_error, a_class_that_declares_the_method_resolves_the_conflict]
  - id: owners-closes
    stage: 7
    name: "no designed rule or gap names this goal"
    run: [bun, nv, owners, --closes, language-rule-debts]
    reach: true
---

# The language rules that are designed are built

## Why

Every rule in the rulebook is `shipped` or `designed`, and every `designed` one names the goal or the
milestone that builds it. These seven describe the language and its compiler, and none is wholly
built: an `ESC` in a literal compiles, `is` names a method, the plaintext rendering has no colour, a
generator clones, `?T $x = null` cannot fill an absent key, a nullable condition is not warned
about, and two defaults for one method resolve silently to the first. This goal builds each and
flips its rule to `shipped`.

It is the last of the three chain goals the rule settlement made, after `request-isolation-debts`
and `server-rule-debts`.

## Standing decisions

**A stage ships its rule.** The stage that builds a rule sets its record's `status` to `shipped`,
removes its `goal` owner, fills `guardedBy` with the tests the stage's checks name
(`data/rules/<topic>/<slug>.json`), and rewrites the fragment whole so it says what is built, with no
"Not yet enforced" or "Not implemented" paragraph left. Home: the rule's own fragment.

**`ESC` in source text.** A string literal whose value holds U+001B is a compile error, whether the
byte is written raw or by an escape (`\e`, `\x1b`, `\033`, `\u{1b}`), in every string literal form
including a heredoc and an HTML template. The check runs where literals are cooked
(`crates/nvs-syntax/src/string_lit.rs:103`). The writer's call: a `bytes` literal is not checked,
because it is data and not text; the terminal sink (`crates/nvs-render/src/text.rs`) still
substitutes it at run time. A new `E` code, whose card names `Core\Cli\Text`.

**`let` and `is`** name nothing a program declares: `parse_decl_name`
(`crates/nvs-syntax/src/parser/decl.rs:366`) refuses them with `E0247`, as the other name positions
already do (`crates/nvs-syntax/src/parser/mod.rs:781`). A method is a function for this rule, so the
`is()` methods in `tests/conformance/lang/mutual-recursion-across-two-classes.nvst:6` are renamed. The
case's source is corrected and its expected output is not touched; the commit says why.

**The renderings.** The plaintext rendering is coloured exactly when the resolved terminal profile has
colour, read from `Core\Cli`'s one profile (`crates/nvs-stdlib/src/cli.rs:4`); the "no colour yet"
section of `crates/nvs-render/src/plain.rs:15-23` goes. The fragment's carrier table is rewritten to
what ships: `Core\Debug::render` returns `Core\Cli\Text` under every sink
(`crates/nvs-stdlib/src/debug.rs:69-81`), and the fragment says so. The session checks each other row
against the code before it flips the rule (not checked when this goal was written).

**A generator does not `clone`.** The copy walk already refuses a generator
(`crates/nvs-runtime/src/graph.rs:394`). `clone` of a value whose type is a generator is a compile
error beside `reject_non_object_clone` (`crates/nvs-types/src/expr/mod.rs:837-839`), and
`nvs_object_clone` (`crates/nvs-runtime/src/object.rs:4009`) throws for one reached through `mixed`,
with the copy walk's message.

**`?T $x = null`.** The checker accepts a written `= null` already (`crates/nvs-types/src/defaults.rs:52`),
so the fragment's "the checker still refuses" is stale. What is missing is the codec field default:
`ConstArg::Null` gives no `FieldDefault` (`crates/nvs-ir/src/lower/mod.rs:358`), so an absent key fails
at `crates/nvs-stdlib/src/json.rs:2797`. A `FieldDefault::Null` closes it for every door that reads a
derived codec.

**The nullable-condition lint** is a warning from `nvs check`, under the next free `W1xxx` code
(`crates/nvs-diagnostics/src/lib.rs:5491-5705`), raised in `check_condition`
(`crates/nvs-types/src/expr/mod.rs:199`) when the condition's type is nullable and not `bool`-only.
`?bool` warns too, since `null` and `false` are both false. It never refuses.

**Member conflict.** A class that does not declare a method reached from two sources, two interface
defaults or a default and a `by` delegate, is a compile error naming every source. A new `E` code, the
one `crates/nvs-diagnostics/src/lib.rs:652` names as `E_INTERFACE_MEMBER_CONFLICT`. The method walk's
order (`crates/nvs-types/src/layout.rs:1053-1056`) stays for `extends` over `implements`, which is not a
conflict. The session checks that `InterfaceName::method()` reaches a default from an override
(`rule:classes/interface-default-methods`); if it does not, building it is the stage's last item.

**Feature proofs.** A stage that changes what a program can write or what it sees updates that
feature's proofs (`rule:testing/feature-proofs`; `bun nv proofs --id '<feature>'` prints what is owed
and where): Stage 3 `Core\Debug::dump` and `Core\Debug::render`, Stage 4 `lang:iteration/generators`,
Stage 5 `Core\Json::decode`, Stage 6 `lang:expressions/logical-operators-and-truth`, Stage 7
`lang:classes/interfaces`. Every `.nvs` and `about.md` they touch follows AGENTS.md § *Text an end
user reads*.

## Stages

### 1. an `ESC` in source text is a compile error

**Does:** a string literal that holds `ESC` does not compile, and the error names `Core\Cli\Text`.

- [ ] The check where a literal is cooked, for every literal form. `crates/nvs-syntax/src/string_lit.rs:103`
- [ ] The new `E` code and its card. `crates/nvs-diagnostics/src/lib.rs:154`
- [ ] The conformance cases. `tests/conformance/lang/an-esc-byte-in-a-string-literal-does-not-compile.nvst:1`

### 2. `let` and `is` name nothing

**Does:** `let` and `is` are refused as the name of every declaration, a method and a parameter included.

- [ ] `parse_decl_name` refuses both with `E0247`. `crates/nvs-syntax/src/parser/decl.rs:366`
- [ ] A parameter named `$let` or `$is` is refused too, if the parser admits one today. `crates/nvs-syntax/src/parser/mod.rs:781`
- [ ] The `is()` methods in the mutual-recursion case renamed. `tests/conformance/lang/mutual-recursion-across-two-classes.nvst:6`

### 3. the renderings match the rule

**Does:** the plaintext rendering is coloured from the terminal profile, and the fragment's table is what ships.

- [ ] Colour from the one profile, and the "no colour yet" section rewritten. `crates/nvs-render/src/plain.rs:15`
- [ ] The carrier table reconciled with `Core\Debug::render`. `crates/nvs-stdlib/src/debug.rs:69`

### 4. a generator does not `clone`

**Does:** `clone` of a generator is a compile error, and a throw when it is reached through `mixed`.

- [ ] The compile error beside the clone operand rule. `crates/nvs-types/src/expr/mod.rs:837`
- [ ] The run-time refusal in the clone helper. `crates/nvs-runtime/src/object.rs:4009`

### 5. `?T $x = null` decodes an absent key

**Does:** a field declared `?T $x = null` decodes an absent key to `null` at every codec door.

- [ ] `FieldDefault::Null` from `ConstArg::Null`. `crates/nvs-ir/src/lower/mod.rs:358`
- [ ] The decoder fills it, and the fragment's designed row and stale sentence go. `crates/nvs-stdlib/src/json.rs:2797`

### 6. `nvs check` warns on a nullable condition

**Does:** a `?T` used directly as a condition is a warning that names `!= null`.

- [ ] The warning in `check_condition`. `crates/nvs-types/src/expr/mod.rs:199`
- [ ] The `W` code and its card. `crates/nvs-diagnostics/src/lib.rs:5675`

### 7. a member reached from two sources is a compile error

**Does:** a method two sources provide, and the class does not declare, is a compile error naming both.

- [ ] The check over every implemented interface and delegate. `crates/nvs-types/src/conformance.rs:136`
- [ ] The `E` code and its card. `crates/nvs-diagnostics/src/lib.rs:652`
- [ ] The method walk keeps `extends` over `implements`. `crates/nvs-types/src/layout.rs:1053`

## Handoff

### State

The goal is written; nothing of it is built. The user placed these seven rules here when every
`designed` rule was given an owner. § *Standing decisions* holds the calls, the writer's marked as
such. The anchors were read when the goal was written.

### Next

**Stages 1 and 2: the literal and the reserved words**, one file set: `crates/nvs-syntax/src/string_lit.rs`, `crates/nvs-syntax/src/parser/decl.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- [ ] `ESC` in a string literal does not compile (`rule:tooling/an-escape-in-source-text-is-a-compile-error`). `crates/nvs-syntax/src/string_lit.rs:103`
- [ ] Its `E` code and card (`rule:tooling/an-escape-in-source-text-is-a-compile-error`). `crates/nvs-diagnostics/src/lib.rs:154`
- [ ] `parse_decl_name` refuses `let` and `is` (`rule:statements/let-and-is-are-reserved`). `crates/nvs-syntax/src/parser/decl.rs:366`
- [ ] Rename the `is()` methods (`rule:statements/let-and-is-are-reserved`). `tests/conformance/lang/mutual-recursion-across-two-classes.nvst:6`

### Backlog

- Stages 3 to 7 — this file's § *Stages*.
