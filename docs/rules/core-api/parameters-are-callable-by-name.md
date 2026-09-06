Every parameter of every `Core` member may be written by name at the call site — the `$name` the spec's
signature column writes, and the trailing bag under the one name `options` — under exactly the rules a
user-declared method's parameters follow (`rule:types/arrays`): written order is evaluation order, a name
fills its own slot, a defaulted parameter may be skipped, a positional after a name is refused, and a name
never reaches a variadic tail.

A parameter's **name is compatibility surface**, versioned where its type is, so renaming one is a
breaking change. That is the price of the feature and it is paid deliberately: the spec has published
every parameter name since it was written, so the surface was already public, and a surface a user's own
method has that a `Core` member lacks is one more rule to learn. The name lives once, on the registry row
beside the type, and the reference card looks it up rather than repeating it
(`rule:core-api/reference-card`). Nothing on the request path changes — a `name:` resolves while checking
and the helper ABI is untouched.
