- **A `-p nvs-stdlib` test cannot build any `nvs_db::Connection`, so a `Core\Db` member is testable
  only below the driver.** Its fields are `pub(crate)`; `filed_connection` downcasts a
  `HeldConnection` to `nvs_db::Connection`, so a fake reaches only driver-free members. Split the
  member's tail into a function over the receiver alone (`stream.rs`'s `park_row`), drive it with
  `crate::instance::build(&CLASS, [...])` and count with `NvsObj::refcount_of`.
  [until: gone crates/nvs-stdlib/src/db/stream.rs:NvsObj::refcount_of]
