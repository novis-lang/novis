- **A `Core` member whose parameter is an enum-case union will not take the whole enum.**
  `Core\Hash::hmac`'s third parameter is `rule:types/enum-case-type`'s
  `Core\Hash::Digest::Sha256|Core\Hash::Digest::Sha384|Core\Hash::Digest::Sha512`, not `Core\Hash::Digest`, so a helper
  forwarding a digest through a parameter typed `Core\Hash::Digest` is refused at the forward. Declare the
  union verbatim: it parses in a parameter position and widens to `Core\Hash::Digest` for the members
  taking the whole enum, so one helper forwards to both `hmac` and `Core\Hash::stream`.
  `until: gone crates/nvs-stdlib/src/hash.rs:Core\Hash::stream`
