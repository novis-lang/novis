- **A `Core` member whose parameter is an enum-case union will not take the whole enum.**
  `Core\Hash::hmac`'s third parameter is `rule:types/enum-case-type`'s
  `Core\Digest::Sha256|Core\Digest::Sha384|Core\Digest::Sha512`, not `Core\Digest`, so a helper
  forwarding a digest through a parameter typed `Core\Digest` is refused at the forward. Declare the
  union verbatim: it parses in a parameter position and widens to `Core\Digest` for the members
  taking the whole enum, so one helper forwards to both `hmac` and `Core\Hash::stream`.
  [until: reviewed 2026-09-06]
