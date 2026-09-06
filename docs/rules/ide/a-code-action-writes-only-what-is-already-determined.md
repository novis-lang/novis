Four code actions **write** rather than fix, and one bound governs all of them: an action may write only
text the type system has already fully determined. It never invents a body, never names a parameter from a
heuristic, and never picks between two possible signatures. Anything needing a choice is a refactoring
the user drives, not an action a light bulb offers.

The four: **implement missing members** — on a class that does not satisfy an interface or abstract
base, insert every missing method and property with the signature copied from the declaration,
`rule:statements/inout-is-the-by-reference-spelling` markers and qualifiers intact, and a body that
throws; **override a method** — a picker over the hierarchy's overridable members, inserting the chosen
signatures with a call to the base where one exists; **declare the function you just called** — on the
unresolved-call diagnostic, insert a declaration whose parameter types are the argument types at the call
and whose return type is the one the context requires, both already computed to produce the diagnostic;
and **narrow an `array<mixed>` to its literal's type**
(`rule:ide/narrow-an-annotation-to-its-literal`), which writes an annotation rather than a declaration
under the same bound.

Applying a generator produces text that compiles, and applying it twice is a no-op.

**Getter/setter generation is refused** under this bound and under `rule:classes/property-observer`:
property hooks mean the pair of methods that action exists to save typing has no reason to be written.
Code snippets are refused for a neighbouring reason — a snippet body is a second copy of a syntactic shape
the grammar already owns, silently stale after a grammar change.
