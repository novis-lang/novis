`Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` never return or count a revoked object
(`rule:classes/revocable-objects`). An `ObjectMap` entry goes when its key or its value is revoked. A
running loop skips an entry revoked during the loop. `count()` is always exact. A `Heap` skips a
revoked entry on `pop` and `peek`, and its count drops at the `revoke()`.

**A key is found with `==`.** An `ObjectMap` key and an `ObjectSet` member are found by the same
comparison `==` makes, and hashed from the same slots. So a `Core\Time` value that compares by value
is one key with every equal value built elsewhere, and the same moment as a `DateTime` in two zones is
one key. Every other object is found by identity alone
(`rule:expressions/object-identity-equality`). **What it spends:** a key of one of those six classes
hashes one to four slots in place of its address, and nothing changes for any other key.

**Removal is eager.** Only a revocable class has the hidden slot, the first slot of its root class
(inherited fields come first in the layout), so it has one offset in every class of the tree. It is
null until the object first enters a collection. Then it points to the object's memberships, one entry
per (collection, kind, key), where the kind is map key, map value, set member or heap entry, and a map
value's entry keeps its key.

- Adding a revocable object to one of the three writes one membership. `remove()`, `clear()` and a
  `set()` that replaces a value remove the old one's membership.
- **A membership is written only after the write lands, and removed by identity.** A write the memory
  ceiling refused leaves the collection as it was and links nothing. A revoked value removes a map
  entry only when the value under its key is that object, so it never removes an entry another value
  holds under the same key.
- `revoke()` sets the flag, follows the memberships and removes each entry in O(1). A map or set leaves
  a hole and compacts later, as an array's `unset` does. A `Heap` entry stays in place, and the heap's
  count drops by one.
- The memberships are then freed, and the slot holds the call-site constant of the `revoke()` call.
  The revoked flag tells the site from the memberships pointer, so no bit of the slot is borrowed: a
  site constant has no alignment to spare one. The error reads the class name and that site from the
  object, so the message costs nothing until it is thrown. A second `revoke()` keeps the first site.
- A collection freed during the request clears its members' memberships in the walk that already
  releases its entries. The cycle collector clears them before it frees any object of the cycle, so the
  order of freeing never matters.
- At the request's end nothing is walked (`rule:classes/no-destructors`). The sweep frees the heap
  whole, and the memberships go with it.
- **A revoked object is never added.** `set`, `add` and `push` of one do nothing, as if it were added
  and revoked at once, so no collection ever holds one.
- **A heap's sifts never order a revoked entry.** The heap counts its revoked entries, and one sorts
  before every other, so it rises to the root, where `pop` and `peek` drop it. A sift up compares with
  the nearest ancestor that is not revoked, so a comparator never sees a revoked object.
- **A program with no revocable class pays one load per write.** Defining a revocable class sets a
  process flag, and every membership step is behind it. A loop reads the thread's revoke count at its
  start, and checks an entry for a revoked object only once that count moves.

**Memory.** A 16-byte hidden slot per revocable object. An object in any collection has one 24-byte
list, and each (collection, kind, key) that holds it costs a 32-byte membership. All of it is on the request heap, so the accounting allocator charges it to the request,
and it is freed with the request: O(in-flight), never O(requests served).

**Trade-offs.** A read from a collection costs what a plain array's read costs. Adding a revocable
object costs one membership, and `revoke()` costs O(collections holding it). A class that is not
revocable pays nothing: it has no slot and writes no membership.

**Declined:**
- **Lazy removal with a check on every read.** It makes `count()` O(n) after a revoke.
- **A back-pointer per plain array store.** It is the cost `rule:classes/revocable-objects` declines
  for automatic removal from arrays.
