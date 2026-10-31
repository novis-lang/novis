The refactorings are LSP requests — workspace-wide rename, extract to method or variable, and
organize-imports — and the deeper completion is signature help, cross-file symbol search, auto-import,
and inlay hints for `var`-inferred types (`rule:types/var-inference`) and call-site parameter names.

None of them writes an alias. Organize-imports is restricted to reordering and removing unused `use`
statements; auto-import inserts the correct fully-qualified name and nothing else. An editor that
resolved a clash with `use Foo as Bar` would be inventing a spelling the language does not have
(`rule:statements/nothing-gets-a-second-name`), and a second name introduced by tooling is exactly as
much a second name as one typed by hand.

**Built:** signature help, inlay hints, and the three places the server writes a `use` line: an
accepted completion, a pasted type (`rule:ide/a-pasted-type-carries-its-use-line`) and an undeclared
name's fix. Each writes the fully-qualified name at `nvs_hir::import_site` and never an alias
(`crates/nvs-lsp/src/imports.rs`). Rename of a member or a local, and the `require` paths a moved file
changes (`crates/nvs-lsp/src/rename.rs`): a method is renamed with every method that overrides it or
that it overrides, and a new name that some class of that family, above it or below it, already
declares is refused with a message. A local's new name may not be bound already in its body. Neither
is ever resolved with an alias. Rename of a class, an interface, an enum or a type alias declared
outside a class: the index records each type annotation and each class written in front of `::`, and
the rename changes the last segment at every site and in every `use` line. A new name that another
type in the workspace has, or that a file with a site already writes, is refused. A type declared
alone in a file named after it renames that file with it, so an `autoload` still finds it.
Rename of a type alias or an enum a class, an interface or an enum declares
(`rule:types/class-scoped-alias`, `rule:enums/class-scoped-enum`): the index declares it as
`Owner::Name` and records every `Owner::Name` written in a type and every bare `Name` inside the
owner's body, and the rename changes the name after `::` at each. A built-in type name, a name the
owner already gives a constant, an enum case, an enum or an alias, and a name the owner's file
already writes are refused, and nothing moves.
Organize imports, a `source.organizeImports` code action (`crates/nvs-lsp/src/organize.rs`): it deletes
each `use` line the `W1026` warning reports, with that warning's own edit, and sorts the rest with
`nvs fmt`'s sort, so a format after it moves nothing. Cross-file symbol search: `workspace/symbol`
reads the reference index's declarations.
Extract to variable and extract to method, two `refactor.extract` code actions over a selection
(`crates/nvs-lsp/src/extract.rs`). Extract to variable writes a typed local declaration on its own
line above the statement, and replaces the selected expression with the variable. Extract to method
writes a `private` method after the one the selection is in, and replaces the selection with a call to
it. Its parameters are the locals the selection reads, with the types they were declared with. Its
return type is the selected expression's, and `void` for a run of whole statements. An expression's
type is the one the checker gives `var $name = <selection>;` written above its statement, and each
action is offered only once the text it writes analyses with no error, so a document that already
reports one is offered neither.

The boundary of extract to method is one exit and no write that escapes. A selection holding a
`return`, `yield` or `goto`, or a `break` or `continue` with no loop around it inside the selection, is
offered nothing, and so is one that assigns or increments a local declared before it, or passes one as
`inout` or `&`. Writing such a method would mean choosing an `inout` parameter or a result to unpack,
which is a choice and not a fact. A selection inside an anonymous function, or holding one, is refused
too. Extract to variable keeps its own boundary: it is refused where moving the expression above its
statement changes when it runs — a loop condition, an `elseif`, the right side of `&&`, `||` or `??`, a
branch of `?:` or `match`, and after `?->`. Both refuse an assignment's target, an increment's operand,
the operand of `isset`, `empty` and `unset`, and a bare variable. Each analysis is of the editor's own
graph and runs only while a selection covers exactly one expression or run of statements.
