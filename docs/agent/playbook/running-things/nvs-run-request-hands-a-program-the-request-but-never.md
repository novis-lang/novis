- **`nvs run --request` hands a program the request but never matches it against the route table, so
  `Core\Request::route()` answers `null` in that leg.** `crates/nvs-cli/src/main.rs:1651` installs the
  table because `Core\Router`'s own members read it, and says in the same breath that a program run off
  the command line is matched against nothing — only the server's door and `runner.rs:504`'s test seam
  call `Inbound::set_route`. A fixture that needs the match spells the door's walk itself, over
  `Core\Request::method()` and `Core\Request::path()`, which is what `examples/parses.nvs` does.
  [until: gone crates/nvs-cli/src/main.rs:matched against nothing]
