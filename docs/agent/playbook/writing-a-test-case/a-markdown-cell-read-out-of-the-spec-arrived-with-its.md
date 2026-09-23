- **A markdown cell read out of the spec arrived with its backslashes doubled**, so a Class column's
  `Core\Reflect` matched no registered class and every member of that row read as unregistered.
  `cells` in `crates/nvs-stdlib/tests/spec_registry_coverage.rs` wrote the backslash when it set its
  `escaped` flag and again when the next character turned out not to be a `|`. Walking a markdown
  cell character by character, write the escape once: a cell is the *rendered* text, so an escape
  that escaped nothing is still one character. [until: reviewed 2026-09-17]
