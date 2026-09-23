- **A `.nvst` case cannot carry a body that is not UTF-8.** `--POST_RAW--` crosses to
  `nvs run --request` as a `String` (`crates/nvs-test/src/request.rs:100`), so nothing in that path
  spells one. Build the octets in the program instead — `Core\Encoding::fromHex` into
  `Core\Test::request`'s `body` key. [until: gone crates/nvs-test/src/request.rs:pub body: Option<String>]
