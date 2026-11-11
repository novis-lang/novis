A `foreach ($a as inout T $v)` loop visits the entries `$a` had when the loop began, as the by-value
loop does. The loop takes its own reference to that array when it starts and reads every position from
it, so nothing the body does to `$a` changes which entries are visited or in what order. An entry the
body adds is not visited, and an entry it removes is still visited.

A write to `$v` goes to the variable `$a`, whatever the body has done to it:

- While `$a` still holds the array the loop began with and nothing else shares it, the value replaces
  the entry at the loop's position. No key is built and nothing is looked up.
- Otherwise the write goes to the entry of `$a` with the same key as the entry being visited. The first
  such write into a shared array copies it once, as any array write does (`rule:types/arrays`).
- Where `$a` has no entry with that key, because the body removed it or replaced the array, the write
  throws a `LogicError` that names the key and says it was removed during the loop. The program can
  catch it, and `$a` is left as it was.

So the body may add entries, remove entries and assign `$a`, and the loop runs to its end unless it
writes `$v` after its own entry is gone. One status-returning runtime helper does the write, so the
compiled loop has no position that can name a missing entry.

**What it spends:** one retain and one release of the array per loop, and one helper call per write
to `$v`, which replaces a key allocation and a hashed lookup. Each write carries one cold landing block
in code size. A body that adds or removes entries copies the array once, as it does under a by-value
loop: O(entries) per loop, and twice the array's bytes while the loop runs. A loop that only writes
`$v` copies nothing.

**Declined:**
- **A compile error for changing the subject inside the loop.** It removes a working idiom, cleaning a
  list while walking it, and needs a flow analysis the checker does not run for any other rule.
- **A position that follows the array's own changes.** It needs a counter or a back pointer on every
  array, paid by every array in the program for the sake of one loop shape.
- **Walking the subject variable's live binding.** Removing entries moves positions, so a write could
  name an entry that is not there, and the runtime aborted the process on it.
