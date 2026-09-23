- **A valgrind sweep that goes red on *every* fixture at once is one allocation on the startup path,
  and the stack names it in one call.** `valgrind --leak-check=full -q <binary> run
  examples/hello.nvs` prints the allocating frame at the top of the stack. The trap is the shrug: a
  deliberate, documented, once-per-process leak reads like something to accept, but the sweep is
  all-or-nothing and a gate with one known-red fixture is a gate nobody reads — a `Box::leak` that
  only widens a borrow to `&'static` has a scoped form, `nvs_runtime::script::scoped`, so reach for
  that before a suppression. [until: reviewed 2026-09-06]
