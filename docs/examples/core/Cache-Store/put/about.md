Writes a value into a cache store under a key.

`put` copies the value into the store. The copy is complete, so a change to your own variable after
the write does not change what is stored. A key that already has a value is replaced, and the
replacement costs the store the room of one value.

You can give the value a lifetime with the `ttl` option, for example `{ttl: 10m}`. After that time
the key returns `null` again. Without a lifetime the value stays until the store needs the room or
your program removes it.

Every store has a size limit. When a new value does not fit, the store forgets the value written
longest ago. So any key may return `null` at any time, and your program must be able to build the
value again.

**The examples below** show a write and a replacement first, then a value with a lifetime, then one
user's data stored under a key of its own.
