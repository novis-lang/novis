```
type         := qualified
qualified    := ('secret')? ('tainted')? union
union        := intersection ('|' intersection)*
intersection := atom ('&' atom)*  |  '(' union ')'        // DNF
atom         := 'null' | 'bool' | 'int' | 'uint' | 'float' | 'decimal'
              | 'string' | 'bytes'
              | 'array' | 'array' '<' type '>'
              | 'class' '<' Name '>'
              | 'property' '<' Name '>'
              | 'object' | 'mixed' | 'void' | 'never' | 'true' | 'false'
              | 'iterable' | 'callable' | 'self' | 'static' | 'parent'
              | 'callable' '(' (type (',' type)*)? ')' ':' type
              | StringLiteral | IntLiteral
              | '{' field (',' field)* '}'
              | Name
              | '?' atom                                  // sugar for atom|null
field        := identifier ':' type
```

Unions are canonicalised — flattened, de-duplicated, order-insensitive — so `int|string` and
`string|int|int` are one type. `array` with no argument is exactly `array<mixed>`. `void` and `never`
are return-only: either on a parameter, a local or a property is `E0742`, since no value has it. `array<T>` is parsed **only in type position**, where a `<` is unambiguously a
type-argument list; two expression positions also admit one, a call site's own `<...>` and a `new`
target's, both settled by a checkpointed trial parse that commits only when the list parses cleanly
and a `(` follows, so `new Foo < $x` stays a comparison. Only a compiler-owned generic declaration may
carry one. `class<T>` is read in type position only too, and adds no ambiguity: `class` is already a
keyword and a class declaration is `class Name`, never `class <`. Its argument closes through the same
`>`-splitting close `array<T>` uses, so `array<class<Animal>>` parses, and `property<T>` reads the same
way. `property` is a keyword in type position only.

`Name` covers four kinds of atom told apart by resolution: a class or interface name, an enum's name,
an enum case (`rule:types/enum-case-type`), and a `type` alias (`rule:types/type-alias`). A class name
carries concrete arguments only where it names a compiler-owned generic interface — `Iterator<User>`
(`rule:iteration/concrete-generic-implements`) — and nowhere else.

`self`, `static` and `parent` are **whole atoms**: a `::` after one in type position is `E0135`. Every
member such a `::` could reach — a `type` alias, a class constant, an enum case — is spelled with the
owner's own name from anywhere, and an alias additionally with its bare name inside the body that
declares it (`rule:types/class-scoped-alias` § *Two spellings and no third*), so the keyword form would
be a second name for each of them, and `static::` could only ever mean the lexical class the written
name already says. The refusal recovers as `mixed`, which is what keeps one misspelt type to one error.

**There is no `resource` type.** A host handle is an ordinary object with an explicit `close()`.
