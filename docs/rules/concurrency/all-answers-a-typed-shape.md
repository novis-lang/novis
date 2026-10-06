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

`all` over a one-field shape is legal and pointless, and nothing special-cases it. Its subject is a
shape and never an array; that is the line between it and `rule:concurrency/map-preserves-keys-and-order`,
and each member refuses the other's subject.
