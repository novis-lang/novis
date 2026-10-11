```nvs
function Core\Program::implementing<T>(): array<T>;   // T is an interface or a class
```

It expands, while compiling, to an array literal of `new` expressions — one per non-abstract class `C`
in the program for which `$c is T` holds, **sorted by fully-qualified name**, so the order never depends
on filesystem enumeration. `T` may be an interface or a class, abstract or not, and a concrete `T` is in
its own list. Each listed class needs a no-argument constructor; a diagnostic names any that does not,
and a class whose constructor takes arguments is enumerated through `rule:programs/constructors`
instead. Because the expansion is ordinary `new` evaluated at the call site, the instances are
per-request like every other object and nothing crosses an isolate boundary.

The selector is a type rather than an attribute because a type is what gives the loop body something to
call through: `object` is opaque, shape types describe data rather than methods, and bare `callable`
carries no signature. An interface and a class both give it, so every other type argument is refused
(`E0743`). A class selector lets a plugin family share an abstract base, with state and a template
method an interface cannot carry, without declaring an interface beside the base to keep in step
with it. **Membership is `is`**, so no second notion of "subclass" exists: `nvs_hir::implementors`
walks `extends` as well as `implements` and counts a declaration as reaching itself.

Answering the query means parsing and collecting declarations from every file under every autoload root
— the one place resolution is not lazy, and the only thing in Novis that makes a compiled unit depend
on a *directory's contents* rather than a file's bytes. It is therefore opt-in: **a program that calls
no `Core\Program` enumeration member — this one, `implementingWith` or `constructors` — nor a
`Core\Router` or `Core\Request` member whose answer comes from the compile-time route table performs no
scan at all**, and a program calling any of them pays the directory-listing dependency once, however
many it calls. Type checking and lowering stay lazy regardless — a discovered class nobody calls is
never checked past its declaration and never reaches codegen.

- **Memory:** the scan's symbol entries are O(declarations under the autoload roots), held per host and
  only by a program that runs a scan. A class selector is compile-time only and adds nothing to the
  request path.

**Declined:**
- **An `extending<T>()` member.** It is a second member answering the same walk with a different word,
  and a program would have to learn which one applies to an abstract base that also implements an
  interface.
- **A hierarchy reader on `Core\Reflect\ClassInfo`** (`parent`, `interfaces`, `isAbstract`). It lets a
  program filter a list it still cannot obtain without a scan.
