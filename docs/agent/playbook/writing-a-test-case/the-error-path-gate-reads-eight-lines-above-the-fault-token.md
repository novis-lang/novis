- **The error-path gate reads eight lines above the `Fault::` token and stops at the first, so one
  comment cannot cover two.** A block comment above `let x = f().map_err(|_| { Fault::thrown(...)
  })?;` covers nothing if "unreachable from source" sits more than `DECLARATION_WINDOW` lines above
  the token (the Rust closure's own lines count), and a second site further down never sees it. Put one
  short declaration immediately above each site, each ending with its own reason.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const DECLARATION_WINDOW]
