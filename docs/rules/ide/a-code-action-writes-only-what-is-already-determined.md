Six code actions **write** rather than fix, and one bound governs all of them: an action may write only
text the type system has already fully determined. It never invents a body, never names a parameter from a
heuristic, and never picks between two possible signatures. Anything needing a choice is a refactoring
the user drives, not an action a light bulb offers.

The six: **implement missing members** — on a class that does not satisfy an interface or abstract
base, insert every missing method with the signature copied from the declaration,
`rule:statements/inout-is-the-by-reference-spelling` markers and qualifiers intact, and a body that
throws; **override a method** — one action per overridable member of the hierarchy, inserting that
signature with a call to the base; **declare the method you just called** — on the
unresolved-call diagnostic, insert a method on the receiver's class, since Novis has no free function
(`E0215`, `E0320`), whose parameter types are the argument types at the call and whose return type is the
one the context requires, both already computed to produce the diagnostic; **declare the attribute** —
on the undeclared-name diagnostic of a `#[Name]`, insert `attribute Name on <site> { … }` whose site is
the one the use is on and whose fields are the keys the use writes;
**narrow an `array<mixed>` to its literal's type**
(`rule:ide/narrow-an-annotation-to-its-literal`), which writes an annotation rather than a declaration
under the same bound; and **write the type every caller passes** into a parameter declared bare
`callable` or `mixed`, where every call of the method passes one static type.

Applying a generator produces text that compiles, and applying it twice is a no-op. Implementing
missing members is the most-used action of its kind in PHP editors, and Novis's explicit types make
it mechanical where PHP's make it a guess. A class with it applied passes `nvs check` with only the
inserted `throw`s reachable.

**Getter/setter generation is refused** under this bound and under `rule:classes/property-observer`:
property hooks mean the pair of methods that action exists to save typing has no reason to be written.
Code snippets are refused for a neighbouring reason — a snippet body is a second copy of a syntactic shape
the grammar already owns, silently stale after a grammar change.

**Built:** implement missing members and override a method (`crates/nvs-lsp/src/generate.rs`).
Implement missing members is a `quickfix` on `E0449`, the one diagnostic for a method an interface or
an `abstract` parent leaves without a body; the checker owes no property, so it writes methods. Each
signature is the declaration's own text with each type written the way the class's file names it,
and the body throws a `LogicError`. Override a method is a `refactor.rewrite` action at a cursor
between a class's members, one per method with a body that a class above it declares and that is not
`private`, `final`, `static`, the constructor, or one with an `inout` parameter. Its body calls
`parent::` and returns what that returns, unless the return type is `void` or `never`. Both actions
analyse the text they write and are offered only when it reports no error, which is what makes the
text compile and makes a second application offer nothing: the class then reports no `E0449`, and
declares every method an override would write.

Override a method is also a completion row, the same generator reached by typing
(`crate::completion`'s `overrides`). Where a member of a class is being written — a name, with any of
`public`, `protected` and `function` before it and nothing after it on the line — each method the
action would offer is a row, and accepting it replaces what was written with the whole method. The
rows are offered when the text of every one, written in place of what was typed, reports no error
inside that text: an error elsewhere is the half-written document's own. After any other modifier,
such as `static` or `private`, nothing is offered, since the override would drop it. It costs one
analysis of the entry's graph per completion request at such a cursor in a class that has a method to
override.

Declare the method you just called is a `quickfix` on the `E0405` of `$obj->name(...)`, offered when the
entry document declares the receiver's class. The checker records the argument types and the type the
context expects beside that diagnostic (`nvs_types::expr_table::UnresolvedCall`), and records nothing for
a program that compiles. A literal argument gives its plain type. The method is `private` when the call
is inside the class and `public` otherwise. A parameter is named after a named argument or a variable
argument, and `$argN` otherwise. With no expectation, the return type is `void` where the call starts a
statement and `mixed` elsewhere. The body throws a `LogicError`, and the text is analysed before it is
offered. A static call and a `...` spread get nothing. Applied once, the call resolves, so a second
application finds no diagnostic.

Declare the attribute is a `quickfix` on the `E0303` of a `#[Name]` that nothing declares
(`crates/nvs-lsp/src/generate.rs`). It writes one declaration at file scope, after the `namespace` and
`use` lines in force where the attribute is used, so the name resolves to the one the use writes. The
site is the declaration the use is on: `class`, `method`, `property` or `parameter`; a use on any other
site gets nothing, since no retrieval reads it. Each field is a key the use writes, typed from its value
as written — `bool`, `int`, `float`, `string`, and `array<T>` for a list whose values all have type `T` —
and `string` for any other value. A qualified name, the member form `Owner::Name`, and a file with the
shape an autoloaded file has (`nvs_hir::autoload::sole_declaration`, where a second declaration is
`E0317`) get nothing. The text is analysed before it is offered, so a field `string` does not fit leaves
the action out. Applied once, the name resolves, so a second application finds no diagnostic. No action
adds a missing required field to a use: its value is not determined, and key completion and signature
help show what is missing.

Narrow an `array<mixed>` is a `refactor.rewrite` at the annotation (`crates/nvs-lsp/src/narrow.rs`). The
type it writes is the one the checker recorded for the literal, and the document with the edit applied
must report the diagnostics it reported before. Applied once, the annotation is no longer
`array<mixed>`, so a second application finds nothing to offer.

Write the type every caller passes is a `refactor.rewrite` at the parameter's type
(`crates/nvs-lsp/src/typed_parameter.rs`), never a `quickfix` or under `source.fixAll.nvs`, and no
diagnostic is behind it: a deliberate `mixed` is not wrong. The calls are the reference index's
occurrences of the method, and the type of each argument is the one the checker recorded at a bare
`callable` or `mixed` parameter of a Novis method
(`nvs_types::expr_table::ExprTypeTable::bare_argument`), widened to its base. A callable with a known
signature is written with its parameter and result types, as `callable(int): string`. Nothing is
offered while an index entry waits, when the method is taken as a callable value anywhere, overrides a
method or is overridden, has no call, or has a call that records no type for the parameter, such as one
that leaves it to its default; nor for a variadic or `inout` parameter, or when two calls differ or one
passes `mixed`. The document with the edit applied must report the diagnostics it reported before.
Applied once, the parameter is no longer bare, so a second application finds nothing to offer. **What
it spends:** while compiling, one type for each argument at such a parameter; per request with the
cursor on such a type, one analysis of each calling file and one of the edited document.

Three refactorings need a choice, so none is an action that writes on its own: each is a refactoring
the user drives. Change parameters, move a member and extract an interface are code actions that
carry a command (`nvs.changeParameters`, `nvs.moveMember`, `nvs.extractInterface`,
`crates/nvs-lsp/src/commands.rs`). The extension asks for the new parameter list, the class or the
interface's name in the editor's own input box, prefilled with the current list, the member's class
or the class's name followed by `Contract`, and the server computes the edit from that text with
the engine `nvs code signature`, `nvs code move` and `nvs code extract interface` run. The other
refactorings, rename, inline, delete unused and extract, write only text the program already has,
and are listed in `rule:ide/no-refactoring-introduces-an-alias`.
