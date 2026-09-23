- **A depth-ceiling constant is not evidence that a recursive walk over a document that deep
  survives.** `Core\Xml`'s ceiling is 1024 and `nvs_host::TASK_STACK_SIZE` is 1 MiB, so one native
  frame per node overflowed at about 900 — inside a bound the parser still accepted, turning a
  stated refusal into a crash. Price the frame against that stack rather than against the constant,
  and put the stack on the heap: `crates/nvs-stdlib/src/xml.rs:@instance_of` is the shape.
  [until: reviewed 2026-09-09]
