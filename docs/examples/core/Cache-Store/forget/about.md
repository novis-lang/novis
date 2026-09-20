Removes the value under a key from a cache store.

After `forget` the key returns `null` again, until something writes it. Forgetting a key that has no
value is allowed and does nothing.

`forget` returns nothing, not even whether there was a value. A cached value may be gone at any time
anyway, so an answer here would tell your program nothing it can use.

Each store removes its own values. A key removed from the store of one CPU core is still in the
store of every other core, and in the store your server shares. So `forget` is not a way to make
every request see a change at once.

The store has no way to remove several keys at once. Your program removes them one by one, with the
keys it knows it wrote.

**The examples below** show removing one key first, then removing a group of keys, then removing a
value after the data behind it changed.
