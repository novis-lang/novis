- **A `Core` member answering `tainted`, or any member under `Core\Request`, owes a closed set in
  `nvs-types` that fails as a set diff.** `crates/nvs-types/src/core_lib.rs`'s
  `a_verified_signature_does_not_launder_its_claims` and
  `every_request_member_returning_outside_data_returns_it_tainted` (and the
  `reveal_and_the_password_helpers…` sibling) are ratchets, so the repair is the set, the count in
  the message and the doc comment's per-member reason together. `grep -n 'closed at'
  crates/nvs-types/src/core_lib.rs` finds them before the full verify does.
  [until: gone crates/nvs-types/src/core_lib.rs:closed at]
