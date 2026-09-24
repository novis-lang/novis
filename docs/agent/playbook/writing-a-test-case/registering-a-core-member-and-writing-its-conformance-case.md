- **Registering a `Core` member and writing its conformance case are one slice, not two.**
  `crates/nvs-stdlib/tests/conformance_coverage.rs` fails the moment a registry row has no `.nvst`
  case calling it, and `nv verify` reports that as a `-p nvs-stdlib` failure with nothing about the
  member in the message. A class constant counts too: `Core\Path::SEPARATOR` needs a case that
  writes it. [until: reviewed 2026-09-06]
