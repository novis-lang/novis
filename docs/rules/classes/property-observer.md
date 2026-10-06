A class observes its own properties by implementing the global interface `PropertyObserver`, whose
members are `onPropertyGet(string $name, mixed $value): void` and
`onPropertySet(string $name, mixed $value): void`. Implementing it once observes *every* declared
property of that class, hooked or not, instead of writing a hook on each one.

`__get` and `__set` are not recognized by name and have no equivalent, because the case they exist
for is gone: an undeclared property is a hard error (`rule:classes/no-dynamic-properties`), so there
is nothing left to fall back onto. What this covers instead is a
cross-cutting observation point for the properties a class really has, which `__get` never saw.

The name is deliberately not `__get`: a magic spelling advertises "the runtime recognizes this name",
which is exactly the ambient behaviour a declared `implements` replaces. Both members return `void`,
so an observer reports and never decides. It costs one ordinary virtual call per access, paid only by
a class that asked for it.
