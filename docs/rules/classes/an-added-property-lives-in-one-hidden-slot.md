An added property is either computed, a property with a `get` hook and no stored value, or stored,
and a stored one lives in one hidden slot of its object that is `null` until the first write of any
added property. A computed property is allowed on every type a block may extend, and compiles to a call
of its hook, as a hooked property of a class does (`rule:classes/property-hooks`). A `set`-only or a
stored property is the stored case. A class's `PropertyGuard` and `PropertyObserver` never see an added
property.

**The slot.** It holds a pointer to a small array of the program's added values for that object, and
is released with the object's other slots when the object is freed. No other object pays anything,
nothing runs on any free path, and no address is ever a key, so a reused address never sees old
values. The precedent is the hidden slot 0 of a revocable class (`rule:classes/revocable-objects`).

- **On a class of the program**, the compiler adds the slot only when a loaded block adds a stored
  property to the class. The slot sits in the class's own part of the layout, so every subclass has it
  at the same index. The class's unit cache key covers the blocks that add properties to it, as
  `rule:config/the-extension-set-is-in-every-unit-key` covers the extension set.
- **On a `Core` class**, the registry records whether the class reserves the slot, after its declared
  slots, so no `Core` member's slot index moves. Every `Core` class with instances reserves it,
  except the value classes `Core\Time` compares by value, a class whose instances can be immortal
  constants, such as `Core\Html\Markup`, and a namespace class with no instances. A test enumerates the
  registry and fails on a class with instances and no recorded decision. A `Core` class whose layout a
  `Core` subclass extends records "no slot" with its reason, and this fragment lists it.

A stored property added to a class that takes no slot, to an interface or to an enum is **`E1014`**,
whose help names a wrapper class of your own for data that belongs to such a value.

**Reading and writing.** The compiler gives each stored property a `get` and a `set` accessor, and
every read and write form calls one of them directly, as it calls a hook. The `get` accessor loads the
slot and the property's entry in its array. An entry's index counts the added properties of the
class's ancestors first, root first, then the class's own: its blocks sorted by name, and each block's
properties in declaration order. So an index holds for every subclass, as a slot index does. An
unwritten property reads its default, or `null` for a `?` type with none. An unwritten `lateinit`
property throws, as an unwritten `lateinit` property of a class does. `readonly` is refused on an added
property, and so is a stored one with none of the three
(`rule:classes/an-extend-block-adds-members-to-a-type`). An element write through an added property,
`$order->tags[] = "x"`, is `E0478`: it would copy the whole array on each write, so the program reads
the array into a variable, writes there and assigns it back.

**Copies.** `clone` and the graph copy to a task start the slot empty. JSON, `Reflect`, `==` and the
session store skip it, and a revoke releases it with the other slots.

**What it spends:** one pointer per instance of each class that takes the slot, and, per object that
has an added value, one array of the program's added properties for its class. Both are charged to the
request by the global allocator and freed with the object, so memory stays O(in-flight).

**Declined:**
- **Side storage keyed by object address.** Every object free in every program would gain a check, a
  read would be a hash lookup, and a reused address would need its entry cleared first.
- **Stored properties on the value classes.** A `Core\Time` value is immutable and compares by value,
  every operation returns a new value, and `Core` cannot copy a value it does not know. A wrapper class
  is the design for data that belongs to such a value.
- **Copying added values with `clone`.** `clone` copies what the class declares, and a block is not
  part of the class.
