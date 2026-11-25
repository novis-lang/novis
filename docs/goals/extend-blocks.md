---
looper: 1
title: "extend T as Name adds methods, constants, enums, attributes and properties to a type the program does not own"
summary: "Reached when a named extend block adds instance and static methods, constants, nested enums and attributes, get-hooked and stored properties to a class, interface or enum from Core or another package, visible in a file by use, resolved at compile time and seen by no dynamic path, enum bodies take methods and constants, an added stored property lives in one hidden slot, the editor and tools know every added member, and the rules, examples, feature proofs and website say so."
context: [crates/nvs-syntax/src/ast.rs, crates/nvs-hir/src/resolve.rs, crates/nvs-types/src/expr/calls.rs, docs/rules/core-api/core-classes-are-sealed.md]
stages:
  - id: 1
    title: "the rules are written, and an extend block parses, resolves its target and loads by its name"
    status: done
    context: [crates/nvs-syntax/src/parser/decl.rs, crates/nvs-syntax/src/parser/stmt.rs, crates/nvs-syntax/src/token.rs, crates/nvs-syntax/src/walk.rs, crates/nvs-hir/src/hierarchy.rs, crates/nvs-hir/src/autoload.rs, crates/nvs-fmt/src/chain.rs, docs/rules/programs/one-declaration-per-autoloaded-file.md, docs/rules/classes/a-type-is-declared-at-file-scope.md, docs/agent/conventions.md]
  - id: 2
    title: "an enum body takes methods and constants"
    status: done
    context: [crates/nvs-syntax/src/parser/decl.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-ir/src/lower/mod.rs, docs/rules/enums.md]
  - id: 3
    title: "added methods, static methods and constants resolve where their block is used, and run as direct calls"
    status: done
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-hir/src/members.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/call.rs, crates/nvs-diagnostics/src/code/names.rs]
  - id: 4
    title: "an added member never changes a type: collisions, unions, interfaces, generics, nested types, restrictions and every dynamic path"
    status: done
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/conformance.rs, crates/nvs-types/src/restrict.rs, crates/nvs-types/src/attributes.rs, crates/nvs-types/src/expr/operators.rs, docs/rules/packaging/a-restriction-limits-where-a-declaration-is-used.md]
  - id: 5
    title: "added properties: get hooks on every type, stored values in one hidden slot"
    status: done
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-types/src/layout.rs, crates/nvs-runtime/src/revoke.rs, crates/nvs-runtime/src/graph.rs, docs/rules/classes/revocable-objects.md]
  - id: 6
    title: "the editor, nvs meta and nvs agent know every added member"
    status: open
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/src/rename.rs, crates/nvs-lsp/src/symbols.rs, crates/nvs-lsp/src/order.rs, crates/nvs-cli/src/meta.rs, crates/nvs-cli/src/agent.rs]
  - id: 7
    title: "the rules ship, and the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [data/reference/lang/50-classes.json, website/src/content/docs/syntax/classes/interfaces.mdx, website/src/data/changes.json, docs/rules/testing/feature-proofs.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: parse
    stage: 1
    name: "an extend block parses with its target and its name, and extend stays an ordinary identifier everywhere else"
    run: [cargo, test, --lib, extend_block_]
    tests: [extend_block_parses_its_target_and_name, extend_block_word_stays_an_identifier_elsewhere]
    timeout: 1800
  - id: fmt-coverage
    stage: 1
    name: "the extend block node kind is on exactly one formatter layout list"
    run: [cargo, test, --test, layout_coverage]
    tests: [every_node_kind_is_on_exactly_one_layout_list]
    timeout: 1800
  - id: shape
    stage: 1
    name: "an extend block of a type in its own package, of a name that is no type, or with a member a block cannot add does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/an-extend-block-of-a-type-in-its-own-package-does-not-compile.nvst, --case, tests/conformance/reject/an-extend-block-of-a-name-that-is-not-a-class-interface-or-enum-does-not-compile.nvst, --case, tests/conformance/reject/an-extend-block-takes-no-constructor-and-no-protected-member.nvst]
    timeout: 1800
  - id: enum-body
    stage: 2
    name: "an enum method reads its case as $this, an enum static method and constant are called on the enum, and an enum body still takes no property"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/enum/an-enum-method-reads-its-case-as-this.nvst, --case, tests/conformance/enum/an-enum-static-method-and-constant-are-called-on-the-enum.nvst, --case, tests/conformance/reject/an-enum-body-still-takes-no-property.nvst]
    timeout: 1800
  - id: added-calls
    stage: 3
    name: "added methods, static methods and constants run on a Core object, a Core enum and another package's class, by every call form"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/an-added-method-is-called-on-a-core-object.nvst, --case, tests/conformance/class/an-added-static-method-and-constant-are-reached-through-the-type.nvst, --case, tests/conformance/class/an-added-method-on-a-core-enum-reads-its-case.nvst, --case, tests/conformance/class/an-added-method-on-another-packages-class.nvst, --case, tests/conformance/class/an-added-method-takes-the-nullsafe-call-and-a-method-reference.nvst]
    timeout: 1800
  - id: added-visibility
    stage: 3
    name: "an added member is visible only in a file that uses its block, the error names the use to add, and a block's body sees only the public members"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/an-added-method-without-its-use-names-the-use-to-add.nvst, --case, tests/conformance/reject/an-added-method-cannot-read-a-private-member-of-its-type.nvst]
    timeout: 1800
  - id: direct-call
    stage: 3
    name: "a call of an added method lowers to one direct call"
    run: [cargo, test, --lib, added_method_lowers_]
    tests: [added_method_lowers_to_a_direct_call]
    timeout: 1800
  - id: collisions
    stage: 4
    name: "an added name that meets a member of its type, another block's member or a descendant's member does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/an-added-member-colliding-with-a-member-of-its-type-does-not-compile.nvst, --case, tests/conformance/reject/two-blocks-adding-one-name-to-a-type-do-not-compile.nvst, --case, tests/conformance/reject/a-subclass-declaring-an-added-name-does-not-compile.nvst]
    timeout: 1800
  - id: reach
    stage: 4
    name: "an added method reaches every implementer of an interface, binds a generic Core class's parameters, and a union finds it only from one block"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/an-added-method-on-an-interface-reaches-every-implementer.nvst, --case, tests/conformance/class/an-added-method-on-a-generic-core-class-binds-its-type-parameters.nvst, --case, tests/conformance/reject/a-union-finds-an-added-method-only-from-one-block.nvst]
    timeout: 1800
  - id: nested
    stage: 4
    name: "an added enum and an added attribute are named through their type, and an attribute added to a sealed Core class does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/an-added-enum-and-attribute-are-named-through-the-type.nvst, --case, tests/conformance/reject/an-attribute-added-to-a-sealed-core-class-does-not-compile.nvst]
    timeout: 1800
  - id: dynamic-paths
    stage: 4
    name: "no mixed value, dynamic call, operator, Reflect or JSON reaches an added member, and an added compareTo does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/an-added-method-is-not-found-through-a-mixed-value-or-reflect.nvst, --case, tests/conformance/reject/an-added-name-the-runtime-calls-by-name-does-not-compile.nvst]
    timeout: 1800
  - id: restrict
    stage: 4
    name: "an extend body cannot escape a restriction on its type, and a restriction names an added member by its block"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/an-extend-body-cannot-escape-a-restriction-on-its-type.nvst, --case, tests/conformance/reject/a-restriction-on-an-added-member-names-its-block.nvst]
    timeout: 1800
  - id: properties
    stage: 5
    name: "an added stored property is read and written on a Core request and on another package's class, a get hook computes on each read, and clone and a task copy start it empty"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/an-added-property-is-read-and-written-on-a-core-request.nvst, --case, tests/conformance/class/an-added-property-on-another-packages-class-reaches-its-subclasses.nvst, --case, tests/conformance/class/an-added-get-hook-is-computed-on-each-read.nvst, --case, tests/conformance/class/an-added-property-is-not-copied-by-clone-or-a-task.nvst, --case, tests/conformance/class/an-added-lateinit-property-read-before-a-write-throws.nvst]
    timeout: 1800
  - id: property-refusals
    stage: 5
    name: "an added stored property on a value class, an interface or an enum, a readonly one and one with no default do not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/an-added-stored-property-on-a-value-class-does-not-compile.nvst, --case, tests/conformance/reject/an-added-stored-property-needs-a-default-and-is-never-readonly.nvst]
    timeout: 1800
  - id: slot
    stage: 5
    name: "a class no block adds a property to has no added slot, the added values are freed with their object, and every Core class with instances has a recorded slot decision"
    run: [cargo, test, --lib, added_slot_]
    tests: [added_slot_is_absent_where_no_block_adds_a_property, added_slot_values_are_freed_with_their_object, added_slot_every_core_class_with_instances_has_a_decision]
    timeout: 1800
  - id: editor
    stage: 6
    name: "completion offers an added member and writes its use, hover and definition name its block, rename renames every call, and a missing use is a fix"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/an-added-method-is-offered-and-writes-its-use.lspt, --case, tests/lsp/hover/an-added-method-names-its-block.lspt, --case, tests/lsp/definition/an-added-method-goes-to-its-block.lspt, --case, tests/lsp/rename/renaming-an-added-method-renames-every-call.lspt, --case, tests/lsp/actions/a-missing-use-of-a-block-is-a-fix.lspt, --case, tests/lsp/symbols/an-extend-block-is-a-symbol-with-its-members.lspt]
    timeout: 1800
  - id: meta
    stage: 6
    name: "nvs meta --json writes an extend block with its target and members, and nvs agent find names an added member"
    run: [cargo, test, --test, meta, an_extend_block_]
    tests: [an_extend_block_is_written_with_its_target_and_members]
    timeout: 1800
  - id: agent
    stage: 6
    name: "nvs agent find names an added member under its type and its block"
    run: [cargo, test, --lib, agent_find_names_an_added_member]
    tests: [agent_find_names_an_added_member]
    timeout: 1800
  - id: proofs
    stage: 7
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, extend-blocks]
    timeout: 1800
  - id: close
    stage: 8
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, extend-blocks]
    timeout: 900
---

# extend T as Name adds methods, constants, enums, attributes and properties to a type the program does not own

## Why

Settled with the user on 2026-11-24, from a report. A project kept a `DateHelper` class beside
`Core\Time\Date` to add its own date functions, and the same pair beside every `Core` feature it used.
A developer then has to know, for each call, which of the two classes holds it. `E0329` refuses
`class X extends Core\Time\Date` (`rule:core-api/core-classes-are-sealed`, raised at
`crates/nvs-hir/src/hierarchy.rs:869`), and that rule stays: a subclass cannot hide a `Core` member,
every `Core` member that returns a new value returns the `Core` class and drops the subclass, and value
equality across the two classes has no answer.

An `extend` block is the replacement. It adds members to a type the program does not own, and the
type stays what it was. With `[[restrict]]` (`rule:packaging/a-restriction-limits-where-a-declaration-is-used`)
a framework adds its own members and blocks the `Core` ones it does not want, and gives one API.

**The user's principle: `extend` only adds and never changes.** The test for every decision below:
every program that compiled before a block is added behaves exactly the same after it.

## Standing decisions

Each decision says whose call it was. "User" means the user chose it in the session on 2026-11-24;
"mine" means the authoring session decided it by the priority ordering and the user did not object.

### The syntax and the name

- **`extend <Type> as <Name> { … }`** (user). `<Type>` is a class, an interface or an enum, written in
  full or through `use`. `<Name>` is the block's own name in the file's namespace, as a class name is.
  `extend` is a **contextual** word, recognised only at statement start before a type name, through
  `contextual_at` (`crates/nvs-syntax/src/parser/mod.rs:867`), the shape of `at_attribute_decl`
  (`crates/nvs-syntax/src/parser/decl.rs:957`). It is not added to the `keywords!` table
  (`crates/nvs-syntax/src/token.rs:393`), so no identifier breaks (the corpus has no method or variable
  named `extend`). `as` is the existing word.
- **A block is a top-level declaration** (mine). It is refused inside a class body with the code
  `E0233` already gives a nested class (`decl.rs:1414`). It may carry `#[Core\Deprecated]`, and no
  other attribute; its members take attributes as class members do. A doc comment and doc tags work on
  the block and on each member.
- **An autoloaded file declares one block and nothing else** (mine), named as the autoload map finds it,
  so `App\Time\BusinessDays` is `BusinessDays.nvs` under `App\Time`'s directory.
  `rule:programs/one-declaration-per-autoloaded-file` gains `extend` beside class, interface, enum, type
  alias and attribute (`check_file_shape`, `crates/nvs-hir/src/autoload.rs:1449`; `scan_shape`, `:1525`).
  A file reached by `require` may declare blocks as it may declare anything.
- **The word for it is "an `extend` block" and "an added method / property / constant"** (mine).
  "Extension" already means a native `nvs ext` component in the code (`env.symbols.is_extension`,
  `crates/nvs-hir/src/symbol.rs:154`) and on the website. No user-facing text, rule title or code name
  calls this feature an extension.

### Where an added member is visible

- **Only in a file that writes `use <Ns>\<Name>;` for the block, and inside the block itself** (user).
  A group `use` and an alias work. The same namespace is no exception: the compiler finds a block only by
  a name that a `use` gives, because names load lazily through the autoload map
  (`crates/nvs-hir/src/autoload.rs:596`) and nothing else references a block. A call that would resolve
  only through a block the file does not use is the ordinary unknown-member error (`E0405`,
  `report_unknown_member`, `crates/nvs-types/src/expr/members.rs:3083`; `E0309` for `::`), and its help
  names the block and the `use` line when the program's autoload map knows a block that adds the name.
- **A block may not extend a type declared in its own package** (mine). For the application, a type
  declared in the application. The help says to add the member to the type. `Packages::of`
  (`crates/nvs-types/src/package.rs:48`) gives both packages. This keeps one place per member for
  every type a project owns. An own enum takes methods in its body instead (stage 2).

### What a block may add

| Added | On a class | On an interface | On an enum | Decision |
|---|---|---|---|---|
| `public` / `private` instance method | yes | yes | yes, `$this` is the case | user |
| `public` / `private` static method | yes | yes | yes | user |
| constant | yes | yes | yes | user |
| nested `enum` | yes | yes | no, an enum body has no nested enum | user |
| nested `attribute` | yes, used on the type's descendants | yes, used on its implementers | no | user |
| property with a `get` hook (computed) | yes | yes | yes | user, after the correction that hooks exist |
| stored property (with or without hooks) | yes, if the class takes an added slot | no | no | user |

Not allowed, each a compile error naming what to write instead (mine, by the add-never-change test):
a constructor (write a static factory), `protected`, `abstract`, `final`, a `static` property (use a
static property of your own class), `readonly` on an added property, a stored property with no default,
`?` or `lateinit`, `parent`, `static::` (write `self::`), a new enum case, an `implements` or
`extends` clause on the block, and an added member that replaces or overrides anything.

- **`$this` is the extended type, and `self` names it** (mine). A block's body sees only what code
  outside the type sees: `public` members of the type, its own `private` members, and the members of
  other blocks its file uses. `private` on an added member means visible inside that block only.
  Visibility is checked by `check_member_visibility` (`crates/nvs-types/src/expr/members.rs:3042`) with
  the block as the accessing place, never the type.
- **A generic `Core` class is extended with one new name per type parameter, in order** (mine,
  stage 4, replacing "the registry's names"): `extend Core\ObjectMap<K, V> as Index { … }`, where any
  bare names may stand for `class_type_params`' entries. A block reads better with names of its own,
  and the call substitutes positionally either way. Each name lowers to a type variable in the block
  (`Env::extend_type_params`), the body is compiled once with each erased to `Tagged`, and a call
  substitutes the receiver's arguments (`substitute_receiver_args`, `ClassSignature::added_params`).
  A list that does not fit, a block of one instance included, is `E0338`. User classes have no type
  parameters, so this is `Core` only. Novis has no source-declared generics, so an added method
  declares no type parameters of its own.
- **A union call through one block is one direct call** (mine, stage 4). When one block adds the
  method to every class of a union, the checker records one `ExprInfo::Call` of the block's
  function, not an `ExprInfo::UnionMember` per class. Any other mix of blocks, or of a block and a
  declared member, is `E1013` with a note naming each class's block.
- **Names the runtime or an operator calls by name are refused as added names** (mine): `compareTo`
  and every other name a `Core` interface or an operator dispatches by name. Stage 4 lists them from
  `crates/nvs-types/src/expr/operators.rs:976` and the `Core` interfaces' required members, and the
  rule fragment lists them. An added method never satisfies an interface obligation
  (`crates/nvs-types/src/conformance.rs:445`).

### An added member never changes a type

- **Every collision is a compile error naming both places** (mine): an added name equal to a member of
  the type or an ancestor; two blocks in the program adding one name to one type; a class or interface
  in the program that extends or implements the type and declares the name; and a later Novis release
  that adds the name to a `Core` type, which turns the block's member into that error. The descendant
  check runs over the whole compiled program, because `resolve_program`
  (`crates/nvs-hir/src/requires.rs:248`) builds it before any body is checked. So whether a call reaches
  an added member or a declared one never depends on the receiver's static type.
- **Only a statically typed call reaches an added member** (mine). A `mixed` or `object` receiver
  (`ExprInfo::ErasedCall`, `crates/nvs-types/src/expr/calls.rs:255`), a dynamic name (`E0235`),
  `Core\Reflect`, JSON encoding and decoding, `==`, `clone`, the graph copy to a task, the ordering
  operators and every `Core` member never see one. `Reflect::methods()` and `properties()` do not list
  added members, and `ClassInfo::call` of an added name throws as for an unknown member.
- **A union receiver finds an added member only when every type in the union gets it from the same
  block** (mine), which happens through an interface they share. Otherwise the call does not compile,
  and the help says to narrow with `is`. `infer_union_method_call` (`calls.rs:379`) gets the case.
- **A restriction does not leak through a block** (mine). An extend body's place, for
  `allows_at` (`crates/nvs-types/src/restrict.rs:709`), is the block's namespace and the block, never the
  extended type, so a block cannot call a member restricted on its type from inside. A
  `[[restrict]]` entry names an added member by its block, `'App\Time\BusinessDays::isBusinessDay'`,
  and an entry on the extended type does not hold its added members. A package may restrict only its
  own blocks, as it may restrict only its own code.
- **`#[Core\Deprecated]` on an added member or a block warns at each use** (mine), through the same
  resolved-use pass (`crate::deprecated::resolved`).

### Calls compile to direct calls

- **An added method is compiled as a function with the receiver as parameter 0** (mine), exactly as
  `lower_method` builds a method (`crates/nvs-ir/src/lower/mod.rs:1722`, `$this` at `:1759`). Its label is
  `<Block>::<name>`. Every call is `InstKind::Call` with `Callee::Direct`
  (`crates/nvs-ir/src/ir.rs:1204`), never `CallVirtual` or `CoreCall`, because the collision rule makes
  the target unique. An added static method takes the extended type's class descriptor in slot 0 as a
  static method does (`mod.rs:1749`). An added method on an enum takes the case's integer.
  **Cost:** the same as calling a static method of a helper class. No memory per call.
- **The checker records every added call as a resolved call** (`resolved_call`, `calls.rs:~320`),
  because `nvs-ir` panics on a call with no target, and the restriction and deprecation passes read
  only resolved entries. Static members and constants are also recorded in the `nvs-hir` member table
  (`record_member`, `crates/nvs-hir/src/members.rs:~2160`), or `E0309` fires before the checker runs.

- **An added static method takes `0` in slot 0 on every target** (mine, stage 3, replacing the
  class descriptor above). A `Core` class has no descriptor a unit can name, and one convention
  keeps every call direct. Nothing in a block reads the called class, because `static::` is refused
  there (stage 4), and `self::` in a block names the target itself (`called_class_set_at`). For the
  same reason, stage 4 refuses `yield` in an added static method and in a block on an enum.

### Added properties

- **A computed property is a property with a `get` hook and no stored value** (user). It compiles to a
  call of its hook, as on a class (`website/src/content/docs/syntax/classes/property-hooks.mdx`). It is
  allowed on every type a block may extend. A `set`-only or a stored property is the stored case below.
  A class's `PropertyGuard` and `PropertyObserver` never see an added property.
- **A stored added property lives in one hidden slot** (user, replacing side storage after the runtime
  showed it). The slot holds an ordinary array value of the program's added values for that object
  (mine, stage 5: the sweep then frees it with no new code on any free path), `null` until the first
  write of any added property, and released with the object's other slots in
  `dismantle` (`crates/nvs-runtime/src/object.rs:5144`). No other object pays anything, nothing runs on
  any free path, and no address is ever a key, so a reused address never sees old values. The precedent
  is the hidden slot 0 of a revocable class (`docs/rules/classes/revocable-objects.md`).
  - **On a class of the program**, the compiler adds the slot only when a loaded block adds a stored
    property to the class. The slot sits in the class's own part of the layout, so every subclass has it
    at the same index. The class's unit cache key covers the blocks that add properties to it: one
    unit is the whole program, and `program_digest` hashes every file, blocks included.
  - **On a `Core` class**, the registry entry (`CoreClass`, `crates/nvs-stdlib/src/registry.rs:1952`)
    records whether the class reserves the slot, after its declared slots, so no `Core` member's slot
    index moves. Every `Core` class with instances reserves it, except: the value classes `Core\Time`
    compares by value (time-values' six), any class whose instances can be immortal constants (such as
    `Core\Html\Markup`, `object.rs:~262`), and namespace classes with no instances. A test enumerates
    the registry and fails on a class with instances and no recorded decision.
  - **Reading** an added stored property is a load of the slot, a null test and an indexed load. An
    unwritten property reads its default, `null`, or throws for `lateinit` exactly as an unwritten
    `lateinit` property does.
  - **A stored added property has two compiled accessors** (mine, stage 5, by simplicity). `nvs-ir`
    compiles a `get` and a `set` function per property under the labels
    `<Block>::$p::get#entry` / `set#entry` (`signatures::entry_label`, `lower_added_accessors`),
    and the checker records every access as a hooked property's. So
    every read and write form (`+=`, `++`, `??`, `isset`, nullsafe) reaches it through the paths a
    hook already has, with no new case in any of them. They call `nvs_added_has`, `nvs_added_get`
    and `nvs_added_set`. **Cost:** a direct call and one or two helper calls per access, against an
    inline load; inlining the accessors is a later latency change that changes no behaviour.
  - **A stored added property's written hooks wrap its entry** (mine, stage 5). An access calls
    the hook the block writes, and the entry's accessor where it writes none. Inside its own hooks,
    `$this->p` calls the entry's accessors, as a class's hook reads its backing slot. A short
    `set => expr;` calls the entry's `set` accessor. A hooked property takes no default
    (`rule:classes/property-hooks`), so a hooked stored one is `?T` or `lateinit`; giving hooked
    properties a default is a language change for classes too, and not this goal's.
  - **An element write through an added property is `E0478`** (mine, stage 5). Through two calls,
    `$o->tags[] = "x"` copies the whole array on each write, because the read holds a second
    reference. The program reads it into a variable, writes there and assigns it back.
  - **`clone` and the graph copy to a task start the slot empty** (user, "never seen by clone"). JSON,
    `Reflect`, `==` and sessions skip the slot. A revoke releases it with the other slots.
  - **What it spends:** one pointer per instance of each class that takes the slot, and per object
    that has an added value, one array of the program's added properties for its class. Both are charged
    to the request by the global allocator (`crates/nvs-runtime/src/budget.rs:2043`) and freed with the
    object, so memory stays O(in-flight).

### Enums take methods and constants in their own body

- **An enum body declares cases, methods and constants** (user). Properties, `implements` (`E0218`)
  and nested enums stay refused. `E0220` (`decl.rs:2118`) keeps refusing a property and a nested enum,
  and its help names a class for state. `$this` in an enum method is the case, and the method compiles
  to a direct call with the case's integer, so the enum representation stays an integer constant
  (`rule:enums/no-class-machinery` is rewritten, not reversed: no descriptor, no allocation).
  `$case->m()` on a value typed as one case (`Ty::EnumCase`, `crates/nvs-types/src/ty.rs:547`) resolves
  as on the enum.
- **An enum's static method takes a `0` in parameter 0** (mine, stage 2). The enum has no descriptor
  to pass, and one convention keeps every call, `self::` call and method reference direct
  (`lower_method_of`, `crates/nvs-ir/src/lower/mod.rs`). An instance method takes the case's integer
  there, typed `Ty::Enum`. A method reference stores the integer in its callable, and its thunk reads
  it back as `Ty::Enum` with no retain (`crates/nvs-ir/src/lower/anon_fn.rs` `lower_callable`).
- **An enum method that uses `yield` is `E0220`** (mine, stage 2, by simplicity). A generator keeps
  its receiver in a state object typed as an object, and a method returning an array does the same
  job. Lifting it means typing the state's `this` field and `lsb` for an integer receiver.
- **A constant may not take a case's name, `E0304`** (mine, stage 2), because `E::X` reads both.
  `resolve_enum` (`crates/nvs-types/src/enums.rs`) reports it.
- **`static::` in an enum is accepted** (mine, stage 2). Nothing extends an enum, so it names the
  enum, and its calls lower as `self::` calls do.

### The help and the editor

- **`E0329`'s help points at `extend`** (mine): "Write `extend Core\Time\Date as Name { … }` to add
  members to it." `E0220` on a method in an enum body is gone.
- **Completion offers every added member a project block adds** (mine), sorted after the type's own
  members, with the block in its `detail`. Choosing one from a block the file does not use writes the
  `use` line, as completing a class name does. Hover names the block. Definition goes to the block's
  member. Rename of an added member renames every call and the member. References find every call. The
  outline shows a block as a class-like symbol named `<Name>` with detail `extend <Type>`. The file
  letter for a block is `X` (`rule:ide/a-file-shows-what-it-declares`). Member order inside a block is
  the class order (`rule:tooling/class-members-have-one-order`, `crates/nvs-lsp/src/order.rs:443`).
- **`nvs meta --json` writes a block as its own row** with `kind: "extend"`, `target` and `members`
  (mine), and `nvs agent find` indexes each added member under `<Type>::<member>`, its card saying
  which block adds it.

### New error codes

Each new code is the highest plus one of the area file it belongs to
(`crates/nvs-diagnostics/src/code/mod.rs:1`): declaration-shape and target errors in `names.rs`
(E0336 onward), type errors in `types_e10.rs` (E1011 onward: E1009 and E1010 were already taken). The
rule fragments name each code: E0336 target not a type, E0337 own type, E0338 member not addable,
E1011 collision, E1012 a name the runtime calls, E1013 union, E1014 stored property where no slot is.
A target nothing declares is the ordinary undeclared-name error with its `use` fix, not E0336 (mine:
the fix is worth more than a second code). A stored added property is refused only when it has no
default, no `?` type and no `lateinit` (the stage 5 `lateinit` case needs it allowed).

### Declined

- **Opening `Core` classes to `extends`.** `rule:core-api/core-classes-are-sealed` keeps its reasons, and
  its Declined list gains a line pointing here.
- **Retroactive `implements` (`extend Date implements Sortable`).** It changes what `$x is Sortable`
  returns for a value that already exists, and dispatch through the interface would depend on which file
  asks.
- **Program-wide visibility, registered once** (user chose per-file `use`). Reading a file would not
  show where a method comes from, two dependencies adding one name would break an application that
  cannot fix either, and every block change would recompile every file.
- **Side storage keyed by object address** (user chose the slot). Every object free in every program
  would gain a check, and a read would be a hash lookup.
- **Stored properties on the value classes.** A `Core\Time` value is immutable and compares by value,
  every operation returns a new value, and `Core` cannot copy a value it does not know. A wrapper class
  is the design for data that belongs to such a value.
- **Extending a scalar (`extend string`).** Novis has no methods on scalars. `extend Core\Str as Slugs`
  adds static functions where the string functions already are.
- **A specialised block of one generic instance (`extend List<int>`).**

### What stops it

One attempt per item and two sessions per stage. An item that cannot land in one attempt is recorded in
the handoff with what failed, and the stage goes on. If stage 5 finds that a `Core` class cannot take
the slot after its declared slots, because a `Core` subclass extends its layout, that class records
"no slot" with the reason in the registry test, and the rule fragment lists it. If the descendant
collision check in stage 4 cannot see the whole program before bodies are checked, it runs in the link
pass over the loaded classes. It never becomes a per-file check, and the handoff says why.

### Lanes

Stage 1 lands first: every later stage builds on its AST node and rules. Stage 2 (enum bodies) and
stage 3 share `crates/nvs-types/src/expr/members.rs` and `calls.rs`, so stage 2 runs first and stage 3
reuses its enum-receiver path. Stages 4 and 5 both need stage 3, and touch different files except
`signatures.rs`, so they run in order. Stage 6 needs stage 5 for properties in completion, and its
LSP half and its `meta`/`agent` half are disjoint files. Stage 7 needs everything. **Gate:** the goal
`time-values` is reached (`docs/goals/time-values.md` is gone) before stage 5, because stage 5's value
classes are the ones that goal makes compare by value.

## Stages

### 1. the rules are written, and an extend block parses, resolves its target and loads by its name

**Split:** none, one session. Write the rules first, then build to them.

- [x] `docs/rules/classes/an-extend-block-adds-members-to-a-type.md:1` and
  `data/rules/classes/an-extend-block-adds-members-to-a-type.json:1`, `status: designed`,
  `goal: extend-blocks`: the syntax, the name, the file, `use` visibility, the own-package error, the
  table of what may be added, the refused members, `$this`/`self`, and the generic binding. Inserted in
  `data/rules/classes.json` beside `delegation-by-property` (conventions.md § *Where a rule sits*).
- [x] `docs/rules/classes/an-added-member-never-changes-a-type.md:1` and its JSON, `designed`: collisions,
  unions, every dynamic path, names the runtime calls, restrictions, deprecation, the direct call and
  its cost, and the Declined list above.
- [x] `docs/rules/classes/an-added-property-lives-in-one-hidden-slot.md:1` and its JSON, `designed`:
  computed and stored properties, which classes take the slot, clone and task copy, and *What it spends*.
- [x] `docs/rules/enums/an-enum-body-takes-methods-and-constants.md:1` and its JSON, `designed`. Then
  `bun nv rules --render`.
- [x] `crates/nvs-syntax/src/ast.rs:1558`: `StmtKind::ExtendDecl(ExtendDecl)` with `target`,
  `name`, `type_params`, `attrs`, `doc` and `members: Vec<ClassMember>` (`ast.rs:2082`), and the kind
  name beside `ast.rs:2415`.
- [x] `crates/nvs-syntax/src/parser/stmt.rs:354` and `decl.rs:2161`: the contextual `extend` dispatch,
  with and without a leading `#[`; the body through `parse_class_body` (`decl.rs:1284`). The two parser
  tests the check `parse` names, in `crates/nvs-syntax/src/parser/tests/decl.rs:493`, run once by name.
- [x] `crates/nvs-syntax/src/walk.rs:191`, `:585`, `:617`, `crates/nvs-syntax/src/visit.rs:1` and
  `crates/nvs-syntax/src/index.rs:1`: the new kind. `crates/nvs-fmt/src/chain.rs:244`, `blank.rs:120`,
  `doc_tags.rs:82`, `brace.rs:52`, `modifiers.rs:1` and the contextual-word gap through the mechanism
  `attribute` uses (`gap.rs:80`). `cargo test --test layout_coverage` once.
- [x] `editors/vscode/syntaxes/nvs.tmLanguage.json:270` and `:161`: `extend` as a declaration word,
  `as <Name>` as the declared name; then `bun nv render` for `website/config/novis.tmLanguage.json`.
  `crates/nvs-lsp/src/completion.rs:1822`: `extend` offered at statement start
  (`rule:ide/keywords-are-offered-where-the-compiler-accepts-them`).
- [x] `crates/nvs-hir/src/symbol.rs:21`: `SymbolKind::Extend`; `crates/nvs-hir/src/resolve.rs:123`:
  collected under its name; `crates/nvs-hir/src/hierarchy.rs:352`: the target resolved through
  `resolve_ref` (`:579`) and `CoreRoster::holds` (`:161`), never through `seals`.
- [x] `crates/nvs-hir/src/autoload.rs:1449` and `:1525`: an autoloaded file may hold one block named as
  the map found it; `sole_declaration` (`:1505`) returns it. Edit
  `docs/rules/programs/one-declaration-per-autoloaded-file.md:1` and
  `docs/rules/classes/a-type-is-declared-at-file-scope.md:1` to say so.
- [x] The shape errors, with their codes in `crates/nvs-diagnostics/src/code/names.rs:254`'s area: a
  target that is not a class, interface or enum; a target in the block's own package; each refused
  member kind. The three reject cases the check `shape` names, run once by name.

### 2. an enum body takes methods and constants

**Split:** none, one session.

- [x] `crates/nvs-syntax/src/parser/decl.rs:2013` and `:2118`: methods and constants parse in an enum
  body; a property and a nested enum stay `E0220` (`crates/nvs-diagnostics/src/code/rejected_php.rs:118`),
  its help rewritten to name a class for state.
- [x] `crates/nvs-types/src/expr/members.rs:774`: `class_qname_of` takes `Ty::EnumCase`;
  `crates/nvs-types/src/expr/calls.rs:42` and `:557`: an enum's methods resolve with `$this` typed as
  the enum.
  - [x] An enum constant that is no case reads through the class-constant arm of `infer_class_const`.
  - [x] A class-owned enum's members are in the signature table (`collect_owned_enum_members`).
  - [x] Inside a class-owned enum, the owner's enums resolve by bare name (`owned_enum_in_scope`).
- [x] `crates/nvs-ir/src/lower/mod.rs:1127`: enum methods lower as functions whose parameter 0 is the
  case's integer; a call is a direct `InstKind::Call`.
  - [x] `lower_method_of` with an enum receiver; the walk lowers file-scope and class-owned enums.
  - [x] Static and instance calls on an enum pass a `0` or the case, never a descriptor.
  - [x] A method reference's thunk reads an enum receiver as an integer.
- [x] `tests/conformance/enum/a-class-of-your-own-replaces-every-php-enum-member.nvst:1` expects the
  old refusal: rewrite its source to the new shape, never its expected output, and say so in the commit.
- [x] `docs/rules/enums.md:388`'s fragment `docs/rules/enums/no-class-machinery.md:1`: rewritten to
  the body that takes methods and constants, its table at `enums.md:408` updated; the designed enum rule
  of stage 1 flips to `shipped`. The three cases the check `enum-body` names, run once by name.

### 3. added methods, static methods and constants resolve where their block is used, and run as direct calls

**Split:** none, one session.

- [x] `crates/nvs-types/src/signatures.rs:2806`: `resolve_method_ref` falls back to the blocks the
  current file uses, after the type's own ancestors; the file's used blocks come from its `use` list
  (`crates/nvs-hir/src/imports.rs:1`).
- [x] `crates/nvs-types/src/expr/calls.rs:42`, `:88`, `:273`, `:557`, `crates/nvs-types/src/expr/members.rs:77`:
  instance, nullsafe, method-reference and static calls and constants reach an added member and record
  a resolved call. `crates/nvs-hir/src/members.rs:2160`: added statics and constants recorded.
- [x] `crates/nvs-types/src/expr/members.rs:3042`: visibility with the block as the place.
  `members.rs:3083`: the `E0405`/`E0309` help that names the block and its `use`.
- [x] `crates/nvs-ir/src/lower/mod.rs:1722` and `crates/nvs-ir/src/lower/expr.rs:4910`, `:5327`: a block's
  members lower as functions labelled `<Block>::<name>`; calls are direct. The IR test the check
  `direct-call` names, run once by name.
- [x] `crates/nvs-diagnostics/src/code/names.rs:254`: `E0329`'s help points at `extend`.
- [x] The five cases the check `added-calls` names and the two the check `added-visibility` names,
  each run once by name. `another-package` cases write a `--FILE package.toml--` and a file under it.

### 4. an added member never changes a type: collisions, unions, interfaces, generics, nested types, restrictions and every dynamic path

**Split:** an `Explore` subagent lists every name a `Core` interface or an operator dispatches by name
(question, `crates/nvs-types/src/conformance.rs` and `crates/nvs-types/src/expr/operators.rs`,
`file:line` and the name each) before the item that refuses them.

- [x] `crates/nvs-hir/src/hierarchy.rs:424`: the program-wide collision check: type member, other block,
  descendant member, with both places in the error. The three cases the check `collisions` names.
- [x] `crates/nvs-types/src/conformance.rs:445`: an added member discharges no obligation;
  `crates/nvs-types/src/expr/operators.rs:976`: ordering never reads one; the refused-name list in the
  block shape check. The cases the check `dynamic-paths` names.
- [x] `crates/nvs-types/src/expr/calls.rs:379` and `:506`: the union rule. Interfaces: an added member
  on an interface reached on every implementer. `crates/nvs-types/src/expr/args.rs:2629`: a generic
  `Core` block binds `class_type_params`. The three cases the check `reach` names.
- [x] `crates/nvs-hir/src/resolve.rs:157`: a block's nested enum declared as `<Type>::<Enum>`;
  `crates/nvs-types/src/attributes.rs:975`: a block's nested attribute is owned by the extended type,
  and one on a sealed `Core` class does not compile. The two cases the check `nested` names.
- [x] `crates/nvs-types/src/restrict.rs:709` and `crates/nvs-config/src/restrict.rs:45`: the block as
  the place; an entry naming `<Block>::<member>`; an entry on a type not holding added members. Edit
  `docs/rules/packaging/a-restriction-limits-where-a-declaration-is-used.md:13` and `:62`. The two cases
  the check `restrict` names.
- [x] `crates/nvs-stdlib/src/reflect.rs:1940`: confirm no roster lists an added member, and the
  `dynamic-paths` mixed-and-Reflect case pins it.

### 5. added properties: get hooks on every type, stored values in one hidden slot

**Split:** an `Explore` subagent lists every `Core` class in `crates/nvs-stdlib/src/classes.rs:14`
with instance slots, and for each whether it is a `Core\Time` value class, can be an immortal constant,
or is extended by another `Core` class (`file:line` and one line each), before the registry item.

- [x] `crates/nvs-stdlib/src/registry.rs:1952`: the slot decision on `CoreClass`; each class listed;
  the registry test `added_slot_every_core_class_with_instances_has_a_decision`.
  - [x] the decision is a roster by name, `crates/nvs-stdlib/src/added_slot.rs`, not a `CoreClass`
    field (198 literals); `decision(name)` reads it
  - [x] the value classes, `Core\Html\Markup` and the seven immortal `Core\Reflect` descriptions
    take no slot; every other class with instances reserves it
  - [x] the test, run once by name, green
- [x] `crates/nvs-runtime/src/object.rs:466` and `:2519`: the descriptor's added-slot index;
  `crates/nvs-types/src/layout.rs:1`: a program class takes the slot when a loaded block adds a stored
  property to it, in its own part of the layout; the unit cache key covers those blocks.
  - [x] `ClassDesc::added_slot`, found by `ADDED_SLOT_NAME` in `push_class`; `is_hidden_slot`,
    `field_slot` and `field_name` hide it
  - [x] a `Core` class whose decision is `Reserved` has it after its declared slots
    (`instance.rs` `reserves_added_slot`); `set_slot_types` gives it `SlotCheck::Any`
  - [x] `flatten_fields` gives a block's target the slot after its own properties
  - [x] the unit key: one unit is the whole program, keyed by `program_digest`
    (`crates/nvs-cli/src/cache.rs:1610`) over every file, so a block's change rekeys it already
- [x] Reads and writes of an added stored property, its default, `null` and `lateinit`; a `get` hook
  lowered as a call; `crates/nvs-runtime/src/object.rs:5359` (`clone`) and
  `crates/nvs-runtime/src/graph.rs:727` start the slot empty; JSON and `Reflect` skip it.
  - [x] a computed (`get`-only) added property resolves through a used block, on a class, a `Core`
    object and an enum case, and lowers as a call of the block's hook
  - [x] a write to it is `E0787`; a stored added property is `E1014` on every type for now
  - [x] a stored added property reads and writes the slot, with its default, `null` and `lateinit`
  - [x] `clone` and the graph copy start the slot empty; JSON and `Reflect` skip it
  - [x] `refuse_stored` narrowed to an interface, an enum, a `Core` class with no slot and a
    namespace class
  - [x] a stored added property with hooks: a `set` hook, a `get` hook beside a stored value, and
    `$this->p` inside them reading and writing the entry
- [x] The two remaining tests the check `slot` names, beside `object.rs`'s tests, run once by name.
- [x] The five cases the check `properties` names and the two the check `property-refusals` names,
  each run once by name. The `Core` request case uses `--GET`.
  - [x] the two `property-refusals` cases and `an-added-get-hook-is-computed-on-each-read`, green
  - [x] the four other `properties` cases

### 6. the editor, nvs meta and nvs agent know every added member

**Split:** the LSP half (completion, hover, definition, rename, references, symbols, order, the file
letter) and the `meta`/`agent` half go to two writing subagents side by side; their files are
disjoint. The session builds once both end and runs each check once.

- [ ] `crates/nvs-lsp/src/completion.rs:648` and `:4382`: added members from every project block, sorted
  after the type's own, the block in `detail`, the `use` written on accept.
- [ ] `crates/nvs-lsp/src/hover.rs:167`, `crates/nvs-lsp/src/definition.rs:528`,
  `crates/nvs-lsp/src/rename.rs:157`, `crates/nvs-lsp/src/query.rs:76`: hover names the block,
  definition reaches it, rename and references cover every call.
- [ ] `crates/nvs-lsp/src/symbols.rs:76`: a block in the outline; `crates/nvs-lsp/src/order.rs:121`:
  member order in a block; the file letter `X` beside `sole_declaration`'s users. The quick fix that
  writes a missing block `use` from `E0405`'s help.
- [ ] The six `.lspt` cases the check `editor` names, run once.
- [ ] `crates/nvs-cli/src/meta.rs:1105`: the block row; `crates/nvs-cli/src/agent.rs:626`: added members
  indexed. The tests the checks `meta` and `agent` name, run once by name.

### 7. the rules ship, and the reference, the examples, the feature proofs and the website say so

**Split:** the examples, the attack and the bench go to one writing subagent once the rules are
flipped; the session writes the rules, the reference and the website entries.

- [ ] The four rule fragments of stage 1 flip to `shipped`, each `guardedBy` the cases above.
  `docs/rules/core-api/core-classes-are-sealed.md:32`: its Declined list points at `extend`.
  `bun nv rules --render`.
- [ ] `data/reference/lang/50-classes.json:1`: an `extend` block section; the enum chapter's
  record says enums take methods and constants.
- [ ] `docs/examples/lang/classes/extend-blocks/about.md:1`, `details.md` and three examples: methods on
  `Core\Time\Date`, a stored property on a request set by one function and read by another, and
  `[[restrict]]` with a block giving one API. Written to AGENTS.md § *Text an end user reads*, with
  neutral names (`BusinessDays`, `Shop`).
- [ ] The bench and the attack `bun nv proofs --owed --goal extend-blocks` names. The attack tries to
  read a private member through a block, to replace a `Core` method, and to call a restricted member
  from an extend body; each must fail to compile.
- [ ] `website/src/content/docs/syntax/classes/extend-blocks.mdx:1` (`covers:` the new feature id), the
  enum page updated for methods, `website/src/content/docs/syntax/classes/interfaces.mdx:1` linking it.
  `website/src/data/changes.json:1`: `new` for `extend` blocks, `new` for enum methods and constants.
  Each page reread and stamped with `bun nv site --stamp`.
- [ ] `bun nv proofs --owed --goal extend-blocks` names nothing.

### 8. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stage 7 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stage 5 is done. A stored added property runs the hooks its block writes, and its accessors read
and write the entry. The `slot`, `properties` and `property-refusals` checks are green, and the new
case `tests/conformance/class/an-added-stored-property-runs-its-hooks.nvst` is green, each run once.

- **Computed**: `added_property_access` (`crates/nvs-types/src/expr/members.rs:2741`) records
  `ExprInfo::HookedProperty` with the block's `get` label; a write is `E0787` (`assign.rs:1947`).
- **Stored**: the same record with a `set` label too. `extend::record_stored`
  (`crates/nvs-types/src/extend.rs`) records `expr_table::AddedStored` (the `#entry` labels from
  `signatures::entry_label`, index, default, `lateinit`) at the property's name span, and
  `extend::added_index` gives the index from `ClassSignature::added_stored`.
  `nvs_ir::lower::lower_added_accessors` (`crates/nvs-ir/src/lower/mod.rs`) compiles the two
  functions; they call `Helper::AddedHas`, `AddedGet` and `AddedSet` (`nvs_added_*`,
  `crates/nvs-runtime/src/object.rs` after `nvs_clone_guard`). `refuse_stored` reports `E1014`
  only where `has_added_slot` is false.
- **Hooks on a stored one**: `added_property_access` takes `inside_own_hook`; inside the block's own
  hooks it records the `#entry` labels, elsewhere a written hook's label or the `#entry` one.
  `lower_property_hook` lowers a short `set => expr;` of an added property as a call of the
  `set#entry` accessor (`entry_set`).
- **The slot**: `nvs_runtime::ADDED_SLOT_NAME` (`object.rs:1076`), hidden from JSON, bytes,
  Reflect and copies. It holds an array keyed by index; an entry never written has no key, and a
  slot that is not an array (`null`, or an error object's `unset`) has no added values.

### Next

Stage 6, by its `**Split:**` line: two writing subagents side by side, then one build and each
check once.

1. The LSP half: completion of added members (`crates/nvs-lsp/src/completion.rs:648`, `:4382`), and
   `ExtendDecl` sent to `member()` at `completion.rs:3235` for completion inside a block body;
   `HierarchyResolver::written` carries blocks for a file the program has not loaded; hover,
   definition, rename, references, symbols, order, the file letter `X`, and the quick fix for a
   missing block `use`. The six `.lspt` cases the check `editor` names.
2. The `meta`/`agent` half: the block row in `crates/nvs-cli/src/meta.rs:1105`, added members in
   `crates/nvs-cli/src/agent.rs:626`, and the tests the checks `meta` and `agent` name.

Tests that may go red: none. Every test this session wrote was run once, by name, and is green.
`cargo clippy --all-targets -- -D warnings` is clean.

### Backlog

- A computed added property read inside its own `get` hook calls the hook again, and the
  recursion error stops it at run time. A class's hook reads its backing slot there instead.
  Refusing the read at compile time would be clearer. Stage 7's newcomer review may want it.
- `docs/rules/classes/an-added-property-lives-in-one-hidden-slot.md` names `Core\Html\Markup` as
  the immortal example. Stage 7 adds the seven `Core\Reflect` descriptions, which
  `added_slot.rs` also leaves without a slot.

- Stage 7 owes the enum pages: `docs/examples/lang/enums/declaring-an-enum/about.md:10` and
  `details.md:19`, `what-an-enum-does-not-have/details.md` and `an-enum-inside-a-class/details.md`
  still say an enum has no method or constant. `data/proofs/review-baseline.json:5917` still names
  the old example file `06-a-constant-in-an-enum.nvs`; the review restamps it.
  `docs/reference/lang/55-enums.md:215` (generated) still shows example 06 as a refused constant.
- `tests/conformance/enum/a-class-of-your-own-replaces-every-php-enum-member.nvst` now keeps every
  member in the enum. Stage 7 may rename it to match; `guardedBy` lists name it.
- A block's attribute other than `#[Core\Deprecated]` is not refused yet. It belongs with stage 4's
  deprecation item.
- `HierarchyResolver::written` (the editor's graph for an unloaded file) does not carry blocks. Stage
  6 needs it for completion in a file the program has not loaded.
- `check_type_params` accepts any bare name, so `extend ObjectMap<Tag, Item>` binds two new names
  that shadow the classes `Tag` and `Item` inside the block. Refusing a name the block's file
  resolves to a type needs the file's `use` map in `check_block`. Stage 7's newcomer review may
  want it.
- An added enum's `#[Core\Deprecated]` is not read: `crates/nvs-types/src/deprecated.rs:621`
  walks only class and interface bodies. It belongs with the deprecation work.
- A block's enum whose name a `Core` type already owns (`Core\Arr::Order`) is not refused: the
  symbol table wins over the registry's enum table in `crate::lower::owned_enum`. `check_collisions`
  should compare against `env.enums` for a `Core` target.
- Completion inside an `extend` body offers nothing yet: `completion.rs:3235` sends only
  `ClassDecl`/`InterfaceDecl` nodes to `member()`. Stage 6 adds `ExtendDecl`.
