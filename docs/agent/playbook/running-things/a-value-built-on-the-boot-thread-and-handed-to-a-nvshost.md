- **A value built on the boot thread and handed to a `nvs_host::Worker` must be `Send`, and
  `nvs_server::arm`'s `Armed` is not.** It holds an `Rc<Cell<usize>>`, so a `[[schedule]]` roster
  armed where its refusals belong — at the boot, before any core exists — cannot then be moved onto
  the core that ticks it, and the failure names `Worker::spawn`'s bound rather than the schedule.
  Hand the worker a flag saying it is the one that ticks and arm inside its body:
  `crates/nvs-cli/src/serve.rs`'s `Core::ticks` is the shape.
  [until: gone crates/nvs-server/src/schedule.rs:Rc<Cell<usize>>]
