`PropertyGuard` is an interface. A class implements it to check every write of its own properties
before the value is stored.

The class writes two methods:

- `beforePropertySet(string $name, mixed $value, array<string> $keys): void` runs before a write.
  `$value` is the new value. `$keys` is `[]` for a write of the whole property, and the keys of the
  element for a write to one element.
- `beforePropertyUnset(string $name, array<string> $keys): void` runs before `unset` removes an
  element.

If either method throws an error, the write does not happen. Both methods return `void`, so they
cannot change the value. To change a value, use a `set` hook.

**Good to know:** a class that does not implement `PropertyGuard` does no extra work on a write.
