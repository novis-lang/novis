An allocation that would carry a request past `[limits] memory` is refused **before** the block is
taken, and the refusal is a complete no-op. A published flag bounds a *loop* of allocations, because
there is a next one to stop; it cannot bound a single allocation, because there is none. So the
ceiling is asked twice: once behind every growing allocation, where crossing it raises the memory bit
in the word compiled code polls, and once in front of a value allocation, where the size is known and
the caller is ours.

**The refusal is not in the global allocator.** A null there reaches a `Vec`'s or `Box`'s own path
and `handle_alloc_error`, which aborts the process and every request on it — worse than the breach.
It sits one level up, in the allocators that make a Novis `string`, `array` or object, and the
aborting wrappers beside them do not exist: the fallible constructor is the only way to make one.

**A primitive that carries no `ctx` refuses by returning a degenerate value, and acquires no status
channel to say so.** `nvs_str_append` answers its target unchanged, which exactly balances the one
reference it consumes; `nvs_str_concat` and `nvs_str_concat_n` answer the immortal empty string,
whose release is already a no-op; `nvs_array_set` and `nvs_array_set_index` answer their array
unchanged, having released the key and value they were handed. This is sound because **the request is
already dead**: the refusal is recorded and the memory bit published before the value is returned, so
the program runs only to its next poll, and in that window it can build wrong values and compare them
and do nothing else. It can write no output, reach no `Core` member and touch nothing durable,
because each of those passes `run_helper`, which asks the ceiling ahead of the body and reports the
breach instead of running it.

**Complete no-op means the refused write does none of the work of the write.** In particular it does
not separate a shared array: `foreach` walks the snapshot it started on, and a refused write that
separated without writing — or wrote into the shared original — is the one way a refusal reaches a
live cursor's entry expectation. It also holds no partial allocation and leaves no half-grown buffer.

The refusal is sticky for the rest of the request: a request refused once is over, and nothing it
does afterwards brings it back under a ceiling it never held the bytes against. Objects are outside
the pre-check by design — an object's allocation is sized by its class, so no program drives one
unbounded — and they stay bounded by the published flag.
