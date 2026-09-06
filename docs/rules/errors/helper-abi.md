Because nothing may unwind through a JIT frame, a runtime helper is declared `extern "C"` — never
`extern "C-unwind"` — and wraps its body in `catch_unwind`, turning a panic into `FATAL` with the
message recorded in `Ctx`. The `nvs_helper!` macro generates the wrapper, so it cannot be forgotten
one helper at a time, and `catch_unwind` costs nothing when no panic occurs.

`panic = "unwind"` is therefore load-bearing rather than a preference, and is set explicitly in
every workspace profile: `panic = "abort"` would turn every containable runtime bug into a process
kill, which is request isolation lost.

The wrapper does not stop at the helper. Code with no request beneath it — the accept loop, the
HTTP reader, the compiled-unit cache index — runs outside `nvs_helper!`, so a worker task's own
root carries a `catch_unwind` as well. And a panic raised while a panic is unwinding aborts the
process whatever the profile says, so nothing on a teardown path may panic and teardown does not
recurse.
