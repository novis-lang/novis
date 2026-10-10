- **A `Core` member may not reach the operating system, and the gate that says so is a list of
  spellings rather than of effects.** `nvs_stdlib_reaches_the_os_only_through_the_gate`
  (`crates/nvs-stdlib/tests/capability.rs`) forbids literals such as `std::fs`,
  `std::process::Command` and `std::env::var` in this crate's `src/`: an env read fails while `std::io::stdin().is_terminal()` or a raw `libc::ioctl` beside it
  passes unremarked. Put the reaching in `nvs-runtime` (`nvs_runtime::terminal`),
  never a `registry::CAPABILITIES` row, and decide the crate before the first line. `until: gone crates/nvs-stdlib/tests/capability.rs:const FORBIDDEN`
