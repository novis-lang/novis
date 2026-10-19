```nvs
function Core\Program::implementingWith<I, T>(string $member = ""): array<{instance: I, attribute: ?T}>;
```

`rule:programs/implementing`'s enumeration joined with `rule:attributes/structural-retrieval`'s retrieval
in one expansion: one row per class the enumeration would instantiate, in its order, each carrying the
instance and what `Core\Attributes::get<T>` answers for that class's own `$member` — the class itself for
an empty name, its method for a method name, its property otherwise joined with the constructor
parameter of that name, so a promoted parameter is read once. A constructor parameter that is not
promoted is no property, and its name is `E0798`. `attribute` is the one attached payload object satisfying `T`, or `null`.

`I` is an interface or a class, selected exactly as `implementing<I>` selects it.

It exists because the two cannot be composed by a program. A retrieval names its target where it is
written, and inside a loop over the enumeration the variable is typed as `I`, so no listed class is
written anywhere; a framework could enumerate its classes or read their attributes, and not both. The
compiler holds both lists at the same moment, and the join is one retrieval fold per class — nothing
about what a retrieval target may be changes, and no reflection table reaches the compiled unit.

The retrieval's refusals are this member's, each naming the class it was made on: a `$member` some
implementor does not declare is `E0798`, two matching payload objects on one class are `E0728`, and a `T` that
is not a shape is `E0729`. A `$member` that is not a string literal names no roster, and every row's
`attribute` is `null`, as the retrieval's computed member folds. The enumeration's own two refusals
hold unchanged.

A row is a shape rather than two parallel arrays because the loop body reads one class's two facts
together. The join is not specific to routes: it serves every attribute a framework reads off the
classes it discovered.

**Memory:** each evaluation makes the allocations `implementing` makes, plus one row object per class
and one payload object per matched attribute, where the call stands. The join itself is paid while
checking, and nothing runs that did not run before.

**Declined:**
- **A retrieval that takes an instance**, `Core\Attributes::get<T>($view, 'render')`, accepted when the
  receiver's static type is one of the enumerated classes. Inside the loop the variable is typed as the
  interface, so no call site of that shape could ever be accepted.
- **A route listing**, `Core\Router::routes()` answering the compiled rows. It removes the duplicated
  path for routes alone, with a new class and several members, and leaves every other attribute where it
  was. Listing a program's routes outside a request is a separate routing question.
- **An interface that requires an attribute**, the compiler checking that every implementor's method
  carries it. That checks the duplication rather than removing it: the path is still written twice.
