- **`cargo insta accept` does not exist on this box, and the failure reads as a mistyped cargo
  subcommand.** The error is *"a command with a similar name exists: `init`"*; `cargo test` still
  writes each pending snapshot as `*.snap.new` beside its `*.snap`, and accepting one by hand is
  copying it over its neighbour minus the `assertion_line:` header line. Run `git status --short`
  first and `diff` each pair: any `.snap.new` you did not just produce — a previous session's
  renamed test, say — is one to delete, not accept; `cargo install cargo-insta` fixes it for every
  future session. [until: reviewed 2026-09-06]
