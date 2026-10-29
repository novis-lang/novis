- **A `Core` member declared to return `mixed` makes every `as string` of its result `tainted`,
  because text out of `mixed` is `tainted` (`rule:security/taint-propagation`).** `Core\Config::setting`
  was designed as `mixed` and "not tainted", and the two cannot both hold: the first test program
  failed with `expected string, found tainted string`. Return a union of what the member really gives,
  as `crates/nvs-stdlib/src/config.rs`'s `SETTING` does. A union's `array<mixed>` arm adds no taint to
  a target with no array in it. `until: gone crates/nvs-stdlib/src/config.rs:const SETTING`
