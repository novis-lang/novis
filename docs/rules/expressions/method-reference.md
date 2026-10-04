A method reference, `Name(...)`, is the only way to take a reference to a declared method or function
member:
`Core\Str::length(...)`, `$user->getName(...)`, `self::helper(...)` (early-bound) and
`static::helper(...)` (late-bound). There is no second reference-taking spelling — no `::ref` form,
and no reuse of `::class`, which stays a class-name-to-string operator unrelated to producing a
callable value.

Every spelling above resolves a member at the reference itself rather than at call time, and the
checker records that resolved target under a variant that cannot be mistaken for an ordinary call.
Two shapes are refused for having no member to resolve: `new C(...)`, because `new` names a class and
a method reference builds a callable carrying a callee rather than an allocation — write
`fn (): C => new C(…)`, which also says which arguments the construction takes; and
`$m->method(...)` on a `mixed` receiver, because a callable outlives the site and there is no class present to read a callee off.

A `callable` may carry its signature. The resolvability this rule is after comes from the reference at
the value's creation site rather than from the static type, so it holds whether or not the slot being
filled declares one.
