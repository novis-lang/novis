- **A goal record's check can name a crate that *depends* on the surface and still cannot host
  the test; what settles it is what kind of test that crate's `tests/` already hold.** A manifest
  naming the dependency passes the sibling triage and says nothing: every case under
  `crates/nvs-types/tests/` compiles a source string and reads `Diagnostics` back, so a runtime fact
  — a media type declared on a response — cannot be observed there. Read the first twenty lines of
  any existing test in the named directory before the manifest. [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
