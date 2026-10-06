A base that declares an element type is checked where it is written. A base whose declared type can
hold no array at all — a scalar, an untested `?array<T>`, a union naming none — is **refused** there
(`E0482`), because a type that has already answered the question does not get to ask it again at run
time.

`mixed` is the one unchecked position (`rule:types/unions-and-mixed`), so a subscript through one
defers not only *which* array is behind the handle but *whether there is one*. The read is answered
from the tag, and the two answers are the element or a catchable throw carrying the same "only an
`array<T>` has elements" wording the refusal above uses — never a `null`. Under a `??` both failures
answer `null` instead, as a `??` read does for any subject.

The **write** side is not deferred. An element write separates a copy-on-write buffer and needs a
holder to write the separated one back through, which a value that is only a tag does not name, so
`$m[$k] = v` keeps the refusal. This is `rule:types/erased-member-access`'s rule one storage kind
along.
