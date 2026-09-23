- **An `echo` list writes each argument as it reaches it, so a call inside one that prints lands in
  the middle of the line.** `echo "status ", Core\Command::run(), "\n"` printed `status ` and then
  the handler's own output, which reads as the member returning something strange rather than as an
  ordering. Assign a call that can print to a variable first, and echo the variable.
  [until: reviewed 2026-09-21]
