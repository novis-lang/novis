---
looper: 1
title: "every refactoring is one engine, reached from the editor and from nvs code"
summary: "Reached when a type moves to another namespace by rename, by nvs code move and by a file drag in the editor, a property's name compared in its class's property callbacks is renamed with it, a list of renames applies in one call, unused declarations are listed and deleted safely, extract, implement, inline, change parameters, move a member and extract an interface each work from the editor and from nvs code, every refactoring checks its edit before it writes, and the rules, the feature proofs and the website say so."
context: [docs/rules/tooling/nvs-code-answers-about-the-project.md, docs/rules/ide/no-refactoring-introduces-an-alias.md, crates/nvs-cli/src/code.rs, crates/nvs-lsp/src/rename.rs]
stages:
  - id: 1
    title: "every refactoring checks its edit before it writes"
    status: open
    context: [crates/nvs-lsp/src/generate.rs, crates/nvs-lsp/src/query.rs, crates/nvs-lsp/src/index.rs]
  - id: 2
    title: "a property's name, compared as a string in its class's property callbacks, is renamed with it"
    status: open
    context: [docs/rules/classes/late-init-initializes-on-the-first-read.md, docs/rules/classes/property-observer-pipeline.md, crates/nvs-hir/src/interfaces.rs, crates/nvs-lsp/src/index.rs, crates/nvs-cli/tests/code.rs]
  - id: 3
    title: "a type moves to another namespace by rename"
    status: open
    context: [docs/rules/programs/autoload.md, docs/rules/programs/one-declaration-per-autoloaded-file.md, crates/nvs-hir/src/autoload.rs, crates/nvs-hir/src/imports.rs, crates/nvs-cli/tests/code.rs]
  - id: 4
    title: "a list of renames applies in one call"
    status: open
    context: [crates/nvs-cli/tests/code.rs]
  - id: 5
    title: "a moved file or folder renames the types its new path names, from nvs code move and from a drag in the editor"
    status: open
    context: [crates/nvs-hir/src/autoload.rs, crates/nvs-lsp/src/server.rs, crates/nvs-cli/tests/code.rs]
  - id: 6
    title: "unused declarations are listed, greyed out and deleted"
    status: open
    context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/diagnostics.rs, crates/nvs-lsp/src/reach.rs, crates/nvs-lsp/src/actions.rs, docs/rules/ide/five-features-are-one-reference-index.md]
  - id: 7
    title: "extract and implement are nvs code verbs"
    status: open
    context: [crates/nvs-lsp/src/extract.rs, crates/nvs-lsp/src/generate.rs]
  - id: 8
    title: "a local and a method are inlined"
    status: open
    context: [crates/nvs-lsp/src/extract.rs, crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/capabilities.rs]
  - id: 9
    title: "a method's parameters change at every call and in every override, from nvs code"
    status: open
    context: [crates/nvs-lsp/src/typed_parameter.rs]
  - id: 10
    title: "the editor runs a refactoring that needs input through workspace/executeCommand"
    status: open
    context: [docs/rules/ide/the-request-set-is-closed.md, docs/rules/ide/contributions-are-frozen-and-only-ever-added.md, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/tests/handshake.rs, editors/vscode/src/extension.ts, editors/vscode/package.json]
  - id: 11
    title: "a member moves to another class: a static one anywhere, an instance one up or down its hierarchy"
    status: open
    context: [crates/nvs-hir/src/imports.rs]
  - id: 12
    title: "an interface is extracted from a class"
    status: open
    context: [crates/nvs-lsp/src/generate.rs, crates/nvs-hir/src/autoload.rs]
  - id: 13
    title: "the rules, the help, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md, docs/examples/tools/cli/nvs-code/about.md, website/src/content/docs/tools/editor/index.mdx, website/src/content/docs/tools/cli/index.mdx, website/src/data/changes.json]
  - id: 14
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: checked
    stage: 1
    name: "a refactoring's edit is analysed before it is written, an edit that adds an error is refused with that error, and the analysis covers only the changed files and their readers"
    run: [cargo, test, --test, refactor]
    tests: [an_edit_that_adds_an_error_is_refused_and_names_the_error, an_edit_that_adds_no_error_is_returned_unchanged, a_warning_the_edit_adds_does_not_refuse_it, the_check_analyses_the_changed_files_and_their_readers_only]
    timeout: 1800
  - id: callback-names
    stage: 2
    name: "renaming a property renames each string its class's initProperty and property observer hooks compare with the property name, refuses a name a callback uses another way, and a compared name no property has is a warning"
    run: [cargo, test, --test, code]
    tests: [code_rename_of_a_lateinit_property_renames_the_name_init_property_compares, code_rename_of_an_observed_property_renames_the_name_its_hooks_compare, code_rename_renames_a_match_arm_on_the_property_name, code_rename_refuses_a_property_whose_name_a_callback_uses_another_way, code_refs_of_a_lateinit_property_lists_the_compared_name]
    timeout: 1800
  - id: namespace
    stage: 3
    name: "rename to a qualified name moves a type to another namespace, rewrites every use, adds and deletes use lines, moves the file under its root, and refuses what it cannot do"
    run: [cargo, test, --test, code]
    tests: [code_rename_moves_a_type_to_another_namespace_and_rewrites_every_use, code_rename_adds_a_use_line_where_a_bare_name_relied_on_the_old_namespace, code_rename_adds_use_lines_inside_the_moved_file_for_its_old_neighbours, code_rename_deletes_a_use_line_that_names_its_own_namespace, code_rename_creates_the_new_namespace_directory_under_its_root, code_rename_refuses_a_namespace_no_autoload_root_reaches, code_rename_refuses_to_move_a_type_that_shares_its_file]
    timeout: 1800
  - id: batch
    stage: 4
    name: "rename --from applies a list of renames in one checked write, allows a swap, and refuses a conflict whole"
    run: [cargo, test, --test, code]
    tests: [code_rename_from_a_list_applies_every_rename_in_one_write, code_rename_from_a_list_swaps_two_names, code_rename_from_a_list_refuses_two_renames_of_one_site_and_writes_nothing, code_rename_from_a_list_refuses_one_new_name_given_twice]
    timeout: 1800
  - id: moved-namespace
    stage: 5
    name: "moving an autoloaded file or folder renames each type to the name its new path gives, and a move that would lose a type is refused"
    run: [cargo, test, --test, code]
    tests: [code_move_of_an_autoloaded_file_renames_its_type_to_the_new_namespace, code_move_of_a_directory_renames_the_namespace_of_every_type_under_it, code_move_to_a_new_file_name_renames_the_type, code_move_refuses_an_autoloaded_file_no_root_reaches_at_its_new_path, code_move_of_a_required_file_keeps_its_names]
    timeout: 1800
  - id: unused
    stage: 6
    name: "nvs code unused lists every declaration nothing uses, counting routes, tests, overrides, strings and a package's API as uses, and nvs code delete removes one only when nothing uses it"
    run: [cargo, test, --test, code]
    tests: [code_unused_lists_every_declaration_nothing_uses, code_unused_counts_routes_tests_overrides_and_strings_as_uses, code_unused_skips_the_public_api_of_a_package, code_delete_deletes_an_unused_method_with_its_doc_comment, code_delete_deletes_the_file_of_an_unused_type, code_delete_refuses_a_declaration_something_uses_and_writes_nothing]
    timeout: 1800
  - id: extract-implement
    stage: 7
    name: "nvs code extract writes a variable or a method over a range and says why when it cannot, and nvs code implement writes every missing method"
    run: [cargo, test, --test, code]
    tests: [code_extract_variable_writes_a_typed_local, code_extract_method_writes_a_private_method, code_extract_refuses_a_selection_with_an_early_return_and_says_why, code_implement_writes_every_missing_method]
    timeout: 1800
  - id: inline
    stage: 8
    name: "nvs code inline replaces a local's reads or a method's calls with what they are, and refuses every case where that would change what runs"
    run: [cargo, test, --test, code]
    tests: [code_inline_replaces_each_read_of_a_local_and_deletes_it, code_inline_refuses_a_local_written_twice, code_inline_of_a_method_inlines_every_call_and_deletes_it, code_inline_at_one_call_keeps_the_method, code_inline_refuses_an_overridden_method, code_inline_refuses_an_argument_that_is_not_a_variable_or_a_literal]
    timeout: 1800
  - id: signature
    stage: 9
    name: "nvs code signature adds, removes, reorders and retypes parameters at every call and in every override, and refuses a removal or a reorder that changes what runs"
    run: [cargo, test, --test, code]
    tests: [code_signature_adds_a_parameter_with_a_default_and_leaves_callers, code_signature_adds_a_parameter_with_a_value_at_every_call, code_signature_removes_a_parameter_and_its_argument, code_signature_reorders_positional_arguments_and_keeps_named_ones, code_signature_changes_every_method_of_the_override_family, code_signature_refuses_removing_an_argument_that_runs_code]
    timeout: 1800
  - id: editor-commands
    stage: 10
    name: "the server answers workspace/executeCommand for nvs.changeParameters by applying the engine's edit, and answers a refusal with its reason"
    run: [cargo, test, --test, actions]
    tests: [change_parameters_command_applies_the_engines_edit, a_refused_change_parameters_command_answers_the_reason, an_unknown_command_is_refused]
    timeout: 1800
  - id: move-member
    stage: 11
    name: "nvs code move Class::member Class moves a static member anywhere, pulls an instance one up, pushes one down, and refuses a move that breaks a use"
    run: [cargo, test, --test, code]
    tests: [code_move_member_moves_a_static_method_and_rewrites_every_call, code_move_member_moves_a_constant_and_qualifies_its_self_reads, code_move_member_pulls_an_instance_method_up_to_its_parent, code_move_member_pushes_a_method_down_when_nothing_reaches_it_through_the_parent, code_move_member_refuses_a_private_member_its_class_still_uses]
    timeout: 1800
  - id: extract-interface
    stage: 12
    name: "nvs code extract interface writes the interface's file under its root and adds it to the class's implements list"
    run: [cargo, test, --test, code]
    tests: [code_extract_interface_writes_the_interface_file_and_implements_it, code_extract_interface_takes_only_the_methods_named, code_extract_interface_refuses_a_name_that_exists]
    timeout: 1800
  - id: proofs
    stage: 13
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, refactor-commands]
    timeout: 1800
  - id: close
    stage: 14
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, refactor-commands]
    timeout: 900
---

# every refactoring is one engine, reached from the editor and from nvs code

## Why

The user asked for refactoring support at the level of a good IDE, from the command line as well as
from the editor. A review on 2026-11-26 compared what exists with that bar.

What exists: `nvs code rename`, `move` and `order` write; the editor adds extract to variable and to
method, implement missing methods, override, declare the method you called, declare the attribute,
narrow the type, write the caller's type, split and join, and the html template rewrite.

What is wrong or missing, in both clients unless said otherwise:

- **Renaming a property breaks the class's property callbacks without a word.** `initProperty`,
  `onPropertyGet`, `onPropertySet` and `beforePropertySet` get the property's name as a `string`, and
  a class tests it with `$name == 'views'`. Rename changes the declaration and every `->views`, and
  leaves `'views'` as it was. Reproduced on 2026-11-26 with
  `docs/examples/lang/classes/lateinit/02-one-method-for-every-property.nvs`: `nvs code rename
  'Page::$views' pageViews --dry-run` lists every site but line 14. The program still compiles, and at
  run time `initProperty` takes the wrong branch and the read throws a `LogicError`.
- **A type cannot change its namespace.** `rename` changes only the last segment
  (`crates/nvs-lsp/src/rename.rs:14`). `move` only rewrites `require` paths, so moving an autoloaded
  file always breaks the program with `E_AUTOLOAD_FILE_SHAPE`. In the editor this happens on an
  ordinary drag in the Explorer, through `workspace/willRenameFiles`
  (`crates/nvs-lsp/src/server.rs:1837`), with nothing offering a fix.
- **No list of renames in one call.** The `clear-member-names` goal made about 150 renames one call
  at a time, each loading the index again.
- **No unused list and no safe delete.** The editor greys out only private members
  (`crates/nvs-lsp/src/index.rs:1743`), and nothing deletes one.
- **Extract and implement are editor only**, so an agent cannot reach them.
- **No inline, no change of parameters, no move of a member, no extract interface** anywhere.

Each refactoring is written once in `nvs-lsp` and reached from both clients, as rename and extract
already are (`rule:ide/one-server-two-thin-clients`).

## Standing decisions

**One engine, two clients.** Every refactoring is a function in `nvs-lsp` that returns
`rename::Rewrite`s and moves, or a refusal message. `nvs code` writes them through the existing write
path (`crates/nvs-cli/src/code.rs:1492` `write_out`, `:1576` `planned`, `:1653` `committed`). The
server turns them into a `WorkspaceEdit`. No client computes an edit of its own.

**Every write is checked first** (stage 1, the keystone). A new module `crates/nvs-lsp/src/refactor.rs`
applies the rewrites in memory and analyses the changed files and every file that reads a declaration
the edit changes (the index already knows readers, `rule:ide/the-index-is-incremental-and-persistent`).
If any file gains an *error* diagnostic, the refactoring is refused with that diagnostic's message and
position. A new warning does not refuse it. `generate.rs:632` `writes_cleanly` is the model and is
moved onto this. Rename and file move get the check too, in both clients, so the two clients keep one
answer. **Cost:** tooling only, nothing on the request path. Per refactoring, one analysis of the
changed files and their readers; the test counts analysed files, never a clock. A slower rename in a
large project is accepted: a wrong edit is worse than a slow one. **Fallback:** if a whole-reader
analysis is too slow to keep in the editor's rename on the largest corpus (`bun nv` corpus,
over 5 s), the editor rename checks only the changed files, the CLI keeps the full check, and the rule
says so. The check catches a compile error only. A break the compiler cannot see, such as stage 2's,
needs the index to hold the site.

**A property's name in its callbacks** (stage 2). The runtime calls five methods with a property's
name as their first parameter: `initProperty` (`LateInit`), `onPropertyGet` and `onPropertySet`
(`PropertyObserver`), `beforePropertySet` (`PropertyGuard`), and `onPropertyChanged`
(`PropertyChangeObserver`, once the side goal `property-changes` lands it;
`crates/nvs-hir/src/interfaces.rs:95`, `:101`, `:106`).
- **A compared name is a use of the property.** Inside one of these methods, while the name parameter
  is never assigned in the body, a string literal compared with it is recorded by the index walk
  (`crates/nvs-lsp/src/index.rs:3036`) as an occurrence of the property with that name on the
  method's class, inherited ones included. The comparisons are `==`, `===`, `!=` and `!==` on either
  side, and a `match` on the parameter, whose arm conditions are the literals.
- Because it is an index occurrence, rename rewrites it, find references and highlight show it, and
  stage 6's `unused` counts it, with no code of their own
  (`rule:ide/five-features-are-one-reference-index`).
- **Any other use of the name refuses the rename.** Inside such a method, a literal equal to the old
  name that is not a recognized comparison (an array of names, an argument to another call) refuses
  the rename and names the place, as a type's rename already does for a string
  (`rename.rs:14`). So does a name parameter that the body assigns. Strings outside these methods are
  not read: they are not tied to the property.
- **A compared name that no property has is a warning.** A literal compared with the parameter that
  names no property of the class, of a class above it or of a class below it, is a new warning at the
  literal: "`'veiws'` is not a property of `Page`". It is a typo the program would only find at run
  time. The session takes the next free `W` code. It is a warning and never an error, since a class
  below may be declared in another program.
- The site is the literal's text without its quotes, so the rename keeps the quote style.

**No alias, ever.** A clash is a refusal (`rule:ide/no-refactoring-introduces-an-alias`). Every `use`
line written goes to `nvs_hir::import_site` (`crates/nvs-hir/src/imports.rs:83`), fully qualified, and
every touched `use` block is sorted with `nvs fmt`'s sort, so `nvs fmt --check` stays green.

**A type's new namespace** (stage 3). `rename` of a type accepts a qualified new name, `Billing\Cart`
or `\Billing\Invoice`; a bare name keeps today's meaning. In the editor the rename box takes the same
text, and `prepareRename`'s range stays the last segment. What it does:
- Only a type that is its file's sole declaration (`crates/nvs-hir/src/autoload.rs:1512`) changes its
  namespace. Any other type is refused: "move it to its own file first".
- The `namespace` line, every `use` line, every qualified site, every `@see` and `@throws` mention.
- A file in the old namespace that wrote the name bare gets a `use` line. If its short name is already
  taken there, the rename is refused.
- Inside the moved file, each name it wrote bare that resolved through its old namespace gets a `use`
  line, and each `use` line that now names its own namespace is deleted. The same deletion applies in
  every file of the new namespace.
- The file moves to the path the autoload map resolves for the new name (`autoload.rs:596`
  `resolve`), the first root of the longest prefix. Missing directories under that root are created,
  inside the project root only, judged before they are joined. A directory the move empties stays.
- A file that was autoloaded but whose new name no root reaches is refused: the program would lose
  the type. A file reached only by `require` keeps its path and changes only its names.
- When several programs read the file and their maps give different paths, the rename is refused and
  names the programs.

**Several renames in one plan** (stage 4). The engine takes a list of renames and computes all of
them against one starting index. Edits are merged per file. Two renames that edit one byte range are
refused, naming both. The clash check treats a name the same list renames away as free, so a swap
works. One new name given twice is refused. The CLI reads `nvs code rename --from <file>`: one
`<target> <new-name>` per line, blank lines and `#` lines skipped, positions read against the files
as they were. Output, `--dry-run` and `--json` are `rename`'s, and each JSON edit carries the `line`
of the list it came from. The editor gets nothing new from this stage.

**A moved file takes the name its path gives** (stage 5). Under an autoload root the path *is* the
name (`rule:programs/one-declaration-per-autoloaded-file`), so a move that keeps the old name always
breaks the program. `rename::moved` (`rename.rs:272`) therefore asks, for each moved `.nvs` file a
map claims (`autoload.rs:565` `claims`), which name its new path gives. When that differs and the file
has a sole declaration, the stage 4 plan renames each such type, with the file move already decided.
A folder move renames every type under it in one plan. A new file name in the same folder renames the
type's last segment. `nvs code move` refuses an autoloaded file whose new path no root claims. The
editor's `willRenameFiles` cannot refuse a drag, so there it returns only the `require` edits, as
today, and the existing diagnostics name what broke. The rule's Declined item "`move` renaming the type
a moved file declares" is deleted, with the reason above. `move` still creates no directory: the user
types that path.

**What counts as a use** (stage 6). `SymbolIndex::unused` generalizes `unused_private`
(`index.rs:1743`) to types, methods, properties, constants and enum cases. A declaration is used when
the index holds an occurrence of it (stage 2's compared names included), and also when:
- the used-by lens reaches it as a route handler or a test (`crates/nvs-lsp/src/reach.rs`);
- it is a constructor, or a member of an override family where any member is used, or it implements
  an interface or `Core` method the runtime calls;
- it is a class `implementing<T>()` can return;
- it is an enum case and `cases`, `from` or `tryFrom` is called on its enum anywhere;
- its name is written in a string literal of the project or in an `nvs.toml`. A name the tools cannot
  follow counts as a use.

In a project with a `package.toml`, every `public` and `protected` declaration is the package's API
and is never listed. **Safe default** for any other doubtful case: count it as a use.
`nvs code unused [paths] [--json]` prints one line per declaration: position, kind, name. It exits 0.
The editor's dimming (`crates/nvs-lsp/src/diagnostics.rs:434`) greys public and protected
declarations too, from the same function, and stays silent at open scope as it does now.

**What delete removes.** `nvs code delete <name> [--dry-run] [--json]` and the editor's *Delete
unused …* code action (`refactor.rewrite`) at a greyed declaration. It refuses while `refs` lists any
use besides the declaration, including a `@see` to it. It removes the declaration with its `///` run,
its attributes and the blank line it leaves. A type that is its file's sole declaration deletes the
file, and is refused while a `require` names that file. A method that overrides or is overridden is
refused, since deleting it changes which body runs.

**Extract and implement on the command line** (stage 7). `nvs code extract variable <path:l:c-l:c>
[--name <n>]`, `nvs code extract method <path:l:c-l:c> [--name <n>]` and `nvs code implement <Class>`.
The editor offers extract only when it works, so it has no message. `extract.rs` gains a reasoned form
that returns why it cannot, and the CLI prints that reason. The editor's offers do not change. The
default name is the one the editor writes (`extract.rs:635`, `:667`).

**What inline allows** (stage 8). `nvs code inline <name|path:l:c>` and a `refactor.inline` code
action. The new kind is added to `capabilities.rs`.
- **A local:** assigned once, at its declaration, with every read after it in the same body. An
  initializer with no call, `new`, assignment or increment, whose locals are not written between the
  declaration and the last read, replaces every read. Any other initializer is inlined only into one
  read in the next statement, at a place where it runs first (`extract.rs:611` `runs_first`). Each read
  is wrapped in parentheses where precedence needs it. The declaration is deleted.
- **A method:** a name inlines every call and deletes the method; a position on one call inlines that
  call and keeps the method. The body must be one `return <expr>;`, or a `void` body with no `return`
  at a call that is a statement. Refused: overridden or overriding, abstract, taken as a callable value
  (`SymbolIndex::unfollowed`), recursive, a body reading a member the call site cannot see, a `$this`
  in the body where the receiver is not a variable, and an argument that is not a variable or a
  literal (it could run twice or not at all). A default value fills a missing argument.

**What change of parameters allows** (stage 9). `nvs code signature <Class::method> '<parameter list>'
[--value '$name=<expr>']...`. The list is the whole new list, as written in source, and parameters
match by name:
- a kept parameter may change its type, default or position;
- an added one needs a default or a `--value`, which is written at every call; with both, calls keep
  the default;
- a removed one deletes its argument at every call, refused when that argument may run code (a call,
  `new`, an assignment), and refused while the body reads it;
- a reorder moves positional arguments, refused when two moved arguments may both run code, since the
  order they run in would change. Named arguments keep their names.
- Every method of the override family gets the same list, with types written for each file's imports.
  A family member outside the workspace, a call through a callable value and a call the index cannot
  read are refused. Constructors work through their `new` sites, and a kept promoted parameter stays
  promoted. `@param` lines follow: kept lines stay, removed ones go, added ones get none.
- A parameter rename is `rename`'s, never this verb's.

**Input from the editor** (stage 10). Change parameters, move a member and extract an interface need
text from the user. The code action carries a client command (`nvs.changeParameters`,
`nvs.moveMember`, `nvs.extractInterface`). The extension shows VS Code's own input box, prefilled with
the current list or name, and sends `workspace/executeCommand` with that text. The server computes the
edit with the engine and sends it with `workspace/applyEdit`, or answers the refusal as an error
response the extension shows. This adds one standard request to the closed set and three command ids
to the frozen contributions (adding is allowed). `rule:ide/the-request-set-is-closed` and
`crates/nvs-lsp/tests/handshake.rs` grow by exactly that. The input box is the editor's own UI
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). **Fallback:** if `executeCommand`
cannot carry the edit, one `nvs/` request per refactoring, admitted the same way.

**What a member move allows** (stage 11). `nvs code move <Class::member> <Class>`: a `from` that
contains `::` is a member, since no path contains `::`; otherwise it is the file move.
- A static method, static property, constant, or owned enum or type alias moves to any project class.
  Every `Old::member` site becomes `New::member`, with `use` lines where needed. Inside the moved body,
  `self::` and `static::` reads of the old class's members become `Old::`. A private or protected
  member that the old class still uses is refused: change its visibility first.
- An instance member moves only up to a class above it (pull up) or down to a class below it (push
  down). The checked write refuses a push down that something still reaches through the parent type
  or a sibling, and a pull up whose body reads what the parent does not have. A property moved this
  way carries its stage 2 compared names only when the target class has the same callbacks; otherwise
  the move is refused.
- A name the target already has is refused.
- In the editor, a *Move member…* code action on the member name asks for the class through stage 10.

**What extract interface writes** (stage 12). `nvs code extract interface <Class> <Name> [--methods
a,b]`. A bare `<Name>` is in the class's namespace. The default methods are every public, non-static
method but the constructor. It writes a new file at the path the autoload map gives `<Name>`, with
each signature and its `///` run, types written for the new file's imports. It adds `<Name>` to the
class's `implements` list. It changes no parameter type to the interface: that is a choice for the
user. It refuses a name that exists or that no root reaches, since an autoloaded file holds one
declaration. In the editor, *Extract interface…* on the class name asks for the name through stage 10,
with the default methods.

**The verb list.** `nvs code` goes from eleven verbs to seventeen: `unused` reads; `delete`, `extract`,
`implement`, `inline` and `signature` write, beside `rename`, `move` and `order`. Every write verb
takes `--dry-run` and `--json` with `rename`'s output shape. Help text follows AGENTS.md § *Text an
end user reads*, as the existing `Rename` help does (`code.rs:141`).

**Rules.** The home of every verb stays `rule:tooling/nvs-code-answers-about-the-project`. The checked
write gets a new fragment, `docs/rules/ide/a-refactoring-checks-its-edit-before-it-writes.md`, written
in stage 1. The compared property name gets `docs/rules/classes/a-compared-property-name-is-a-use.md`,
written in stage 2, with the warning and its cost. The namespace move gets
`docs/rules/ide/a-type-moves-with-its-namespace.md`, written in stage 3. Each later stage adds its
paragraph to the tooling rule as it lands, and stage 13 rereads all of them.

**Neutral names.** Tests and examples use `Shop`, `Billing`, `Cart`, `Invoice`, `Order`, `Page`.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt goes to `### Backlog` with what blocked it, and the stage goes on.

**Lanes.** Every stage touches `crates/nvs-cli/src/code.rs` or `crates/nvs-cli/tests/code.rs`, and
stages 1 to 5 share `rename.rs` and `index.rs`, so the stages run in order and none is a side goal.
Stage 2 reads `PropertyChangeObserver` only if `property-changes` has landed
(`crates/nvs-hir/src/interfaces.rs` names it); otherwise it covers the other four methods and the
handoff names the fifth for whichever goal lands second. Inside a stage, the CLI tests and the `.lspt`
cases are disjoint files and may go to two writing subagents.

## Stages

### 1. every refactoring checks its edit before it writes

- [ ] `crates/nvs-lsp/src/refactor.rs:1` (new): `checked(workspace, index, rewrites, moves)` applies the
  rewrites in memory, analyses the changed files and their readers, and returns the rewrites or the
  first new error with its position. This stage writes the rule
  `ide/a-refactoring-checks-its-edit-before-it-writes` that states it.
- [ ] `crates/nvs-lsp/src/generate.rs:632`: `writes_cleanly` calls `refactor::checked` or is replaced by
  it; the four generators keep their behaviour.
- [ ] `crates/nvs-lsp/src/query.rs:395` and `crates/nvs-lsp/src/server.rs:1837`: rename and the file
  move pass through `checked` in both clients.
- [ ] `crates/nvs-lsp/tests/refactor.rs:1` (new): the four tests of the check `checked`, the last one
  counting analysed files. Then `cargo test --test refactor` once.
- [ ] `docs/rules/ide/a-refactoring-checks-its-edit-before-it-writes.md:1` (new), with its cost line
  and the fallback of § *Standing decisions*, then `bun nv rules`.

### 2. a property's name, compared as a string in its class's property callbacks, is renamed with it

- [ ] `crates/nvs-lsp/src/index.rs:3036` (`occurrences`): inside the five callbacks of § *A property's
  name in its callbacks*, a literal compared with the unassigned name parameter is an occurrence of
  the property; the callback names come from `crates/nvs-hir/src/interfaces.rs:95`, `:101`, `:106`.
- [ ] `crates/nvs-lsp/src/rename.rs:744` (`members`): a property's rename rewrites those occurrences
  inside the quotes, and refuses a literal equal to the old name that a callback uses any other way,
  or a name parameter the body assigns, naming the place.
- [ ] The warning for a compared name that no property has, in the checker where the class's
  properties are known (an `Explore` subagent finds where `initProperty` is checked, as
  `crates/nvs-types/src/iter_lib.rs:238` registers it), with a diagnostics case beside the existing
  lateinit cases.
- [ ] `crates/nvs-cli/tests/code.rs:719`: the five tests of the check `callback-names`, on a neutral
  `Page` class. Then `cargo test --test code code_rename code_refs` once, as two runs.
- [ ] `tests/lsp/rename/a-lateinit-property-renames-the-name-init-property-compares.lspt:1` and
  `tests/lsp/references/a-lateinit-property-lists-the-name-init-property-compares.lspt:1` (new), then
  `nvs lsp-test tests/lsp/rename/ tests/lsp/references/` once.
- [ ] `docs/rules/classes/a-compared-property-name-is-a-use.md:1` (new), one sentence in
  `docs/rules/classes/late-init-initializes-on-the-first-read.md:1` and
  `docs/rules/classes/property-observer-pipeline.md:1` pointing to it, and the rename list in
  `rename.rs:1`'s module doc. Then `bun nv rules`.

### 3. a type moves to another namespace by rename

- [ ] `crates/nvs-lsp/src/rename.rs:874` (`types`): a qualified new name moves the type per § *A
  type's new namespace*; the sole-declaration and autoload-root refusals; `named_after` (`:1074`) gives
  the new path from `AutoloadMap::resolve` (`crates/nvs-hir/src/autoload.rs:596`).
- [ ] `crates/nvs-lsp/src/rename.rs:874`: the `use` lines added and deleted, at
  `crates/nvs-hir/src/imports.rs:83`, and each touched `use` block sorted by `nvs fmt`'s sort.
- [ ] `crates/nvs-cli/src/code.rs:1116` and `:1492`: the CLI writes the move, creating missing
  directories under the root after judging them; help at `code.rs:141` says a class can take a full
  name.
- [ ] `crates/nvs-cli/tests/code.rs:613`: the seven tests of the check `namespace`. Then `cargo test
  --test code code_rename` once.
- [ ] `tests/lsp/rename/a-class-moves-to-another-namespace.lspt:1` and
  `tests/lsp/rename/a-class-that-shares-its-file-is-refused-a-namespace.lspt:1` (new), then `nvs
  lsp-test tests/lsp/rename/` once.
- [ ] `docs/rules/ide/a-type-moves-with-its-namespace.md:1` (new) and
  `docs/rules/tooling/nvs-code-answers-about-the-project.md:42`: the rename paragraph, then `bun nv
  rules`.

### 4. a list of renames applies in one call

- [ ] `crates/nvs-lsp/src/rename.rs:168`: a plan over several renames, computed against one index,
  merged per file, with the conflict and clash rules of § *Several renames in one plan*; `rename`
  becomes the plan of one.
- [ ] `crates/nvs-cli/src/code.rs:151` and `:285`: `--from <file>`, conflicting with the two
  positional arguments; the `line` field in `--json`.
- [ ] `crates/nvs-cli/tests/code.rs:835`: the four tests of the check `batch`. Then `cargo test --test
  code code_rename_from` once.
- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:42`: one paragraph for `--from`.

### 5. a moved file or folder renames the types its new path names, from nvs code move and from a drag in the editor

- [ ] `crates/nvs-lsp/src/rename.rs:272` (`moved`): each claimed file whose new path gives another
  name joins a stage 4 plan; a folder move renames every type under it.
- [ ] `crates/nvs-cli/src/code.rs:1162` (`judged_move`): the refusal of an autoloaded file no root
  claims at its new path; the `Move` help at `code.rs:177` says the class is renamed with it.
- [ ] `crates/nvs-lsp/src/server.rs:1837`: `willRenameFiles` returns the plan's edits and returns only
  the `require` edits where the plan refuses.
- [ ] `crates/nvs-cli/tests/code.rs:898`: the five tests of the check `moved-namespace`. Then `cargo
  test --test code code_move` once.
- [ ] `tests/lsp/rename/a-dragged-autoloaded-file-takes-its-new-namespace.lspt:1` (new), beside
  `a-renamed-files-require-is-rewritten.lspt`, then `nvs lsp-test tests/lsp/rename/` once.
- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:54` and `:147`: the move paragraph,
  and the Declined item deleted with its reason.

### 6. unused declarations are listed, greyed out and deleted

**Split:** before the classifier is written, one `Explore` subagent lists every way a project
declaration is reached with no index occurrence: route attributes, `nvs.toml` keys that name a class,
`implementing<T>()`, `Core` interfaces the runtime calls (`PropertyObserver` and the rest), schedule
and queue handlers. Answer shape: `file.rs:NN` and one line each.

- [ ] `crates/nvs-lsp/src/index.rs:1743`: `SymbolIndex::unused` per § *What counts as a use*;
  `unused_private` becomes a filter of it.
- [ ] `crates/nvs-lsp/src/diagnostics.rs:434`: dimming covers public and protected declarations at
  workspace scope.
- [ ] `crates/nvs-lsp/src/refactor.rs:1`: `delete` per § *What delete removes*, and its code action in
  `crates/nvs-lsp/src/actions.rs:259`.
- [ ] `crates/nvs-cli/src/code.rs:206`: the `Unused` and `Delete` verbs with their help.
- [ ] `crates/nvs-cli/tests/code.rs:1058`: the six tests of the check `unused`. Then `cargo test
  --test code code_unused code_delete` once, as two runs.
- [ ] `tests/lsp/actions/an-unused-method-offers-its-deletion.lspt:1` and
  `tests/lsp/diagnostics/an-unused-public-method-is-greyed-at-workspace-scope.lspt:1` (new), then `nvs
  lsp-test tests/lsp/` once.
- [ ] `docs/rules/ide/five-features-are-one-reference-index.md:1`: dimming covers public declarations
  at workspace scope; the tooling rule gets the two verbs.

### 7. extract and implement are nvs code verbs

- [ ] `crates/nvs-lsp/src/extract.rs:155`: a reasoned form that returns why a selection cannot be
  extracted; `at` keeps its offers.
- [ ] `crates/nvs-lsp/src/generate.rs:178`: `implement` reachable by class name, through
  `refactor::checked`.
- [ ] `crates/nvs-cli/src/code.rs:206`: the `Extract` verb with `variable` and `method` (stage 12 adds
  `interface`), and the `Implement` verb, each with help. The range parser sits beside
  `split_position` (`code.rs:1462`).
- [ ] `crates/nvs-cli/tests/code.rs:1058`: the four tests of the check `extract-implement`. Then `cargo
  test --test code code_extract code_implement` once, as two runs.
- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:62`: the two verbs.

### 8. a local and a method are inlined

- [ ] `crates/nvs-lsp/src/inline.rs:1` (new): inline a local and a method per § *What inline allows*,
  reusing `extract.rs:611` `runs_first`, through `refactor::checked`.
- [ ] `crates/nvs-lsp/src/actions.rs:259` and `crates/nvs-lsp/src/capabilities.rs:96`: the
  `refactor.inline` action and kind.
- [ ] `crates/nvs-cli/src/code.rs:206`: the `Inline` verb with its help.
- [ ] `crates/nvs-cli/tests/code.rs:1058`: the six tests of the check `inline`. Then `cargo test
  --test code code_inline` once.
- [ ] `tests/lsp/actions/a-local-is-inlined.lspt:1` and `tests/lsp/actions/a-method-is-inlined.lspt:1`
  (new), then `nvs lsp-test tests/lsp/actions/` once.
- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:62` and
  `docs/rules/ide/no-refactoring-introduces-an-alias.md:1`: inline in both lists.

### 9. a method's parameters change at every call and in every override, from nvs code

- [ ] `crates/nvs-lsp/src/signature_change.rs:1` (new): the engine per § *What change of parameters
  allows*, reading the family as `rename.rs:610` `family` does and the call sites as `rename.rs:1194`
  `named_arguments` does, through `refactor::checked`.
- [ ] `crates/nvs-cli/src/code.rs:206`: the `Signature` verb with `--value`, and its help.
- [ ] `crates/nvs-cli/tests/code.rs:1058`: the six tests of the check `signature`. Then `cargo test
  --test code code_signature` once.
- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:62`: the verb.

### 10. the editor runs a refactoring that needs input through workspace/executeCommand

- [ ] `crates/nvs-lsp/src/capabilities.rs:96` and `crates/nvs-lsp/src/server.rs:1727`: the
  `executeCommandProvider` with `nvs.changeParameters`, `nvs.moveMember` and `nvs.extractInterface`;
  the handler sends `workspace/applyEdit` or answers the refusal. `nvs.moveMember` and
  `nvs.extractInterface` answer "not yet" until stages 11 and 12.
- [ ] `crates/nvs-lsp/src/actions.rs:259`: the *Change parameters…* action on a method name, carrying
  the client command with the current list.
- [ ] `editors/vscode/src/extension.ts:1` and `editors/vscode/package.json:1`: the three client
  commands show an input box and send `workspace/executeCommand`.
- [ ] `crates/nvs-lsp/tests/actions.rs:1` and `crates/nvs-lsp/tests/handshake.rs:1`: the three tests of
  the check `editor-commands`, and the declared capabilities grown by exactly one. Then `cargo test
  --test actions` and `cargo test --test handshake` once each.
- [ ] `docs/rules/ide/the-request-set-is-closed.md:1`: `workspace/executeCommand` and its three
  commands.

### 11. a member moves to another class: a static one anywhere, an instance one up or down its hierarchy

- [ ] `crates/nvs-lsp/src/move_member.rs:1` (new): the engine per § *What a member move allows*,
  through `refactor::checked`.
- [ ] `crates/nvs-cli/src/code.rs:1162`: `move` sends a `from` with `::` to the member move; the `Move`
  help names both forms.
- [ ] `crates/nvs-lsp/src/server.rs:1727` and `crates/nvs-lsp/src/actions.rs:259`: `nvs.moveMember`
  runs the engine, and the *Move member…* action is offered on a member name.
- [ ] `crates/nvs-cli/tests/code.rs:1058`: the five tests of the check `move-member`. Then `cargo test
  --test code code_move_member` once.
- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:54`: the member form of `move`.

### 12. an interface is extracted from a class

- [ ] `crates/nvs-lsp/src/extract_interface.rs:1` (new): the engine per § *What extract interface
  writes*, writing signatures as `generate.rs:178` `implement` does, through `refactor::checked`.
- [ ] `crates/nvs-cli/src/code.rs:206`: `extract interface` beside `variable` and `method`.
- [ ] `crates/nvs-lsp/src/server.rs:1727` and `crates/nvs-lsp/src/actions.rs:259`:
  `nvs.extractInterface` runs the engine, and *Extract interface…* is offered on a class name.
- [ ] `crates/nvs-cli/tests/code.rs:1058`: the three tests of the check `extract-interface`. Then
  `cargo test --test code code_extract_interface` once.
- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:62`: the third form of `extract`.

### 13. the rules, the help, the examples, the feature proofs and the website say so

**Split:** once the rules are reread, one writing subagent writes the examples under
`docs/examples/tools/cli/nvs-code/` and a second writes the website sections. Their files are disjoint.
Each brief names AGENTS.md § *Text an end user reads* and the neutral names.

- [ ] `docs/rules/tooling/nvs-code-answers-about-the-project.md:1`: the first paragraph names seventeen
  verbs and which write; each verb's paragraph reread against the code.
- [ ] `docs/rules/ide/no-refactoring-introduces-an-alias.md:1` and
  `docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md:1`: the *Built* lists name
  every new action. Change parameters, move a member and extract an interface are "a refactoring the
  user drives", as that rule already puts it. Then `bun nv rules`.
- [ ] Each `nvs code` help text reread against AGENTS.md § *Text an end user reads*.
- [ ] `docs/examples/tools/cli/nvs-code/about.md:1` and the examples `bun nv proofs --owed --goal
  refactor-commands` names. Foreseen: moving a class to another namespace, renaming from a list,
  finding and deleting unused code, inlining, changing a method's parameters.
- [ ] `docs/examples/lang/classes/lateinit/about.md:1`: one sentence that renaming a property also
  renames the name `initProperty` compares.
- [ ] `website/src/content/docs/tools/editor/index.mdx:37`: the Rename section says a class can take a
  new namespace, a file drag renames the class, and a property's rename reaches the names its
  callbacks compare; new sections for delete unused, inline, change parameters, move member and
  extract interface. Then `bun nv site --stamp` on it.
- [ ] `website/src/content/docs/tools/cli/index.mdx:1`: the `nvs code` section lists the new verbs.
  Then `bun nv site --stamp` on it.
- [ ] `website/src/data/changes.json:1`: `fixed` for a property's rename reaching its callbacks;
  `changed` for `nvs code rename`, `nvs code move` and the editor's rename and file move; `added` for
  the compared-name warning, `nvs code unused`, `delete`, `extract`, `implement`, `inline` and
  `signature`, and for the editor's inline, delete, change parameters, move member and extract
  interface. Then `bun nv site --check stale`, `changes` and `review` green.
- [ ] `bun nv proofs --owed --goal refactor-commands` names nothing.

### 14. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by an interactive session on 2026-11-26, from the user's request for refactoring support at
the level of a good IDE, from the command line and the editor. The user agreed to the list, to the
namespace move as the first new feature, and raised the `lateinit` rename break, which the session
reproduced (§ *Why*). Every other choice in § *Standing decisions* is the session's own, made by the
priority ordering. Nothing of this goal is built. Every anchor was read that day, while
`deeper-debug-records` ran stage 7.

### Next

Stage 1 from its first item, then stage 2. The check `checked` is red until
`crates/nvs-lsp/tests/refactor.rs` exists with its four tests. Every later check names tests that do
not exist yet.

### Backlog
