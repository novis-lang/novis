- **`conformance_coverage`'s error-path gate reads an 8-line window, so one comment cannot
  declare two adjacent internal-error guards.** A new `Core` member on
  `registry::WRITTEN_CLASS_MEMBERS` has three `Fault::fatal` guards in a row for the constants
  its call site wrote, and a leading paragraph covering all three reaches only the first —
  `every_error_path_is_asserted_or_declared_unreachable` then names one line and reads like a
  missing case. Write "unreachable from source" plus the diagnostic that refuses the call
  directly above **each** guard.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION_WINDOW]
