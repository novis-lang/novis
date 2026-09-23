- **A case can *look* like it exercises the intrinsic-literal fold and exercise nothing: the list
  names the member that reads the pattern, not its siblings.** `rule:expressions/intrinsic-literals`
  lists `Core\Regex::compile`, not `matches`, so `Core\Regex::matches($s, "^order-\\d+$")` only
  checks that two runtime calls agree. Check the line against `INTRINSICS` in
  `crates/nvs-types/src/intrinsics.rs`, and prove it: a deliberate error in the same text is an
  `E0769` from `nvs check`. [until: reviewed 2026-09-06]
