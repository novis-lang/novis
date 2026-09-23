- **The error-path gate's stem is the text before the first `{`, so a message opening on a format
  hole is silently ineligible.** A `Fault::thrown` written as `format!("{CLASS_NAME}::set(): ...")`
  has an empty stem, falls under the fourteen-character floor, and the gate never asks for the case
  that catches it, which is the wrong direction for a ratchet. Write a refusal a case will assert as
  a literal that opens with real text; the name constants are worth less here than the gate is.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:fn fault_sites]
