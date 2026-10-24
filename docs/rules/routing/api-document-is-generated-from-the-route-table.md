The OpenAPI 3.1 document is a rendering of the compiler's own route table. Every fact in it was
decided by the front end and is read off a row; the emitter parses, resolves and infers nothing.
A specification generated from the code cannot disagree with the code, which is the failure every
hand-written or annotation-scanned API document eventually has.

What supplies what: the path, verb and `operationId` come from `#[Route]`'s `path`, `method` and
`name` (a shared name takes the verb as a suffix — `rule:routing/a-shared-name-is-one-endpoint-everywhere`);
path parameters from the handler's own parameters, by declared type, which is the binding matching
already makes; query parameters from `#[Query]` parameters, a default making one optional; the
response body from the declared return type, a class in it being a `components.schemas` entry of
its JSON codec's fields (`rule:core-classes/derive-attribute`) and an enum its backing values, which
is what that codec writes; required-ness from definite initialization and
parameter defaults, decided rather than guessed; nullability from `?T` and nothing else; an
enumeration from a closed enum's cases or a set of allowed values; the summary and description from
the declaration's doc comment — first sentence, then the rest. What the types cannot say comes
from `#[Api]` (`rule:attributes/api-adds-and-cannot-contradict`), which may add and may not
contradict.

**A program with no `#[Route]` generates nothing and runs no pass**, and a program the front end
refuses emits no document at all rather than a partial one. A handler returning `mixed` produces a
useless schema, visibly, which is the correct incentive.

The document is only as precise as the handlers' types, and the type system is what makes it precise:
every variable has a declared type, a property is definitely initialized, `?T` is the only
nullability, and an enum is closed, so each maps onto JSON Schema without guessing. A doc comment is
already parsed for tooling, so reading it adds no machinery, but it makes the comment part of a
published artifact.

**Declined:**
- **Reflecting over the route table at run time**, the way annotation-scanning frameworks build their
  document. It is compile-time work done at run time, it needs a cache that goes stale, and it cannot
  make a contradiction a compile error, which is the whole value
  (`rule:attributes/api-adds-and-cannot-contradict`).
- **A hand-written specification the compiler checks the code against.** It makes the document a
  second source of truth kept in sync in the other direction, and a check strong enough to be worth it
  is a much larger feature. Nothing stops a team from diffing a generated document against a
  hand-written one with `rule:routing/api-diff-fails-a-breaking-change`.
- **Generating clients.** The OpenAPI generator ecosystem exists and is specialised per target
  language, so Novis emits the input it reads rather than reimplementing it.
- **GraphQL and gRPC descriptions.** The document is OpenAPI alone; a handler that does not answer JSON
  is an operation with an opaque response
  (`rule:routing/api-document-is-a-deterministic-build-artifact`).
