- **A `Fault::` message opening on a format hole cannot be asserted, and a `///` does not declare
  the site.** `every_error_path_is_asserted_or_declared_unreachable` keys a site by the literal text
  before its first `{`, so `Core\Metrics::{member}: …` had the stem `Core\Metrics::`, which no case
  could match. Put plain words before the first hole, and the `unreachable from source` sentence in
  a `//` comment within eight lines above the `Fault::` line — not on the enclosing `fn`.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_error_path_is_asserted_or_declared_unreachable]
