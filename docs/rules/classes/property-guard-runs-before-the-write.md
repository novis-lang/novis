A class implementing the global interface `PropertyGuard` has
`beforePropertySet(string $name, mixed $value, array<string> $keys): void` called before every write of
one of its properties, and `beforePropertyUnset(string $name, array<string> $keys): void` before every
`unset` of an element of one, and a throw from either stops the change: nothing is stored, and no
observer runs. The guard runs in the constructor too, a promoted constructor parameter's store
included, and on a hooked property before the `set` hook, with the caller's value. Its place in the order is `rule:classes/property-observer-pipeline`'s.

The guard gets the change, never the resulting array. `$keys` is `[]` for a whole write, and `$value`
is the new value. For an element write `$value` is the new element and `$keys` is its path from the
property, each key as a string: `["0", "1"]` for `$p->cells[0][1] = $v`, and the next index the array
would give for `$p->tags[] = $v`. An increment or a compound assignment on an element passes the
element's new value. `unset($p->tags["a"])` calls `beforePropertyUnset` with the path of the entry it
removes, so `$keys` has at least one key.

Both methods return `void`, so a guard allows or stops a write and never changes its value; a value
that has to be transformed belongs in the property's `set` hook (`rule:classes/property-hooks`). The
class is decided from the object's own class at run time, so a write through `mixed` or
`Core\Reflect` runs the same guard as a compiled one. A `static` property has no receiving instance and
reaches no guard, and an access inside the property's own hooks, or a write that initializes a
`lateinit` property while `initProperty` runs, is plain
(`rule:classes/late-init-initializes-on-the-first-read`).

- **Memory:** an element write in a guarded class builds one small `array<string>` of its keys, freed
  when the guard returns. A class that does not implement `PropertyGuard` pays nothing, at the write or
  anywhere else.

**Declined:**
- **The guard receives the array after the write.** It would have to be computed before the store,
  which copies the array on every element write, and the guard would then have to compare two arrays
  to find the one change it is asked about.
- **The guard returns the value to store.** A second authority over the stored value beside the
  `set` hook raises an ordering question with no answer that is not arbitrary.
- **One interface for the guard, the observer and `LateInit`.** A class pays only for the contract it
  implements.
