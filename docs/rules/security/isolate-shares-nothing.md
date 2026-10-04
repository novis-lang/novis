`spawn script` runs another `.nvs` file — or a static method — as an isolated unit of work inside this
process. It shares exactly two things with its parent: the immutable compiled unit, which is
content-addressed and process-wide, and the parent's resource budget
(`rule:security/isolate-budget-is-the-trees`). Everything else is its own: arena, refcounts, values,
globals, class statics, runtime-defined constants, output buffer, include graph, autoloader state and
config overlay. Open resources are not shared and cannot be passed.

The operand is decided syntactically at the spawn site — a `string` expression, or a
`Class::method(...)` reference written there — and nothing else. A `callable` variable is refused,
because whether *it* captures is not known statically; an anonymous function is refused with a diagnostic
naming the method form.

The isolate joins `spawn` and `spawn worker` rather than introducing a second concurrency vocabulary:
it is awaited like them, and it dies with its parent like them. It is as strong as Novis's request
boundary and no stronger — running code that must be assumed adversarial at the memory-safety level is
what the wasm component tier is for.
