`Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` never return or count a revoked object
(`rule:classes/revocable-objects`). An `ObjectMap` entry goes when its key or its value is revoked. A
running loop skips an entry revoked during the loop. `count()` is always exact. A `Heap` skips a
revoked entry on `pop` and `peek`, and its count drops at the `revoke()`.

**Removal is eager.** Only a revocable class has the hidden slot, the first slot of its root class
(inherited fields come first in the layout), so it has one offset in every class of the tree. It is
null until the object first enters a collection. Then it points to the object's memberships, one entry
per (collection, kind, key), where the kind is map key, map value, set member or heap entry, and a map
value's entry keeps its key.

- Adding a revocable object to one of the three writes one membership. `remove()`, `clear()` and a
  `set()` that replaces a value remove the old one's membership.
- `revoke()` sets the flag, follows the memberships and removes each entry in O(1). A map or set leaves
  a hole and compacts later, as an array's `unset` does. A `Heap` entry stays in place, and the heap's
  count drops by one.
- The memberships are then freed, and the slot holds the call-site constant of the `revoke()` call,
  with its low bit set to tell it from a pointer. The error reads the class name and that site from the
  object, so the message costs nothing until it is thrown. A second `revoke()` keeps the first site.
- A collection freed during the request clears its members' memberships in the walk that already
  releases its entries. The cycle collector clears them before it frees any object of the cycle, so the
  order of freeing never matters.
- At the request's end nothing is walked (`rule:classes/no-destructors`). The sweep frees the heap
  whole, and the memberships go with it.

**Memory.** 8 bytes per revocable object, and about 16 to 24 bytes per (object, collection)
membership. All of it is on the request heap, so the accounting allocator charges it to the request,
and it is freed with the request: O(in-flight), never O(requests served).

**Trade-offs.** A read from a collection costs what a plain array's read costs. Adding a revocable
object costs one membership, and `revoke()` costs O(collections holding it). A class that is not
revocable pays nothing: it has no slot and writes no membership.

**Declined:**
- **Lazy removal with a check on every read.** It makes `count()` O(n) after a revoke.
- **A back-pointer per plain array store.** It is the cost `rule:classes/revocable-objects` declines
  for automatic removal from arrays.
