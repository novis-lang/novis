Ends a digest and returns it.

`finish` returns the digest of everything you gave to `update`, as `bytes`. This is the same value
that `Core\Hash::of` returns for all the pieces joined together. After `finish`, the stream is
closed and frees the pieces it kept.

**Good to know:** a stream gives its digest only once. A second `finish`, or an `update` after it,
throws a `RuntimeError`. For the next digest, start a new stream with `Core\Hash::stream`. This
replaces PHP's `hash_final`.
