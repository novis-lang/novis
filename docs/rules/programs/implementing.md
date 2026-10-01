```php
function Core\Program::implementing<T>(): array<T>;   // T must be an interface type
```

It expands, while compiling, to an array literal of `new` expressions — one per non-abstract class in
the program implementing `T`, **sorted by fully-qualified name**, so the order never depends on
filesystem enumeration. Each such class needs a no-argument constructor; a diagnostic names any that
does not, and dependencies arrive through the interface's own methods instead. Because the expansion is
ordinary `new` evaluated at the call site, the instances are per-request like every other object and
nothing crosses an isolate boundary.

The selector is an interface rather than an attribute because the interface is what gives the loop body
a static type to call through: `object` is opaque, shape types describe data rather than methods, and
`callable` carries no signature.

Answering the query means parsing and collecting declarations from every file under every autoload root
— the one place resolution is not lazy, and the only thing in Novis that makes a compiled unit depend
on a *directory's contents* rather than a file's bytes. It is therefore opt-in: **a program that calls
neither this member, nor `implementingWith`, nor a `Core\Router` member that reads the compile-time
route table performs no scan at all**, and a program calling any of them pays the directory-listing
dependency once, however many it calls. Type checking and lowering stay
lazy regardless — a discovered class nobody calls is never checked past its declaration and never
reaches codegen.
