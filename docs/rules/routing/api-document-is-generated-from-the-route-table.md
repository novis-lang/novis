The OpenAPI 3.1 document is a rendering of the compiler's own route table. Every fact in it was
decided by the front end and is read off a row; the emitter parses, resolves and infers nothing.
A specification generated from the code cannot disagree with the code, which is the failure every
hand-written or annotation-scanned API document eventually has.

What supplies what: the path, verb and `operationId` come from `#[Route]`'s `path`, `method` and
`name` (a shared name takes the verb as a suffix — `rule:routing/a-shared-name-is-one-endpoint-everywhere`);
path parameters from the handler's own parameters, by declared type, which is the binding matching
already makes; query parameters from `#[Query]` parameters, a default making one optional; the
response body from the declared return type; required-ness from definite initialization and
parameter defaults, decided rather than guessed; nullability from `?T` and nothing else; an
enumeration from a closed enum's cases or a set of allowed values; the summary and description from
the declaration's doc comment — first sentence, then the rest. What the types cannot say comes
from `#[Api]` (`rule:attributes/api-adds-and-cannot-contradict`), which may add and may not
contradict.

**A program with no `#[Route]` generates nothing and runs no pass**, and a program the front end
refuses emits no document at all rather than a partial one. A handler returning `mixed` produces a
useless schema, visibly, which is the correct incentive.
