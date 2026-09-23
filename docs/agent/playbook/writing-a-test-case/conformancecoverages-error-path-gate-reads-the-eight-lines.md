- **`conformance_coverage`'s error-path gate reads the eight lines above the throw, and a `match`
  arm is its own site.** One "no case can reach this" declaration above a `map_err(|error| match …)`
  does not cover the arms inside it, so the gate reports the arm's line even though a comment sits
  three lines above the call. Put the comment on the arm; and since the gate names the message
  prefix rather than the line, two arms formatting the same prefix are one entry and go green
  together. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:no case can reach this]
