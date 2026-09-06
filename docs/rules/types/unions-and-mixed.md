A union permits only the operations valid for *every* member. Reaching a member's own operations means
narrowing (`rule:types/narrowing`), and there is no `is_int()`-style predicate to narrow with, because
there are no free functions. Getting a scalar out of a union or out of `mixed` is `as T`, which
throws, or `as ?T`, which yields `null` — deliberately the same reviewable spelling either way, which
is why `Core\Validate` carries no numeric predicates.

`mixed` is **not checked at all** — that is its entire job. It holds anything, every operation on it
is allowed, and every operation on it is resolved dynamically at runtime through the generic helper
path. That is PHP's semantics, exactly, at PHP's cost, which is the right pressure: the fast path is
the typed one. `Core\Reflect::typeOf` is the one type-introspection member, and it is meaningful only
on a `mixed`, because the checker already knows every other case.

`mixed` is where untrusted input lands, deliberately. `Core\Request::query()`/`::post()`,
`Core\Server::*`, `Core\Script::args()` and `Core\Json::decode`'s result are `array<mixed>`, or return
`mixed` per key, because input genuinely is untyped and pretending otherwise would be a lie in the
type:

```php
uint $id = Core\Request::query('id') as uint;     // throws on "abc", on "-1", on "" — never quietly 0
```

`mixed` never absorbs implicitly in the other direction: `int $n = $m;` where `$m` is `mixed` is a
diagnostic, not a runtime check.
