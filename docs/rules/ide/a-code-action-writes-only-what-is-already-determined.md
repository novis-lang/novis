Four code actions **write** rather than fix, and one bound governs all of them: an action may write only
text the type system has already fully determined. It never invents a body, never names a parameter from a
heuristic, and never picks between two possible signatures. Anything needing a choice is a refactoring
the user drives, not an action a light bulb offers.

The four: **implement missing members** — on a class that does not satisfy an interface or abstract
base, insert every missing method with the signature copied from the declaration,
`rule:statements/inout-is-the-by-reference-spelling` markers and qualifiers intact, and a body that
throws; **override a method** — one action per overridable member of the hierarchy, inserting that
signature with a call to the base; **declare the method you just called** — on the
unresolved-call diagnostic, insert a method on the receiver's class, since Novis has no free function
(`E0215`, `E0320`), whose parameter types are the argument types at the call and whose return type is the
one the context requires, both already computed to produce the diagnostic;
and **narrow an `array<mixed>` to its literal's type**
(`rule:ide/narrow-an-annotation-to-its-literal`), which writes an annotation rather than a declaration
under the same bound.

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

Declare the method you just called is a `quickfix` on the `E0405` of `$obj->name(...)`, offered when the
entry document declares the receiver's class. The checker records the argument types and the type the
context expects beside that diagnostic (`nvs_types::expr_table::UnresolvedCall`), and records nothing for
a program that compiles. A literal argument gives its plain type. The method is `private` when the call
is inside the class and `public` otherwise. A parameter is named after a named argument or a variable
argument, and `$argN` otherwise. With no expectation, the return type is `void` where the call starts a
statement and `mixed` elsewhere. The body throws a `LogicError`, and the text is analysed before it is
offered. A static call and a `...` spread get nothing. Applied once, the call resolves, so a second
application finds no diagnostic.

**Missing:** the narrowing action.
