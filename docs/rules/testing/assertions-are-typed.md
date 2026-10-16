The assertion surface is static members on `Core\Test`, each taking its **subject first**:
`assertEquals($actual, $expected)`, with one trailing options shape, nothing mutating and failure
throwing. Both operands of an equality member are one type variable, so comparing an `int` against a
`string` does not compile. Because reversing the two is the commonest mistake in the ecosystem this
language is migrated from, every failure report labels the sides by name rather than by position, so
a reversed call still reads correctly.

Three equality members, and which one was asked for is visible at the call site. `assertSame` is
identity. `assertEquals` compares values — scalars natively, objects only through `Comparable`.
`assertEqualsDeep` is an explicit structural walk of properties, arrays and shapes, with a diff.
`assertEquals` on an object with no `compareTo` names `assertEqualsDeep` rather than falling back to
a property walk: a comparison that quietly changes meaning when a class gains a field is a bug found
years later.

The roster is wide on purpose. Ranking the right member by the subject's type at the call site is
the language server's job, and not a reason to reshape the API into a chain.

**Declined:**
- **A fluent matcher chain** (`Core\Test::that($actual)->equals($expected)`). A chain's one
  advantage over a static roster is matchers scoped to the subject's type, and that needs members
  that exist only at some instantiations of a generic class, which Novis does not have. Without it
  every matcher is declared for every subject, so `that(5)->startsWith("x")` checks the prefix and
  not the subject: the chain type-checks strictly less than the roster, and also costs a builder and
  a second shape unlike every other `Core` member. Only member specialization would reopen it, never
  the roster's size.
- **Static members for scalars beside a per-kind assertion class for collections.** It does scope
  its matchers correctly, and it buys not repeating the subject across consecutive lines at the cost
  of two ways to assert about an array and a boundary between them to defend forever. Each repeated
  line also carries its own failure location.
