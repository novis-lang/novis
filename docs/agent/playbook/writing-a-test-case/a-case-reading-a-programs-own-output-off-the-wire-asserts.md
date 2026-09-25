- **A case reading a program's own output off the wire asserts nothing once a door replaces that
  response, and it still goes green.** A connection door writes the response itself, so the line the
  request wrote is nowhere on the wire and a `contains` over what is left can still hold. When a
  slice makes a door write the response, move the request's claims onto the fixture's own reporting
  and re-check what each `read_until` needle now matches. [until: gone crates/nvs-server/src/serve.rs:read_until]
