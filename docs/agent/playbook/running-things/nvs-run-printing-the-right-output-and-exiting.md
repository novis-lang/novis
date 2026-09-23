- **`nvs run` printing the right output and exiting **127** is a heap corruption at teardown**, not
  a missing command: Windows reports a double release that way, with nothing on stderr, and a
  refcount bug is otherwise silent until the WSL valgrind leg catches it. Check `$?` on every
  scratch run rather than reading the output and moving on. [until: reviewed 2026-09-06]
