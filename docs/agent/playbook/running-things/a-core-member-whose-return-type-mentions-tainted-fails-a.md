- **A `Core` member whose return type mentions `tainted` fails a test in `nvs-types`, and the
  message names neither the roster nor the member you added.** `core_lib.rs`'s
  `a_verified_signature_does_not_launder_its_claims` asserts a closed `BTreeSet` of every
  `(class, member, answer)` whose interned return type carries the qualifier, so a new row comes
  back as a bare `left`/`right` set diff in `-p nvs-types --lib`, two crates from the row you wrote.
  Add the tuple and a sentence to that test's own doc comment in the same slice as the registry row,
  the way its `Core\Zip` and `Core\Xml\Node` paragraphs read. [until: reviewed 2026-09-09]
