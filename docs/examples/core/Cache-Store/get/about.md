Reads a value back out of a cache store.

`get` returns what `put` wrote under the key. The value is copied out of the store, so a change to
what you got back does not change the entry, and two reads of one key give you two values.

A key that nothing wrote returns `null`. So does a key whose lifetime has run out, and a key the
store had to forget to make room. Any key may return `null` at any time, so your program must be
able to build the value again.

The return type is `mixed`, because a store holds whatever was written into it. Write `as` after the
call to get the type you need, for example `as string`. Write `??` after the call to give a value to
work with when the key is missing.

**The examples below** show a read and a missing key first, then reading values back with their
types, then the pattern most programs use: read, build when the value is missing, and store.
