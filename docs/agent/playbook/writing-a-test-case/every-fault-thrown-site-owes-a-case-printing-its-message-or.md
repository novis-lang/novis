- **Every `Fault::thrown` site owes a case printing its message, or an "unreachable from source"
  declaration.** `every_error_path_is_asserted_or_declared_unreachable` greps each message's stem
  across every case file, so a message that interpolates a dependency's text cannot be asserted.
  Give a refusal a fixed sentence, and satisfy the gate with `catch (LogicError $e) { echo
  $e->message, "\n"; }`.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const DECLARATION: &str]
