Name resolution is `nvs_hir::resolve_ref` and nowhere else. Every resolver in `nvs-hir` and `nvs-types`
already funnels through it — `extends` and `implements`, a `type` alias body, an attribute name, a static
call target, a `require` target, a route or command target — so the resolution rule is that one function's
body and none of its callers.

Keep it that way. A second place that decides what a name means is the state these rules exist to leave.
Because a capability grant is keyed on the namespace enclosing the code that asks, "which namespace does
this name sit in" is an authority question and not only a readability one: a resolution rule with two homes
is a grant lookup with two homes.
