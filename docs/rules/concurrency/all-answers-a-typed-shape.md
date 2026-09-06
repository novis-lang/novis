`Core\Task::all({...}, {limit?, deadline?}): S` takes a shape literal whose every field is a
zero-argument closure, runs them concurrently, and answers **a shape with the same field names, each
field carrying that field's own declared return type**:

```php
$page = Task::all({
    user:   fn(): User         => Users::load($id),
    orders: fn(): array<Order> => Orders::recent($id, 20),
}, {deadline: 2s});

echo $page->user->name;          // typed User, not mixed
```

That typing is the whole reason the member is worth having. The uniform alternative answers
`array<mixed>` and every call site then pays a cast, which is the untypeable-container failure the
language refuses everywhere else. No ordinary type at that position can say it — the argument's own
type is a shape of opaque `callable`s — so the binding is a type the registry carries for this one
purpose.

`all` over a one-field shape is legal and pointless, and nothing special-cases it. Its subject is a
shape literal and never an array; that is the line between it and `rule:concurrency/map-preserves-keys-and-order`,
and each member refuses the other's subject.
