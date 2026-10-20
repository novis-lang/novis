`Core\Task::all({...}, {limit?, deadline?}): S` takes a shape whose every field is a zero-argument
callable, runs them concurrently, and answers **a shape with the same field names, each field
carrying that field's own declared return type**:

```nvs
$page = Task::all({
    user:   fn(): User         => Users::load($id),
    orders: fn(): array<Order> => Orders::recent($id, 20),
}, {deadline: 2s});

echo $page->user->name;          // typed User, not mixed
```

That typing is the whole reason the member is worth having. The uniform alternative answers
`array<mixed>` and every call site then pays a cast, which is the untypeable-container failure the
language refuses everywhere else. No writable type says it, because the answer's field names are the
argument's own and the call site is what chooses them — so the parameter is a type the registry
carries for this one purpose, and what each field is worth is read off
`rule:concurrency/an-all-field-answers-what-its-callable-declares`.

Each field's function is called with no arguments. A field whose written signature declares a
parameter is `E0878` at that field, and a shape held in a variable is the plain `E0401`. A field typed
bare `callable` names no parameters, so `all` reads each field's arity before any child starts, and a
function that takes one throws a `LogicError`.

`all` over a one-field shape is legal and pointless, and nothing special-cases it. Its subject is a
shape and never an array; that is the line between it and `rule:concurrency/map-preserves-keys-and-order`,
and each member refuses the other's subject.

**Declined:**
- **One untyped signature answering `array<mixed>`.** It is simpler to declare and puts a cast at every
  call site, which costs more than the type the registry carries for this one parameter.
