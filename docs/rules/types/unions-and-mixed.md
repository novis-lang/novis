A union permits only the operations valid for *every* member. Reaching a member's own operations means
narrowing (`rule:types/narrowing`), and the spelling that narrows is the `is` operator
(`rule:types/type-test`) — never an `is_int()`-style free function, because there are no free
functions, which is also why `Core\Validate` carries no numeric predicates. *Getting* a scalar out of
a union or out of `mixed` is a different question from testing for one: that is `as T`, which throws,
or `as ?T`, which yields `null`. `as` converts and so accepts what can be converted — `"7" as ?int` is
`7` — while `is` reads the representation and so answers `false` for the same value.

`mixed` is **not checked at all** — that is its entire job. It holds anything, every operation on it
is allowed, and every operation on it is resolved dynamically at runtime through the generic helper
path. That is dynamic typing at dynamic typing's cost, which is the right pressure: the fast path is
the typed one. `Core\Reflect::typeOf` is the one type-introspection member, and it is meaningful only
on a `mixed`, because the checker already knows every other case.

`mixed` is where untrusted input lands, deliberately. `Core\Request::query()`/`::post()`,
`Core\Server::*`, `Core\Script::args()` and `Core\Json::decode`'s result are `array<mixed>`, or return
`mixed` per key, because input genuinely is untyped and pretending otherwise would be a lie in the
type. What makes that safe is that text out of `mixed` is `tainted` (`rule:security/taint-propagation`),
while a number, a `bool` or an enum is proven by its own conversion:

```nvs
uint $id = Core\Request::query('id') as uint;     // throws on "abc", on "-1", on "" — never quietly 0
tainted string $q = Core\Request::query('q') as string;
```

`mixed` never absorbs implicitly in the other direction: `int $n = $m;` where `$m` is `mixed` is a
diagnostic, not a runtime check.
